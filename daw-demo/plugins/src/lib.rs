// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Los efectos e instrumentos de Beethoven, de fábrica y los que instale quien lo usa, como plugins VST3 y CLAP. Un solo
//! archivo trae todos: cada uno aparece en el otro programa como «Beethoven …».
#![allow(non_snake_case, non_upper_case_globals, unsafe_op_in_unsafe_fn)]

mod clap;
mod lienzo;
mod ventana;
mod vst;

use beethoven_daw_demo::instancia;
use beethoven_daw_demo::lua::{Ficha, Tipo};
use std::ffi::CString;
use std::sync::OnceLock;

/// Uno de los que se ofrecen.
pub struct Cual {
    pub clave: String,
    pub nombre: String,
    pub ficha: Ficha,
    pub id: [u8; 16],
    pub c_nombre: CString,
    pub c_id: CString,
}

impl Cual {
    pub fn instrumento(&self) -> bool {
        self.ficha.tipo == Tipo::Instrumento
    }
}

pub fn todos() -> &'static [Cual] {
    static T: OnceLock<Vec<Cual>> = OnceLock::new();
    T.get_or_init(|| {
        beethoven_daw_demo::ui::poner_idioma(Some(&beethoven_daw_demo::ui::carpeta()));
        let limpio = |s: &str| CString::new(s.replace('\0', "")).unwrap_or_default();
        instancia::disponibles()
            .into_iter()
            .map(|(clave, ficha)| {
                let nombre = format!("Beethoven {}", ficha.nombre.ver());
                Cual { id: huella(&clave), c_id: limpio(&format!("org.beethoven.{clave}")), c_nombre: limpio(&nombre), nombre, clave, ficha }
            })
            .collect()
    })
}

/// Dieciséis números fijos a partir de la clave: así un proyecto guardado lo vuelve a encontrar siempre.
fn huella(clave: &str) -> [u8; 16] {
    let uno = |sal: u64| {
        clave
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325u64 ^ sal, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3))
            .rotate_left(17)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
    };
    let mut v = [0; 16];
    v[..8].copy_from_slice(&uno(0x6265_6574).to_be_bytes());
    v[8..].copy_from_slice(&uno(0x686f_7665).to_be_bytes());
    v
}

/// De 0 a 1 y de regreso. Las frecuencias, en octavas.
pub fn a01(c: &beethoven_daw_demo::receta::Control, v: f32) -> f64 {
    let (a, b, v) = (c.min as f64, c.max as f64, v as f64);
    if a > 0.0 && b / a > 20.0 { (v / a).ln() / (b / a).ln() } else { (v - a) / (b - a) }.clamp(0.0, 1.0)
}

pub fn de01(c: &beethoven_daw_demo::receta::Control, x: f64) -> f32 {
    let (a, b, x) = (c.min as f64, c.max as f64, x.clamp(0.0, 1.0));
    (if a > 0.0 && b / a > 20.0 { a * (b / a).powf(x) } else { a + (b - a) * x }) as f32
}

pub fn escrito(v: f32) -> String {
    if v.abs() >= 100.0 {
        format!("{v:.0}")
    } else if v.abs() >= 10.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}

/// Un error aquí no puede tumbar al otro programa.
pub fn seguro<T>(malo: T, f: impl FnOnce() -> T) -> T {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or(malo)
}
