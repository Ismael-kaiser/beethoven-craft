// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Reverb de convolución: multiplica el sonido por la huella de un lugar (una sala, una placa, un resorte o un audio
//! tuyo). Sin retraso: el principio de la huella se calcula muestra por muestra y la cola por partes, con Fourier.
use std::f32::consts::TAU;

/// Muestras de cada parte.
pub const PARTE: usize = 512;
const FFT: usize = PARTE * 2;
const BINS: usize = PARTE + 1;
/// Lo más larga que puede ser una huella, en segundos.
pub const MAX_SEG: f32 = 4.0;

/// Cuántos lugares de fábrica hay: cabina, estudio, sala, catedral, placa, resorte, baño y bocina de radio.
pub const LUGARES: usize = 8;

#[derive(Debug, Default)]
struct Lado {
    /// El principio, al revés.
    cabeza: Vec<f32>,
    /// La cola: partes reales e imaginarias de cada trozo.
    partes: Vec<(Vec<f32>, Vec<f32>)>,
}

#[derive(Debug)]
pub struct Impulso {
    lados: [Lado; 2],
    /// Cosenos y senos de la transformada.
    giros: Vec<(f32, f32)>,
    /// La huella resumida en 256 picos, para dibujarla.
    pub picos: Vec<f32>,
    pub segundos: f32,
}

fn giros() -> Vec<(f32, f32)> {
    (0..FFT / 2)
        .map(|k| {
            let (s, c) = (TAU * k as f32 / FFT as f32).sin_cos();
            (c, s)
        })
        .collect()
}

fn fft(re: &mut [f32], im: &mut [f32], giros: &[(f32, f32)], atras: bool) {
    let mut j = 0;
    for i in 1..FFT {
        let mut b = FFT >> 1;
        while j & b != 0 {
            j ^= b;
            b >>= 1;
        }
        j |= b;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let signo = if atras { 1.0 } else { -1.0 };
    let mut m = 2;
    while m <= FFT {
        let (mitad, salto) = (m / 2, FFT / m);
        for i in (0..FFT).step_by(m) {
            for k in 0..mitad {
                let (c, s) = giros[k * salto];
                let s = s * signo;
                let q = i + k + mitad;
                let (x, y) = (re[q] * c - im[q] * s, re[q] * s + im[q] * c);
                (re[q], im[q]) = (re[i + k] - x, im[i + k] - y);
                re[i + k] += x;
                im[i + k] += y;
            }
        }
        m <<= 1;
    }
}

impl Impulso {
    /// De una huella estéreo a la frecuencia del motor.
    pub fn nuevo(mut h: [Vec<f32>; 2], sr: f32) -> Impulso {
        let tope = (MAX_SEG * sr) as usize;
        let n = h[0].len().min(h[1].len()).min(tope).max(1);
        // Que el final no truene.
        let baja = (n / 8).clamp(1, (sr * 0.05) as usize + 1);
        for l in &mut h {
            l.resize(n, 0.0);
            for k in 0..baja {
                l[n - 1 - k] *= k as f32 / baja as f32;
            }
        }
        // La misma fuerza para todas: ni se pierde ni revienta al cambiar de lugar.
        let energia = h.iter().flatten().map(|x| x * x).sum::<f32>() / 2.0;
        let g = if energia > 1e-12 { 1.0 / energia.sqrt() } else { 0.0 };
        h.iter_mut().flatten().for_each(|x| *x *= g);
        let giros = giros();
        let picos =
            (0..256).map(|k| h[0][k * n / 256..((k + 1) * n / 256).max(k * n / 256 + 1).min(n)].iter().fold(0f32, |m, x| m.max(x.abs()))).collect();
        let lados = [0, 1].map(|l| {
            let mut cabeza: Vec<f32> = h[l][..n.min(PARTE)].to_vec();
            cabeza.resize(PARTE, 0.0);
            cabeza.reverse();
            let partes = h[l][n.min(PARTE)..]
                .chunks(PARTE)
                .map(|c| {
                    let (mut re, mut im) = (vec![0.0; FFT], vec![0.0; FFT]);
                    re[..c.len()].copy_from_slice(c);
                    fft(&mut re, &mut im, &giros, false);
                    re.truncate(BINS);
                    im.truncate(BINS);
                    (re, im)
                })
                .collect();
            Lado { cabeza, partes }
        });
        Impulso { lados, giros, picos, segundos: n as f32 / sr }
    }

    /// Uno de los de fábrica. `largo`: qué tanto de su cola queda (0 a 1).
    pub fn de_fabrica(cual: usize, sr: f32, largo: f32, reves: bool) -> Impulso {
        let h = huella(cual, sr);
        Impulso::nuevo(recortar(h, largo, reves), sr)
    }

    /// Cuántos números guarda cada lado al sonar.
    pub fn memoria(sr: f32) -> usize {
        3 + PARTE * 2 + FFT + PARTE + FFT * 4 + Self::cuantas(sr) * BINS * 2
    }

    fn cuantas(sr: f32) -> usize {
        ((MAX_SEG * sr) as usize).div_ceil(PARTE)
    }

    /// Hilo de audio: no pide memoria.
    pub fn tocar(&self, lado: usize, x: &[f32], o: &mut [f32], mem: &mut [f32], sr: f32) {
        let h = &self.lados[lado];
        let anillo = Self::cuantas(sr);
        let (cab, resto) = mem.split_at_mut(3);
        let (hist, resto) = resto.split_at_mut(PARTE * 2);
        let (junta, resto) = resto.split_at_mut(FFT);
        let (sale, resto) = resto.split_at_mut(PARTE);
        let (re, resto) = resto.split_at_mut(FFT);
        let (im, resto) = resto.split_at_mut(FFT);
        let (ar, resto) = resto.split_at_mut(FFT);
        let (ai, espectros) = resto.split_at_mut(FFT);
        let (mut lleva, mut va, mut p) = (cab[0] as usize, cab[1] as usize, cab[2] as usize);
        for (x, o) in x.iter().zip(o.iter_mut()) {
            // La cabeza, directo. La historia va dos veces seguidas para leerla sin dar la vuelta.
            hist[p] = *x;
            hist[p + PARTE] = *x;
            let desde = &hist[p + 1..p + 1 + PARTE];
            let mut s = [0.0f32; 8];
            for (a, b) in desde.chunks_exact(8).zip(h.cabeza.chunks_exact(8)) {
                for k in 0..8 {
                    s[k] += a[k] * b[k];
                }
            }
            p = (p + 1) % PARTE;
            *o = s.iter().sum::<f32>() + sale[lleva];
            junta[PARTE + lleva] = *x;
            lleva += 1;
            if lleva < PARTE {
                continue;
            }
            lleva = 0;
            re.copy_from_slice(junta);
            im.fill(0.0);
            fft(re, im, &self.giros, false);
            va = (va + 1) % anillo;
            let e = &mut espectros[va * BINS * 2..(va + 1) * BINS * 2];
            e[..BINS].copy_from_slice(&re[..BINS]);
            e[BINS..].copy_from_slice(&im[..BINS]);
            ar.fill(0.0);
            ai.fill(0.0);
            for (k, (hr, hi)) in h.partes.iter().take(anillo).enumerate() {
                let de = (va + anillo - k) % anillo;
                let (xr, xi) = espectros[de * BINS * 2..(de + 1) * BINS * 2].split_at(BINS);
                let (ar, ai) = (&mut ar[..BINS], &mut ai[..BINS]);
                for b in 0..BINS {
                    ar[b] += xr[b] * hr[b] - xi[b] * hi[b];
                    ai[b] += xr[b] * hi[b] + xi[b] * hr[b];
                }
            }
            for b in 1..PARTE {
                ar[FFT - b] = ar[b];
                ai[FFT - b] = -ai[b];
            }
            fft(ar, ai, &self.giros, true);
            for (s, a) in sale.iter_mut().zip(&ar[PARTE..]) {
                *s = a / FFT as f32;
            }
            junta.copy_within(PARTE.., 0);
        }
        (cab[0], cab[1], cab[2]) = (lleva as f32, va as f32, p as f32);
    }
}

/// Deja el principio de la huella, apagándose, y la voltea si se pide.
pub fn recortar(mut h: [Vec<f32>; 2], largo: f32, reves: bool) -> [Vec<f32>; 2] {
    let largo = largo.clamp(0.02, 1.0);
    for l in &mut h {
        let n = ((l.len() as f32 * largo) as usize).max(1);
        l.truncate(n);
        if largo < 0.999 {
            l.iter_mut().enumerate().for_each(|(i, x)| *x *= (1.0 - i as f32 / n as f32).powf(1.0 - largo * 0.5));
        }
        if reves {
            l.reverse();
        }
    }
    h
}

/// Las huellas de fábrica salen de cuentas: ruido que se apaga y se opaca, rebotes sueltos, barridos de resorte.
fn huella(cual: usize, sr: f32) -> [Vec<f32>; 2] {
    // segundos, brillo al inicio y al final (Hz), rebotes, hasta cuándo llegan, cuánto tarda en llenarse.
    let (seg, hz0, hz1, rebotes, hasta, llena): (f32, f32, f32, usize, f32, f32) = match cual {
        0 => (0.25, 9000.0, 3000.0, 10, 0.02, 0.001),
        1 => (0.6, 11000.0, 4000.0, 18, 0.04, 0.004),
        2 => (2.2, 10000.0, 2500.0, 26, 0.09, 0.02),
        3 => (4.0, 8000.0, 1200.0, 34, 0.18, 0.06),
        4 => (2.4, 15000.0, 6000.0, 0, 0.0, 0.0005),
        5 => (2.0, 4500.0, 3500.0, 0, 0.0, 0.001),
        6 => (0.9, 14000.0, 7000.0, 30, 0.03, 0.001),
        _ => (0.06, 4000.0, 3000.0, 3, 0.006, 0.0002),
    };
    let n = (seg * sr) as usize;
    [0u32, 1].map(|lado| {
        let mut s = 0x9E37_79B9u32 ^ (cual as u32 + 1).wrapping_mul(7919) ^ lado.wrapping_mul(104_729);
        let mut azar = || {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            s as f32 / u32::MAX as f32 * 2.0 - 1.0
        };
        let mut v = vec![0.0f32; n];
        if cual == 5 {
            // El resorte: cada vuelta llega como un barrido, con los graves primero.
            for vuelta in 0..14 {
                let ini = sr * (0.033 + lado as f32 * 0.004) * (vuelta as f32 * 1.02 + 0.6);
                let dura = sr * (0.03 + vuelta as f32 * 0.012);
                let fuerza = 0.78f32.powi(vuelta);
                let mut fase = azar();
                for k in 0..dura as usize {
                    let i = ini as usize + k;
                    if i >= n {
                        break;
                    }
                    let t = k as f32 / dura;
                    fase += (180.0 + 3800.0 * t * t) / sr;
                    v[i] += (TAU * fase).sin() * fuerza * (1.0 - t) * (t * 12.0).min(1.0);
                }
            }
            return v;
        }
        let (mut y, mut y2) = (0.0f32, 0.0f32);
        for (i, v) in v.iter_mut().enumerate() {
            let t = i as f32 / sr;
            let hz = hz0 * (hz1 / hz0).powf(t / seg);
            let k = 1.0 - (-TAU * hz / sr).exp();
            y += (azar() - y) * k;
            y2 += (y - y2) * k;
            // 60 dB abajo al final.
            *v = y2 / k.sqrt() * 10f32.powf(-3.0 * t / seg) * (1.0 - (-t / llena).exp());
        }
        for k in 0..rebotes {
            let t = hasta * ((k as f32 + 0.5 + azar() * 0.45) / rebotes as f32).max(0.0).powf(1.4);
            let i = (t * sr) as usize;
            if i < n {
                v[i] += (if k % 2 == 0 { 1.0 } else { -1.0 }) * 2.2 * (1.0 - 0.8 * t / hasta);
            }
        }
        if cual == 7 {
            // La bocina no da graves.
            let mut z = 0.0f32;
            let k = 1.0 - (-TAU * 350.0 / sr).exp();
            v[0] += 3.0;
            v.iter_mut().for_each(|x| {
                z += (*x - z) * k;
                *x -= z;
            });
        }
        v
    })
}
