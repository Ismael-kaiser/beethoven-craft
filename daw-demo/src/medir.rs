// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Hace pasar una señal de prueba por un efecto, fuera del motor, para dibujar lo que de verdad hace.
use crate::receta::{BLOQUE, Buf, DatosVoz, Receta, comunes, voz};

pub fn correr(r: &Receta, ctl: &[f32], entra: &[Vec<f32>; 2]) -> [Vec<f32>; 2] {
    correr_con(r, ctl, entra, None)
}

/// Con la huella de un lugar.
pub fn correr_con(r: &Receta, ctl: &[f32], entra: &[Vec<f32>; 2], m: Option<&crate::muestrario::Muestra>) -> [Vec<f32>; 2] {
    let mut mem = vec![0.0; r.memoria];
    let mut b: Vec<Buf> = vec![[0.0; BLOQUE]; r.cuantos_nodos()];
    let mut x: [Buf; 2] = [[0.0; BLOQUE]; 2];
    let largo = entra[0].len();
    let mut sale = [Vec::with_capacity(largo), Vec::with_capacity(largo)];
    for ini in (0..largo).step_by(BLOQUE) {
        let n = BLOQUE.min(largo - ini);
        for l in 0..2 {
            x[l][..n].copy_from_slice(&entra[l][ini..ini + n]);
        }
        comunes(r, ctl, &mut b, n);
        let s = voz(r, &DatosVoz { negra: (ini as f64 * 2.0 / r.sr() as f64) as f32, ..Default::default() }, ctl, m, Some(&x), &mut mem, &mut b, n);
        for l in 0..2 {
            sale[l].extend_from_slice(&b[s[l] as usize][..n]);
        }
    }
    sale
}

/// Una nota de un instrumento, suelta a la mitad.
pub fn nota(r: &Receta, ctl: &[f32], hz: f32, largo: usize) -> [Vec<f32>; 2] {
    let (mut mem, mut b): (Vec<f32>, Vec<Buf>) = (vec![0.0; r.memoria], vec![[0.0; BLOQUE]; r.cuantos_nodos()]);
    let mut sale = [vec![], vec![]];
    for ini in (0..largo).step_by(BLOQUE) {
        let n = BLOQUE.min(largo - ini);
        comunes(r, ctl, &mut b, n);
        let d = DatosVoz { hz, fuerza: 0.9, azar: 0.4, puerta: ini < largo / 2, ..Default::default() };
        let s = voz(r, &d, ctl, None, None, &mut mem, &mut b, n);
        (0..2).for_each(|l| sale[l].extend_from_slice(&b[s[l] as usize][..n]));
    }
    sale
}
