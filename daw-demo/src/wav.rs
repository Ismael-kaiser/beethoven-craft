// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Leer y escribir WAV.
use crate::idioma::{t, tf};
use std::path::Path;

pub struct Wav {
    pub izq: Vec<f32>,
    pub der: Vec<f32>,
    pub sr: f32,
}

pub fn leer(ruta: &Path) -> Result<Wav, String> {
    de_bytes(&std::fs::read(ruta).map_err(|e| e.to_string())?)
}

pub fn de_bytes(b: &[u8]) -> Result<Wav, String> {
    if b.len() < 12 || &b[..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err(t("no es un WAV").into());
    }
    let u16_ = |x: &[u8], i: usize| u16::from_le_bytes([x[i], x[i + 1]]);
    let u32_ = |x: &[u8], i: usize| u32::from_le_bytes([x[i], x[i + 1], x[i + 2], x[i + 3]]);
    let (mut i, mut formato) = (12, None);
    while i + 8 <= b.len() {
        let largo = u32_(b, i + 4) as usize;
        let d = &b[i + 8..(i + 8).saturating_add(largo).min(b.len())];
        match &b[i..i + 4] {
            b"fmt " if d.len() >= 16 => {
                let mut tipo = u16_(d, 0);
                if tipo == 0xFFFE && d.len() >= 26 {
                    tipo = u16_(d, 24);
                }
                formato = Some((tipo, u16_(d, 2) as usize, u32_(d, 4) as f32, u16_(d, 14) as usize));
            }
            b"data" => {
                let Some((tipo, canales, sr, bits)) = formato else { return Err(t("WAV sin formato").into()) };
                let ancho = bits / 8;
                if canales == 0 || sr < 1000.0 || ancho == 0 {
                    return Err(t("WAV con un formato raro").into());
                }
                let uno = |x: &[u8]| -> Option<f32> {
                    Some(match (tipo, bits) {
                        (1, 8) => (x[0] as f32 - 128.0) / 128.0,
                        (1, 16) => i16::from_le_bytes([x[0], x[1]]) as f32 / 32768.0,
                        (1, 24) => (i32::from_le_bytes([0, x[0], x[1], x[2]]) >> 8) as f32 / 8_388_608.0,
                        (1, 32) => i32::from_le_bytes([x[0], x[1], x[2], x[3]]) as f32 / 2_147_483_648.0,
                        (3, 32) => {
                            let v = f32::from_le_bytes([x[0], x[1], x[2], x[3]]);
                            if v.is_finite() { v } else { 0.0 }
                        }
                        _ => return None,
                    })
                };
                let (mut izq, mut der) = (vec![], vec![]);
                for c in d.chunks_exact(ancho * canales) {
                    let a = uno(c).ok_or_else(|| tf("WAV de {} bits (tipo {}) no soportado", &[&bits, &tipo]))?;
                    izq.push(a);
                    der.push(if canales > 1 { uno(&c[ancho..]).unwrap_or(a) } else { a });
                }
                return Ok(Wav { izq, der, sr });
            }
            _ => {}
        }
        i += 8 + largo + (largo & 1);
    }
    Err(t("WAV sin sonido").into())
}

/// Estéreo de 24 bits.
pub fn escribir(ruta: &Path, sr: u32, x: &[[f32; 2]]) -> Result<(), String> {
    let datos = x.len() * 6;
    let mut b = Vec::with_capacity(44 + datos);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + datos as u32).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&sr.to_le_bytes());
    b.extend_from_slice(&(sr * 6).to_le_bytes());
    b.extend_from_slice(&6u16.to_le_bytes());
    b.extend_from_slice(&24u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(datos as u32).to_le_bytes());
    for v in x.iter().flatten() {
        let i = (v.clamp(-1.0, 1.0) * 8_388_607.0).round() as i32;
        b.extend_from_slice(&i.to_le_bytes()[..3]);
    }
    std::fs::write(ruta, b).map_err(|e| e.to_string())
}
