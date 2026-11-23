//! Editor de audio del clip elegido. Nada de lo que hace cambia el archivo original.
use super::estilo::*;
use super::*;

#[derive(Default)]
pub struct Estado {
    clip: Option<u64>,
    /// Selección en segundos de la línea de tiempo.
    pub sel: Option<(f64, f64)>,
    ancla: f64,
}

impl App {
    pub fn ui_editor(&mut self, ui: &mut Ui) {
        let id = self.clip.unwrap_or(0);
        if self.editor.clip != Some(id) {
            self.editor = Estado { clip: Some(id), ..Default::default() };
        }
        let Some(q) = self.p.clip(id).cloned() else {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(crate::idioma::t("Haz doble clic en un clip de audio para editarlo aquí")).size(16.0).color(TEXT_FAINT))
            });
            return;
        };
        let fin = q.inicio + q.dur;
        let sel = self.editor.sel.filter(|(a, b)| b - a > 0.01);
        let mut accion = "";
        Frame::new().inner_margin(Margin::symmetric(8, 4)).show(ui, |ui| {
            ui.horizontal(|ui| {
                let corto: String = q.nombre.chars().take(28).collect();
                ui.label(RichText::new(corto).size(15.0).color(MAIN)).on_hover_text(&q.nombre);
                ui.separator();
                let b = |ui: &mut Ui, activo: bool, t: &'static str, ayuda: &str, a: &mut &'static str| {
                    if ui.add_enabled(activo, Button::new(t)).on_hover_text(ayuda).clicked() {
                        *a = t;
                    }
                };
                let s = sel.is_some();
                b(ui, s, "Dejar solo la selección", "Recorta el clip a lo marcado", &mut accion);
                b(ui, s, "Silenciar", "Quita lo marcado y deja el hueco", &mut accion);
                b(ui, s, "Cortar y juntar", "Quita lo marcado y cierra el hueco", &mut accion);
                b(ui, true, "Dividir", "Parte el clip en el cursor de reproducción", &mut accion);
                ui.separator();
                b(ui, true, "Normalizar", "Sube el clip hasta casi el máximo sin distorsionar", &mut accion);
                b(ui, true, if q.invertido { "Quitar reversa" } else { "Reversa" }, "Suena al revés", &mut accion);
                b(ui, true, "Separar voz", "Voz e instrumental en dos pistas nuevas", &mut accion);
            });
        });
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            self.perillas_clip(ui, id);
            if let Some((a, b)) = sel {
                ui.separator();
                ui.label(RichText::new(crate::idioma::tf("Selección: {} – {} ({} s)", &[&(tiempo(a)), &(tiempo(b)), &format!("{:.2}", b - a)])).color(TEXT_DIM));
            }
        });
        let q = self.p.clip(id).unwrap().clone();
        let r = ui.available_rect_before_wrap().shrink2(vec2(8.0, 6.0));
        let resp = ui.interact(r, Id::new("editor-onda"), Sense::click_and_drag());
        let p = ui.painter_at(r);
        p.rect_filled(r, 6, WELL);
        p.hline(r.x_range(), r.center().y, Stroke::new(1.0, c(0x333333)));
        let a_x = |t: f64| r.left() + ((t - q.inicio) / q.dur) as f32 * r.width();
        let a_t = |x: f32| (q.inicio + ((x - r.left()) / r.width()) as f64 * q.dur).clamp(q.inicio, fin);
        if let Some(a) = self.almacen.get(&q.ruta) {
            linea::onda(&p, r, a, q.desde, q.desde + q.dur * q.velocidad, q.invertido, db(q.ganancia_db), DEEP);
        }
        for (f, izq) in [(q.fade_in, true), (q.fade_out, false)] {
            if f > 0.0 {
                let (a, b) = if izq { (a_x(q.inicio), a_x(q.inicio + f)) } else { (a_x(fin), a_x(fin - f)) };
                p.line_segment([pos2(a, r.bottom()), pos2(b, r.top())], Stroke::new(1.5, HIGHLIGHT));
            }
        }
        if let Some((a, b)) = self.editor.sel {
            p.rect_filled(Rect::from_min_max(pos2(a_x(a), r.top()), pos2(a_x(b), r.bottom())), 0, MAIN.gamma_multiply(0.25));
        }
        let t = self.ahora();
        if t >= q.inicio && t <= fin {
            p.vline(a_x(t), r.y_range(), Stroke::new(1.5, Color32::WHITE));
        }
        if let Some(m) = resp.interact_pointer_pos() {
            let tm = a_t(m.x);
            if resp.drag_started() {
                self.editor.ancla = a_t(ui.input(|i| i.pointer.press_origin()).unwrap_or(m).x);
            }
            if resp.dragged() {
                let a = self.editor.ancla;
                self.editor.sel = Some((a.min(tm), a.max(tm)));
            } else if resp.clicked() {
                self.editor.sel = None;
                self.ir(tm);
            }
        }
        match accion {
            "" => {}
            "Dejar solo la selección" => {
                if let Some((a, b)) = sel {
                    self.p.quitar_tramo(id, b, fin, false);
                    // El recorte de la izquierda conserva el resto, que pudo cambiar de id al dividir.
                    self.p.quitar_tramo(id, q.inicio, a, false);
                    self.reelegir(a);
                }
            }
            "Silenciar" | "Cortar y juntar" => {
                if let Some((a, b)) = sel {
                    self.p.quitar_tramo(id, a, b, accion != "Silenciar");
                    self.reelegir(q.inicio);
                }
            }
            "Dividir" => {
                let t = self.ahora();
                if !self.p.dividir(id, t) {
                    self.avisar(crate::idioma::t("Pon el cursor de reproducción dentro del clip"), false);
                }
            }
            "Normalizar" => {
                if let Some(a) = self.almacen.get(&q.ruta) {
                    let pico = a.pico(q.desde, q.desde + q.dur * q.velocidad);
                    if pico > 1e-4 {
                        self.p.clip(id).unwrap().ganancia_db = a_db(0.97 / pico).clamp(-24.0, 24.0);
                    }
                }
            }
            "Separar voz" => self.separar(q.ruta.clone(), q.inicio, Some(q), vec![]),
            _ => {
                let k = self.p.clip(id).unwrap();
                k.invertido = !k.invertido;
            }
        }
    }

    /// Tras una edición, vuelve a elegir el clip que empieza en `t` (dividir crea ids nuevos).
    fn reelegir(&mut self, t: f64) {
        self.editor.sel = None;
        let Canal::Pista(p) = self.canal else { return };
        self.clip = self
            .p
            .pista(p)
            .and_then(|p| p.clips.iter().filter(|c| c.inicio + c.dur > t).min_by(|a, b| a.inicio.total_cmp(&b.inicio)).map(|c| c.id));
    }
}
