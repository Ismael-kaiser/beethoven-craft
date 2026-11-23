//! Panel de color y medidores de imagen.
use super::estilo::*;
use super::*;
use crate::composicion::fx_nuevo;

/// Corrección de color completa de EffectCraft.
const LUMETRI: &str = "ec.color.lumetri";

/// (grupo, nombre, control, mínimo, máximo, neutro)
const CONTROLES: [(&str, &str, &str, f64, f64, f64); 15] = [
    ("Básico", "Temperatura", "basicCorrection/whiteBalance/temperature", -100.0, 100.0, 0.0),
    ("Básico", "Tinte", "basicCorrection/whiteBalance/tint", -100.0, 100.0, 0.0),
    ("Básico", "Exposición", "basicCorrection/tone/exposure", -4.0, 4.0, 0.0),
    ("Básico", "Contraste", "basicCorrection/tone/contrast", -100.0, 100.0, 0.0),
    ("Básico", "Luces altas", "basicCorrection/tone/highlights", -100.0, 100.0, 0.0),
    ("Básico", "Sombras", "basicCorrection/tone/shadows", -100.0, 100.0, 0.0),
    ("Básico", "Blancos", "basicCorrection/tone/whites", -100.0, 100.0, 0.0),
    ("Básico", "Negros", "basicCorrection/tone/blacks", -100.0, 100.0, 0.0),
    ("Básico", "Saturación", "basicCorrection/saturation", 0.0, 200.0, 100.0),
    ("Creativo", "Película deslavada", "creative/adjustments/fadedFilm", 0.0, 100.0, 0.0),
    ("Creativo", "Nitidez", "creative/adjustments/sharpen", -100.0, 100.0, 0.0),
    ("Creativo", "Intensidad", "creative/adjustments/vibrance", -100.0, 100.0, 0.0),
    ("Viñeta", "Cantidad", "vignette/amount", -5.0, 5.0, 0.0),
    ("Viñeta", "Punto medio", "vignette/midpoint", 0.0, 100.0, 50.0),
    ("Viñeta", "Difuminado", "vignette/feather", 0.0, 100.0, 50.0),
];

/// Estilos: cambios sobre lo neutro.
const ESTILOS: [(&str, &[(&str, f64)]); 7] = [
    ("Cálido", &[("basicCorrection/whiteBalance/temperature", 35.0), ("basicCorrection/tone/contrast", 12.0), ("creative/adjustments/vibrance", 15.0)]),
    ("Frío", &[("basicCorrection/whiteBalance/temperature", -35.0), ("basicCorrection/tone/contrast", 10.0)]),
    ("Cine", &[("basicCorrection/tone/contrast", 25.0), ("basicCorrection/tone/highlights", -25.0), ("basicCorrection/tone/shadows", 15.0), ("basicCorrection/saturation", 85.0), ("vignette/amount", -1.2)]),
    ("Blanco y negro", &[("basicCorrection/saturation", 0.0), ("basicCorrection/tone/contrast", 30.0)]),
    ("Antiguo", &[("creative/adjustments/fadedFilm", 60.0), ("basicCorrection/whiteBalance/temperature", 25.0), ("basicCorrection/saturation", 70.0), ("vignette/amount", -1.5)]),
    ("Vivo", &[("basicCorrection/saturation", 130.0), ("basicCorrection/tone/contrast", 20.0), ("creative/adjustments/sharpen", 20.0)]),
    ("Noche", &[("basicCorrection/tone/exposure", -0.8), ("basicCorrection/whiteBalance/temperature", -45.0), ("basicCorrection/tone/contrast", 20.0)]),
];

impl App {
    pub fn ui_color(&mut self, ui: &mut Ui) {
        self.medidores_imagen(ui);
        let Some(id) = self.clip.filter(|id| self.p.video.iter().any(|c| c.id == *id)) else {
            ui.add_space(16.0);
            ui.vertical_centered(|ui| ui.label(RichText::new(crate::idioma::t("Elige un clip de video para corregir su color")).color(TEXT_FAINT)));
            return;
        };
        let nuevo_id = self.p.sig_id + 1;
        let t = {
            let c = self.p.clip_video(id).unwrap();
            let ini = c.inicio;
            (ini, c.dur)
        };
        let t = (self.ahora() - t.0).clamp(0.0, t.1);
        let c = self.p.clip_video(id).unwrap();
        // La corrección se agrega sola al primer cambio, en neutro.
        let neutro = || {
            let mut f = fx_nuevo(nuevo_id, LUMETRI).unwrap();
            for (_, _, k, _, _, def) in CONTROLES {
                if let Some(p) = f.p.get_mut(k) {
                    p.v = Valor::Num(def);
                }
            }
            f
        };
        let i = c.fx.iter().position(|f| f.tipo == LUMETRI);
        let mut f = i.map_or_else(neutro, |i| c.fx[i].clone());
        let antes = f.clone();
        ui.horizontal(|ui| {
            if led(ui, f.activo, "Encender la corrección de color").clicked() {
                f.activo = !f.activo;
            }
            ui.menu_button(crate::idioma::t("Estilos"), |ui| {
                for (n, v) in ESTILOS {
                    if ui.button(crate::idioma::t(&n)).clicked() {
                        let (id, activo) = (f.id, f.activo);
                        f = FxVideo { id, activo, ..neutro() };
                        for (k, x) in v {
                            if let Some(p) = f.p.get_mut(*k) {
                                p.v = Valor::Num(*x);
                            }
                        }
                    }
                }
            });
            if ui.button(crate::idioma::t("Restablecer")).clicked() {
                f = FxVideo { id: f.id, ..neutro() };
            }
        });
        ScrollArea::vertical().id_salt("color").auto_shrink([false, false]).show(ui, |ui| {
            let mut g = "";
            for (grupo, n, k, min, max, def) in CONTROLES {
                let Some(p) = f.p.get_mut(k) else { continue };
                if g != grupo {
                    g = grupo;
                    ui.add_space(4.0);
                    ui.label(RichText::new(crate::idioma::t(&grupo)).size(15.0));
                }
                inspector::numero(ui, n, p, t, min, max, def, "");
            }
        });
        if f != antes {
            match i {
                Some(i) => c.fx[i] = f,
                None => {
                    c.fx.push(f);
                    self.p.sig_id = nuevo_id;
                }
            }
        }
    }

    /// Forma de onda RGB, histograma y vectorscopio del cuadro que se ve.
    fn medidores_imagen(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            for (i, n) in ["Onda RGB", "Histograma", "Vectorscopio"].into_iter().enumerate() {
                if biblioteca::pastilla(ui, n, self.medidor_img == i).clicked() {
                    self.medidor_img = i;
                    self.medidor_v = u64::MAX;
                }
            }
        });
        let (w, h) = (256usize, 128usize);
        if self.medidor_v != self.cuadro_v {
            self.medidor_v = self.cuadro_v;
            if let Some((an, al, q)) = &self.cuadro {
                let img = medir(self.medidor_img, *an, *al, q, w, h);
                match &mut self.medidor_tex {
                    Some(t) => t.set(img, TextureOptions::LINEAR),
                    None => self.medidor_tex = Some(ui.ctx().load_texture("medidor", img, TextureOptions::LINEAR)),
                }
            }
        }
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::hover());
        ui.painter().rect_filled(r, 4, WELL);
        if let Some(t) = &self.medidor_tex {
            let d = if self.medidor_img == 2 { Rect::from_center_size(r.center(), vec2(r.height(), r.height()) - Vec2::splat(6.0)) } else { r.shrink(4.0) };
            ui.painter().image(t.id(), d, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        ui.add_space(4.0);
    }
}

pub fn medir(que: usize, an: usize, al: usize, q: &[u8], w: usize, h: usize) -> ColorImage {
    let w = if que == 2 { h } else { w };
    let mut n = vec![[0u32; 3]; w * h];
    for y in (0..al).step_by(2) {
        for x in (0..an).step_by(2) {
            let p = &q[(y * an + x) * 4..][..3];
            match que {
                0 => {
                    // Tres ondas lado a lado.
                    for k in 0..3 {
                        let cx = (k * w + x * w / an.max(1)) / 3;
                        let cy = (255 - p[k] as usize) * (h - 1) / 255;
                        n[cy * w + cx.min(w - 1)][k] += 1;
                    }
                }
                1 => {
                    for k in 0..3 {
                        n[p[k] as usize * (w - 1) / 255][k] += 1;
                    }
                }
                _ => {
                    let (r, g, b) = (p[0] as f32, p[1] as f32, p[2] as f32);
                    let (cb, cr) = (-0.169 * r - 0.331 * g + 0.5 * b, 0.5 * r - 0.419 * g - 0.081 * b);
                    let cx = ((cb / 128.0 + 1.0) * 0.5 * (w - 1) as f32) as usize;
                    let cy = ((1.0 - cr / 128.0) * 0.5 * (h - 1) as f32) as usize;
                    let d = &mut n[cy.min(h - 1) * w + cx.min(w - 1)];
                    (0..3).for_each(|k| d[k] += 1 + p[k] as u32 / 64);
                }
            }
        }
    }
    let mut o = vec![0u8; w * h * 4];
    if que == 1 {
        let max = n[..w].iter().flatten().copied().max().unwrap_or(1).max(1) as f32;
        for x in 0..w {
            for k in 0..3 {
                let a = ((n[x][k] as f32 / max).sqrt() * h as f32) as usize;
                for y in h - a.min(h)..h {
                    o[(y * w + x) * 4 + k] = 190;
                }
            }
        }
    } else {
        for (d, c) in o.chunks_exact_mut(4).zip(&n) {
            for k in 0..3 {
                d[k] = ((c[k] as f32).sqrt() * if que == 0 { 45.0 } else { 22.0 }).min(255.0) as u8;
            }
        }
    }
    o.chunks_exact_mut(4).for_each(|d| d[3] = 255);
    ColorImage::from_rgba_unmultiplied([w, h], &o)
}
