//! Monitor de origen: ver un archivo aparte, marcar un tramo y meterlo a las pistas.
use super::estilo::*;
use super::*;
use crate::edicion::Carril;

pub struct Origen {
    pub ruta: PathBuf,
    pub dur: f64,
    pub tiene_video: bool,
    pub t: f64,
    pub entrada: Option<f64>,
    pub salida: Option<f64>,
    tocando: bool,
    p: Arc<Proyecto>,
    vista: VistaPrevia,
    tex: Option<TextureHandle>,
}

impl App {
    pub fn abrir_origen(&mut self, ruta: &Path) {
        let Ok(info) = medios::sondear(ruta) else {
            self.avisar(crate::idioma::t("No pude abrir ese archivo"), true);
            return;
        };
        let mut p = Proyecto::nuevo();
        if let Some((w, h, fps)) = info.video {
            (p.ancho, p.alto, p.fps) = (w, h, fps);
            let con = if es_imagen(ruta) { Contenido::Imagen } else { Contenido::Archivo };
            let c = p.nuevo_video(ruta.into(), con, 0.0, info.dur.max(5.0));
            p.video.push(c);
        }
        self.parar_origen();
        if info.audio && !self.almacen.contains_key(ruta) {
            self.leer_audio(ruta.into(), Destino2::Nada, "Leyendo el audio");
        }
        self.origen = Some(Origen {
            ruta: ruta.into(),
            dur: if es_imagen(ruta) { 5.0 } else { info.dur },
            tiene_video: info.video.is_some(),
            t: 0.0,
            entrada: None,
            salida: None,
            tocando: false,
            p: Arc::new(p),
            vista: VistaPrevia::default(),
            tex: None,
        });
        self.panel_izq = PanelIzq::Origen;
    }

    pub fn parar_origen(&mut self) {
        self.motor.lock().unwrap().origen = None;
        if let Some(o) = &mut self.origen {
            o.tocando = false;
        }
        self.archivos.parar_vista();
    }

    fn tocar_origen(&mut self) {
        let Some(o) = &mut self.origen else { return };
        if o.tocando {
            self.parar_origen();
            return;
        }
        if o.t >= o.dur - 0.05 {
            o.t = 0.0;
        }
        o.tocando = true;
        self.archivos.parar_vista();
        let a = self.almacen.get(&o.ruta).cloned();
        let mut m = self.motor.lock().unwrap();
        // Con sonido manda el audio; sin él, un audio vacío lleva la cuenta.
        m.tocando = false;
        m.origen = Some((a.unwrap_or_else(|| Arc::new(Audio::nuevo(vec![]))), (o.t * SR as f64) as i64));
    }

    pub fn ui_origen(&mut self, ui: &mut Ui) {
        if self.origen.is_none() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| ui.label(RichText::new(crate::idioma::t("Doble clic en un medio para verlo aquí")).color(TEXT_FAINT)));
            return;
        }
        let pos = self.motor.lock().unwrap().origen.as_ref().map(|o| o.1 as f64 / SR as f64);
        let o = self.origen.as_mut().unwrap();
        if o.tocando {
            o.t = pos.unwrap_or(o.t).min(o.dur);
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
        let todo = ui.available_rect_before_wrap();
        let img = Rect::from_min_max(todo.min, pos2(todo.right(), todo.bottom() - 92.0));
        ui.allocate_rect(todo, Sense::hover());
        ui.painter().rect_filled(img, 0, Color32::BLACK);
        if o.tiene_video {
            let h = 360.min(o.p.alto.max(2) as usize) / 2 * 2;
            let w = (h as f64 * o.p.ancho as f64 / o.p.alto.max(1) as f64 / 2.0).round() as usize * 2;
            let t = o.t.min((o.dur - 0.02).max(0.0));
            if let Some(q) = o.vista.actualizar(&o.p, 1, t, o.tocando, (w.max(2), h.max(2))) {
                let i = ColorImage::from_rgba_unmultiplied([q.ancho, q.alto], &q.rgba);
                match &mut o.tex {
                    Some(x) => x.set(i, TextureOptions::LINEAR),
                    None => o.tex = Some(ui.ctx().load_texture("origen", i, TextureOptions::LINEAR)),
                }
            }
            if o.vista.pendientes > 0 {
                ui.ctx().request_repaint_after(Duration::from_millis(12));
            }
            if let Some(x) = &o.tex {
                let k = (img.width() / w as f32).min(img.height() / h as f32);
                let d = Rect::from_center_size(img.center(), vec2(w as f32, h as f32) * k);
                ui.painter().image(x.id(), d, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
        } else if let Some(a) = self.almacen.get(&o.ruta) {
            linea::onda(&ui.painter_at(img), img.shrink(8.0), a, 0.0, o.dur, false, 1.0, DEEP);
        }
        // Barra de tiempo
        let barra = Rect::from_min_size(pos2(todo.left() + 8.0, img.bottom() + 6.0), vec2(todo.width() - 16.0, 18.0));
        let x_de = |t: f64| barra.left() + (t / o.dur.max(0.01)) as f32 * barra.width();
        ui.painter().rect_filled(barra, 4, WELL);
        let (a, b) = (o.entrada.unwrap_or(0.0), o.salida.unwrap_or(o.dur));
        ui.painter().rect_filled(Rect::from_min_max(pos2(x_de(a), barra.top()), pos2(x_de(b), barra.bottom())), 4, MAIN.gamma_multiply(0.35));
        ui.painter().vline(x_de(o.t), barra.y_range(), Stroke::new(2.0, MAIN));
        let rb = ui.interact(barra, Id::new("origen-barra"), Sense::click_and_drag());
        let mut ir = None;
        if let (true, Some(m)) = (rb.dragged() || rb.clicked(), rb.interact_pointer_pos()) {
            ir = Some(((m.x - barra.left()) / barra.width()).clamp(0.0, 1.0) as f64 * o.dur);
        }
        if let Some(t) = ir {
            o.t = t;
            if let Some(x) = &mut self.motor.lock().unwrap().origen {
                x.1 = (t * SR as f64) as i64;
            }
        }
        let pie = Rect::from_min_max(pos2(todo.left() + 8.0, barra.bottom() + 4.0), todo.max - vec2(8.0, 2.0));
        let mut u = ui.new_child(UiBuilder::new().max_rect(pie));
        let (mut tocar, mut meter) = (false, None);
        u.horizontal(|ui| {
            ui.label(RichText::new(tiempo(o.t)).color(MAIN));
            if boton_color(ui, &tabler::PLAYER_PLAY, o.tocando, "Reproducir el origen", GLYPH, 24.0).clicked() {
                tocar = true;
            }
            if ui.button(crate::idioma::t("Entrada")).on_hover_text(crate::idioma::t("Marca el principio del tramo")).clicked() {
                o.entrada = Some(o.t);
            }
            if ui.button(crate::idioma::t("Salida")).on_hover_text(crate::idioma::t("Marca el final del tramo")).clicked() {
                o.salida = Some(o.t);
            }
            if ui.button(crate::idioma::t("Quitar marcas")).clicked() {
                (o.entrada, o.salida) = (None, None);
            }
            ui.label(RichText::new(crate::idioma::tf("tramo {}", &[&(tiempo((b - a).max(0.0)))])).color(TEXT_DIM).size(13.0));
        });
        u.horizontal(|ui| {
            if ui.button(RichText::new(crate::idioma::t("Insertar")).color(MAIN)).on_hover_text(crate::idioma::t("En el cursor, empujando lo que sigue")).clicked() {
                meter = Some(true);
            }
            if ui.button(RichText::new(crate::idioma::t("Sobrescribir")).color(MAIN)).on_hover_text(crate::idioma::t("En el cursor, tapando lo que haya")).clicked() {
                meter = Some(false);
            }
        });
        let (ruta, tramo) = (o.ruta.clone(), (a.min(b), a.max(b)));
        if tocar {
            self.tocar_origen();
        }
        if let Some(insertar) = meter {
            let t = self.ahora();
            self.poner_medio(&ruta, t, 0, Some(tramo), insertar);
        }
    }

    /// Pone un archivo en `t`: imagen en la capa y sonido en su pista, vinculados.
    pub fn poner_medio(&mut self, ruta: &Path, t: f64, capa: usize, tramo: Option<(f64, f64)>, insertar: bool) {
        let Ok(info) = medios::sondear(ruta) else {
            self.avisar(crate::idioma::t("No pude leer ese archivo"), true);
            return;
        };
        self.poner_medio_info(ruta, &info, t, Some(capa), tramo, insertar);
    }

    /// `poner_medio` de un archivo ya sondeado. Sin `capa`, solo su sonido.
    pub fn poner_medio_info(&mut self, ruta: &Path, info: &Info, t: f64, capa: Option<usize>, tramo: Option<(f64, f64)>, insertar: bool) {
        let imagen = es_imagen(ruta);
        let total = if imagen { 5.0 } else { info.dur };
        let (a, b) = tramo.unwrap_or((0.0, total));
        let dur = (b - a).max(0.05);
        if insertar {
            self.p.dividir_todo(t);
            self.p.empujar(t, dur, &[]);
        }
        let mut ids = vec![];
        if let (Some((w, h, fps)), Some(capa)) = (info.video, capa) {
            if self.p.video.is_empty() && !self.p.fijo {
                (self.p.ancho, self.p.alto, self.p.fps) = (w, h, if imagen { 30.0 } else { fps });
            }
            let con = if imagen { Contenido::Imagen } else { Contenido::Archivo };
            let mut c = self.p.nuevo_video(ruta.into(), con, t, dur);
            (c.ancho, c.alto, c.fps, c.capa, c.desde, c.total) = (w, h, fps, capa.min(self.p.capas.len() - 1), if imagen { 0.0 } else { a }, total);
            self.p.vaciar(Carril::Video(c.capa), t, t + dur, &[]);
            ids.push(c.id);
            self.clip = Some(c.id);
            self.p.video.push(c);
        }
        if info.audio && !imagen {
            // A la pista de audio elegida si es normal; si no, a la primera que sirva o a una nueva.
            let sirve = |p: &Pista| p.instrumento.is_none() && !p.bloqueada && !p.armada;
            let elegida = match self.canal {
                Canal::Pista(id) => self.p.pistas.iter().find(|p| p.id == id && sirve(p)).map(|p| p.id),
                _ => None,
            };
            let pista = elegida.or_else(|| self.p.pistas.iter().find(|p| sirve(p)).map(|p| p.id));
            let pista = pista.unwrap_or_else(|| self.p.nueva_pista("Audio"));
            self.p.vaciar(Carril::Audio(pista), t, t + dur, &[]);
            let id = self.p.id();
            let clip = Clip {
                id,
                ruta: ruta.into(),
                nombre: nombre_de(ruta),
                inicio: t,
                desde: a,
                dur,
                total,
                ganancia_db: 0.0,
                fade_in: 0.0,
                fade_out: 0.0,
                invertido: false,
                grupo: None,
                velocidad: 1.0, bus: None, color: None
            };
            self.p.pista(pista).unwrap().clips.push(clip);
            ids.push(id);
            if !self.almacen.contains_key(ruta) {
                self.leer_audio(ruta.into(), Destino2::Nada, "Leyendo el audio");
            }
        }
        if ids.len() > 1 {
            self.p.vincular(&ids);
        }
    }
}
