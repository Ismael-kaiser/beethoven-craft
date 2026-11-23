//! Ventana de atajos de teclado: buscar, cambiar, agregar, quitar y restablecer.
use super::estilo::*;
use super::*;
use crate::atajos::{self, ACCIONES, GRUPOS};
use crate::idioma::{t, tf};

#[derive(Default)]
pub struct Estado {
    pub abierto: bool,
    buscar: String,
    /// Acción que espera teclas y qué combinación reemplaza (None = agrega otra).
    espera: Option<(&'static str, Option<usize>)>,
    aviso: String,
}

impl App {
    pub fn ventana_atajos(&mut self, ctx: &Context) {
        if !self.atajos.abierto {
            return;
        }
        if let Some((id, cual)) = self.atajos.espera {
            let k = ctx.input_mut(|i| {
                let k = atajos::capturar(&i.events);
                if k.is_some() {
                    i.events.clear();
                }
                k
            });
            if let Some(k) = k {
                self.atajos.espera = None;
                if k.logical_key != Key::Escape || !k.modifiers.is_none() {
                    self.poner_atajo(id, cual, k);
                }
            }
        }
        let mut cerrar = false;
        let mut cambio = false;
        let esperando = self.atajos.espera;
        let m = Modal::new(Id::new("atajos")).show(ctx, |ui| {
            ui.set_width(620.0);
            ui.horizontal(|ui| {
                ui.heading(t("Atajos de teclado"));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add(TextEdit::singleline(&mut self.atajos.buscar).hint_text(t("Buscar…")).desired_width(170.0).margin(Margin::symmetric(8, 5)));
                });
            });
            ui.label(RichText::new(t("Clic en unas teclas para cambiarlas y aprieta las nuevas. Esc cancela. «+» agrega otras, «✕» las quita.")).size(13.0).color(TEXT_FAINT));
            let aviso = if esperando.is_some() { t("Aprieta las teclas nuevas…").to_string() } else { self.atajos.aviso.clone() };
            ui.label(RichText::new(if aviso.is_empty() { " ".into() } else { aviso }).color(MAIN));
            ui.separator();
            let q = self.atajos.buscar.to_lowercase();
            ScrollArea::vertical().max_height(430.0).auto_shrink([false, false]).show(ui, |ui| {
                for g in GRUPOS {
                    let v: Vec<_> = ACCIONES
                        .iter()
                        .filter(|a| a.grupo == g && (q.is_empty() || t(a.nombre).to_lowercase().contains(&q) || atajos::de(a.id).iter().any(|k| atajos::texto(k).to_lowercase() == q)))
                        .collect();
                    if v.is_empty() {
                        continue;
                    }
                    ui.add_space(6.0);
                    ui.label(RichText::new(t(g)).color(MAIN).size(15.0));
                    for a in v {
                        ui.horizontal(|ui| {
                            ui.allocate_ui_with_layout(vec2(290.0, 24.0), Layout::left_to_right(Align::Center), |ui| {
                                ui.set_width(290.0);
                                ui.add(Label::new(RichText::new(t(a.nombre)).color(TEXT_DIM)).truncate());
                            });
                            let puestas = atajos::de(a.id);
                            let mut quitar = None;
                            for (i, k) in puestas.iter().enumerate() {
                                let este = esperando == Some((a.id, Some(i)));
                                let txt = if este { "…".into() } else { atajos::texto(k) };
                                let b = ui.add(Button::new(RichText::new(txt).color(if este { COLORED_DARK } else { TEXT })).fill(if este { MAIN } else { BUTTON }).min_size(vec2(44.0, 24.0)));
                                b.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, format!("{}: {}", t(a.nombre), atajos::texto(k))));
                                if b.clicked() {
                                    self.atajos.espera = Some((a.id, Some(i)));
                                    self.atajos.aviso.clear();
                                }
                                if b.secondary_clicked() {
                                    quitar = Some(i);
                                }
                                let x = ui.add(Button::new(RichText::new("✕").size(11.0).color(TEXT_FAINT)).frame(false)).on_hover_text(t("Quitar"));
                                if x.clicked() {
                                    quitar = Some(i);
                                }
                            }
                            let nueva = esperando == Some((a.id, None));
                            let b = ui.add(Button::new(if nueva { "…" } else { "+" }).fill(if nueva { MAIN } else { BUTTON })).on_hover_text(t("Agregar teclas"));
                            b.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tf("Agregar teclas a {}", &[&t(a.nombre)])));
                            if b.clicked() {
                                self.atajos.espera = Some((a.id, None));
                                self.atajos.aviso.clear();
                            }
                            if atajos::cambiado(a.id) && ui.button(RichText::new(t("De fábrica")).size(12.5)).clicked() {
                                atajos::restablecer(Some(a.id));
                                cambio = true;
                            }
                            if let Some(i) = quitar {
                                let mut v = puestas;
                                v.remove(i);
                                atajos::poner(a.id, v);
                                cambio = true;
                            }
                        });
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(t("Regresar todos a los de fábrica")).clicked() {
                    atajos::restablecer(None);
                    self.atajos.aviso = t("Atajos de fábrica").into();
                    cambio = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button(t("Listo")).clicked() {
                        cerrar = true;
                    }
                });
            });
        });
        if cambio {
            self.atajos.espera = None;
            self.guardar_atajos();
        }
        if cerrar || (m.should_close() && esperando.is_none()) {
            self.atajos = Estado::default();
        }
    }

    fn poner_atajo(&mut self, id: &'static str, cual: Option<usize>, k: KeyboardShortcut) {
        self.atajos.aviso.clear();
        // Las teclas son de una sola acción: se le quitan a la otra.
        if let Some(otro) = atajos::dueno(&k, id) {
            let v = atajos::de(otro.id).into_iter().filter(|x| atajos::escribir(x) != atajos::escribir(&k)).collect();
            atajos::poner(otro.id, v);
            self.atajos.aviso = tf("{} era de «{}» y se le quitó", &[&atajos::texto(&k), &t(otro.nombre)]);
        }
        let mut v = atajos::de(id);
        v.retain(|x| atajos::escribir(x) != atajos::escribir(&k));
        match cual.filter(|i| *i < v.len()) {
            Some(i) => v[i] = k,
            None => v.push(k),
        }
        atajos::poner(id, v);
        self.guardar_atajos();
    }

    fn guardar_atajos(&self) {
        if !self.sin_dispositivo {
            inicio::guardar_ajustes();
        }
    }
}
