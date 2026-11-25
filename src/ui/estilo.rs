// Copyright (C) 2026 ItsAlphaNeon (diseño y colores de la piel «Lemon» de Lemon Studio)
// SPDX-License-Identifier: GPL-3.0-or-later
// Adapted for Toma Uno on 2026-11-23 from Lemon Studio (https://github.com/ItsAlphaNeon/Lemon-Studio, commit 349c6ba):
// crates/os-theme/src/design.rs (colours) and crates/os-ui/src/kit/lemon.rs (knob, fader, LED, buttons), repainted
// with egui. Fonts: Cuprum and Barlow (OFL 1.1). Icons: Tabler (MIT).
#![allow(dead_code)]
use super::tabler::TablerIcon;
use eframe::egui::{self, *};
use std::f32::consts::PI;

/// Un color. Los grises de las superficies se llevan al modo oscuro: más abajo y con un toque frío; los de las letras
/// se quedan.
pub const fn c(h: u32) -> Color32 {
    let (r, g, b) = ((h >> 16) as u8, (h >> 8) as u8, h as u8);
    if r == g && g == b && r < 0x70 {
        let v = r as u32;
        let n = v * 58 / 100;
        return Color32::from_rgb(n as u8, (n + n / 22) as u8, (n + n / 7 + 1) as u8);
    }
    Color32::from_rgb(r, g, b)
}
pub const MAIN: Color32 = c(0xffe066);
pub const HIGHLIGHT: Color32 = c(0xfff1b8);
pub const COLORED_DARK: Color32 = c(0x473900);
pub const DEEP: Color32 = c(0xc9a92c);
pub const TOOLBAR: Color32 = c(0x3b3b3b);
pub const TOOLBAR_LINE: Color32 = c(0x2a2a2a);
pub const SUNKEN: Color32 = c(0x2b2b2b);
pub const TIME_PANEL: Color32 = c(0x232323);
pub const BUTTON: Color32 = c(0x474747);
pub const BUTTON_HI: Color32 = c(0x555555);
pub const GLYPH: Color32 = c(0xe6e6e6);
pub const TEXT: Color32 = c(0xececec);
pub const TEXT_DIM: Color32 = c(0xa3a3a3);
pub const TEXT_FAINT: Color32 = c(0x7f7f7f);
pub const RECORD: Color32 = c(0xff6159);
pub const WORKSPACE: Color32 = c(0x262626);
pub const WINDOW_TITLE: Color32 = c(0x333333);
pub const WINDOW_BODY: Color32 = c(0x2e2e2e);
pub const TRACK_HEADER: Color32 = c(0x383838);
pub const RULER: Color32 = c(0x232323);
pub const RULER_TEXT: Color32 = c(0x9e9e9e);
pub const GRID: Color32 = c(0x282828);
pub const GRID_ALT: Color32 = c(0x2d2d2d);
pub const GRID_GAP: Color32 = c(0x202020);
pub const GRID_BAR: Color32 = c(0x161616);
pub const STRIP: Color32 = c(0x303030);
pub const STRIP_DARK: Color32 = c(0x242424);
pub const SLOT: Color32 = c(0x2b2b2b);
pub const WELL: Color32 = c(0x1f1f1f);
pub const WELL_LINE: Color32 = c(0x181818);
pub const METER_CLIP: Color32 = c(0xf05040);

/// Colores de pista, en el orden en que se crean.
pub const PISTAS: [Color32; 8] = [
    c(0xe0564f),
    c(0x43c0c8),
    c(0x9b63d6),
    c(0xe8b842),
    c(0x4a90e0),
    c(0x5cc46a),
    c(0xe27a3f),
    c(0xd667ac),
];

/// Apagados hacia el gris para que no compitan con la imagen.
pub fn color_pista(i: usize) -> Color32 {
    mezcla(PISTAS[i % PISTAS.len()], c(0x8a8a8a), 0.4)
}

pub fn aplicar(ctx: &Context) {
    let mut f = FontDefinitions::default();
    let mut pon = |n: &str, b: &'static [u8]| {
        f.font_data.insert(n.into(), std::sync::Arc::new(FontData::from_static(b)));
    };
    pon("cuprum", include_bytes!("../../recursos/Cuprum.ttf"));
    pon("barlow", include_bytes!("../../recursos/BarlowSemiCondensed-SemiBold.ttf"));
    f.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "cuprum".into());
    // Nombres de archivo en japonés, coreano, etc.: letra del sistema como último recurso.
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let unicode: Vec<String> = if cfg!(windows) {
        ["msyh.ttc", "malgun.ttf", "seguisym.ttf"].iter().map(|f| format!("{windir}\\Fonts\\{f}")).collect()
    } else if cfg!(target_os = "macos") {
        vec!["/System/Library/Fonts/Supplemental/Arial Unicode.ttf".into()]
    } else {
        ["/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"]
            .map(String::from)
            .to_vec()
    };
    if let Some(b) = unicode.iter().find_map(|r| std::fs::read(r).ok()) {
        f.font_data.insert("unicode".into(), std::sync::Arc::new(FontData::from_owned(b)));
        f.families.get_mut(&FontFamily::Proportional).unwrap().push("unicode".into());
    }
    f.families.insert(FontFamily::Name("barlow".into()), vec!["barlow".into(), "cuprum".into()]);
    ctx.set_fonts(f);
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|s| {
        let v = &mut s.visuals;
        v.panel_fill = WORKSPACE;
        v.window_fill = WINDOW_BODY;
        v.window_stroke = Stroke::new(1.0, c(0x1a1a1a));
        v.extreme_bg_color = WELL;
        v.override_text_color = Some(TEXT);
        v.selection.bg_fill = DEEP;
        v.selection.stroke = Stroke::new(1.0, MAIN);
        let r = CornerRadius::same(6);
        for w in [
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
            &mut v.widgets.noninteractive,
        ] {
            w.corner_radius = r;
            w.bg_stroke = Stroke::NONE;
        }
        v.widgets.inactive.weak_bg_fill = BUTTON;
        v.widgets.inactive.bg_fill = BUTTON;
        v.widgets.hovered.weak_bg_fill = BUTTON_HI;
        v.widgets.hovered.bg_fill = BUTTON_HI;
        v.widgets.active.weak_bg_fill = DEEP;
        v.widgets.open.weak_bg_fill = BUTTON_HI;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, TOOLBAR_LINE);
        // Relieve en los botones de egui: un borde oscuro los despega del fondo.
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDE_REPOSO);
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDE_ENCIMA);
        v.widgets.active.bg_stroke = Stroke::new(1.0, BORDE_APRETADO);
        v.widgets.open.bg_stroke = Stroke::new(1.0, BORDE_ABIERTO);
        v.window_shadow = epaint::Shadow { offset: [0, 10], blur: 30, spread: 0, color: Color32::from_black_alpha(150) };
        v.popup_shadow = epaint::Shadow { offset: [0, 6], blur: 16, spread: 0, color: Color32::from_black_alpha(130) };
        for (k, t) in s.text_styles.iter_mut() {
            t.size = match k {
                TextStyle::Heading => 19.0,
                TextStyle::Small => 12.0,
                _ => 15.0,
            };
        }
        s.interaction.tooltip_delay = 0.4;
        s.spacing.button_padding = vec2(8.0, 4.0);
    });
}

// Bordes de los botones, listas y casillas de egui. Son únicos (nada más los usa) para encontrarlos en `metalizar`.
const BORDE_REPOSO: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x1c);
const BORDE_ENCIMA: Color32 = Color32::from_rgb(0x6a, 0x6a, 0x6b);
const BORDE_APRETADO: Color32 = Color32::from_rgb(0x11, 0x11, 0x12);
const BORDE_ABIERTO: Color32 = Color32::from_rgb(0x1b, 0x1c, 0x1b);

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
    let borde = Shape::Rect(epaint::RectShape::stroke(q, cr, r.stroke, StrokeKind::Inside));
    let hundido = f == WELL || r.stroke.color == BORDE_APRETADO;
    if hundido {
        // Campo o botón apretado: metido en el tablero.
        v.push(Shape::Rect(epaint::RectShape::filled(q, cr, if f == WELL { f } else { mezcla(f, Color32::BLACK, 0.25) })));
        let arriba = Rect::from_min_max(q.min, pos2(q.right(), q.top() + 3.0));
        v.push(Shape::Rect(epaint::RectShape::filled(arriba, CornerRadius { nw: cr.nw, ne: cr.ne, sw: 0, se: 0 }, Color32::from_black_alpha(70))));
        v.push(borde);
        let k = cr.nw as f32 * 0.7;
        v.push(Shape::hline(q.left() + k..=q.right() - k, q.bottom() + 0.5, Stroke::new(1.0, Color32::from_white_alpha(16))));
        return Shape::Vec(v);
    }
    // Sombra, cuerpo oscuro, mitad de arriba más clara, filo de luz y borde.
    v.push(Shape::Rect(epaint::RectShape::filled(q.translate(vec2(0.0, 1.5)), cr, Color32::from_black_alpha(80))));
    // En piezas altas el degradado se queda en los primeros 30 puntos: si no, se verían casi negras abajo.
    let alto = q.height().min(30.0);
    let abajo = mezcla(f, Color32::BLACK, 0.24 * alto / 30.0 + 0.06);
    v.push(Shape::Rect(epaint::RectShape::filled(q, cr, abajo)));
    let arriba = Rect::from_min_max(q.min, pos2(q.right(), q.top() + alto));
    let cr_arriba = if alto < q.height() { CornerRadius { nw: cr.nw, ne: cr.ne, sw: 0, se: 0 } } else { cr };
    v.push(Shape::mesh(malla(arriba, cr_arriba, mezcla(f, Color32::WHITE, 0.14), abajo, suave)));
    let k = (cr.nw as f32 * 0.7).max(2.0);
    v.push(Shape::hline(q.left() + k..=q.right() - k, q.top() + 1.25, Stroke::new(1.0, Color32::from_white_alpha(45))));
    v.push(Shape::hline(q.left() + k..=q.right() - k, q.bottom() - 1.25, Stroke::new(1.0, Color32::from_black_alpha(55))));
    v.push(borde);
    Shape::Vec(v)
}

pub fn mezcla(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

pub fn fuente(t: f32) -> FontId {
    FontId::proportional(t)
}
pub fn barlow(t: f32) -> FontId {
    FontId::new(t, FontFamily::Name("barlow".into()))
}

/// Rectángulo redondeado con degradado vertical (arriba, abajo): una sola figura lisa, sin franjas.
pub fn degradado(p: &Painter, r: Rect, rad: u8, a: Color32, b: Color32) {
    degradado_en(p, r, CornerRadius::same(rad), a, b);
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
    let uv = |_: Pos2| epaint::WHITE_UV;
    let o = r.center();
    m.vertices.push(epaint::Vertex { pos: o, uv: uv(o), color: col(o.y) });
    for i in 0..n {
        let (prev, q, sig) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
        let d1 = (q - prev).normalized();
        let d2 = (sig - q).normalized();
        // Hacia afuera (el contorno va en el sentido del reloj, con y hacia abajo).
        let nn = (vec2(d1.y, -d1.x) + vec2(d2.y, -d2.x)).normalized();
        let (dentro, fuera) = (q - nn * suave * 0.5, q + nn * suave * 0.5);
        m.vertices.push(epaint::Vertex { pos: dentro, uv: uv(dentro), color: col(q.y) });
        m.vertices.push(epaint::Vertex { pos: fuera, uv: uv(fuera), color: Color32::TRANSPARENT });
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
        p.with_clip_rect(arriba.intersect(p.clip_rect())).rect_stroke(r.shrink(0.5), rr, Stroke::new(2.0, Color32::from_black_alpha(110)), StrokeKind::Inside);
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
    p.with_clip_rect(arriba.intersect(p.clip_rect())).rect_stroke(r.shrink(0.5), rr, Stroke::new(2.0, Color32::from_black_alpha(120)), StrokeKind::Inside);
    let k = rad as f32 * 0.7;
    p.hline(r.left() + k..=r.right() - k, r.bottom() + 0.5, Stroke::new(1.0, Color32::from_white_alpha(18)));
}

pub fn arco(p: &Painter, o: Pos2, r: f32, a: f32, barrido: f32, w: f32, col: Color32) {
    let n = ((barrido.abs() * r / 2.0) as usize).clamp(4, 48);
    let pts = (0..=n).map(|i| o + r * Vec2::angled(a + barrido * i as f32 / n as f32)).collect();
    p.add(Shape::line(pts, Stroke::new(w, col)));
}

/// Dibuja un ícono de Tabler (rejilla de 24) en el cuadro `r`.
pub fn icono(p: &Painter, i: &TablerIcon, r: Rect, col: Color32) {
    let k = r.width().min(r.height()) / 24.0;
    let o = r.center() - vec2(12.0, 12.0) * k;
    let q = |x: f32, y: f32| o + vec2(x, y) * k;
    let mut t = i.d.split_ascii_whitespace();
    let num = |t: &mut std::str::SplitAsciiWhitespace| -> f32 { t.next().and_then(|s| s.parse().ok()).unwrap_or(0.0) };
    let mut pts: Vec<Pos2> = vec![];
    let traza = |pts: &mut Vec<Pos2>, cerrado: bool| {
        let v = std::mem::take(pts);
        if v.len() < 2 {
            return;
        }
        if i.filled {
            p.add(Shape::convex_polygon(v, col, Stroke::NONE));
        } else {
            let s = Stroke::new((2.0 * k).max(1.1), col);
            p.add(if cerrado { Shape::closed_line(v, s) } else { Shape::line(v, s) });
        }
    };
    while let Some(op) = t.next() {
        match op {
            "M" => {
                traza(&mut pts, false);
                let (x, y) = (num(&mut t), num(&mut t));
                pts.push(q(x, y));
            }
            "L" => {
                let (x, y) = (num(&mut t), num(&mut t));
                pts.push(q(x, y));
            }
            "Q" | "C" => {
                let n = if op == "Q" { 2 } else { 3 };
                let a = *pts.last().unwrap_or(&o);
                let v: Vec<Pos2> = (0..n)
                    .map(|_| {
                        let (x, y) = (num(&mut t), num(&mut t));
                        q(x, y)
                    })
                    .collect();
                for s in 1..=6 {
                    let u = s as f32 / 6.0;
                    let w = 1.0 - u;
                    let z = if n == 2 {
                        a.to_vec2() * w * w + v[0].to_vec2() * 2.0 * w * u + v[1].to_vec2() * u * u
                    } else {
                        a.to_vec2() * w * w * w
                            + v[0].to_vec2() * 3.0 * w * w * u
                            + v[1].to_vec2() * 3.0 * w * u * u
                            + v[2].to_vec2() * u * u * u
                    };
                    pts.push(z.to_pos2());
                }
            }
            "Z" => traza(&mut pts, true),
            _ => {}
        }
    }
    traza(&mut pts, false);
}

/// Botón cuadrado con ícono. `on`: encendido (amarillo).
pub fn boton(ui: &mut Ui, i: &TablerIcon, on: bool, ayuda: &str) -> Response {
    boton_color(ui, i, on, ayuda, GLYPH, 32.0)
}

pub fn boton_color(ui: &mut Ui, i: &TablerIcon, on: bool, ayuda: &str, col: Color32, lado: f32) -> Response {
    let ayuda = crate::idioma::t(ayuda);
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(lado), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), ayuda));
    let p = ui.painter();
    let rr = r.shrink(1.0);
    let mut tinta = if ui.is_enabled() { col } else { TEXT_FAINT };
    if on {
        encendido(p, rr, 6, MAIN);
        tinta = COLORED_DARK;
    } else {
        relieve(p, rr, 6, BUTTON, resp.hovered(), resp.is_pointer_button_down_on());
    }
    icono(p, i, r.shrink(lado * 0.22), tinta);
    resp.on_hover_text(ayuda)
}

/// Interruptor redondeado (amarillo si está encendido).
pub fn led(ui: &mut Ui, on: bool, ayuda: &str) -> Response {
    let ayuda = crate::idioma::t(ayuda);
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, on, ayuda));
    pinta_led(ui.painter(), r.center(), 6.0, on, resp.hovered());
    resp.on_hover_text(ayuda)
}

pub fn pinta_led(p: &Painter, o: Pos2, h: f32, on: bool, hover: bool) {
    let r = Rect::from_center_size(o, Vec2::splat(2.0 * h));
    hueco(p, r.expand(1.0), 4, if hover { c(0x3a3a3a) } else { c(0x151515) });
    if on {
        encendido(p, r.shrink(1.0), 2, MAIN);
    } else {
        degradado(p, r.shrink(1.0), 2, mezcla(COLORED_DARK, TOOLBAR_LINE, 0.25), mezcla(COLORED_DARK, Color32::BLACK, 0.5));
    }
}

/// Botoncito de letra (S, G…).
pub fn letra(ui: &mut Ui, s: &str, on: bool, col: Color32, ayuda: &str) -> Response {
    let ayuda = crate::idioma::t(ayuda);
    let (r, resp) = ui.allocate_exact_size(vec2(20.0, 18.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, on, ayuda));
    if on {
        encendido(ui.painter(), r, 4, col);
    } else {
        relieve(ui.painter(), r, 4, BUTTON, resp.hovered(), resp.is_pointer_button_down_on());
    }
    let tinta = if on { c(0x1c1c1c) } else { TEXT_DIM };
    if s == "●" {
        ui.painter().circle_filled(r.center(), 4.5, tinta);
    } else {
        ui.painter().text(r.center(), Align2::CENTER_CENTER, s, barlow(12.0), tinta);
    }
    resp.on_hover_text(ayuda)
}

/// Perilla como de consola: marcas alrededor, aro iluminado con el valor y una tapa metálica con sombra, brillo y
/// estrías en el borde.
pub fn pinta_perilla(p: &Painter, o: Pos2, r: f32, v: f32, bipolar: bool, hover: bool) {
    let v = v.clamp(0.0, 1.0);
    let (ini, bar) = (0.75 * PI, 1.5 * PI);
    let a = ini + bar * v;
    let w = (r * 0.16).clamp(1.6, 2.6);
    let pista = r - w / 2.0;
    // Canal hundido y el valor encendido, con su resplandor.
    arco(p, o, pista, ini, bar, w + 1.0, c(0x121212));
    let a0 = if bipolar { ini + bar / 2.0 } else { ini };
    if (a - a0).abs() > 0.02 {
        let col = if hover { HIGHLIGHT } else { MAIN };
        arco(p, o, pista, a0, a - a0, w + 3.0, col.gamma_multiply(0.18));
        arco(p, o, pista, a0, a - a0, w, col);
    }
    let disco = r - w - 1.5;
    if disco < 3.0 {
        return;
    }
    // Sombra que cae abajo.
    p.circle_filled(o + vec2(0.0, disco * 0.12), disco + 0.8, Color32::from_black_alpha(120));
    // Tapa: borde oscuro y cuerpo que se aclara hacia arriba (la luz viene de arriba).
    p.circle_filled(o, disco, c(0x1e1e1e));
    let (claro, oscuro) = if hover { (c(0x8a8a8a), c(0x3c3c3c)) } else { (c(0x7a7a7a), c(0x353535)) };
    let n = 6;
    for i in 0..n {
        let f = i as f32 / (n - 1) as f32;
        let rr = (disco - 1.0) * (1.0 - 0.45 * f);
        p.circle_filled(o - vec2(0.0, (disco - 1.0 - rr) * 0.55), rr, mezcla(oscuro, claro, f));
    }
    // Estrías del borde.
    if disco > 7.0 {
        for i in 0..20 {
            let d = Vec2::angled(i as f32 / 20.0 * 2.0 * PI);
            p.line_segment([o + d * (disco - 2.2), o + d * (disco - 0.8)], Stroke::new(0.8, Color32::from_black_alpha(70)));
        }
    }
    // Brillo arriba.
    p.circle_filled(o + vec2(-disco * 0.22, -disco * 0.34), disco * 0.32, Color32::from_white_alpha(16));
    // Raya que marca hacia dónde apunta.
    let d = Vec2::angled(a);
    let ancho = (disco * 0.2).clamp(1.4, 2.2);
    let (x0, x1) = (o + d * disco * 0.2, o + d * (disco - 1.8).max(1.0));
    p.line_segment([x0, x1], Stroke::new(ancho + 1.4, Color32::from_black_alpha(140)));
    p.line_segment([x0, x1], Stroke::new(ancho, c(0xf2f2f2)));
}

/// Perilla sin texto. Arrastrar arriba/abajo, doble clic vuelve a `def`. `v` va de `min` a `max`.
pub fn perilla(ui: &mut Ui, v: &mut f32, min: f32, max: f32, def: f32, log: bool, r: f32, nombre: &str) -> Response {
    let nombre = crate::idioma::t(nombre);
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::splat(2.0 * r), Sense::click_and_drag());
    let a01 = |x: f32| if log { (x / min).ln() / (max / min).ln() } else { (x - min) / (max - min) };
    let de01 = |t: f32| if log { min * (max / min).powf(t) } else { min + (max - min) * t };
    if resp.dragged() {
        let fino = if ui.input(|i| i.modifiers.shift) { 0.15 } else { 1.0 };
        let d = resp.drag_delta();
        let t = (a01(*v) + (d.x - d.y) * fino / 160.0).clamp(0.0, 1.0);
        *v = de01(t).clamp(min, max);
        resp.mark_changed();
    }
    if resp.double_clicked() {
        *v = def;
        resp.mark_changed();
    }
    resp.widget_info(|| WidgetInfo::slider(true, *v as f64, nombre));
    pinta_perilla(ui.painter(), rect.center(), r, a01(*v), min < 0.0 && max > 0.0, resp.hovered() || resp.dragged());
    resp
}

/// Fundido de medidor: de amarillo oscuro a claro, rojo si se pasa de 0 dB.
pub fn medidor(p: &Painter, r: Rect, pico: [f32; 2]) {
    hueco(p, r, 3, c(0x111111));
    let w = (r.width() - 1.0) / 2.0;
    for (k, x) in pico.into_iter().enumerate() {
        let d = crate::modelo::a_db(x);
        let f = ((d + 54.0) / 60.0).clamp(0.0, 1.0);
        let x0 = r.left() + k as f32 * (w + 1.0);
        let y = r.bottom() - r.height() * f;
        let b = Rect::from_min_max(pos2(x0, y), pos2(x0 + w, r.bottom()));
        let col = if d > 0.0 { METER_CLIP } else { mezcla(DEEP, HIGHLIGHT, f * f) };
        p.rect_filled(b, 1, col);
        // Rayitas que lo parten en lucecitas, como un medidor de verdad.
        let mut yy = r.bottom() - 3.0;
        while yy > y {
            p.hline(b.x_range(), yy, Stroke::new(1.0, Color32::from_black_alpha(110)));
            yy -= 3.0;
        }
    }
    // Marca de 0 dB.
    let y0 = r.bottom() - r.height() * 0.9;
    p.hline(r.x_range(), y0, Stroke::new(1.0, c(0x4a4a4a)));
}

pub fn tiempo(s: f64) -> String {
    let s = s.max(0.0);
    format!("{}:{:02}:{:02}", s as u64 / 60, s as u64 % 60, (s.fract() * 100.0) as u64)
}
