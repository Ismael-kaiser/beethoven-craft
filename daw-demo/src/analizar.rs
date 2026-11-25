// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Oír con números: nivel, silencios, tempo, tonalidad y reparto de frecuencias de un audio.

/// En dB; -120 es nada.
pub fn db(x: f32) -> f32 {
    20.0 * x.max(1e-6).log10()
}

pub fn pico(x: &[f32]) -> f32 {
    x.iter().fold(0f32, |m, v| m.max(v.abs()))
}

pub fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt() as f32
}

/// Los pedazos callados de al menos `minimo` segundos, en segundos.
pub fn silencios(x: &[f32], sr: f32, umbral_db: f32, minimo: f32) -> Vec<(f32, f32)> {
    let paso = (sr * 0.01) as usize;
    let umbral = 10f32.powf(umbral_db / 20.0);
    let (mut v, mut ini) = (vec![], None);
    let n = x.len().div_ceil(paso.max(1));
    for (i, c) in x.chunks(paso.max(1)).enumerate() {
        match (pico(c) < umbral, ini) {
            (true, None) => ini = Some(i),
            (false, Some(a)) => {
                v.push((a, i));
                ini = None;
            }
            _ => {}
        }
    }
    if let Some(a) = ini {
        v.push((a, n));
    }
    let fin = x.len() as f32 / sr;
    v.into_iter().map(|(a, b)| (a as f32 * 0.01, (b as f32 * 0.01).min(fin))).filter(|(a, b)| b - a >= minimo).collect()
}

/// Qué tanto sube la energía, cada 10 ms.
fn golpes(x: &[f32], sr: f32) -> Vec<f32> {
    let paso = (sr * 0.01) as usize;
    let mut antes = [0f32; 3];
    let (mut b1, mut b2) = (0f32, 0f32);
    let mut v = vec![];
    for c in x.chunks_exact(paso.max(1)) {
        // Tres bandas con dos filtros de un polo: graves, medios y agudos.
        let mut e = [0f32; 3];
        for s in c {
            b1 += (s - b1) * (1.0 - (-2.0 * std::f32::consts::PI * 150.0 / sr).exp());
            b2 += (s - b2) * (1.0 - (-2.0 * std::f32::consts::PI * 2500.0 / sr).exp());
            e[0] += b1 * b1;
            e[1] += (b2 - b1).powi(2);
            e[2] += (s - b2).powi(2);
        }
        let l = e.map(|x| (x / paso as f32 * 1e4 + 1.0).ln());
        v.push((0..3).map(|k| (l[k] - antes[k]).max(0.0)).sum());
        antes = l;
    }
    v
}

/// El tempo y qué tan seguro es (0 a 1).
pub fn tempo(x: &[f32], sr: f32) -> Option<(f32, f32)> {
    let g = golpes(x, sr);
    if g.len() < 400 {
        return None;
    }
    let media = g.iter().sum::<f32>() / g.len() as f32;
    let g: Vec<f32> = g.iter().map(|v| v - media).collect();
    let cero: f32 = g.iter().map(|v| v * v).sum();
    if cero < 1e-3 {
        return None;
    }
    let cor = |d: usize| g.iter().zip(&g[d..]).map(|(a, b)| a * b).sum::<f32>() / (g.len() - d) as f32 * g.len() as f32 / cero;
    // Entre 60 y 200; lo cercano a 120 pesa un poco más, para no dar el doble o la mitad.
    let puntos = |d: usize| {
        let bpm = 6000.0 / d as f32;
        let cerca = (-((bpm / 115.0).log2() / 0.75).powi(2) / 2.0).exp();
        let mas = if d * 2 < g.len() / 2 { cor(d * 2) } else { 0.0 };
        (cor(d) + 0.5 * mas) * cerca
    };
    let d = (30..=100).max_by(|a, b| puntos(*a).total_cmp(&puntos(*b)))?;
    // Más exacto: el mismo golpe ocho veces después.
    let lejos = (8 * d).min(g.len() / 2) / d;
    let centro = d * lejos;
    let m = (centro.saturating_sub(lejos)..=centro + lejos).max_by(|a, b| cor(*a).total_cmp(&cor(*b)))?;
    let (y0, y1, y2) = (cor(m - 1), cor(m), cor(m + 1));
    let fino = if (y0 - 2.0 * y1 + y2).abs() > 1e-9 { 0.5 * (y0 - y2) / (y0 - 2.0 * y1 + y2) } else { 0.0 };
    Some((6000.0 * lejos as f32 / (m as f32 + fino.clamp(-0.5, 0.5)), cor(d).clamp(0.0, 1.0)))
}

pub const NOTAS: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/// Cuánto hay de cada nota, sumando octavas.
pub fn notas(x: &[f32], sr: f32) -> [f32; 12] {
    let mut v = [0f32; 12];
    // Con medio minuto basta, repartido por todo el audio.
    let bloque = (sr * 0.2) as usize;
    let saltos = (x.len() / bloque.max(1) / 150).max(1);
    for c in x.chunks_exact(bloque.max(1)).step_by(saltos) {
        for (k, o) in v.iter_mut().enumerate() {
            for octava in 3..7 {
                // Goertzel: una sola frecuencia.
                let hz = 440.0 * 2f32.powf((12 * octava + k as i32 - 57) as f32 / 12.0);
                let w = 2.0 * std::f32::consts::PI * hz / sr;
                let co = 2.0 * w.cos();
                let (mut a, mut b) = (0f32, 0f32);
                let l = c.len() as f32;
                for (i, s) in c.iter().enumerate() {
                    let ventana = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / l).cos();
                    let n = s * ventana + co * a - b;
                    b = a;
                    a = n;
                }
                *o += (a * a + b * b - co * a * b).max(0.0).sqrt() / l;
            }
        }
    }
    v
}

/// La tonalidad que más se parece (perfiles de Krumhansl y Kessler): tónica y si es mayor.
pub fn tonalidad(x: &[f32], sr: f32) -> Option<(usize, bool)> {
    const MAYOR: [f32; 12] = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
    const MENOR: [f32; 12] = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];
    let v = notas(x, sr);
    if v.iter().sum::<f32>() < 1e-4 {
        return None;
    }
    let parecido = |p: &[f32; 12], t: usize| {
        let (ma, mb) = (v.iter().sum::<f32>() / 12.0, p.iter().sum::<f32>() / 12.0);
        let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
        for k in 0..12 {
            let (a, b) = (v[(k + t) % 12] - ma, p[k] - mb);
            ab += a * b;
            aa += a * a;
            bb += b * b;
        }
        ab / (aa * bb).sqrt().max(1e-9)
    };
    (0..24)
        .max_by(|a, b| {
            parecido(if a / 12 == 0 { &MAYOR } else { &MENOR }, a % 12).total_cmp(&parecido(if b / 12 == 0 { &MAYOR } else { &MENOR }, b % 12))
        })
        .map(|k| (k % 12, k / 12 == 0))
}

/// Nivel medio en dB de graves (<200), medios (200-4000) y agudos (>4000).
pub fn bandas(x: &[f32], sr: f32) -> [f32; 3] {
    let k = |hz: f32| 1.0 - (-2.0 * std::f32::consts::PI * hz / sr).exp();
    let (ka, kb) = (k(200.0), k(4000.0));
    let (mut a, mut a2, mut b, mut b2) = (0f32, 0f32, 0f32, 0f32);
    let mut e = [0f64; 3];
    for s in x {
        a += (s - a) * ka;
        a2 += (a - a2) * ka;
        b += (s - b) * kb;
        b2 += (b - b2) * kb;
        for (e, v) in e.iter_mut().zip([a2, b2 - a2, s - b2]) {
            *e += (v as f64).powi(2);
        }
    }
    e.map(|e| db((e / x.len().max(1) as f64).sqrt() as f32))
}
