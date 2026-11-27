// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Colores, letra y piezas chicas, con el metal de Movie.
use super::metal::*;
use eframe::egui::*;
use std::f32::consts::PI;

pub const FONDO: Color32 = Color32::from_rgb(0x12, 0x13, 0x16);
pub const PANEL: Color32 = Color32::from_rgb(0x1e, 0x1f, 0x23);
pub const CAJA: Color32 = Color32::from_rgb(0x38, 0x39, 0x3e);
pub const CAJA2: Color32 = Color32::from_rgb(0x45, 0x46, 0x4c);
pub const RAYA: Color32 = Color32::from_rgb(0x33, 0x34, 0x39);
pub const HUECO: Color32 = Color32::from_rgb(0x0c, 0x0d, 0x0f);
pub const TEXTO: Color32 = Color32::from_rgb(0xec, 0xeb, 0xe6);
pub const TENUE: Color32 = Color32::from_rgb(0x96, 0x97, 0x9e);
pub const ACENTO: Color32 = Color32::from_rgb(0xff, 0xd9, 0x5e);
pub const VERDE: Color32 = Color32::from_rgb(0x8a, 0xe0, 0x8f);
pub const ROJO: Color32 = Color32::from_rgb(0xff, 0x5f, 0x5a);

pub fn color(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

pub fn vestir(ctx: &Context) {
    ctx.set_theme(Theme::Dark);
    let mut f = FontDefinitions::default();
    f.font_data.insert(
        "cuprum".into(),
        std::sync::Arc::new(
            FontData::from_static(include_bytes!("../../../recursos/Cuprum.ttf")).tweak(FontTweak { y_offset_factor: 0.06, ..Default::default() }),
        ),
    );
    f.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "cuprum".into());
    ctx.set_fonts(f);
    ctx.all_styles_mut(|s| {
        let v = &mut s.visuals;
        v.panel_fill = PANEL;
        v.window_fill = PANEL;
        v.window_stroke = Stroke::new(1.0, Color32::BLACK);
        v.window_corner_radius = 8.into();
        v.extreme_bg_color = HUECO;
        v.override_text_color = Some(TEXTO);
        v.selection.bg_fill = ACENTO.gamma_multiply(0.35);
        v.selection.stroke = Stroke::new(1.0, ACENTO);
        for (w, c, b) in [
            (&mut v.widgets.inactive, CAJA, BORDE_REPOSO),
            (&mut v.widgets.hovered, CAJA2, BORDE_ENCIMA),
            (&mut v.widgets.active, CAJA2, BORDE_APRETADO),
            (&mut v.widgets.open, CAJA2, BORDE_ABIERTO),
        ] {
            w.bg_fill = c;
            w.weak_bg_fill = c;
            w.bg_stroke = Stroke::new(1.0, b);
            w.expansion = 0.0;
            w.corner_radius = 5.into();
        }
        s.spacing.item_spacing = vec2(6.0, 6.0);
        s.spacing.button_padding = vec2(9.0, 3.0);
        s.spacing.interact_size.y = 24.0;
        for (k, n) in [(TextStyle::Body, 15.0), (TextStyle::Button, 15.0), (TextStyle::Heading, 19.0), (TextStyle::Small, 12.0)] {
            s.text_styles.insert(k, letra(n));
        }
    });
}

pub fn letra(n: f32) -> FontId {
    FontId::proportional(n)
}

/// Pinta un texto recortado a un ancho.
pub fn rotulo(p: &Painter, r: Rect, s: &str, n: f32, c: Color32) {
    let g = p.layout_no_wrap(s.to_string(), letra(n), c);
    p.with_clip_rect(r.intersect(p.clip_rect())).galley(pos2(r.left(), r.center().y - g.size().y / 2.0), g, c);
}

/// Placa de metal de fondo: de claro a oscuro, con filo de luz.
pub fn placa(p: &Painter, r: Rect, rad: u8) {
    degradado(p, r, rad, Color32::from_rgb(0x2d, 0x2e, 0x33), Color32::from_rgb(0x17, 0x18, 0x1b));
    p.hline(r.left() + rad as f32..=r.right() - rad as f32, r.top() + 0.75, Stroke::new(1.0, Color32::from_white_alpha(34)));
}

/// Tecla de metal; prendida se ilumina con su color.
pub fn foco(ui: &mut Ui, si: bool, s: &str, c: Color32) -> Response {
    let g = ui.painter().layout_no_wrap(s.to_string(), letra(15.0), TEXTO);
    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 18.0, 24.0), Sense::click());
    if si {
        encendido(ui.painter(), r, 5, c);
    } else {
        relieve(ui.painter(), r, 5, CAJA, resp.hovered(), resp.is_pointer_button_down_on());
    }
    let col = if si { Color32::from_rgb(0x1a, 0x17, 0x0c) } else { TEXTO };
    ui.painter().text(r.center(), Align2::CENTER_CENTER, s, letra(15.0), col);
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, si, s));
    resp
}

/// Botón de metal con un dibujo adentro.
pub fn boton(ui: &mut Ui, tam: Vec2, si: bool, c: Color32, nombre: &str, dibujo: impl Fn(&Painter, Pos2, Color32)) -> Response {
    let (r, resp) = ui.allocate_exact_size(tam, Sense::click());
    if si {
        encendido(ui.painter(), r, 6, c);
    } else {
        relieve(ui.painter(), r, 6, CAJA, resp.hovered(), resp.is_pointer_button_down_on());
    }
    dibujo(ui.painter(), r.center(), if si { Color32::from_rgb(0x16, 0x16, 0x16) } else { c });
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, si, nombre));
    resp.on_hover_text(nombre)
}

/// Pantallita con números encendidos.
pub fn pantalla(ui: &mut Ui, s: &str, ancho: f32, n: f32) -> Response {
    let (r, resp) = ui.allocate_exact_size(vec2(ancho, 30.0), Sense::hover());
    hueco(ui.painter(), r, 5, HUECO);
    ui.painter().text(r.center(), Align2::CENTER_CENTER, s, letra(n), ACENTO.gamma_multiply(0.25));
    ui.painter().text(r.center(), Align2::CENTER_CENTER, s, letra(n), ACENTO);
    vidrio_en(ui.painter(), r.shrink(1.0), 4, Color32::from_rgb(0x30, 0x2c, 0x18), false, r.height() * 0.4);
    resp
}

/// Perilla de consola. Arrastrar arriba o abajo la mueve, Mayús afina y doble clic la regresa.
pub fn perilla(ui: &mut Ui, nombre: &str, v: &mut f32, min: f32, max: f32, fabrica: f32, c: Color32, tam: f32) -> Response {
    perilla_de(ui, nombre, v, min, max, fabrica, c, tam, Color32::from_gray(0x50))
}

/// La misma, con la tapa de otro metal.
#[allow(clippy::too_many_arguments)]
pub fn perilla_de(ui: &mut Ui, nombre: &str, v: &mut f32, min: f32, max: f32, fabrica: f32, c: Color32, tam: f32, metal: Color32) -> Response {
    let con = !nombre.is_empty();
    let ancho = if con { (tam + 22.0).max(60.0) } else { tam };
    let (r, mut resp) = ui.allocate_exact_size(vec2(ancho, tam + if con { 30.0 } else { 0.0 }), Sense::click_and_drag());
    let antes = *v;
    // Los rangos muy abiertos (frecuencias) se sienten parejos en escala logarítmica.
    let log = min > 0.0 && max / min > 20.0;
    let a01 = |x: f32| if log { (x / min).ln() / (max / min).ln() } else { (x - min) / (max - min) };
    let de01 = |x: f32| if log { min * (max / min).powf(x) } else { min + (max - min) * x };
    if resp.dragged() {
        let fino = if ui.input(|i| i.modifiers.shift) { 0.1 } else { 1.0 };
        let d = resp.drag_delta();
        *v = de01((a01(*v) + (d.x - d.y) / 160.0 * fino).clamp(0.0, 1.0));
    }
    if resp.double_clicked() {
        *v = fabrica;
    }
    if resp.hovered() {
        let y = ui.input(|i| i.smooth_scroll_delta.y);
        if y != 0.0 && ui.input(|i| i.modifiers.command) {
            *v = de01((a01(*v) + y / 1500.0).clamp(0.0, 1.0));
        }
    }
    if *v != antes {
        resp.mark_changed();
    }
    let p = ui.painter();
    let o = pos2(r.center().x, r.top() + tam / 2.0 + if con { 15.0 } else { 0.0 });
    let rad = tam / 2.0 - 2.0;
    let x = a01(*v).clamp(0.0, 1.0);
    let ang = |x: f32| PI * (0.75 + 1.5 * x);
    let arco = |a: f32, b: f32, w: f32, col: Color32| {
        if (b - a).abs() > 1e-3 {
            let v: Vec<Pos2> = (0..=24).map(|i| o + (rad - 1.0) * Vec2::angled(ang(a + (b - a) * i as f32 / 24.0))).collect();
            p.add(Shape::line(v, Stroke::new(w, col)));
        }
    };
    let hover = resp.hovered() || resp.dragged();
    let cero = if min < 0.0 && max > 0.0 { a01(0.0) } else { 0.0 };
    let g = (tam * 0.07).clamp(2.6, 5.0);
    super::torno::canal(p, o, rad - 1.0, g + 1.6);
    arco(cero, x, g + 3.6, c.gamma_multiply(0.16));
    arco(cero, x, g, if hover { mezcla(c, Color32::WHITE, 0.35) } else { c });
    arco(cero, x, g * 0.35, mezcla(c, Color32::WHITE, 0.55));
    super::torno::tapa(p, o, rad - g - 2.0, ang(x), metal, c, hover);
    if con {
        let pone = |s: String, y: f32, col: Color32| {
            let g = p.layout_no_wrap(s, letra(12.0), col);
            let x0 = (r.center().x - g.size().x / 2.0).max(r.left());
            p.with_clip_rect(r).galley(pos2(x0, y), g, col);
        };
        pone(nombre.to_string(), r.top(), TENUE);
        pone(numero(*v), r.bottom() - 15.0, TEXTO);
    }
    resp.widget_info(|| WidgetInfo::slider(true, *v as f64, nombre));
    if con { resp } else { resp.on_hover_text(numero(*v)) }
}

/// La perilla de los plugins: se mueve igual que las del programa y se ve como de aparato de antes.
#[allow(clippy::too_many_arguments)]
pub fn perilla_vieja(ui: &mut Ui, v: &mut f32, min: f32, max: f32, fabrica: f32, c: Color32, tam: f32, metal: Color32, k: f32) -> Response {
    let (r, mut resp) = ui.allocate_exact_size(Vec2::splat(tam), Sense::click_and_drag());
    let antes = *v;
    let log = min > 0.0 && max / min > 20.0;
    let a01 = |x: f32| if log { (x / min).ln() / (max / min).ln() } else { (x - min) / (max - min) };
    let de01 = |x: f32| if log { min * (max / min).powf(x) } else { min + (max - min) * x };
    if resp.dragged() {
        let fino = if ui.input(|i| i.modifiers.shift) { 0.1 } else { 1.0 };
        let d = resp.drag_delta();
        *v = de01((a01(*v) + (d.x - d.y) / 160.0 * fino).clamp(0.0, 1.0));
    }
    if resp.double_clicked() {
        *v = fabrica;
    }
    if resp.hovered() {
        let y = ui.input(|i| i.smooth_scroll_delta.y);
        if y != 0.0 && ui.input(|i| i.modifiers.command) {
            *v = de01((a01(*v) + y / 1500.0).clamp(0.0, 1.0));
        }
    }
    if *v != antes {
        resp.mark_changed();
    }
    // Deja lugar para la escala alrededor.
    let d = tam / 2.0 / 1.3;
    super::torno::perilla_vieja(ui.painter(), r.center(), d, a01(*v).clamp(0.0, 1.0), c, metal, resp.hovered() || resp.dragged(), k);
    resp.widget_info(|| WidgetInfo::slider(true, *v as f64, ""));
    resp.on_hover_text(numero(*v))
}

pub fn numero(v: f32) -> String {
    if v.abs() >= 100.0 {
        format!("{v:.0}")
    } else if v.abs() >= 10.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}

/// Medidor de lucecitas, hundido en la placa.
pub fn nivel(p: &Painter, r: Rect, v: [f32; 2]) {
    hueco(p, r, 3, HUECO);
    let dentro = r.shrink(2.0);
    for (i, x) in v.iter().enumerate() {
        let w = dentro.width() / 2.0 - 0.5;
        let b = Rect::from_min_size(pos2(dentro.left() + i as f32 * (w + 1.0), dentro.top()), vec2(w, dentro.height()));
        let db = 20.0 * x.max(1e-5).log10();
        let y = b.bottom() - b.height() * ((db + 54.0) / 60.0).clamp(0.0, 1.0);
        let en = |db: f32| b.bottom() - b.height() * (db + 54.0) / 60.0;
        for (a, z, c) in [(-54.0, -9.0, VERDE), (-9.0, -1.0, ACENTO), (-1.0, 6.0, ROJO)] {
            let (abajo, arriba) = (en(a), en(z).max(y));
            if arriba < abajo {
                p.rect_filled(Rect::from_min_max(pos2(b.left(), arriba), pos2(b.right(), abajo)), 0.0, c);
            }
        }
        let mut k = b.bottom() - 3.0;
        while k > y {
            p.hline(b.x_range(), k, Stroke::new(1.0, Color32::from_black_alpha(120)));
            k -= 3.0;
        }
    }
}

/// Tirador de consola: riel hundido y tapa con su raya.
pub fn tirador(ui: &mut Ui, r: Rect, v: &mut f32, max: f32, id: Id) -> Response {
    let resp = ui.interact(r, id, Sense::click_and_drag());
    let util = r.height() - 30.0;
    if resp.dragged() {
        let fino = if ui.input(|i| i.modifiers.shift) { 0.1 } else { 1.0 };
        *v = (*v - resp.drag_delta().y / util * max * fino).clamp(0.0, max);
    }
    if resp.double_clicked() {
        *v = 0.8;
    }
    let p = ui.painter();
    let riel = Rect::from_center_size(r.center(), vec2(5.0, r.height()));
    hueco(p, riel, 2, HUECO);
    for i in 0..=10 {
        let y = r.top() + 15.0 + util * i as f32 / 10.0;
        p.hline(r.left()..=riel.left() - 4.0, y, Stroke::new(1.0, Color32::from_white_alpha(if i % 5 == 0 { 50 } else { 22 })));
    }
    let y = r.bottom() - 15.0 - *v / max * util;
    p.rect_filled(Rect::from_min_max(pos2(riel.left() + 1.0, y), riel.max - vec2(1.0, 0.0)), 1.0, ACENTO.gamma_multiply(0.75));
    let b = Rect::from_center_size(pos2(r.center().x, y), vec2(22.0, 30.0));
    p.rect_filled(b.translate(vec2(0.0, 3.0)).expand(1.0), 4.0, Color32::from_black_alpha(120));
    relieve(p, b, 4, Color32::from_gray(if resp.hovered() { 0x78 } else { 0x62 }), false, false);
    for d in [-8.0, -5.0, 5.0, 8.0] {
        p.hline(b.x_range().shrink(4.0), y + d, Stroke::new(1.0, Color32::from_black_alpha(90)));
    }
    p.hline(b.x_range().shrink(1.0), y, Stroke::new(2.0, ACENTO));
    resp
}

/// Una fila con todo centrado a lo alto.
pub fn fila<R>(ui: &mut Ui, f: impl FnOnce(&mut Ui) -> R) -> R {
    ui.allocate_ui_with_layout(vec2(ui.available_width(), 30.0), Layout::left_to_right(Align::Center), f).inner
}

pub fn rueda(ui: &Ui, r: Rect) -> Vec2 {
    if ui.rect_contains_pointer(r) { ui.input(|i| i.smooth_scroll_delta) } else { Vec2::ZERO }
}

/// Canal en la mezcla: un bloque de color con vidrio encima (clips y notas).
pub fn bloque(p: &Painter, r: Rect, rad: u8, c: Color32, elegido: bool, hover: bool) {
    let c = if elegido { mezcla(c, Color32::WHITE, 0.55) } else { c };
    degradado(p, r, rad, mezcla(c, Color32::BLACK, 0.25), mezcla(c, Color32::BLACK, 0.55));
    vidrio(p, r, rad, c, hover);
}

pub const TITULO: f32 = 28.0;

/// Qué se pidió en la barra de título.
#[derive(Default)]
pub struct Pide {
    pub cerrar: bool,
    pub agrandar: bool,
    pub fijar: bool,
}

/// La placa y la barra de título de una ventana. `entera`: con agrandar y fijar como pestaña.
pub fn titulo(ui: &mut Ui, todo: Rect, nombre: &str, activa: bool, id: Id, entera: bool) -> Pide {
    let p = ui.painter();
    p.rect_filled(todo.translate(vec2(0.0, 4.0)).expand(3.0), 10.0, Color32::from_black_alpha(90));
    placa(p, todo, 8);
    p.rect_stroke(todo, 8.0, Stroke::new(1.0, if activa { ACENTO.gamma_multiply(0.45) } else { Color32::BLACK }), StrokeKind::Outside);
    let barra = Rect::from_min_size(todo.min, vec2(todo.width(), TITULO));
    p.hline(todo.x_range(), barra.bottom(), Stroke::new(1.0, Color32::BLACK));
    p.hline(todo.x_range(), barra.bottom() + 1.0, Stroke::new(1.0, Color32::from_white_alpha(18)));
    if activa {
        p.hline(barra.left() + 24.0..=barra.left() + 130.0, barra.top() + 1.0, Stroke::new(2.0, ACENTO));
    }
    tornillo(p, barra.left_center() + vec2(13.0, 0.0), 4.0, (id.value() % 7) as f32, false);
    rotulo(
        p,
        Rect::from_min_max(barra.min + vec2(26.0, 0.0), barra.max - vec2(if entera { 88.0 } else { 34.0 }, 0.0)),
        nombre,
        16.0,
        if activa { ACENTO } else { TEXTO },
    );
    let mut sale = [false; 3];
    let ayuda = ["Cerrar", "Agrandar", "Fijar como pestaña"];
    for i in 0..if entera { 3 } else { 1 } {
        let q = Rect::from_center_size(barra.right_center() - vec2(17.0 + 26.0 * i as f32, 0.0), vec2(20.0, 18.0));
        let b = ui.interact(q, id.with(i), Sense::click()).on_hover_text(crate::idioma::t(ayuda[i]));
        b.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, crate::idioma::t(ayuda[i])));
        relieve(ui.painter(), q, 4, CAJA, b.hovered(), b.is_pointer_button_down_on());
        let (o, st, p) = (q.center(), Stroke::new(1.4, TEXTO), ui.painter());
        match i {
            0 => {
                p.line_segment([o - vec2(3.5, 3.5), o + vec2(3.5, 3.5)], st);
                p.line_segment([o + vec2(-3.5, 3.5), o + vec2(3.5, -3.5)], st);
            }
            1 => _ = p.rect_stroke(Rect::from_center_size(o, vec2(8.0, 7.0)), 1.0, st, StrokeKind::Inside),
            _ => pestanita(p, o, TEXTO),
        }
        sale[i] = b.clicked();
    }
    Pide { cerrar: sale[0], agrandar: sale[1], fijar: sale[2] }
}

/// El dibujo de una pestaña.
pub fn pestanita(p: &Painter, o: Pos2, c: Color32) {
    let st = Stroke::new(1.4, c);
    p.add(Shape::line(
        vec![o + vec2(-5.5, 4.0), o + vec2(-5.5, -4.0), o + vec2(0.0, -4.0), o + vec2(1.5, -1.0), o + vec2(5.5, -1.0), o + vec2(5.5, 4.0)],
        st,
    ));
    p.hline(o.x - 6.5..=o.x + 6.5, o.y + 4.0, st);
}
