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

/// Tornillo de cabeza redonda, chico: sombrita, cabeza que toma la luz arriba y su ranura.
pub fn tornillo(p: &Painter, o: Pos2, r: f32, giro: f32) {
    let gris = |v: f32| Color32::from_gray((v * 255.0).clamp(0.0, 255.0) as u8);
    p.circle_filled(o + vec2(0.0, r * 0.35), r * 1.08, Color32::from_black_alpha(110));
    let mut m = Mesh::default();
    aro(&mut m, o, 0.0, r, 32, |a| gris(0.42 + 0.3 * torneado(a + giro)), |a| gris(0.3 + 0.36 * da(Vec2::angled(a)).max(-0.4)), true);
    p.add(Shape::mesh(m));
    p.circle_stroke(o, r - 0.4, Stroke::new(0.6, Color32::from_black_alpha(90)));
    let d = Vec2::angled(giro) * r * 0.66;
    p.line_segment([o - d + vec2(0.0, 0.6), o + d + vec2(0.0, 0.6)], Stroke::new((r * 0.24).max(0.7), Color32::from_white_alpha(50)));
    p.line_segment([o - d, o + d], Stroke::new((r * 0.24).max(0.7), Color32::from_black_alpha(200)));
}

/// Frente de aparato: metal pintado, mate, con un grano fino y la luz cayendo de arriba. Tornillos chicos en las esquinas.
pub fn acero(p: &Painter, r: Rect, rad: f32, tinte: Color32, k: f32) {
    let cr = rad as u8;
    sombra(p, r, rad, 3.0 * k, 10.0 * k, 160);
    super::metal::degradado(p, r, cr, mezcla(tinte, Color32::WHITE, 0.1), mezcla(tinte, Color32::BLACK, 0.28));
    cepillar(p, r, rad, k * 1.6, 0.32);
    // Viñeta: las orillas un poco más oscuras, como la pintura vieja.
    let pc = p.with_clip_rect(r.intersect(p.clip_rect()));
    let (n, t) = (Color32::from_black_alpha(46), Color32::TRANSPARENT);
    let w = (r.width() * 0.12).min(40.0 * k);
    franja(&pc, Rect::from_min_max(r.min, pos2(r.left() + w, r.bottom())), n, t, true);
    franja(&pc, Rect::from_min_max(pos2(r.right() - w, r.top()), r.max), t, n, true);
    franja(&pc, Rect::from_min_max(pos2(r.left(), r.bottom() - w), r.max), t, n, false);
    bisel(p, r, rad, 1.6 * k, false, 110.0);
    p.rect_stroke(r, rad, Stroke::new(1.0, Color32::from_black_alpha(200)), StrokeKind::Inside);
    if r.width() > 90.0 * k && r.height() > 60.0 * k {
        let m = 8.0 * k;
        let esquinas = [r.left_top() + vec2(m, m), r.right_top() + vec2(-m, m), r.left_bottom() + vec2(m, -m), r.right_bottom() + vec2(-m, -m)];
        for (j, o) in esquinas.into_iter().enumerate() {
            tornillo(p, o, 2.6 * k, 0.4 + j as f32 * 1.3);
        }
    }
}

/// Costado de nogal barnizado, como los aparatos de antes.
pub fn madera(p: &Painter, r: Rect, rad: f32, k: f32) {
    let base = Color32::from_rgb(0x6e, 0x40, 0x24);
    let cr = CornerRadius::same(rad as u8);
    p.add(eframe::epaint::RectShape::filled(r, cr, base));
    let pc = p.with_clip_rect(r.intersect(p.clip_rect()));
    // La veta: rayas largas que se ondulan, más claras y más oscuras.
    let mut s = 0x2545_F491u32 ^ (r.top() as u32).wrapping_mul(31) ^ (r.left() as u32).wrapping_mul(977);
    let mut azar = || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as f32 / u32::MAX as f32
    };
    let rayas = (r.width() / (1.6 * k)) as usize + 2;
    for i in 0..rayas {
        let x0 = r.left() + (i as f32 + azar() * 0.6) * 1.6 * k;
        let (fase, onda, luz) = (azar() * 6.3, 0.8 + azar() * 2.2, azar());
        let col = if luz > 0.55 {
            Color32::from_rgba_unmultiplied(0xd8, 0x9a, 0x62, (luz * 80.0) as u8)
        } else {
            Color32::from_rgba_unmultiplied(0x24, 0x10, 0x06, ((1.0 - luz) * 120.0) as u8)
        };
        let pasos = (r.height() / (6.0 * k)) as usize + 2;
        let v: Vec<Pos2> = (0..pasos)
            .map(|j| {
                let y = r.top() + r.height() * j as f32 / (pasos - 1) as f32;
                pos2(x0 + (y / (37.0 * k) + fase).sin() * onda * k + (y / (11.0 * k) + fase * 2.0).sin() * 0.35 * k, y)
            })
            .collect();
        pc.add(Shape::line(v, Stroke::new((0.6 + azar() * 1.1) * k, col)));
    }
    // Barniz: luz de un lado y sombra del otro, porque es redondeado.
    let canto = (3.0 * k).min(r.width() * 0.2);
    franja(&pc, Rect::from_min_max(r.min, pos2(r.left() + canto, r.bottom())), Color32::from_black_alpha(60), Color32::TRANSPARENT, true);
    franja(&pc, Rect::from_min_max(pos2(r.right() - canto, r.top()), r.max), Color32::TRANSPARENT, Color32::from_black_alpha(80), true);
    p.vline(r.left() + canto, r.y_range(), Stroke::new(1.0, Color32::from_white_alpha(22)));
    franja(&pc, Rect::from_min_max(r.min, pos2(r.right(), r.top() + 14.0 * k)), Color32::from_white_alpha(30), Color32::TRANSPARENT, false);
    p.rect_stroke(r, rad, Stroke::new(1.0, Color32::from_black_alpha(220)), StrokeKind::Inside);
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

/// Pantalla de tubo: bisel negro, vidrio un poco abombado, fondo verde muy oscuro y una rejilla tenue.
pub fn pantalla(p: &Painter, r: Rect, rad: f32, k: f32) {
    let marco = r.expand(3.0 * k);
    p.rect_filled(marco, rad + 3.0 * k, Color32::from_gray(0x14));
    bisel(p, marco, rad + 3.0 * k, 1.6 * k, false, 70.0);
    bisel(p, r.expand(1.0 * k), rad + 1.0 * k, 1.0 * k, true, 170.0);
    super::metal::degradado(p, r, rad as u8, Color32::from_rgb(0x0a, 0x10, 0x0d), Color32::from_rgb(0x04, 0x07, 0x06));
    let pc = p.with_clip_rect(r.intersect(p.clip_rect()));
    // El fósforo encendido en el centro, apagándose hacia las orillas.
    let (w, h) = (r.width(), r.height());
    for (e, a) in [(0.9, 10u8), (0.6, 8), (0.35, 6)] {
        pc.rect_filled(Rect::from_center_size(r.center(), vec2(w * e, h * e)), rad * 2.0, Color32::from_rgba_unmultiplied(0x40, 0x90, 0x60, a));
    }
    if h > 30.0 * k {
        let paso = (h / 4.0).max(10.0 * k);
        let mut y = r.top() + paso;
        while y < r.bottom() - 2.0 {
            pc.hline(r.x_range(), y, Stroke::new(0.6, Color32::from_rgba_unmultiplied(0x60, 0xc0, 0x90, 10)));
            y += paso;
        }
    }
    adentro(p, r, rad, k, 150);
    // Reflejo del vidrio, arriba a la izquierda.
    let q = vec![
        r.left_top() + vec2(w * 0.03, 2.0),
        r.left_top() + vec2(w * 0.4, 2.0),
        r.left_top() + vec2(w * 0.25, h * 0.45),
        r.left_top() + vec2(w * 0.02, h * 0.6),
    ];
    pc.add(Shape::convex_polygon(q, Color32::from_white_alpha(6), Stroke::NONE));
}

/// Letras fresadas y pintadas: sombra en el filo de arriba y luz en el de abajo.
pub fn grabado(p: &Painter, o: Pos2, a: Align2, s: &str, n: f32, c: Color32) {
    let f = FontId::proportional(n);
    let d = (n * 0.045).clamp(0.6, 1.6);
    p.text(o + vec2(0.0, d), a, s, f.clone(), Color32::from_white_alpha(64));
    p.text(o - vec2(0.0, d), a, s, f.clone(), Color32::from_black_alpha(190));
    p.text(o, a, s, f, c);
}

/// Tecla de aparato: cuerpo de baquelita mate con su canto. Prendida, una lámpara la alumbra por dentro.
pub fn tecla(p: &Painter, r: Rect, rad: f32, c: Color32, si: bool, encima: bool, apretada: bool, k: f32) {
    let rad = rad.min(4.0 * k);
    let baja = (apretada || si) as u8 as f32 * 1.0 * k;
    p.rect_filled(r.expand(1.5 * k), rad + 1.5 * k, Color32::from_black_alpha(170));
    if !apretada {
        sombra(p, r, rad, 2.0 * k, 3.0 * k, 150);
    }
    let r = r.translate(vec2(0.0, baja));
    if si {
        // Cristal de color, encendido de abajo: luz cálida y su halo.
        for (e, a) in [(5.0, 0.05), (3.0, 0.09)] {
            p.rect_filled(r.expand(e * k), rad + e * k, c.gamma_multiply(a));
        }
        super::metal::degradado(p, r, rad as u8, mezcla(c, Color32::WHITE, 0.42), mezcla(c, Color32::BLACK, 0.18));
        let brillo = Rect::from_center_size(r.center() + vec2(0.0, r.height() * 0.1), vec2(r.width() * 0.8, r.height() * 0.5));
        p.rect_filled(brillo, rad * 2.0, Color32::from_white_alpha(40));
    } else {
        let base = Color32::from_gray(if encima { 0x3c } else { 0x30 });
        super::metal::degradado(p, r, rad as u8, mezcla(base, Color32::WHITE, 0.1), mezcla(base, Color32::BLACK, 0.3));
    }
    // El canto de arriba toma la luz.
    let f = (rad * 0.8).max(1.5);
    p.hline(r.left() + f..=r.right() - f, r.top() + 0.8, Stroke::new(1.0, Color32::from_white_alpha(if si { 120 } else { 40 })));
    p.hline(r.left() + f..=r.right() - f, r.bottom() - 0.8, Stroke::new(1.0, Color32::from_black_alpha(90)));
    p.rect_stroke(r, rad, Stroke::new(1.0, Color32::from_black_alpha(200)), StrokeKind::Inside);
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

/// Interruptor de palanca: tuerca de seis caras, arandela y la palanca de cromo, que se inclina hacia arriba o abajo.
pub fn palanca(p: &Painter, o: Pos2, si: bool, k: f32) {
    let gris = |v: f32| Color32::from_gray((v * 255.0).clamp(0.0, 255.0) as u8);
    let r = 8.0 * k;
    p.circle_filled(o + vec2(0.0, 1.5 * k), r * 1.25, Color32::from_black_alpha(120));
    let mut m = Mesh::default();
    // La tuerca.
    let v: Vec<Pos2> = (0..6).map(|i| o + Vec2::angled(TAU * i as f32 / 6.0 + PI / 6.0) * r * 1.12).collect();
    for i in 0..6 {
        let (a, b) = (v[i], v[(i + 1) % 6]);
        let u = ((a + (b - a) * 0.5) - o).normalized();
        let j = m.vertices.len() as u32;
        punto(&mut m, o, gris(0.5 + 0.3 * da(u)));
        punto(&mut m, a, gris(0.38 + 0.4 * da(u)));
        punto(&mut m, b, gris(0.38 + 0.4 * da(u)));
        m.indices.extend([j, j + 1, j + 2]);
    }
    aro(&mut m, o, 0.0, r * 0.62, 24, |a| gris(0.3 + 0.5 * torneado(a)), |a| gris(0.25 + 0.4 * da(Vec2::angled(a))), true);
    p.add(Shape::mesh(m));
    // La palanca, más angosta en la base y redonda en la punta.
    let hacia = if si { -1.0 } else { 1.0 };
    let punta = o + vec2(1.5 * k, 15.0 * k * hacia);
    let ancho = |t: f32| (2.0 + 1.8 * t) * k;
    let largo = (punta - o).length();
    let u = (punta - o) / largo;
    let n = vec2(-u.y, u.x);
    p.line_segment([o + vec2(2.0, 3.0) * k, punta + vec2(3.0, 4.0) * k], Stroke::new(5.0 * k, Color32::from_black_alpha(80)));
    let cuerpo: Vec<Pos2> = [(0.0, -1.0), (1.0, -1.0), (1.0, 1.0), (0.0, 1.0)].iter().map(|(t, l)| o + u * largo * *t + n * ancho(*t) * *l).collect();
    p.add(Shape::convex_polygon(cuerpo, gris(0.55), Stroke::new(0.8, gris(0.2))));
    p.line_segment([o + n * ancho(0.0) * 0.4, punta + n * ancho(1.0) * 0.4], Stroke::new(1.3 * k, gris(0.95)));
    p.circle_filled(punta, ancho(1.0) * 1.1, gris(0.62));
    p.circle_filled(punta - vec2(0.8, 1.0) * k, ancho(1.0) * 0.5, gris(0.97));
    p.circle_stroke(punta, ancho(1.0) * 1.1, Stroke::new(0.7, gris(0.25)));
}

/// Lámpara piloto: aro de cromo y un cristal tallado que se enciende.
pub fn lampara(p: &Painter, o: Pos2, r: f32, c: Color32, v: f32) {
    let gris = |v: f32| Color32::from_gray((v * 255.0).clamp(0.0, 255.0) as u8);
    for (e, a) in [(3.2, 0.06), (2.2, 0.1), (1.6, 0.16)] {
        p.circle_filled(o, r * e, c.gamma_multiply(a * v));
    }
    p.circle_filled(o + vec2(0.0, r * 0.15), r * 1.45, Color32::from_black_alpha(150));
    let mut m = Mesh::default();
    aro(&mut m, o, r * 1.0, r * 1.38, 32, |a| gris(0.3 - 0.2 * da(Vec2::angled(a))), |a| gris(0.55 + 0.4 * da(Vec2::angled(a))), true);
    // El cristal tiene caras: cada una toma la luz distinto.
    let apagado = mezcla(c, Color32::BLACK, 0.72);
    let prendido = mezcla(c, Color32::WHITE, 0.25);
    let cara = |a: f32| mezcla(apagado, prendido, (v * (0.75 + 0.25 * ((a * 6.0).sin()))).clamp(0.0, 1.0));
    aro(&mut m, o, 0.0, r, 36, |_| mezcla(apagado, mezcla(c, Color32::WHITE, 0.6), v), cara, false);
    p.add(Shape::mesh(m));
    p.circle_filled(o + vec2(-0.35, -0.4) * r, r * 0.28, Color32::from_white_alpha(70 + (140.0 * v) as u8));
}

/// Bulbo: zócalo de baquelita con sus patas, vidrio redondo con su punta, placa gris, discos de mica y el filamento que se
/// pone naranja. `v` de 0 a 1 es qué tan caliente está.
pub fn valvula(p: &Painter, r: Rect, v: f32, k: f32) {
    let gris = |v: f32| Color32::from_gray((v * 255.0).clamp(0.0, 255.0) as u8);
    let (w, h) = (r.width(), r.height());
    let fuego = Color32::from_rgb(0xff, 0x7a, 0x22);
    // Zócalo.
    let zocalo = Rect::from_min_max(pos2(r.left() + w * 0.08, r.bottom() - h * 0.14), pos2(r.right() - w * 0.08, r.bottom()));
    sombra(p, zocalo, 4.0 * k, 3.0 * k, 7.0 * k, 200);
    cilindro(p, zocalo, 4.0 * k, Color32::from_rgb(0x2a, 0x22, 0x1c), 0.5);
    // El vidrio: cuerpo derecho, hombros redondos y la punta donde se selló.
    let vidrio = Rect::from_min_max(pos2(r.left() + w * 0.14, r.top() + h * 0.07), pos2(r.right() - w * 0.14, zocalo.top() + 1.0));
    let medio = vidrio.center();
    // Calor dentro del vidrio.
    let calor = pos2(medio.x, vidrio.top() + vidrio.height() * 0.62);
    for (e, a) in [(1.3, 0.06), (0.9, 0.1), (0.55, 0.18), (0.32, 0.26)] {
        p.circle_filled(calor, vidrio.width() * e, fuego.gamma_multiply(a * v));
    }
    let pc = p.with_clip_rect(vidrio.intersect(p.clip_rect()));
    // Lo de adentro primero, el vidrio después.
    let mica = |y: f32| {
        let q = Rect::from_center_size(pos2(medio.x, y), vec2(vidrio.width() * 0.8, 2.4 * k));
        pc.rect_filled(q, 1.0, Color32::from_rgba_unmultiplied(0xd8, 0xd0, 0xb8, 120));
    };
    let placa = Rect::from_min_max(
        pos2(medio.x - vidrio.width() * 0.26, vidrio.top() + vidrio.height() * 0.32),
        pos2(medio.x + vidrio.width() * 0.26, vidrio.bottom() - vidrio.height() * 0.1),
    );
    for d in [-0.3, 0.3] {
        pc.vline(medio.x + vidrio.width() * d, vidrio.top() + vidrio.height() * 0.26..=vidrio.bottom(), Stroke::new(1.0 * k, gris(0.55)));
    }
    cilindro(&pc, placa, 1.5 * k, Color32::from_gray(0x50), 0.7);
    pc.vline(medio.x, placa.y_range().shrink(placa.height() * 0.1), Stroke::new(2.4 * k, Color32::from_black_alpha(200)));
    // El filamento se ve por la ranura de la placa.
    for (g, a) in [(9.0, 0.22), (4.4, 0.5), (2.0, 1.0)] {
        let c = mezcla(fuego, Color32::from_rgb(0xff, 0xe6, 0xa8), (v * a).min(1.0) * 0.6);
        pc.vline(medio.x, placa.y_range().shrink(placa.height() * 0.16), Stroke::new(g * k, c.gamma_multiply((a * (0.15 + 0.85 * v)).min(1.0))));
    }
    mica(placa.top() - 2.0 * k);
    mica(placa.bottom() + 2.0 * k);
    // El espejito de arriba (el getter).
    let tapa = Rect::from_min_max(vidrio.min, pos2(vidrio.right(), vidrio.top() + vidrio.height() * 0.22));
    franja(&pc, tapa, Color32::from_rgba_unmultiplied(0xb8, 0xb8, 0xbc, 150), Color32::from_rgba_unmultiplied(0x70, 0x70, 0x78, 30), false);
    // El vidrio: transparente, más claro en las orillas por donde se dobla la luz.
    let rad = vidrio.width() / 2.0;
    let orilla = Color32::from_white_alpha(46);
    let w2 = vidrio.width() * 0.22;
    franja(&pc, Rect::from_min_max(vidrio.min, pos2(vidrio.left() + w2, vidrio.bottom())), orilla, Color32::TRANSPARENT, true);
    franja(&pc, Rect::from_min_max(pos2(vidrio.right() - w2, vidrio.top()), vidrio.max), Color32::TRANSPARENT, orilla, true);
    let brillo = Rect::from_min_size(vidrio.min + vec2(vidrio.width() * 0.2, rad * 0.5), vec2(vidrio.width() * 0.1, vidrio.height() * 0.62));
    pc.rect_filled(brillo, 3.0 * k, Color32::from_white_alpha(70));
    p.rect_stroke(vidrio, rad, Stroke::new(1.0, Color32::from_white_alpha(70)), StrokeKind::Inside);
    p.circle_filled(pos2(medio.x, vidrio.top() + 0.5), 2.2 * k, Color32::from_white_alpha(90));
}

/// Un cilindro visto de frente: oscuro en los lados, luz en el centro un poco a la izquierda.
fn cilindro(p: &Painter, r: Rect, rad: f32, base: Color32, brillo: f32) {
    let pc = p.with_clip_rect(r.intersect(p.clip_rect()));
    let (os, cl) = (mezcla(base, Color32::BLACK, 0.55), mezcla(base, Color32::WHITE, brillo * 0.6));
    let x1 = r.left() + r.width() * 0.38;
    p.rect_filled(r, rad, base);
    franja(&pc, Rect::from_min_max(r.min, pos2(x1, r.bottom())), os, cl, true);
    franja(&pc, Rect::from_min_max(pos2(x1, r.top()), r.max), cl, os, true);
    p.rect_stroke(r, rad, Stroke::new(1.0, Color32::from_black_alpha(160)), StrokeKind::Inside);
}

/// Pedazo de aro de `r0` a `r1` entre los ángulos `a0` y `a1`; el color según el radio y el ángulo.
fn sector(m: &mut Mesh, o: Pos2, r0: f32, r1: f32, a0: f32, a1: f32, n: usize, col: impl Fn(f32, f32) -> Color32) {
    let base = m.vertices.len() as u32;
    for i in 0..=n {
        let a = a0 + (a1 - a0) * i as f32 / n as f32;
        let u = Vec2::angled(a);
        punto(m, o + u * r0, col(r0, a));
        punto(m, o + u * r1, col(r1, a));
    }
    for i in 0..n as u32 {
        let (x, y) = (base + i * 2, base + i * 2 + 2);
        m.indices.extend([x, x + 1, y, x + 1, y + 1, y]);
    }
}

/// Carrete de cinta de estudio visto de frente: tapa de aluminio torneado con tres ventanas por donde se ve
/// la cinta enrollada, brillosa, y el centro con sus tres muescas. Todo gira con `giro`.
pub fn carrete(p: &Painter, o: Pos2, r: f32, giro: f32, lleno: f32, k: f32) {
    let gris = |v: f32| Color32::from_gray((v * 255.0).clamp(0.0, 255.0) as u8);
    sombra(p, Rect::from_center_size(o, Vec2::splat(r * 2.0)), r, 5.0 * k, 14.0 * k, 200);
    let g0 = TAU * giro;
    // La tapa: aluminio torneado, con su brillo que no gira (la luz está quieta).
    let mut m = Mesh::default();
    aro(&mut m, o, 0.0, r, 96, |a| gris(0.5 + 0.32 * torneado(a)), |a| gris(0.46 + 0.4 * torneado(a)), true);
    p.add(Shape::mesh(m));
    let mut q = r * 0.3;
    let mut par = false;
    while q < r * 0.94 {
        p.circle_stroke(o, q, Stroke::new(0.6, if par { Color32::from_white_alpha(14) } else { Color32::from_black_alpha(16) }));
        q += 2.2 * k;
        par = !par;
    }
    // Las tres ventanas: se ve la cinta (café, con brillo de laca) y, más allá, la oscuridad del aparato.
    let cinta = r * (0.36 + 0.5 * lleno);
    let (v0, v1) = (r * 0.3, r * 0.86);
    let cafe = |t: f32| mezcla(Color32::from_rgb(0x1a, 0x0d, 0x06), Color32::from_rgb(0x9a, 0x66, 0x40), t.clamp(0.0, 1.0));
    let ancho = 0.6;
    for i in 0..3 {
        let c = g0 + TAU * i as f32 / 3.0;
        let (a0, a1) = (c - ancho, c + ancho);
        let mut m = Mesh::default();
        sector(&mut m, o, v0, cinta.max(v0), a0, a1, 24, |rr, a| cafe(0.12 + 0.55 * torneado(a) * (0.6 + 0.4 * rr / cinta)));
        if cinta < v1 {
            sector(&mut m, o, cinta, v1, a0, a1, 24, |rr, _| gris(0.04 + 0.05 * (rr - cinta) / (v1 - cinta)));
        }
        p.add(Shape::mesh(m));
        // Las vueltas de la cinta y la orilla del rollo.
        let arco = |rr: f32| (0..=24).map(|j| o + Vec2::angled(a0 + (a1 - a0) * j as f32 / 24.0) * rr).collect::<Vec<_>>();
        let mut q = v0 + 2.0 * k;
        while q < cinta - 1.0 {
            p.add(Shape::line(arco(q), Stroke::new(0.6, Color32::from_black_alpha(40))));
            q += 2.8 * k;
        }
        if cinta < v1 {
            p.add(Shape::line(arco(cinta), Stroke::new(1.2, Color32::from_rgba_unmultiplied(0xd8, 0xa0, 0x70, 90))));
        }
        // El corte en el aluminio: sombra arriba, filo con luz abajo.
        let mut borde = arco(v1);
        borde.extend(arco(v0).into_iter().rev());
        borde.push(borde[0]);
        p.add(Shape::line(borde.clone(), Stroke::new(1.6 * k, Color32::from_black_alpha(150))));
        let filo: Vec<Pos2> = borde.iter().map(|q| *q + vec2(0.0, 0.8 * k)).collect();
        p.add(Shape::line(filo, Stroke::new(0.7 * k, Color32::from_white_alpha(46))));
    }
    // Canto doblado de la tapa.
    let mut m = Mesh::default();
    aro(&mut m, o, r * 0.94, r, 96, |a| gris(0.4 + 0.3 * da(Vec2::angled(a))), |a| gris(0.3 + 0.4 * da(Vec2::angled(a))), true);
    // El centro: anillo cromado y maza negra.
    aro(&mut m, o, r * 0.2, r * 0.27, 64, |a| gris(0.5 + 0.45 * torneado(a)), |a| gris(0.3 + 0.4 * torneado(a)), false);
    aro(&mut m, o, 0.0, r * 0.2, 48, |a| gris(0.1 + 0.08 * torneado(a)), |a| gris(0.16 + 0.12 * torneado(a)), false);
    p.add(Shape::mesh(m));
    p.circle_stroke(o, r * 0.27, Stroke::new(1.0, Color32::from_black_alpha(140)));
    for i in 0..3 {
        let u = Vec2::angled(g0 + TAU * (i as f32 / 3.0 + 1.0 / 6.0));
        p.line_segment([o + u * r * 0.07, o + u * r * 0.17], Stroke::new(r * 0.06, gris(0.02)));
        p.line_segment([o + u * r * 0.07 + vec2(0.0, 0.8), o + u * r * 0.17 + vec2(0.0, 0.8)], Stroke::new(0.6, Color32::from_white_alpha(30)));
    }
    tornillo(p, o, r * 0.06, g0);
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

/// Perilla de aparato de antes: escala impresa en el frente, falda negra de baquelita, tapa de aluminio y su raya.
/// `x` de 0 a 1. El color va en la raya y en el número de la escala.
pub fn perilla_vieja(p: &Painter, o: Pos2, d: f32, x: f32, c: Color32, metal: Color32, encima: bool, k: f32) {
    let ang = |x: f32| PI * (0.75 + 1.5 * x);
    let tinta = Color32::from_rgba_unmultiplied(0xf2, 0xec, 0xdc, 200);
    // La escala, como pintada en el frente: once rayas y la de donde está, de color.
    for i in 0..=10 {
        let t = i as f32 / 10.0;
        let u = Vec2::angled(ang(t));
        let (a, b) = (d * 1.12, d * if i % 5 == 0 { 1.34 } else { 1.24 });
        p.line_segment([o + u * a, o + u * b], Stroke::new(if i % 5 == 0 { 1.4 } else { 1.0 } * k, tinta));
    }
    let u = Vec2::angled(ang(x));
    p.line_segment([o + u * d * 1.1, o + u * d * 1.4], Stroke::new(2.4 * k, c.gamma_multiply(0.35)));
    p.line_segment([o + u * d * 1.1, o + u * d * 1.4], Stroke::new(1.4 * k, mezcla(c, Color32::WHITE, 0.3)));
    // Sombra en el frente.
    let caja = Rect::from_center_size(o, Vec2::splat(d * 2.0));
    sombra(p, caja, d, d * 0.28, d * 0.55, 200);
    let gris = |v: f32| Color32::from_gray((v * 255.0).clamp(0.0, 255.0) as u8);
    let negro = |t: f32| mezcla(Color32::from_rgb(0x0e, 0x0d, 0x0c), Color32::from_rgb(0x4a, 0x47, 0x44), t.clamp(0.0, 1.0));
    let mut m = Mesh::default();
    // La falda, con estrías suaves que giran con la perilla.
    let dientes = ((d * 1.0) as usize).clamp(14, 36);
    let estria = |a: f32, fondo: f32| negro(0.22 + 0.26 * da(Vec2::angled(a)) + 0.12 * ((a - ang(x)) * dientes as f32).sin() - fondo);
    aro(&mut m, o, d * 0.72, d, dientes * 6, |a| estria(a, 0.0), |a| estria(a, 0.12), true);
    // El hombro y la tapa de aluminio torneado.
    aro(&mut m, o, d * 0.62, d * 0.74, 64, |a| negro(0.1 + 0.4 * da(Vec2::angled(a)).max(0.0)), |a| negro(0.3 + 0.5 * da(Vec2::angled(a))), false);
    let claro = mezcla(metal, Color32::WHITE, if encima { 0.75 } else { 0.62 });
    let oscuro = mezcla(metal, Color32::BLACK, 0.5);
    aro(
        &mut m,
        o,
        0.0,
        d * 0.58,
        64,
        |a| mezcla(oscuro, claro, 0.35 + 0.5 * torneado(a)),
        |a| mezcla(oscuro, claro, 0.25 + 0.6 * torneado(a)),
        false,
    );
    p.add(Shape::mesh(m));
    p.circle_stroke(o, d * 0.58, Stroke::new(1.0, gris(0.08)));
    // La raya de la perilla, grabada en la falda.
    let (a, b) = (o + u * d * 0.6, o + u * d * 0.97);
    p.line_segment([a, b], Stroke::new(2.4 * k, Color32::from_black_alpha(200)));
    p.line_segment([a, b], Stroke::new(1.3 * k, Color32::from_rgb(0xf4, 0xee, 0xde)));
}
