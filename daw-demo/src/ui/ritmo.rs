// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Canales y pasos.
use super::*;

fn golpe(i: u32) -> Nota {
    Nota { tick: i * PASO, dur: PASO, tono: 60, fuerza: 0.8 }
}

/// Bombo, caja, platillo y aplauso; `x` fuerte y `o` suave.
pub const RITMOS: &[(&str, [&str; 4])] = &[
    ("Rock", ["x.......x.x.....", "....x.......x...", "x.o.x.o.x.o.x.o.", ""]),
    ("Pop", ["x.....x.x.......", "....x.......x..o", "x.x.x.x.x.x.x.x.", "....x.......x..."]),
    ("House", ["x...x...x...x...", "", "..x...x...x...x.", "....x.......x..."]),
    ("Hip hop", ["x..o....o.x.....", "....x.......x...", "x.x.x.xox.x.x.x.", ""]),
    ("Trap", ["x......x..x.....", "", "x.x.x.xxx.x.xooo", "........x......."]),
    ("Reggaetón", ["x...x...x...x...", "...x..x....x..x.", "x.o.x.o.x.o.x.o.", ""]),
    ("Cumbia", ["x.......x.......", "....o.......o...", "x.oox.oox.oox.oo", "..x...x...x...x."]),
    ("Corrido tumbado", ["x.....x...x.....", "......o.......o.", "..o...o...o...o.", ""]),
    ("Funk", ["x.x.......x..x..", "....x..o.o..x..o", "xoxoxoxoxoxoxoxo", ""]),
    ("Bossa nova", ["x..ox...x..ox...", "x..x..x...x..x..", "xoxoxoxoxoxoxoxo", ""]),
    ("Drum and bass", ["x.........x.....", "....x.......x...", "x.x.x.x.x.x.x.x.", ""]),
];

#[derive(Default)]
pub struct Ritmo {
    /// Si el arrastre está poniendo o quitando pasos.
    pinta: Option<bool>,
}

impl App {
    /// Elegir, renombrar, duplicar y borrar patrones. Lo usan Ritmo y Notas.
    pub(super) fn barra_patron(&mut self, ui: &mut Ui) {
        let Some(i) = self.p.patrones.iter().position(|x| x.id == self.patron) else { return };
        ComboBox::from_id_salt("patron").selected_text(self.p.patrones[i].nombre.clone()).width(130.0).show_ui(ui, |ui| {
            for x in &self.p.patrones {
                ui.selectable_value(&mut self.patron, x.id, &x.nombre);
            }
        });
        if ui.button("+").on_hover_text(t("Patrón nuevo")).clicked() {
            self.elegir = Some(lado::Elegir::patron());
            return;
        }
        ui.menu_button("…", |ui| {
            ui.text_edit_singleline(&mut self.p.patrones[i].nombre);
            if ui.button(t("Duplicar")).clicked() {
                let copia = self.p.patrones[i].clone();
                let id = self.p.agregar_patron();
                let n = self.p.patrones.last_mut().unwrap();
                n.notas = copia.notas;
                n.compases = copia.compases;
                n.nombre = tf("{} (copia)", &[&copia.nombre]);
                self.patron = id;
            }
            if ui.button(t("Vaciar")).clicked() {
                self.p.patrones[i].notas.clear();
            }
            if ui.add_enabled(self.p.patrones.len() > 1, Button::new(t("Borrar"))).clicked() {
                self.borrar_patron(self.patron);
            }
        });
        if let Some(x) = self.p.patrones.iter_mut().find(|x| x.id == self.patron) {
            let s = t(if x.compases == 1 { " compás" } else { " compases" });
            ui.add(DragValue::new(&mut x.compases).range(1..=64).speed(0.05).suffix(s)).on_hover_text(t("Largo del patrón"));
        }
    }

    pub(super) fn ver_ritmo(&mut self, ui: &mut Ui) {
        fila(ui, |ui| {
            self.barra_patron(ui);
            ui.menu_button(t("Ritmos listos"), |ui| {
                for (nombre, partes) in RITMOS {
                    if ui.button(*nombre).clicked() {
                        self.ritmo_listo(partes);
                    }
                }
            })
            .response
            .on_hover_text(t("Escribe la batería en este patrón; agrega los instrumentos que falten"));
            if ui.button(t("Poner en el arreglo")).on_hover_text(t("Agrega este patrón al final de la canción")).clicked() {
                let tick = self.p.clips.iter().map(|c| c.tick + c.dur).max().unwrap_or(0);
                let dur = self.p.patron(self.patron).map_or(COMPAS, |x| x.largo());
                self.p.clips.push(Clip::de_patron(self.patron, 0, tick, dur));
                self.mostrar(Vista::Arreglo);
            }
        });
        ui.add_space(4.0);
        if self.p.canales.is_empty() {
            ui.label(RichText::new(t("Elige un instrumento de la izquierda para empezar")).color(TENUE));
        }
        let largo = self.p.patron(self.patron).map_or(COMPAS, |x| x.largo());
        let pasos = largo / PASO;
        let aqui = (self.tocando() && !self.cancion).then(|| self.tick() as u32 / PASO);
        let soltado = !ui.input(|i| i.pointer.any_down());
        if soltado {
            self.ritmo.pinta = None;
        }
        let mut borrar = None;
        let mut mover = None;
        ScrollArea::both().auto_shrink(false).show(ui, |ui| {
            for k in 0..self.p.canales.len() {
                ui.horizontal(|ui| {
                    let c = &mut self.p.canales[k];
                    let id = c.id;
                    let col = color(c.color);
                    let (r, led) = ui.allocate_exact_size(vec2(18.0, 36.0), Sense::click());
                    let q = Rect::from_center_size(r.center(), vec2(12.0, 12.0));
                    if c.mudo {
                        hueco(ui.painter(), q, 3, HUECO);
                    } else {
                        encendido(ui.painter(), q, 3, if c.solo { ACENTO } else { VERDE });
                    }
                    if led.on_hover_text(t("Clic: silenciar · clic derecho: solo este")).clicked() {
                        c.mudo = !c.mudo;
                    }
                    if ui.rect_contains_pointer(r) && ui.input(|i| i.pointer.secondary_clicked()) {
                        c.solo = !c.solo;
                    }
                    let elegido = self.canal == id;
                    let (r, b) = ui.allocate_exact_size(vec2(128.0, 34.0), Sense::click());
                    let p = ui.painter();
                    relieve(p, r, 6, CAJA, b.hovered() || elegido, b.is_pointer_button_down_on());
                    p.rect_filled(Rect::from_min_size(r.min + vec2(3.0, 4.0), vec2(4.0, r.height() - 8.0)), 2.0, col);
                    if elegido {
                        p.rect_stroke(r, 6.0, Stroke::new(1.5, ACENTO), StrokeKind::Outside);
                    }
                    let marca = if c.solo { "S · " } else { "" };
                    rotulo(p, r.shrink2(vec2(12.0, 0.0)), &format!("{marca}{}", c.nombre), 15.0, if elegido { ACENTO } else { TEXTO });
                    b.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &c.nombre));
                    if b.is_pointer_button_down_on() && b.drag_started() || b.clicked() {
                        self.canal = id;
                        self.clip = None;
                    }
                    if b.clicked() {
                        self.sonar(id, 60, 0.9);
                        self.teclas.insert(Key::F35, (id, 60));
                    }
                    if b.double_clicked() {
                        self.mostrar(Vista::Canal);
                    }
                    b.context_menu(|ui| {
                        ui.text_edit_singleline(&mut self.p.canales[k].nombre);
                        if ui.button(t("Abrir en Notas")).clicked() {
                            self.canal = id;
                            self.mostrar(Vista::Notas);
                        }
                        for (s, cada) in [("Cada 2 pasos", 2), ("Cada 4 pasos", 4), ("Todos los pasos", 1)] {
                            if ui.button(t(s)).clicked() {
                                let v = (0..pasos).step_by(cada).map(golpe).collect();
                                self.poner_notas(id, v);
                            }
                        }
                        ui.menu_button(t("Reparto parejo"), |ui| {
                            for g in [3u32, 5, 7, 9, 11, 13] {
                                if ui.button(tf("{} golpes en 16", &[&g])).clicked() {
                                    let v = (0..pasos).filter(|i| (i % 16) * g % 16 < g).map(golpe).collect();
                                    self.poner_notas(id, v);
                                }
                            }
                        });
                        if ui.button(t("Limpiar")).clicked() {
                            self.poner_notas(id, vec![]);
                        }
                        ui.separator();
                        if ui.button(t("Subir")).clicked() && k > 0 {
                            mover = Some((k, k - 1));
                        }
                        if ui.button(t("Bajar")).clicked() && k + 1 < self.p.canales.len() {
                            mover = Some((k, k + 1));
                        }
                        if ui.button(t("Borrar canal")).clicked() {
                            borrar = Some(id);
                        }
                    });
                    if b.hovered() {
                        self.ayuda("Clic: elegir y oír · doble clic: abrir el instrumento · clic derecho: más opciones");
                    }
                    let c = &mut self.p.canales[k];
                    perilla(ui, "", &mut c.volumen, 0.0, 1.2, 0.8, ACENTO, 30.0).on_hover_text(t("Volumen"));
                    perilla(ui, "", &mut c.pan, -1.0, 1.0, 0.0, ACENTO, 30.0).on_hover_text(t("Izquierda · derecha"));
                    let (r, _) = ui.allocate_exact_size(vec2(26.0, 20.0), Sense::hover());
                    hueco(ui.painter(), r, 3, HUECO);
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, c.pista, letra(13.0), ACENTO);
                    self.pasos(ui, k, pasos, aqui);
                });
            }
        });
        if soltado && let Some((c, n)) = self.teclas.remove(&Key::F35) {
            self.callar(c, n);
        }
        if let Some((a, b)) = mover {
            self.p.canales.swap(a, b);
        }
        if let Some(id) = borrar {
            self.borrar_canal(id);
        }
    }

    pub(super) fn ritmo_listo(&mut self, partes: &[&str; 4]) {
        let largo = self.p.patron(self.patron).map_or(COMPAS, |x| x.largo());
        for (ins, s) in ["bombo", "caja", "platillo", "aplauso"].into_iter().zip(partes) {
            let canal = self.p.canales.iter().find(|c| c.instrumento == ins && c.plugin.is_none()).map(|c| c.id);
            if s.is_empty() && canal.is_none() {
                continue;
            }
            let canal = canal.unwrap_or_else(|| {
                let n = self.bib.ver(ins).map_or(ins.into(), |g| g.ficha.nombre.ver().to_string());
                self.p.agregar_canal(ins, &n)
            });
            let b = s.as_bytes();
            let v = (0..largo / PASO).filter_map(|i| match b.get(i as usize % 16)? {
                b'x' => Some(Nota { fuerza: 0.95, ..golpe(i) }),
                b'o' => Some(Nota { fuerza: 0.5, ..golpe(i) }),
                _ => None,
            });
            self.poner_notas(canal, v.collect());
        }
    }

    fn poner_notas(&mut self, canal: u32, v: Vec<Nota>) {
        if let Some(x) = self.p.patrones.iter_mut().find(|x| x.id == self.patron) {
            if v.is_empty() {
                x.notas.remove(&canal);
            } else {
                x.notas.insert(canal, v);
            }
        }
    }

    fn pasos(&mut self, ui: &mut Ui, k: usize, pasos: u32, aqui: Option<u32>) {
        let (id, col) = (self.p.canales[k].id, color(self.p.canales[k].color));
        let Some(pat) = self.p.patrones.iter_mut().find(|x| x.id == self.patron) else { return };
        let notas = pat.notas.get(&id);
        let ancho = 22.0;
        let melodia = notas.is_some_and(|v| v.iter().any(|n| n.tono != 60 || n.dur > PASO || n.tick % PASO != 0));
        let (r, resp) = ui.allocate_exact_size(vec2(pasos as f32 * ancho, 34.0), Sense::click_and_drag());
        let p = ui.painter_at(r);
        if melodia {
            let v = notas.unwrap();
            hueco(&p, r.shrink(1.0), 5, HUECO);
            let (lo, hi) = v.iter().fold((127, 0), |m, n| (m.0.min(n.tono), m.1.max(n.tono)));
            let alto = (hi - lo).max(6) as f32 + 2.0;
            for n in v {
                let x = r.left() + n.tick as f32 / PASO as f32 * ancho;
                let y = r.bottom() - (n.tono - lo) as f32 / alto * (r.height() - 8.0) - 6.0;
                p.rect_filled(Rect::from_min_size(pos2(x, y), vec2((n.dur as f32 / PASO as f32 * ancho - 1.0).max(2.0), 4.0)), 1.0, col);
            }
            if let Some(a) = aqui {
                let x = r.left() + a as f32 * ancho;
                p.rect_filled(Rect::from_min_size(pos2(x, r.top()), vec2(ancho, r.height())), 0.0, Color32::from_white_alpha(14));
            }
            if resp.hovered() {
                self.ayuda("Esto es una melodía: haz clic para abrirla en Notas");
            }
            if resp.clicked() {
                self.canal = id;
                self.mostrar(Vista::Notas);
            }
            return;
        }
        for i in 0..pasos {
            let n = notas.and_then(|v| v.iter().find(|n| n.tick / PASO == i));
            let b = Rect::from_min_size(pos2(r.left() + i as f32 * ancho + 1.0, r.top() + 3.0), vec2(ancho - 3.0, 28.0));
            let base = if i / 4 % 2 == 0 { Color32::from_rgb(0x4c, 0x4d, 0x53) } else { Color32::from_rgb(0x34, 0x35, 0x3a) };
            match n {
                Some(n) => encendido(&p, b, 4, mezcla(col.gamma_multiply(0.5).to_opaque(), col, n.fuerza)),
                None => relieve(&p, b, 4, base, false, false),
            }
            if aqui == Some(i) {
                p.rect_stroke(b, 4.0, Stroke::new(1.5, TEXTO), StrokeKind::Inside);
            }
        }
        if resp.hovered() {
            self.pista_ayuda = t("Clic o arrastra: poner o quitar golpes · clic derecho: quitar · rueda con ⌘/Ctrl: fuerza").into();
        }
        let Some(pos) = ui.input(|i| i.pointer.interact_pos()).filter(|x| r.contains(*x)) else { return };
        let i = ((pos.x - r.left()) / ancho) as u32;
        let hay = notas.is_some_and(|v| v.iter().any(|n| n.tick / PASO == i));
        let Some(pat) = self.p.patrones.iter_mut().find(|x| x.id == self.patron) else { return };
        let v = pat.notas.entry(id).or_default();
        let (pri, sec) = ui.input(|x| (x.pointer.primary_down(), x.pointer.secondary_down()));
        let rueda = ui.input(|x| if x.modifiers.command { x.smooth_scroll_delta.y } else { 0.0 });
        if rueda != 0.0 {
            v.iter_mut().filter(|n| n.tick / PASO == i).for_each(|n| n.fuerza = (n.fuerza + rueda / 400.0).clamp(0.05, 1.0));
        }
        if (pri || sec) && resp.is_pointer_button_down_on() || self.ritmo.pinta.is_some() {
            let poner = *self.ritmo.pinta.get_or_insert(pri && !hay);
            if poner && !hay {
                v.push(Nota { tick: i * PASO, dur: PASO, tono: 60, fuerza: 0.8 });
            } else if !poner {
                v.retain(|n| n.tick / PASO != i);
            }
        }
        if v.is_empty() {
            pat.notas.remove(&id);
        }
    }
}
