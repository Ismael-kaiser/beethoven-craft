//! Curvas de animación: cómo va un valor de un cuadro clave al siguiente.
//!
//! Las fórmulas vienen de AniCraft (https://github.com/storytold/anicraft, commit 7ce262d, `crates/doc/src/easing.rs`),
//! Copyright (c) 2026 ArtCraft Team and the AniCraft contributors, MIT o Apache-2.0: las ecuaciones de suavizado
//! clásicas (R. Penner) en sus tres sentidos y la curva Bézier al estilo CSS. «Suave» es la que Beethoven Movie usaba
//! antes de tener curvas, para que los proyectos viejos se animen igual.
use serde::{Deserialize, Serialize};

/// Las familias de curvas con nombre.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Familia {
    Quad,
    Cubic,
    Quart,
    Quint,
    Sine,
    Back,
    Circ,
    Bounce,
    Elastic,
}

/// Hacia dónde suaviza.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sentido {
    Entrar,
    Salir,
    Ambos,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Curva {
    /// Arranque y llegada suaves (la de siempre).
    #[default]
    Suave,
    Lineal,
    /// Salto: se queda en el valor hasta el siguiente cuadro clave.
    Salto,
    Familia(Familia, Sentido),
    /// Bézier cúbica de (0,0) a (1,1) con sus dos manijas.
    Bezier(f32, f32, f32, f32),
}

/// El exceso de «Back» (cerca del 10 %).
const BACK: f64 = 1.701_58;

impl Familia {
    pub const TODAS: [Familia; 9] = [Familia::Quad, Familia::Cubic, Familia::Quart, Familia::Quint, Familia::Sine, Familia::Back, Familia::Circ, Familia::Bounce, Familia::Elastic];

    fn entrar(self, t: f64) -> f64 {
        use std::f64::consts::PI;
        match self {
            Familia::Quad => t * t,
            Familia::Cubic => t * t * t,
            Familia::Quart => t * t * t * t,
            Familia::Quint => t * t * t * t * t,
            Familia::Sine => 1.0 - (t * PI * 0.5).cos(),
            Familia::Back => (BACK + 1.0) * t * t * t - BACK * t * t,
            Familia::Circ => 1.0 - (1.0 - t * t).max(0.0).sqrt(),
            Familia::Bounce => 1.0 - rebote(1.0 - t),
            Familia::Elastic => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    -(2f64.powf(10.0 * t - 10.0)) * ((10.0 * t - 10.75) * (2.0 * PI / 3.0)).sin()
                }
            }
        }
    }

    pub fn nombre(self) -> &'static str {
        match self {
            Familia::Quad => "Suave",
            Familia::Cubic => "Media",
            Familia::Quart => "Fuerte",
            Familia::Quint => "Muy fuerte",
            Familia::Sine => "Seno",
            Familia::Back => "Con impulso",
            Familia::Circ => "Circular",
            Familia::Bounce => "Rebote",
            Familia::Elastic => "Elástica",
        }
    }
}

fn rebote(t: f64) -> f64 {
    const N: f64 = 7.5625;
    const D: f64 = 2.75;
    if t < 1.0 / D {
        N * t * t
    } else if t < 2.0 / D {
        let u = t - 1.5 / D;
        N * u * u + 0.75
    } else if t < 2.5 / D {
        let u = t - 2.25 / D;
        N * u * u + 0.9375
    } else {
        let u = t - 2.625 / D;
        N * u * u + 0.984_375
    }
}

fn bez(t: f64, p: [f64; 4]) -> f64 {
    let mt = 1.0 - t;
    mt * mt * mt * p[0] + 3.0 * mt * mt * t * p[1] + 3.0 * mt * t * t * p[2] + t * t * t * p[3]
}

fn dbez(t: f64, p: [f64; 4]) -> f64 {
    let mt = 1.0 - t;
    3.0 * mt * mt * (p[1] - p[0]) + 6.0 * mt * t * (p[2] - p[1]) + 3.0 * t * t * (p[3] - p[2])
}

/// El parámetro en que la Bézier `p` (que no baja) llega a `x`: pasos de Newton dentro de un intervalo que se achica.
fn resolver(x: f64, p: [f64; 4]) -> f64 {
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    let mut s = x.clamp(0.0, 1.0);
    for _ in 0..48 {
        let err = bez(s, p) - x;
        if err.abs() < 1e-10 {
            break;
        }
        if err < 0.0 {
            lo = s;
        } else {
            hi = s;
        }
        let d = dbez(s, p);
        let sig = if d.abs() > 1e-12 { s - err / d } else { f64::NAN };
        s = if sig > lo && sig < hi { sig } else { 0.5 * (lo + hi) };
    }
    s
}

impl Curva {
    /// Avance suavizado para un avance lineal `t` entre 0 y 1. Puede pasarse de 0..1 (impulso, rebote, elástica).
    pub fn aplicar(self, t: f64) -> f64 {
        let t = if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 };
        match self {
            Curva::Suave => t * t * (3.0 - 2.0 * t),
            Curva::Lineal => t,
            Curva::Salto => {
                if t >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
            Curva::Familia(f, Sentido::Entrar) => f.entrar(t),
            Curva::Familia(f, Sentido::Salir) => 1.0 - f.entrar(1.0 - t),
            Curva::Familia(f, Sentido::Ambos) => {
                if t < 0.5 {
                    0.5 * f.entrar(2.0 * t)
                } else {
                    1.0 - 0.5 * f.entrar(2.0 - 2.0 * t)
                }
            }
            Curva::Bezier(x1, y1, x2, y2) => {
                if ![x1, y1, x2, y2].iter().all(|v| v.is_finite()) {
                    return t;
                }
                let (x1, x2) = (f64::from(x1.clamp(0.0, 1.0)), f64::from(x2.clamp(0.0, 1.0)));
                bez(resolver(t, [0.0, x1, x2, 1.0]), [0.0, f64::from(y1), f64::from(y2), 1.0])
            }
        }
    }

    /// Las que se ofrecen en los menús.
    pub fn menu() -> Vec<Curva> {
        let mut v = vec![Curva::Suave, Curva::Lineal, Curva::Salto];
        for f in Familia::TODAS {
            for s in [Sentido::Salir, Sentido::Entrar, Sentido::Ambos] {
                v.push(Curva::Familia(f, s));
            }
        }
        v
    }

    /// Su nombre, ya en el idioma de la app.
    pub fn nombre(self) -> String {
        use crate::idioma::{t, tf};
        match self {
            Curva::Suave => t("Suave (la de siempre)").into(),
            Curva::Lineal => t("Lineal").into(),
            Curva::Salto => t("Salto").into(),
            Curva::Bezier(..) => t("Bézier").into(),
            Curva::Familia(f, s) => {
                let sentido = match s {
                    Sentido::Entrar => t("al arrancar"),
                    Sentido::Salir => t("al llegar"),
                    Sentido::Ambos => t("al arrancar y al llegar"),
                };
                tf("{} {}", &[&t(f.nombre()), &sentido])
            }
        }
    }
}
