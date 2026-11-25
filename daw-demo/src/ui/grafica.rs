// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! La gráfica de cada efecto. No es un dibujo de adorno: se le pasa una señal de prueba y se pinta lo que sale.
use super::*;
use crate::medir::correr;
use crate::receta::Receta;
use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq)]
pub enum Clase {
    /// Cuánto sale según cuánto entra.
    Niveles,
    Frecuencias,
    Cola,
    Movimiento,
    Onda,
}

impl Clase {
    pub fn de(grupo: &str) -> Option<Clase> {
        Some(match grupo {
            "Dinámica" => Clase::Niveles,
            "Tono" => Clase::Frecuencias,
            "Espacio" => Clase::Cola,
            "Movimiento" => Clase::Movimiento,
            "Carácter" => Clase::Onda,
            _ => return None,
        })
    }

    /// La que pide un guion («niveles», «agudos», «frecuencias», «cola», «movimiento» u «onda»), y si se prueba con agudos.
    pub fn pedida(s: &str) -> Option<(Clase, bool)> {
        Some(match s {
            "niveles" => (Clase::Niveles, false),
            "agudos" => (Clase::Niveles, true),
            "frecuencias" => (Clase::Frecuencias, false),
            "cola" => (Clase::Cola, false),
            "movimiento" => (Clase::Movimiento, false),
            "onda" => (Clase::Onda, false),
            _ => return None,
        })
    }

    fn nombre(self) -> &'static str {
        match self {
            Clase::Niveles => "Cuánto sale según cuánto entra",
            Clase::Frecuencias => "Respuesta en frecuencia",
            Clase::Cola => "Ecos y cola (3 s)",
            Clase::Movimiento => "Cómo se mueve (2 s): izquierda y derecha",
            Clase::Onda => "La onda al entrar y al salir",
        }
    }
}

/// Dos líneas, cada punto entre 0 (abajo) y 1 (arriba).
pub fn medida(c: Clase, r: &Receta, ctl: &[f32], sr: f32, agudo: bool) -> [Vec<f32>; 2] {
    let seno = |hz: f32, a: f32, n: usize| -> Vec<f32> { (0..n).map(|i| (TAU * hz * i as f32 / sr).sin() * a).collect() };
    let pasa = |x: Vec<f32>| correr(r, ctl, &[x.clone(), x]);
    let db = |v: f32| 20.0 * v.max(1e-6).log10();
    let pico = |v: &[f32]| v.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    match c {
        Clase::Niveles => {
            let n = (sr * 0.25) as usize;
            let hz = if agudo { 8000.0 } else { 1000.0 };
            let sale = (0..=30)
                .map(|i| {
                    let y = pasa(seno(hz, 10f32.powf((i as f32 * 2.0 - 60.0) / 20.0), n));
                    ((db(pico(&y[0][n - (sr * 0.02) as usize..])) + 60.0) / 66.0).clamp(0.0, 1.0)
                })
                .collect();
            [sale, (0..=30).map(|i| i as f32 * 2.0 / 66.0).collect()]
        }
        Clase::Frecuencias => {
            const N: usize = 8192;
            let mut x = vec![0.0; N];
            x[0] = 0.1;
            let y = pasa(x);
            let curva = (0..160)
                .map(|k| {
                    let w = TAU * 20.0 * 1000f32.powf(k as f32 / 159.0) / sr;
                    let (re, im) = y[0].iter().enumerate().fold((0.0f32, 0.0f32), |(a, b), (i, v)| {
                        let (s, c) = (w * i as f32).sin_cos();
                        (a + v * c, b - v * s)
                    });
                    ((db(re.hypot(im) / 0.1) + 24.0) / 48.0).clamp(0.0, 1.0)
                })
                .collect();
            [curva, vec![0.5; 2]]
        }
        Clase::Cola => {
            let n = (sr * 3.0) as usize;
            let mut x = vec![0.0; n];
            x[0] = 1.0;
            let y = pasa(x);
            [0, 1].map(|l| y[l].chunks(n / 300).map(|t| ((db(pico(t)) + 60.0) / 60.0).clamp(0.0, 1.0)).collect())
        }
        Clase::Movimiento => {
            let n = (sr * 2.0) as usize;
            let y = pasa(seno(220.0, 0.5, n));
            [0, 1].map(|l| y[l].chunks(n / 300).map(|t| (pico(t) * 0.9).clamp(0.0, 1.0)).collect())
        }
        Clase::Onda => {
            let (antes, dos) = ((sr * 0.1) as usize, (sr / 200.0) as usize * 2);
            let x = seno(200.0, 0.8, antes + dos);
            let y = pasa(x.clone());
            let a_la_vista = |v: &[f32]| v[antes..].iter().map(|v| (v * 0.45 + 0.5).clamp(0.0, 1.0)).collect();
            [a_la_vista(&y[0]), a_la_vista(&x)]
        }
    }
}

pub fn pintar(ui: &mut Ui, c: Clase, r: &Receta, v: &BTreeMap<String, f32>, sr: f32, agudo: bool, id: Id) {
    let (z, _) = ui.allocate_exact_size(vec2(ui.available_width(), 110.0), Sense::hover());
    en(ui, z, c, r, v, sr, agudo, id, 1.0);
}

/// En ese lugar.
#[allow(clippy::too_many_arguments)]
pub fn en(ui: &mut Ui, z: Rect, c: Clase, r: &Receta, v: &BTreeMap<String, f32>, sr: f32, agudo: bool, id: Id, k: f32) {
    let ctl: Vec<f32> = r.controles.iter().map(|k| v.get(&k.clave).copied().unwrap_or(k.valor)).collect();
    // Solo se vuelve a medir cuando cambia algo.
    let ya: Option<(Vec<f32>, Arc<[Vec<f32>; 2]>)> = ui.data(|d| d.get_temp(id));
    let datos = match ya.filter(|x| x.0 == ctl) {
        Some(x) => x.1,
        None => {
            let x = Arc::new(medida(c, r, &ctl, sr, agudo));
            ui.data_mut(|d| d.insert_temp(id, (ctl, x.clone())));
            x
        }
    };
    let resp = ui.interact(z, id.with("zona"), Sense::hover());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, t(c.nombre())));
    let p = ui.painter_at(z.expand(1.0));
    torno::pantalla(ui.painter(), z, 5.0 * k, k);
    for k in 1..4 {
        let k = k as f32 / 4.0;
        p.vline(z.left() + z.width() * k, z.y_range(), Stroke::new(1.0, Color32::from_white_alpha(12)));
        p.hline(z.x_range(), z.top() + z.height() * k, Stroke::new(1.0, Color32::from_white_alpha(12)));
    }
    let dentro = z.shrink2(vec2(2.0, 4.0));
    let linea = |v: &[f32]| -> Vec<Pos2> {
        let n = v.len().max(2) - 1;
        v.iter().enumerate().map(|(i, y)| pos2(dentro.left() + dentro.width() * i as f32 / n as f32, dentro.bottom() - dentro.height() * y)).collect()
    };
    // La segunda va detrás: la referencia, o el otro lado.
    let otra = if matches!(c, Clase::Cola | Clase::Movimiento) { Color32::from_rgb(0x6a, 0x9c, 0xff) } else { Color32::from_white_alpha(60) };
    p.add(Shape::line(linea(&datos[1]), Stroke::new(1.2, otra)));
    p.add(Shape::line(linea(&datos[0]), Stroke::new(1.8, ACENTO)));
    p.text(z.left_top() + vec2(6.0, 3.0), Align2::LEFT_TOP, t(c.nombre()), letra(11.0 * k), TENUE);
}
