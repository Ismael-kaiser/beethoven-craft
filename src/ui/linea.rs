//! Línea de tiempo: regla, pistas de video y de audio, y las herramientas de edición.
use super::biblioteca::Soltar;
use super::estilo::*;
use super::*;
use eframe::egui::epaint;
use crate::edicion::{Carril, Orilla, Tramo};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Herramienta {
    Flecha,
    Adelante,
    Ondulado,
    Rodar,
    Estirar,
    Cuchilla,
    Deslizar,
    Correr,
    Mano,
    Lupa,
    Texto,
}

/// (herramienta, ícono, tecla, ayuda)
pub const HERRAMIENTAS: [(Herramienta, &tabler::TablerIcon, egui::Key, &str); 11] = [
    (Herramienta::Flecha, &tabler::HAND_FINGER, egui::Key::V, "Selección (V): mover, recortar y elegir; Shift suma a la selección"),
    (Herramienta::Adelante, &tabler::ARROW_AUTOFIT_RIGHT, egui::Key::A, "Selección hacia adelante (A): mueve el clip y todo lo que sigue"),
    (Herramienta::Ondulado, &tabler::ARROWS_HORIZONTAL, egui::Key::B, "Recorte ondulado (B): recorta y recorre lo demás, sin huecos"),
    (Herramienta::Rodar, &tabler::BLEND_MODE, egui::Key::N, "Rodar (N): mueve el corte entre dos clips"),
    (Herramienta::Estirar, &tabler::STOPWATCH, egui::Key::X, "Estirar (X): cambia la velocidad jalando la orilla"),
    (Herramienta::Cuchilla, &tabler::SLICE, egui::Key::C, "Cuchilla (C): corta; con Shift corta todas las pistas"),
    (Herramienta::Deslizar, &tabler::ROTATE_CLOCKWISE_2, egui::Key::Y, "Deslizar (Y): cambia qué parte del archivo se ve, sin mover el clip"),
    (Herramienta::Correr, &tabler::REPEAT, egui::Key::U, "Correr (U): mueve el clip y ajusta a sus vecinos"),
    (Herramienta::Mano, &tabler::MAXIMIZE, egui::Key::H, "Mano (H): desplaza la vista"),
    (Herramienta::Lupa, &tabler::ZOOM_IN, egui::Key::Z, "Lupa (Z): acerca; con Alt aleja"),
    (Herramienta::Texto, &tabler::FILE_TYPOGRAPHY, egui::Key::T, "Texto (T): clic en una pista de video pone un título"),
];

pub struct Estado {
    /// Píxeles por segundo y segundo en el borde izquierdo.
    pub zoom: f64,
    pub x0: f64,
    pub iman: bool,
    pub encuadrar: bool,
    pub herramienta: Herramienta,
    /// Alto de las pistas.
    pub alto: f32,
    arrastre: Option<Arrastre>,
    /// Automatización visible por pista: índice en `Pista::auto`.
    pub auto_ver: HashMap<u64, usize>,
    punto: Option<(u64, usize)>,
    /// Segundo donde quedó pegado el último arrastre, para la guía.
    guia: Option<f64>,
}

impl Default for Estado {
    fn default() -> Self {
        Estado {
            zoom: 40.0,
            x0: 0.0,
            iman: true,
            encuadrar: false,
            herramienta: Herramienta::Flecha,
            alto: 64.0,
            arrastre: None,
            auto_ver: HashMap::new(),
            punto: None,
            guia: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Zona {
    Mover,
    Izq,
    Der,
    FadeIn,
    FadeOut,
}

/// Un arrastre guarda el proyecto como estaba: cada cuadro se rehace desde ahí, con los límites de cada operación.
struct Arrastre {
    id: u64,
    zona: Zona,
    /// Segundo donde se tomó.
    t: f64,
    base: Box<Proyecto>,
    cambio: bool,
}

/// Qué pistas se muestran.
#[derive(Clone, Copy, PartialEq)]
pub enum Que {
    Todo,
    Audio,
}

pub const AYUDA_VACIO: &str = "No tiene imagen: los efectos, estilos y colores que le pongas se aplican a todo lo que está debajo, también a los textos";
pub const CAB: f32 = 212.0;
const REGLA: f32 = 30.0;
const ORILLA: f32 = 7.0;

/// Forma de onda del tramo visible de un clip.
#[allow(clippy::too_many_arguments)]
pub fn onda(p: &Painter, r: Rect, a: &Audio, de: f64, a_seg: f64, inv: bool, g: f32, col: Color32) {
    let vis = r.intersect(p.clip_rect());
    if vis.width() <= 0.0 || a.onda.is_empty() {
        return;
    }
    let pps = SR as f64 / medios::PUNTO as f64;
    let seg_px = (a_seg - de) / r.width() as f64;
    let mut m = Mesh::default();
    let cy = r.center().y;
    let h = r.height() / 2.0 - 1.0;
    let mut x = vis.left().floor();
    while x < vis.right() {
        let mut t0 = de + (x - r.left()) as f64 * seg_px;
        if inv {
            t0 = a_seg - (x - r.left()) as f64 * seg_px - seg_px;
        }
        let i = ((t0 * pps) as usize).min(a.onda.len() - 1);
        let j = (((t0 + seg_px) * pps).ceil() as usize).clamp(i + 1, a.onda.len());
        let (lo, hi) = a.onda[i..j].iter().fold((0f32, 0f32), |(l, u), v| (l.min(v.0), u.max(v.1)));
        let (y0, y1) = (cy - (hi * g).min(1.0) * h, cy - (lo * g).max(-1.0) * h);
        m.add_colored_rect(Rect::from_min_max(pos2(x, y0 - 0.5), pos2(x + 1.0, y1 + 0.5)), col);
        x += 1.0;
    }
    p.add(Shape::mesh(m));
}

/// Rango de una automatización: mínimo, máximo, logarítmico.
fn rango(p: &Pista, d: &Destino) -> (f32, f32, bool) {
    match d {
        Destino::Volumen => (-40.0, 6.0, false),
        Destino::Paneo => (-1.0, 1.0, false),
        Destino::Fx(id, k) => p.fx.iter().find(|f| f.id == *id).and_then(|f| f.tipo.params().get(*k)).map_or((0.0, 1.0, false), |x| (x.min, x.max, x.log)),
    }
}

pub fn nombre_auto(p: &Pista, d: &Destino) -> String {
    match d {
        Destino::Volumen => "Volumen".into(),
        Destino::Paneo => "Paneo".into(),
        Destino::Fx(id, k) => p
            .fx
            .iter()
            .find(|f| f.id == *id)
            .and_then(|f| Some(format!("{} · {}", f.tipo.nombre(), f.tipo.params().get(*k)?.nombre)))
            .unwrap_or(crate::idioma::t("(efecto quitado)").into()),
    }
}

impl App {
    /// Pega `t` al borde más cercano de otro clip, al cursor, a una marca o al cero.
    fn pegar(&self, base: &Proyecto, t: f64, propios: &[u64]) -> Option<f64> {
        if !self.linea.iman {
            return None;
        }
        let tol = 8.0 / self.linea.zoom;
        let mut v = vec![0.0, self.reloj.lock().unwrap().pos];
        v.extend(base.tramos().iter().filter(|x| !propios.contains(&x.id)).flat_map(|x| [x.inicio, x.fin()]));
        v.extend(base.marcas.iter().map(|m| m.t));
        if base.pistas.iter().any(|p| !p.patrones.is_empty()) {
            let k = base.pulso();
            v.push((t / k).round() * k);
        }
        v.into_iter().filter(|b| (b - t).abs() < tol).min_by(|a, b| (a - t).abs().total_cmp(&(b - t).abs()))
    }

    /// Aplica el arrastre en curso con el ratón en el segundo `t`.
    fn arrastrar(&mut self, t: f64, carril: Option<Carril>) {
        let Some(a) = &self.linea.arrastre else { return };
        let (id, zona, h) = (a.id, a.zona, self.linea.herramienta);
        let mut p = (*a.base).clone();
        let mut d = t - a.t;
        let Some(tr) = p.tramo_de(id) else { return };
        let mut ids: Vec<u64> = if self.seleccion.contains(&id) { self.seleccion.iter().copied().collect() } else { vec![id] };
        // Cada elegido arrastra a sus vinculados; un grupo se mueve una sola vez.
        ids.retain(|x| p.tramo_de(*x).is_some());
        let propios: Vec<u64> = ids.iter().flat_map(|x| p.vinculados(*x)).map(|x| x.id).collect();
        let mut guia = None;
        let deslizar = h == Herramienta::Deslizar && zona == Zona::Mover;
        if !deslizar && !matches!(zona, Zona::FadeIn | Zona::FadeOut) {
            let bordes: Vec<f64> = match zona {
                Zona::Izq => vec![tr.inicio],
                Zona::Der => vec![tr.fin()],
                _ => vec![tr.inicio, tr.fin()],
            };
            if let Some((g, x)) = bordes
                .iter()
                .filter_map(|b| self.pegar(&p, b + d, &propios).map(|g| (g, g - b)))
                .min_by(|a, b| (a.1 - d).abs().total_cmp(&(b.1 - d).abs()))
            {
                guia = Some(g);
                d = x;
            }
        }
        let o = if zona == Zona::Izq { Orilla::Izq } else { Orilla::Der };
        match (zona, h) {
            (Zona::Mover, Herramienta::Deslizar) => p.deslizar(id, -d),
            (Zona::Mover, Herramienta::Correr) => {
                p.correr(id, d);
            }
            (Zona::Mover, Herramienta::Adelante) => {
                let d = d.max(-tr.inicio);
                p.empujar(tr.inicio, d, &[]);
            }
            (Zona::Mover, _) => {
                // Nadie queda antes del cero, así que todos se mueven lo mismo.
                let min = propios.iter().filter_map(|x| p.tramo_de(*x)).map(|x| x.inicio).fold(f64::MAX, f64::min);
                let d = d.max(-min);
                let mut hechos: Vec<u64> = vec![];
                for x in &ids {
                    if !hechos.contains(x) {
                        hechos.extend(p.vinculados(*x).iter().map(|y| y.id));
                        p.mover_grupo(*x, d);
                    }
                }
                if let Some(c) = carril {
                    cambiar_carril(&mut p, id, c);
                }
            }
            (Zona::FadeIn, _) => {
                if let Some(c) = p.clip(id) {
                    c.fade_in = (c.fade_in + d).clamp(0.0, c.dur - c.fade_out);
                }
            }
            (Zona::FadeOut, _) => {
                if let Some(c) = p.clip(id) {
                    c.fade_out = (c.fade_out - d).clamp(0.0, c.dur - c.fade_in);
                }
            }
            (_, Herramienta::Ondulado) => {
                p.recorte_ondulado(id, o, d);
            }
            (_, Herramienta::Rodar) => {
                p.rodar(id, o, d);
            }
            (_, Herramienta::Estirar) => {
                let dur = if o == Orilla::Der { tr.dur + d } else { tr.dur - d };
                if p.estirar(id, dur).is_some() && o == Orilla::Izq {
                    let n = p.tramo_de(id).map_or(tr.dur, |x| x.dur);
                    p.mover_grupo(id, tr.dur - n);
                }
            }
            _ => {
                p.recortar(id, o, d);
            }
        }
        self.linea.guia = guia;
        self.p = p;
        if let Some(a) = &mut self.linea.arrastre {
            a.cambio |= d.abs() > 1e-6;
        }
    }

    /// Al soltar: lo que se movió tapa a lo que estaba debajo en su pista.
    fn soltar_arrastre(&mut self) {
        let Some(a) = self.linea.arrastre.take() else { return };
        self.linea.guia = None;
        let mueve = a.zona == Zona::Mover && matches!(self.linea.herramienta, Herramienta::Flecha | Herramienta::Mano | Herramienta::Lupa | Herramienta::Texto);
        if !a.cambio || !(mueve || a.zona == Zona::Izq || a.zona == Zona::Der) || matches!(self.linea.herramienta, Herramienta::Ondulado | Herramienta::Rodar) {
            return;
        }
        let ids: Vec<u64> = if self.seleccion.contains(&a.id) { self.seleccion.iter().copied().collect() } else { vec![a.id] };
        let v: Vec<Tramo> = ids.iter().flat_map(|x| self.p.vinculados(*x)).collect();
        let propios: Vec<u64> = v.iter().map(|x| x.id).collect();
        for x in v {
            // Los patrones pueden ir encimados con grabaciones: suenan juntos.
            if !self.p.pistas.iter().any(|p| p.patrones.iter().any(|c| c.id == x.id)) {
                self.p.vaciar(x.carril, x.inicio, x.fin(), &propios);
            }
        }
    }

    pub fn ui_linea(&mut self, ui: &mut Ui, que: Que) {
        let todo = ui.available_rect_before_wrap();
        ui.allocate_rect(todo, Sense::hover());
        let ancho = (todo.width() - CAB).max(50.0);
        if self.linea.encuadrar {
            self.linea.encuadrar = false;
            let fin = self.p.fin().max(5.0);
            self.linea.zoom = (ancho as f64 * 0.96 / fin).clamp(0.5, 400.0);
            self.linea.x0 = 0.0;
        }
        let raton = ui.input(|i| i.pointer.hover_pos()).filter(|m| todo.contains(*m) && !ui.ctx().any_popup_open());
        if let Some(m) = raton {
            let (z, s) = ui.input(|i| (i.zoom_delta() as f64, i.smooth_scroll_delta()));
            if z != 1.0 {
                self.acercar(z, (m.x - todo.left() - CAB) as f64);
            }
            self.linea.x0 -= s.x as f64 / self.linea.zoom;
        }
        self.linea.x0 = self.linea.x0.clamp(0.0, (self.p.fin() + 30.0).max(0.0));
        let t = self.ahora();
        let izq = todo.left() + CAB;
        if self.tocando() {
            let x = (t - self.linea.x0) * self.linea.zoom;
            if x > ancho as f64 * 0.95 || x < 0.0 {
                self.linea.x0 = (t - ancho as f64 * 0.05 / self.linea.zoom).max(0.0);
            }
        }
        let (zoom, x0) = (self.linea.zoom, self.linea.x0);
        let a_x = move |s: f64| izq + ((s - x0) * zoom) as f32;
        let a_t = move |x: f32| x0 + (x - izq) as f64 / zoom;

        // Regla
        let regla = Rect::from_min_size(pos2(izq, todo.top()), vec2(ancho, REGLA));
        let cab = Rect::from_min_size(todo.min, vec2(CAB, REGLA));
        ui.painter().rect_filled(Rect::from_min_max(todo.min, regla.max), 0, RULER);
        ui.painter().text(cab.left_center() + vec2(8.0, 0.0), Align2::LEFT_CENTER, tiempo(t), fuente(16.0), MAIN);
        let pasos = [0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0];
        let paso = pasos.into_iter().find(|p| p * zoom >= 70.0).unwrap_or(600.0);
        let pr = ui.painter_at(Rect::from_min_max(regla.min, todo.max));
        pr.rect_filled(Rect::from_min_max(pos2(izq, regla.bottom()), todo.max), 0, GRID);
        let mut k = (x0 / paso).floor();
        while a_x(k * paso) < todo.right() {
            let x = a_x(k * paso);
            if k as i64 % 2 == 0 {
                pr.rect_filled(Rect::from_min_max(pos2(x, regla.bottom()), pos2(a_x((k + 1.0) * paso), todo.bottom())), 0, GRID_ALT);
            }
            pr.vline(x, regla.bottom() - 6.0..=todo.bottom(), Stroke::new(1.0, GRID_BAR));
            let s = k * paso;
            let txt = if paso < 1.0 {
                format!("{}:{:04.1}", (s / 60.0) as u64, s % 60.0)
            } else {
                format!("{}:{:02}", (s / 60.0) as u64, (s % 60.0).round() as u64)
            };
            pr.text(pos2(x + 4.0, regla.top() + 4.0), Align2::LEFT_TOP, txt, fuente(13.0), RULER_TEXT);
            k += 1.0;
        }
        // Tramo marcado
        if let (Some(a), Some(b)) = self.p.tramo {
            pr.rect_filled(Rect::from_min_max(pos2(a_x(a), regla.top()), pos2(a_x(b), todo.bottom())), 0, MAIN.gamma_multiply(0.10));
            pr.rect_filled(Rect::from_min_max(pos2(a_x(a), regla.top()), pos2(a_x(b), regla.top() + 3.0)), 0, MAIN);
        }
        let rr = ui.interact(regla, Id::new("regla"), Sense::click_and_drag());
        if let Some(m) = rr.interact_pointer_pos() {
            if rr.clicked() || rr.dragged() || rr.is_pointer_button_down_on() {
                self.ir(a_t(m.x));
            }
        }

        let mut acciones: Vec<Accion> = vec![];
        let filas = Rect::from_min_max(pos2(todo.left(), regla.bottom()), todo.max);
        let mut filas_ui = ui.new_child(UiBuilder::new().max_rect(filas));
        let mut carriles: Vec<(Carril, f32)> = vec![];
        ScrollArea::vertical().id_salt("filas").auto_shrink([false, false]).show(&mut filas_ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            if que == Que::Todo {
                for capa in (0..self.p.capas.len()).rev() {
                    let y = self.fila_video(ui, capa, ancho, &a_x, &a_t, &mut acciones);
                    carriles.push((Carril::Video(capa), y));
                }
            }
            if que == Que::Todo {
                // Separador entre imagen y sonido.
                let (s, _) = ui.allocate_exact_size(vec2(CAB + ancho, 7.0), Sense::hover());
                ui.painter().rect_filled(s, 0, c(0x191919));
                ui.painter().hline(s.x_range(), s.center().y, Stroke::new(1.0, MAIN.gamma_multiply(0.35)));
            }
            for i in 0..self.p.pistas.len() {
                let y = self.fila_audio(ui, i, ancho, &a_x, &a_t, &mut acciones);
                carriles.push((Carril::Audio(self.p.pistas[i].id), y));
            }
            // Un solo botón, del ancho de las cabeceras: no se encima en las pistas.
            let (r, _) = ui.allocate_exact_size(vec2(CAB + ancho, 44.0), Sense::hover());
            ui.painter().hline(r.left() + CAB..=r.right(), r.top(), Stroke::new(1.0, c(0x1a1a1a)));
            let caja = Rect::from_min_size(r.min + vec2(4.0, 7.0), vec2(CAB - 8.0, 30.0));
            let mut u = ui.new_child(UiBuilder::new().max_rect(caja));
            let b = Button::new(RichText::new(crate::idioma::t("+  Agregar pista")).size(15.0)).min_size(caja.size());
            egui::containers::menu::MenuButton::from_button(b).ui(&mut u, |ui| {
                ui.set_min_width(CAB - 24.0);
                if ui.button(crate::idioma::t("Pista de audio")).clicked() {
                    acciones.push(Accion::PistaAudio);
                }
                if ui.button(crate::idioma::t("Pista de grabación (tu voz)")).clicked() {
                    acciones.push(Accion::PistaNueva);
                }
                ui.menu_button(crate::idioma::t("Instrumento"), |ui| self.menu_instrumentos(ui));
                if que == Que::Todo && ui.button(crate::idioma::t("Pista de video")).clicked() {
                    acciones.push(Accion::CapaNueva);
                }
                if que == Que::Todo && ui.button(crate::idioma::t("Clip vacío (efectos para todo)")).on_hover_text(crate::idioma::t(AYUDA_VACIO)).clicked() {
                    let (t, dur) = self.hueco_vacio();
                    acciones.push(Accion::ClipVacio(t, dur));
                }
                ui.separator();
                if ui.button(crate::idioma::t("Importar video o audio…")).clicked() {
                    acciones.push(Accion::Importar);
                }
            });
            if ui.is_rect_visible(r) {
                ui.painter_at(Rect::from_min_max(pos2(izq, r.top()), r.max)).text(pos2(izq + 12.0, r.center().y), Align2::LEFT_CENTER, crate::idioma::t("Suelta aquí videos, canciones o imágenes"), fuente(13.0), c(0x5a5a5a));
            }
        });
        // Soltar debajo de las pistas: título o estilo encima de lo que haya, medio nuevo o efecto.
        let vacio = Rect::from_min_max(pos2(izq, carriles.last().map_or(filas.top(), |c| c.1 + self.linea.alto) + 1.0), todo.max);
        if vacio.height() > 10.0 {
            let rv = ui.interact(vacio, Id::new("linea-vacia"), Sense::click());
            if rv.clicked() {
                self.clip = None;
                self.seleccion.clear();
            }
            if rv.contains_pointer() && DragAndDrop::has_payload_of_type::<Soltar>(ui.ctx()) {
                ui.painter().rect_stroke(vacio.shrink(4.0), 8, Stroke::new(1.5, MAIN), StrokeKind::Inside);
            }
            if let Some(s) = rv.dnd_release_payload::<Soltar>() {
                let t = ui.input(|i| i.pointer.interact_pos()).map_or(0.0, |m| a_t(m.x).max(0.0));
                match &*s {
                    Soltar::Titulo(i) => self.poner_titulo(*i, t, None),
                    Soltar::Medio(r) => self.poner_medio(&r.clone(), t, 0, None, false),
                    // Un efecto suelto en el vacío vale para todo lo de abajo.
                    Soltar::FxVideo(f) => {
                        let id = self.clip_vacio(t, 5.0);
                        self.agregar_fx_video(id, f);
                    }
                    Soltar::Filtro(i) => {
                        let id = self.clip_vacio(t, 5.0);
                        self.poner_filtro(Some(id), *i);
                    }
                    Soltar::Estilo(i) => {
                        let id = self.clip_vacio(t, 5.0);
                        self.poner_estilo(Some(id), *i);
                    }
                    _ => self.avisar(crate::idioma::t("Suéltalo sobre un clip o una pista"), false),
                }
            }
        }
        // Arrastre en curso
        if self.linea.arrastre.is_some() {
            if let Some(m) = ui.input(|i| i.pointer.interact_pos()) {
                let carril = carriles.iter().find(|(_, y)| m.y >= *y && m.y < y + self.linea.alto).map(|c| c.0);
                self.arrastrar(a_t(m.x), carril);
            }
            if !ui.input(|i| i.pointer.any_down()) {
                self.soltar_arrastre();
            }
        }
        // Mano y lupa actúan sobre toda la zona
        let zona = Rect::from_min_max(pos2(izq, regla.bottom()), todo.max);
        if matches!(self.linea.herramienta, Herramienta::Mano | Herramienta::Lupa) {
            let r = ui.interact(zona, Id::new("mano-lupa"), Sense::click_and_drag());
            if self.linea.herramienta == Herramienta::Mano {
                if r.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::Grab);
                }
                self.linea.x0 -= r.drag_delta().x as f64 / zoom;
            } else {
                if r.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::ZoomIn);
                }
                if let (true, Some(m)) = (r.clicked(), r.interact_pointer_pos()) {
                    let k = if ui.input(|i| i.modifiers.alt) { 0.6 } else { 1.6 };
                    self.acercar(k, (m.x - izq) as f64);
                }
            }
        }
        let pr = ui.painter_at(Rect::from_min_max(regla.min, todo.max));
        for m in &self.p.marcas {
            let x = a_x(m.t);
            pr.vline(x, regla.bottom() - 8.0..=todo.bottom(), Stroke::new(1.0, c(0x43c0c8)));
            pr.text(pos2(x + 3.0, regla.bottom() - 11.0), Align2::LEFT_CENTER, &m.nombre, fuente(11.0), c(0x43c0c8));
        }
        if let Some(g) = self.linea.guia {
            pr.vline(a_x(g), regla.bottom()..=todo.bottom(), Stroke::new(1.0, Color32::WHITE));
        }
        let x = a_x(t);
        if x >= izq - 1.0 {
            let col = if self.grab.is_some() { RECORD } else { MAIN };
            pr.vline(x, regla.top() + 8.0..=todo.bottom(), Stroke::new(1.5, col));
            pr.add(Shape::convex_polygon(
                vec![pos2(x - 6.0, regla.top() + 4.0), pos2(x + 6.0, regla.top() + 4.0), pos2(x, regla.top() + 14.0)],
                col,
                Stroke::NONE,
            ));
        }
        for a in acciones {
            self.accion(a);
        }
    }

    /// Pie: mapa de todo el proyecto, zoom y alto de las pistas.
    pub fn pie_linea(&mut self, ui: &mut Ui) {
        let todo = ui.available_rect_before_wrap();
        ui.allocate_rect(todo, Sense::hover());
        let mapa = Rect::from_min_max(pos2(todo.left() + CAB - 6.0, todo.top()), pos2(todo.right() - 290.0, todo.bottom()));
        let fin = (self.p.fin() * 1.05).max(10.0);
        let p = ui.painter_at(mapa);
        p.rect_filled(mapa, 5, WELL);
        let x = |t: f64| mapa.left() + (t / fin) as f32 * mapa.width();
        let n = (self.p.capas.len() + self.p.pistas.len()).max(1) as f32;
        let h = (mapa.height() - 4.0) / n;
        for t in self.p.tramos() {
            let (f, col) = match t.carril {
                Carril::Video(i) => ((self.p.capas.len() - 1 - i.min(self.p.capas.len() - 1)) as f32, DEEP),
                Carril::Audio(id) => {
                    let i = self.p.pistas.iter().position(|p| p.id == id).unwrap_or(0);
                    ((self.p.capas.len() + i) as f32, color_pista(i))
                }
            };
            let y = mapa.top() + 2.0 + f * h;
            p.rect_filled(Rect::from_min_max(pos2(x(t.inicio), y), pos2(x(t.fin()).max(x(t.inicio) + 1.0), y + (h - 0.5).max(1.0))), 1, col);
        }
        let vis = (mapa.width() + 290.0) as f64 / self.linea.zoom;
        let v = Rect::from_min_max(pos2(x(self.linea.x0), mapa.top()), pos2(x((self.linea.x0 + vis).min(fin)), mapa.bottom()));
        p.rect_filled(v, 4, Color32::from_white_alpha(28));
        p.rect_stroke(v, 4, Stroke::new(1.0, MAIN.gamma_multiply(0.7)), StrokeKind::Inside);
        p.vline(x(self.ahora()), mapa.y_range(), Stroke::new(1.0, MAIN));
        let r = ui.interact(mapa, Id::new("mapa"), Sense::click_and_drag());
        if let (true, Some(m)) = (r.dragged() || r.clicked(), r.interact_pointer_pos()) {
            self.linea.x0 = (((m.x - mapa.left()) / mapa.width()) as f64 * fin - vis / 2.0).max(0.0);
        }
        let mut u = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(pos2(mapa.right() + 8.0, todo.top()), todo.max)).layout(Layout::left_to_right(Align::Center)));
        u.spacing_mut().slider_width = 84.0;
        if u.button(crate::idioma::t("Todo")).on_hover_text("Ver todo el proyecto (\\)").clicked() {
            self.linea.encuadrar = true;
        }
        let mut z = self.linea.zoom.ln();
        u.label(RichText::new(crate::idioma::t("Zoom")).size(12.0).color(TEXT_FAINT));
        if u.add(Slider::new(&mut z, 0.5f64.ln()..=1000f64.ln()).show_value(false)).changed() {
            // Acerca alrededor del cursor de reproducción si se ve; si no, al centro.
            let c = (self.ahora() - self.linea.x0) * self.linea.zoom;
            let px = if c >= 0.0 && c <= mapa.width() as f64 { c } else { mapa.width() as f64 / 2.0 };
            self.acercar(z.exp() / self.linea.zoom, px);
        }
        u.label(RichText::new(crate::idioma::t("Alto")).size(12.0).color(TEXT_FAINT));
        u.add(Slider::new(&mut self.linea.alto, 44.0..=130.0).show_value(false));
    }

    pub fn acercar(&mut self, k: f64, px: f64) {
        let t = self.linea.x0 + px / self.linea.zoom;
        self.linea.zoom = (self.linea.zoom * k).clamp(0.5, 2000.0);
        self.linea.x0 = (t - px / self.linea.zoom).max(0.0);
    }

    fn accion(&mut self, a: Accion) {
        match a {
            Accion::Importar => self.menu_importar(),
            Accion::PistaNueva => {
                let id = self.p.nueva_pista_grabacion();
                self.canal = Canal::Pista(id);
            }
            Accion::PistaAudio => {
                let n = self.p.pistas.len() + 1;
                self.canal = Canal::Pista(self.p.nueva_pista(&crate::idioma::tf("Audio {}", &[&n])));
            }
            Accion::ClipVacio(t, dur) => {
                self.clip_vacio(t, dur);
            }
            Accion::CapaNueva => {
                let n = self.p.capas.len();
                self.p.capas.push(Capa { nombre: format!("V{}", n + 1), visible: true, bloqueada: false });
            }
            Accion::BorrarCapa(i) => {
                if self.p.capas.len() > 1 {
                    self.p.video.retain(|c| c.capa != i);
                    self.p.video.iter_mut().filter(|c| c.capa > i).for_each(|c| c.capa -= 1);
                    self.p.capas.remove(i);
                }
            }
            Accion::Soltado(s, clip, capa, pista, t, final_) => self.soltado(s, clip, capa, pista, t, final_),
            Accion::BorrarPista(id) => {
                self.p.pistas.retain(|p| p.id != id);
                if self.canal == Canal::Pista(id) {
                    self.canal = Canal::Maestro;
                }
            }
            Accion::SubirPista(i, d) => {
                let j = i as i64 + d;
                if j >= 0 && (j as usize) < self.p.pistas.len() {
                    self.p.pistas.swap(i, j as usize);
                }
            }
            Accion::Armar(id) => {
                self.p.armar(id);
                self.canal = Canal::Pista(id);
            }
            Accion::Dividir(id, t, todo) => {
                if todo {
                    self.p.dividir_todo(t);
                } else {
                    self.p.dividir_grupo(id, t);
                }
                // Cortar no deja nada elegido: así un Borrar después no se lleva pedazos de más.
                self.clip = None;
                self.seleccion.clear();
            }
            Accion::BorrarClip(id, cerrar) => {
                if cerrar {
                    self.p.borrar_ondulado(id);
                } else {
                    self.p.borrar_grupo(id);
                }
            }
            Accion::Duplicar(id) => {
                let nuevo = self.p.id();
                for p in &mut self.p.pistas {
                    if let Some(c) = p.clips.iter().find(|c| c.id == id) {
                        let mut d = c.clone();
                        (d.id, d.grupo) = (nuevo, None);
                        d.inicio = c.inicio + c.dur;
                        p.clips.push(d);
                        break;
                    }
                }
            }
            Accion::VerControles(id) => {
                self.clip = Some(id);
                self.panel_izq = PanelIzq::Controles;
            }
            Accion::DuplicarVideo(id) => {
                if let Some(c) = self.p.clip_video(id).cloned() {
                    let capa = self.capa_libre(c.inicio, c.dur);
                    let mut d = c;
                    (d.id, d.capa, d.grupo) = (self.p.id(), capa, None);
                    for f in &mut d.fx {
                        f.id = self.p.id();
                    }
                    self.clip = Some(d.id);
                    self.p.video.push(d);
                }
            }
            Accion::TraerAudio(id) => {
                if let Some(c) = self.p.clip_video(id).cloned() {
                    self.traer_audio(&c);
                }
            }
            Accion::Congelar(id) => self.congelar(id),
            Accion::Animar(id) => self.animar(id),
            Accion::Desvincular(id) => self.p.desvincular(id),
            Accion::Vincular => {
                let v: Vec<u64> = self.seleccion.iter().copied().collect();
                self.p.vincular(&v);
            }
            Accion::Separar(id) => {
                if let Some(c) = self.p.clip(id).cloned() {
                    self.separar(c.ruta.clone(), c.inicio, Some(c), vec![]);
                }
            }
            Accion::Editar(id) => {
                self.clip = Some(id);
                self.pestana = Pestana::Editor;
                self.espacio = Espacio::Audio;
            }
            Accion::Auto(id, d) => {
                let t = self.ahora();
                if let Some(p) = self.p.pista(id) {
                    let i = p.auto.iter().position(|a| a.destino == d).unwrap_or_else(|| {
                        let v = match &d {
                            Destino::Volumen => p.vol_db,
                            Destino::Paneo => p.pan,
                            Destino::Fx(f, k) => p.fx.iter().find(|x| x.id == *f).and_then(|x| x.p.get(*k).copied()).unwrap_or(0.0),
                        };
                        p.auto.push(Automatizacion { destino: d, activa: true, puntos: vec![(t, v)] });
                        p.auto.len() - 1
                    });
                    self.linea.auto_ver.insert(id, i);
                }
            }
            Accion::AutoOcultar(id) => {
                self.linea.auto_ver.remove(&id);
            }
            Accion::AutoBorrar(id) => {
                if let Some(i) = self.linea.auto_ver.remove(&id) {
                    if let Some(p) = self.p.pista(id).filter(|p| i < p.auto.len()) {
                        p.auto.remove(i);
                    }
                }
            }
        }
    }

    fn soltado(&mut self, s: Soltar, clip: Option<u64>, capa: Option<usize>, pista: Option<u64>, t: f64, final_: bool) {
        // El efecto de imagen soltado en el sonido (o al revés) va a su pareja vinculada.
        let pareja = |p: &Proyecto, id: u64, video: bool| p.vinculados(id).into_iter().find(|x| matches!(x.carril, Carril::Video(_)) == video);
        match s {
            Soltar::FxVideo(f) => match clip.and_then(|id| pareja(&self.p, id, true)) {
                Some(x) => self.agregar_fx_video(x.id, f),
                None => self.avisar(crate::idioma::t("Suelta el efecto de video sobre un clip de video"), false),
            },
            Soltar::Transicion(x) => match clip.and_then(|id| pareja(&self.p, id, true)) {
                Some(v) => self.poner_transicion(v.id, x, final_),
                None => self.avisar(crate::idioma::t("Suelta la transición sobre un clip de video"), false),
            },
            Soltar::FxAudio(f) => {
                let pista = pista.or_else(|| match clip.and_then(|id| pareja(&self.p, id, false))?.carril {
                    Carril::Audio(id) => Some(id),
                    _ => None,
                });
                match pista {
                    Some(id) => {
                        let x = self.p.nuevo_fx(f);
                        self.canal = Canal::Pista(id);
                        self.clip = None;
                        self.fx_de(Canal::Pista(id)).push(x);
                        self.panel_izq = PanelIzq::Controles;
                    }
                    None => self.avisar(crate::idioma::t("Ese video no tiene su sonido en una pista. Suéltalo sobre una pista de audio."), false),
                }
            }
            Soltar::Titulo(i) => self.poner_titulo(i, t, capa),
            Soltar::Filtro(i) => {
                let v = clip.and_then(|id| pareja(&self.p, id, true)).map(|x| x.id);
                self.poner_filtro(v, i);
            }
            Soltar::Estilo(i) => {
                let v = clip.and_then(|id| pareja(&self.p, id, true)).map(|x| x.id);
                self.poner_estilo(v, i);
            }
            Soltar::Medio(r) => match capa {
                Some(capa) => self.poner_medio(&r, t, capa, None, false),
                None => self.importar(r),
            },
        }
    }

    /// Zona del clip bajo el ratón y su cursor, según la herramienta.
    fn zona(&self, ui: &Ui, resp: &Response, rc: Rect, fades: bool) -> Option<Zona> {
        use Herramienta::*;
        let m = if resp.drag_started() { ui.input(|i| i.pointer.press_origin()) } else { resp.hover_pos() }?;
        let h = self.linea.herramienta;
        let orilla = if m.x - rc.left() < ORILLA {
            Some(Zona::Izq)
        } else if rc.right() - m.x < ORILLA {
            Some(Zona::Der)
        } else {
            None
        };
        let z = match h {
            Cuchilla => {
                ui.ctx().set_cursor_icon(CursorIcon::VerticalText);
                return None;
            }
            Mano | Lupa => return None,
            Deslizar | Correr | Adelante => Zona::Mover,
            // Estas solo trabajan en orillas: se toma la más cercana.
            Ondulado | Rodar | Estirar => orilla.unwrap_or(if m.x < rc.center().x { Zona::Izq } else { Zona::Der }),
            _ if fades && m.y < rc.top() + 18.0 && rc.width() > 50.0 && m.x - rc.left() < 14.0 => Zona::FadeIn,
            _ if fades && m.y < rc.top() + 18.0 && rc.width() > 50.0 && rc.right() - m.x < 14.0 => Zona::FadeOut,
            _ => orilla.unwrap_or(Zona::Mover),
        };
        ui.ctx().set_cursor_icon(match (z, h) {
            (Zona::Mover, Deslizar | Correr) => CursorIcon::Move,
            (Zona::Mover, _) => CursorIcon::Grab,
            (Zona::FadeIn | Zona::FadeOut, _) => CursorIcon::Crosshair,
            _ => CursorIcon::ResizeHorizontal,
        });
        Some(z)
    }

    /// Selección, cuchilla y comienzo de arrastre, igual para imagen, sonido y patrones.
    fn tocar_clip(&mut self, ui: &Ui, resp: &Response, rc: Rect, id: u64, fades: bool, bloqueada: bool, a_t: &dyn Fn(f32) -> f64, ac: &mut Vec<Accion>) {
        if bloqueada {
            return;
        }
        let zona = self.zona(ui, resp, rc, fades);
        let shift = ui.input(|i| i.modifiers.shift);
        if self.linea.herramienta == Herramienta::Cuchilla {
            if let (true, Some(m)) = (resp.clicked(), resp.interact_pointer_pos()) {
                ac.push(Accion::Dividir(id, a_t(m.x), shift));
            }
            if let Some(m) = resp.hover_pos() {
                ui.painter().vline(m.x, rc.y_range(), Stroke::new(1.5, Color32::WHITE));
            }
            return;
        }
        if resp.clicked() {
            if shift {
                if !self.seleccion.remove(&id) {
                    self.seleccion.insert(id);
                }
            } else {
                self.seleccion.clear();
                self.seleccion.insert(id);
            }
            self.clip = Some(id);
        }
        if resp.drag_started() {
            if !self.seleccion.contains(&id) {
                self.seleccion.clear();
                self.seleccion.insert(id);
            }
            self.clip = Some(id);
            if let (Some(zona), Some(m)) = (zona, ui.input(|i| i.pointer.press_origin())) {
                self.linea.arrastre = Some(Arrastre { id, zona, t: a_t(m.x), base: Box::new(self.p.clone()), cambio: false });
            }
        }
    }

    fn elegido(&self, id: u64) -> bool {
        self.seleccion.contains(&id)
    }

    /// Marco del clip: elegido, o pareja de uno elegido.
    fn marco(&self, pr: &Painter, rc: Rect, id: u64, grupo: Option<u64>) {
        if self.elegido(id) {
            pr.rect_stroke(rc, 6, Stroke::new(2.0, MAIN), StrokeKind::Inside);
        } else if grupo.is_some() && self.seleccion.iter().any(|x| self.p.tramo_de(*x).is_some_and(|t| t.grupo == grupo)) {
            pr.rect_stroke(rc, 6, Stroke::new(1.5, MAIN.gamma_multiply(0.55)), StrokeKind::Inside);
        }
    }

    fn fila_video(&mut self, ui: &mut Ui, capa: usize, ancho: f32, a_x: &dyn Fn(f64) -> f32, a_t: &dyn Fn(f32) -> f64, ac: &mut Vec<Accion>) -> f32 {
        let fila = self.linea.alto;
        let (r, _) = ui.allocate_exact_size(vec2(CAB + ancho, fila), Sense::hover());
        let cab = Rect::from_min_size(r.min, vec2(CAB, fila));
        let rcab = ui.interact(cab, Id::new(("cabv", capa)), Sense::click());
        rcab.context_menu(|ui| {
            if ui.button(crate::idioma::t("Borrar pista de video")).clicked() {
                ac.push(Accion::BorrarCapa(capa));
            }
        });
        cabecera(ui, cab, &format!("V{}", capa + 1), false);
        let mut h = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(cab.min + vec2(50.0, 5.0), cab.max - vec2(8.0, 4.0))));
        h.spacing_mut().item_spacing.y = 2.0;
        h.horizontal(|ui| {
            let c = &mut self.p.capas[capa];
            ui.add(TextEdit::singleline(&mut c.nombre).frame(Frame::NONE).font(fuente(15.5)).desired_width(CAB - 136.0));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                if candado(ui, c.bloqueada).clicked() {
                    c.bloqueada = !c.bloqueada;
                }
                if ojo(ui, c.visible).clicked() {
                    c.visible = !c.visible;
                }
            });
        });
        if capa == 0 && fila > 46.0 {
            h.label(RichText::new(crate::idioma::tf("{}×{} · {} cps", &[&self.p.ancho, &self.p.alto, &format!("{:.2}", self.p.fps)])).size(12.0).color(TEXT_FAINT));
        }
        let carril = Rect::from_min_max(pos2(r.left() + CAB, r.top()), r.max);
        let rcar = ui.interact(carril, Id::new(("carrilv", capa)), Sense::click());
        if rcar.clicked() {
            match (self.linea.herramienta, rcar.interact_pointer_pos()) {
                (Herramienta::Texto, Some(m)) => {
                    self.poner_titulo(0, a_t(m.x).max(0.0), Some(capa));
                    self.linea.herramienta = Herramienta::Flecha;
                }
                _ => {
                    self.clip = None;
                    self.seleccion.clear();
                }
            }
        }
        let bloqueada = self.p.capas[capa].bloqueada;
        let pr = ui.painter_at(carril);
        if capa % 2 == 1 {
            pr.rect_filled(carril, 0, Color32::from_white_alpha(5));
        }
        pr.hline(carril.x_range(), carril.bottom() - 0.5, Stroke::new(1.0, c(0x1a1a1a)));
        rcar.context_menu(|ui| {
            let t = ui.input(|i| i.pointer.press_origin()).map_or(self.ahora(), |m| a_t(m.x).max(0.0));
            let k = Id::new("menu-carril-t");
            let t = ui.data_mut(|d| *d.get_temp_mut_or(k, t));
            if ui.button(crate::idioma::t("Clip vacío aquí (efectos para lo de abajo)")).on_hover_text(crate::idioma::t(AYUDA_VACIO)).clicked() {
                ac.push(Accion::ClipVacio(t, 5.0));
            }
            if ui.button(crate::idioma::t("Clip vacío para todo el video")).clicked() {
                ac.push(Accion::ClipVacio(0.0, self.p.fin().max(5.0)));
            }
        });
        if !rcar.context_menu_opened() {
            ui.data_mut(|d| d.remove::<f64>(Id::new("menu-carril-t")));
        }
        let mut soltado_en: Option<u64> = None;
        for i in 0..self.p.video.len() {
            if self.p.video[i].capa != capa {
                continue;
            }
            let v = self.p.video[i].clone();
            let rc = Rect::from_min_max(pos2(a_x(v.inicio), r.top() + 3.0), pos2((a_x(v.fin()) - 1.0).max(a_x(v.inicio) + 2.0), r.bottom() - 3.0));
            if !rc.intersects(carril) && self.linea.arrastre.as_ref().is_none_or(|a| a.id != v.id) {
                continue;
            }
            let (fondo, cinta) = match v.contenido {
                Contenido::Archivo => (c(0x5a4d1c), c(0x8f7a2a)),
                Contenido::Imagen => (c(0x24504a), c(0x357a70)),
                Contenido::Titulo(_) => (c(0x55264a), c(0x8a3f78)),
                Contenido::Ajuste => (c(0x2c3f5c), c(0x43608c)),
                Contenido::Forma(_) => (c(0x56331f), c(0x8a5432)),
                Contenido::Color(_) => (c(0x3c3c46), c(0x5c5c6c)),
            };
            pr.rect_filled(rc, 5, fondo);
            let tit = Rect::from_min_size(rc.min, vec2(rc.width(), 17.0));
            let cuerpo = Rect::from_min_max(pos2(rc.left(), rc.top() + 17.0), rc.max);
            if let Some(Some(tl)) = self.tiras.get(&v.ruta).filter(|_| v.contenido == Contenido::Archivo) {
                let pm = ui.painter_at(cuerpo.intersect(carril));
                let tam = tl.tex[0].size_vec2() * (cuerpo.height() / tl.tex[0].size_vec2().y);
                let mut x = cuerpo.left() + ((carril.left() - cuerpo.left()) / tam.x).floor().max(0.0) * tam.x;
                while x < cuerpo.right().min(carril.right()) {
                    let ts = v.fuente(a_t(x + tam.x / 2.0));
                    let k = ((ts / tl.paso) as usize).min(tl.tex.len() - 1);
                    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                    pm.image(tl.tex[k].id(), Rect::from_min_size(pos2(x, cuerpo.top()), tam), uv, Color32::from_gray(190));
                    x += tam.x;
                }
            }
            pr.rect_filled(tit, CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 }, cinta);
            for (tr, izq) in [(&v.entrada, true), (&v.salida, false)] {
                if let Some(tr) = tr {
                    let w = (tr.dur * self.linea.zoom) as f32;
                    let z = if izq {
                        Rect::from_min_max(cuerpo.min, pos2(cuerpo.left() + w, cuerpo.bottom()))
                    } else {
                        Rect::from_min_max(pos2(cuerpo.right() - w, cuerpo.top()), cuerpo.max)
                    };
                    pr.rect_filled(z, 2, MAIN.gamma_multiply(0.35));
                    let (a, b) = if izq { (z.left_bottom(), z.right_top()) } else { (z.left_top(), z.right_bottom()) };
                    pr.line_segment([a, b], Stroke::new(1.5, MAIN));
                }
            }
            let m = &v.mov;
            for q in m.props().into_iter().chain(v.fx.iter().flat_map(|f| f.p.values())) {
                for k in q.claves.iter().filter(|k| k.0 >= 0.0 && k.0 <= v.dur) {
                    let o = pos2(a_x(v.inicio + k.0), cuerpo.bottom() - 6.0);
                    pr.add(Shape::convex_polygon(
                        vec![o - vec2(0.0, 4.0), o + vec2(4.0, 0.0), o + vec2(0.0, 4.0), o - vec2(4.0, 0.0)],
                        HIGHLIGHT,
                        Stroke::new(1.0, COLORED_DARK),
                    ));
                }
            }
            let tr = tit.intersect(carril);
            let mut nombre = match &v.contenido {
                Contenido::Titulo(t) => t.texto.lines().next().unwrap_or("Título").to_string(),
                Contenido::Color(_) => "Color".into(),
                Contenido::Ajuste => crate::idioma::t("Clip vacío: efectos para lo de abajo").into(),
                Contenido::Forma(f) => crate::idioma::t(TipoForma::TODOS.iter().find(|x| x.0 == f.tipo).map_or("Figura", |x| x.1)).into(),
                _ => nombre_de(&v.ruta),
            };
            let nfx = v.fx.iter().filter(|f| f.activo).count();
            if (m.velocidad - 1.0).abs() > 1e-3 {
                nombre = crate::idioma::tf("[{} %] {}", &[&format!("{:.0}", m.velocidad * 100.0), &nombre]);
            }
            if nfx > 0 {
                nombre = crate::idioma::tf("fx{} · {}", &[&nfx, &nombre]);
            }
            titulo_clip(ui, tr, 6.0, &nombre, c(0xfff6d8));
            self.marco(&pr, rc, v.id, v.grupo);
            if bloqueada {
                pr.rect_filled(rc, 5, Color32::from_black_alpha(90));
            }
            let resp = ui.interact(rc.intersect(carril), Id::new(("video", v.id)), Sense::click_and_drag());
            if resp.contains_pointer() {
                soltado_en = Some(v.id);
                if DragAndDrop::has_payload_of_type::<Soltar>(ui.ctx()) {
                    pr.rect_stroke(rc, 5, Stroke::new(2.0, HIGHLIGHT), StrokeKind::Inside);
                }
            }
            self.tocar_clip(ui, &resp, rc, v.id, false, bloqueada, a_t, ac);
            if resp.double_clicked() {
                ac.push(Accion::VerControles(v.id));
            }
            resp.context_menu(|ui| {
                if ui.button(crate::idioma::t("Dividir en el cursor")).clicked() {
                    ac.push(Accion::Dividir(v.id, self.ahora(), false));
                }
                if ui.button(crate::idioma::t("Duplicar en otra pista")).clicked() {
                    ac.push(Accion::DuplicarVideo(v.id));
                }
                if matches!(v.contenido, Contenido::Titulo(_) | Contenido::Imagen | Contenido::Color(_) | Contenido::Forma(_) | Contenido::Ajuste | Contenido::Archivo) {
                    if v.animado {
                        if ui.button(crate::idioma::t("Abrir en la ventana de animación")).clicked() {
                            ac.push(Accion::Animar(v.id));
                        }
                    } else if ui.button(crate::idioma::t("Agregar a la ventana de animación")).clicked() {
                        ac.push(Accion::Animar(v.id));
                    }
                }
                if v.contenido == Contenido::Archivo {
                    if ui.button(crate::idioma::t("Congelar cuadro aquí")).on_hover_text(crate::idioma::t("Pone el cuadro del cursor como imagen, encima")).clicked() {
                        ac.push(Accion::Congelar(v.id));
                    }
                    if ui.button(crate::idioma::t("Traer su audio a una pista")).clicked() {
                        ac.push(Accion::TraerAudio(v.id));
                    }
                }
                self.menu_vinculo(ui, v.id, v.grupo, ac);
                ui.separator();
                if ui.button(crate::idioma::t("Borrar")).clicked() {
                    ac.push(Accion::BorrarClip(v.id, false));
                }
                if ui.button(crate::idioma::t("Borrar y cerrar hueco")).clicked() {
                    ac.push(Accion::BorrarClip(v.id, true));
                }
            });
        }
        if let (Some(s), Some(m)) = (rcar.dnd_release_payload::<Soltar>(), ui.input(|i| i.pointer.interact_pos())) {
            let t = a_t(m.x).max(0.0);
            let final_ = soltado_en.and_then(|id| self.p.video.iter().find(|c| c.id == id)).is_some_and(|c| t > c.inicio + c.dur / 2.0);
            ac.push(Accion::Soltado((*s).clone(), soltado_en, Some(capa), None, t, final_));
        }
        r.top()
    }

    fn menu_vinculo(&self, ui: &mut Ui, id: u64, grupo: Option<u64>, ac: &mut Vec<Accion>) {
        if grupo.is_some() && ui.button(crate::idioma::t("Desvincular imagen y sonido")).clicked() {
            ac.push(Accion::Desvincular(id));
        }
        if self.seleccion.len() > 1 && ui.button(crate::idioma::t("Vincular lo elegido")).clicked() {
            ac.push(Accion::Vincular);
        }
    }

    fn fila_audio(&mut self, ui: &mut Ui, i: usize, ancho: f32, a_x: &dyn Fn(f64) -> f32, a_t: &dyn Fn(f32) -> f64, ac: &mut Vec<Accion>) -> f32 {
        let fila = self.linea.alto;
        let (r, _) = ui.allocate_exact_size(vec2(CAB + ancho, fila), Sense::hover());
        let col = color_pista(i);
        let id = self.p.pistas[i].id;
        let elegida = self.canal == Canal::Pista(id);
        let cab = Rect::from_min_size(r.min, vec2(CAB, fila));
        let rcab = ui.interact(cab, Id::new(("cab", id)), Sense::click());
        if rcab.clicked() {
            self.canal = Canal::Pista(id);
            self.clip = None;
            self.seleccion.clear();
        }
        let p0 = self.p.pistas[i].clone();
        rcab.context_menu(|ui| {
            ui.menu_button(crate::idioma::t("Automatizar"), |ui| {
                for d in [Destino::Volumen, Destino::Paneo] {
                    if ui.button(nombre_auto(&p0, &d)).clicked() {
                        ac.push(Accion::Auto(id, d));
                    }
                }
                for f in &p0.fx {
                    ui.menu_button(crate::idioma::t(&f.tipo.nombre()), |ui| {
                        for (k, x) in f.tipo.params().iter().enumerate() {
                            if ui.button(crate::idioma::t(&x.nombre)).clicked() {
                                ac.push(Accion::Auto(id, Destino::Fx(f.id, k)));
                            }
                        }
                    });
                }
            });
            if ui.button(crate::idioma::t("Subir")).clicked() {
                ac.push(Accion::SubirPista(i, -1));
            }
            if ui.button(crate::idioma::t("Bajar")).clicked() {
                ac.push(Accion::SubirPista(i, 1));
            }
            if ui.button(crate::idioma::t("Borrar pista")).clicked() {
                ac.push(Accion::BorrarPista(id));
            }
        });
        cabecera(ui, cab, &format!("A{}", i + 1), elegida);
        // Medidor de la pista, pegado a la orilla.
        let md = Rect::from_min_max(pos2(cab.right() - 12.0, cab.top() + 5.0), pos2(cab.right() - 5.0, cab.bottom() - 5.0));
        medidor(ui.painter(), md, self.picos.get(&id).copied().unwrap_or_default());
        let mut h = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(cab.min + vec2(50.0, 5.0), cab.max - vec2(18.0, 4.0))));
        h.spacing_mut().item_spacing.y = 2.0;
        h.horizontal(|ui| {
            let p = &mut self.p.pistas[i];
            ui.add(TextEdit::singleline(&mut p.nombre).frame(Frame::NONE).font(fuente(15.5)).desired_width(CAB - 120.0));
            if !p.fx.is_empty() {
                let n = p.fx.iter().filter(|f| f.activo).count();
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.label(RichText::new(format!("fx{n}")).size(11.5).color(MAIN)));
            }
        });
        h.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            let p = &mut self.p.pistas[i];
            if letra(ui, "M", p.mudo, c(0xe0564f), "Silenciar la pista").clicked() {
                p.mudo = !p.mudo;
            }
            if letra(ui, "S", p.solo, MAIN, "Solo: oír únicamente esta pista").clicked() {
                p.solo = !p.solo;
            }
            if letra(ui, "●", p.armada, RECORD, "Armar: aquí caerá la grabación").clicked() {
                ac.push(Accion::Armar(id));
            }
            if candado(ui, p.bloqueada).clicked() {
                p.bloqueada = !p.bloqueada;
            }
            let mut v = p.vol_db;
            deslizador(ui, &mut v, "Volumen").on_hover_text(crate::idioma::tf("Volumen: {} dB (doble clic: 0)", &[&format!("{:.1}", v)]));
            p.vol_db = v;
        });
        let carril = Rect::from_min_max(pos2(r.left() + CAB, r.top()), r.max);
        let pr = ui.painter_at(carril);
        if elegida {
            pr.rect_filled(carril, 0, Color32::from_white_alpha(4));
        }
        pr.hline(carril.x_range(), carril.bottom() - 0.5, Stroke::new(1.0, c(0x1a1a1a)));
        let (mudo, bloqueada) = (self.p.pistas[i].mudo, self.p.pistas[i].bloqueada);
        let rfila = ui.interact(r, Id::new(("fila", id)), Sense::hover());
        if rfila.contains_pointer() && DragAndDrop::has_payload_of_type::<Soltar>(ui.ctx()) {
            ui.painter().rect_stroke(r.shrink(1.0), 5, Stroke::new(1.5, HIGHLIGHT), StrokeKind::Inside);
        }
        let mut soltado_en = None;
        self.patrones_ui(ui, i, carril, a_x, a_t, ac);
        for j in 0..self.p.pistas[i].clips.len() {
            let q = self.p.pistas[i].clips[j].clone();
            let rc = Rect::from_min_max(pos2(a_x(q.inicio), r.top() + 3.0), pos2((a_x(q.inicio + q.dur) - 1.0).max(a_x(q.inicio) + 2.0), r.bottom() - 3.0));
            if !rc.intersects(carril) && self.linea.arrastre.as_ref().is_none_or(|a| a.id != q.id) {
                continue;
            }
            let gris = |x: Color32| if mudo { mezcla(x, c(0x404040), 0.75) } else { x };
            pr.rect_filled(rc, 5, gris(mezcla(col, c(0x141414), 0.62)));
            let tit = Rect::from_min_size(rc.min, vec2(rc.width(), 18.0));
            pr.rect_filled(tit, CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 }, gris(mezcla(col, c(0x303030), 0.3)));
            let cuerpo = Rect::from_min_max(pos2(rc.left(), rc.top() + 18.0), rc.max);
            match self.almacen.get(&q.ruta) {
                Some(a) => onda(&pr, cuerpo, a, q.desde, q.desde + q.dur * q.velocidad, q.invertido, db(q.ganancia_db), gris(mezcla(col, Color32::WHITE, 0.55))),
                None => {
                    pr.text(cuerpo.center(), Align2::CENTER_CENTER, crate::idioma::t("Falta el audio"), fuente(13.0), TEXT_DIM);
                }
            }
            let sombra = Color32::from_black_alpha(130);
            for (f, izq) in [(q.fade_in, true), (q.fade_out, false)] {
                if f > 0.0 {
                    let w = (f / q.dur) as f32 * rc.width();
                    let (a, b) = if izq { (rc.left(), rc.left() + w) } else { (rc.right(), rc.right() - w) };
                    pr.add(Shape::convex_polygon(vec![pos2(a, cuerpo.top()), pos2(b, cuerpo.top()), pos2(a, cuerpo.bottom())], sombra, Stroke::NONE));
                    pr.line_segment([pos2(a, cuerpo.bottom()), pos2(b, cuerpo.top())], Stroke::new(1.0, Color32::WHITE));
                }
            }
            let tr = tit.intersect(carril);
            let pt = ui.painter_at(tr);
            icono(&pt, &tabler::WAVE_SINE, Rect::from_min_size(tr.min + vec2(5.0, 3.0), Vec2::splat(12.0)), Color32::WHITE);
            let mut nombre = q.nombre.clone();
            if (q.velocidad - 1.0).abs() > 1e-3 {
                let falta = !self.almacen.contains_key(&q.clave());
                nombre = crate::idioma::tf("[{} %{}] {}", &[&format!("{:.0}", q.velocidad * 100.0), &(if falta { "…" } else { "" }), &nombre]);
            }
            titulo_clip(ui, tr, 20.0, &nombre, Color32::WHITE);
            self.marco(&pr, rc, q.id, q.grupo);
            if self.elegido(q.id) {
                for x in [rc.left() + 6.0, rc.right() - 6.0] {
                    pr.circle_filled(pos2(x, rc.top() + 9.0), 4.0, MAIN);
                }
            }
            if bloqueada {
                pr.rect_filled(rc, 5, Color32::from_black_alpha(90));
            }
            let resp = ui.interact(rc.intersect(carril), Id::new(("clip", q.id)), Sense::click_and_drag());
            if resp.contains_pointer() {
                soltado_en = Some(q.id);
            }
            if !self.linea.auto_ver.contains_key(&id) {
                self.tocar_clip(ui, &resp, rc, q.id, true, bloqueada, a_t, ac);
            }
            if resp.clicked() || resp.drag_started() {
                self.canal = Canal::Pista(id);
            }
            if resp.double_clicked() {
                ac.push(Accion::Editar(q.id));
            }
            resp.context_menu(|ui| {
                if ui.button(crate::idioma::t("Editar audio")).clicked() {
                    ac.push(Accion::Editar(q.id));
                }
                if ui.button(crate::idioma::t("Dividir en el cursor")).clicked() {
                    ac.push(Accion::Dividir(q.id, self.ahora(), false));
                }
                if ui.button(crate::idioma::t("Duplicar")).clicked() {
                    ac.push(Accion::Duplicar(q.id));
                }
                if ui.button(crate::idioma::t("Separar voz e instrumental")).clicked() {
                    ac.push(Accion::Separar(q.id));
                }
                self.menu_vinculo(ui, q.id, q.grupo, ac);
                ui.separator();
                if ui.button(crate::idioma::t("Borrar")).clicked() {
                    ac.push(Accion::BorrarClip(q.id, false));
                }
                if ui.button(crate::idioma::t("Borrar y cerrar hueco")).clicked() {
                    ac.push(Accion::BorrarClip(q.id, true));
                }
            });
        }
        if let (Some(s), Some(m)) = (rfila.dnd_release_payload::<Soltar>(), ui.input(|i| i.pointer.interact_pos())) {
            ac.push(Accion::Soltado((*s).clone(), soltado_en, None, Some(id), a_t(m.x).max(0.0), false));
        }
        if let Some(g) = self.grab.as_ref().filter(|g| g.pista == id && g.arrancada) {
            let rc = Rect::from_min_max(pos2(a_x(g.inicio), r.top() + 3.0), pos2(a_x(self.ahora()), r.bottom() - 3.0));
            pr.rect_filled(rc, 5, RECORD.gamma_multiply(0.5));
        }
        self.auto_ui(ui, i, carril, a_x, a_t, ac);
        r.top()
    }

    /// Curva de automatización sobre la pista: clic pone un punto, arrastrar lo mueve, clic derecho lo quita.
    fn auto_ui(&mut self, ui: &mut Ui, i: usize, carril: Rect, a_x: &dyn Fn(f64) -> f32, a_t: &dyn Fn(f32) -> f64, ac: &mut Vec<Accion>) {
        let id = self.p.pistas[i].id;
        let Some(&k) = self.linea.auto_ver.get(&id) else { return };
        let Some(a) = self.p.pistas[i].auto.get(k).cloned() else {
            self.linea.auto_ver.remove(&id);
            return;
        };
        let (min, max, log) = rango(&self.p.pistas[i], &a.destino);
        let r = carril.shrink2(vec2(0.0, 6.0));
        let y_de = |v: f32| {
            let f = if log { (v / min).ln() / (max / min).ln() } else { (v - min) / (max - min) };
            r.bottom() - f.clamp(0.0, 1.0) * r.height()
        };
        let v_de = |y: f32| {
            let f = ((r.bottom() - y) / r.height()).clamp(0.0, 1.0);
            if log { min * (max / min).powf(f) } else { min + (max - min) * f }
        };
        let pr = ui.painter_at(carril);
        pr.rect_filled(carril, 0, Color32::from_black_alpha(70));
        let resp = ui.interact(carril, Id::new(("auto", id)), Sense::click_and_drag());
        let cerca = |m: Pos2| a.puntos.iter().position(|q| (a_x(q.0) - m.x).abs() < 8.0 && (y_de(q.1) - m.y).abs() < 10.0);
        let m0 = ui.input(|i| i.pointer.press_origin());
        let au = &mut self.p.pistas[i].auto[k];
        if let Some(m) = resp.interact_pointer_pos() {
            if resp.secondary_clicked() {
                if let Some(j) = cerca(m) {
                    au.puntos.remove(j);
                }
            } else if resp.clicked() && cerca(m).is_none() {
                au.puntos.push((a_t(m.x).max(0.0), v_de(m.y)));
                au.puntos.sort_by(|a, b| a.0.total_cmp(&b.0));
            } else if resp.drag_started_by(PointerButton::Primary) {
                let o = m0.unwrap_or(m);
                let j = cerca(o).unwrap_or_else(|| {
                    au.puntos.push((a_t(o.x).max(0.0), v_de(o.y)));
                    au.puntos.sort_by(|a, b| a.0.total_cmp(&b.0));
                    au.puntos.iter().position(|q| (a_x(q.0) - o.x).abs() < 1.0).unwrap_or(0)
                });
                self.linea.punto = Some((id, j));
            }
            if let (true, Some((_, j))) = (resp.dragged(), self.linea.punto.filter(|p| p.0 == id)) {
                // Un punto no brinca a sus vecinos.
                let izq = if j > 0 { au.puntos[j - 1].0 + 0.01 } else { 0.0 };
                let der = au.puntos.get(j + 1).map_or(f64::MAX, |q| q.0 - 0.01);
                if let Some(q) = au.puntos.get_mut(j) {
                    *q = (a_t(m.x).clamp(izq, der.max(izq)), v_de(m.y));
                }
            }
        }
        if resp.drag_stopped() {
            self.linea.punto = None;
        }
        let au = &self.p.pistas[i].auto[k];
        let col = if au.activa { MAIN } else { TEXT_FAINT };
        if let (Some(a), Some(z)) = (au.puntos.first(), au.puntos.last()) {
            let mut l = vec![pos2(carril.left(), y_de(a.1))];
            l.extend(au.puntos.iter().map(|q| pos2(a_x(q.0), y_de(q.1))));
            l.push(pos2(carril.right(), y_de(z.1)));
            pr.add(Shape::line(l, Stroke::new(2.0, col)));
        }
        for q in &au.puntos {
            pr.circle_filled(pos2(a_x(q.0), y_de(q.1)), 4.5, col);
            pr.circle_stroke(pos2(a_x(q.0), y_de(q.1)), 4.5, Stroke::new(1.0, COLORED_DARK));
        }
        // Etiqueta con encender, ocultar y borrar
        let et = Rect::from_min_size(carril.min + vec2(6.0, 4.0), vec2(250.0, 20.0));
        let nombre = nombre_auto(&self.p.pistas[i], &a.destino);
        let mut u = ui.new_child(UiBuilder::new().max_rect(et).layout(Layout::left_to_right(Align::Center)));
        let au = &mut self.p.pistas[i].auto[k];
        if led(&mut u, au.activa, "Encender la automatización").clicked() {
            au.activa = !au.activa;
        }
        u.label(RichText::new(crate::idioma::t(&nombre)).size(13.0).color(HIGHLIGHT));
        if boton_color(&mut u, &tabler::MINUS, false, "Ocultar la curva", TEXT_DIM, 18.0).clicked() {
            ac.push(Accion::AutoOcultar(id));
        }
        if boton_color(&mut u, &tabler::X, false, "Borrar la automatización", TEXT_DIM, 18.0).clicked() {
            ac.push(Accion::AutoBorrar(id));
        }
    }

    /// Para los patrones (`musica.rs`).
    pub fn tocar_patron(&mut self, ui: &Ui, resp: &Response, rc: Rect, id: u64, bloqueada: bool, a_t: &dyn Fn(f32) -> f64, ac: &mut Vec<Accion>) {
        self.tocar_clip(ui, resp, rc, id, false, bloqueada, a_t, ac);
    }
}

/// Pasa un clip a otra pista del mismo tipo.
fn cambiar_carril(p: &mut Proyecto, id: u64, c: Carril) {
    match c {
        Carril::Video(i) => {
            if !p.capas.get(i).is_some_and(|x| x.bloqueada) {
                if let Some(v) = p.clip_video(id) {
                    v.capa = i;
                }
            }
        }
        Carril::Audio(d) => {
            if p.pistas.iter().any(|x| x.id == d && x.bloqueada) {
                return;
            }
            let Some(i) = p.pistas.iter().position(|x| x.clips.iter().any(|c| c.id == id)) else { return };
            if p.pistas[i].id != d {
                let j = p.pistas[i].clips.iter().position(|c| c.id == id).unwrap();
                let c = p.pistas[i].clips.remove(j);
                if let Some(x) = p.pista(d) {
                    x.clips.push(c);
                }
            }
        }
    }
}

pub enum Accion {
    PistaNueva,
    PistaAudio,
    Importar,
    BorrarPista(u64),
    SubirPista(usize, i64),
    Armar(u64),
    /// clip, segundo, todas las pistas
    Dividir(u64, f64, bool),
    /// clip, cerrar el hueco
    BorrarClip(u64, bool),
    Duplicar(u64),
    Separar(u64),
    Editar(u64),
    CapaNueva,
    ClipVacio(f64, f64),
    BorrarCapa(usize),
    VerControles(u64),
    DuplicarVideo(u64),
    TraerAudio(u64),
    Congelar(u64),
    Animar(u64),
    Desvincular(u64),
    Vincular,
    Auto(u64, Destino),
    AutoOcultar(u64),
    AutoBorrar(u64),
    /// qué, clip, capa, pista de audio, segundo, mitad final del clip
    Soltado(Soltar, Option<u64>, Option<usize>, Option<u64>, f64, bool),
}

/// Cabecera de pista: placa con su clave y franja de color.
fn cabecera(ui: &Ui, r: Rect, clave: &str, elegida: bool) {
    let p = ui.painter();
    let caja = r.shrink2(vec2(2.0, 1.5));
    p.rect_filled(caja, 6, if elegida { c(0x444444) } else { c(0x343434) });
    let placa = Rect::from_min_max(caja.min, pos2(caja.left() + 42.0, caja.bottom()));
    // Sin color: la clave en gris, un poco más clara si está elegida.
    p.rect_filled(placa, CornerRadius { nw: 6, sw: 6, ne: 0, se: 0 }, if elegida { c(0x575757) } else { c(0x464646) });
    p.text(placa.center(), Align2::CENTER_CENTER, clave, barlow(17.0), if elegida { TEXT } else { TEXT_DIM });
    if elegida {
        p.rect_stroke(caja, 6, Stroke::new(1.5, MAIN), StrokeKind::Inside);
    }
}

fn candado(ui: &mut Ui, on: bool) -> Response {
    let (r, resp) = ui.allocate_exact_size(vec2(20.0, 18.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, on, "Bloquear"));
    let p = ui.painter();
    p.rect_filled(r, 4, if on { MAIN } else if resp.hovered() { BUTTON_HI } else { BUTTON });
    let col = if on { c(0x1c1c1c) } else { TEXT_DIM };
    let o = r.center();
    p.rect_filled(Rect::from_center_size(o + vec2(0.0, 2.0), vec2(9.0, 6.5)), 1, col);
    let a = if on { 0.0 } else { 2.5 };
    p.add(Shape::line(vec![o + vec2(-2.5 + a, -1.0), o + vec2(-2.5 + a, -4.0), o + vec2(-1.2 + a, -5.5), o + vec2(1.2 + a, -5.5), o + vec2(2.5 + a, -4.0), o + vec2(2.5 + a, if on { -1.0 } else { -3.0 })], Stroke::new(1.4, col)));
    resp.on_hover_text(crate::idioma::t("Bloquear: nada mueve ni corta esta pista"))
}

fn ojo(ui: &mut Ui, on: bool) -> Response {
    let (r, resp) = ui.allocate_exact_size(vec2(20.0, 18.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, on, "Mostrar"));
    let p = ui.painter();
    p.rect_filled(r, 4, if resp.hovered() { BUTTON_HI } else { BUTTON });
    let col = if on { MAIN } else { TEXT_FAINT };
    let o = r.center();
    let arco = |s: f32| -> Vec<Pos2> { (0..=8).map(|i| { let u = i as f32 / 8.0; o + vec2(-6.5 + 13.0 * u, s * 4.2 * (std::f32::consts::PI * u).sin()) }).collect() };
    p.add(Shape::line(arco(1.0), Stroke::new(1.3, col)));
    p.add(Shape::line(arco(-1.0), Stroke::new(1.3, col)));
    p.circle_filled(o, 2.0, col);
    if !on {
        p.line_segment([o + vec2(-6.0, 5.0), o + vec2(6.0, -5.0)], Stroke::new(1.5, TEXT_DIM));
    }
    resp.on_hover_text(crate::idioma::t("Mostrar u ocultar esta pista"))
}

/// Volumen en barra, en dB.
fn deslizador(ui: &mut Ui, v: &mut f32, n: &str) -> Response {
    let n = crate::idioma::t(n);
    let (r, resp) = ui.allocate_exact_size(vec2((ui.available_width() - 2.0).max(24.0), 18.0), Sense::click_and_drag());
    resp.widget_info(|| WidgetInfo::slider(true, *v as f64, n));
    let via = Rect::from_center_size(r.center(), vec2(r.width() - 8.0, 4.0));
    let a01 = |d: f32| ((d + 40.0) / 46.0).clamp(0.0, 1.0);
    if let (true, Some(m)) = (resp.dragged(), resp.interact_pointer_pos()) {
        *v = ((m.x - via.left()) / via.width()).clamp(0.0, 1.0) * 46.0 - 40.0;
        if v.abs() < 0.7 {
            *v = 0.0;
        }
    }
    if resp.double_clicked() {
        *v = 0.0;
    }
    let p = ui.painter();
    p.rect_filled(via, 2, c(0x1a1a1a));
    let x = via.left() + a01(*v) * via.width();
    p.rect_filled(Rect::from_min_max(via.min, pos2(x, via.bottom())), 2, DEEP);
    p.vline(via.left() + a01(0.0) * via.width(), r.center().y - 5.0..=r.center().y + 5.0, Stroke::new(1.0, c(0x666666)));
    p.circle_filled(pos2(x, r.center().y), 5.5, if resp.hovered() || resp.dragged() { HIGHLIGHT } else { c(0xd6d6d6) });
    resp
}

/// Nombre del clip, con puntos suspensivos si no cabe.
fn titulo_clip(ui: &Ui, r: Rect, x: f32, n: &str, col: Color32) {
    if r.width() < x + 14.0 {
        return;
    }
    let mut job = epaint::text::LayoutJob::simple_singleline(n.into(), fuente(13.0), col);
    job.wrap = epaint::text::TextWrapping { max_width: r.width() - x - 6.0, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    let g = ui.painter().layout_job(job);
    let y = r.center().y - g.size().y / 2.0;
    ui.painter_at(r).galley(pos2(r.left() + x, y), g, col);
}
