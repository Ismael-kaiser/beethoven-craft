//! Ajustes del proyecto: tamaño del cuadro y fotogramas por segundo.
use super::estilo::*;
use super::*;
use crate::idioma::t;

pub const TAMANOS: [(&str, u32, u32); 12] = [
    ("Full HD · 1920×1080", 1920, 1080),
    ("HD · 1280×720", 1280, 720),
    ("2K · 2560×1440", 2560, 1440),
    ("4K · 3840×2160", 3840, 2160),
    ("Cine 4K · 4096×2160", 4096, 2160),
    ("Cine ancho · 2560×1080", 2560, 1080),
    ("Clásico 4:3 · 1440×1080", 1440, 1080),
    ("Vertical · 1080×1920", 1080, 1920),
    ("Vertical HD · 720×1280", 720, 1280),
    ("Retrato 4:5 · 1080×1350", 1080, 1350),
    ("Cuadrado · 1080×1080", 1080, 1080),
    ("Cuadrado 4K · 2160×2160", 2160, 2160),
];

pub const FPS: [f64; 10] = [12.0, 24000.0 / 1001.0, 24.0, 25.0, 30000.0 / 1001.0, 30.0, 48.0, 50.0, 60000.0 / 1001.0, 60.0];

pub fn nombre_fps(f: f64) -> String {
    let s = format!("{f:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Lo que se está cambiando antes de aceptar.
#[derive(Clone)]
pub struct Ajustes {
    pub ancho: u32,
    pub alto: u32,
    pub fps: f64,
    /// Qué hacer con los clips si cambia la forma: 0 nada, 1 completo, 2 llenar.
    pub clips: usize,
}

impl App {
    pub fn abrir_ajustes_proyecto(&mut self) {
        self.ajustes_proyecto = Some(Ajustes { ancho: self.p.ancho, alto: self.p.alto, fps: self.p.fps, clips: 1 });
    }

    pub fn dialogo_proyecto(&mut self, ctx: &Context) {
        let Some(mut a) = self.ajustes_proyecto.clone() else { return };
        let (mut cerrar, mut listo) = (false, false);
        Modal::new(Id::new("ajustes-proyecto")).show(ctx, |ui| {
            ui.set_width(470.0);
            ui.heading(t("Ajustes del proyecto"));
            ui.label(RichText::new(t("Cómo se ve y se exporta todo el video.")).color(TEXT_FAINT).size(13.0));
            ui.add_space(8.0);
            ui.label(RichText::new(t("Resolución")).color(MAIN).size(14.0));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
                for (n, w, h) in TAMANOS {
                    if biblioteca::pastilla(ui, n, (a.ancho, a.alto) == (w, h)).clicked() {
                        (a.ancho, a.alto) = (w, h);
                    }
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(t("A la medida")).color(TEXT_DIM));
                let w = ui.add(DragValue::new(&mut a.ancho).range(16..=8192).suffix(" px"));
                w.widget_info(|| WidgetInfo::labeled(WidgetType::DragValue, true, t("Ancho del proyecto")));
                ui.label("×");
                let h = ui.add(DragValue::new(&mut a.alto).range(16..=8192).suffix(" px"));
                h.widget_info(|| WidgetInfo::labeled(WidgetType::DragValue, true, t("Alto del proyecto")));
                if ui.button(t("Girar")).on_hover_text(t("Cambia entre horizontal y vertical")).clicked() {
                    (a.ancho, a.alto) = (a.alto, a.ancho);
                }
                // La forma, dibujada.
                let (r, _) = ui.allocate_exact_size(vec2(54.0, 34.0), Sense::hover());
                let k = (r.width() / a.ancho as f32).min(r.height() / a.alto as f32);
                ui.painter().rect_stroke(Rect::from_center_size(r.center(), vec2(a.ancho as f32, a.alto as f32) * k), 2, Stroke::new(1.5, MAIN), StrokeKind::Inside);
            });
            ui.add_space(8.0);
            ui.label(RichText::new(t("Fotogramas por segundo")).color(MAIN).size(14.0));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
                for f in FPS {
                    if biblioteca::pastilla(ui, &nombre_fps(f), (a.fps - f).abs() < 0.005).clicked() {
                        a.fps = f;
                    }
                }
                ui.add(DragValue::new(&mut a.fps).range(1.0..=120.0).speed(0.1).max_decimals(3).suffix(" cps"));
            });
            if a.ancho as u64 * self.p.alto as u64 != a.alto as u64 * self.p.ancho as u64 && self.p.video.iter().any(|c| c.ancho > 0) {
                ui.add_space(8.0);
                ui.label(RichText::new(t("La forma cambia. ¿Qué hago con los clips?")).color(TEXT_DIM));
                ui.horizontal(|ui| {
                    for (i, n) in [(1, "Que se vean completos"), (2, "Que llenen el cuadro"), (0, "Dejarlos igual")] {
                        if biblioteca::pastilla(ui, n, a.clips == i).clicked() {
                            a.clips = i;
                        }
                    }
                });
            }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.add_sized([130.0, 32.0], Button::new(RichText::new(t("Aplicar")).color(MAIN).size(16.0))).clicked() {
                    listo = true;
                }
                if ui.add_sized([100.0, 32.0], Button::new(t("Cancelar"))).clicked() {
                    cerrar = true;
                }
            });
        });
        if listo {
            self.aplicar_ajustes(&a);
        }
        self.ajustes_proyecto = (!cerrar && !listo).then_some(a);
    }

    pub fn aplicar_ajustes(&mut self, a: &Ajustes) {
        let cambia = a.ancho as u64 * self.p.alto as u64 != a.alto as u64 * self.p.ancho as u64;
        (self.p.ancho, self.p.alto, self.p.fps, self.p.fijo) = (a.ancho / 2 * 2, a.alto / 2 * 2, a.fps, true);
        if cambia && a.clips > 0 {
            // Los que el usuario ya animó se dejan como están.
            let v: Vec<u64> = self.p.video.iter().filter(|c| c.ancho > 0 && c.mov.props().iter().all(|q| q.claves.is_empty())).map(|c| c.id).collect();
            for id in v {
                self.acomodar(id, a.clips - 1);
            }
        }
    }
}
