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
    // Marcas de dB a los lados de la ranura, como en una consola.
    for db in [6.0, 0.0, -6.0, -12.0, -24.0, -48.0] {
        let yy = via.bottom() - via.height() * de_db(db);
        let largo = if db == 0.0 { 6.0 } else { 3.5 };
        let col = if db == 0.0 { c(0x6a6a6a) } else { c(0x444444) };
        p.hline(r.left()..=r.left() + largo, yy, Stroke::new(1.0, col));
        p.hline(r.right() - largo..=r.right(), yy, Stroke::new(1.0, col));
    }
    hueco(p, via, 2, c(0x0f0f0f));
    let y = via.bottom() - via.height() * de_db(*v);
    if sel {
        p.rect_filled(Rect::from_min_max(pos2(via.left(), y), via.max), 2, MAIN);
    }
    // La perilla del fader: una pieza con relieve y estrías para agarrarla.
    let h = Rect::from_center_size(pos2(r.center().x, y), vec2(18.0, 26.0));
    relieve(p, h, 4, c(0x5e5e5e), resp.hovered() || resp.dragged(), false);
    for k in [-7.0, -4.5, 4.5, 7.0] {
        p.hline(h.left() + 3.0..=h.right() - 3.0, h.center().y + k, Stroke::new(1.0, Color32::from_black_alpha(90)));
    }
    p.rect_filled(Rect::from_center_size(h.center(), vec2(14.0, 2.0)), 1, if sel { MAIN } else { c(0xe6e6e6) });
    resp.on_hover_text(crate::idioma::tf("{} dB (doble clic: 0 dB)", &[&format!("{:.1}", v)]));
}

impl App {
    pub fn ui_consola(&mut self, ui: &mut Ui) {
        let alto = ui.available_height();
        ScrollArea::horizontal().id_salt("consola").auto_shrink([false, false]).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                self.tira(ui, Canal::Maestro, alto);
                ui.add_space(8.0);
                for i in 0..self.p.pistas.len() {
                    self.tira(ui, Canal::Pista(self.p.pistas[i].id), alto);
                }
                if !self.p.buses.is_empty() {
                    ui.add_space(8.0);
                }
                for id in self.p.buses.iter().map(|b| b.id).collect::<Vec<_>>() {
                    self.tira(ui, Canal::Bus(id), alto);
                }
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    // A media altura de las tiras. Pregunta qué tipo de pista agregar.
                    ui.add_space(((alto - 36.0) / 2.0).max(0.0));
                    let b = Button::new("").min_size(vec2(36.0, 36.0));
                    let (resp, _) = egui::containers::menu::MenuButton::from_button(b).ui(ui, |ui| {
                        ui.set_min_width(220.0);
                        if ui.button(crate::idioma::t("Pista de audio")).clicked() {
                            let n = self.p.pistas.len() + 1;
                            self.canal = Canal::Pista(self.p.nueva_pista(&crate::idioma::tf("Audio {}", &[&n])));
                        }
                        if ui.button(crate::idioma::t("Pista de grabación (tu voz)")).clicked() {
                            self.canal = Canal::Pista(self.p.nueva_pista_grabacion());
                        }
                        ui.menu_button(crate::idioma::t("Instrumento"), |ui| self.menu_instrumentos(ui));
                        if ui.button(crate::idioma::t("Mezclador")).on_hover_text(crate::idioma::t("Junta los clips y pistas que le mandes, con sus propios efectos")).clicked() {
                            self.canal = Canal::Bus(self.bus_nuevo());
                        }
                    });
                    // El + con dos rayas: con la letra quedaba bajo.
                    let o = resp.rect.center();
                    let col = if resp.hovered() { HIGHLIGHT } else { GLYPH };
                    for d in [vec2(7.0, 0.0), vec2(0.0, 7.0)] {
                        ui.painter().line_segment([o - d, o + d], Stroke::new(2.2, col));
                    }
                    resp.on_hover_text(crate::idioma::t("Agregar una pista o un mezclador a la consola"));
                });
            });
        });
    }

    fn tira(&mut self, ui: &mut Ui, canal: Canal, alto: f32) {
        let sel = self.canal == canal;
        let (r, resp) = ui.allocate_exact_size(vec2(TIRA, alto), Sense::click());
        if resp.clicked() {
            self.canal = canal;
        }
        // Placa de metal: más clara arriba, filo de luz y borde.
        relieve(ui.painter(), r, 4, if sel { c(0x3a3a3a) } else { STRIP }, false, false);
        let (nombre, col) = match canal {
            Canal::Pista(id) => {
                let i = self.p.pistas.iter().position(|p| p.id == id).unwrap();
                (self.p.pistas[i].nombre.clone(), color_pista(i))
            }
            Canal::Bus(id) => (self.p.bus(id).unwrap().nombre.clone(), HIGHLIGHT),
            Canal::Maestro => (crate::idioma::t("Maestro").into(), MAIN),
        };
        let cab = Rect::from_min_size(r.min + vec2(3.0, 3.0), vec2(TIRA - 6.0, 22.0));
        if sel {
            degradado(ui.painter(), cab, 6, mezcla(HIGHLIGHT, MAIN, 0.45), MAIN);
        } else {
            ui.painter().rect_filled(cab, 6, STRIP_DARK);
            ui.painter().rect_filled(Rect::from_min_size(cab.min + vec2(2.0, 4.0), vec2(3.0, 14.0)), 1, col);
        }
        let tinta = if sel { COLORED_DARK } else { TEXT };
        if let Canal::Bus(id) = canal {
            // El nombre se escribe ahí mismo.
            let b = self.p.bus(id).unwrap();
            let t = TextEdit::singleline(&mut b.nombre).frame(Frame::NONE).font(fuente(13.0)).text_color(tinta).margin(Margin::ZERO);
            let t = ui.place(Rect::from_min_max(cab.min + vec2(8.0, 3.0), cab.max - vec2(3.0, 1.0)), t).on_hover_text(crate::idioma::t("Escribe el nombre del mezclador"));
            t.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, crate::idioma::tf("Nombre de {}", &[&nombre])));
            if t.gained_focus() {
                self.canal = canal;
            }
            resp.context_menu(|ui| {
                if ui.button(crate::idioma::t("Borrar mezclador")).on_hover_text(crate::idioma::t("Lo que iba a él vuelve a su pista")).clicked() {
                    self.borrar_bus(id);
                }
            });
            if self.p.bus(id).is_none() {
                return;
            }
        } else {
            let mut g = ui.painter().layout_no_wrap(nombre.clone(), fuente(14.0), tinta);
            if g.size().x > cab.width() - 12.0 {
                g = ui.painter().layout_no_wrap(nombre.clone(), fuente(11.5), tinta);
            }
            ui.painter_at(cab).galley(pos2(cab.left() + 8.0, cab.center().y - g.size().y / 2.0), g, tinta);
        }

        let dentro = Rect::from_min_max(pos2(r.left() + 4.0, cab.bottom() + 4.0), r.max - vec2(4.0, 4.0));
        let mut u = ui.new_child(UiBuilder::new().max_rect(dentro).layout(Layout::top_down(Align::Center)));
        u.spacing_mut().item_spacing = vec2(3.0, 3.0);
        match canal {
            Canal::Pista(id) => {
                let p = self.p.pista(id).unwrap();
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
                let salida = p.bus;
                if armar {
                    self.p.armar(id);
                }
                let nombre = salida.and_then(|b| self.p.bus(b)).map_or(crate::idioma::t("Maestro").to_string(), |b| b.nombre.clone());
                u.menu_button(RichText::new(format!("→ {nombre}")).size(11.5).color(TEXT_DIM), |ui| self.menu_salida(ui, id));
            }
            Canal::Bus(id) => {
                let b = self.p.bus(id).unwrap();
                perilla(&mut u, &mut b.pan, -1.0, 1.0, 0.0, false, 14.0, "Paneo").on_hover_text(crate::idioma::t("Izquierda / derecha"));
                u.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if led(ui, !b.mudo, "Encender o silenciar").clicked() {
                        b.mudo = !b.mudo;
                    }
                    if letra(ui, "S", b.solo, MAIN, "Solo").clicked() {
                        b.solo = !b.solo;
                    }
                });
                u.label(RichText::new(crate::idioma::t("Mezclador")).size(11.5).color(TEXT_FAINT));
            }
            Canal::Maestro => {
                u.add_space(4.0);
                u.label(RichText::new(crate::idioma::t("Salida")).size(13.0).color(TEXT_DIM));
                u.add_space(22.0);
            }
        }
        let pico = match canal {
            Canal::Pista(id) | Canal::Bus(id) => self.picos.get(&id).copied().unwrap_or_default(),
            Canal::Maestro => self.pico_maestro,
        };
        let (v, fx) = match canal {
            Canal::Pista(id) => {
                let p = self.p.pista(id).unwrap();
                (&mut p.vol_db, p.fx.iter().filter(|f| f.activo).count())
            }
            Canal::Bus(id) => {
                let b = self.p.bus(id).unwrap();
                (&mut b.vol_db, b.fx.iter().filter(|f| f.activo).count())
            }
            Canal::Maestro => (&mut self.p.maestro_db, self.p.fx_maestro.iter().filter(|f| f.activo).count()),
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
            if let Some(t) = menu_efectos(ui) {
                nuevo = Some(t);
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
            Canal::Bus(id) => match self.p.bus(id) {
                Some(b) => b.nombre.clone(),
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
                        if let Some(t) = menu_efectos(ui) {
                            agregar = Some(t);
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
                                if boton_color(ui, &iconos::X, false, "Quitar efecto", TEXT_DIM, 22.0).clicked() {
                                    quitar = Some(i);
                                }
                                if boton_color(ui, &iconos::CARET_UP, false, "Subir en la cadena", TEXT_DIM, 22.0).clicked() && i > 0 {
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
        if let Canal::Bus(id) = canal {
            if let Some(i) = self.p.buses.iter().position(|b| b.id == id) {
                return &mut self.p.buses[i].fx;
            }
        }
        if let Canal::Pista(id) = canal {
            if let Some(i) = self.p.pistas.iter().position(|p| p.id == id) {
                return &mut self.p.pistas[i].fx;
            }
        }
        &mut self.p.fx_maestro
    }

    pub fn bus_nuevo(&mut self) -> u64 {
        // El primer número libre, para no repetir nombres después de borrar.
        let n = (1..).map(|n| crate::idioma::tf("Mezclador {}", &[&n])).find(|n| self.p.buses.iter().all(|b| b.nombre != *n)).unwrap();
        self.p.nuevo_bus(&n)
    }

    pub fn borrar_bus(&mut self, id: u64) {
        self.p.quitar_bus(id);
        if self.canal == Canal::Bus(id) {
            self.canal = Canal::Maestro;
        }
    }

    /// Los mezcladores y uno nuevo. Devuelve el que se elige (`None`: ninguno, por `normal`).
    fn lista_buses(&mut self, ui: &mut Ui, actual: Option<u64>, normal: &str) -> Option<Option<u64>> {
        let mut r = None;
        ui.set_min_width(190.0);
        if ui.selectable_label(actual.is_none(), crate::idioma::t(normal)).clicked() {
            r = Some(None);
        }
        for b in &self.p.buses {
            if ui.selectable_label(actual == Some(b.id), &b.nombre).clicked() {
                r = Some(Some(b.id));
            }
        }
        ui.separator();
        if ui.button(RichText::new(crate::idioma::t("+ Nuevo mezclador")).color(MAIN)).clicked() {
            r = Some(Some(self.bus_nuevo()));
        }
        if r.is_some() {
            ui.close();
        }
        r
    }

    /// A qué mezclador sale toda una pista.
    pub fn menu_salida(&mut self, ui: &mut Ui, pista: u64) {
        let actual = self.p.pista(pista).and_then(|p| p.bus);
        if let Some(b) = self.lista_buses(ui, actual, "Maestro") {
            self.p.pista(pista).unwrap().bus = b;
        }
    }

    /// A qué mezclador va un clip (y los demás elegidos con él).
    pub fn menu_bus_clip(&mut self, ui: &mut Ui, id: u64) {
        let actual = self.p.clip(id).and_then(|c| c.bus);
        let Some(b) = self.lista_buses(ui, actual, "Por su pista") else { return };
        for id in self.junto_con(id) {
            if let Some(c) = self.p.clip(id) {
                c.bus = b;
            }
        }
        self.canal_de_clip(id);
    }

    /// Del clip elegido: a qué mezclador va, su nombre y su color.
    pub fn ruta_clip(&mut self, ui: &mut Ui, id: u64) {
        let Some(bus) = self.p.clip(id).map(|c| c.bus) else { return };
        ui.horizontal(|ui| {
            ui.label(RichText::new(crate::idioma::t("Mezclador")).color(TEXT_DIM));
            let nombre = bus.and_then(|b| self.p.bus(b)).map_or(crate::idioma::t("Por su pista").to_string(), |b| b.nombre.clone());
            ui.menu_button(format!("→ {nombre}"), |ui| self.menu_bus_clip(ui, id))
                .response
                .on_hover_text(crate::idioma::t("A qué mezclador va este audio: ahí tiene sus propios efectos"));
            if let Some(b) = bus.and_then(|b| self.p.bus(b)) {
                ui.add(TextEdit::singleline(&mut b.nombre).desired_width(130.0)).on_hover_text(crate::idioma::t("Escribe el nombre del mezclador"));
            }
            ui.menu_button(crate::idioma::t("Color"), |ui| self.menu_color(ui, id));
        });
    }

    /// Los efectos que se ven al elegir un clip: los del mezclador al que va, o los de su pista.
    pub fn canal_de_clip(&mut self, id: u64) {
        if let Some(p) = self.p.pistas.iter().find(|p| p.clips.iter().any(|c| c.id == id)) {
            self.canal = p.clips.iter().find(|c| c.id == id).and_then(|c| c.bus).map_or(Canal::Pista(p.id), Canal::Bus);
        }
    }

    /// El clip y, si está entre los elegidos, todos ellos.
    fn junto_con(&self, id: u64) -> Vec<u64> {
        if self.seleccion.contains(&id) { self.seleccion.iter().copied().collect() } else { vec![id] }
    }

    /// Clase de un clip de video (y de los demás elegidos con él).
    pub fn menu_clase(&mut self, ui: &mut Ui, id: u64) {
        ui.set_min_width(230.0);
        let actual = self.p.video.iter().find(|c| c.id == id).and_then(|c| c.clase);
        let mut poner = None;
        if let Some(k) = actual.and_then(|k| self.p.clase(k)) {
            ui.horizontal(|ui| {
                ui.color_edit_button_srgb(&mut k.color);
                ui.add(TextEdit::singleline(&mut k.nombre).desired_width(170.0)).on_hover_text(crate::idioma::t("Nombre de la clase"));
            });
            ui.separator();
        }
        for k in &self.p.clases {
            let n = self.p.video.iter().filter(|c| c.clase == Some(k.id)).count();
            let [r, g, b] = k.color;
            let txt = RichText::new(format!("●  {}  ({n})", k.nombre)).color(if actual == Some(k.id) { COLORED_DARK } else { mezcla(Color32::from_rgb(r, g, b), Color32::WHITE, 0.25) });
            if ui.selectable_label(actual == Some(k.id), txt).clicked() {
                poner = Some(Some(k.id));
            }
        }
        if ui.button(RichText::new(crate::idioma::t("+ Nueva clase")).color(MAIN)).on_hover_text(crate::idioma::t("Los efectos que le pongas a un clip de la clase les pasan a todos")).clicked() {
            poner = Some(Some(self.clase_nueva()));
        }
        if let Some(k) = actual {
            ui.separator();
            if ui.button(crate::idioma::t("Elegir todos los de la clase")).clicked() {
                self.seleccion = self.p.video.iter().filter(|c| c.clase == Some(k)).map(|c| c.id).collect();
                self.clip = Some(id);
            }
            if ui.button(crate::idioma::t("Pasar los efectos de este clip a todos")).on_hover_text(crate::idioma::t("También los que ya traía antes de entrar a la clase")).clicked() {
                self.p.compartir_todo(id);
            }
            if ui.button(crate::idioma::t("Sacar de la clase")).clicked() {
                poner = Some(None);
            }
            if ui.button(crate::idioma::t("Borrar la clase")).on_hover_text(crate::idioma::t("Los clips se quedan con sus efectos")).clicked() {
                self.p.quitar_clase(k);
            }
        }
        if let Some(k) = poner {
            for id in self.junto_con(id) {
                if let Some(c) = self.p.clip_video(id) {
                    c.clase = k;
                }
            }
            ui.close();
        }
    }

    pub fn clase_nueva(&mut self) -> u64 {
        let n = (1..).map(|n| crate::idioma::tf("Clase {}", &[&n])).find(|n| self.p.clases.iter().all(|k| k.nombre != *n)).unwrap();
        // Un color que no se repita mientras haya.
        let x = TINTES.iter().map(|t| t.1).find(|x| self.p.clases.iter().all(|k| k.color != rgb(*x))).unwrap_or(TINTES[self.p.clases.len() % TINTES.len()].1);
        self.p.nueva_clase(&n, rgb(x))
    }

    /// Teñir un clip (y los demás elegidos con él).
    pub fn menu_color(&mut self, ui: &mut Ui, id: u64) {
        let actual = self.p.clip(id).map(|c| c.color).or_else(|| self.p.video.iter().find(|c| c.id == id).map(|c| c.color)).flatten();
        let mut nuevo = None;
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        for fila in TINTES.chunks(6) {
            ui.horizontal(|ui| {
                for (n, x) in fila {
                    let (r, resp) = ui.allocate_exact_size(vec2(26.0, 22.0), Sense::click());
                    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, crate::idioma::t(n)));
                    ui.painter().rect_filled(r, 4, c(*x));
                    if resp.hovered() || actual == Some(c(*x).to_srgba_unmultiplied()[..3].try_into().unwrap()) {
                        ui.painter().rect_stroke(r, 4, Stroke::new(2.0, Color32::WHITE), StrokeKind::Inside);
                    }
                    if resp.on_hover_text(crate::idioma::t(n)).clicked() {
                        nuevo = Some(Some([(x >> 16) as u8, (x >> 8) as u8, *x as u8]));
                        ui.close();
                    }
                }
            });
        }
        ui.add_space(2.0);
        // El selector se queda abierto mientras se busca el tono.
        let firme = egui::containers::menu::MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside);
        egui::containers::menu::SubMenuButton::new(crate::idioma::t("Otro color")).config(firme).ui(ui, |ui| {
            let [r, g, b] = actual.unwrap_or([0x8a, 0x6f, 0xc4]);
            let mut x = Color32::from_rgb(r, g, b);
            if egui::color_picker::color_picker_color32(ui, &mut x, egui::color_picker::Alpha::Opaque) {
                nuevo = Some(Some([x.r(), x.g(), x.b()]));
            }
        });
        if actual.is_some() && ui.button(crate::idioma::t("Quitar color")).clicked() {
            nuevo = Some(None);
        }
        let Some(n) = nuevo else { return };
        for id in self.junto_con(id) {
            if let Some(k) = self.p.clip(id) {
                k.color = n;
            }
            if let Some(k) = self.p.video.iter_mut().find(|k| k.id == id) {
                k.color = n;
            }
        }
    }
}

fn rgb(x: u32) -> [u8; 3] {
    [(x >> 16) as u8, (x >> 8) as u8, x as u8]
}

/// Colores para teñir clips.
const TINTES: [(&str, u32); 12] = [
    ("Rojo", 0xc8453f),
    ("Naranja", 0xd9772f),
    ("Ámbar", 0xcfa22c),
    ("Lima", 0x93ad35),
    ("Verde", 0x3f9a55),
    ("Turquesa", 0x2f9c92),
    ("Cielo", 0x3f90c4),
    ("Azul", 0x4767c9),
    ("Violeta", 0x7a52c7),
    ("Magenta", 0xb04aa8),
    ("Rosa", 0xd0668c),
    ("Gris", 0x7d8088),
];

/// Los efectos por grupo. Devuelve el elegido.
fn menu_efectos(ui: &mut Ui) -> Option<TipoFx> {
    let mut r = None;
    ui.set_min_width(190.0);
    for g in TipoFx::GRUPOS {
        ui.menu_button(crate::idioma::t(g), |ui| {
            ui.set_min_width(210.0);
            for t in TipoFx::TODOS.into_iter().filter(|t| t.grupo() == g) {
                if ui.button(t.nombre()).on_hover_text(t.ayuda()).clicked() {
                    r = Some(t);
                }
            }
        });
    }
    r
}

fn valor(v: f32, u: &str) -> String {
    match u {
        "esc" => crate::propios::ESCALAS[(v.round() as usize).min(4)].0.to_string(),
        "acorde" => crate::idioma::t(efectos::ACORDES[(v.round() as usize).min(4)]).to_string(),
        "banda" => crate::idioma::t(efectos::TIPOS_BANDA[(v.round() as usize).min(7)]).to_string(),
        "n" => format!("{v:.0}"),
        "midi" => {
            let n = v.round() as i32;
            format!("{}{}", valor((n % 12) as f32, "nota"), n / 12 - 1)
        }
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
    if f.tipo == TipoFx::Parametrico {
        return parametrico(ui, f);
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 4.0);
        for (k, d) in f.tipo.params().iter().enumerate() {
            ui.allocate_ui_with_layout(vec2(84.0, 74.0), Layout::top_down(Align::Center), |ui| {
                ui.set_width(84.0);
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.label(RichText::new(crate::idioma::t(&d.nombre)).size(12.0).color(TEXT_DIM));
                let v = &mut f.p[k];
                if d.unidad.is_empty() && d.max <= 1.0 {
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

/// El ecualizador de siete bandas: los puntos se arrastran sobre la curva.
fn parametrico(ui: &mut Ui, f: &mut Fx) {
    const TOPE: f32 = 18.0;
    let col = |k: usize| color_banda(k);
    let id = Id::new(("banda-eq", f.id));
    let mut k: usize = ui.data(|d| d.get_temp(id)).unwrap_or(3);
    let (r, fondo) = ui.allocate_exact_size(vec2(ui.available_width(), 170.0), Sense::click_and_drag());
    fondo.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, crate::idioma::t("Curva del ecualizador")));
    let pr = ui.painter_at(r);
    hueco(&pr, r, 6, WELL);
    let x_de = |hz: f32| r.left() + r.width() * (hz / 20.0).log10() / 3.0;
    let y_de = |db: f32| r.center().y - db / TOPE * (r.height() / 2.0 - 8.0);
    for (hz, n) in [(50.0f32, "50"), (100.0, "100"), (200.0, "200"), (500.0, "500"), (1000.0, "1k"), (2000.0, "2k"), (5000.0, "5k"), (10000.0, "10k")] {
        pr.vline(x_de(hz), r.y_range(), Stroke::new(1.0, c(0x2c2c2c)));
        pr.text(pos2(x_de(hz) + 2.0, r.bottom() - 2.0), Align2::LEFT_BOTTOM, n, fuente(10.5), TEXT_FAINT);
    }
    for db in [-12.0f32, -6.0, 6.0, 12.0] {
        pr.hline(r.x_range(), y_de(db), Stroke::new(1.0, c(0x2a2a2a)));
        pr.text(pos2(r.right() - 3.0, y_de(db)), Align2::RIGHT_CENTER, format!("{db:+.0}"), fuente(10.5), TEXT_FAINT);
    }
    pr.hline(r.x_range(), y_de(0.0), Stroke::new(1.0, c(0x4a4a4a)));
    let n = (r.width() as usize / 2).max(2);
    let trazo = |p: &[f32]| -> Vec<Pos2> {
        let y = efectos::curva_par(p, (0..=n).map(|i| 20.0 * 1000f64.powf(i as f64 / n as f64)));
        y.iter().enumerate().map(|(i, d)| pos2(r.left() + r.width() * i as f32 / n as f32, y_de((*d as f32).clamp(-TOPE * 1.3, TOPE * 1.3)))).collect()
    };
    // La banda elegida, sola, en su color.
    let mut sola = f.p.clone();
    for j in (0..7).filter(|j| *j != k) {
        sola[j * 4] = 0.0;
    }
    sola[28] = 0.0;
    let mut m = Mesh::default();
    for w in trazo(&sola).windows(2) {
        m.add_colored_rect(Rect::from_two_pos(w[0], pos2(w[1].x, y_de(0.0))), col(k).gamma_multiply(0.22));
    }
    pr.add(Shape::mesh(m));
    pr.add(Shape::line(trazo(&f.p), Stroke::new(2.0, HIGHLIGHT)));
    for j in 0..7 {
        let tipo = f.p[j * 4].round() as i32;
        let plano = !matches!(tipo, 5..=7);
        let o = pos2(x_de(f.p[j * 4 + 1]), y_de(if plano { 0.0 } else { f.p[j * 4 + 2] }));
        let resp = ui.interact(Rect::from_center_size(o, Vec2::splat(20.0)), Id::new(("punto-eq", f.id, j)), Sense::click_and_drag());
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, crate::idioma::tf("Banda {}", &[&(j + 1)])));
        if resp.drag_started() || resp.clicked() {
            k = j;
        }
        if let (true, Some(q)) = (resp.dragged(), resp.interact_pointer_pos()) {
            f.p[j * 4 + 1] = (20.0 * 1000f32.powf(((q.x - r.left()) / r.width()).clamp(0.0, 1.0))).clamp(20.0, 20000.0);
            if !plano {
                f.p[j * 4 + 2] = ((r.center().y - q.y) / (r.height() / 2.0 - 8.0) * TOPE).clamp(-TOPE, TOPE);
            }
        }
        if resp.double_clicked() {
            f.p[j * 4 + 2] = 0.0;
        }
        if resp.hovered() {
            let giro = ui.input(|i| i.smooth_scroll_delta().y);
            if giro != 0.0 {
                f.p[j * 4 + 3] = (f.p[j * 4 + 3] * (1.0 - giro * 0.004)).clamp(1.0, 100.0);
                ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
            }
        }
        let vivo = tipo != 0;
        pr.circle(o, if j == k { 8.5 } else { 7.0 }, if vivo { col(j) } else { c(0x4a4a4a) }, Stroke::new(if j == k { 2.0 } else { 1.0 }, if j == k { Color32::WHITE } else { Color32::BLACK }));
        pr.text(o, Align2::CENTER_CENTER, (j + 1).to_string(), fuente(11.0), COLORED_DARK);
        resp.on_hover_text(crate::idioma::t("Arrastra: frecuencia y nivel. Rueda: ancho. Doble clic: nivel a cero."));
    }
    ui.data_mut(|d| d.insert_temp(id, k));
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(crate::idioma::tf("Banda {}", &[&(k + 1)])).color(col(k)).size(14.0));
        ComboBox::from_id_salt(("tipo-banda", f.id)).width(120.0).selected_text(valor(f.p[k * 4], "banda")).show_ui(ui, |ui| {
            for (i, n) in efectos::TIPOS_BANDA.into_iter().enumerate() {
                if ui.selectable_label(f.p[k * 4].round() as usize == i, crate::idioma::t(n)).clicked() {
                    f.p[k * 4] = i as f32;
                }
            }
        });
        ui.spacing_mut().item_spacing = vec2(2.0, 4.0);
        let todos = f.tipo.params();
        for i in [k * 4 + 1, k * 4 + 2, k * 4 + 3, 28] {
            let d = &todos[i];
            ui.allocate_ui_with_layout(vec2(78.0, 74.0), Layout::top_down(Align::Center), |ui| {
                ui.set_width(78.0);
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.label(RichText::new(crate::idioma::t(d.nombre)).size(12.0).color(TEXT_DIM));
                perilla(ui, &mut f.p[i], d.min, d.max, d.def, d.log, 18.0, d.nombre);
                ui.label(RichText::new(valor(f.p[i], d.unidad)).size(12.0));
            });
        }
    });
}

fn color_banda(k: usize) -> Color32 {
    [c(0xff7a7a), c(0xffa95c), c(0xffe066), c(0x8ddf6f), c(0x5cd6d0), c(0x6fa8ff), c(0xc48bff)][k % 7]
}

/// Curva del ecualizador, de 20 Hz a 20 kHz y ±18 dB.
fn curva(ui: &mut Ui, p: &[f32]) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 84.0), Sense::hover());
    let pr = ui.painter_at(r);
    hueco(&pr, r, 4, WELL);
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
