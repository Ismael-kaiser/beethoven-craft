// Beethoven Craft. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! El metal, el vidrio y la relojería: botones con relieve, huecos, brillos y los marcos de egui vueltos de metal.
use super::estilo::WELL;
use eframe::egui::*;
use std::f32::consts::PI;

pub const BORDE_REPOSO: Color32 = Color32::from_rgb(0x15, 0x16, 0x18);
pub const BORDE_ENCIMA: Color32 = Color32::from_rgb(0x16, 0x17, 0x19);
pub const BORDE_APRETADO: Color32 = Color32::from_rgb(0x11, 0x11, 0x12);
pub const BORDE_ABIERTO: Color32 = Color32::from_rgb(0x1b, 0x1c, 0x1b);

/// De `a` a `b`.
pub fn mezcla(a: Color32, b: Color32, k: f32) -> Color32 {
    a.lerp_to_gamma(b, k)
}

/// Rectángulo redondeado con degradado de arriba a abajo.
pub fn degradado(p: &Painter, r: Rect, rad: u8, a: Color32, b: Color32) {
    degradado_en(p, r, CornerRadius::same(rad), a, b);
}

/// El metal en todo: al final de cada cuadro, los marcos de los botones, listas, casillas y campos de egui (los que
/// tienen sus bordes) se cambian por piezas con relieve, sin tocar cada botón. Los campos de texto quedan hundidos.
/// Solo revisa lo ya dibujado: cuesta unos microsegundos.
pub fn metalizar(ctx: &Context) {
    let capas: Vec<LayerId> = ctx.memory(|m| m.layer_ids().collect());
    let suave = 1.0 / ctx.pixels_per_point();
    ctx.graphics_mut(|g| {
        for l in capas {
            let Some(lista) = g.get_mut(l) else { continue };
            let cuales: Vec<usize> = lista.all_entries().enumerate().filter(|(_, s)| de_egui(&s.shape)).map(|(i, _)| i).collect();
            for i in cuales {
                lista.mutate_shape(layers::ShapeIdx(i), |s| s.shape = metal(&s.shape, suave));
            }
        }
    });
}

fn de_egui(s: &Shape) -> bool {
    let borde = |x: &Stroke| (x.width - 1.0).abs() < 0.01 && [BORDE_REPOSO, BORDE_ENCIMA, BORDE_APRETADO, BORDE_ABIERTO].contains(&x.color);
    match s {
        Shape::Rect(r) => borde(&r.stroke) && r.fill.a() == 255,
        _ => false,
    }
}

fn metal(s: &Shape, suave: f32) -> Shape {
    let Shape::Rect(r) = s else { return s.clone() };
    let (q, cr, f) = (r.rect, r.corner_radius, r.fill);
    let mut v = vec![];
    let borde = Shape::Rect(eframe::epaint::RectShape::stroke(q, cr, r.stroke, StrokeKind::Inside));
    let hundido = f == WELL || r.stroke.color == BORDE_APRETADO;
    if hundido {
        // Campo o botón apretado: metido en el tablero.
        v.push(Shape::Rect(eframe::epaint::RectShape::filled(q, cr, if f == WELL { f } else { mezcla(f, Color32::BLACK, 0.25) })));
        let arriba = Rect::from_min_max(q.min, pos2(q.right(), q.top() + 3.0));
        v.push(Shape::Rect(eframe::epaint::RectShape::filled(
            arriba,
            CornerRadius { nw: cr.nw, ne: cr.ne, sw: 0, se: 0 },
            Color32::from_black_alpha(70),
        )));
        v.push(borde);
        let k = cr.nw as f32 * 0.7;
        v.push(Shape::hline(q.left() + k..=q.right() - k, q.bottom() + 0.5, Stroke::new(1.0, Color32::from_white_alpha(16))));
        return Shape::Vec(v);
    }
    // Sombra, cuerpo oscuro, mitad de arriba más clara, filo de luz y borde.
    v.push(Shape::Rect(eframe::epaint::RectShape::filled(q.translate(vec2(0.0, 1.5)), cr, Color32::from_black_alpha(80))));
    // En piezas altas el degradado se queda en los primeros 30 puntos: si no, se verían casi negras abajo.
    let alto = q.height().min(30.0);
    let abajo = mezcla(f, Color32::BLACK, 0.24 * alto / 30.0 + 0.06);
    v.push(Shape::Rect(eframe::epaint::RectShape::filled(q, cr, abajo)));
    let arriba = Rect::from_min_max(q.min, pos2(q.right(), q.top() + alto));
    let cr_arriba = if alto < q.height() { CornerRadius { nw: cr.nw, ne: cr.ne, sw: 0, se: 0 } } else { cr };
    v.push(Shape::mesh(malla(arriba, cr_arriba, mezcla(f, Color32::WHITE, 0.14), abajo, suave)));
    let k = (cr.nw as f32 * 0.7).max(2.0);
    v.push(Shape::hline(q.left() + k..=q.right() - k, q.top() + 1.25, Stroke::new(1.0, Color32::from_white_alpha(45))));
    v.push(Shape::hline(q.left() + k..=q.right() - k, q.bottom() - 1.25, Stroke::new(1.0, Color32::from_black_alpha(55))));
    v.push(borde);
    Shape::Vec(v)
}

/// Lo mismo, con cada esquina a su medida.
pub fn degradado_en(p: &Painter, r: Rect, cr: CornerRadius, a: Color32, b: Color32) {
    p.add(Shape::mesh(malla(r, cr, a, b, 1.0 / p.pixels_per_point())));
}

/// Contorno de un rectángulo redondeado, en el sentido del reloj, con pocos puntos por esquina.
fn contorno(r: Rect, cr: CornerRadius) -> Vec<Pos2> {
    let tope = r.width().min(r.height()) / 2.0;
    let mut v: Vec<Pos2> = vec![];
    for (rad, x, y, a0) in [
        (cr.nw, r.left(), r.top(), PI),
        (cr.ne, r.right(), r.top(), 1.5 * PI),
        (cr.se, r.right(), r.bottom(), 0.0),
        (cr.sw, r.left(), r.bottom(), 0.5 * PI),
    ] {
        let rad = (rad as f32).min(tope);
        if rad < 0.5 {
            v.push(pos2(x, y));
            continue;
        }
        // El centro de la esquina, hacia adentro.
        let c = pos2(if x == r.left() { x + rad } else { x - rad }, if y == r.top() { y + rad } else { y - rad });
        let n = ((rad * 0.6).ceil() as usize).clamp(2, 8);
        for i in 0..=n {
            v.push(c + rad * Vec2::angled(a0 + 0.5 * PI * i as f32 / n as f32));
        }
    }
    v.dedup_by(|a, b| a.distance(*b) < 0.05);
    if v.len() > 1 && v[0].distance(v[v.len() - 1]) < 0.05 {
        v.pop();
    }
    v
}

fn entre(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()), m(a.a(), b.a()))
}

/// Malla de un rectángulo redondeado con color de `a` (arriba) a `b` (abajo) y orilla suavizada de `suave` puntos.
fn malla(r: Rect, cr: CornerRadius, a: Color32, b: Color32, suave: f32) -> Mesh {
    let mut m = Mesh::default();
    if r.width() <= 0.0 || r.height() <= 0.0 {
        return m;
    }
    let pts = contorno(r, cr);
    let n = pts.len();
    let col = |y: f32| entre(a, b, ((y - r.top()) / r.height()).clamp(0.0, 1.0));
    let uv = |_: Pos2| eframe::epaint::WHITE_UV;
    let o = r.center();
    m.vertices.push(eframe::epaint::Vertex { pos: o, uv: uv(o), color: col(o.y) });
    for i in 0..n {
        let (prev, q, sig) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
        let d1 = (q - prev).normalized();
        let d2 = (sig - q).normalized();
        // Hacia afuera (el contorno va en el sentido del reloj, con y hacia abajo).
        let nn = (vec2(d1.y, -d1.x) + vec2(d2.y, -d2.x)).normalized();
        let (dentro, fuera) = (q - nn * suave * 0.5, q + nn * suave * 0.5);
        m.vertices.push(eframe::epaint::Vertex { pos: dentro, uv: uv(dentro), color: col(q.y) });
        m.vertices.push(eframe::epaint::Vertex { pos: fuera, uv: uv(fuera), color: Color32::TRANSPARENT });
    }
    for i in 0..n as u32 {
        let j = (i + 1) % n as u32;
        let (di, fi, dj, fj) = (1 + 2 * i, 2 + 2 * i, 1 + 2 * j, 2 + 2 * j);
        m.indices.extend([0, di, dj, di, fi, dj, fi, fj, dj]);
    }
    m
}

/// Botón con relieve, como una tecla de un aparato: sombra abajo, de claro a oscuro, filo de luz arriba y borde.
/// `hundido`: apretado, se ve metido y con la sombra por dentro.
pub fn relieve(p: &Painter, r: Rect, rad: u8, base: Color32, hover: bool, hundido: bool) {
    let base = if hover && !hundido { mezcla(base, Color32::WHITE, 0.07) } else { base };
    let rr = CornerRadius::same(rad);
    if hundido {
        degradado(p, r, rad, mezcla(base, Color32::BLACK, 0.35), mezcla(base, Color32::BLACK, 0.12));
        // Sombra por dentro, arriba.
        let arriba = Rect::from_min_max(r.min, pos2(r.right(), r.top() + 3.0));
        p.with_clip_rect(arriba.intersect(p.clip_rect())).rect_stroke(
            r.shrink(0.5),
            rr,
            Stroke::new(2.0, Color32::from_black_alpha(110)),
            StrokeKind::Inside,
        );
        p.rect_stroke(r, rr, Stroke::new(1.0, Color32::from_black_alpha(150)), StrokeKind::Outside);
        return;
    }
    p.rect_filled(r.translate(vec2(0.0, 1.5)).expand(0.5), rr, Color32::from_black_alpha(90));
    degradado(p, r, rad, mezcla(base, Color32::WHITE, 0.14), mezcla(base, Color32::BLACK, 0.24));
    bisel(p, r, rad);
    p.rect_stroke(r, rr, Stroke::new(1.0, Color32::from_black_alpha(150)), StrokeKind::Outside);
}

/// Bisel de 1 punto: luz en la orilla de arriba y sombra en la de abajo, por dentro. Hace ver la pieza maquinada.
fn bisel(p: &Painter, r: Rect, rad: u8) {
    let k = (rad as f32 * 0.7).max(1.5);
    if r.width() > 2.0 * k {
        p.hline(r.left() + k..=r.right() - k, r.top() + 0.75, Stroke::new(1.0, Color32::from_white_alpha(45)));
        p.hline(r.left() + k..=r.right() - k, r.bottom() - 0.75, Stroke::new(1.0, Color32::from_black_alpha(60)));
    }
}

/// Botón encendido: se ilumina con su color y suelta un poco de luz alrededor.
pub fn encendido(p: &Painter, r: Rect, rad: u8, col: Color32) {
    for (e, a) in [(5.0, 0.05), (3.0, 0.08), (1.5, 0.12)] {
        p.rect_filled(r.expand(e), CornerRadius::same(rad + e as u8), col.gamma_multiply(a));
    }
    degradado(p, r, rad, mezcla(col, Color32::WHITE, 0.35), mezcla(col, Color32::BLACK, 0.12));
    // Reflejo de cromo: una luz blanca que se apaga hacia la mitad.
    let reflejo = Rect::from_min_max(r.min + vec2(1.0, 1.0), pos2(r.right() - 1.0, r.top() + r.height() * 0.55));
    let rr = rad.saturating_sub(1);
    degradado_en(p, reflejo, CornerRadius { nw: rr, ne: rr, sw: 0, se: 0 }, Color32::from_white_alpha(130), Color32::from_white_alpha(0));
    let k = rad as f32 * 0.7;
    p.hline(r.left() + k..=r.right() - k, r.top() + 0.75, Stroke::new(1.0, Color32::from_white_alpha(120)));
    p.rect_stroke(r, CornerRadius::same(rad), Stroke::new(1.0, mezcla(col, Color32::BLACK, 0.55)), StrokeKind::Outside);
}

/// Hueco metido en el tablero (pantallas, medidores, campos): oscuro, con sombra arriba y luz abajo.
pub fn hueco(p: &Painter, r: Rect, rad: u8, fondo: Color32) {
    let rr = CornerRadius::same(rad);
    p.rect_filled(r, rr, fondo);
    let arriba = Rect::from_min_max(r.min, pos2(r.right(), r.top() + 3.0));
    p.with_clip_rect(arriba.intersect(p.clip_rect())).rect_stroke(
        r.shrink(0.5),
        rr,
        Stroke::new(2.0, Color32::from_black_alpha(120)),
        StrokeKind::Inside,
    );
    let k = rad as f32 * 0.7;
    p.hline(r.left() + k..=r.right() - k, r.bottom() + 0.5, Stroke::new(1.0, Color32::from_white_alpha(18)));
}

/// Tornillito pulido de reloj: cabeza redonda con luz, ranura y bisel. `giro`: hacia dónde quedó la ranura.
pub fn tornillo(p: &Painter, o: Pos2, r: f32, giro: f32, azul: bool) {
    let (claro, oscuro) = if azul {
        (Color32::from_rgb(0x6f, 0x93, 0xd8), Color32::from_rgb(0x1c, 0x2c, 0x5e))
    } else {
        (Color32::from_rgb(0xd4, 0xd6, 0xda), Color32::from_rgb(0x5a, 0x5c, 0x61))
    };
    p.circle_filled(o, r + 1.0, Color32::from_black_alpha(150));
    p.circle_filled(o + vec2(0.0, 0.6), r + 0.6, Color32::from_white_alpha(22));
    p.circle_filled(o, r, oscuro);
    p.circle_filled(o - vec2(0.0, r * 0.18), r * 0.82, mezcla(oscuro, claro, 0.55));
    p.circle_filled(o - vec2(r * 0.15, r * 0.34), r * 0.45, claro);
    let d = Vec2::angled(giro) * r * 0.8;
    p.line_segment([o - d, o + d], Stroke::new((r * 0.36).max(0.8), Color32::from_black_alpha(190)));
}

/// Esfera chica de cronógrafo: aro pulido, minutería, y dos manecillas (segundos y minutos) con el tiempo `s`.
pub fn esfera(p: &Painter, o: Pos2, r: f32, s: f64, col: Color32) {
    p.circle_filled(o + vec2(0.0, 1.0), r + 2.0, Color32::from_black_alpha(150));
    // Aro: oscuro, acero y un filo de luz arriba.
    p.circle_filled(o, r + 1.2, Color32::from_rgb(0x2c, 0x2d, 0x31));
    p.circle_filled(o - vec2(0.0, 0.5), r + 0.4, Color32::from_rgb(0x9b, 0x9e, 0xa5));
    p.circle_filled(o + vec2(0.0, 0.4), r - 0.4, Color32::from_rgb(0x3a, 0x3c, 0x41));
    p.circle_filled(o, r - 1.3, Color32::from_rgb(0x10, 0x11, 0x14));
    for i in 0..12 {
        let d = Vec2::angled(i as f32 / 12.0 * 2.0 * PI - PI / 2.0);
        let largo = if i % 3 == 0 { 3.2 } else { 1.8 };
        p.line_segment(
            [o + d * (r - 2.0), o + d * (r - 2.0 - largo)],
            Stroke::new(if i % 3 == 0 { 1.1 } else { 0.8 }, Color32::from_white_alpha(if i % 3 == 0 { 210 } else { 110 })),
        );
    }
    let mano = |a: f64, l: f32, w: f32, c: Color32| {
        let d = Vec2::angled((a * 2.0 * std::f64::consts::PI - std::f64::consts::FRAC_PI_2) as f32);
        p.line_segment([o - d * l * 0.28, o + d * l], Stroke::new(w, c));
    };
    mano(s / 3600.0, (r - 2.0) * 0.6, 1.8, Color32::from_rgb(0xcf, 0xd2, 0xd8));
    mano(s / 60.0, (r - 2.0) * 0.92, 1.1, col);
    p.circle_filled(o, 1.8, col);
    p.circle_filled(o - vec2(0.4, 0.5), 0.7, Color32::from_white_alpha(200));
    // Cristal abombado: media luna de luz.
    let n = 14;
    let mut v: Vec<Pos2> = (0..=n).map(|i| o + (r - 1.6) * Vec2::angled(PI + PI * i as f32 / n as f32)).collect();
    v.extend((0..=n).rev().map(|i| {
        let a = PI + PI * i as f32 / n as f32;
        o + vec2(a.cos() * (r - 1.6), a.sin() * (r - 1.6) * 0.12)
    }));
    p.add(Shape::convex_polygon(v[..=n].to_vec(), Color32::from_white_alpha(20), Stroke::NONE));
}

/// Índice aplicado de una carátula: barrita facetada, con un lado en luz y el otro en sombra.
pub fn indice(p: &Painter, x: f32, y0: f32, y1: f32, w: f32, col: Color32) {
    p.rect_filled(Rect::from_min_max(pos2(x - w / 2.0 + 0.6, y0 + 1.2), pos2(x + w / 2.0 + 0.8, y1 + 1.2)), 1, Color32::from_black_alpha(130));
    p.rect_filled(Rect::from_min_max(pos2(x - w / 2.0, y0), pos2(x, y1)), 0, mezcla(col, Color32::WHITE, 0.45));
    p.rect_filled(Rect::from_min_max(pos2(x, y0), pos2(x + w / 2.0, y1)), 0, mezcla(col, Color32::BLACK, 0.35));
}

/// Vidrio encima de algo ya dibujado, como el Aero de antes: la mitad de arriba con un brillo que termina en seco, luz que
/// sube desde abajo con el tono `col`, filo blanco por dentro y borde oscuro por fuera. Sin desenfoques: casi no cuesta.
pub fn vidrio(p: &Painter, r: Rect, rad: u8, col: Color32, hover: bool) {
    vidrio_en(p, r, rad, col, hover, r.height() * 0.46);
}

/// [`vidrio`], con el brillo de arriba hasta `alto`.
pub fn vidrio_en(p: &Painter, r: Rect, rad: u8, col: Color32, hover: bool, alto: f32) {
    if r.width() < 3.0 || r.height() < 3.0 {
        return;
    }
    let k = if hover { 1.35 } else { 1.0 };
    let blanco = |a: f32| Color32::from_white_alpha((a * k).min(255.0) as u8);
    let corte = r.top() + alto.min(r.height());
    degradado_en(p, Rect::from_min_max(r.min, pos2(r.right(), corte)), CornerRadius { nw: rad, ne: rad, sw: 0, se: 0 }, blanco(58.0), blanco(16.0));
    let luz = mezcla(col, Color32::WHITE, 0.45);
    let t = |a: f32| Color32::from_rgba_unmultiplied(luz.r(), luz.g(), luz.b(), (a * k).min(255.0) as u8);
    degradado_en(
        p,
        Rect::from_min_max(pos2(r.left(), r.bottom() - r.height() * 0.38), r.max),
        CornerRadius { nw: 0, ne: 0, sw: rad, se: rad },
        t(0.0),
        t(46.0),
    );
    p.rect_stroke(r.shrink(0.5), CornerRadius::same(rad.saturating_sub(1)), Stroke::new(1.0, blanco(46.0)), StrokeKind::Inside);
    let f = (rad as f32 * 0.7).max(1.5);
    p.hline(r.left() + f..=r.right() - f, r.top() + 0.75, Stroke::new(1.0, blanco(120.0)));
    p.rect_stroke(r, CornerRadius::same(rad), Stroke::new(1.0, mezcla(col, Color32::BLACK, 0.78)), StrokeKind::Outside);
}

/// Rueda dentada fina (engrane), girada `a` radianes.
pub fn engrane(p: &Painter, o: Pos2, r: f32, dientes: usize, a: f32, col: Color32) {
    for i in 0..dientes {
        let d = Vec2::angled(a + i as f32 / dientes as f32 * 2.0 * PI);
        p.line_segment([o + d * (r - 1.0), o + d * (r + 1.8)], Stroke::new(1.3, col));
    }
    p.circle_stroke(o, r - 0.6, Stroke::new(1.4, col));
}
