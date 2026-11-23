//! Figuras para animar: se dibujan por distancia al borde, así que salen lisas a cualquier tamaño.
use crate::modelo::{Forma, TipoForma};
use effectcraft_raster::Image;
use rayon::prelude::*;

/// Distancia al borde (negativa adentro) de la figura de medio ancho `a` y medio alto `b`.
fn distancia(f: &Forma, x: f32, y: f32, a: f32, b: f32) -> f32 {
    let poligono = |v: &[[f32; 2]]| -> f32 {
        let (mut d, mut dentro) = (f32::MAX, false);
        for i in 0..v.len() {
            let ([x0, y0], [x1, y1]) = (v[i], v[(i + 1) % v.len()]);
            let (ex, ey, px, py) = (x1 - x0, y1 - y0, x - x0, y - y0);
            let k = ((px * ex + py * ey) / (ex * ex + ey * ey).max(1e-6)).clamp(0.0, 1.0);
            d = d.min((px - ex * k).hypot(py - ey * k));
            if (y0 > y) != (y1 > y) && x < x0 + (y - y0) / (y1 - y0) * (x1 - x0) {
                dentro = !dentro;
            }
        }
        if dentro { -d } else { d }
    };
    match f.tipo {
        TipoForma::Rectangulo => {
            let r = a.min(b) * (f.extra / 100.0).clamp(0.0, 1.0);
            let (qx, qy) = (x.abs() - a + r, y.abs() - b + r);
            qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
        }
        TipoForma::Circulo => {
            let k = (x / a).hypot(y / b);
            (k - 1.0) * a.min(b)
        }
        TipoForma::Triangulo => poligono(&[[0.0, -b], [a, b], [-a, b]]),
        TipoForma::Estrella => {
            let n = (5.0 + f.extra / 10.0).round() as usize;
            let v: Vec<[f32; 2]> = (0..n * 2)
                .map(|i| {
                    let g = std::f32::consts::PI * i as f32 / n as f32 - std::f32::consts::FRAC_PI_2;
                    let r = if i % 2 == 0 { 1.0 } else { 0.45 };
                    [g.cos() * r * a, g.sin() * r * b]
                })
                .collect();
            poligono(&v)
        }
        TipoForma::Corazon => {
            let v: Vec<[f32; 2]> = (0..64)
                .map(|i| {
                    let t = std::f32::consts::TAU * i as f32 / 64.0;
                    let s = t.sin();
                    [s * s * s * a, -(13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos() + 2.5) / 14.5 * b]
                })
                .collect();
            poligono(&v)
        }
        TipoForma::Flecha => poligono(&[[-a, -b * 0.4], [a * 0.2, -b * 0.4], [a * 0.2, -b], [a, 0.0], [a * 0.2, b], [a * 0.2, b * 0.4], [-a, b * 0.4]]),
    }
}

pub fn dibujar(f: &Forma, w: usize, h: usize) -> Image {
    let mut img = Image::new(w as u32, h as u32);
    let (a, b) = ((f.tam[0] / 200.0 * h as f32).max(0.5), (f.tam[1] / 200.0 * h as f32).max(0.5));
    let linea = f.linea / 100.0 * h as f32;
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    img.data.par_chunks_mut(w.max(1)).enumerate().for_each(|(y, fila)| {
        let py = y as f32 + 0.5 - cy;
        if py.abs() > b + linea + 2.0 {
            return;
        }
        for (x, q) in fila.iter_mut().enumerate() {
            let px = x as f32 + 0.5 - cx;
            if px.abs() > a + linea + 2.0 {
                continue;
            }
            let mut d = distancia(f, px, py, a, b);
            if linea > 0.0 {
                d = d.abs() - linea / 2.0;
            }
            let k = (0.5 - d).clamp(0.0, 1.0) * f.color[3];
            if k > 0.0 {
                *q = [f.color[0] * k, f.color[1] * k, f.color[2] * k, k];
            }
        }
    });
    img
}
