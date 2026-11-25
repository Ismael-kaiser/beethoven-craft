// Beethoven Craft. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Colores, letra y controles de Movie. El metal y el vidrio están en `metal.rs`.
use super::iconos::Icono;
pub use super::metal::{degradado, degradado_en, encendido, engrane, esfera, hueco, indice, metalizar, mezcla, relieve, tornillo, vidrio, vidrio_en};
use eframe::egui::*;
use std::f32::consts::{PI, TAU};

/// Un color. Los grises oscuros de las superficies bajan más y toman un tono frío; lo demás queda igual.
pub const fn c(h: u32) -> Color32 {
    let (r, g, b) = ((h >> 16) as u8, (h >> 8) as u8, h as u8);
    if r == g && g == b && r < 0x70 {
        let v = r as u32;
        return Color32::from_rgb((v * 147 / 256) as u8, (v * 151 / 256) as u8, (v * 169 / 256 + 1) as u8);
    }
    Color32::from_rgb(r, g, b)
}

const fn x(h: u32) -> Color32 {
    Color32::from_rgb((h >> 16) as u8, (h >> 8) as u8, h as u8)
}

pub const MAIN: Color32 = x(0xffd95e);
pub const HIGHLIGHT: Color32 = x(0xfff0bd);
pub const COLORED_DARK: Color32 = x(0x2e2808);
pub const DEEP: Color32 = x(0xbd9d33);
pub const TOOLBAR: Color32 = x(0x222327);
pub const TOOLBAR_LINE: Color32 = x(0x111215);
pub const SUNKEN: Color32 = x(0x18191d);
pub const BUTTON: Color32 = x(0x38393e);
pub const BUTTON_HI: Color32 = x(0x45464c);
pub const GLYPH: Color32 = x(0xdedcd6);
pub const TEXT: Color32 = x(0xecebe6);
pub const TEXT_DIM: Color32 = x(0x96979e);
pub const TEXT_FAINT: Color32 = x(0x6e6f76);
pub const RECORD: Color32 = x(0xff5f5a);
pub const WORKSPACE: Color32 = x(0x16171a);
pub const WINDOW_TITLE: Color32 = x(0x1e1f23);
pub const WINDOW_BODY: Color32 = x(0x1b1c20);
pub const TRACK_HEADER: Color32 = x(0x27282d);
pub const RULER: Color32 = x(0x141519);
pub const RULER_TEXT: Color32 = x(0x8e9098);
pub const GRID: Color32 = x(0x17181b);
pub const GRID_ALT: Color32 = x(0x1a1b1f);
pub const GRID_GAP: Color32 = x(0x101114);
pub const GRID_BAR: Color32 = x(0x0a0b0d);
pub const STRIP: Color32 = x(0x26272b);
pub const STRIP_DARK: Color32 = x(0x17181b);
pub const SLOT: Color32 = x(0x1b1c20);
pub const WELL: Color32 = x(0x0c0d0f);
pub const METER_CLIP: Color32 = x(0xff4538);

/// Colores de pista, en el orden en que se crean.
pub const PISTAS: [Color32; 8] = [x(0xd4766e), x(0xd99a55), x(0xc9b84f), x(0x7fb46a), x(0x55b3a6), x(0x6a9fd4), x(0x9a86d6), x(0xcf7fb2)];

pub fn color_pista(i: usize) -> Color32 {
    PISTAS[i % PISTAS.len()]
}

/// Letra de la interfaz (Cuprum).
pub fn fuente(t: f32) -> FontId {
    FontId::proportional(t)
}

/// Letra de números y títulos (Barlow).
pub fn barlow(t: f32) -> FontId {
    FontId::new(t, FontFamily::Name("barlow".into()))
}

/// Letras, colores y medidas de toda la app.
pub fn aplicar(ctx: &Context) {
    ctx.set_theme(Theme::Dark);
    let mut f = FontDefinitions::default();
    let cuprum = FontData::from_static(include_bytes!("../../recursos/Cuprum.ttf")).tweak(FontTweak { y_offset_factor: 0.06, ..Default::default() });
    let barlow = FontData::from_static(include_bytes!("../../recursos/BarlowSemiCondensed-SemiBold.ttf"));
    f.font_data.insert("cuprum".into(), cuprum.into());
    f.font_data.insert("barlow".into(), barlow.into());
    f.families.entry(FontFamily::Proportional).or_default().insert(0, "cuprum".into());
    // De respaldo, para las flechas y figuras que Cuprum no trae.
    f.families.entry(FontFamily::Proportional).or_default().push("Hack".into());
    let mut numeros = f.families[&FontFamily::Proportional].clone();
    numeros.insert(0, "barlow".into());
    f.families.insert(FontFamily::Name("barlow".into()), numeros);
    ctx.set_fonts(f);
    ctx.all_styles_mut(|s| {
        use super::metal::{BORDE_ABIERTO, BORDE_APRETADO, BORDE_ENCIMA, BORDE_REPOSO};
        let v = &mut s.visuals;
        v.panel_fill = WINDOW_TITLE;
        v.window_fill = WINDOW_BODY;
        v.window_stroke = Stroke::new(1.0, Color32::BLACK);
        v.window_corner_radius = 8.into();
        v.menu_corner_radius = 6.into();
        v.extreme_bg_color = WELL;
        v.faint_bg_color = Color32::from_white_alpha(5);
        v.override_text_color = Some(TEXT);
        v.hyperlink_color = MAIN;
        v.selection.bg_fill = MAIN.gamma_multiply(0.32);
        v.selection.stroke = Stroke::new(1.0, MAIN);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, TOOLBAR_LINE);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
        for (w, fondo, borde) in [
            (&mut v.widgets.inactive, BUTTON, BORDE_REPOSO),
            (&mut v.widgets.hovered, BUTTON_HI, BORDE_ENCIMA),
            (&mut v.widgets.active, BUTTON_HI, BORDE_APRETADO),
            (&mut v.widgets.open, BUTTON_HI, BORDE_ABIERTO),
        ] {
            w.bg_fill = fondo;
            w.weak_bg_fill = fondo;
            w.bg_stroke = Stroke::new(1.0, borde);
            w.fg_stroke = Stroke::new(1.0, TEXT);
            w.expansion = 0.0;
            w.corner_radius = 5.into();
        }
        s.spacing.item_spacing = vec2(6.0, 6.0);
        s.spacing.button_padding = vec2(8.0, 3.0);
        s.spacing.interact_size.y = 22.0;
        for (k, n) in [(TextStyle::Body, 15.0), (TextStyle::Button, 15.0), (TextStyle::Heading, 19.0), (TextStyle::Small, 12.0)] {
            s.text_styles.insert(k, fuente(n));
        }
        s.text_styles.insert(TextStyle::Monospace, FontId::monospace(13.0));
    });
}

/// Un arco de `a` radianes en adelante (en el sentido del reloj).
pub fn arco(p: &Painter, o: Pos2, r: f32, a: f32, barrido: f32, w: f32, col: Color32) {
    let n = ((barrido.abs() * r / 3.0).ceil() as usize).clamp(2, 96);
    let v: Vec<Pos2> = (0..=n).map(|i| o + r * Vec2::angled(a + barrido * i as f32 / n as f32)).collect();
    p.add(Shape::line(v, Stroke::new(w, col)));
}

/// Dibuja un ícono (rejilla de 24) en el cuadro `r`.
pub fn icono(p: &Painter, i: &Icono, r: Rect, col: Color32) {
    let k = r.width().min(r.height()) / 24.0;
    let o = r.center() - vec2(12.0, 12.0) * k;
    i.pintar(p, |x, y| o + vec2(x, y) * k, 1.9 * k, col);
}

/// Botón cuadrado con ícono. `on`: encendido.
pub fn boton(ui: &mut Ui, i: &Icono, on: bool, ayuda: &str) -> Response {
    boton_color(ui, i, on, ayuda, GLYPH, 30.0)
}

/// Como `boton`, con el color del dibujo y el tamaño a elegir.
pub fn boton_color(ui: &mut Ui, i: &Icono, on: bool, ayuda: &str, col: Color32, lado: f32) -> Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(lado), Sense::click());
    if on {
        encendido(ui.painter(), r, 6, MAIN);
    } else {
        relieve(ui.painter(), r, 6, BUTTON, resp.hovered(), resp.is_pointer_button_down_on());
    }
    let tinta = if on { COLORED_DARK } else if ui.is_enabled() { col } else { TEXT_FAINT };
    icono(ui.painter(), i, r.shrink(lado * 0.2), tinta);
    let nombre = crate::idioma::t(ayuda).to_string();
    let activo = ui.is_enabled();
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, activo, on, &nombre));
    resp.on_hover_text(nombre.clone())
}

/// El foquito: hundido en el tablero, encendido en amarillo.
pub fn pinta_led(p: &Painter, o: Pos2, h: f32, on: bool, hover: bool) {
    let r = Rect::from_center_size(o, Vec2::splat(h));
    hueco(p, r, 3, WELL);
    let dentro = r.shrink(2.0);
    if on {
        encendido(p, dentro, 2, MAIN);
    } else {
        p.rect_filled(dentro, 2, if hover { BUTTON_HI } else { BUTTON });
    }
}

/// Interruptor chico.
pub fn led(ui: &mut Ui, on: bool, ayuda: &str) -> Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
    pinta_led(ui.painter(), r.center(), 14.0, on, resp.hovered());
    let nombre = crate::idioma::t(ayuda).to_string();
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, on, &nombre));
    resp.on_hover_text(nombre.clone())
}

/// Botoncito de una letra (S, G…), encendido con su color.
pub fn letra(ui: &mut Ui, s: &str, on: bool, col: Color32, ayuda: &str) -> Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
    if on {
        encendido(ui.painter(), r, 5, col);
    } else {
        relieve(ui.painter(), r, 5, BUTTON, resp.hovered(), resp.is_pointer_button_down_on());
    }
    let tinta = if on { x(0x15150f) } else { TEXT_DIM };
    ui.painter().text(r.center(), Align2::CENTER_CENTER, s, fuente(13.0), tinta);
    let nombre = crate::idioma::t(ayuda).to_string();
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, &nombre));
    resp.on_hover_text(nombre.clone())
}

/// Perilla: aro con el valor y una tapa torneada. `v` de 0 a 1; con `bipolar` el aro nace del centro.
pub fn pinta_perilla(p: &Painter, o: Pos2, r: f32, v: f32, bipolar: bool, hover: bool) {
    let (desde, vuelta) = (PI * 0.75, PI * 1.5);
    let v = v.clamp(0.0, 1.0);
    let angulo = desde + vuelta * v;
    arco(p, o, r - 1.0, desde, vuelta, 2.4, Color32::from_black_alpha(170));
    let nace = if bipolar { desde + vuelta * 0.5 } else { desde };
    if (angulo - nace).abs() > 0.02 {
        arco(p, o, r - 1.0, nace, angulo - nace, 2.4, if hover { HIGHLIGHT } else { MAIN });
    }
    let t = r - 4.0;
    p.circle_filled(o + vec2(0.0, 2.0), t + 0.5, Color32::from_black_alpha(130));
    p.circle_filled(o, t, x(0x232428));
    p.circle_filled(o - vec2(0.0, 0.8), t - 1.0, if hover { x(0x6a6c73) } else { x(0x575960) });
    p.circle_filled(o, t - 2.2, x(0x3a3b41));
    // Luz de arriba y marcas del torno.
    arco(p, o, t - 1.2, PI * 1.15, PI * 0.7, 1.0, Color32::from_white_alpha(90));
    for k in 0..16 {
        let d = Vec2::angled(angulo + k as f32 * TAU / 16.0);
        p.line_segment([o + d * (t - 2.0), o + d * (t - 0.6)], Stroke::new(0.8, Color32::from_black_alpha(90)));
    }
    let d = Vec2::angled(angulo);
    p.line_segment([o + d * t * 0.25, o + d * (t - 2.0)], Stroke::new(2.0, Color32::from_gray(235)));
}

/// Perilla sin texto. Arrastrar arriba o abajo la mueve (con Mayús, fino) y doble clic la regresa a `def`.
#[allow(clippy::too_many_arguments)]
pub fn perilla(ui: &mut Ui, v: &mut f32, min: f32, max: f32, def: f32, log: bool, r: f32, nombre: &str) -> Response {
    let (q, mut resp) = ui.allocate_exact_size(Vec2::splat(r * 2.0 + 2.0), Sense::click_and_drag());
    let log = log && min > 0.0;
    let a_parte = |v: f32| if log { (v / min).ln() / (max / min).ln() } else { (v - min) / (max - min) };
    let de_parte = |p: f32| if log { min * (max / min).powf(p) } else { min + p * (max - min) };
    if resp.double_clicked() {
        *v = def;
        resp.mark_changed();
    } else if resp.dragged() {
        let fino = if ui.input(|i| i.modifiers.shift) { 0.1 } else { 1.0 };
        let d = resp.drag_delta();
        if d != Vec2::ZERO {
            *v = de_parte((a_parte(*v) + (d.x - d.y) / 160.0 * fino).clamp(0.0, 1.0)).clamp(min, max);
            resp.mark_changed();
        }
    }
    pinta_perilla(ui.painter(), q.center(), r, a_parte(*v), !log && min < 0.0 && max > 0.0, resp.hovered() || resp.dragged());
    let nombre = crate::idioma::t(nombre).to_string();
    let valor = *v as f64;
    resp.widget_info(|| WidgetInfo::slider(true, valor, &nombre));
    resp
}

/// Medidor de dos canales (`pico`: de 0 a 1 o más): sube en barra y se pone rojo al pasar de 0 dB.
pub fn medidor(p: &Painter, r: Rect, pico: [f32; 2]) {
    hueco(p, r, 3, WELL);
    let dentro = r.shrink(2.0);
    let w = (dentro.width() - 1.0) / 2.0;
    for (k, x_) in pico.into_iter().enumerate() {
        // De -60 a +6 dB.
        let alto = ((crate::modelo::a_db(x_) + 60.0) / 66.0).clamp(0.0, 1.0);
        if alto <= 0.0 {
            continue;
        }
        let izq = dentro.left() + k as f32 * (w + 1.0);
        let barra = Rect::from_min_max(pos2(izq, dentro.bottom() - dentro.height() * alto), pos2(izq + w, dentro.bottom()));
        let (arriba, abajo) = if x_ > 1.0 { (METER_CLIP, x(0x9a2a22)) } else { (mezcla(DEEP, HIGHLIGHT, alto), DEEP.gamma_multiply(0.8)) };
        degradado(p, barra, 1, arriba, abajo);
    }
}

/// Minutos, segundos y centésimas.
pub fn tiempo(s: f64) -> String {
    let c = (s.max(0.0) * 100.0) as u64;
    format!("{}:{:02}:{:02}", c / 6000, c / 100 % 60, c % 100)
}
