// Beethoven Craft. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Transiciones propias de Beethoven Craft: corazón, telón, tinta, humo, rayo y panal. Son cuentas nuestras, hechas
//! aquí; no vienen de FilmCraft ni de otro programa. Mismo contrato que las demás: con `p = 0` es el cuadro que sale
//! (`a`), con `p = 1` el que entra (`b`), y cambia sin saltos.

use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

use super::{Tx, ease_in_out, fbm, hash, mid_mix, mix, paint, smoothstep, vnoise};
use crate::image::Image;

pub(super) fn apply(id: &str, t: &Tx) -> Option<Image> {
    Some(match id {
        "bc_corazon" => corazon(t),
        "bc_telon" => telon(t),
        "bc_tinta" => tinta(t),
        "bc_humo" => humo(t),
        "bc_rayo" => rayo(t),
        "bc_panal" => panal(t),
        _ => return None,
    })
}

/// Suma luz de color `c` con fuerza `k`.
#[inline]
fn luz(x: [f32; 4], c: [f32; 4], k: f32) -> [f32; 4] {
    [x[0] + c[0] * k, x[1] + c[1] * k, x[2] + c[2] * k, (x[3] + k).min(1.0)]
}

// ---- corazón

/// Qué tan lejos llega el corazón en cada dirección (desde su centro), en unidades de la curva clásica
/// `x = 16 sen³θ`, `y = 13 cos θ - 5 cos 2θ - 2 cos 3θ - cos 4θ`.
fn borde_corazon() -> &'static [f32; 720] {
    static T: OnceLock<[f32; 720]> = OnceLock::new();
    T.get_or_init(|| {
        let mut r = [0f32; 720];
        for i in 0..8192 {
            let th = TAU * i as f32 / 8192.0;
            let x = 16.0 * th.sin().powi(3);
            // Con y hacia abajo, como en la imagen, y el centro un poco abajo de la hendidura.
            let y = -(13.0 * th.cos() - 5.0 * (2.0 * th).cos() - 2.0 * (3.0 * th).cos() - (4.0 * th).cos()) + 2.0;
            let k = ((y.atan2(x) + PI) / TAU * 720.0) as usize % 720;
            r[k] = r[k].max((x * x + y * y).sqrt());
        }
        // Sin huecos entre muestras.
        for _ in 0..2 {
            let c = r;
            for k in 0..720 {
                if r[k] == 0.0 {
                    r[k] = c[(k + 719) % 720].max(c[(k + 1) % 720]);
                }
            }
        }
        r
    })
}

/// Un corazón que crece desde el centro y deja ver el que entra, con un brillo en su orilla.
fn corazon(t: &Tx) -> Image {
    let tabla = borde_corazon();
    let (cx, cy) = t.center();
    let (w, h) = (t.wf, t.hf);
    let lejos = [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)].iter().map(|(x, y): &(f32, f32)| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt()).fold(1.0, f32::max);
    // Al final cubre hasta la esquina más lejana aun por su lado más corto (la hendidura).
    let corto = tabla.iter().copied().fold(f32::MAX, f32::min).max(0.5);
    let s = lejos / corto * 1.05 * ease_in_out(t.p).powf(2.6);
    let suave = t.px("feather").max(1.0);
    let ancho = t.px("width").max(1.0);
    let col = t.color("color");
    let prende = smoothstep(0.0, 0.08, t.p) * (1.0 - smoothstep(0.85, 1.0, t.p));
    paint(t.w, t.h, |x, y| {
        let (dx, dy) = (x - cx, y - cy);
        let r = (dx * dx + dy * dy).sqrt();
        let k = ((dy.atan2(dx) + PI) / TAU * 720.0) as usize % 720;
        let d = r - s * tabla[k];
        let dentro = 1.0 - smoothstep(-suave, suave, d);
        let base = mix(t.pa(x, y), t.pb(x, y), dentro);
        let brillo = (-(d.abs()) / ancho).exp() * prende;
        luz(base, col, brillo * 0.9)
    })
}

// ---- telón

/// Un telón de terciopelo que se cierra sobre lo que sale y se abre sobre lo que entra.
fn telon(t: &Tx) -> Image {
    let p = t.p;
    let cierra = if p < 0.5 { ease_in_out(p * 2.0) } else { ease_in_out((1.0 - p) * 2.0) };
    let (w, h) = (t.wf, t.hf);
    let col = t.color("color");
    let pliegue = t.px("size").max(8.0) * (0.45 + 0.55 * cierra);
    let cambio = smoothstep(0.49, 0.51, p);
    paint(t.w, t.h, |x, y| {
        let base = mix(t.pa(x, y), t.pb(x, y), cambio);
        let v = y / h;
        // La orilla de cada lado: más adentro abajo, como tela que cuelga.
        let orilla = cierra * w * 0.5 * (1.0 + 0.05 * v * v) + 1.0;
        let (desde, lejos) = if x < w * 0.5 { (x, orilla - x) } else { (w - x, orilla - (w - x)) };
        if lejos > 0.0 {
            // Pliegues: luz y sombra que se juntan cuando el telón está abierto.
            let u = desde / pliegue;
            let pl = 0.5 + 0.5 * (u * TAU + 0.6 * (v * 3.0 + u).sin()).cos();
            let luzv = (0.38 + 0.62 * pl.powf(0.8)) * (0.7 + 0.3 * v) * (1.0 - 0.35 * (-lejos / 6.0).exp());
            let mut c = [col[0] * luzv, col[1] * luzv, col[2] * luzv, 1.0];
            // Un filo dorado en la orilla.
            let filo = (-(lejos / 2.5)).exp();
            c = luz(c, [1.0, 0.75, 0.3, 1.0], filo * 0.5);
            // Suave en la orilla.
            mix(base, c, smoothstep(0.0, 1.5, lejos))
        } else {
            // La sombra del telón sobre la imagen.
            let s = 1.0 - 0.5 * cierra.min(0.6) * (lejos / 28.0).exp();
            [base[0] * s, base[1] * s, base[2] * s, base[3]]
        }
    })
}

// ---- tinta

/// Manchas de tinta que se abren y dejan ver lo que entra, con su orilla oscura.
fn tinta(t: &Tx) -> Image {
    let (cx, cy) = (t.wf / 2.0, t.hf / 2.0);
    let lejos = (cx * cx + cy * cy).sqrt();
    let tam = t.px("size").max(10.0);
    let semilla = t.seed();
    let col = t.color("color");
    let umbral = 1.08 - 1.16 * ease_in_out(t.p);
    paint(t.w, t.h, |x, y| {
        let centro = 1.0 - ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() / lejos;
        let f = fbm(x / tam, y / tam, semilla) * 0.65 + 0.35 * centro;
        let k = smoothstep(umbral - 0.02, umbral + 0.02, f);
        let orilla = (-((f - umbral) / 0.035).powi(2)).exp();
        mix(mix(t.pa(x, y), t.pb(x, y), k), col, orilla * 0.85)
    })
}

// ---- humo

/// Una nube de humo que sube, tapa el cambio y se disipa.
fn humo(t: &Tx) -> Image {
    let p = t.p;
    let tapa = (1.0 - (2.0 * p - 1.0).abs()).powf(1.6) * t.frac("density").clamp(0.0, 1.0);
    let tam = 320.0 * t.unit().max(0.01);
    let semilla = t.seed();
    let col = t.color("color");
    let cambio = mid_mix(p);
    paint(t.w, t.h, |x, y| {
        let n = fbm(x / tam + p * 0.6, y / tam - p * 1.6, semilla);
        // Bocanadas sueltas al principio y al final; a la mitad, una nube que lo tapa casi todo.
        let d = smoothstep(0.86 - tapa * 0.72, 1.0 - tapa * 0.56, n) * tapa.sqrt();
        let sombra = 0.72 + 0.28 * n;
        let humo = [col[0] * sombra, col[1] * sombra, col[2] * sombra, 1.0];
        mix(mix(t.pa(x, y), t.pb(x, y), cambio), humo, d.clamp(0.0, 1.0))
    })
}

// ---- rayo

/// Un rayo cruza la imagen con su relámpago; en el destello, cambia.
fn rayo(t: &Tx) -> Image {
    let p = t.p;
    let (w, h) = (t.wf, t.hf);
    let semilla = t.seed();
    // El camino del rayo: una caminata al azar de arriba abajo.
    let n = 14;
    let mut nodos = Vec::with_capacity(n + 1);
    let mut x = w * (0.3 + 0.4 * hash(1, 2, semilla));
    for k in 0..=n {
        nodos.push(x);
        x = (x + (hash(k as i32, 7, semilla) - 0.5) * w * 0.14).clamp(w * 0.05, w * 0.95);
    }
    let ancho = t.px("width").max(1.0);
    let col = t.color("color");
    let destello = (-((p - 0.5) / 0.06).powi(2)).exp().max(0.6 * (-((p - 0.38) / 0.025).powi(2)).exp());
    let cambio = smoothstep(0.47, 0.53, p);
    let unidad = t.unit().max(0.01);
    paint(t.w, t.h, |x, y| {
        let v = (y / h * n as f32).clamp(0.0, n as f32 - 1e-3);
        let i = v as usize;
        let bx = nodos[i] + (nodos[i + 1] - nodos[i]) * (v - i as f32) + (vnoise(y / (9.0 * unidad), 3.0, semilla) - 0.5) * 14.0 * unidad;
        let d = (x - bx).abs();
        let nucleo = (-(d / ancho).powi(2)).exp();
        let brillo = (-d / (ancho * 9.0)).exp();
        let base = mix(t.pa(x, y), t.pb(x, y), cambio);
        let blanco = destello * 0.85;
        let c = [base[0] * (1.0 - blanco) + blanco, base[1] * (1.0 - blanco) + blanco, base[2] * (1.0 - blanco) + blanco, base[3].max(blanco)];
        luz(luz(c, col, brillo * destello * 0.8), [1.0; 4], nucleo * destello)
    })
}

// ---- panal

/// Celdas de panal que se abren del centro hacia afuera, con un filo de luz.
fn panal(t: &Tx) -> Image {
    let r = t.px("size").max(6.0);
    let (cx, cy) = (t.wf / 2.0, t.hf / 2.0);
    let lejos = (cx * cx + cy * cy).sqrt().max(1.0);
    let semilla = t.seed();
    let col = t.color("color");
    let s3 = 3f32.sqrt();
    paint(t.w, t.h, |x, y| {
        // A coordenadas de hexágonos (punta arriba) y al más cercano.
        let q = (s3 / 3.0 * x - y / 3.0) / r;
        let rr = 2.0 / 3.0 * y / r;
        let (mut qi, mut ri, si) = (q.round(), rr.round(), (-q - rr).round());
        let (dq, dr, ds) = ((qi - q).abs(), (ri - rr).abs(), (si + q + rr).abs());
        if dq > dr && dq > ds {
            qi = -ri - si;
        } else if dr > ds {
            ri = -qi - si;
        }
        let (hx, hy) = (r * s3 * (qi + ri / 2.0), r * 1.5 * ri);
        let (dx, dy) = ((x - hx).abs(), (y - hy).abs());
        let hd = (dx / (s3 / 2.0 * r)).max((dx / s3 + dy) / r);
        let retraso = 0.62 * ((hx - cx).powi(2) + (hy - cy).powi(2)).sqrt() / lejos + 0.13 * hash(qi as i32, ri as i32, semilla);
        let abre = smoothstep(retraso, retraso + 0.25, t.p);
        let k = 1.0 - smoothstep(abre * 1.03 - 0.02, abre * 1.03 + 0.02, hd);
        let filo = (-((hd - abre) / 0.07).powi(2)).exp() * 4.0 * abre * (1.0 - abre);
        luz(mix(t.pa(x, y), t.pb(x, y), k), col, filo * 0.8)
    })
}
