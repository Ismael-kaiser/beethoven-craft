//! Biblioteca: medios, efectos, transiciones y títulos. Todo se arrastra a las pistas o se pone con doble clic.
use super::estilo::*;
use super::*;
use crate::efectos::TipoFx;
use crate::nombres;
use crate::titulos;

/// Lo que se puede soltar en la línea de tiempo.
#[derive(Clone, Debug)]
pub enum Soltar {
    FxVideo(&'static str),
    FxAudio(TipoFx),
    Transicion(&'static str),
    Titulo(usize),
    Medio(PathBuf),
    Estilo(usize),
    Filtro(usize),
}

#[derive(Clone, Copy, PartialEq)]
pub enum Seccion {
    Medios,
    Estilos,
    Filtros,
    FxVideo,
    Transiciones,
    Titulos,
    FxAudio,
    Grabar,
}

pub struct Estado {
    pub seccion: Seccion,
    buscar: String,
    /// Los efectos del catálogo van a un clip vacío para todo el video.
    pub a_todo: bool,
}

impl Default for Estado {
    fn default() -> Self {
        Estado { seccion: Seccion::Medios, buscar: String::new(), a_todo: false }
    }
}

fn contiene(t: &str, q: &str) -> bool {
    let llano = |s: &str| -> String {
        s.to_lowercase()
            .chars()
            .map(|c| match c {
                'á' => 'a',
                'é' => 'e',
                'í' => 'i',
                'ó' => 'o',
                'ú' => 'u',
                c => c,
            })
            .collect()
    };
    llano(t).contains(&llano(q))
}

/// Renglón arrastrable. Devuelve verdadero con doble clic.
fn renglon(ui: &mut Ui, icon: &tabler::TablerIcon, t: &str, s: Soltar) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click_and_drag());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, t));
    if resp.hovered() || resp.dragged() {
        ui.painter().rect_filled(r, 4, BUTTON);
    }
    icono(ui.painter(), icon, Rect::from_center_size(pos2(r.left() + 12.0, r.center().y), Vec2::splat(14.0)), MAIN);
    ui.painter_at(r).text(pos2(r.left() + 26.0, r.center().y), Align2::LEFT_CENTER, t, fuente(14.5), TEXT);
    resp.dnd_set_drag_payload(s);
    if resp.dragged() {
        // Etiqueta junto al ratón mientras se arrastra.
        if let Some(p) = ui.ctx().pointer_interact_pos() {
            let capa = ui.ctx().layer_painter(LayerId::new(Order::Tooltip, Id::new("arrastrando")));
            let g = capa.layout_no_wrap(t.into(), fuente(14.0), COLORED_DARK);
            let caja = Rect::from_min_size(p + vec2(12.0, 6.0), g.size() + vec2(14.0, 8.0));
            capa.rect_filled(caja, 6, MAIN);
            capa.galley(caja.min + vec2(7.0, 4.0), g, COLORED_DARK);
        }
    }
    resp.on_hover_text(crate::idioma::t("Arrástralo a un clip o pista, o doble clic para ponerlo en lo elegido")).double_clicked()
}

impl App {
    /// `solo_fx`: en la ventana de Efectos no salen medios ni grabación.
    pub fn biblioteca(&mut self, ui: &mut Ui, solo_fx: bool) {
        use Seccion::*;
        let todas = [
            (Medios, "Medios"),
            (Estilos, "Estilos"),
            (Filtros, "Filtros"),
            (FxVideo, "Efectos de video"),
            (Transiciones, "Transiciones"),
            (Titulos, "Títulos"),
            (FxAudio, "Efectos de audio"),
            (Grabar, "Grabar"),
        ];
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for (s, n) in todas.into_iter().filter(|(s, _)| !solo_fx || !matches!(s, Medios | Grabar)) {
                if pastilla(ui, n, self.lib.seccion == s).clicked() {
                    self.lib.seccion = s;
                }
            }
        });
        ui.add_space(4.0);
        if solo_fx && matches!(self.lib.seccion, Medios | Grabar) {
            self.lib.seccion = FxVideo;
        }
        let s = self.lib.seccion;
        if matches!(s, FxVideo | Transiciones | FxAudio) {
            ui.add(TextEdit::singleline(&mut self.lib.buscar).hint_text(crate::idioma::t("Buscar…")).desired_width(f32::INFINITY));
        }
        let q = self.lib.buscar.clone();
        let mut poner: Option<Soltar> = None;
        ScrollArea::vertical().id_salt(("biblioteca", s as u8)).auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            match s {
                Medios => self.medios_ui(ui, &mut poner),
                Grabar => self.opciones_grabar(ui),
                Filtros => {
                    let mut g = "";
                    for (i, e) in crate::filtros::FILTROS.iter().enumerate().filter(|e| contiene(e.1.nombre, &q) || contiene(e.1.grupo, &q)) {
                        if g != e.grupo {
                            g = e.grupo;
                            ui.label(RichText::new(crate::idioma::t(g)).color(MAIN).size(13.0));
                        }
                        if renglon(ui, &tabler::STAR, crate::idioma::t(e.nombre), Soltar::Filtro(i)) {
                            poner = Some(Soltar::Filtro(i));
                        }
                    }
                }
                Estilos => {
                    for (i, e) in crate::estilos::ESTILOS.iter().enumerate() {
                        if renglon(ui, &tabler::STAR, e.nombre, Soltar::Estilo(i)) {
                            poner = Some(Soltar::Estilo(i));
                        }
                    }
                }
                Titulos => {
                    for i in 0..titulos::PLANTILLAS {
                        if renglon(ui, &tabler::FILE_TYPOGRAPHY, titulos::plantilla(i).0, Soltar::Titulo(i)) {
                            poner = Some(Soltar::Titulo(i));
                        }
                    }
                    ui.add_space(8.0);
                    if ui.button(crate::idioma::t("Fondo de color")).clicked() {
                        let (t, capa) = (self.ahora(), self.capa_libre(self.ahora(), 5.0));
                        let mut c = self.p.nuevo_video("Color".into(), Contenido::Color([0.45, 0.12, 0.16, 1.0]), t, 5.0);
                        c.capa = capa;
                        self.clip = Some(c.id);
                        self.p.video.push(c);
                    }
                }
                FxAudio => {
                    for g in TipoFx::GRUPOS {
                        let v: Vec<TipoFx> = TipoFx::TODOS.into_iter().filter(|t| t.grupo() == g && (contiene(t.nombre(), &q) || contiene(t.ayuda(), &q))).collect();
                        if !v.is_empty() {
                            ui.label(RichText::new(crate::idioma::t(&g)).color(MAIN).size(13.0));
                        }
                        for t in v {
                            if renglon(ui, &tabler::PLUG, t.nombre(), Soltar::FxAudio(t)) {
                                poner = Some(Soltar::FxAudio(t));
                            }
                        }
                    }
                }
                FxVideo | Transiciones => {
                    let lista = if s == FxVideo { nombres::EFECTOS } else { nombres::TRANSICIONES };
                    let ic = if s == FxVideo { &tabler::FLASK_2 } else { &tabler::BLEND_MODE };
                    let hacer = |id| if s == FxVideo { Soltar::FxVideo(id) } else { Soltar::Transicion(id) };
                    let mut i = 0;
                    while i < lista.len() {
                        let cat = lista[i].0;
                        let j = i + lista[i..].iter().take_while(|e| e.0 == cat).count();
                        let vis: Vec<_> = lista[i..j].iter().filter(|e| q.is_empty() || contiene(e.1, &q) || contiene(cat, &q)).collect();
                        i = j;
                        if vis.is_empty() {
                            continue;
                        }
                        CollapsingHeader::new(crate::idioma::tf("{} ({})", &[&cat, &(vis.len())]))
                            .id_salt((s as u8, cat))
                            .open((!q.is_empty()).then_some(true))
                            .show(ui, |ui| {
                                for e in vis {
                                    if renglon(ui, ic, e.1, hacer(e.2)) {
                                        poner = Some(hacer(e.2));
                                    }
                                }
                            });
                    }
                }
            }
        });
        if let Some(s) = poner {
            self.poner_en_elegido(s);
        }
    }

    fn medios_ui(&mut self, ui: &mut Ui, poner: &mut Option<Soltar>) {
        let ancho = ui.available_width();
        if ui.add_sized([ancho, 28.0], Button::new(crate::idioma::t("Importar video, audio o imagen"))).on_hover_text(crate::idioma::t("También puedes arrastrarlos a la ventana (⌘I)")).clicked() {
            self.menu_importar();
        }
        ui.add_space(4.0);
        let mut rutas: Vec<PathBuf> = self.p.video.iter().filter(|c| !c.ruta.as_os_str().is_empty() && c.contenido != Contenido::Color([0.0; 4])).map(|c| c.ruta.clone()).collect();
        rutas.retain(|r| r.is_absolute());
        rutas.extend(self.p.pistas.iter().flat_map(|p| &p.clips).map(|c| c.ruta.clone()));
        rutas.sort();
        rutas.dedup();
        let mut abrir = None;
        for r in rutas {
            let video = self.p.video.iter().any(|c| c.ruta == r);
            let ic = if video { &tabler::LAYOUT_ROWS } else { &tabler::FILE_MUSIC };
            if renglon(ui, ic, &nombre_de(&r), Soltar::Medio(r.clone())) {
                abrir = Some(r);
            }
        }
        if let Some(r) = abrir {
            self.abrir_origen(&r);
        }
        let _ = poner;
        ui.add_space(6.0);
        if ui.add_sized([ancho, 28.0], Button::new(crate::idioma::t("Clip vacío (efectos para todo)"))).on_hover_text(crate::idioma::t(linea::AYUDA_VACIO)).clicked() {
            let (t, dur) = self.hueco_vacio();
            self.clip_vacio(t, dur);
        }
        if ui.add_sized([ancho, 28.0], Button::new(crate::idioma::t("Importar subtítulos SRT"))).clicked() {
            self.importar_srt();
        }
    }

    fn opciones_grabar(&mut self, ui: &mut Ui) {
        let ancho = ui.available_width();
        let b = |ui: &mut Ui, t: &str, ayuda: &str| ui.add_sized([ancho, 28.0], Button::new(t)).on_hover_text(ayuda).clicked();
        let antes = self.mic.clone();
        let actual = self.mic.clone().unwrap_or(crate::idioma::t("Micrófono del sistema").into());
        let cb = ComboBox::from_id_salt("mic").selected_text(actual).width(ancho - 8.0).show_ui(ui, |ui| {
            ui.selectable_value(&mut self.mic, None, "Micrófono del sistema");
            for m in &self.mics {
                ui.selectable_value(&mut self.mic, Some(m.clone()), m);
            }
        });
        if cb.response.clicked() && !self.sin_dispositivo {
            self.mics = audio::entradas();
        }
        if self.mic != antes && self.monitor.take().is_some() {
            self.encender_monitor();
        }
        ui.add_space(4.0);
        if b(ui, "+ Pista de grabación", "Con ecualizador y compresor de voz") {
            let id = self.p.nueva_pista_grabacion();
            self.canal = Canal::Pista(id);
        }
        let t = if self.grab.is_some() { "Detener grabación" } else { "Grabar en la pista armada" };
        if b(ui, t, "Tecla R. Usa audífonos para que no se cuele la música.") {
            self.grabar();
        }
        if let Some(g) = &self.grab {
            let x = ((a_db(g.nivel) + 54.0) / 54.0).clamp(0.0, 1.0);
            let col = if g.nivel > 0.98 { METER_CLIP } else { MAIN };
            ui.add(ProgressBar::new(x).desired_width(ancho).desired_height(8.0).fill(col));
            if !g.arrancada {
                ui.colored_label(RECORD, crate::idioma::t("Abriendo micrófono…"));
            }
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if led(ui, self.escuchar, "Escucharme al grabar").clicked() {
                self.escuchar = !self.escuchar;
            }
            ui.label(crate::idioma::t("Escucharme al grabar")).on_hover_text(crate::idioma::t("Al grabar te oyes en vivo con los efectos de la pista. Los efectos no se graban: puedes cambiarlos después."));
        });
        let on = self.monitor.is_some();
        ui.horizontal(|ui| {
            if led(ui, on, "Escucharme ahora").clicked() {
                self.alternar_monitor();
            }
            ui.label(RichText::new(crate::idioma::t("Escucharme ahora, sin grabar")).color(if on { MAIN } else { TEXT }));
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(crate::idioma::t("Ajuste de atraso")).color(TEXT_DIM));
            ui.add(DragValue::new(&mut self.p.ajuste_ms).range(-500.0..=500.0).suffix(" ms").speed(1.0))
                .on_hover_text(crate::idioma::t("Si tu voz queda atrasada, súbelo; si queda adelantada, bájalo. Vale para las tomas nuevas."));
        });
    }

    /// Primera capa sin clips en ese tramo, arriba de la de abajo; si no hay, crea una.
    /// Dónde va un clip vacío nuevo: el tramo marcado, o todo el video.
    pub fn hueco_vacio(&self) -> (f64, f64) {
        match self.p.tramo {
            (Some(a), Some(b)) if b > a => (a, b - a),
            _ => (0.0, self.p.fin().max(5.0)),
        }
    }

    /// Clip sin imagen encima de todo: sus efectos caen sobre lo que está debajo, textos incluidos.
    pub fn clip_vacio(&mut self, t: f64, dur: f64) -> u64 {
        // La pista más alta con algo en ese tramo; el clip va arriba de ella.
        let encima = self.p.video.iter().filter(|c| c.inicio < t + dur && c.fin() > t).map(|c| c.capa + 1).max().unwrap_or(1).max(1);
        while self.p.capas.len() <= encima {
            let n = self.p.capas.len() + 1;
            self.p.capas.push(Capa { nombre: format!("V{n}"), visible: true, bloqueada: false });
        }
        let mut c = self.p.nuevo_video("Ajuste".into(), Contenido::Ajuste, t, dur);
        c.capa = encima;
        let id = c.id;
        self.p.video.push(c);
        self.clip = Some(id);
        self.panel_izq = PanelIzq::Controles;
        id
    }

    pub fn capa_libre(&mut self, t: f64, dur: f64) -> usize {
        let encima = |c: &&ClipVideo| c.inicio < t + dur && c.fin() > t;
        // Debajo de los clips vacíos, para que sus efectos también le toquen a lo nuevo.
        let tope = self.p.video.iter().filter(encima).filter(|c| c.contenido == Contenido::Ajuste).map(|c| c.capa).min().unwrap_or(usize::MAX);
        let libre = (1..self.p.capas.len().min(tope)).find(|&i| !self.p.video.iter().filter(encima).any(|c| c.capa == i));
        libre.unwrap_or_else(|| {
            let n = self.p.capas.len();
            let nueva = Capa { nombre: format!("V{}", n + 1), visible: true, bloqueada: false };
            if tope >= n {
                self.p.capas.push(nueva);
                return n;
            }
            let i = tope.max(1);
            self.p.capas.insert(i, nueva);
            self.p.video.iter_mut().filter(|c| c.capa >= i).for_each(|c| c.capa += 1);
            i
        })
    }

    pub fn poner_titulo(&mut self, i: usize, t: f64, capa: Option<usize>) {
        let (_, x, y) = titulos::plantilla(i);
        let capa = capa.unwrap_or_else(|| self.capa_libre(t, 5.0));
        let mut c = self.p.nuevo_video("Título".into(), Contenido::Titulo(x), t, 5.0);
        c.capa = capa;
        c.mov.y = Prop::num(y);
        self.clip = Some(c.id);
        self.p.video.push(c);
    }

    pub fn agregar_fx_video(&mut self, clip: u64, tipo: &str) {
        let id = self.p.id();
        let bpm = self.p.bpm as f64;
        if let (Some(mut f), Some(c)) = (crate::composicion::fx_nuevo(id, tipo), self.p.clip_video(clip)) {
            crate::vpropios::a_tempo(&mut f, bpm, c.inicio);
            c.fx.push(f);
            self.clip = Some(clip);
        }
    }

    pub fn poner_transicion(&mut self, clip: u64, tipo: &str, al_final: bool) {
        let tr = crate::composicion::transicion_nueva(tipo);
        let Some((capa, fin)) = self.p.clip_video(clip).map(|c| (c.capa, c.fin())) else { return };
        // Al final de un clip pegado a otro, la transición es del que sigue.
        let sigue = self.p.video.iter().find(|c| c.capa == capa && (c.inicio - fin).abs() < 0.05).map(|c| c.id);
        let (id, salida) = match (al_final, sigue) {
            (true, Some(s)) => (s, false),
            (true, None) => (clip, true),
            (false, _) => (clip, false),
        };
        let c = self.p.clip_video(id).unwrap();
        let tr = Transicion { dur: tr.dur.min(c.dur / 2.0), ..tr };
        if salida {
            c.salida = Some(tr);
        } else {
            c.entrada = Some(tr);
        }
        self.clip = Some(id);
    }

    /// Un filtro es una «Mezcla de color»; poner otro reemplaza al anterior.
    pub fn poner_filtro(&mut self, clip: Option<u64>, i: usize) {
        let id = self.p.id();
        let Some(c) = clip.and_then(|x| self.p.clip_video(x)) else {
            self.avisar(crate::idioma::t("Pon el cursor sobre un clip de video o elígelo"), false);
            return;
        };
        let Some(f) = crate::filtros::efecto(id, i) else { return };
        match c.fx.iter_mut().find(|x| x.tipo == "bc:color") {
            Some(x) => *x = f,
            None => c.fx.push(f),
        }
        self.clip = clip;
        self.panel_izq = PanelIzq::Controles;
        let n = crate::idioma::t(crate::filtros::FILTROS[i].nombre);
        self.avisar(crate::idioma::tf("Filtro «{}» puesto", &[&n]), false);
    }

    pub fn poner_estilo(&mut self, clip: Option<u64>, i: usize) {
        match clip {
            Some(id) => {
                let n = crate::estilos::aplicar(&mut self.p, id, i);
                self.clip = Some(id);
                self.panel_izq = PanelIzq::Controles;
                self.avisar(crate::idioma::tf("Estilo «{}» puesto ({} cambios)", &[&(crate::estilos::ESTILOS[i].nombre), &n]), false);
            }
            None => self.avisar(crate::idioma::t("Pon el cursor sobre un clip de video o elígelo"), false),
        }
    }

    /// Hace que la imagen lata con el sonido de su pista.
    pub fn latir(&mut self, id: u64) {
        let Some(c) = self.p.clip_video(id).cloned() else { return };
        // El sonido vinculado; si no hay, el primer clip de audio que suene encima.
        let v: Vec<u64> = self.p.vinculados(id).iter().map(|x| x.id).collect();
        let audio = self
            .p
            .pistas
            .iter()
            .flat_map(|p| &p.clips)
            .find(|a| v.contains(&a.id))
            .or_else(|| self.p.pistas.iter().flat_map(|p| &p.clips).find(|a| a.inicio < c.fin() && a.fin() > c.inicio))
            .cloned();
        let Some((a, s)) = audio.and_then(|a| Some((a.clone(), self.almacen.get(&a.ruta)?.clone()))) else {
            self.avisar(crate::idioma::t("No hay sonido sobre ese clip"), false);
            return;
        };
        let de = a.desde + (c.inicio - a.inicio) * a.velocidad;
        let g: Vec<(f64, f64)> = crate::estilos::golpes(&s, de.max(0.0), de + c.dur * a.velocidad).into_iter().map(|(t, f)| (t / a.velocidad, f)).collect();
        let n = crate::estilos::latir(self.p.clip_video(id).unwrap(), &g, 0.09);
        self.avisar(crate::idioma::tf("La imagen late en {} golpes de la música", &[&n]), false);
    }

    /// Catálogo de la pestaña de efectos: una lista; al pasar el mouse se ve la muestra.
    pub fn catalogo(&mut self, ui: &mut Ui) {
        use Seccion::*;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (s, n) in [(Estilos, "Estilos listos"), (Filtros, "Filtros"), (FxVideo, "Video"), (Transiciones, "Transiciones"), (FxAudio, "Audio"), (Titulos, "Títulos")] {
                if pestana_chica(ui, n, self.lib.seccion == s).clicked() {
                    self.lib.seccion = s;
                }
            }
        });
        if matches!(self.lib.seccion, Medios | Grabar) {
            self.lib.seccion = Estilos;
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.lib.buscar).hint_text(crate::idioma::t("Buscar efecto…")).margin(Margin::symmetric(8, 4)).desired_width(ui.available_width() - 26.0 - ui.spacing().item_spacing.x - 2.0));
            if boton_color(ui, &super::tabler::ROTATE_CLOCKWISE_2, false, "Muestras con este cuadro: rehace las miniaturas con lo que se está viendo", GLYPH, 26.0).clicked() {
                self.pedir_muestras();
            }
        });
        if matches!(self.lib.seccion, Estilos | Filtros | FxVideo) {
            ui.checkbox(&mut self.lib.a_todo, crate::idioma::t("Aplicar a todo el video")).on_hover_text(crate::idioma::t("Pone el efecto en un clip vacío encima de todo: afecta videos, imágenes y textos"));
        }
        ui.add_space(2.0);
        let q = self.lib.buscar.clone();
        let s = self.lib.seccion;
        // (grupo, nombre, ayuda, clave de la muestra, qué pone)
        let todo: Vec<(&str, &str, &str, String, Soltar)> = match s {
            Estilos => crate::estilos::ESTILOS.iter().enumerate().map(|(i, e)| ("Estilos listos", e.nombre, e.ayuda, format!("estilo:{i}"), Soltar::Estilo(i))).collect(),
            Filtros => crate::filtros::FILTROS.iter().enumerate().map(|(i, e)| (e.grupo, e.nombre, "", format!("filtro:{i}"), Soltar::Filtro(i))).collect(),
            Transiciones => nombres::TRANSICIONES.iter().map(|e| (e.0, e.1, "", format!("tr:{}", e.2), Soltar::Transicion(e.2))).collect(),
            FxAudio => TipoFx::GRUPOS.iter().flat_map(|g| TipoFx::TODOS.into_iter().filter(move |t| t.grupo() == *g).map(move |t| (*g, t.nombre(), t.ayuda(), String::new(), Soltar::FxAudio(t)))).collect(),
            Titulos => (0..titulos::PLANTILLAS).map(|i| ("Títulos", titulos::plantilla(i).0, "", String::new(), Soltar::Titulo(i))).collect(),
            _ => nombres::EFECTOS.iter().map(|e| (e.0, e.1, "", e.2.to_string(), Soltar::FxVideo(e.2))).collect(),
        };
        let todo: Vec<_> = todo.into_iter().filter(|e| q.is_empty() || contiene(e.1, &q) || contiene(e.0, &q) || contiene(e.2, &q)).collect();
        let mut poner = None;
        ScrollArea::vertical().id_salt(("catalogo", s as u8)).auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            let ancho = ui.available_width();
            let mut grupo = "";
            for e in &todo {
                if e.0 != grupo {
                    grupo = e.0;
                    ui.add_space(8.0);
                    ui.label(RichText::new(crate::idioma::t(grupo).to_uppercase()).size(11.5).color(TEXT_FAINT));
                    ui.add_space(2.0);
                }
                if fila(ui, ancho, e.1, e.2, self.muestras.get(&e.3), e.4.clone()) {
                    poner = Some(e.4.clone());
                }
            }
            if todo.is_empty() {
                ui.label(RichText::new(crate::idioma::t("Nada con ese nombre")).color(TEXT_FAINT));
            }
        });
        if let Some(s) = poner {
            if self.lib.a_todo && matches!(s, Soltar::FxVideo(_) | Soltar::Estilo(_) | Soltar::Filtro(_)) {
                // El mismo clip vacío recibe los siguientes.
                let fin = self.p.fin();
                let ya = self.clip.filter(|id| self.p.video.iter().any(|c| c.id == *id && c.contenido == Contenido::Ajuste)).or_else(|| {
                    self.p.video.iter().filter(|c| c.contenido == Contenido::Ajuste && c.inicio < 0.05 && c.fin() > fin - 0.05).max_by_key(|c| c.capa).map(|c| c.id)
                });
                let id = match ya {
                    Some(id) => id,
                    None => self.clip_vacio(0.0, fin.max(5.0)),
                };
                match s {
                    Soltar::FxVideo(f) => self.agregar_fx_video(id, f),
                    Soltar::Estilo(i) => self.poner_estilo(Some(id), i),
                    Soltar::Filtro(i) => self.poner_filtro(Some(id), i),
                    _ => {}
                }
                return;
            }
            self.poner_en_elegido(s);
        }
    }

    fn poner_en_elegido(&mut self, s: Soltar) {
        let video = self.clip.filter(|id| self.p.video.iter().any(|c| c.id == *id));
        let t = self.ahora();
        match (s, video) {
            (Soltar::FxVideo(f), Some(id)) => self.agregar_fx_video(id, f),
            (Soltar::Transicion(x), Some(id)) => self.poner_transicion(id, x, false),
            // Sin nada elegido, al clip que se ve en el cursor.
            (Soltar::FxVideo(f), None) => match (0..self.p.capas.len()).rev().find_map(|i| self.p.video_en(i, t)).map(|c| c.id) {
                Some(id) => self.agregar_fx_video(id, f),
                None => self.avisar(crate::idioma::t("Pon el cursor sobre un clip de video o elígelo"), false),
            },
            (Soltar::Transicion(_), None) => self.avisar(crate::idioma::t("Elige primero un clip de video"), false),
            (Soltar::FxAudio(f), _) => {
                let x = self.p.nuevo_fx(f);
                // Con un clip de video elegido, a la pista de su sonido vinculado.
                if let Some(id) = video {
                    if let Some(crate::edicion::Carril::Audio(p)) = self.p.vinculados(id).iter().map(|x| x.carril).find(|c| matches!(c, crate::edicion::Carril::Audio(_))) {
                        self.canal = Canal::Pista(p);
                    }
                }
                let canal = self.canal;
                self.clip = None;
                self.fx_de(canal).push(x);
                self.panel_izq = PanelIzq::Controles;
            }
            (Soltar::Titulo(i), _) => self.poner_titulo(i, t, None),
            (Soltar::Medio(r), _) => self.importar(r),
            (Soltar::Filtro(i), _) => {
                let id = video.or_else(|| (0..self.p.capas.len()).rev().find_map(|i| self.p.video_en(i, t)).map(|c| c.id));
                self.poner_filtro(id, i);
            }
            (Soltar::Estilo(i), _) => {
                let id = video.or_else(|| (0..self.p.capas.len()).rev().find_map(|i| self.p.video_en(i, t)).map(|c| c.id));
                self.poner_estilo(id, i);
            }
        }
    }
}

/// Fila del catálogo: miniatura chica, nombre y ayuda. Al pasar el mouse se ve la muestra en grande. Verdadero con doble clic.
fn fila(ui: &mut Ui, ancho: f32, n: &str, ayuda: &str, tex: Option<&TextureHandle>, s: Soltar) -> bool {
    let (n, ayuda) = (crate::idioma::t(n), crate::idioma::t(ayuda));
    let (r, resp) = ui.allocate_exact_size(vec2(ancho, 30.0), Sense::click_and_drag());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, n));
    if ui.is_rect_visible(r) {
        let p = ui.painter_at(r);
        if resp.hovered() {
            p.rect_filled(r, 5, BUTTON);
        }
        let mut x = r.left() + 6.0;
        if let Some(t) = tex {
            let img = Rect::from_min_size(pos2(x, r.top() + 4.0), vec2(39.0, 22.0));
            p.image(t.id(), img, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            x = img.right() + 8.0;
        }
        let g = p.layout_no_wrap(n.into(), fuente(14.0), TEXT);
        let fin_n = x + g.size().x;
        p.galley(pos2(x, r.center().y - g.size().y / 2.0), g, TEXT);
        if !ayuda.is_empty() && fin_n + 40.0 < r.right() {
            let h = p.layout(ayuda.into(), fuente(12.0), TEXT_FAINT, f32::INFINITY);
            let zona = Rect::from_min_max(pos2(fin_n + 10.0, r.top()), pos2(r.right() - 6.0, r.bottom()));
            ui.painter_at(zona).galley(pos2(zona.left(), r.center().y - h.size().y / 2.0), h, TEXT_FAINT);
        }
    }
    resp.dnd_set_drag_payload(s);
    if resp.dragged() {
        if let Some(m) = ui.ctx().pointer_interact_pos() {
            let capa = ui.ctx().layer_painter(LayerId::new(Order::Tooltip, Id::new("arrastrando")));
            let g = capa.layout_no_wrap(n.into(), fuente(14.0), COLORED_DARK);
            let caja = Rect::from_min_size(m + vec2(12.0, 6.0), g.size() + vec2(14.0, 8.0));
            capa.rect_filled(caja, 6, MAIN);
            capa.galley(caja.min + vec2(7.0, 4.0), g, COLORED_DARK);
        }
    }
    let dbl = resp.double_clicked();
    if !resp.dragged() {
        resp.on_hover_ui(|ui| {
            ui.set_max_width(256.0);
            if let Some(t) = tex {
                ui.add(Image::new(t).fit_to_exact_size(vec2(256.0, 144.0)).corner_radius(4));
            }
            ui.label(RichText::new(crate::idioma::t(&n)).size(15.0).color(TEXT));
            if !ayuda.is_empty() {
                ui.label(RichText::new(ayuda).size(13.0).color(TEXT_DIM));
            }
            ui.label(RichText::new(crate::idioma::t("Doble clic para ponerlo, o arrástralo a un clip")).size(12.0).color(TEXT_FAINT));
        });
    }
    dbl
}

/// Pestaña chica y sin color, para elegir qué se lista.
fn pestana_chica(ui: &mut Ui, n: &str, on: bool) -> Response {
    let n = crate::idioma::t(n);
    let g = ui.painter().layout_no_wrap(n.into(), fuente(13.0), TEXT);
    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 14.0, 22.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, n));
    if on || resp.hovered() {
        ui.painter().rect_filled(r, 5, if on { BUTTON_HI } else { BUTTON });
    }
    ui.painter().text(r.center(), Align2::CENTER_CENTER, n, fuente(13.0), if on { TEXT } else { TEXT_DIM });
    resp
}

pub fn pastilla(ui: &mut Ui, n: &str, on: bool) -> Response {
    let n = crate::idioma::t(n);
    let g = ui.painter().layout_no_wrap(n.into(), fuente(14.0), TEXT);
    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 18.0, 24.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, n));
    if on {
        degradado(ui.painter(), r, 6, mezcla(HIGHLIGHT, MAIN, 0.45), MAIN);
    } else {
        ui.painter().rect_filled(r, 6, if resp.hovered() { BUTTON_HI } else { BUTTON });
    }
    ui.painter().text(r.center(), Align2::CENTER_CENTER, n, fuente(14.0), if on { COLORED_DARK } else { TEXT });
    resp
}
