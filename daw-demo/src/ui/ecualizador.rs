// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! La curva del ecualizador: cada banda se arrastra con el ratón.
use super::*;
use crate::receta::{Banda, banda, respuesta};

const TIPOS: [Banda; 4] = [Banda::Campana, Banda::Graves, Banda::Agudos, Banda::Muesca];
const TONOS: [Color32; 6] = [
    Color32::from_rgb(0xff, 0x8a, 0x6a),
    Color32::from_rgb(0xff, 0xc8, 0x5e),
    Color32::from_rgb(0xa8, 0xe0, 0x70),
    Color32::from_rgb(0x5e, 0xd8, 0xc8),
    Color32::from_rgb(0x7a, 0xae, 0xff),
    Color32::from_rgb(0xc8, 0x90, 0xff),
];
const DB: f32 = 20.0;

pub fn curva(ui: &mut Ui, f: &crate::lua::Ficha, v: &mut BTreeMap<String, f32>, id: Id, alto: f32, onda: Option<&[f32]>, sr: f32) {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), alto), Sense::click_and_drag());
    let p = ui.painter_at(r.expand(1.0));
    torno::pantalla(ui.painter(), r, 5.0, 1.0);
    let x_de = |hz: f32| r.left() + r.width() * (hz / 20.0).ln() / 1000f32.ln();
    let hz_de = |x: f32| 20.0 * ((x - r.left()) / r.width() * 1000f32.ln()).exp();
    let y_de = |db: f32| r.center().y - db / DB * r.height() / 2.0;
    for hz in [50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0] {
        p.vline(x_de(hz), r.y_range(), Stroke::new(1.0, Color32::from_white_alpha(12)));
    }
    for (hz, s) in [(100.0, "100"), (1000.0, "1k"), (10000.0, "10k")] {
        p.text(pos2(x_de(hz) + 2.0, r.bottom() - 2.0), Align2::LEFT_BOTTOM, s, letra(10.5), TENUE);
    }
    for db in [-12.0, 0.0, 12.0] {
        p.hline(r.x_range(), y_de(db), Stroke::new(1.0, Color32::from_white_alpha(if db == 0.0 { 34 } else { 14 })));
    }
    // Lo que suena, detrás de la curva.
    let memoria = id.with("espectro");
    let mut esp: Vec<f32> = ui.data(|d| d.get_temp(memoria)).unwrap_or_else(|| vec![-90.0; 96]);
    let ahora = onda.map(|o| espectro(o, sr, esp.len()));
    for (k, e) in esp.iter_mut().enumerate() {
        let x = ahora.as_ref().map_or(-90.0, |a| a[k]);
        *e = if x > *e { x } else { *e + (x - *e) * 0.12 };
    }
    let n = esp.len();
    let punto_e =
        |k: usize| pos2(r.left() + r.width() * k as f32 / (n - 1) as f32, r.bottom() - ((esp[k] + 84.0) / 84.0).clamp(0.0, 1.0) * r.height());
    let mut m = Mesh::default();
    for k in 0..n {
        let q = punto_e(k);
        m.colored_vertex(q, Color32::from_rgb(0x6a, 0x9c, 0xff).gamma_multiply(0.45));
        m.colored_vertex(pos2(q.x, r.bottom()), Color32::from_rgb(0x6a, 0x9c, 0xff).gamma_multiply(0.08));
        if k > 0 {
            let j = 2 * k as u32;
            m.add_triangle(j - 2, j - 1, j);
            m.add_triangle(j - 1, j, j + 1);
        }
    }
    p.add(Shape::mesh(m));
    ui.data_mut(|d| d.insert_temp(memoria, esp));
    let lee = |v: &BTreeMap<String, f32>, k: String| v.get(&k).copied().or(f.controles.iter().find(|c| c.clave == k).map(|c| c.valor)).unwrap_or(0.0);
    // Tantas bandas como «h1», «h2»… tenga el guion.
    let bandas = (1..=16).take_while(|i| f.controles.iter().any(|c| c.clave == format!("h{i}"))).count();
    let b: Vec<(f32, f32, f32)> =
        (0..bandas).map(|i| (lee(v, format!("h{}", i + 1)), lee(v, format!("g{}", i + 1)), lee(v, format!("q{}", i + 1)))).collect();
    let tipo = |i: usize| TIPOS[(lee(v, format!("t{}", i + 1)).round() as usize).min(3)];
    let k: Vec<[f32; 5]> =
        (0..bandas).map(|i| banda(tipo(i), b[i].0, b[i].1, if tipo(i) == Banda::Muesca { b[i].2 * 2.0 } else { b[i].2 }, 48000.0)).collect();
    let (corta, tope) = (lee(v, "corta".into()), lee(v, "tope".into()));
    let punto = |i: usize| pos2(x_de(b[i].0), y_de(b[i].1));
    let corte = |r: f32| -10.0 * (1.0 + r.powi(4)).log10();
    let en = |x: f32| {
        let hz = hz_de(x);
        let suma = k.iter().map(|k| respuesta(k, hz, 48000.0)).sum::<f32>();
        suma + if corta > 20.5 { corte(corta / hz) } else { 0.0 } + if tope > 0.0 && tope < 19900.0 { corte(hz / tope) } else { 0.0 }
    };
    let pts: Vec<Pos2> = (0..=r.width() as usize / 2)
        .map(|i| r.left() + i as f32 * 2.0)
        .map(|x| pos2(x, y_de(en(x)).clamp(r.top() + 1.0, r.bottom() - 1.0)))
        .collect();
    let cero = y_de(0.0);
    let mut m = Mesh::default();
    for (i, q) in pts.iter().enumerate() {
        m.colored_vertex(*q, ACENTO.gamma_multiply(0.28));
        m.colored_vertex(pos2(q.x, cero), ACENTO.gamma_multiply(0.04));
        if i > 0 {
            let j = 2 * i as u32;
            m.add_triangle(j - 2, j - 1, j);
            m.add_triangle(j - 1, j, j + 1);
        }
    }
    p.add(Shape::mesh(m));
    p.add(Shape::line(pts, Stroke::new(2.0, ACENTO)));
    let o = ui.input(|i| i.pointer.interact_pos());
    let cerca = o
        .filter(|o| r.contains(*o))
        .and_then(|o| (0..bandas).map(|i| (i, punto(i).distance(o))).filter(|x| x.1 < 14.0).min_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0));
    let dato = id.with("banda");
    if resp.drag_started() || resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.any_pressed()) {
        ui.data_mut(|d| d.insert_temp(dato, cerca));
    }
    let tomada: Option<usize> = ui.data(|d| d.get_temp(dato)).flatten();
    for i in 0..bandas {
        let q = punto(i);
        let el = tomada == Some(i) || cerca == Some(i);
        p.circle_filled(q, if el { 8.0 } else { 6.5 }, TONOS[i % TONOS.len()]);
        p.circle_filled(q - vec2(1.5, 2.0), 2.2, Color32::from_white_alpha(130));
        p.circle_stroke(q, if el { 8.0 } else { 6.5 }, Stroke::new(1.0, Color32::BLACK));
        p.text(q, Align2::CENTER_CENTER, format!("{}", i + 1), letra(11.0), FONDO);
    }
    let mut pone = |i: usize, k: &str, x: f32| {
        let clave = format!("{k}{}", i + 1);
        if let Some(c) = f.controles.iter().find(|c| c.clave == clave) {
            v.insert(clave, x.clamp(c.min, c.max));
        }
    };
    if let (Some(i), Some(o), true) = (tomada, o, resp.dragged()) {
        pone(i, "h", hz_de(o.x));
        pone(i, "g", (r.center().y - o.y) / (r.height() / 2.0) * DB);
    }
    if let Some(i) = cerca {
        let y = ui.input(|i| i.smooth_scroll_delta.y);
        if y != 0.0 {
            pone(i, "q", b[i].2 * (y / 200.0).exp());
            ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
        }
        if resp.double_clicked() {
            pone(i, "g", 0.0);
        }
        resp.on_hover_text(tf("{} Hz · {} dB · ancho {}", &[&numero(b[i].0), &format!("{:+.1}", b[i].1), &numero(b[i].2)]));
    }
}

/// Decibeles de 20 Hz a 20 kHz en `n` puntos, parejos en octavas.
pub fn espectro(x: &[f32], sr: f32, n: usize) -> Vec<f32> {
    let l = x.len();
    let pi = std::f32::consts::PI;
    let mut re: Vec<f32> = x.iter().enumerate().map(|(i, v)| v * (0.5 - 0.5 * (2.0 * pi * i as f32 / l as f32).cos())).collect();
    let mut im = vec![0.0f32; l];
    // Transformada rápida de Fourier, de base dos.
    let bits = l.trailing_zeros();
    for i in 0..l {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if j > i {
            re.swap(i, j);
        }
    }
    let mut t = 2;
    while t <= l {
        let a = -2.0 * pi / t as f32;
        for i in (0..l).step_by(t) {
            for k in 0..t / 2 {
                let (s, c) = (a * k as f32).sin_cos();
                let (u, v) = (i + k, i + k + t / 2);
                let (xr, xi) = (re[v] * c - im[v] * s, re[v] * s + im[v] * c);
                (re[v], im[v]) = (re[u] - xr, im[u] - xi);
                (re[u], im[u]) = (re[u] + xr, im[u] + xi);
            }
        }
        t *= 2;
    }
    let en = |k: f32| 20.0 * 1000f32.powf(k / (n - 1) as f32) * l as f32 / sr;
    (0..n)
        .map(|k| {
            let (a, b) = (en(k as f32 - 0.5), en(k as f32 + 0.5));
            let (a, b) = ((a as usize).clamp(1, l / 2 - 1), (b.ceil() as usize).clamp(2, l / 2));
            let m = (a..b.max(a + 1)).map(|i| re[i].hypot(im[i])).fold(0f32, f32::max);
            20.0 * (m * 4.0 / l as f32).max(1e-5).log10()
        })
        .collect()
}
