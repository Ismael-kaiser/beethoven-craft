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

/// Rectángulo redondeado con degradado vertical (arriba, abajo).
pub fn degradado(p: &Painter, r: Rect, rad: u8, a: Color32, b: Color32) {
    p.rect_filled(r, CornerRadius::same(rad), b);
    let alto = r.height();
    let n = 8;
    for i in 0..n {
        let f = i as f32 / n as f32;
        // Franjas recortadas del mismo rectángulo redondeado: así las esquinas quedan limpias.
        let franja = Rect::from_min_max(
            pos2(r.left(), r.top() + alto * f),
            pos2(r.right(), r.top() + alto * (f + 1.0 / n as f32)),
        );
        p.with_clip_rect(franja.intersect(p.clip_rect())).rect_filled(
            r,
            CornerRadius::same(rad),
            mezcla(a, b, f),
        );
    }
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
    if on || resp.is_pointer_button_down_on() {
        degradado(p, rr, 6, mezcla(HIGHLIGHT, MAIN, 0.45), MAIN);
        tinta = COLORED_DARK;
    } else {
        p.rect_filled(rr, 6, if resp.hovered() { BUTTON_HI } else { BUTTON });
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
    p.rect_filled(r, 3, if hover { c(0x767676) } else { c(0x212121) });
    if on {
        degradado(p, r.shrink(1.0), 2, HIGHLIGHT, MAIN);
    } else {
        p.rect_filled(r.shrink(1.0), 2, mezcla(COLORED_DARK, TOOLBAR_LINE, 0.35));
    }
}

/// Botoncito de letra (S, G…).
pub fn letra(ui: &mut Ui, s: &str, on: bool, col: Color32, ayuda: &str) -> Response {
    let ayuda = crate::idioma::t(ayuda);
    let (r, resp) = ui.allocate_exact_size(vec2(20.0, 18.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, on, ayuda));
    let f = if on { col } else if resp.hovered() { BUTTON_HI } else { BUTTON };
    ui.painter().rect_filled(r, 4, f);
    let tinta = if on { c(0x1c1c1c) } else { TEXT_DIM };
    if s == "●" {
        ui.painter().circle_filled(r.center(), 4.5, tinta);
    } else {
        ui.painter().text(r.center(), Align2::CENTER_CENTER, s, barlow(12.0), tinta);
    }
    resp.on_hover_text(ayuda)
}

pub fn pinta_perilla(p: &Painter, o: Pos2, r: f32, v: f32, bipolar: bool, hover: bool) {
    let v = v.clamp(0.0, 1.0);
    let (ini, bar) = (0.75 * PI, 1.5 * PI);
    let a = ini + bar * v;
    let w = (r * 0.16).clamp(1.6, 2.6);
    arco(p, o, r - w / 2.0, ini, bar, w, c(0x1d1d1d));
    let a0 = if bipolar { ini + bar / 2.0 } else { ini };
    if (a - a0).abs() > 0.02 {
        arco(p, o, r - w / 2.0, a0, a - a0, w, if hover { HIGHLIGHT } else { MAIN });
    }
    let disco = r - w - 1.5;
    let (arr, aba) = if hover { (c(0x5c5c5c), c(0x404040)) } else { (c(0x525252), c(0x383838)) };
    p.circle_filled(o, disco, aba);
    p.circle_filled(o - vec2(0.0, disco * 0.12), disco * 0.86, mezcla(arr, aba, 0.35));
    let d = Vec2::angled(a);
    p.line_segment(
        [o + d * disco * 0.25, o + d * (disco - 1.5).max(1.0)],
        Stroke::new((disco * 0.22).clamp(1.4, 2.2), c(0xe6e6e6)),
    );
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
    p.rect_filled(r, 3, WELL);
    let w = (r.width() - 1.0) / 2.0;
    for (k, x) in pico.into_iter().enumerate() {
        let d = crate::modelo::a_db(x);
        let f = ((d + 54.0) / 60.0).clamp(0.0, 1.0);
        let x0 = r.left() + k as f32 * (w + 1.0);
        let y = r.bottom() - r.height() * f;
        let b = Rect::from_min_max(pos2(x0, y), pos2(x0 + w, r.bottom()));
        let col = if d > 0.0 { METER_CLIP } else { mezcla(DEEP, HIGHLIGHT, f * f) };
        p.rect_filled(b, 2, col);
    }
    // Marca de 0 dB.
    let y0 = r.bottom() - r.height() * 0.9;
    p.hline(r.x_range(), y0, Stroke::new(1.0, c(0x4a4a4a)));
}

pub fn tiempo(s: f64) -> String {
    let s = s.max(0.0);
    format!("{}:{:02}:{:02}", s as u64 / 60, s as u64 % 60, (s.fract() * 100.0) as u64)
}
