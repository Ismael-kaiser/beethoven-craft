// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Piezas maquinadas para las perillas y las interfaces de Lua. Todo se alumbra desde el mismo lado (arriba, un poco a
//! la izquierda): cada canto se aclara u oscurece según hacia dónde mira.
use super::metal::mezcla;
use eframe::egui::*;
use eframe::epaint::{RectShape, Shadow, Vertex, WHITE_UV};
use std::f32::consts::{PI, TAU};

/// De dónde viene la luz.
const LUZ: Vec2 = vec2(-0.42, -0.9);

/// Qué tanto mira a la luz: de -1 a 1.
fn da(u: Vec2) -> f32 {
    u.x * LUZ.x + u.y * LUZ.y
}

/// Luz (blanca) o sombra (negra) encima de lo ya pintado.
fn velo(t: f32, fuerza: f32) -> Color32 {
    let a = (t.abs() * fuerza).min(255.0) as u8;
    if t > 0.0 { Color32::from_white_alpha(a) } else { Color32::from_black_alpha(a) }
}

fn punto(m: &mut Mesh, q: Pos2, c: Color32) {
    m.vertices.push(Vertex { pos: q, uv: WHITE_UV, color: c });
}

/// Sombra suave en el piso.
pub fn sombra(p: &Painter, r: Rect, rad: f32, baja: f32, borrosa: f32, a: u8) {
    let s = Shadow { offset: [0, baja.min(40.0) as i8], blur: borrosa.min(60.0) as u8, spread: 0, color: Color32::from_black_alpha(a) };
    p.add(s.as_shape(r, CornerRadius::same(rad.min(200.0) as u8)));
}

/// Aro de `r0` a `r1`, con el color de cada orilla según el ángulo. La de afuera se suaviza si es la última.
fn aro(m: &mut Mesh, o: Pos2, r0: f32, r1: f32, n: usize, dentro: impl Fn(f32) -> Color32, fuera: impl Fn(f32) -> Color32, suave: bool) {
    let radios = if suave { 3 } else { 2 };
    let base = m.vertices.len() as u32;
    for i in 0..=n {
        let a = TAU * i as f32 / n as f32;
        let u = Vec2::angled(a);
        punto(m, o + u * r0, dentro(a));
        punto(m, o + u * (r1 - if suave { 0.5 } else { 0.0 }), fuera(a));
        if suave {
            punto(m, o + u * (r1 + 0.5), Color32::TRANSPARENT);
        }
    }
    for i in 0..n as u32 {
        for k in 0..radios - 1 {
            let (x, y) = (base + i * radios + k, base + (i + 1) * radios + k);
            m.indices.extend([x, x + 1, y, x + 1, y + 1, y]);
        }
    }
}

/// Brillo del metal torneado: dos reflejos opuestos, como en la tapa de un reloj.
fn torneado(a: f32) -> f32 {
    let luz = LUZ.angle();
    let reflejo = |giro: f32, filo: i32| (0.5 + 0.5 * (2.0 * (a - luz - giro)).cos()).powi(filo);
    (0.16 + 0.72 * reflejo(0.0, 3) + 0.3 * reflejo(1.15, 8) + 0.14 * reflejo(-0.8, 12)).min(1.0)
}

/// Contorno con las mismas cuentas sin importar el tamaño, y hacia dónde mira cada punto.
fn canto(r: Rect, rad: f32) -> Vec<(Pos2, Vec2)> {
    let rad = rad.min(r.width().min(r.height()) / 2.0).max(0.0);
    let mut v = Vec::with_capacity(28);
    for (x, y, a0) in [
        (r.left() + rad, r.top() + rad, PI),
        (r.right() - rad, r.top() + rad, 1.5 * PI),
        (r.right() - rad, r.bottom() - rad, 0.0),
        (r.left() + rad, r.bottom() - rad, 0.5 * PI),
    ] {
        for i in 0..=6 {
            let u = Vec2::angled(a0 + 0.5 * PI * i as f32 / 6.0);
            v.push((pos2(x, y) + u * rad, u));
        }
    }
    v
}

/// Canto biselado de `g` puntos por dentro de `r`. Saliente: luz arriba y sombra abajo; hundido, al revés.
pub fn bisel(p: &Painter, r: Rect, rad: f32, g: f32, hundido: bool, fuerza: f32) {
    if r.width() < g * 2.0 + 1.0 || r.height() < g * 2.0 + 1.0 {
        return;
    }
    let (a, b) = (canto(r, rad), canto(r.shrink(g), (rad - g).max(0.0)));
    let mut m = Mesh::default();
    for (x, y) in a.iter().zip(&b) {
        let t = da(x.1) * if hundido { -1.0 } else { 1.0 };
        punto(&mut m, x.0, velo(t, fuerza));
        punto(&mut m, y.0, velo(t, fuerza * 0.75));
    }
    let n = a.len() as u32;
    for i in 0..n {
        let (x, y) = (i * 2, ((i + 1) % n) * 2);
        m.indices.extend([x, x + 1, y, x + 1, y + 1, y]);
    }
    p.add(Shape::mesh(m));
}

/// Franja que va de un color a otro, de lado a lado o de arriba a abajo.
fn franja(p: &Painter, r: Rect, a: Color32, b: Color32, acostada: bool) {
    let mut m = Mesh::default();
    let (tr, bl) = if acostada { (b, a) } else { (a, b) };
    punto(&mut m, r.left_top(), a);
    punto(&mut m, r.right_top(), tr);
    punto(&mut m, r.right_bottom(), b);
    punto(&mut m, r.left_bottom(), bl);
    m.indices.extend([0, 1, 2, 0, 2, 3]);
    p.add(Shape::mesh(m));
}

/// El cepillado del acero: se dibuja una vez y se repite.
fn cepillado(ctx: &Context) -> TextureHandle {
    let id = Id::new("acero-cepillado");
    if let Some(t) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return t;
    }
    const N: usize = 256;
    let mut s = 0x1234_5678u32;
    let mut azar = || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as f32 / u32::MAX as f32 - 0.5
    };
    let mut px = Vec::with_capacity(N * N);
    for _ in 0..N {
        let renglon = azar();
        let crudo: Vec<f32> = (0..N).map(|_| azar()).collect();
        // Rayas largas: el promedio de muchos vecinos, dando la vuelta para que no se note dónde se repite.
        let mut suma: f32 = crudo[..40].iter().sum();
        for x in 0..N {
            let v = renglon * 0.55 + suma / 40.0 * 3.2 + crudo[x] * 0.1;
            let a = (v.abs() * 62.0).min(70.0) as u8;
            px.push(if v > 0.0 { Color32::from_white_alpha(a) } else { Color32::from_black_alpha(a) });
            suma += crudo[(x + 40) % N] - crudo[x];
        }
    }
    let t = ctx.load_texture("acero", ColorImage::new([N, N], px), TextureOptions::LINEAR_REPEAT);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
}

/// Cubre con el cepillado.
fn cepillar(p: &Painter, r: Rect, rad: f32, k: f32, fuerza: f32) {
    let t = cepillado(p.ctx());
    let uv = Rect::from_min_size(pos2(0.0, 0.0), r.size() / (256.0 * k));
    p.add(RectShape::filled(r, CornerRadius::same(rad as u8), Color32::WHITE.gamma_multiply(fuerza)).with_texture(t.id(), uv));
}

/// Tornillo avellanado: agujero con su canto, cabeza torneada y ranura en cruz.
pub fn tornillo(p: &Painter, o: Pos2, r: f32, giro: f32) {
    let gris = |v: f32| Color32::from_gray((v * 255.0).clamp(0.0, 255.0) as u8);
    let mut m = Mesh::default();
    let hoyo = |a: f32| velo(-da(Vec2::angled(a)), 190.0);
    aro(&mut m, o, r * 1.0, r * 1.55, 28, |_| Color32::from_black_alpha(190), hoyo, true);
    aro(&mut m, o, 0.0, r * 0.8, 28, |a| gris(0.2 + 0.62 * torneado(a + giro)), |a| gris(0.24 + 0.6 * torneado(a + giro)), false);
    aro(&mut m, o, r * 0.8, r, 28, |a| gris(0.5 + 0.4 * da(Vec2::angled(a))), |a| gris(0.36 + 0.34 * da(Vec2::angled(a))), true);
    p.add(Shape::mesh(m));
    for g in [giro, giro + PI / 2.0] {
        let d = Vec2::angled(g) * r * 0.62;
        p.line_segment([o - d + vec2(0.0, 0.7), o + d + vec2(0.0, 0.7)], Stroke::new((r * 0.34).max(0.8), Color32::from_white_alpha(70)));
        p.line_segment([o - d, o + d], Stroke::new((r * 0.3).max(0.8), Color32::from_black_alpha(220)));
    }
}

/// Panel grueso de acero cepillado: sombra, bisel ancho, reflejo que lo cruza y tornillos avellanados.
pub fn acero(p: &Painter, r: Rect, rad: f32, tinte: Color32, k: f32) {
    let cr = rad as u8;
    super::metal::degradado(p, r, cr, mezcla(tinte, Color32::WHITE, 0.16), mezcla(tinte, Color32::BLACK, 0.4));
    let d = r.shrink(2.5 * k);
    // El reflejo: una banda clara que no llega a las orillas.
    let (c, medio) = (Color32::from_white_alpha(34), d.left() + d.width() * 0.38);
    let pc = p.with_clip_rect(d.intersect(p.clip_rect()));
    franja(&pc, Rect::from_min_max(pos2(d.left() + rad, d.top()), pos2(medio, d.bottom())), Color32::TRANSPARENT, c, true);
    franja(&pc, Rect::from_min_max(pos2(medio, d.top()), pos2(d.right() - rad, d.bottom())), c, Color32::TRANSPARENT, true);
    cepillar(p, r, rad, k, 1.0);
    bisel(p, r, rad, 3.0 * k, false, 150.0);
    // El filo donde termina el bisel.
    p.rect_stroke(r.shrink(3.0 * k), (rad - 3.0 * k).max(0.0), Stroke::new(1.0, Color32::from_black_alpha(46)), StrokeKind::Inside);
    p.rect_stroke(r, rad, Stroke::new(1.0, Color32::from_black_alpha(210)), StrokeKind::Inside);
    if r.width() > 90.0 * k && r.height() > 60.0 * k {
        let m = 9.0 * k;
        let esquinas = [r.left_top() + vec2(m, m), r.right_top() + vec2(-m, m), r.left_bottom() + vec2(m, -m), r.right_bottom() + vec2(-m, -m)];
        for (j, o) in esquinas.into_iter().enumerate() {
            tornillo(p, o, 3.2 * k, 0.5 + j as f32 * 1.9);
        }
    }
}

/// Sombra que cae dentro de algo hundido.
fn adentro(p: &Painter, r: Rect, rad: f32, k: f32, a: u8) {
    let pc = p.with_clip_rect(r.intersect(p.clip_rect()));
    let (n, t) = (Color32::from_black_alpha(a), Color32::TRANSPARENT);
    franja(&pc, Rect::from_min_max(r.min + vec2(rad * 0.3, 0.0), pos2(r.right() - rad * 0.3, r.top() + 10.0 * k)), n, t, false);
    franja(&pc, Rect::from_min_max(r.min + vec2(0.0, rad * 0.3), pos2(r.left() + 6.0 * k, r.bottom() - rad * 0.3)), n.gamma_multiply(0.7), t, true);
}

/// Charola fresada en el panel: baja un escalón, con fondo de metal mate.
pub fn bandeja(p: &Painter, r: Rect, rad: f32, tinte: Color32, k: f32) {
    bisel(p, r.expand(2.0 * k), rad + 2.0 * k, 2.0 * k, true, 170.0);
    super::metal::degradado(p, r, rad as u8, mezcla(tinte, Color32::BLACK, 0.35), mezcla(tinte, Color32::WHITE, 0.07));
    cepillar(p, r, rad, k * 0.8, 0.5);
    adentro(p, r, rad, k, 130);
    p.rect_stroke(r, rad, Stroke::new(1.0, Color32::from_black_alpha(170)), StrokeKind::Inside);
}

/// Pantalla: marco biselado, vidrio oscuro y su reflejo.
pub fn pantalla(p: &Painter, r: Rect, rad: f32, k: f32) {
    let marco = r.expand(3.5 * k);
    p.rect_filled(marco, rad + 3.5 * k, Color32::from_gray(0x1b));
    bisel(p, marco, rad + 3.5 * k, 2.2 * k, false, 95.0);
    bisel(p, r.expand(1.3 * k), rad + 1.3 * k, 1.3 * k, true, 190.0);
    super::metal::degradado(p, r, rad as u8, Color32::from_rgb(0x05, 0x06, 0x07), Color32::from_rgb(0x14, 0x16, 0x19));
    adentro(p, r, rad, k, 160);
    let pc = p.with_clip_rect(r.intersect(p.clip_rect()));
    let (w, h) = (r.width(), r.height());
    let q = vec![
        r.left_top() + vec2(w * 0.04, 2.0),
        r.left_top() + vec2(w * 0.46, 2.0),
        r.left_top() + vec2(w * 0.34, h - 2.0),
        r.left_top() + vec2(w * 0.01, h - 2.0),
    ];
    pc.add(Shape::convex_polygon(q, Color32::from_white_alpha(7), Stroke::NONE));
}

/// Letras fresadas y pintadas: sombra en el filo de arriba y luz en el de abajo.
pub fn grabado(p: &Painter, o: Pos2, a: Align2, s: &str, n: f32, c: Color32) {
    let f = FontId::proportional(n);
    let d = (n * 0.045).clamp(0.6, 1.6);
    p.text(o + vec2(0.0, d), a, s, f.clone(), Color32::from_white_alpha(64));
    p.text(o - vec2(0.0, d), a, s, f.clone(), Color32::from_black_alpha(190));
    p.text(o, a, s, f, c);
}

/// Tecla de metal: canto biselado y cara un poco hundida. Prendida brilla con su color.
pub fn tecla(p: &Painter, r: Rect, rad: f32, c: Color32, si: bool, encima: bool, apretada: bool, k: f32) {
    let baja = si || apretada;
    if !baja {
        sombra(p, r, rad, 2.5 * k, 5.0 * k, 170);
    }
    p.rect_filled(r.expand(1.2 * k), rad + 1.2 * k, Color32::from_black_alpha(220));
    let base = if si { c } else { Color32::from_gray(if encima { 0x58 } else { 0x4b }) };
    if si {
        for (e, a) in [(7.0, 0.05), (4.5, 0.08), (2.5, 0.12)] {
            p.rect_filled(r.expand(e * k), rad + e * k, c.gamma_multiply(a));
        }
    }
    super::metal::degradado(p, r, rad as u8, mezcla(base, Color32::WHITE, 0.2), mezcla(base, Color32::BLACK, 0.45));
    let g = (2.2 * k).min(r.height() * 0.2);
    bisel(p, r, rad, g, baja && !si, 150.0);
    let cara = r.shrink(g);
    let rr = (rad - g).max(0.0);
    // La cara va al revés que el canto: por eso se ve cóncava.
    super::metal::degradado(
        p,
        cara,
        rr as u8,
        mezcla(base, Color32::BLACK, if si { 0.1 } else { 0.28 }),
        mezcla(base, Color32::WHITE, if si { 0.3 } else { 0.1 }),
    );
    if !si {
        cepillar(p, cara, rr, k * 0.6, 0.45);
    }
    p.rect_stroke(cara, rr, Stroke::new(1.0, Color32::from_black_alpha(60)), StrokeKind::Inside);
}

/// Foco con aro de cromo y cristal abombado.
pub fn foco(p: &Painter, o: Pos2, r: f32, c: Color32, v: f32) {
    for (e, a) in [(3.4, 0.05), (2.5, 0.09), (1.8, 0.14)] {
        p.circle_filled(o, r * e, c.gamma_multiply(a * v));
    }
    let gris = |v: f32| Color32::from_gray((v * 255.0).clamp(0.0, 255.0) as u8);
    let (claro, oscuro) = (mezcla(c, Color32::WHITE, 0.65 * v), mezcla(c, Color32::BLACK, 0.8 - 0.62 * v));
    let mut m = Mesh::default();
    aro(&mut m, o, r * 1.2, r * 1.6, 24, |_| Color32::from_black_alpha(170), |a| velo(-da(Vec2::angled(a)), 150.0), true);
    aro(&mut m, o, r, r * 1.3, 24, |a| gris(0.4 - 0.34 * da(Vec2::angled(a))), |a| gris(0.5 + 0.44 * da(Vec2::angled(a))), true);
    aro(
        &mut m,
        o,
        0.0,
        r,
        24,
        |_| mezcla(oscuro, claro, 0.04 + 0.8 * v),
        |a| mezcla(oscuro, claro, (0.06 + 0.14 * v) * (1.0 - da(Vec2::angled(a)))),
        false,
    );
    p.add(Shape::mesh(m));
    p.circle_filled(o + LUZ * r * 0.42, r * 0.26, Color32::from_white_alpha(90 + (130.0 * v) as u8));
}

/// La perilla: falda estriada que gira, chaflán, tapa torneada y su raya de color.
pub fn tapa(p: &Painter, o: Pos2, d: f32, ang: f32, metal: Color32, raya: Color32, encima: bool) {
    let metal = if encima { mezcla(metal, Color32::WHITE, 0.14) } else { metal };
    let (claro, oscuro) = (mezcla(metal, Color32::WHITE, 0.72), mezcla(metal, Color32::BLACK, 0.72));
    let tono = |t: f32| mezcla(oscuro, claro, t.clamp(0.0, 1.0));
    let caja = Rect::from_center_size(o, Vec2::splat(d * 2.0));
    sombra(p, caja, d, d * 0.32, d * 0.7, 210);
    sombra(p, caja, d, 2.0, 4.0, 200);
    let mut m = Mesh::default();
    // Las estrías: cada diente tiene un lado en luz y el otro en sombra.
    let dientes = ((d * 1.1) as usize).clamp(9, 44);
    let estria = |a: f32, orilla: f32| {
        let lado = ((a - ang) * dientes as f32).sin();
        tono(0.3 + 0.22 * da(Vec2::angled(a)) + 0.26 * lado - orilla)
    };
    let n = dientes * 6;
    aro(&mut m, o, d * 0.8, d, n, |a| estria(a, 0.0), |a| estria(a, 0.2), true);
    // Ranura entre la falda y la tapa, y el chaflán.
    aro(&mut m, o, d * 0.76, d * 0.82, 64, |_| tono(0.0), |_| tono(0.02), false);
    aro(&mut m, o, d * 0.66, d * 0.77, 64, |a| tono(0.42 + 0.4 * da(Vec2::angled(a))), |a| tono(0.5 + 0.56 * da(Vec2::angled(a))), false);
    // La cara.
    aro(&mut m, o, 0.0, d * 0.66, 72, |a| tono(0.1 + 0.62 * torneado(a)), |a| tono(0.14 + if d < 13.0 { 0.45 } else { 0.74 } * torneado(a)), false);
    p.add(Shape::mesh(m));
    if d > 11.0 {
        // Las marcas del torno.
        let mut r = d * 0.1;
        while r < d * 0.64 {
            p.circle_stroke(o, r, Stroke::new(0.6, Color32::from_black_alpha(26)));
            p.circle_stroke(o, r + 0.8, Stroke::new(0.5, Color32::from_white_alpha(16)));
            r += (d * 0.075).max(2.0);
        }
    }
    let u = Vec2::angled(ang);
    // En las chicas la raya va blanca y más larga: si no, no se lee.
    let chica = d < 13.0;
    let raya = if chica { Color32::from_gray(0xf4) } else { raya };
    let (a, b) = (o + u * d * if chica { 0.1 } else { 0.26 }, o + u * d * 0.95);
    let g = (d * 0.11).clamp(1.6, 4.5);
    p.line_segment([a + vec2(0.0, 0.8), b + vec2(0.0, 0.8)], Stroke::new(g + 1.6, Color32::from_white_alpha(60)));
    p.line_segment([a, b], Stroke::new(g + 1.4, Color32::from_black_alpha(230)));
    p.line_segment([a, b], Stroke::new(g * 0.62, mezcla(raya, Color32::WHITE, 0.25)));
}

/// El canal donde va la luz de la perilla, con sus marcas.
pub fn canal(p: &Painter, o: Pos2, r: f32, g: f32) {
    let mut m = Mesh::default();
    aro(&mut m, o, r - g * 0.5, r + g * 0.5, 48, |_| Color32::from_black_alpha(210), |_| Color32::from_black_alpha(210), true);
    aro(&mut m, o, r + g * 0.5, r + g * 0.5 + 1.2, 48, |a| velo(-da(Vec2::angled(a)), 120.0), |_| Color32::TRANSPARENT, false);
    p.add(Shape::mesh(m));
}

/// Riel y tapa de un tirador.
pub fn corredera(p: &Painter, r: Rect, a: f32, c: Color32, encima: bool, k: f32) {
    let parado = r.height() > r.width();
    let util = if parado { r.height() } else { r.width() } - 22.0 * k;
    let centro = if parado { pos2(r.center().x, r.bottom() - 11.0 * k - a * util) } else { pos2(r.left() + 11.0 * k + a * util, r.center().y) };
    let riel = Rect::from_center_size(r.center(), if parado { vec2(7.0 * k, r.height()) } else { vec2(r.width(), 7.0 * k) });
    let raya = |x0: Pos2, x1: Pos2, g: f32, c: Color32| p.line_segment([x0, x1], Stroke::new(g, c));
    for i in 0..=8 {
        let t = 11.0 * k + util * i as f32 / 8.0;
        let l = if i % 4 == 0 { 0.5 } else { 0.38 };
        for lado in [-1.0, 1.0] {
            let col = Color32::from_white_alpha(if i % 4 == 0 { 90 } else { 46 });
            if parado {
                let y = r.bottom() - t;
                raya(pos2(r.center().x + lado * 6.0 * k, y), pos2(r.center().x + lado * r.width() * l, y), 1.0, col);
            } else {
                let x = r.left() + t;
                raya(pos2(x, r.center().y + lado * 6.0 * k), pos2(x, r.center().y + lado * r.height() * l), 1.0, col);
            }
        }
    }
    bisel(p, riel.expand(1.5 * k), 5.0 * k, 1.5 * k, true, 190.0);
    p.rect_filled(riel, 3.5 * k, Color32::from_gray(0x06));
    let lleno =
        if parado { Rect::from_min_max(pos2(riel.left(), centro.y), riel.max) } else { Rect::from_min_max(riel.min, pos2(centro.x, riel.bottom())) };
    p.rect_filled(lleno.shrink(2.0 * k), 2.0, c);
    p.rect_filled(lleno, 3.0, c.gamma_multiply(0.16));
    let t = Rect::from_center_size(centro, if parado { vec2(r.width() * 0.92, 22.0 * k) } else { vec2(22.0 * k, r.height() * 0.92) });
    sombra(p, t, 4.0 * k, 5.0 * k, 9.0 * k, 200);
    let base = Color32::from_gray(if encima { 0x92 } else { 0x7e });
    // La tapa es una cuña: sube, tiene una hendidura a la mitad y vuelve a bajar.
    let mitad = |x: Rect, arriba: Color32, abajo: Color32| franja(p, x, arriba, abajo, !parado);
    p.rect_filled(t, 3.0 * k, Color32::BLACK);
    let d = t.shrink(1.0);
    let (x, y) = if parado { d.split_top_bottom_at_fraction(0.5) } else { d.split_left_right_at_fraction(0.5) };
    mitad(x, mezcla(base, Color32::WHITE, 0.4), mezcla(base, Color32::BLACK, 0.55));
    mitad(y, mezcla(base, Color32::WHITE, 0.28), mezcla(base, Color32::BLACK, 0.68));
    for e in [0.16, 0.29, 0.71, 0.84] {
        let (a, b) = if parado {
            let y = d.top() + d.height() * e;
            (pos2(d.left() + 2.0, y), pos2(d.right() - 2.0, y))
        } else {
            let x = d.left() + d.width() * e;
            (pos2(x, d.top() + 2.0), pos2(x, d.bottom() - 2.0))
        };
        raya(a, b, 1.0, Color32::from_black_alpha(130));
        let s = if parado { vec2(0.0, 1.0) } else { vec2(1.0, 0.0) };
        raya(a + s, b + s, 1.0, Color32::from_white_alpha(60));
    }
    bisel(p, t, 3.0 * k, 1.5 * k, false, 150.0);
    let (a, b) = if parado { (d.left_center(), d.right_center()) } else { (d.center_top(), d.center_bottom()) };
    raya(a, b, 3.4 * k, Color32::from_black_alpha(220));
    raya(a, b, 1.8 * k, mezcla(c, Color32::WHITE, 0.2));
}

/// Interruptor de pie: tuerca, rosca y botón torneado que baja al pisarlo.
pub fn pisoton(p: &Painter, o: Pos2, r: f32, abajo: bool) {
    let gris = |v: f32| Color32::from_gray((v * 255.0).clamp(0.0, 255.0) as u8);
    sombra(p, Rect::from_center_size(o, Vec2::splat(r * 2.0)), r, r * 0.25, r * 0.6, 200);
    // La tuerca, de seis caras.
    let v: Vec<Pos2> = (0..6).map(|i| o + Vec2::angled(TAU * i as f32 / 6.0 + 0.2) * r * 1.22).collect();
    let mut m = Mesh::default();
    for i in 0..6 {
        let (a, b) = (v[i], v[(i + 1) % 6]);
        let u = ((a + (b - a) * 0.5) - o).normalized();
        let c = gris(0.46 + 0.36 * da(u));
        let j = m.vertices.len() as u32;
        punto(&mut m, o, c);
        punto(&mut m, a, c);
        punto(&mut m, b, gris(0.4 + 0.36 * da(u)));
        m.indices.extend([j, j + 1, j + 2]);
    }
    let q = if abajo { 0.74 } else { 0.8 };
    aro(&mut m, o, r * 0.8, r * 0.98, 48, |_| gris(0.08), |a| gris(0.55 + 0.4 * da(Vec2::angled(a))), true);
    aro(&mut m, o, r * q * 0.74, r * q, 48, |a| gris(0.5 + 0.2 * da(Vec2::angled(a))), |a| gris(0.4 + 0.46 * da(Vec2::angled(a))), true);
    let luz = if abajo { 0.5 } else { 0.72 };
    aro(&mut m, o, 0.0, r * q * 0.76, 48, |a| gris(0.22 + luz * torneado(a)), |a| gris(0.26 + luz * torneado(a)), false);
    p.add(Shape::mesh(m));
}

/// Palanca con su tuerca y su vástago cromado.
pub fn palanca(p: &Painter, o: Pos2, si: bool, k: f32) {
    let gris = |v: f32| Color32::from_gray((v * 255.0).clamp(0.0, 255.0) as u8);
    let r = 11.0 * k;
    let mut m = Mesh::default();
    aro(&mut m, o, 0.0, r, 32, |a| gris(0.2 + 0.5 * torneado(a)), |a| gris(0.5 + 0.42 * da(Vec2::angled(a))), true);
    aro(&mut m, o, 0.0, r * 0.55, 32, |_| gris(0.02), |a| gris(0.16 - 0.14 * da(Vec2::angled(a))), true);
    let punta = o + vec2(0.0, if si { -15.0 } else { 15.0 }) * k;
    let sombra_ = punta + vec2(2.0, 4.0) * k;
    p.add(Shape::mesh(m));
    p.line_segment([o + vec2(2.0, 3.0) * k, sombra_], Stroke::new(8.0 * k, Color32::from_black_alpha(90)));
    for (g, x, c) in [(8.0, 0.0, gris(0.22)), (5.6, -0.6, gris(0.56)), (2.0, -1.5, gris(0.95))] {
        p.line_segment([o + vec2(x, 0.0) * k, punta + vec2(x, 0.0) * k], Stroke::new(g * k, c));
    }
    let mut m = Mesh::default();
    aro(&mut m, punta, 0.0, 7.0 * k, 28, |_| gris(0.72), |a| gris(0.48 + 0.46 * da(Vec2::angled(a))), true);
    p.add(Shape::mesh(m));
    p.circle_filled(punta + LUZ * 2.6 * k, 2.0 * k, Color32::from_white_alpha(220));
}

/// Bulbo de vidrio sobre su base, con el filamento encendido según `v`.
pub fn valvula(p: &Painter, r: Rect, v: f32, k: f32) {
    let gris = |v: f32| Color32::from_gray((v * 255.0).clamp(0.0, 255.0) as u8);
    let (o, w) = (r.center(), r.width());
    let fuego = Color32::from_rgb(0xff, 0x8a, 0x2c);
    let medio = pos2(o.x, r.top() + r.height() * 0.58);
    for (e, a) in [(1.1, 0.05), (0.8, 0.09), (0.52, 0.14)] {
        p.circle_filled(medio, w * e, fuego.gamma_multiply(a * (0.25 + 0.75 * v)));
    }
    let base = Rect::from_min_max(pos2(r.left() + w * 0.06, r.bottom() - r.height() * 0.2), pos2(r.right() - w * 0.06, r.bottom()));
    sombra(p, base, 3.0 * k, 4.0 * k, 8.0 * k, 190);
    // Cuerpo de vidrio: domo arriba y costados derechos.
    let vidrio = Rect::from_min_max(pos2(r.left() + w * 0.12, r.top() + r.height() * 0.06), pos2(r.right() - w * 0.12, base.top() + 2.0));
    let rr = vidrio.width() / 2.0;
    super::metal::degradado(
        p,
        vidrio,
        rr as u8,
        Color32::from_rgba_unmultiplied(120, 132, 146, 120),
        Color32::from_rgba_unmultiplied(40, 24, 14, 170),
    );
    let pc = p.with_clip_rect(vidrio.intersect(p.clip_rect()));
    // Placas y filamento.
    let placa = Rect::from_center_size(pos2(o.x, medio.y), vec2(w * 0.42, r.height() * 0.4));
    super::metal::degradado(&pc, placa, 2, gris(0.34), gris(0.12));
    pc.rect_stroke(placa, 2.0, Stroke::new(1.0, gris(0.04)), StrokeKind::Inside);
    for d in [-0.28, 0.28] {
        pc.vline(o.x + w * d, placa.y_range().expand(4.0 * k), Stroke::new(1.6 * k, gris(0.5)));
    }
    for (g, c) in
        [(6.0, fuego.gamma_multiply(0.2 + 0.4 * v)), (3.0, fuego.gamma_multiply(0.4 + 0.6 * v)), (1.2, mezcla(fuego, Color32::WHITE, 0.2 + 0.6 * v))]
    {
        pc.vline(o.x, placa.y_range().shrink(placa.height() * 0.12), Stroke::new(g * k, c));
    }
    // El getter, plateado, y los reflejos del vidrio.
    pc.rect_filled(Rect::from_min_size(vidrio.min, vec2(vidrio.width(), rr * 0.5)), rr, Color32::from_white_alpha(40));
    pc.rect_filled(
        Rect::from_min_size(vidrio.min + vec2(vidrio.width() * 0.12, rr * 0.7), vec2(vidrio.width() * 0.1, vidrio.height() * 0.6)),
        4.0,
        Color32::from_white_alpha(62),
    );
    pc.rect_filled(
        Rect::from_min_size(vidrio.right_top() + vec2(-vidrio.width() * 0.16, rr * 0.8), vec2(vidrio.width() * 0.05, vidrio.height() * 0.5)),
        3.0,
        Color32::from_white_alpha(34),
    );
    p.rect_stroke(vidrio, rr, Stroke::new(1.3, Color32::from_white_alpha(120)), StrokeKind::Inside);
    super::metal::degradado(p, base, (3.0 * k) as u8, gris(0.2), gris(0.06));
    bisel(p, base, 3.0 * k, 1.5 * k, false, 120.0);
}

/// Carrete de cinta visto de frente.
pub fn carrete(p: &Painter, o: Pos2, r: f32, giro: f32, lleno: f32, k: f32) {
    let gris = |v: f32| Color32::from_gray((v * 255.0).clamp(0.0, 255.0) as u8);
    sombra(p, Rect::from_center_size(o, Vec2::splat(r * 2.0)), r, 4.0 * k, 12.0 * k, 190);
    let mut m = Mesh::default();
    aro(&mut m, o, 0.0, r * 0.98, 72, |a| gris(0.16 + 0.1 * torneado(a)), |a| gris(0.2 + 0.2 * torneado(a)), true);
    // La cinta enrollada: café, con sus vueltas.
    let cinta = r * (0.42 + 0.46 * lleno);
    let cafe = |t: f32| mezcla(Color32::from_rgb(0x1c, 0x0f, 0x08), Color32::from_rgb(0x8a, 0x58, 0x38), t.clamp(0.0, 1.0));
    aro(&mut m, o, r * 0.34, cinta, 72, |a| cafe(0.1 + 0.5 * torneado(a)), |a| cafe(0.16 + 0.6 * torneado(a)), true);
    p.add(Shape::mesh(m));
    let mut q = r * 0.4;
    while q < cinta {
        p.circle_stroke(o, q, Stroke::new(0.7, Color32::from_black_alpha(50)));
        q += 2.6 * k;
    }
    // Tres brazos de aluminio.
    for i in 0..3 {
        let a = TAU * (giro + i as f32 / 3.0);
        let (u, t) = (Vec2::angled(a), Vec2::angled(a + PI / 2.0));
        let v = vec![o + u * r * 0.2 + t * r * 0.12, o + u * r * 0.95 + t * r * 0.2, o + u * r * 0.95 - t * r * 0.2, o + u * r * 0.2 - t * r * 0.12];
        let luz = 0.42 + 0.2 * da(t);
        p.add(Shape::convex_polygon(v, gris(luz), Stroke::new(1.0, gris(0.1))));
        p.line_segment([o + u * r * 0.2 + t * r * 0.11, o + u * r * 0.94 + t * r * 0.18], Stroke::new(1.0, Color32::from_white_alpha(80)));
    }
    let mut m = Mesh::default();
    aro(&mut m, o, r * 0.9, r, 72, |a| gris(0.32 + 0.2 * da(Vec2::angled(a))), |a| gris(0.46 + 0.34 * da(Vec2::angled(a))), true);
    aro(&mut m, o, 0.0, r * 0.32, 48, |a| gris(0.22 + 0.6 * torneado(a)), |a| gris(0.36 + 0.5 * torneado(a)), true);
    p.add(Shape::mesh(m));
    for i in 0..3 {
        let u = Vec2::angled(TAU * (giro + i as f32 / 3.0 + 0.16));
        p.line_segment([o + u * r * 0.06, o + u * r * 0.2], Stroke::new(r * 0.07, Color32::from_black_alpha(220)));
    }
    tornillo(p, o, r * 0.07, giro * TAU);
}

/// Asa de aparato de estudio, parada.
pub fn asa(p: &Painter, r: Rect, k: f32) {
    let gris = |v: f32| Color32::from_gray((v * 255.0).clamp(0.0, 255.0) as u8);
    sombra(p, r.translate(vec2(3.0 * k, 3.0 * k)), r.width() / 2.0, 3.0 * k, 8.0 * k, 190);
    for y in [r.top() + r.width() * 0.5, r.bottom() - r.width() * 0.5] {
        tornillo(p, pos2(r.center().x, y), r.width() * 0.46, 0.8);
    }
    let barra = r.shrink2(vec2(r.width() * 0.14, r.width() * 0.5));
    let medio = barra.left() + barra.width() * 0.36;
    let pc = p.with_clip_rect(barra.intersect(p.clip_rect()));
    franja(&pc, Rect::from_min_max(barra.min, pos2(medio, barra.bottom())), gris(0.24), gris(0.95), true);
    franja(&pc, Rect::from_min_max(pos2(medio, barra.top()), barra.max), gris(0.95), gris(0.12), true);
    p.rect_stroke(barra, 1.0, Stroke::new(1.0, gris(0.08)), StrokeKind::Inside);
}
