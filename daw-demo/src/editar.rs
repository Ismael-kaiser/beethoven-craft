// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Editar audio: cortar, nivelar, limpiar y cambiar tiempo o tono. Todo sobre cuadros estéreo en memoria; el archivo
//! original no se toca.
use std::f32::consts::PI;
use std::ops::Range;

pub type Audio = Vec<[f32; 2]>;

/// A cuánto lleva el pico «normalizar»: un decibel bajo el tope.
pub const PICO: f32 = 0.891;

fn cada(a: &mut [[f32; 2]], r: Range<usize>, f: impl Fn(usize, [f32; 2]) -> [f32; 2]) {
    for (i, x) in a[r].iter_mut().enumerate() {
        *x = f(i, *x);
    }
}

pub fn silenciar(a: &mut [[f32; 2]], r: Range<usize>) {
    cada(a, r, |_, _| [0.0; 2]);
}

pub fn ganancia(a: &mut [[f32; 2]], r: Range<usize>, db: f32) {
    let k = 10f32.powf(db / 20.0);
    cada(a, r, |_, x| x.map(|v| v * k));
}

pub fn normalizar(a: &mut [[f32; 2]], r: Range<usize>) {
    let pico = a[r.clone()].iter().flatten().fold(0.0f32, |m, v| m.max(v.abs()));
    if pico > 1e-6 {
        cada(a, r, |_, x| x.map(|v| v * PICO / pico));
    }
}

/// Fundido de potencia pareja, de entrada o de salida.
pub fn fundido(a: &mut [[f32; 2]], r: Range<usize>, entra: bool) {
    let n = r.len().saturating_sub(1).max(1) as f32;
    cada(a, r, |i, x| {
        let k = i as f32 / n;
        let g = ((if entra { k } else { 1.0 - k }) * PI / 2.0).sin();
        x.map(|v| v * g)
    });
}

pub fn invertir(a: &mut [[f32; 2]], r: Range<usize>) {
    a[r].reverse();
}

pub fn voltear_fase(a: &mut [[f32; 2]], r: Range<usize>) {
    cada(a, r, |_, x| x.map(|v| -v));
}

pub fn mono(a: &mut [[f32; 2]], r: Range<usize>) {
    cada(a, r, |_, x| [(x[0] + x[1]) * 0.5; 2]);
}

pub fn cambiar_lados(a: &mut [[f32; 2]], r: Range<usize>) {
    cada(a, r, |_, x| [x[1], x[0]]);
}

/// Quita la corriente continua: centra la onda.
pub fn centrar(a: &mut [[f32; 2]], r: Range<usize>) {
    let n = r.len().max(1) as f64;
    let m = [0, 1].map(|l| (a[r.clone()].iter().map(|x| x[l] as f64).sum::<f64>() / n) as f32);
    cada(a, r, |_, x| [x[0] - m[0], x[1] - m[1]]);
}

/// Acorta a una décima de segundo cada silencio (menos de -45 dB) que pase de tres décimas.
pub fn quitar_silencios(a: &[[f32; 2]], sr: f32) -> Audio {
    let paso = ((sr * 0.01) as usize).max(1);
    let umbral = 10f32.powf(-45.0 / 20.0);
    let callado: Vec<bool> = a.chunks(paso).map(|c| c.iter().flatten().all(|v| v.abs() < umbral)).collect();
    let (mut sale, mut i) = (Audio::with_capacity(a.len()), 0);
    while i < callado.len() {
        let j = i + callado[i..].iter().take_while(|c| **c == callado[i]).count();
        let trozo = &a[i * paso..(j * paso).min(a.len())];
        if callado[i] && j - i > 30 {
            // La mitad del principio y la mitad del final: así no se cortan las colas ni los ataques.
            sale.extend_from_slice(&trozo[..5 * paso]);
            sale.extend_from_slice(&trozo[trozo.len() - 5 * paso..]);
        } else {
            sale.extend_from_slice(trozo);
        }
        i = j;
    }
    sale
}

/// Transformada rápida de Fourier en el sitio; el largo es potencia de dos.
fn fft(re: &mut [f32], im: &mut [f32], atras: bool) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut b = n >> 1;
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
    let mut m = 2;
    while m <= n {
        let ang = if atras { 2.0 } else { -2.0 } * PI / m as f32;
        for k in 0..m / 2 {
            let (s, c) = (ang * k as f32).sin_cos();
            for i in (k..n).step_by(m) {
                let q = i + m / 2;
                let (x, y) = (re[q] * c - im[q] * s, re[q] * s + im[q] * c);
                (re[q], im[q]) = (re[i] - x, im[i] - y);
                re[i] += x;
                im[i] += y;
            }
        }
        m <<= 1;
    }
    if atras {
        re.iter_mut().chain(im.iter_mut()).for_each(|v| *v /= n as f32);
    }
}

const CUADRO: usize = 2048;
const SALTO: usize = CUADRO / 4;

/// Hann con su raíz: se usa al entrar y al salir.
fn ventana() -> Vec<f32> {
    (0..CUADRO).map(|i| (PI * i as f32 / CUADRO as f32).sin()).collect()
}

fn espectro(x: &[f32], desde: isize, w: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let mut re: Vec<f32> = (0..CUADRO).map(|i| x.get((desde + i as isize) as usize).map_or(0.0, |v| v * w[i])).collect();
    let mut im = vec![0.0; CUADRO];
    fft(&mut re, &mut im, false);
    (re, im)
}

/// Resta el ruido que se aprendió de `muestra` (un trozo donde solo se oye el ruido).
pub fn quitar_ruido(a: &[[f32; 2]], muestra: &[[f32; 2]]) -> Audio {
    let w = ventana();
    let mut sale = vec![[0.0f32; 2]; a.len()];
    for l in 0..2 {
        let x: Vec<f32> = a.iter().map(|v| v[l]).collect();
        let r: Vec<f32> = muestra.iter().map(|v| v[l]).collect();
        let mut ruido = vec![0.0f32; CUADRO];
        let cuantos = (r.len().saturating_sub(CUADRO) / SALTO) + 1;
        for k in 0..cuantos {
            let (re, im) = espectro(&r, (k * SALTO) as isize, &w);
            for i in 0..CUADRO {
                ruido[i] += (re[i] * re[i] + im[i] * im[i]).sqrt() / cuantos as f32;
            }
        }
        let mut antes = vec![1.0f32; CUADRO];
        // Empieza antes del principio para que las orillas queden completas.
        let mut desde = -(CUADRO as isize) + SALTO as isize;
        while desde < x.len() as isize {
            let (mut re, mut im) = espectro(&x, desde, &w);
            let crudo: Vec<f32> = (0..CUADRO)
                .map(|i| {
                    let m = (re[i] * re[i] + im[i] * im[i]).sqrt();
                    ((m - 2.0 * ruido[i]) / m.max(1e-12)).max(0.06)
                })
                .collect();
            for i in 0..CUADRO {
                // Parejo entre vecinos y en el tiempo, para que no «gorjee».
                let v = (crudo[i.saturating_sub(1)] + crudo[i] + crudo[(i + 1).min(CUADRO - 1)]) / 3.0;
                antes[i] = v.max(antes[i] * 0.6);
                re[i] *= antes[i];
                im[i] *= antes[i];
            }
            fft(&mut re, &mut im, true);
            for i in 0..CUADRO {
                let j = desde + i as isize;
                if j >= 0 && (j as usize) < sale.len() {
                    // Cuatro ventanas encimadas suman 2.
                    sale[j as usize][l] += re[i] * w[i] * 0.5;
                }
            }
            desde += SALTO as isize;
        }
    }
    sale
}

/// Cambia lo que dura sin cambiar el tono: encima trozos cortos, cada uno donde mejor empata con el anterior.
pub fn estirar(a: &[[f32; 2]], sr: f32, veces: f32) -> Audio {
    let medio = ((sr * 0.02) as usize).max(32);
    let busca = (medio / 2) as isize;
    let largo = (a.len() as f64 * veces as f64).round() as usize;
    if a.len() < medio * 4 {
        return remuestrear(a, largo);
    }
    let junto: Vec<f32> = a.iter().map(|v| v[0] + v[1]).collect();
    let en = |i: isize| if i < 0 { 0.0 } else { junto.get(i as usize).copied().unwrap_or(0.0) };
    let mut sale = vec![[0.0f32; 2]; largo + medio * 2];
    let mut sigue = 0isize;
    let mut k = 0;
    while k * medio < largo {
        let ideal = ((k * medio) as f64 / veces as f64) as isize;
        let mut mejor = (f32::MIN, ideal);
        if k > 0 {
            for d in (-busca..=busca).step_by(2) {
                let c = ideal + d;
                let (mut xy, mut yy) = (0.0, 1e-9);
                for i in (0..medio as isize).step_by(4) {
                    xy += en(sigue + i) * en(c + i);
                    yy += en(c + i) * en(c + i);
                }
                let p = xy / yy.sqrt();
                if p > mejor.0 {
                    mejor = (p, c);
                }
            }
        }
        let c = mejor.1.clamp(0, a.len() as isize - 1) as usize;
        for i in 0..medio * 2 {
            let Some(x) = a.get(c + i) else { break };
            let mut g = 0.5 - 0.5 * (PI * i as f32 / medio as f32).cos();
            // El primer trozo no entra en fundido.
            if k == 0 && i < medio {
                g = 1.0;
            }
            let s = &mut sale[k * medio + i];
            *s = [s[0] + x[0] * g, s[1] + x[1] * g];
        }
        sigue = (c + medio) as isize;
        k += 1;
    }
    sale.truncate(largo);
    sale
}

/// El mismo sonido en otra cantidad de cuadros (curva de cuatro puntos).
pub fn remuestrear(a: &[[f32; 2]], largo: usize) -> Audio {
    if a.is_empty() || largo == 0 {
        return vec![];
    }
    let paso = a.len() as f64 / largo as f64;
    let en = |i: isize, l: usize| a[i.clamp(0, a.len() as isize - 1) as usize][l];
    (0..largo)
        .map(|j| {
            let q = j as f64 * paso;
            let (i, f) = (q as isize, q.fract() as f32);
            [0, 1].map(|l| {
                let (p, b, c, d) = (en(i - 1, l), en(i, l), en(i + 1, l), en(i + 2, l));
                b + 0.5 * f * (c - p + f * (2.0 * p - 5.0 * b + 4.0 * c - d + f * (3.0 * (b - c) + d - p)))
            })
        })
        .collect()
}

/// Sube o baja el tono sin cambiar lo que dura.
pub fn cambiar_tono(a: &[[f32; 2]], sr: f32, semitonos: f32) -> Audio {
    let r = 2f32.powf(semitonos / 12.0);
    remuestrear(&estirar(a, sr, r), a.len())
}
