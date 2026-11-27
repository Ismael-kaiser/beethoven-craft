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

pub fn pintar(ui: &mut Ui, c: Clase, r: &Receta, v: &BTreeMap<String, f32>, sr: f32, agudo: bool, id: Id, m: &crate::interfaz::Momento) {
    let (z, _) = ui.allocate_exact_size(vec2(ui.available_width(), 110.0), Sense::hover());
    en(ui, z, c, r, v, sr, agudo, id, 1.0, m);
}

/// En ese lugar.
#[allow(clippy::too_many_arguments)]
pub fn en(
    ui: &mut Ui,
    z: Rect,
    c: Clase,
    r: &Receta,
    v: &BTreeMap<String, f32>,
    sr: f32,
    agudo: bool,
    id: Id,
    k: f32,
    vivo: &crate::interfaz::Momento,
) {
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
    // Lo que suena en este momento, si suena.
    let suena = vivo.historia[1].len() > 1;
    let azul = Color32::from_rgb(0x6a, 0x9c, 0xff);
    let en_db = |x: f32| ((db_de(x) + 60.0) / 66.0).clamp(0.0, 1.0);
    let relleno = |v: &[f32], col: Color32| {
        let pts = linea(v);
        let mut q = Mesh::default();
        for (i, o) in pts.iter().enumerate() {
            q.colored_vertex(*o, col);
            q.colored_vertex(pos2(o.x, dentro.bottom()), col.gamma_multiply(0.25));
            if i > 0 {
                let j = 2 * i as u32;
                q.add_triangle(j - 2, j - 1, j);
                q.add_triangle(j - 1, j, j + 1);
            }
        }
        p.add(Shape::mesh(q));
    };
    let dbs = |v: &[f32]| -> Vec<f32> { v.iter().map(|x| en_db(*x)).collect() };
    let lineal = |v: &[f32]| -> Vec<f32> { v.iter().map(|x| (x * 0.9).clamp(0.0, 1.0)).collect() };
    match (c, suena) {
        // La señal de verdad pasa por detrás de la curva, y un punto dice dónde va.
        (Clase::Niveles, true) => {
            relleno(&dbs(&vivo.historia[0]), Color32::from_white_alpha(34));
            let sale: Vec<f32> = vivo.historia[1].iter().zip(&vivo.historia[2]).map(|(a, b)| a.max(*b)).collect();
            relleno(&dbs(&sale), ACENTO.gamma_multiply(0.22));
            p.add(Shape::line(linea(&datos[1]), Stroke::new(1.0, Color32::from_white_alpha(60))));
            p.add(Shape::line(linea(&datos[0]), Stroke::new(1.8, ACENTO)));
            let x = ((db_de(vivo.nivel_entra) + 60.0) / 60.0).clamp(0.0, 1.0);
            let o = pos2(dentro.left() + dentro.width() * x, dentro.bottom() - dentro.height() * en_db(vivo.nivel));
            p.circle_filled(o, 9.0 * k, ACENTO.gamma_multiply(0.2));
            p.circle_filled(o, 4.5 * k, Color32::WHITE);
        }
        (Clase::Frecuencias, true) => {
            relleno(&vivo.espectro_entra, Color32::from_white_alpha(30));
            relleno(&vivo.espectro, azul.gamma_multiply(0.4));
            p.add(Shape::line(linea(&datos[1]), Stroke::new(1.0, Color32::from_white_alpha(60))));
            p.add(Shape::line(linea(&datos[0]), Stroke::new(1.8, ACENTO)));
        }
        (Clase::Onda, true) => {
            let alto = vivo.onda.iter().chain(&vivo.onda_entra).fold(0.25f32, |m, x| m.max(x.abs()));
            let a = |v: &[f32]| -> Vec<f32> { v.iter().map(|x| 0.5 + x / alto * 0.42).collect() };
            p.add(Shape::line(linea(&a(&vivo.onda_entra)), Stroke::new(1.2, Color32::from_white_alpha(90))));
            p.add(Shape::line(linea(&a(&vivo.onda)), Stroke::new(1.8, ACENTO)));
        }
        (Clase::Cola, true) => {
            p.add(Shape::line(linea(&datos[0]), Stroke::new(1.0, Color32::from_white_alpha(40))));
            relleno(&dbs(&vivo.historia[0]), Color32::from_white_alpha(36));
            p.add(Shape::line(linea(&dbs(&vivo.historia[2])), Stroke::new(1.2, azul)));
            p.add(Shape::line(linea(&dbs(&vivo.historia[1])), Stroke::new(1.8, ACENTO)));
        }
        (Clase::Movimiento, true) => {
            relleno(&lineal(&vivo.historia[0]), Color32::from_white_alpha(26));
            p.add(Shape::line(linea(&lineal(&vivo.historia[2])), Stroke::new(1.2, azul)));
            p.add(Shape::line(linea(&lineal(&vivo.historia[1])), Stroke::new(1.8, ACENTO)));
        }
        // Callado: lo que haría, medido con una señal de prueba. La segunda va detrás: la referencia, o el otro lado.
        _ => {
            let otra = if matches!(c, Clase::Cola | Clase::Movimiento) { azul } else { Color32::from_white_alpha(60) };
            p.add(Shape::line(linea(&datos[1]), Stroke::new(1.2, otra)));
            p.add(Shape::line(linea(&datos[0]), Stroke::new(1.8, ACENTO)));
        }
    }
    if suena {
        torno::foco(&p, z.right_top() + vec2(-10.0, 10.0) * k, 3.0 * k, ROJO, 1.0);
    }
    let nombre = match (c, suena) {
        (Clase::Cola | Clase::Movimiento, true) => "Lo que suena (4 s)",
        _ => c.nombre(),
    };
    p.text(z.left_top() + vec2(6.0, 3.0), Align2::LEFT_TOP, t(nombre), letra(11.0 * k), TENUE);
}

fn db_de(x: f32) -> f32 {
    20.0 * x.max(1e-6).log10()
}

/// La onda que entra, detrás, y la que sale.
pub fn onda(p: &Painter, z: Rect, m: &crate::interfaz::Momento, c: Color32, k: f32) {
    torno::pantalla(p, z, 4.0 * k, k * 0.8);
    let p = p.with_clip_rect(z.intersect(p.clip_rect()));
    p.hline(z.x_range(), z.center().y, Stroke::new(1.0, Color32::from_white_alpha(18)));
    // Lo bajito se agranda para que se vea; callado es una raya.
    let alto = m.onda.iter().chain(&m.onda_entra).fold(0.1f32, |m, x| m.max(x.abs()));
    let linea = |v: &[f32]| -> Vec<Pos2> {
        let n = v.len().max(2) - 1;
        v.iter()
            .enumerate()
            .map(|(i, y)| pos2(z.left() + z.width() * i as f32 / n as f32, z.center().y - (y / alto).clamp(-1.1, 1.1) * z.height() * 0.44))
            .collect()
    };
    if m.onda.len() > 1 {
        if m.onda_entra.iter().any(|x| x.abs() > 1e-4) {
            p.add(Shape::line(linea(&m.onda_entra), Stroke::new(1.0, Color32::from_white_alpha(80))));
        }
        p.add(Shape::line(linea(&m.onda), Stroke::new(3.5 * k, c.gamma_multiply(0.18))));
        p.add(Shape::line(linea(&m.onda), Stroke::new(1.4 * k, c)));
    }
}

/// Los últimos segundos, como la forma de onda de un clip que va pasando.
pub fn historia(p: &Painter, z: Rect, m: &crate::interfaz::Momento, c: Color32, k: f32) {
    torno::pantalla(p, z, 4.0 * k, k * 0.8);
    let p = p.with_clip_rect(z.intersect(p.clip_rect()));
    let n = m.historia[1].len();
    for (v, col, g) in [(&m.historia[0], Color32::from_white_alpha(60), 1.0), (&m.historia[1], c, 0.8)] {
        for (i, x) in v.iter().enumerate().filter(|x| *x.1 > 1e-4) {
            let x0 = z.left() + z.width() * i as f32 / n as f32;
            let h = x.min(1.0) * z.height() * 0.46;
            p.vline(x0, z.center().y - h..=z.center().y + h, Stroke::new((z.width() / n as f32 * g).max(1.0), col));
        }
    }
}
