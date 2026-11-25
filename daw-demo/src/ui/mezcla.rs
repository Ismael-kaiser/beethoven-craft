// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! El Mixer: las tiras, las ranuras de efectos de la pista elegida y la ventana de cada efecto.
use super::*;
use crate::motor::eq_de_pista;
use crate::receta::respuesta;

/// Ranuras que se ven; si hay más efectos, se agregan.
const RANURAS: usize = 10;
const AZUL: Color32 = Color32::from_rgb(0x8c, 0xb8, 0xff);

/// Qué hacer con los efectos después de dibujarlos.
enum Cambio {
    Poner(String),
    Plugin,
    Quitar(usize),
    Mover(usize, usize),
    Abrir(usize),
}

impl App {
    pub(super) fn ver_mezcla(&mut self, ui: &mut Ui) {
        const LADO: f32 = 250.0;
        const ANCHO: f32 = 78.0;
        let todo = ui.available_rect_before_wrap();
        let der = Rect::from_min_max(pos2(todo.right() - LADO, todo.top()), todo.max);
        let p = ui.painter();
        p.vline(der.left() - 4.0, der.y_range(), Stroke::new(1.0, Color32::BLACK));
        p.vline(der.left() - 3.0, der.y_range(), Stroke::new(1.0, Color32::from_white_alpha(16)));
        let mut c = ui.new_child(UiBuilder::new().max_rect(der.shrink2(vec2(4.0, 0.0))).id_salt("ranuras"));
        c.set_clip_rect(der.intersect(ui.clip_rect()));
        self.ranuras(&mut c);
        let izq = Rect::from_min_max(todo.min, pos2(der.left() - 8.0, todo.bottom()));
        let mut ui = ui.new_child(UiBuilder::new().max_rect(izq).id_salt("tiras"));
        let alto = izq.height() - 12.0;
        self.tiras.clear();
        ScrollArea::horizontal().auto_shrink(false).show(&mut ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for i in 0..self.p.mezcla.len() {
                    if i == PRIMER_ENVIO {
                        ui.add_space(8.0);
                    }
                    let (r, plana) = ui.allocate_exact_size(vec2(ANCHO, alto), Sense::click());
                    self.tira(ui, i, r, plana);
                }
            });
        });
    }

    fn tira(&mut self, ui: &mut Ui, i: usize, r: Rect, plana: Response) {
        self.tiras.push((i, r.intersect(ui.clip_rect())));
        if plana.clicked() {
            self.pista = i;
        }
        let el = self.pista == i;
        let p = ui.painter();
        placa(p, r, 6);
        p.rect_stroke(r, 6.0, Stroke::new(1.0, if el { ACENTO.gamma_multiply(0.7) } else { Color32::BLACK }), StrokeKind::Inside);
        for (k, o) in [r.left_bottom() + vec2(9.0, -9.0), r.right_bottom() - vec2(9.0, 9.0)].into_iter().enumerate() {
            tornillo(p, o, 3.2, i as f32 * 1.7 + k as f32, false);
        }
        let n = Rect::from_min_size(r.min + vec2(5.0, 5.0), vec2(r.width() - 10.0, 24.0));
        if el {
            encendido(p, n, 4, ACENTO);
        } else {
            hueco(p, n, 4, HUECO);
        }
        let col = if el {
            Color32::from_rgb(0x1a, 0x17, 0x0c)
        } else if i == 0 {
            ACENTO
        } else if i >= PRIMER_ENVIO {
            AZUL
        } else {
            TEXTO
        };
        rotulo(p, n.shrink2(vec2(6.0, 0.0)), &self.p.mezcla[i].nombre, 14.0, col);
        let x = ui.interact(n, plana.id.with("nombre"), Sense::click());
        if x.clicked() {
            self.pista = i;
        }
        let m = &mut self.p.mezcla[i];
        x.context_menu(|ui| {
            ui.text_edit_singleline(&mut m.nombre);
            if (1..PRIMER_ENVIO).contains(&i) && ui.button(t("Pasar la pista a audio")).clicked() {
                self.pasar = Some(i);
                ui.close();
            }
        });
        let area = Rect::from_min_size(pos2(r.center().x - 16.0, n.bottom() + 5.0), vec2(32.0, 32.0));
        let mut c = ui.new_child(UiBuilder::new().max_rect(area).id_salt(("pan", i)));
        perilla(&mut c, "", &mut m.pan, -1.0, 1.0, 0.0, ACENTO, 32.0).on_hover_text(t("Izquierda · derecha"));
        for (k, (s, ayuda, v, c)) in [("M", "Mudo", &mut m.mudo, ROJO), ("S", "Solo", &mut m.solo, ACENTO)].into_iter().enumerate() {
            // Solo las pistas tienen solo.
            if k == 1 && !(1..PRIMER_ENVIO).contains(&i) {
                continue;
            }
            let q = Rect::from_min_size(pos2(r.left() + 6.0 + 22.0 * k as f32, n.bottom() + 42.0), vec2(20.0, 20.0));
            let b = ui.interact(q, plana.id.with(s), Sense::click()).on_hover_text(t(ayuda));
            if *v {
                encendido(ui.painter(), q, 4, c);
            } else {
                relieve(ui.painter(), q, 4, CAJA, b.hovered(), b.is_pointer_button_down_on());
            }
            ui.painter().text(q.center(), Align2::CENTER_CENTER, s, letra(13.0), if *v { FONDO } else { TEXTO });
            if b.clicked() {
                *v = !*v;
            }
        }
        let fx = m.efectos.len();
        let q = Rect::from_min_size(pos2(r.right() - 29.0, n.bottom() + 42.0), vec2(24.0, 20.0));
        hueco(ui.painter(), q, 4, HUECO);
        ui.painter().text(q.center(), Align2::CENTER_CENTER, format!("fx {fx}"), letra(11.5), if fx > 0 { ACENTO } else { TENUE });
        let (y0, y1) = (n.bottom() + 70.0, r.bottom() - 38.0);
        if y1 - y0 > 50.0 {
            let med = Rect::from_min_max(pos2(r.right() - 24.0, y0 + 9.0), pos2(r.right() - 7.0, y1));
            nivel(ui.painter(), med, self.picos[i]);
            // El pico más alto queda marcado; si pasó de cero, en rojo hasta hacerle clic.
            let m = self.picos[i][0].max(self.picos[i][1]);
            self.altos[i] = self.altos[i].max(m);
            let luz = Rect::from_min_size(pos2(med.left(), y0), vec2(med.width(), 7.0));
            if self.altos[i] >= 1.0 {
                encendido(ui.painter(), luz, 2, ROJO);
            } else {
                hueco(ui.painter(), luz, 2, HUECO);
            }
            if ui
                .interact(luz.expand(2.0), plana.id.with("pico"), Sense::click())
                .on_hover_text(t("Se prende si la señal se pasa; clic para apagarla"))
                .clicked()
            {
                self.altos = [0.0; PISTAS_MEZCLA];
            }
            let riel = Rect::from_min_max(pos2(r.left() + 8.0, y0), pos2(r.right() - 28.0, y1));
            let f = tirador(ui, riel, &mut self.p.mezcla[i].volumen, 1.25, plana.id.with("vol"));
            if f.is_pointer_button_down_on() {
                self.pista = i;
            }
            if f.hovered() {
                self.ayuda("Arrastra: volumen · Mayús: fino · doble clic: como estaba · clic derecho en el nombre: renombrar");
            }
        }
        let db = 20.0 * self.p.mezcla[i].volumen.max(1e-4).log10();
        let s = if db < -70.0 { "-∞ dB".into() } else { format!("{db:+.1} dB") };
        ui.painter().text(pos2(r.center().x, r.bottom() - 24.0), Align2::CENTER_CENTER, s, letra(12.5), TENUE);
    }

    /// El panel derecho: las ranuras de la pista elegida, sus envíos y su ecualizador.
    fn ranuras(&mut self, ui: &mut Ui) {
        let i = self.pista;
        ui.horizontal(|ui| {
            ui.add(Label::new(RichText::new(&self.p.mezcla[i].nombre).size(17.0).color(ACENTO)).truncate());
        });
        let mut cambio = None;
        let abajo = if (1..PRIMER_ENVIO).contains(&i) { 218.0 } else { 148.0 };
        let alto = (ui.available_height() - abajo).max(60.0);
        ScrollArea::vertical().id_salt("ranuras").max_height(alto).min_scrolled_height(alto).auto_shrink(false).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            for k in 0..RANURAS.max(self.p.mezcla[i].efectos.len() + 1) {
                if let Some(c) = self.ranura(ui, i, k) {
                    cambio = Some(c);
                }
            }
        });
        if (1..PRIMER_ENVIO).contains(&i) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for k in 0..ENVIOS {
                    let n = self.p.mezcla[PRIMER_ENVIO + k].nombre.clone();
                    let r = perilla(ui, &n, &mut self.p.mezcla[i].envios[k], 0.0, 1.0, 0.0, AZUL, 30.0);
                    if r.hovered() {
                        self.pista_ayuda = t("Cuánto de esta pista va a ese envío (reverberación, eco…)").into();
                    }
                }
            });
        }
        self.eq_de_tira(ui, i);
        ui.horizontal(|ui| {
            let m = &mut self.p.mezcla[i];
            perilla(ui, t("Ancho"), &mut m.ancho, 0.0, 2.0, 1.0, ACENTO, 28.0).on_hover_text(t("0 es mono, 1 normal, 2 muy abierto"));
            if foco(ui, m.fase, "Ø", ACENTO).on_hover_text(t("Invierte la polaridad")).clicked() {
                m.fase = !m.fase;
            }
        });
        match cambio {
            Some(Cambio::Poner(k)) => {
                self.p.mezcla[i].efectos.push(EfectoPuesto::nuevo(&k));
                self.abrir_efecto(i, self.p.mezcla[i].efectos.len() - 1);
            }
            Some(Cambio::Plugin) => {
                let n = self.p.mezcla[i].efectos.len();
                self.elegir_plugin(Some(i));
                if self.p.mezcla[i].efectos.len() > n {
                    self.abrir_efecto(i, n);
                }
            }
            Some(Cambio::Quitar(k)) => {
                self.fx_abiertos.retain(|x| x.0 != i);
                if let Some(x) = self.p.mezcla[i].efectos.remove(k).plugin {
                    self.ventanas.remove(&x.uid);
                }
            }
            Some(Cambio::Mover(a, b)) => {
                self.fx_abiertos.retain(|x| x.0 != i);
                self.p.mezcla[i].efectos.swap(a, b);
            }
            Some(Cambio::Abrir(k)) => self.abrir_efecto(i, k),
            None => {}
        }
    }

    /// Los efectos para elegir, por grupo.
    fn menu_efectos(&self, ui: &mut Ui) -> Option<Cambio> {
        let mut c = None;
        for (g, v) in self.bib.grupos(Tipo::Efecto) {
            ui.menu_button(g, |ui| {
                for x in v {
                    let b = match self.logo_de(ui.ctx(), &x.clave) {
                        Some(t) => ui.add(Button::new((egui::Image::new((t.id(), vec2(16.0, 16.0))), x.ficha.nombre.ver()))),
                        None => ui.button(x.ficha.nombre.ver()),
                    };
                    if b.clicked() {
                        c = Some(Cambio::Poner(x.clave.clone()));
                        ui.close();
                    }
                }
            });
        }
        ui.separator();
        if ui.button(t("Plugin VST3 o CLAP…")).clicked() {
            c = Some(Cambio::Plugin);
            ui.close();
        }
        c
    }

    fn nombre_efecto(&self, e: &EfectoPuesto) -> String {
        match (&e.plugin, self.bib.ver(&e.efecto)) {
            (Some(x), _) => x.nombre.clone(),
            (_, Some(g)) => g.ficha.nombre.ver().to_string(),
            _ => e.efecto.clone(),
        }
    }

    fn ranura(&mut self, ui: &mut Ui, i: usize, k: usize) -> Option<Cambio> {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width() - 4.0, 28.0), Sense::hover());
        let id = ui.id().with(("ranura", i, k));
        let hay = k < self.p.mezcla[i].efectos.len();
        let b = ui.interact(r, id, Sense::click());
        let mut c = None;
        if !hay {
            hueco(ui.painter(), r, 4, Color32::from_rgb(0x15, 0x16, 0x19));
            let libre = k == self.p.mezcla[i].efectos.len();
            let s = if libre { t("+ Agregar efecto").to_string() } else { tf("Ranura {}", &[&(k + 1)]) };
            let col = if libre { mezcla(TENUE, ACENTO, if b.hovered() { 1.0 } else { 0.5 }) } else { Color32::from_gray(0x4c) };
            rotulo(ui.painter(), r.shrink2(vec2(10.0, 0.0)), &s, 14.0, col);
            Popup::menu(&b).show(|ui| {
                ui.set_min_width(150.0);
                if let Some(x) = self.menu_efectos(ui) {
                    c = Some(x);
                }
            });
            return c;
        }
        let abierta = self.fx_abiertos.contains(&(i, k));
        let nombre = self.nombre_efecto(&self.p.mezcla[i].efectos[k]);
        relieve(ui.painter(), r, 4, if abierta { CAJA2 } else { CAJA }, b.hovered(), b.is_pointer_button_down_on());
        let e = &mut self.p.mezcla[i].efectos[k];
        // Foco, nombre y cuánto efecto.
        let q = Rect::from_center_size(r.left_center() + vec2(13.0, 0.0), vec2(12.0, 12.0));
        let l = ui.interact(q.expand(5.0), id.with("foco"), Sense::click()).on_hover_text(t("Prender o apagar"));
        if e.activo {
            encendido(ui.painter(), q, 3, ACENTO);
        } else {
            hueco(ui.painter(), q, 3, HUECO);
        }
        if l.clicked() {
            e.activo = !e.activo;
        }
        let col = if !e.activo {
            TENUE
        } else if abierta {
            ACENTO
        } else {
            TEXTO
        };
        rotulo(ui.painter(), Rect::from_min_max(r.min + vec2(28.0, 0.0), r.max - vec2(36.0, 0.0)), &nombre, 14.5, col);
        let mut pk =
            ui.new_child(UiBuilder::new().max_rect(Rect::from_center_size(r.right_center() - vec2(17.0, 0.0), vec2(26.0, 26.0))).id_salt(("mez", k)));
        perilla(&mut pk, "", &mut e.mezcla, 0.0, 1.0, 1.0, ACENTO, 26.0).on_hover_text(t("Cuánto efecto"));
        b.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &nombre));
        if b.clicked() {
            c = Some(Cambio::Abrir(k));
        }
        if b.hovered() {
            self.pista_ayuda = t("Clic: abrir el efecto · foco: prender o apagar · perilla: cuánto efecto · clic derecho: mover o quitar").into();
        }
        let n = self.p.mezcla[i].efectos.len();
        b.context_menu(|ui| {
            if k > 0 && ui.button(t("Subir")).clicked() {
                c = Some(Cambio::Mover(k, k - 1));
                ui.close();
            }
            if k + 1 < n && ui.button(t("Bajar")).clicked() {
                c = Some(Cambio::Mover(k, k + 1));
                ui.close();
            }
            if ui.button(t("Quitar")).clicked() {
                c = Some(Cambio::Quitar(k));
                ui.close();
            }
        });
        c
    }

    /// Las tres bandas de la pista, con su curva.
    fn eq_de_tira(&mut self, ui: &mut Ui, i: usize) {
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width() - 4.0, 66.0), Sense::click_and_drag());
        let p = ui.painter_at(r.expand(1.0));
        hueco(&p, r, 5, HUECO);
        let eq = self.p.mezcla[i].eq;
        let k = eq_de_pista(&eq, 48000.0);
        let hz = |x: f32| 20.0 * ((x - r.left()) / r.width() * 1000f32.ln()).exp();
        let y = |db: f32| r.center().y - db / 18.0 * (r.height() / 2.0 - 4.0);
        p.hline(r.x_range(), r.center().y, Stroke::new(1.0, Color32::from_white_alpha(24)));
        let pts = (0..=r.width() as usize / 3).map(|j| r.left() + j as f32 * 3.0);
        let pts: Vec<Pos2> = pts.map(|x| pos2(x, y(k.iter().map(|k| respuesta(k, hz(x), 48000.0)).sum::<f32>().clamp(-18.0, 18.0)))).collect();
        p.add(Shape::line(pts, Stroke::new(1.8, ACENTO)));
        let tercio = r.width() / 3.0;
        let o = ui.input(|i| i.pointer.interact_pos()).unwrap_or_default();
        let en = ((o.x - r.left()) / tercio).clamp(0.0, 2.0) as usize;
        let dato = resp.id.with("banda");
        if resp.drag_started() {
            ui.data_mut(|d| d.insert_temp(dato, en));
        }
        let toma: usize = ui.data(|d| d.get_temp(dato)).unwrap_or(en);
        if resp.dragged() {
            let v = &mut self.p.mezcla[i].eq[toma];
            *v = (*v - resp.drag_delta().y * 0.4).clamp(-18.0, 18.0);
        }
        if resp.double_clicked() {
            self.p.mezcla[i].eq[en] = 0.0;
        }
        for (j, s) in ["Graves", "Medios", "Agudos"].into_iter().enumerate() {
            let x = r.left() + tercio * (j as f32 + 0.5);
            let el = resp.hovered() && en == j;
            p.circle_filled(pos2(x, y(eq[j])), if el { 6.0 } else { 4.5 }, if eq[j] == 0.0 { TENUE } else { ACENTO });
            p.text(pos2(x, r.bottom() - 2.0), Align2::CENTER_BOTTOM, t(s), letra(10.5), TENUE);
            if eq[j] != 0.0 || el {
                p.text(pos2(x, r.top() + 2.0), Align2::CENTER_TOP, format!("{:+.1}", eq[j]), letra(10.5), TEXTO);
            }
        }
        if resp.hovered() {
            self.ayuda("Ecualizador de la pista: arrastra arriba o abajo cada banda · doble clic: en cero");
        }
    }

    /// Abre la ventana de un efecto, o la trae al frente. Los plugins abren la suya.
    pub fn abrir_efecto(&mut self, i: usize, k: usize) {
        let Some(e) = self.p.mezcla[i].efectos.get(k) else { return };
        if let Some(uid) = e.plugin.as_ref().map(|x| x.uid) {
            let Some(p) = self.bib.rack.ver(uid).cloned() else { return };
            if let Some(v) = self.ventanas.get(&uid) {
                return v.al_frente();
            }
            if let Some(v) = Ventana::abrir(&p) {
                self.ventanas.insert(uid, v);
                return;
            }
        }
        self.fx_abiertos.retain(|x| *x != (i, k));
        self.fx_abiertos.push((i, k));
        self.fx_frente = Some((i, k));
    }

    /// Las ventanas de los efectos abiertos.
    pub(super) fn ver_efectos(&mut self, ui: &mut Ui, z: Rect) {
        self.fx_abiertos.retain(|(i, k)| self.p.mezcla.get(*i).is_some_and(|m| *k < m.efectos.len()));
        // Se escucha la pista del ecualizador o de la interfaz propia que esté más al frente.
        let mira = self
            .fx_abiertos
            .clone()
            .into_iter()
            .rev()
            .find(|(i, k)| {
                let e = self.p.mezcla[*i].efectos[*k].efecto.clone();
                self.lienzo(&e).is_some()
            })
            .map(|x| x.0)
            // Si no, la del instrumento que se está viendo.
            .or_else(|| {
                let c = self.p.canales.iter().find(|c| c.id == self.canal).map(|c| (c.instrumento.clone(), c.pista))?;
                (self.abierta(Vista::Canal) && self.lienzo(&c.0).is_some()).then_some(c.1)
            });
        self.estado.escucha.store(mira.map_or(u32::MAX, |i| i as u32), Relaxed);
        let mut cerrar = None;
        for (n, (i, k)) in self.fx_abiertos.clone().into_iter().enumerate() {
            let e = self.p.mezcla[i].efectos[k].clone();
            let nombre = format!("{} · {}", self.nombre_efecto(&e), self.p.mezcla[i].nombre);
            let logo = if e.plugin.is_none() { self.logo_de(ui.ctx(), &e.efecto) } else { None };
            let lienzo = self.lienzo(&e.efecto).map(|x| vec2((x.ancho + 24.0).max(460.0), x.alto + 82.0));
            let g = self.bib.ver(&e.efecto);
            let tam = match (&e.plugin, e.efecto.as_str(), g) {
                (Some(_), ..) => vec2(340.0, 420.0),
                _ if lienzo.is_some() => lienzo.unwrap_or_default(),
                (.., Some(g)) => {
                    // A la medida de sus perillas, en filas de cinco.
                    let c = g.ficha.controles.len().max(1);
                    let grafica = if super::grafica::Clase::de(&g.ficha.grupo.es).is_some() { 118.0 } else { 0.0 };
                    vec2((c.min(6) as f32 * 70.0 + 76.0).max(480.0), c.div_ceil(6) as f32 * 80.0 + 130.0 + grafica)
                }
                _ => vec2(300.0, 140.0),
            };
            let id = Id::new(("efecto", i, k, &e.efecto));
            let lugar = Rect::from_min_size(z.min + vec2(60.0 + 26.0 * (n % 8) as f32, 40.0 + 26.0 * (n % 8) as f32), tam);
            let w = Window::new(&nombre)
                .id(id)
                .title_bar(false)
                .frame(Frame::NONE)
                .constrain_to(z)
                .default_rect(lugar)
                .min_size(vec2(220.0, 120.0))
                .resizable(true);
            let r = w.show(ui.ctx(), |ui| {
                let (todo, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
                let c = titulo(ui, todo, &nombre, true, id, false).cerrar;
                // Su logo, en el lugar del tornillo.
                if let Some(t) = &logo {
                    let r = Rect::from_center_size(pos2(todo.left() + 13.0, todo.top() + TITULO / 2.0), Vec2::splat(20.0));
                    ui.painter().rect_filled(r.expand(3.0), 5.0, CAJA);
                    App::pinta_logo(ui.painter(), r, t);
                }
                let dentro = Rect::from_min_max(pos2(todo.left() + 10.0, todo.top() + TITULO + 6.0), todo.max - vec2(10.0, 8.0));
                let mut h = ui.new_child(UiBuilder::new().max_rect(dentro));
                h.set_clip_rect(dentro.expand(3.0).intersect(ui.clip_rect()));
                self.cuerpo_efecto(&mut h, i, k);
                c
            });
            if let Some(r) = r {
                if r.inner == Some(true) {
                    cerrar = Some((i, k));
                }
                if self.fx_frente == Some((i, k)) {
                    ui.ctx().move_to_top(r.response.layer_id);
                    self.fx_frente = None;
                }
            }
        }
        if let Some(x) = cerrar {
            self.fx_abiertos.retain(|y| *y != x);
        }
    }

    fn cuerpo_efecto(&mut self, ui: &mut Ui, i: usize, k: usize) {
        let onda = (self.estado.escucha.load(Relaxed) == i as u32 && self.tocando()).then(|| self.estado.onda());
        let sr = self.bib.sr;
        let mut momento = self.momento(ui, i);
        if let Some(g) = self.bib.ver(&self.p.mezcla[i].efectos[k].efecto)
            && let Some(r) = g.receta.clone()
        {
            if let Some(x) = self.bib.medidas.get(&(i, k)) {
                momento.medidas = r.nombres_medidas().zip(x.iter()).map(|(n, v)| (n.to_string(), f32::from_bits(v.load(Relaxed)))).collect();
            }
            if r.con_huella() {
                let e = self.p.mezcla[i].efectos[k].clone();
                momento.huella = self.bib.huella(&e).and_then(|m| m.impulso.as_ref().map(|h| h.picos.clone())).unwrap_or_default();
                momento.archivo = e.muestra.map_or(String::new(), |r| Path::new(&r).file_name().unwrap_or_default().to_string_lossy().into());
            }
        }
        let clave = self.p.mezcla[i].efectos[k].efecto.clone();
        self.lienzo(&clave);
        let ficha = self.bib.ver(&clave).map(|g| g.ficha.clone());
        let controladores: Vec<_> = self.controladores().into_iter().filter(|c| c.0 != (i, k)).collect();
        let pistas: Vec<String> = self.p.mezcla.iter().map(|m| m.nombre.clone()).collect();
        let e = &mut self.p.mezcla[i].efectos[k];
        ui.horizontal(|ui| {
            if foco(ui, e.activo, t(if e.activo { "Prendido" } else { "Apagado" }), ACENTO).clicked() {
                e.activo = !e.activo;
            }
            ui.add(Slider::new(&mut e.mezcla, 0.0..=1.0).show_value(false).text(t("Cuánto efecto")));
            if e.plugin.is_none() && ui.button(t("Reiniciar")).clicked() {
                e.controles.clear();
            }
            if e.plugin.is_none()
                && let Some(f) = ficha.as_ref().filter(|f| !f.presets.is_empty())
            {
                ui.menu_button(t("Preajustes"), |ui| {
                    for (nombre, v) in &f.presets {
                        if ui.button(nombre.ver()).clicked() {
                            e.controles = v.clone();
                            ui.close();
                        }
                    }
                });
            }
        });
        ui.add_space(4.0);
        if let Some(x) = &e.plugin {
            if let Some(p) = self.bib.rack.ver(x.uid) {
                ScrollArea::vertical().auto_shrink(false).show(ui, |ui| lado::controles_plugin(ui, p));
            }
            return;
        }
        let Some(g) = self.bib.ver(&e.efecto) else {
            ui.label(RichText::new(t("Falta este guion")).color(ROJO));
            return;
        };
        if let Some((_, Some(it))) = self.lienzos.get_mut(&clave) {
            let mut ent = lienzo::Entorno {
                activo: Some(&mut e.activo),
                datos: Some(e.datos.clone()),
                receta: g.receta.as_deref(),
                onda: onda.clone(),
                sr,
                pistas,
                controladores,
                vinculados: e.vinculos.keys().cloned().collect(),
                ..Default::default()
            };
            lienzo::pintar(ui, it, &g.ficha, &mut e.controles, momento, &mut ent);
            let (elegir, quitar, vincular, datos) = (ent.elegir, ent.quitar, ent.vincular.take(), ent.datos.take());
            if let Some(d) = datos {
                e.datos = d;
            }
            if quitar {
                e.muestra = None;
            }
            if elegir && let Some(r) = rfd::FileDialog::new().add_filter("Audio", crate::decodificar::EXTENSIONES).pick_file() {
                e.muestra = Some(r.to_string_lossy().into());
            }
            if let Some((c, de)) = vincular {
                let id = de.map(|d| self.id_de(d));
                let v = &mut self.p.mezcla[i].efectos[k].vinculos;
                match id {
                    Some(id) => _ = v.insert(c, id),
                    None => _ = v.remove(&c),
                }
            }
            return;
        }
        // El mismo panel de metal que los que traen su diseño, con un color por familia.
        let tinte = match g.ficha.grupo.es.as_str() {
            "Dinámica" => Color32::from_rgb(0x55, 0x40, 0x5e),
            "Tono" => Color32::from_rgb(0x3c, 0x55, 0x38),
            "Espacio" => Color32::from_rgb(0x2e, 0x46, 0x70),
            "Movimiento" => Color32::from_rgb(0x24, 0x5a, 0x58),
            "Carácter" => Color32::from_rgb(0x74, 0x40, 0x2e),
            _ => Color32::from_rgb(0x4c, 0x50, 0x58),
        };
        let panel = ui.available_rect_before_wrap();
        torno::acero(ui.painter(), panel, 9.0, tinte, 1.0);
        let mut ui = ui.new_child(UiBuilder::new().max_rect(panel.shrink2(vec2(18.0, 17.0))));
        let ui = &mut ui;
        if let (Some(c), Some(r)) = (super::grafica::Clase::de(&g.ficha.grupo.es), g.receta.as_deref()) {
            super::grafica::pintar(ui, c, r, &e.controles, sr, false, ui.id().with(("grafica", i, k, &clave)));
        }
        ui.add_space(7.0);
        let charola = ui.available_rect_before_wrap();
        torno::bandeja(ui.painter(), charola, 7.0, Color32::from_rgb(0x2a, 0x2c, 0x30), 1.0);
        let mut ui = ui.new_child(UiBuilder::new().max_rect(charola.shrink2(vec2(8.0, 6.0))));
        ScrollArea::vertical().auto_shrink(false).show(&mut ui, |ui| lado::controles(ui, &g.ficha, &mut e.controles, ACENTO));
    }
}
