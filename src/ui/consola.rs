//! Consola (una tira por pista) y panel de efectos del canal elegido.
use super::estilo::*;
use super::*;
use crate::efectos::{self, TipoFx};

const TIRA: f32 = 92.0;

/// Posición del fader (0 abajo, 1 arriba) y su inverso, en dB de -60 a +6.
fn de_db(d: f32) -> f32 {
    ((d + 60.0) / 66.0).clamp(0.0, 1.0).powf(2.2)
}
fn a_db_f(x: f32) -> f32 {
    x.clamp(0.0, 1.0).powf(1.0 / 2.2) * 66.0 - 60.0
}

fn fader(ui: &mut Ui, v: &mut f32, alto: f32, sel: bool, nombre: &str) {
    let (r, resp) = ui.allocate_exact_size(vec2(22.0, alto), Sense::click_and_drag());
    let via = Rect::from_center_size(r.center(), vec2(4.0, alto - 20.0));
    if resp.dragged() {
        let fino = if ui.input(|i| i.modifiers.shift) { 0.2 } else { 1.0 };
        *v = a_db_f(de_db(*v) - resp.drag_delta().y / via.height() * fino);
        if v.abs() < 0.15 {
            *v = 0.0;
        }
    }
    if resp.double_clicked() {
        *v = 0.0;
    }
    resp.widget_info(|| WidgetInfo::slider(true, *v as f64, nombre));
    let p = ui.painter();
    p.rect_filled(via, 2, c(0x161616));
    let y0 = via.bottom() - via.height() * de_db(0.0);
    p.hline(r.left()..=r.right(), y0, Stroke::new(1.0, c(0x4a4a4a)));
    let y = via.bottom() - via.height() * de_db(*v);
    if sel {
        p.rect_filled(Rect::from_min_max(pos2(via.left(), y), via.max), 2, MAIN);
    }
    let h = Rect::from_center_size(pos2(r.center().x, y), vec2(16.0, 24.0));
    p.rect_filled(h.expand(1.0), 5, c(0x161616));
    let b = if resp.hovered() || resp.dragged() { c(0x6a6a6a) } else { c(0x5c5c5c) };
    degradado(p, h, 4, mezcla(b, Color32::WHITE, 0.08), mezcla(b, Color32::BLACK, 0.25));
    p.rect_filled(Rect::from_center_size(h.center(), vec2(12.0, 2.0)), 1, if sel { MAIN } else { c(0xdedede) });
    resp.on_hover_text(crate::idioma::tf("{} dB (doble clic: 0 dB)", &[&format!("{:.1}", v)]));
}

impl App {
    pub fn ui_consola(&mut self, ui: &mut Ui) {
        let alto = ui.available_height();
        ScrollArea::horizontal().id_salt("consola").auto_shrink([false, false]).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                self.tira(ui, None, alto);
                ui.add_space(8.0);
                for i in 0..self.p.pistas.len() {
                    self.tira(ui, Some(i), alto);
                }
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.add_space(alto / 2.0 - 20.0);
                    // Pregunta qué tipo de pista agregar.
                    let b = Button::new(RichText::new("+").size(20.0)).min_size(vec2(32.0, 32.0));
                    egui::containers::menu::MenuButton::from_button(b).ui(ui, |ui| {
                        ui.set_min_width(220.0);
                        if ui.button(crate::idioma::t("Pista de audio")).clicked() {
                            let n = self.p.pistas.len() + 1;
                            self.canal = Canal::Pista(self.p.nueva_pista(&crate::idioma::tf("Audio {}", &[&n])));
                        }
                        if ui.button(crate::idioma::t("Pista de grabación (tu voz)")).clicked() {
                            self.canal = Canal::Pista(self.p.nueva_pista_grabacion());
                        }
                        ui.menu_button(crate::idioma::t("Instrumento"), |ui| self.menu_instrumentos(ui));
                    })
                    .0
                    .on_hover_text(crate::idioma::t("Agregar una pista a la consola"));
                });
            });
        });
    }

    fn tira(&mut self, ui: &mut Ui, i: Option<usize>, alto: f32) {
        let canal = i.map_or(Canal::Maestro, |i| Canal::Pista(self.p.pistas[i].id));
        let sel = self.canal == canal;
        let (r, resp) = ui.allocate_exact_size(vec2(TIRA, alto), Sense::click());
        if resp.clicked() {
            self.canal = canal;
        }
        ui.painter().rect_filled(r, 4, if sel { c(0x3a3a3a) } else { STRIP });
        let (nombre, col) = match i {
            Some(i) => (self.p.pistas[i].nombre.clone(), color_pista(i)),
            None => (crate::idioma::t("Maestro").into(), MAIN),
        };
        let cab = Rect::from_min_size(r.min + vec2(3.0, 3.0), vec2(TIRA - 6.0, 22.0));
        if sel {
            degradado(ui.painter(), cab, 6, mezcla(HIGHLIGHT, MAIN, 0.45), MAIN);
        } else {
            ui.painter().rect_filled(cab, 6, STRIP_DARK);
            ui.painter().rect_filled(Rect::from_min_size(cab.min + vec2(2.0, 4.0), vec2(3.0, 14.0)), 1, col);
        }
        let tinta = if sel { COLORED_DARK } else { TEXT };
        let mut g = ui.painter().layout_no_wrap(nombre.clone(), fuente(14.0), tinta);
        if g.size().x > cab.width() - 12.0 {
            g = ui.painter().layout_no_wrap(nombre.clone(), fuente(11.5), tinta);
        }
        ui.painter_at(cab).galley(pos2(cab.left() + 8.0, cab.center().y - g.size().y / 2.0), g, tinta);

        let dentro = Rect::from_min_max(pos2(r.left() + 4.0, cab.bottom() + 4.0), r.max - vec2(4.0, 4.0));
        let mut u = ui.new_child(UiBuilder::new().max_rect(dentro).layout(Layout::top_down(Align::Center)));
        u.spacing_mut().item_spacing = vec2(3.0, 3.0);
        match i {
            Some(i) => {
                let p = &mut self.p.pistas[i];
                perilla(&mut u, &mut p.pan, -1.0, 1.0, 0.0, false, 14.0, "Paneo").on_hover_text(crate::idioma::t("Izquierda / derecha"));
                let mut armar = false;
                u.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if led(ui, !p.mudo, "Encender o silenciar").clicked() {
                        p.mudo = !p.mudo;
                    }
                    if letra(ui, "S", p.solo, MAIN, "Solo").clicked() {
                        p.solo = !p.solo;
                    }
                    armar = letra(ui, "●", p.armada, RECORD, "Armar para grabar").clicked();
                });
                if armar {
                    let id = self.p.pistas[i].id;
                    self.p.armar(id);
                }
            }
            None => {
                u.add_space(4.0);
                u.label(RichText::new(crate::idioma::t("Salida")).size(13.0).color(TEXT_DIM));
                u.add_space(22.0);
            }
        }
        let pico = match canal {
            Canal::Pista(id) => self.picos.get(&id).copied().unwrap_or_default(),
            Canal::Maestro => self.pico_maestro,
        };
        let (v, fx) = match i {
            Some(i) => {
                let p = &mut self.p.pistas[i];
                (&mut p.vol_db, p.fx.iter().filter(|f| f.activo).count())
            }
            None => (&mut self.p.maestro_db, self.p.fx_maestro.iter().filter(|f| f.activo).count()),
        };
        let h = (u.available_height() - 46.0).max(40.0);
        u.horizontal(|ui| {
            ui.add_space(16.0);
            fader(ui, v, h, sel, &crate::idioma::tf("Fader {}", &[&nombre]));
            let (m, _) = ui.allocate_exact_size(vec2(16.0, h), Sense::hover());
            medidor(ui.painter(), m, pico);
        });
        let txt = if *v <= -59.5 { "-∞ dB".into() } else { crate::idioma::tf("{} dB", &[&format!("{:+.1}", v)]) };
        u.label(RichText::new(txt).size(12.0).color(if sel { MAIN } else { TEXT_DIM }));
        // Efectos desde la consola: encender, apagar o agregar.
        let t = match fx {
            0 => crate::idioma::t("+ Efecto").to_string(),
            1 => crate::idioma::t("1 efecto +").into(),
            n => crate::idioma::tf("{} efectos +", &[&n]),
        };
        let mut nuevo = None;
        let r = u.menu_button(RichText::new(t).size(12.5).color(MAIN), |ui| {
            ui.set_min_width(190.0);
            let lista = self.fx_de(canal);
            for f in lista.iter_mut() {
                ui.horizontal(|ui| {
                    if led(ui, f.activo, f.tipo.nombre()).clicked() {
                        f.activo = !f.activo;
                    }
                    ui.label(crate::idioma::t(&f.tipo.nombre()));
                });
            }
            if !lista.is_empty() {
                ui.separator();
            }
            ui.label(RichText::new(crate::idioma::t("Agregar")).size(12.0).color(TEXT_FAINT));
            for t in TipoFx::TODOS {
                if ui.button(crate::idioma::t(&t.nombre())).on_hover_text(t.ayuda()).clicked() {
                    nuevo = Some(t);
                }
            }
        });
        if r.response.clicked() {
            self.canal = canal;
        }
        if let Some(t) = nuevo {
            let f = self.p.nuevo_fx(t);
            self.fx_de(canal).push(f);
            self.canal = canal;
        }
    }

    pub fn panel_fx(&mut self, ui: &mut Ui) {
        let canal = self.canal;
        let titulo = match canal {
            Canal::Maestro => crate::idioma::t("Maestro").to_string(),
            Canal::Pista(id) => match self.p.pista(id) {
                Some(p) => p.nombre.clone(),
                None => {
                    self.canal = Canal::Maestro;
                    return;
                }
            },
        };
        let mut agregar = None;
        Frame::new().inner_margin(Margin::symmetric(8, 5)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(crate::idioma::tf("Efectos - {}", &[&titulo])).size(17.0));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.menu_button(RichText::new(crate::idioma::t("+ Agregar efecto")).color(MAIN), |ui| {
                        for t in TipoFx::TODOS {
                            if ui.button(crate::idioma::t(&t.nombre())).on_hover_text(t.ayuda()).clicked() {
                                agregar = Some(t);
                            }
                        }
                    });
                });
            });
        });
        if let Some(t) = agregar {
            let f = self.p.nuevo_fx(t);
            self.fx_de(canal).push(f);
        }
        let mut quitar = None;
        let mut mover = None;
        let lista = self.fx_de(canal);
        let n = lista.len();
        ScrollArea::vertical().id_salt("fx").auto_shrink([false, false]).show(ui, |ui| {
            if n == 0 {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| ui.label(RichText::new(crate::idioma::t("Sin efectos. Agrega uno con el botón de arriba.")).color(TEXT_FAINT)));
            }
            for (i, f) in lista.iter_mut().enumerate() {
                Frame::new()
                    .fill(SLOT)
                    .stroke(Stroke::new(1.0, c(0x222222)))
                    .corner_radius(6)
                    .inner_margin(6.0)
                    .outer_margin(Margin::symmetric(6, 2))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let id = Id::new(("fx-abierto", f.id));
                        let mut abierto = ui.data(|d| d.get_temp(id)).unwrap_or(true);
                        ui.horizontal(|ui| {
                            if led(ui, f.activo, &crate::idioma::tf("Encender {}", &[&(f.tipo.nombre())])).clicked() {
                                f.activo = !f.activo;
                            }
                            let t = RichText::new(crate::idioma::t(&f.tipo.nombre())).size(16.0).color(if f.activo { c(0xdedede) } else { c(0x707070) });
                            if ui.add(Label::new(t).sense(Sense::click())).on_hover_text(f.tipo.ayuda()).clicked() {
                                abierto = !abierto;
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                if boton_color(ui, &tabler::X, false, "Quitar efecto", TEXT_DIM, 22.0).clicked() {
                                    quitar = Some(i);
                                }
                                if boton_color(ui, &tabler::CARET_UP, false, "Subir en la cadena", TEXT_DIM, 22.0).clicked() && i > 0 {
                                    mover = Some(i);
                                }
                                if !f.tipo.ajustes().is_empty() {
                                    ui.menu_button(RichText::new(crate::idioma::t("Ajustes")).size(13.0), |ui| {
                                        for (n, v) in f.tipo.ajustes() {
                                            if ui.button(*n).clicked() {
                                                f.p = v.to_vec();
                                            }
                                        }
                                        if ui.button(crate::idioma::t("Restablecer")).clicked() {
                                            f.p = f.tipo.defaults();
                                        }
                                    });
                                }
                            });
                        });
                        ui.data_mut(|d| d.insert_temp(id, abierto));
                        if abierto {
                            controles(ui, f);
                        }
                    });
            }
        });
        if let Some(i) = quitar {
            lista.remove(i);
        }
        if let Some(i) = mover {
            lista.swap(i, i - 1);
        }
    }

    pub fn fx_de(&mut self, canal: Canal) -> &mut Vec<Fx> {
        if let Canal::Pista(id) = canal {
            if let Some(i) = self.p.pistas.iter().position(|p| p.id == id) {
                return &mut self.p.pistas[i].fx;
            }
        }
        &mut self.p.fx_maestro
    }
}

fn valor(v: f32, u: &str) -> String {
    match u {
        "esc" => crate::propios::ESCALAS[(v.round() as usize).min(4)].0.to_string(),
        "nota" => ["Do", "Do#", "Re", "Re#", "Mi", "Fa", "Fa#", "Sol", "Sol#", "La", "La#", "Si"][(v.round() as usize).min(11)].to_string(),
        "gr" => format!("{v:+.0}"),
        "bpm" => crate::idioma::tf("{} bpm", &[&format!("{:.0}", v)]),
        "st" | "cents" => format!("{v:+.0} {u}"),
        "Hz" if v >= 1000.0 => crate::idioma::tf("{} kHz", &[&format!("{:.1}", v / 1000.0)]),
        "Hz" => crate::idioma::tf("{} Hz", &[&format!("{:.0}", v)]),
        "dB" => crate::idioma::tf("{} dB", &[&format!("{:+.1}", v)]),
        "ms" if v < 10.0 => crate::idioma::tf("{} ms", &[&format!("{:.1}", v)]),
        "ms" | "%" => format!("{v:.0} {u}"),
        _ => format!("{v:.1} {u}"),
    }
}

fn controles(ui: &mut Ui, f: &mut Fx) {
    if f.tipo == TipoFx::Ecualizador {
        curva(ui, &f.p);
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 4.0);
        for (k, d) in f.tipo.params().iter().enumerate() {
            ui.allocate_ui_with_layout(vec2(84.0, 74.0), Layout::top_down(Align::Center), |ui| {
                ui.set_width(84.0);
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.label(RichText::new(crate::idioma::t(&d.nombre)).size(12.0).color(TEXT_DIM));
                let v = &mut f.p[k];
                if d.unidad.is_empty() {
                    ui.add_space(6.0);
                    if led(ui, *v >= 0.5, d.nombre).clicked() {
                        *v = if *v >= 0.5 { 0.0 } else { 1.0 };
                    }
                } else {
                    perilla(ui, v, d.min, d.max, d.def, d.log, 18.0, d.nombre)
                        .on_hover_text("Arrastra; Shift = fino; doble clic = valor inicial");
                    ui.label(RichText::new(valor(*v, d.unidad)).size(12.0));
                }
            });
        }
    });
}

/// Curva del ecualizador, de 20 Hz a 20 kHz y ±18 dB.
fn curva(ui: &mut Ui, p: &[f32]) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 84.0), Sense::hover());
    let pr = ui.painter_at(r);
    pr.rect_filled(r, 4, WELL);
    for hz in [100.0f32, 1000.0, 10000.0] {
        let x = r.left() + r.width() * (hz / 20.0).log10() / 3.0;
        pr.vline(x, r.y_range(), Stroke::new(1.0, c(0x2c2c2c)));
    }
    pr.hline(r.x_range(), r.center().y, Stroke::new(1.0, c(0x3a3a3a)));
    let n = (r.width() as usize / 2).max(2);
    let hz = (0..=n).map(|i| 20.0 * 1000f64.powf(i as f64 / n as f64));
    let y = efectos::curva_eq(p, hz);
    let pts: Vec<Pos2> = y
        .iter()
        .enumerate()
        .map(|(i, d)| pos2(r.left() + r.width() * i as f32 / n as f32, r.center().y - (*d as f32 / 18.0).clamp(-1.2, 1.2) * r.height() / 2.0))
        .collect();
    let mut m = Mesh::default();
    for w in pts.windows(2) {
        let b = Rect::from_two_pos(w[0], pos2(w[1].x, r.center().y));
        m.add_colored_rect(b, MAIN.gamma_multiply(0.18));
    }
    pr.add(Shape::mesh(m));
    pr.add(Shape::line(pts, Stroke::new(1.8, MAIN)));
}
