//! Ventana de audio: instrumento, rack de pasos y piano roll. También los patrones en la Playlist.
use super::biblioteca::pastilla;
use super::estilo::*;
use super::*;
use crate::instrumentos::{FAMILIAS, ONDAS, PIEZAS, SONIDOS, TRIO, nombre_sonido};

pub struct Estado {
    pub patron: Option<u64>,
    /// Pixeles por pulso.
    zoom: f32,
    x0: f64,
    /// Ajuste en pulsos.
    paso: f64,
    largo: f64,
    arrastre: Option<Arr>,
    sonando: Option<(u64, u8)>,
    bajar: bool,
}

struct Arr {
    nota: usize,
    orilla: bool,
    t: f64,
    ini: f64,
    dur: f64,
}

impl Default for Estado {
    fn default() -> Self {
        Estado { patron: None, zoom: 90.0, x0: 0.0, paso: 0.25, largo: 1.0, arrastre: None, sonando: None, bajar: true }
    }
}

const TECLA: f32 = 14.0;
const TECLAS: f32 = 62.0;
const VEL: f32 = 70.0;
/// Teclas visibles en el piano roll: de Do1 a Do8.
const MIN: i32 = 24;
const MAX: i32 = 108;

fn nombre_tecla(t: i32) -> String {
    let n = ["Do", "Do♯", "Re", "Re♯", "Mi", "Fa", "Fa♯", "Sol", "Sol♯", "La", "La♯", "Si"];
    format!("{}{}", n[t.rem_euclid(12) as usize].replace('♯', "#"), t / 12 - 1)
}

fn negra(t: i32) -> bool {
    matches!(t.rem_euclid(12), 1 | 3 | 6 | 8 | 10)
}

impl App {
    /// Pista y patrón que se están editando: el elegido, o el primero de la pista elegida.
    fn patron_actual(&mut self) -> Option<(u64, u64)> {
        // Elegir otra pista de instrumento cambia lo que se edita.
        if let Canal::Pista(id) = self.canal {
            if let Some(t) = self.p.pistas.iter().find(|t| t.id == id && t.instrumento.is_some() && !t.patrones.is_empty()) {
                if !t.patrones.iter().any(|c| Some(c.patron) == self.musica.patron) {
                    self.musica.patron = Some(t.patrones[0].patron);
                }
            }
        }
        if let Some(pat) = self.musica.patron.filter(|x| self.p.patrones.iter().any(|p| p.id == *x)) {
            if let Some(t) = self.p.pistas.iter().find(|t| t.patrones.iter().any(|c| c.patron == pat)) {
                return Some((t.id, pat));
            }
        }
        let con = |t: &&Pista| t.instrumento.is_some() && !t.patrones.is_empty();
        let t = match self.canal {
            Canal::Pista(id) => self.p.pistas.iter().filter(con).find(|t| t.id == id),
            _ => None,
        };
        let t = t.or_else(|| self.p.pistas.iter().find(con))?;
        let r = (t.id, t.patrones[0].patron);
        self.musica.patron = Some(r.1);
        Some(r)
    }

    pub fn agregar_instrumento(&mut self, ins: Instrumento, nombre: &str) {
        let compas = 4.0 * self.p.pulso();
        // Al principio del compás donde está el cursor.
        let t = (self.ahora() / compas + 0.001).floor() * compas;
        let bat = ins == Instrumento::Bateria;
        let (pista, pat) = self.p.nueva_pista_instrumento(nombre, ins, t);
        self.canal = Canal::Pista(pista);
        self.musica.patron = Some(pat);
        self.musica.bajar = true;
        self.espacio = Espacio::Audio;
        self.pestana = if bat { Pestana::Rack } else { Pestana::Piano };
    }

    /// Lista de sonidos por familia. Devuelve el elegido y su nombre.
    fn menu_sonidos(ui: &mut Ui) -> Option<(Instrumento, String)> {
        let mut r = None;
        for (f, l) in FAMILIAS {
            ui.menu_button(f, |ui| {
                for n in l {
                    if ui.button(*n).clicked() {
                        r = SONIDOS.iter().find(|s| s.0 == *n).map(|s| (Instrumento::Trio(s.1.to_vec()), n.to_string()));
                    }
                }
            });
        }
        if ui.button(crate::idioma::t("Batería")).clicked() {
            r = Some((Instrumento::Bateria, "Batería".into()));
        }
        ui.separator();
        if ui.button(crate::idioma::t("Mi propio sonido (archivo)…")).clicked() {
            if let Some(x) = rfd::FileDialog::new().add_filter(crate::idioma::t("Sonido"), &["wav", "mp3", "flac", "ogg", "m4a", "aif", "aiff"]).pick_file() {
                let n = nombre_de(&x);
                r = Some((Instrumento::Muestra { ruta: x, raiz: 60 }, n));
            }
        }
        r
    }

    /// Menú para una pista nueva de instrumento.
    pub fn menu_instrumentos(&mut self, ui: &mut Ui) {
        if let Some((i, n)) = Self::menu_sonidos(ui) {
            if let Instrumento::Muestra { ruta, .. } = &i {
                self.cargar_muestra(ruta.clone());
            }
            self.agregar_instrumento(i, &n);
        }
    }

    /// Cambia el sonido de una pista sin tocar sus notas.
    pub fn cambiar_sonido(&mut self, pista: u64, i: Instrumento, n: &str) {
        if let Instrumento::Muestra { ruta, .. } = &i {
            self.cargar_muestra(ruta.clone());
        }
        if let Some(p) = self.p.pista(pista) {
            // Si la pista se llama como su sonido, cambia también su nombre.
            let viejo = p.instrumento.as_ref().map(nombre_sonido);
            if viejo.is_some_and(|v| p.nombre.starts_with(&v)) {
                p.nombre = n.into();
            }
            p.instrumento = Some(i);
        }
    }

    /// Columna de instrumentos del Rack y del Piano roll.
    pub fn lista_instrumentos(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        ui.label(RichText::new(crate::idioma::t("INSTRUMENTOS")).size(12.5).color(TEXT_FAINT));
        ui.label(RichText::new(crate::idioma::t("Elige cuál vas a tocar")).size(12.0).color(TEXT_FAINT));
        ui.add_space(4.0);
        let actual = self.patron_actual().map(|x| x.0);
        let lista: Vec<(usize, u64, String, String)> =
            self.p.pistas.iter().enumerate().filter_map(|(i, t)| Some((i, t.id, t.nombre.clone(), nombre_sonido(t.instrumento.as_ref()?)))).collect();
        ScrollArea::vertical().id_salt("lista-ins").auto_shrink([false, false]).show(ui, |ui| {
            for (i, id, n, sonido) in lista {
                let col = color_pista(i);
                let on = actual == Some(id);
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width() - 4.0, 46.0), Sense::click());
                resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, crate::idioma::tf("Tocar {}", &[&n])));
                let p = ui.painter();
                p.rect_filled(r, 6, if on { c(0x484848) } else if resp.hovered() { BUTTON_HI } else { c(0x353535) });
                p.rect_filled(Rect::from_min_size(r.min, vec2(6.0, r.height())), CornerRadius { nw: 6, sw: 6, ne: 0, se: 0 }, col);
                let pt = ui.painter_at(r.shrink2(vec2(4.0, 0.0)));
                pt.text(r.min + vec2(14.0, 14.0), Align2::LEFT_CENTER, &n, fuente(15.0), if on { TEXT } else { TEXT_DIM });
                pt.text(r.min + vec2(14.0, 32.0), Align2::LEFT_CENTER, &sonido, fuente(12.0), if on { MAIN } else { TEXT_FAINT });
                if on {
                    p.rect_stroke(r, 6, Stroke::new(1.5, MAIN), StrokeKind::Inside);
                }
                if resp.on_hover_text(crate::idioma::t("Clic: editar sus notas")).clicked() {
                    self.canal = Canal::Pista(id);
                    self.musica.bajar = true;
                    // Una pista sin patrones estrena uno.
                    if self.p.pista(id).is_some_and(|p| p.patrones.is_empty()) {
                        let pat = self.p.nuevo_patron(&n);
                        let (clip, dur) = (self.p.id(), 4.0 * self.p.pulso());
                        if let Some(t) = self.p.pista(id) {
                            t.patrones.push(ClipPatron { id: clip, patron: pat, inicio: 0.0, dur });
                        }
                        self.musica.patron = Some(pat);
                    }
                }
                ui.add_space(3.0);
            }
            ui.add_space(4.0);
            ui.menu_button(RichText::new(crate::idioma::t("+ Agregar instrumento")).color(MAIN), |ui| self.menu_instrumentos(ui));
        });
    }

    /// Nota de prueba con el ratón.
    fn probar(&mut self, pista: u64, tecla: u8) {
        self.sincronizar();
        self.callar_prueba();
        self.motor.lock().unwrap().probar(pista, tecla, 100, true);
        self.musica.sonando = Some((pista, tecla));
    }

    fn callar_prueba(&mut self) {
        if let Some((p, t)) = self.musica.sonando.take() {
            self.motor.lock().unwrap().probar(p, t, 0, false);
        }
    }

    /// Barra común de Rack y Piano roll.
    fn barra_patron(&mut self, ui: &mut Ui, pista: u64, pat: u64) {
        ui.horizontal(|ui| {
            ui.set_min_height(26.0);
            ui.spacing_mut().item_spacing.x = 5.0;
            let sonido = self.p.pista(pista).and_then(|p| p.instrumento.as_ref().map(nombre_sonido)).unwrap_or_default();
            ui.label(RichText::new(crate::idioma::t("Suena")).size(13.0).color(TEXT_FAINT));
            let mut nuevo = None;
            ui.menu_button(RichText::new(crate::idioma::tf("{} ▾", &[&sonido])).color(MAIN), |ui| nuevo = Self::menu_sonidos(ui)).response.on_hover_text(crate::idioma::t("Cambiar el instrumento de esta pista"));
            if let Some((i, n)) = nuevo {
                self.cambiar_sonido(pista, i, &n);
            }
            ui.separator();
            let mut cambiar = None;
            let (mut guardar, mut importar, mut conectar) = (false, false, None);
            let de_pista: Vec<u64> = self.p.pista(pista).map(|p| p.patrones.iter().map(|c| c.patron).collect()).unwrap_or_default();
            let lista: Vec<(u64, String)> = self.p.patrones.iter().filter(|p| de_pista.contains(&p.id)).map(|p| (p.id, p.nombre.clone())).collect();
            let Some(q) = self.p.patron(pat) else { return };
            ui.add(TextEdit::singleline(&mut q.nombre).desired_width(110.0));
            ui.menu_button(crate::idioma::t("Patrones"), |ui| {
                for (id, n) in &lista {
                    if ui.button(crate::idioma::t(&n)).clicked() {
                        cambiar = Some(*id);
                    }
                }
            });
            ui.label(crate::idioma::t("Largo"));
            let mut c = q.largo / 4.0;
            if ui.add(DragValue::new(&mut c).range(1.0..=32.0).speed(0.05).max_decimals(0).suffix(" compases")).changed() {
                q.largo = c.round() * 4.0;
            }
            if ui.button("×2").on_hover_text(crate::idioma::t("Duplica el patrón: copia las notas en la segunda mitad")).clicked() {
                let l = q.largo;
                let copia: Vec<Nota> = q.notas.iter().filter(|n| n.ini < l).map(|n| Nota { ini: n.ini + l, ..n.clone() }).collect();
                q.notas.extend(copia);
                q.largo = (l * 2.0).min(128.0);
            }
            if ui.button(crate::idioma::t("Cuantizar")).on_hover_text(crate::idioma::t("Alinea las notas al ajuste")).clicked() {
                let p = self.musica.paso;
                q.notas.iter_mut().for_each(|n| n.ini = (n.ini / p).round() * p);
            }
            if ui.button(crate::idioma::t("Vaciar")).clicked() {
                q.notas.clear();
            }
            ui.menu_button(crate::idioma::t("MIDI"), |ui| {
                if ui.button(crate::idioma::t("Guardar patrón como .mid…")).clicked() {
                    guardar = true;
                }
                if ui.button(crate::idioma::t("Importar .mid…")).clicked() {
                    importar = true;
                }
                ui.separator();
                ui.label(RichText::new(crate::idioma::t("Teclado")).size(12.0).color(TEXT_FAINT));
                for n in crate::midi::teclados() {
                    let on = self.teclado.as_ref().is_some_and(|t| t.nombre == n);
                    if ui.selectable_label(on, &n).clicked() {
                        conectar = Some(n);
                    }
                }
                if let Some(t) = &self.teclado {
                    ui.label(RichText::new(crate::idioma::tf("Conectado: {}", &[&t.nombre])).size(12.0).color(MAIN));
                }
            });
            if letra(ui, "●", self.midi_rec, RECORD, "Grabar el teclado MIDI en este patrón mientras suena").clicked() {
                self.midi_rec = !self.midi_rec;
            }
            ComboBox::from_id_salt("paso").width(64.0).selected_text(match self.musica.paso {
                x if x == 1.0 => "1/4",
                x if x == 0.5 => "1/8",
                x if x == 0.125 => "1/32",
                _ => "1/16",
            }).show_ui(ui, |ui| {
                for (v, n) in [(1.0, "1/4"), (0.5, "1/8"), (0.25, "1/16"), (0.125, "1/32")] {
                    ui.selectable_value(&mut self.musica.paso, v, n);
                }
            });
            if ui.button(crate::idioma::t("Patrón nuevo")).on_hover_text(crate::idioma::t("Vacío, puesto después del último de esta pista")).clicked() {
                let n = self.p.patrones.len() + 1;
                let id = self.p.nuevo_patron(&crate::idioma::tf("Patrón {}", &[&n]));
                let (clip, dur) = (self.p.id(), 4.0 * self.p.pulso());
                if let Some(t) = self.p.pista(pista) {
                    let ini = t.patrones.iter().map(|c| c.inicio + c.dur).fold(0.0, f64::max);
                    t.patrones.push(ClipPatron { id: clip, patron: id, inicio: ini, dur });
                }
                cambiar = Some(id);
            }
            if cambiar.is_some() {
                self.musica.patron = cambiar;
            }
            if guardar {
                self.guardar_midi(pat);
            }
            if importar {
                self.importar_midi_dialogo();
            }
            if let Some(n) = conectar {
                match crate::midi::conectar(&n) {
                    Ok(t) => self.teclado = Some(t),
                    Err(e) => self.avisar(e, true),
                }
            }
        });
    }

    fn sin_patron(&mut self, ui: &mut Ui) {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(crate::idioma::t("Agrega un instrumento para empezar")).size(17.0).color(TEXT_FAINT));
            ui.add_space(8.0);
            ui.menu_button(RichText::new(crate::idioma::t("+ Instrumento")).color(MAIN).size(16.0), |ui| self.menu_instrumentos(ui));
        });
    }

    /// Rack: pasos de dieciseisavo por pieza.
    pub fn ui_rack(&mut self, ui: &mut Ui) {
        let Some((pista, pat)) = self.patron_actual() else {
            self.sin_patron(ui);
            return;
        };
        Frame::new().inner_margin(8.0).show(ui, |ui| {
            self.barra_patron(ui, pista, pat);
            ui.add_space(6.0);
            let bat = self.p.pista(pista).is_some_and(|p| p.instrumento == Some(Instrumento::Bateria));
            let filas: Vec<(u8, String)> = if bat {
                PIEZAS.iter().map(|(t, n)| (*t, n.to_string())).collect()
            } else {
                // En un instrumento melódico, las notas que ya usa y una octava desde Do4.
                let mut v: Vec<u8> = self.p.patron(pat).map(|p| p.notas.iter().map(|n| n.tecla).collect()).unwrap_or_default();
                v.extend(60..=72);
                v.sort();
                v.dedup();
                v.into_iter().rev().map(|t| (t, nombre_tecla(t as i32))).collect()
            };
            let pulso = (self.ahora() / self.p.pulso()).rem_euclid(1e9);
            let tocando = self.tocando();
            let inicio = self.p.pista(pista).and_then(|t| t.patrones.iter().find(|c| c.patron == pat).map(|c| c.inicio)).unwrap_or(0.0) / self.p.pulso();
            let Some(largo) = self.p.patron(pat).map(|q| q.largo) else { return };
            let n = (largo * 4.0).round() as usize;
            let actual = ((pulso - inicio).rem_euclid(largo) * 4.0) as usize;
            let mut probar = None;
            ScrollArea::both().id_salt("rack").auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(3.0, 4.0);
                for (tecla, nombre) in &filas {
                    ui.horizontal(|ui| {
                        let (r, resp) = ui.allocate_exact_size(vec2(150.0, 30.0), Sense::click());
                        relieve(ui.painter(), r, 6, BUTTON, resp.hovered(), resp.is_pointer_button_down_on());
                        ui.painter().text(r.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, nombre, fuente(14.5), TEXT);
                        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, nombre));
                        if resp.clicked() {
                            probar = Some(*tecla);
                        }
                        for k in 0..n {
                            let t = k as f64 * 0.25;
                            let q = self.p.patron(pat).unwrap();
                            let i = q.notas.iter().position(|x| x.tecla == *tecla && (x.ini - t).abs() < 0.06);
                            let (r, resp) = ui.allocate_exact_size(vec2(24.0, 30.0), Sense::click_and_drag());
                            resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, i.is_some(), crate::idioma::tf("{} paso {}", &[&nombre, &(k + 1)])));
                            let base = if (k / 4) % 2 == 0 { c(0x4a4a4a) } else { c(0x5b4f22) };
                            let pr = ui.painter();
                            if let Some(i) = i {
                                let f = q.notas[i].vel as f32 / 127.0;
                                degradado(pr, r, 5, mezcla(DEEP, HIGHLIGHT, f), mezcla(DEEP, MAIN, f));
                            } else {
                                pr.rect_filled(r, 5, if resp.hovered() { mezcla(base, Color32::WHITE, 0.15) } else { base });
                            }
                            if tocando && k == actual {
                                pr.rect_stroke(r, 5, Stroke::new(2.0, Color32::WHITE), StrokeKind::Inside);
                            }
                            if resp.clicked() {
                                match i {
                                    Some(i) => {
                                        q.notas.remove(i);
                                    }
                                    None => {
                                        q.notas.push(Nota { tecla: *tecla, vel: 100, ini: t, dur: 0.25 });
                                        probar = Some(*tecla);
                                    }
                                }
                            } else if let (Some(i), true) = (i, resp.dragged()) {
                                // Arrastrar arriba o abajo cambia la fuerza.
                                let v = q.notas[i].vel as f32 - resp.drag_delta().y;
                                q.notas[i].vel = v.clamp(1.0, 127.0) as u8;
                            }
                            resp.on_hover_text(crate::idioma::t("Clic: poner o quitar. Arrastra arriba o abajo: fuerza"));
                        }
                    });
                }
            });
            if let Some(t) = probar {
                self.probar(pista, t);
            }
        });
    }

    pub fn ui_piano(&mut self, ui: &mut Ui) {
        let Some((pista, pat)) = self.patron_actual() else {
            self.sin_patron(ui);
            return;
        };
        Frame::new().inner_margin(Margin::symmetric(8, 6)).show(ui, |ui| self.barra_patron(ui, pista, pat));
        let todo = ui.available_rect_before_wrap();
        let vel = Rect::from_min_max(pos2(todo.left(), todo.bottom() - VEL), todo.max);
        let cuerpo = Rect::from_min_max(todo.min, pos2(todo.right(), vel.top() - 3.0));
        let izq = todo.left() + TECLAS;
        // Zoom y desplazamiento horizontal
        if ui.input(|i| i.pointer.hover_pos()).is_some_and(|m| todo.contains(m)) {
            let (z, s) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta()));
            self.musica.zoom = (self.musica.zoom * z).clamp(20.0, 400.0);
            self.musica.x0 = (self.musica.x0 - (s.x / self.musica.zoom) as f64).max(0.0);
        }
        let (zoom, x0, paso) = (self.musica.zoom, self.musica.x0, self.musica.paso);
        let Some(largo) = self.p.patron(pat).map(|q| q.largo) else { return };
        let a_x = move |t: f64| izq + ((t - x0) as f32) * zoom;
        let a_t = move |x: f32| x0 + ((x - izq) / zoom) as f64;
        let ajusta = |t: f64| (t / paso + 1e-6).floor() * paso;
        let color = self.p.pistas.iter().position(|p| p.id == pista).map_or(MAIN, color_pista);
        let pulso = self.p.pulso();
        let inicio = self.p.pista(pista).and_then(|t| t.patrones.iter().find(|c| c.patron == pat).map(|c| (c.inicio, c.dur)));
        let t_rep = self.tocando().then(|| self.ahora()).and_then(|t| inicio.filter(|(i, d)| t >= *i && t < i + d).map(|(i, _)| ((t - i) / pulso).rem_euclid(largo)));
        let mut probar: Option<u8> = None;
        let mut soltar = false;
        let mut hijo = ui.new_child(UiBuilder::new().max_rect(cuerpo));
        let mut sa = ScrollArea::vertical().id_salt("piano").auto_shrink([false, false]);
        if std::mem::take(&mut self.musica.bajar) {
            // Empieza a la altura de Do5.
            sa = sa.vertical_scroll_offset(((MAX - 76) as f32 * TECLA - cuerpo.height() / 2.0).max(0.0));
        }
        sa.show(&mut hijo, |ui| {
            let (r, _) = ui.allocate_exact_size(vec2(cuerpo.width(), (MAX - MIN + 1) as f32 * TECLA), Sense::hover());
            let y_de = |t: i32| r.top() + (MAX - t) as f32 * TECLA;
            let rejilla = Rect::from_min_max(pos2(izq, r.top()), r.max);
            let vis = ui.clip_rect();
            let pr = ui.painter_at(rejilla.intersect(vis));
            let pt = ui.painter_at(Rect::from_min_max(r.min, pos2(izq, r.bottom())).intersect(vis));
            for t in MIN..=MAX {
                let f = Rect::from_min_size(pos2(r.left(), y_de(t)), vec2(r.width(), TECLA));
                if !f.intersects(vis) {
                    continue;
                }
                pr.rect_filled(f, 0, if negra(t) { c(0x232323) } else { c(0x2b2b2b) });
                pr.hline(f.x_range(), f.bottom(), Stroke::new(1.0, if t % 12 == 0 { c(0x111111) } else { c(0x202020) }));
                let k = Rect::from_min_size(f.min, vec2(TECLAS - 2.0, TECLA - 1.0));
                pt.rect_filled(k, 2, if negra(t) { c(0x2a2a2a) } else { c(0xe3e3e3) });
                if t % 12 == 0 {
                    pt.text(k.right_center() - vec2(5.0, 0.0), Align2::RIGHT_CENTER, nombre_tecla(t), fuente(12.0), c(0x555555));
                }
            }
            let mut b = (x0 / paso).floor() * paso;
            while a_x(b) < r.right() {
                let col = if (b / 4.0).fract().abs() < 1e-6 { c(0x0c0c0c) } else if b.fract().abs() < 1e-6 { c(0x1a1a1a) } else { c(0x262626) };
                pr.vline(a_x(b), rejilla.y_range(), Stroke::new(1.0, col));
                b += paso;
            }
            pr.rect_filled(Rect::from_min_max(pos2(a_x(largo), r.top()), r.max), 0, Color32::from_black_alpha(130));
            // Teclado
            let rt = ui.interact(Rect::from_min_max(r.min, pos2(izq, r.bottom())).intersect(vis), Id::new("teclado"), Sense::click_and_drag());
            if let (true, Some(m)) = (rt.is_pointer_button_down_on(), rt.interact_pointer_pos()) {
                let t = (MAX - ((m.y - r.top()) / TECLA) as i32).clamp(MIN, MAX) as u8;
                if self.musica.sonando.map(|s| s.1) != Some(t) {
                    probar = Some(t);
                }
            } else if rt.drag_stopped() || rt.clicked() {
                soltar = true;
            }
            // Rejilla: clic pone, arrastrar mueve o alarga, clic derecho quita.
            let resp = ui.interact(rejilla.intersect(vis), Id::new("rejilla"), Sense::click_and_drag());
            let Some(q) = self.p.patrones.iter_mut().find(|p| p.id == pat) else { return };
            let caja = |n: &Nota| Rect::from_min_size(pos2(a_x(n.ini), y_de(n.tecla as i32) + 1.0), vec2((n.dur as f32 * zoom - 1.0).max(4.0), TECLA - 2.0));
            let en = |q: &Patron, m: Pos2| q.notas.iter().rposition(|n| caja(n).contains(m));
            if let Some(m) = resp.hover_pos() {
                match en(q, m) {
                    Some(i) if caja(&q.notas[i]).right() - m.x < 6.0 => ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal),
                    Some(_) => ui.ctx().set_cursor_icon(CursorIcon::Grab),
                    None => ui.ctx().set_cursor_icon(CursorIcon::Crosshair),
                }
            }
            let ratón = resp.interact_pointer_pos();
            if resp.secondary_clicked() {
                if let Some(i) = ratón.and_then(|m| en(q, m)) {
                    q.notas.remove(i);
                }
            } else if let (true, Some(m)) = (resp.drag_started_by(PointerButton::Primary) || resp.clicked_by(PointerButton::Primary), ratón) {
                let m0 = ui.input(|i| i.pointer.press_origin()).unwrap_or(m);
                let i = en(q, m0).unwrap_or_else(|| {
                    let tecla = (MAX - ((m0.y - r.top()) / TECLA) as i32).clamp(MIN, MAX) as u8;
                    q.notas.push(Nota { tecla, vel: 100, ini: ajusta(a_t(m0.x)).max(0.0), dur: self.musica.largo });
                    probar = Some(tecla);
                    q.notas.len() - 1
                });
                let n = &q.notas[i];
                let orilla = caja(n).right() - m0.x < 6.0;
                if resp.drag_started() {
                    self.musica.arrastre = Some(Arr { nota: i, orilla, t: a_t(m0.x), ini: n.ini, dur: n.dur });
                } else {
                    probar = Some(n.tecla);
                }
            }
            if let (true, Some(m), Some(a)) = (resp.dragged(), ratón, &self.musica.arrastre) {
                if let Some(n) = q.notas.get_mut(a.nota) {
                    let d = a_t(m.x) - a.t;
                    if a.orilla {
                        n.dur = ((a.dur + d) / paso).round().max(1.0) * paso;
                        self.musica.largo = n.dur;
                    } else {
                        n.ini = ((a.ini + d) / paso).round().max(0.0) * paso;
                        let t = (MAX - ((m.y - r.top()) / TECLA) as i32).clamp(MIN, MAX) as u8;
                        if n.tecla != t {
                            n.tecla = t;
                            probar = Some(t);
                        }
                    }
                }
            }
            if resp.drag_stopped() || resp.clicked() {
                self.musica.arrastre = None;
                soltar = resp.drag_stopped();
            }
            for n in &q.notas {
                let k = caja(n);
                if k.intersects(vis) {
                    let f = 0.35 + 0.65 * n.vel as f32 / 127.0;
                    degradado(&pr, k, 3, mezcla(color, Color32::WHITE, 0.35 * f), mezcla(c(0x2a2a2a), color, f));
                    if k.width() > 26.0 {
                        pr.text(k.left_center() + vec2(4.0, 0.0), Align2::LEFT_CENTER, nombre_tecla(n.tecla as i32), fuente(11.0), c(0x1c1c1c));
                    }
                }
            }
            if let Some(t) = t_rep {
                pr.vline(a_x(t), rejilla.y_range(), Stroke::new(1.5, MAIN));
            }
        });
        // Fuerza de cada nota
        let pv = ui.painter_at(vel);
        pv.rect_filled(vel, 0, WELL);
        pv.text(vel.left_center() + vec2(8.0, 0.0), Align2::LEFT_CENTER, crate::idioma::t("Fuerza"), fuente(13.0), TEXT_DIM);
        let rv = ui.interact(Rect::from_min_max(pos2(izq, vel.top()), vel.max), Id::new("fuerza"), Sense::click_and_drag());
        if let Some(q) = self.p.patron(pat) {
            if let (true, Some(m)) = (rv.dragged() || rv.clicked(), rv.interact_pointer_pos()) {
                let v = ((vel.bottom() - m.y) / VEL * 127.0).clamp(1.0, 127.0) as u8;
                let t = a_t(m.x);
                if let Some(cerca) = q.notas.iter().map(|n| (n.ini - t).abs()).min_by(f64::total_cmp).filter(|d| *d < 0.3) {
                    q.notas.iter_mut().filter(|n| ((n.ini - t).abs() - cerca).abs() < 1e-6).for_each(|n| n.vel = v);
                }
            }
            let pv = ui.painter_at(Rect::from_min_max(pos2(izq, vel.top()), vel.max));
            for n in &q.notas {
                let (x, y) = (a_x(n.ini) + 3.0, vel.bottom() - n.vel as f32 / 127.0 * (VEL - 8.0));
                pv.vline(x, y..=vel.bottom(), Stroke::new(2.0, mezcla(color, Color32::WHITE, 0.2)));
                pv.circle_filled(pos2(x, y), 4.0, MAIN);
            }
        }
        ui.allocate_rect(todo, Sense::hover());
        if let Some(t) = probar {
            self.probar(pista, t);
        }
        if soltar || (self.musica.sonando.is_some() && !ui.input(|i| i.pointer.any_down())) {
            self.callar_prueba();
        }
    }

    /// Controles del instrumento de la pista elegida.
    pub fn instrumento_ui(&mut self, ui: &mut Ui) {
        let Canal::Pista(id) = self.canal else { return };
        let mut cargar = None;
        let mut cambio = None;
        let Some(ins) = self.p.pista(id).and_then(|p| p.instrumento.as_mut()) else { return };
        Frame::new().fill(SLOT).corner_radius(6).inner_margin(8.0).outer_margin(6.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            match ins {
                Instrumento::Trio(v) => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(crate::idioma::t("Sintetizador")).size(16.0).color(MAIN));
                        ui.menu_button(crate::idioma::t("Sonidos"), |ui| {
                            if let Some((i, n)) = Self::menu_sonidos(ui) {
                                cambio = Some((i, n));
                            }
                        });
                    });
                    v.resize(TRIO.len(), 0.0);
                    ui.horizontal(|ui| {
                        for k in 0..3 {
                            ComboBox::from_id_salt(("onda", k)).width(96.0).selected_text(ONDAS[v[k].clamp(0.0, 5.0) as usize]).show_ui(ui, |ui| {
                                for (i, n) in ONDAS.iter().enumerate() {
                                    ui.selectable_value(&mut v[k], i as f32, *n);
                                }
                            });
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(2.0, 3.0);
                        for (k, d) in TRIO.iter().enumerate().skip(3) {
                            ui.allocate_ui_with_layout(vec2(62.0, 64.0), Layout::top_down(Align::Center), |ui| {
                                ui.set_width(62.0);
                                ui.spacing_mut().item_spacing.y = 1.0;
                                ui.label(RichText::new(crate::idioma::t(&d.nombre)).size(11.5).color(TEXT_DIM));
                                perilla(ui, &mut v[k], d.min, d.max, d.def, d.log, 15.0, d.nombre);
                                let s = if d.unidad == "st" { format!("{:+.1}", v[k]) } else { format!("{:.0} {}", v[k], d.unidad) };
                                ui.label(RichText::new(crate::idioma::t(&s)).size(11.5));
                            });
                        }
                    });
                }
                Instrumento::Bateria => {
                    ui.label(RichText::new(crate::idioma::t("Batería")).size(16.0).color(MAIN));
                    ui.label(RichText::new(crate::idioma::t("Haz el ritmo en el Rack")).color(TEXT_DIM));
                }
                Instrumento::Muestra { ruta, raiz } => {
                    ui.label(RichText::new(crate::idioma::t("Muestra")).size(16.0).color(MAIN));
                    ui.horizontal(|ui| {
                        if ui.button(nombre_de(ruta)).on_hover_text(crate::idioma::t("Cambiar sonido")).clicked() {
                            if let Some(r) = rfd::FileDialog::new().add_filter(crate::idioma::t("Sonido"), &["wav", "mp3", "flac", "ogg", "m4a", "aif", "aiff"]).pick_file() {
                                *ruta = r.clone();
                                cargar = Some(r);
                            }
                        }
                        ui.label(crate::idioma::t("Nota original"));
                        ui.add(DragValue::new(raiz).range(12..=108).custom_formatter(|v, _| nombre_tecla(v as i32)));
                    });
                }
            }
        });
        if let Some((i, n)) = cambio {
            self.cambiar_sonido(id, i, &n);
        }
        if let Some(r) = cargar {
            self.cargar_muestra(r);
        }
    }

    /// Patrones de una pista en la Playlist.
    pub fn patrones_ui(&mut self, ui: &mut Ui, i: usize, carril: Rect, a_x: &dyn Fn(f64) -> f32, a_t: &dyn Fn(f32) -> f64, ac: &mut Vec<linea::Accion>) {
        let col = color_pista(i);
        let id_pista = self.p.pistas[i].id;
        let pr = ui.painter_at(carril);
        let pulso = self.p.pulso();
        let (mut borrar, mut duplicar) = (None, None);
        for j in 0..self.p.pistas[i].patrones.len() {
            let k = self.p.pistas[i].patrones[j].clone();
            let rc = Rect::from_min_max(pos2(a_x(k.inicio), carril.top() + 2.0), pos2(a_x(k.inicio + k.dur), carril.bottom() - 3.0));
            if !rc.intersects(carril) {
                continue;
            }
            pr.rect_filled(rc, 5, mezcla(col, c(0x141414), 0.62));
            let tit = Rect::from_min_size(rc.min, vec2(rc.width(), 17.0));
            pr.rect_filled(tit, CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 }, mezcla(col, c(0x303030), 0.3));
            let cuerpo = Rect::from_min_max(pos2(rc.left(), rc.top() + 19.0), rc.max - vec2(0.0, 3.0));
            if let Some(q) = self.p.patrones.iter().find(|p| p.id == k.patron) {
                let pt = ui.painter_at(tit.intersect(carril));
                icono(&pt, &iconos::PIANO, Rect::from_min_size(tit.min + vec2(5.0, 3.0), Vec2::splat(12.0)), Color32::WHITE);
                pt.text(tit.left_center() + vec2(22.0, 0.0), Align2::LEFT_CENTER, &q.nombre, fuente(13.0), Color32::WHITE);
                let (lo, hi) = q.notas.iter().fold((127u8, 0u8), |(a, b), n| (a.min(n.tecla), b.max(n.tecla)));
                let alto = (hi.max(lo) - lo.min(hi)).max(8) as f32 + 1.0;
                let pn = ui.painter_at(cuerpo.intersect(carril));
                let vuelta = q.largo * pulso;
                let mut t0 = k.inicio;
                while t0 < k.inicio + k.dur - 1e-6 {
                    if t0 > k.inicio {
                        pn.vline(a_x(t0), cuerpo.y_range(), Stroke::new(1.0, c(0x1a1a1a)));
                    }
                    for n in q.notas.iter().filter(|n| n.ini < q.largo) {
                        let a = t0 + n.ini * pulso;
                        if a < k.inicio + k.dur {
                            let y = cuerpo.bottom() - (n.tecla.saturating_sub(lo) as f32 + 1.0) / alto * cuerpo.height();
                            let x1 = a_x((a + n.dur * pulso).min(k.inicio + k.dur));
                            pn.rect_filled(Rect::from_min_max(pos2(a_x(a), y), pos2(x1.max(a_x(a) + 2.0), y + (cuerpo.height() / alto).clamp(2.0, 6.0))), 1, mezcla(col, Color32::WHITE, 0.7));
                        }
                    }
                    t0 += vuelta.max(0.05);
                }
            }
            if self.clip == Some(k.id) {
                pr.rect_stroke(rc, 5, Stroke::new(2.0, MAIN), StrokeKind::Inside);
            }
            let resp = ui.interact(rc.intersect(carril), Id::new(("patron", k.id)), Sense::click_and_drag());
            let bloqueada = self.p.pistas[i].bloqueada;
            if !self.linea.auto_ver.contains_key(&id_pista) {
                self.tocar_patron(ui, &resp, rc, k.id, bloqueada, a_t, ac);
            }
            if resp.clicked() || resp.drag_started() {
                self.canal = Canal::Pista(id_pista);
                self.musica.patron = Some(k.patron);
            }
            if resp.double_clicked() {
                let bat = self.p.pistas[i].instrumento == Some(Instrumento::Bateria);
                self.espacio = Espacio::Audio;
                self.pestana = if bat { Pestana::Rack } else { Pestana::Piano };
            }
            resp.context_menu(|ui| {
                if ui.button(crate::idioma::t("Editar notas")).clicked() {
                    self.espacio = Espacio::Audio;
                    self.pestana = Pestana::Piano;
                    self.musica.patron = Some(k.patron);
                }
                if ui.button(crate::idioma::t("Repetir después")).clicked() {
                    duplicar = Some(j);
                }
                if ui.button(crate::idioma::t("Borrar")).clicked() {
                    borrar = Some(k.id);
                }
            });
        }
        if let Some(j) = duplicar {
            let id = self.p.id();
            let mut n = self.p.pistas[i].patrones[j].clone();
            n.inicio += n.dur;
            n.id = id;
            self.p.pistas[i].patrones.push(n);
        }
        if let Some(id) = borrar {
            self.p.borrar_clip(id);
        }
    }

    fn guardar_midi(&mut self, pat: u64) {
        let bpm = self.p.bpm;
        let Some(q) = self.p.patron(pat).cloned() else { return };
        if let Some(r) = rfd::FileDialog::new().set_file_name(format!("{}.mid", q.nombre)).add_filter(crate::idioma::t("MIDI"), &["mid"]).save_file() {
            match std::fs::write(&r, crate::midi::escribir(&q.notas, bpm, &q.nombre)) {
                Ok(()) => self.avisar(crate::idioma::tf("Guardado en {}", &[&(r.display())]), false),
                Err(e) => self.avisar(crate::idioma::tf("No pude guardar: {}", &[&e]), true),
            }
        }
    }

    fn importar_midi_dialogo(&mut self) {
        if let Some(r) = rfd::FileDialog::new().add_filter(crate::idioma::t("MIDI"), &["mid", "midi"]).pick_file() {
            self.importar_midi(&r);
        }
    }

    /// Cada pista del archivo entra como una pista de instrumento.
    pub fn importar_midi(&mut self, r: &Path) {
        let Some((bpm, pistas)) = std::fs::read(r).ok().and_then(|b| crate::midi::leer(&b)) else {
            self.avisar(crate::idioma::t("No pude leer ese archivo MIDI"), true);
            return;
        };
        if let Some(b) = bpm {
            self.p.bpm = b.clamp(40.0, 240.0);
        }
        for (n, notas) in pistas {
            let bat = n == "Batería";
            let ins = if bat { Instrumento::Bateria } else { Instrumento::Trio(SONIDOS[1].1.to_vec()) };
            let (pista, pat) = self.p.nueva_pista_instrumento(&n, ins, 0.0);
            let fin = notas.iter().map(|x| x.ini + x.dur).fold(4.0, f64::max);
            let largo = (fin / 4.0).ceil() * 4.0;
            let q = self.p.patron(pat).unwrap();
            (q.notas, q.largo) = (notas, largo);
            let dur = largo * self.p.pulso();
            self.p.pista(pista).unwrap().patrones[0].dur = dur;
            self.musica.patron = Some(pat);
        }
        self.linea.encuadrar = true;
    }

    /// Teclado MIDI: suena en la pista elegida y, si se graba, escribe en el patrón.
    pub fn atender_teclado(&mut self) {
        let Some(t) = &self.teclado else { return };
        let ev: Vec<(u8, u8)> = t.rx.try_iter().collect();
        if ev.is_empty() {
            return;
        }
        let Some((pista, pat)) = self.patron_actual() else { return };
        self.sincronizar();
        let (ahora, pulso, tocando) = (self.ahora(), self.p.pulso(), self.tocando());
        let inicio = self.p.pista(pista).and_then(|t| t.patrones.iter().find(|c| c.patron == pat && ahora >= c.inicio && ahora < c.inicio + c.dur).map(|c| c.inicio));
        for (tecla, vel) in ev {
            self.motor.lock().unwrap().probar(pista, tecla, vel.max(1), vel > 0);
            let (true, true, Some(ini)) = (self.midi_rec, tocando, inicio) else { continue };
            let Some(q) = self.p.patrones.iter_mut().find(|p| p.id == pat) else { continue };
            let b = ((ahora - ini) / pulso).rem_euclid(q.largo);
            if vel > 0 {
                self.midi_abiertas.push((tecla, q.notas.len()));
                q.notas.push(Nota { tecla, vel, ini: b, dur: 0.25 });
            } else if let Some(k) = self.midi_abiertas.iter().position(|x| x.0 == tecla) {
                let (_, j) = self.midi_abiertas.remove(k);
                if let Some(n) = q.notas.get_mut(j) {
                    n.dur = (b - n.ini).rem_euclid(q.largo).max(0.06);
                }
            }
        }
    }

    /// Tempo y metrónomo, para la barra.
    pub fn tempo_ui(&mut self, ui: &mut Ui) {
        ui.add(DragValue::new(&mut self.p.bpm).range(40.0..=240.0).speed(0.2).max_decimals(1).suffix(" bpm")).on_hover_text(crate::idioma::t("Tempo de los patrones"));
        let on = self.metronomo;
        if boton(ui, &iconos::METRONOME, on, "Metrónomo (no se exporta)").clicked() {
            self.metronomo = !on;
        }
        let c = if self.metronomo { (self.p.pulso() * SR as f64).round() as i64 } else { 0 };
        let mut m = self.motor.lock().unwrap();
        if m.metronomo != c {
            m.metronomo = c;
        }
    }
}

pub fn pestanas(ui: &mut Ui, actual: &mut Pestana) {
    for (p, n) in [(Pestana::Consola, "Consola"), (Pestana::Rack, "Rack"), (Pestana::Piano, "Piano roll"), (Pestana::Editor, "Editor de audio")] {
        if pastilla(ui, n, *actual == p).clicked() {
            *actual = p;
        }
    }
}
