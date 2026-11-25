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
pub const HERRAMIENTAS: [(Herramienta, &iconos::Icono, egui::Key, &str); 11] = [
    (Herramienta::Flecha, &iconos::HAND_FINGER, egui::Key::V, "Selección (V): mover, recortar y elegir; Shift suma a la selección"),
    (Herramienta::Adelante, &iconos::ARROW_AUTOFIT_RIGHT, egui::Key::A, "Selección hacia adelante (A): mueve el clip y todo lo que sigue"),
    (Herramienta::Ondulado, &iconos::ARROWS_HORIZONTAL, egui::Key::B, "Recorte ondulado (B): recorta y recorre lo demás, sin huecos"),
    (Herramienta::Rodar, &iconos::BLEND_MODE, egui::Key::N, "Rodar (N): mueve el corte entre dos clips"),
    (Herramienta::Estirar, &iconos::STOPWATCH, egui::Key::X, "Estirar (X): cambia la velocidad jalando la orilla"),
    (Herramienta::Cuchilla, &iconos::SLICE, egui::Key::C, "Cuchilla (C): corta; con Shift corta todas las pistas"),
    (Herramienta::Deslizar, &iconos::ROTATE_CLOCKWISE_2, egui::Key::Y, "Deslizar (Y): cambia qué parte del archivo se ve, sin mover el clip"),
    (Herramienta::Correr, &iconos::REPEAT, egui::Key::U, "Correr (U): mueve el clip y ajusta a sus vecinos"),
    (Herramienta::Mano, &iconos::MAXIMIZE, egui::Key::H, "Mano (H): desplaza la vista"),
    (Herramienta::Lupa, &iconos::ZOOM_IN, egui::Key::Z, "Lupa (Z): acerca; con Alt aleja"),
    (Herramienta::Texto, &iconos::FILE_TYPOGRAPHY, egui::Key::T, "Texto (T): clic en una pista de video pone un título"),
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
    /// Llave que se está moviendo: clip y segundo.
    llave: Option<(u64, f64)>,
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
            llave: None,
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
const REGLA: f32 = 34.0;
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
    let mut cuerpo = Mesh::default();
    let fuerte = mezcla(col, Color32::WHITE, 0.45);
    let tenue = col.gamma_multiply(0.55);
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
        // Dos tonos: los picos tenues y el volumen promedio (RMS) brillante, como en las consolas.
        m.add_colored_rect(Rect::from_min_max(pos2(x, y0 - 0.5), pos2(x + 1.0, y1 + 0.5)), tenue);
        let q = a.rms[i..j.min(a.rms.len())].iter().fold(0f32, |u, v| u.max(*v));
        let k = ((q * g).min(1.0) * h).min((y1 - y0) / 2.0);
        if k > 0.3 {
            cuerpo.add_colored_rect(Rect::from_min_max(pos2(x, cy - k), pos2(x + 1.0, cy + k)), fuerte);
        }
        x += 1.0;
    }
    p.add(Shape::mesh(m));
    p.add(Shape::mesh(cuerpo));
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
        self.p.renombrar_capas();
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
        // La regla como la carátula de un reloj: acero oscuro, minutería de riel e índices aplicados, bajo un cristal.
        let barra = Rect::from_min_max(todo.min, regla.max);
        degradado(ui.painter(), barra, 0, c(0x363636), c(0x1c1c1c));
        ui.painter().hline(barra.x_range(), barra.top() + 0.5, Stroke::new(1.0, Color32::from_white_alpha(30)));
        ui.painter().hline(barra.x_range(), barra.bottom() + 1.0, Stroke::new(2.0, Color32::from_black_alpha(110)));
        {
            // La ventanilla del tiempo, biselada y con su lupa; y la esfera de segundos con su engrane, que avanza con el cursor.
            let p = ui.painter();
            let v = Rect::from_min_size(pos2(cab.left() + 6.0, cab.top() + 4.0), vec2(96.0, REGLA - 8.0));
            p.rect_filled(v.expand(1.5), 6, Color32::from_black_alpha(140));
            degradado(p, v.expand(1.0), 6, c(0x8a8a8a), c(0x4a4a4a));
            hueco(p, v.shrink(0.5), 5, c(0x0c0c0c));
            p.text(v.center(), Align2::CENTER_CENTER, tiempo(t), barlow(16.0), MAIN);
            degradado_en(p, Rect::from_min_max(v.min + vec2(1.5, 1.5), pos2(v.right() - 1.5, v.top() + v.height() * 0.48)), CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 }, Color32::from_white_alpha(46), Color32::from_white_alpha(10));
            let o = pos2(cab.right() - 22.0, cab.center().y);
            let (re, giro) = (REGLA / 2.0 - 4.0, (t * 0.8) as f32);
            engrane(p, o - vec2(re * 1.75, -1.0), re * 0.72, 16, -giro * 1.6, Color32::from_rgb(0x8c, 0x76, 0x35));
            p.circle_filled(o - vec2(re * 1.75, -1.0), 2.2, Color32::from_rgb(0xc2, 0x2f, 0x4e));
            p.circle_filled(o - vec2(re * 1.75 + 0.6, -0.2), 0.8, Color32::from_white_alpha(170));
            esfera(p, o, re, t, if self.grab.is_some() { RECORD } else { MAIN });
            // Un segundo engrane, más chico, que gira al revés; y el tornillo que los sujeta.
            engrane(p, o - vec2(re * 2.95, 1.5), re * 0.46, 11, giro * 2.5, Color32::from_rgb(0x84, 0x86, 0x8c));
            tornillo(p, o - vec2(re * 2.95, 1.5), 2.4, 0.7, true);
        }
        let pasos = [0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0];
        let paso = pasos.into_iter().find(|p| p * zoom >= 70.0).unwrap_or(600.0);
        let pr = ui.painter_at(Rect::from_min_max(regla.min, todo.max));
        pr.rect_filled(Rect::from_min_max(pos2(izq, regla.bottom()), todo.max), 0, GRID);
        // Rayitas entre las marcas grandes: 5, o 4 con la del medio más larga si el paso se parte en mitades.
        let sub = if [2.0, 60.0, 120.0].contains(&paso) || paso == 0.2 { 4.0 } else { 5.0 };
        let fps = self.p.fps.max(1.0);
        let mut k = (x0 / paso).floor();
        while a_x(k * paso) < todo.right() {
            let x = a_x(k * paso);
            if k as i64 % 2 == 0 {
                pr.rect_filled(Rect::from_min_max(pos2(x, regla.bottom()), pos2(a_x((k + 1.0) * paso), todo.bottom())), 0, GRID_ALT);
            }
            pr.vline(x, regla.bottom()..=todo.bottom(), Stroke::new(1.0, GRID_BAR));
            // Índice aplicado en la marca grande; entre marcas, la minutería: una más larga a la mitad y rayitas finas.
            indice(&pr, x, regla.bottom() - 13.0, regla.bottom() - 4.0, 3.0, c(0xd2d2d2));
            for j in 1..sub as i32 {
                let xs = a_x((k + j as f32 as f64 / sub) * paso);
                let largo = sub == 4.0 && j == 2;
                pr.vline(xs, regla.bottom() - if largo { 11.0 } else { 9.0 }..=regla.bottom() - 4.5, Stroke::new(if largo { 1.4 } else { 1.0 }, c(0xa8a8a8)));
            }
            if paso * zoom / sub >= 22.0 {
                for j in 0..(sub * 5.0) as i32 {
                    if j % 5 != 0 {
                        let xs = a_x((k + j as f64 / (sub * 5.0)) * paso);
                        pr.vline(xs, regla.bottom() - 7.0..=regla.bottom() - 4.5, Stroke::new(0.8, c(0x808080)));
                    }
                }
            }
            let s = k * paso;
            // Con mucho acercamiento, minutos:segundos:cuadro; si no, minutos:segundos.
            let txt = if paso < 1.0 {
                format!("{}:{:02}:{:02}", (s / 60.0) as u64, (s % 60.0).floor() as u64, ((s.fract() * fps).round() as u64).min(fps as u64 - 1))
            } else {
                format!("{}:{:02}", (s / 60.0) as u64, (s % 60.0).round() as u64)
            };
            pr.text(pos2(x + 5.0, regla.top() + 5.0), Align2::LEFT_TOP, &txt, barlow(12.5), Color32::from_black_alpha(160));
            pr.text(pos2(x + 4.0, regla.top() + 4.0), Align2::LEFT_TOP, txt, barlow(12.5), RULER_TEXT);
            k += 1.0;
        }
        // El riel de la minutería y el cristal de la carátula.
        for y in [regla.bottom() - 4.0, regla.bottom() - 9.5] {
            pr.hline(regla.x_range(), y, Stroke::new(0.7, Color32::from_white_alpha(38)));
        }
        degradado(ui.painter(), Rect::from_min_max(barra.min, pos2(barra.right(), barra.top() + REGLA * 0.42)), 0, Color32::from_white_alpha(22), Color32::from_white_alpha(4));
        // Barra de preparación, abajo de la regla: naranja lo que cuesta y aún no está hecho; verde lo listo para verse
        // fluido. Se va llenando sola en pausa.
        for (a, b, listo) in self.vista.preparado() {
            let (xa, xb) = (a_x(a).max(izq), a_x(b).min(todo.right()));
            if xb > xa {
                let r = Rect::from_min_max(pos2(xa, regla.bottom() - 3.0), pos2(xb, regla.bottom()));
                pr.rect_filled(r, 0, if listo { c(0x4caf6a) } else { c(0xe08a3c) });
            }
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
                // Una ranura con su filete dorado.
                degradado(ui.painter(), s, 0, c(0x121212), c(0x262626));
                ui.painter().hline(s.x_range(), s.center().y - 0.5, Stroke::new(1.2, mezcla(DEEP, Color32::BLACK, 0.35)));
                ui.painter().hline(s.x_range(), s.center().y - 1.3, Stroke::new(0.6, HIGHLIGHT.gamma_multiply(0.5)));
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
                    Soltar::Letra(n) => self.titulo_con_letra(&n.clone(), t, None, None),
                    Soltar::Medio(r) => self.poner_medio(&r.clone(), t, 0, None, false),
                    Soltar::Medios(v) => self.poner_archivos(v, t, None, None),
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
            // Guía al pegar un clip con otro: amarilla y con resplandor.
            pr.vline(a_x(g), regla.bottom()..=todo.bottom(), Stroke::new(5.0, MAIN.gamma_multiply(0.18)));
            pr.vline(a_x(g), regla.bottom()..=todo.bottom(), Stroke::new(1.5, HIGHLIGHT));
        }
        // Línea fantasma donde está el ratón sobre la regla: ahí cae el cursor si haces clic.
        if let Some(m) = rr.hover_pos().filter(|_| !rr.dragged()) {
            pr.vline(m.x, regla.top()..=todo.bottom(), Stroke::new(1.0, Color32::from_white_alpha(45)));
        }
        let x = a_x(t);
        if x >= izq - 1.0 {
            let col = if self.grab.is_some() { RECORD } else { MAIN };
            // Línea con un poco de resplandor.
            pr.vline(x, regla.bottom()..=todo.bottom(), Stroke::new(4.0, col.gamma_multiply(0.18)));
            pr.vline(x, regla.top() + 16.0..=todo.bottom(), Stroke::new(1.5, col));
            // Cabeza como manecilla facetada: un lado en luz y el otro en sombra, con un rubí.
            let (y0, y1, y2) = (regla.top() + 2.0, regla.top() + 12.0, regla.bottom() + 1.0);
            let cabeza = [pos2(x - 7.0, y0), pos2(x + 7.0, y0), pos2(x + 7.0, y1), pos2(x, y2), pos2(x - 7.0, y1)];
            pr.add(Shape::convex_polygon(cabeza.iter().map(|q| *q + vec2(0.0, 1.5)).collect(), Color32::from_black_alpha(120), Stroke::NONE));
            pr.add(Shape::convex_polygon(vec![cabeza[0], pos2(x, y0), cabeza[3], cabeza[4]], mezcla(col, Color32::WHITE, 0.35), Stroke::NONE));
            pr.add(Shape::convex_polygon(vec![pos2(x, y0), cabeza[1], cabeza[2], cabeza[3]], mezcla(col, Color32::BLACK, 0.28), Stroke::NONE));
            pr.add(Shape::closed_line(cabeza.to_vec(), Stroke::new(1.0, mezcla(col, Color32::BLACK, 0.62))));
            let rubi = pos2(x, y0 + 5.5);
            pr.circle_filled(rubi, 3.4, mezcla(col, Color32::BLACK, 0.6));
            pr.circle_filled(rubi, 2.7, Color32::from_rgb(0x9c, 0x14, 0x36));
            pr.circle_filled(rubi - vec2(0.3, 0.4), 1.7, Color32::from_rgb(0xe0, 0x3a, 0x62));
            pr.circle_filled(rubi - vec2(0.9, 1.0), 0.8, Color32::from_white_alpha(210));
            // Al arrastrarlo, el tiempo exacto al lado.
            if rr.dragged() || rr.is_pointer_button_down_on() {
                let s = t.max(0.0);
                let txt = format!("{}:{:02}:{:02}", (s / 60.0) as u64, (s % 60.0).floor() as u64, (s.fract() * fps).floor() as u64);
                let g = pr.layout_no_wrap(txt, barlow(13.0), COLORED_DARK);
                let caja = Rect::from_min_size(pos2(x + 11.0, regla.top() + 3.0), g.size() + vec2(10.0, 4.0));
                encendido(&pr, caja, 5, col);
                pr.galley(caja.min + vec2(5.0, 2.0), g, COLORED_DARK);
            }
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
        hueco(&p, mapa, 5, WELL);
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
        // Lo que se ve en la línea de tiempo, en píxeles y en segundos.
        let px_vis = (mapa.width() + 290.0) as f64;
        let vis = px_vis / self.linea.zoom;
        let v = Rect::from_min_max(pos2(x(self.linea.x0), mapa.top()), pos2(x((self.linea.x0 + vis).min(fin)).max(x(self.linea.x0) + 8.0), mapa.bottom()));
        // La parte que se ve, bajo una lupa de vidrio.
        p.rect_filled(v, 4, Color32::from_white_alpha(20));
        vidrio(&p, v, 4, MAIN, false);
        p.rect_stroke(v, 4, Stroke::new(1.0, MAIN.gamma_multiply(0.8)), StrokeKind::Inside);
        p.vline(x(self.ahora()), mapa.y_range(), Stroke::new(1.0, MAIN));
        let t_de = |px: f32| ((px - mapa.left()) / mapa.width()) as f64 * fin;
        // Fuera del recuadro: clic lo centra ahí. Dentro: arrastrar lo desplaza.
        let r = ui.interact(mapa, Id::new("mapa"), Sense::click_and_drag());
        if let (true, Some(m)) = (r.clicked() || (r.drag_started() && !v.contains(m_o(&r))), r.interact_pointer_pos()) {
            self.linea.x0 = (t_de(m.x) - vis / 2.0).max(0.0);
        } else if r.dragged() {
            self.linea.x0 = (self.linea.x0 + (r.drag_delta().x / mapa.width()) as f64 * fin).max(0.0);
        }
        // Orillas del recuadro: estirarlas acerca o aleja, dejando fija la otra orilla.
        for (izq, o) in [(true, v.left()), (false, v.right())] {
            let zona = Rect::from_center_size(pos2(o, v.center().y), vec2(10.0, v.height()));
            let ro = ui.interact(zona, Id::new(("mapa-orilla", izq)), Sense::drag());
            ro.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, crate::idioma::t(if izq { "Orilla izquierda: acercar o alejar" } else { "Orilla derecha: acercar o alejar" })));
            let activa = ro.hovered() || ro.dragged();
            if activa {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
            // Agarradera de metal con dos rayitas.
            let g = Rect::from_center_size(pos2(o, v.center().y), vec2(6.0, (v.height() - 6.0).min(16.0)));
            relieve(&p, g, 2, if activa { HIGHLIGHT } else { c(0x9a9a9a) }, false, false);
            for k in [-1.5f32, 1.5] {
                p.vline(g.center().x + k, g.center().y - 3.0..=g.center().y + 3.0, Stroke::new(1.0, Color32::from_black_alpha(140)));
            }
            if let (true, Some(m)) = (ro.dragged(), ro.interact_pointer_pos()) {
                let (a, b) = (self.linea.x0, self.linea.x0 + vis);
                let (a, b) = if izq { (t_de(m.x).clamp(0.0, b - 0.2), b) } else { (a, t_de(m.x).max(a + 0.2)) };
                self.linea.zoom = (px_vis / (b - a)).clamp(0.5, 1000.0);
                self.linea.x0 = if izq { (b - px_vis / self.linea.zoom).max(0.0) } else { a };
            }
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
                    self.p.renumerar(&mut d);
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
            Soltar::Letra(n) => self.titulo_con_letra(&n, t, capa, None),
            Soltar::Filtro(i) => {
                let v = clip.and_then(|id| pareja(&self.p, id, true)).map(|x| x.id);
                self.poner_filtro(v, i);
            }
            Soltar::Estilo(i) => {
                let v = clip.and_then(|id| pareja(&self.p, id, true)).map(|x| x.id);
                self.poner_estilo(v, i);
            }
            Soltar::Medios(v) => self.poner_archivos(&v, t, capa, pista),
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

    /// Línea de volumen dentro del clip de audio: el centro es 0 dB; arrastrarla sube o baja el volumen (Shift: fino) y
    /// doble clic la regresa a 0 dB.
    fn linea_volumen(&mut self, ui: &Ui, pr: &Painter, rc: Rect, cuerpo: Rect, carril: Rect, q: &Clip) {
        let alto = cuerpo.height() - 8.0;
        if alto < 10.0 || rc.width() < 40.0 {
            return;
        }
        let y_de = |g: f32| cuerpo.bottom() - 4.0 - alto * ((g + 24.0) / 48.0).clamp(0.0, 1.0);
        let y = y_de(q.ganancia_db);
        let zona = Rect::from_min_max(pos2(rc.left() + 14.0, y - 4.0), pos2(rc.right() - 14.0, y + 4.0)).intersect(carril);
        if zona.width() <= 0.0 {
            return;
        }
        let rv = ui.interact(zona, Id::new(("vol-clip", q.id)), Sense::click_and_drag());
        rv.widget_info(|| WidgetInfo::slider(true, q.ganancia_db as f64, crate::idioma::t("Volumen del clip")));
        let mut g = q.ganancia_db;
        if rv.dragged() {
            let fino = if ui.input(|i| i.modifiers.shift) { 0.2 } else { 1.0 };
            // Un cuarto de dB por píxel: fino aunque el clip sea bajito.
            g = (g - rv.drag_delta().y * 0.25 * fino).clamp(-24.0, 24.0);
            // Se pega a 0 dB al pasar cerca.
            if g.abs() < 0.4 && fino == 1.0 {
                g = 0.0;
            }
        }
        if rv.double_clicked() {
            g = 0.0;
        }
        if g != q.ganancia_db {
            if let Some(k) = self.p.clip(q.id) {
                k.ganancia_db = g;
            }
        }
        let activo = rv.hovered() || rv.dragged();
        if activo {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
        }
        let y = y_de(g);
        let (x0, x1) = (rc.left().max(carril.left()) + 3.0, rc.right().min(carril.right()) - 3.0);
        let fuerte = activo || self.elegido(q.id);
        pr.hline(x0..=x1, y + 1.0, Stroke::new(1.0, Color32::from_black_alpha(if fuerte { 160 } else { 110 })));
        pr.hline(x0..=x1, y, Stroke::new(if fuerte { 1.8 } else { 1.2 }, Color32::from_white_alpha(if fuerte { 240 } else { 160 })));
        if activo {
            let m = rv.hover_pos().map_or((x0 + x1) / 2.0, |m| m.x).clamp(x0 + 30.0, (x1 - 30.0).max(x0 + 30.0));
            pr.circle_filled(pos2(m, y), 4.0, Color32::WHITE);
            pr.circle_stroke(pos2(m, y), 4.0, Stroke::new(1.0, Color32::from_black_alpha(160)));
            let txt = format!("{:+.1} dB", g);
            let arriba = if y - cuerpo.top() > 16.0 { -10.0 } else { 10.0 };
            // Al lado del cursor, para que no la tape.
            let o = pos2(m + 34.0, y + arriba);
            let gal = pr.layout_no_wrap(txt, barlow(12.0), Color32::WHITE);
            let caja = Rect::from_center_size(o, gal.size() + vec2(10.0, 2.0));
            pr.rect_filled(caja, 4, Color32::from_black_alpha(190));
            pr.galley(caja.center() - gal.size() / 2.0, gal, Color32::WHITE);
        }
    }

    /// La orilla de abajo de una cabecera: arrastrarla cambia el alto de las pistas.
    fn tirador_alto(&mut self, ui: &Ui, cab: Rect, id: (&str, u64)) {
        let zona = Rect::from_min_max(pos2(cab.left(), cab.bottom() - 3.0), pos2(cab.right(), cab.bottom() + 2.0));
        let r = ui.interact(zona, Id::new(id), Sense::drag());
        if r.hovered() || r.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
            ui.painter().hline(cab.left() + 8.0..=cab.right() - 8.0, cab.bottom() - 1.0, Stroke::new(2.0, MAIN.gamma_multiply(0.7)));
        }
        if r.dragged() {
            self.linea.alto = (self.linea.alto + r.drag_delta().y).clamp(44.0, 130.0);
        }
    }

    fn elegido(&self, id: u64) -> bool {
        self.seleccion.contains(&id)
    }

    /// Marco del clip: elegido, o pareja de uno elegido.
    fn marco(&self, pr: &Painter, rc: Rect, id: u64, grupo: Option<u64>) {
        if self.elegido(id) {
            for (e, a) in [(3.0, 0.10), (1.5, 0.20)] {
                pr.rect_stroke(rc.expand(e), 6 + e as u8, Stroke::new(1.5, MAIN.gamma_multiply(a)), StrokeKind::Middle);
            }
            pr.rect_stroke(rc, 6, Stroke::new(2.0, MAIN), StrokeKind::Inside);
            pr.rect_stroke(rc.shrink(2.0), 5, Stroke::new(1.0, HIGHLIGHT.gamma_multiply(0.35)), StrokeKind::Inside);
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
        cabecera(ui, cab, &format!("V{}", capa + 1), false, c(0xc9a92c));
        self.tirador_alto(ui, cab, ("alto-v", capa as u64));
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
        // Carril metido: sombra arriba, como la ranura de una platina.
        degradado(&pr, Rect::from_min_size(carril.min, vec2(carril.width(), 6.0)), 0, Color32::from_black_alpha(70), Color32::from_black_alpha(0));
        pr.hline(carril.x_range(), carril.bottom() - 0.5, Stroke::new(1.0, c(0x161616)));
        // Filo de luz debajo de la ranura: separa las pistas como un tablero.
        pr.hline(carril.x_range(), carril.bottom() + 0.5, Stroke::new(1.0, Color32::from_white_alpha(7)));
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
            let tinte = v.color.map(|[r, g, b]| Color32::from_rgb(r, g, b));
            let (fondo, cinta) = tinte.map_or((fondo, cinta), |x| (mezcla(x, c(0x141414), 0.55), mezcla(x, c(0x202020), 0.25)));
            clip_fondo(&pr, rc, fondo);
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
            if let Some(x) = tinte {
                pr.rect_filled(cuerpo, CornerRadius { nw: 0, ne: 0, sw: 6, se: 6 }, x.gamma_multiply(0.3));
            }
            pr.rect_filled(tit, CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 }, translucido(cinta, 215));
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
            let llaves = crate::animaciones::momentos(&v);
            let sitio = |tl: f64| pos2(a_x(v.inicio + tl), cuerpo.bottom() - 7.0);
            for (tl, que) in llaves.iter().filter(|k| k.0 >= 0.0 && k.0 <= v.dur) {
                libre::pinta_rombo(&pr, sitio(*tl), 4.5, libre::resumen(que).1, COLORED_DARK);
            }
            let tr = tit.intersect(carril);
            let nombre = match &v.contenido {
                Contenido::Titulo(t) => t.texto.lines().next().unwrap_or("Título").to_string(),
                Contenido::Color(_) => "Color".into(),
                Contenido::Ajuste => crate::idioma::t("Clip vacío: efectos para lo de abajo").into(),
                Contenido::Forma(f) => crate::idioma::t(TipoForma::TODOS.iter().find(|x| x.0 == f.tipo).map_or("Figura", |x| x.1)).into(),
                _ => nombre_de(&v.ruta),
            };
            // Lo que tiene el clip, en etiquetas de colores a la derecha del nombre.
            let nfx = v.fx.iter().filter(|f| f.activo).count();
            let mut tags: Vec<(String, Color32)> = vec![];
            let clase = v.clase.and_then(|k| self.p.clases.iter().find(|x| x.id == k)).cloned();
            if let Some(k) = &clase {
                let [r, g, b] = k.color;
                tags.push((k.nombre.clone(), mezcla(Color32::from_rgb(r, g, b), Color32::WHITE, 0.35)));
            }
            if nfx > 0 {
                tags.push((crate::idioma::tf("FX {}", &[&nfx]), TAG_FX));
            }
            if v.nodos.as_ref().is_some_and(|g| g.activo) {
                tags.push((crate::idioma::t("Nodos").into(), TAG_NODOS));
            }
            if v.animado || !llaves.is_empty() {
                tags.push((crate::idioma::t("Animado").into(), TAG_ANIM));
            }
            if v.entrada.is_some() || v.salida.is_some() {
                tags.push((crate::idioma::t("Transición").into(), TAG_TRANS));
            }
            if (m.velocidad - 1.0).abs() > 1e-3 {
                tags.push((format!("{:.0} %", m.velocidad * 100.0), TAG_VEL));
            }
            let tr = etiquetas(&pr, tr, &tags);
            titulo_clip(ui, tr, 6.0, &nombre, c(0xfff6d8));
            clip_cristal(&pr, rc, cinta);
            self.marco(&pr, rc, v.id, v.grupo);
            if bloqueada {
                pr.rect_filled(rc, 6, Color32::from_black_alpha(90));
            }
            let resp = ui.interact(rc.intersect(carril), Id::new(("video", v.id)), Sense::click_and_drag());
            resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, crate::idioma::tf("Clip de video {}", &[&i])));
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
            // Las llaves del clip elegido: clic va a ella, se arrastra en el tiempo y clic derecho la cambia.
            if self.clip == Some(v.id) && !bloqueada {
                self.llaves_en_clip(ui, &v, &llaves, carril, &sitio, a_t);
            }
            let k = Id::new("menu-llave-t");
            if !resp.context_menu_opened() {
                ui.data_mut(|d| d.remove::<f64>(k));
            }
            resp.context_menu(|ui| {
                let donde = ui.input(|i| i.pointer.press_origin().or(i.pointer.interact_pos())).map_or(self.ahora(), |m| a_t(m.x).clamp(v.inicio, v.fin()));
                let donde = ui.data_mut(|d| *d.get_temp_mut_or(k, donde));
                ui.menu_button(RichText::new(format!("◆ {}", crate::idioma::t("Poner llave aquí"))).color(MAIN), |ui| {
                    ui.set_min_width(210.0);
                    self.menu_llaves(ui, v.id, donde);
                });
                ui.menu_button(crate::idioma::t("Acomodar en el cuadro"), |ui| {
                    for (i, n) in ["Completo", "Llenar el cuadro", "Estirar al cuadro"].into_iter().enumerate() {
                        if ui.button(crate::idioma::t(n)).clicked() {
                            self.acomodar(v.id, i);
                        }
                    }
                });
                let n = clase.as_ref().map_or(crate::idioma::t("Agregar a una clase").to_string(), |k| crate::idioma::tf("Clase «{}»", &[&k.nombre]));
                ui.menu_button(n, |ui| self.menu_clase(ui, v.id));
                ui.menu_button(crate::idioma::t("Color del clip"), |ui| self.menu_color(ui, v.id));
                ui.separator();
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
        self.fantasma(ui, rcar.contains_pointer(), carril, a_x, a_t);
        if let (Some(s), Some(m)) = (rcar.dnd_release_payload::<Soltar>(), ui.input(|i| i.pointer.interact_pos())) {
            let t = a_t(m.x).max(0.0);
            let final_ = soltado_en.and_then(|id| self.p.video.iter().find(|c| c.id == id)).is_some_and(|c| t > c.inicio + c.dur / 2.0);
            ac.push(Accion::Soltado((*s).clone(), soltado_en, Some(capa), None, t, final_));
        }
        r.top()
    }

    fn llaves_en_clip(&mut self, ui: &mut Ui, v: &ClipVideo, llaves: &[(f64, Vec<crate::animaciones::Pista>)], carril: Rect, sitio: &dyn Fn(f64) -> Pos2, a_t: &dyn Fn(f32) -> f64) {
        let moviendo = self.linea.llave.filter(|l| l.0 == v.id);
        for (i, (tl, que)) in llaves.iter().enumerate() {
            let o = sitio(*tl);
            let zona = Rect::from_center_size(o, vec2(13.0, 14.0)).intersect(carril);
            if zona.width() <= 0.0 {
                continue;
            }
            let r = ui.interact(zona, Id::new(("llave", v.id, i)), Sense::click_and_drag());
            let (nombre, col) = libre::resumen(que);
            r.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, crate::idioma::tf("Llave: {}", &[&nombre])));
            if r.hovered() || moviendo.is_some_and(|l| (l.1 - tl).abs() < 0.02) {
                libre::pinta_rombo(&ui.painter_at(carril), o, 7.0, col, Color32::WHITE);
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
            if r.clicked() {
                self.ir(v.inicio + tl);
            }
            if r.drag_started() {
                self.linea.llave = Some((v.id, *tl));
            }
            r.context_menu(|ui| {
                ui.set_min_width(210.0);
                self.menu_llaves(ui, v.id, v.inicio + tl);
            });
            r.on_hover_text(crate::idioma::tf("{} en {}. Arrástrala para moverla; clic derecho: más", &[&nombre, &tiempo(v.inicio + tl)]));
        }
        if let Some((id, de)) = moviendo {
            let (m, abajo) = ui.input(|i| (i.pointer.interact_pos(), i.pointer.primary_down()));
            if !abajo {
                self.linea.llave = None;
            } else if let Some(m) = m {
                let a = self.a_cuadro(a_t(m.x) - v.inicio).clamp(0.0, v.dur);
                if let Some(c) = self.p.clip_video(id).filter(|_| (a - de).abs() > 1e-6) {
                    if crate::animaciones::mover_momento(c, de, a) {
                        self.linea.llave = Some((id, a));
                    }
                }
            }
        }
    }

    /// Dónde quedaría lo que se arrastra desde el panel de archivos.
    fn fantasma(&self, ui: &Ui, encima: bool, carril: Rect, a_x: &dyn Fn(f64) -> f32, a_t: &dyn Fn(f32) -> f64) {
        let Some(s) = DragAndDrop::payload::<Soltar>(ui.ctx()).filter(|_| encima) else { return };
        let (Soltar::Medios(v), Some(m)) = (&*s, ui.input(|i| i.pointer.interact_pos())) else { return };
        let t = a_t(m.x).max(0.0);
        let dur: f64 = v.iter().map(|r| self.archivos.dur(r).unwrap_or(5.0)).sum();
        let r = Rect::from_min_max(pos2(a_x(t), carril.top() + 3.0), pos2(a_x(t + dur), carril.bottom() - 3.0));
        let p = ui.painter_at(carril);
        p.rect(r, 5, MAIN.gamma_multiply(0.25), Stroke::new(1.5, MAIN), StrokeKind::Inside);
        p.text(pos2(r.left() + 6.0, r.top() + 4.0), Align2::LEFT_TOP, tiempo(t), fuente(12.5), TEXT);
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
            ui.menu_button(crate::idioma::t("Salida al mezclador"), |ui| self.menu_salida(ui, id));
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
        cabecera(ui, cab, &format!("A{}", i + 1), elegida, color_pista(i));
        self.tirador_alto(ui, cab, ("alto-a", id));
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
        // Carril metido: sombra arriba, como la ranura de una platina.
        degradado(&pr, Rect::from_min_size(carril.min, vec2(carril.width(), 6.0)), 0, Color32::from_black_alpha(70), Color32::from_black_alpha(0));
        pr.hline(carril.x_range(), carril.bottom() - 0.5, Stroke::new(1.0, c(0x161616)));
        // Filo de luz debajo de la ranura: separa las pistas como un tablero.
        pr.hline(carril.x_range(), carril.bottom() + 0.5, Stroke::new(1.0, Color32::from_white_alpha(7)));
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
            let col = q.color.map_or(col, |[r, g, b]| Color32::from_rgb(r, g, b));
            let gris = |x: Color32| if mudo { mezcla(x, c(0x404040), 0.75) } else { x };
            clip_fondo(&pr, rc, gris(mezcla(col, c(0x141414), 0.62)));
            let tit = Rect::from_min_size(rc.min, vec2(rc.width(), 18.0));
            pr.rect_filled(tit, CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 }, translucido(gris(mezcla(col, c(0x303030), 0.3)), 215));
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
            icono(&pt, &iconos::WAVE_SINE, Rect::from_min_size(tr.min + vec2(5.0, 3.0), Vec2::splat(12.0)), Color32::WHITE);
            let mut tags: Vec<(String, Color32)> = vec![];
            if let Some(b) = q.bus.and_then(|b| self.p.bus(b)) {
                tags.push((format!("→ {}", b.nombre), TAG_FX));
            }
            if q.ganancia_db.abs() > 0.05 {
                tags.push((format!("{:+.1} dB", q.ganancia_db), TAG_VOL));
            }
            if q.invertido {
                tags.push((crate::idioma::t("Al revés").into(), TAG_REV));
            }
            if (q.velocidad - 1.0).abs() > 1e-3 {
                let falta = !self.almacen.contains_key(&q.clave());
                tags.push((format!("{:.0} %{}", q.velocidad * 100.0, if falta { "…" } else { "" }), TAG_VEL));
            }
            let tr = etiquetas(&pr, tr, &tags);
            titulo_clip(ui, tr, 20.0, &q.nombre, Color32::WHITE);
            clip_cristal(&pr, rc, col);
            self.marco(&pr, rc, q.id, q.grupo);
            if self.elegido(q.id) {
                for x in [rc.left() + 6.0, rc.right() - 6.0] {
                    pr.circle_filled(pos2(x, rc.top() + 9.0), 4.0, MAIN);
                }
            }
            if bloqueada {
                pr.rect_filled(rc, 6, Color32::from_black_alpha(90));
            }
            let resp = ui.interact(rc.intersect(carril), Id::new(("clip", q.id)), Sense::click_and_drag());
            resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &q.nombre));
            if resp.contains_pointer() {
                soltado_en = Some(q.id);
            }
            if !self.linea.auto_ver.contains_key(&id) {
                self.tocar_clip(ui, &resp, rc, q.id, true, bloqueada, a_t, ac);
                if !bloqueada && self.linea.herramienta == Herramienta::Flecha {
                    self.linea_volumen(ui, &pr, rc, cuerpo, carril, &q);
                }
            }
            if resp.clicked() || resp.drag_started() {
                self.canal_de_clip(q.id);
            }
            if resp.double_clicked() {
                ac.push(Accion::Editar(q.id));
            }
            resp.context_menu(|ui| {
                // Volumen del clip, a la mano.
                ui.label(RichText::new(crate::idioma::t("Volumen del clip")).size(12.5).color(TEXT_DIM));
                let mut g = q.ganancia_db;
                let mut cambia = ui.add(Slider::new(&mut g, -24.0..=24.0).suffix(" dB").fixed_decimals(1)).changed();
                ui.horizontal(|ui| {
                    for (n, x) in [("−6 dB", -6.0), ("−3 dB", -3.0), ("0 dB", 0.0), ("+3 dB", 3.0), ("+6 dB", 6.0)] {
                        if ui.small_button(n).clicked() {
                            (g, cambia) = (x, true);
                        }
                    }
                });
                if cambia {
                    if let Some(k) = self.p.clip(q.id) {
                        k.ganancia_db = g;
                    }
                }
                ui.separator();
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
                ui.menu_button(crate::idioma::t("Mandar al mezclador"), |ui| self.menu_bus_clip(ui, q.id));
                ui.menu_button(crate::idioma::t("Color del clip"), |ui| self.menu_color(ui, q.id));
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
        self.fantasma(ui, rfila.contains_pointer(), carril, a_x, a_t);
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
        if boton_color(&mut u, &iconos::MINUS, false, "Ocultar la curva", TEXT_DIM, 18.0).clicked() {
            ac.push(Accion::AutoOcultar(id));
        }
        if boton_color(&mut u, &iconos::X, false, "Borrar la automatización", TEXT_DIM, 18.0).clicked() {
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
/// Cabecera de pista: placa de metal con una franja del color de la pista y la clave grabada.
fn cabecera(ui: &Ui, r: Rect, clave: &str, elegida: bool, col: Color32) {
    let p = ui.painter();
    let caja = r.shrink2(vec2(2.0, 1.5));
    // Placa de metal con la clave grabada en un hueco.
    relieve(p, caja, 6, if elegida { c(0x444444) } else { c(0x343434) }, false, false);
    // Franja del color de la pista, pegada a la orilla izquierda.
    let franja = Rect::from_min_max(caja.min + vec2(1.0, 1.0), pos2(caja.left() + 5.0, caja.bottom() - 1.0));
    degradado_en(p, franja, CornerRadius { nw: 5, sw: 5, ne: 0, se: 0 }, mezcla(col, Color32::WHITE, 0.25), mezcla(col, Color32::BLACK, 0.2));
    // Vidrio sobre la mitad de arriba de la placa.
    degradado_en(p, Rect::from_min_max(caja.min + vec2(5.0, 1.0), pos2(caja.right() - 1.0, caja.top() + caja.height() * 0.44)), CornerRadius { nw: 0, ne: 5, sw: 0, se: 0 }, Color32::from_white_alpha(30), Color32::from_white_alpha(7));
    let placa = Rect::from_min_max(caja.min + vec2(8.0, 4.0), pos2(caja.left() + 40.0, caja.bottom() - 4.0));
    // Marco pulido de la clave, con sus tornillos azulados.
    degradado(p, placa.expand(0.8), 6, c(0x747474), c(0x404040));
    hueco(p, placa.shrink(0.3), 5, if elegida { c(0x2a2a2a) } else { c(0x1c1c1c) });
    if placa.height() > 44.0 {
        for (o, g) in [(pos2(placa.left() + 5.5, placa.top() + 5.5), 0.5), (pos2(placa.right() - 5.5, placa.top() + 5.5), 2.1), (pos2(placa.left() + 5.5, placa.bottom() - 5.5), 1.3), (pos2(placa.right() - 5.5, placa.bottom() - 5.5), 2.8)] {
            tornillo(p, o, 2.2, g, true);
        }
    }
    p.text(placa.center() + vec2(0.0, 1.0), Align2::CENTER_CENTER, clave, barlow(17.0), Color32::from_black_alpha(160));
    p.text(placa.center(), Align2::CENTER_CENTER, clave, barlow(17.0), if elegida { MAIN } else { TEXT_DIM });
    if elegida {
        p.rect_stroke(caja, 6, Stroke::new(1.5, MAIN), StrokeKind::Inside);
    }
}

fn candado(ui: &mut Ui, on: bool) -> Response {
    let (r, resp) = ui.allocate_exact_size(vec2(20.0, 18.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, on, "Bloquear"));
    let p = ui.painter();
    if on { encendido(p, r, 4, MAIN) } else { relieve(p, r, 4, BUTTON, resp.hovered(), resp.is_pointer_button_down_on()) }
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
    relieve(p, r, 4, BUTTON, resp.hovered(), resp.is_pointer_button_down_on());
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
    hueco(p, via, 2, c(0x111111));
    let x = via.left() + a01(*v) * via.width();
    p.rect_filled(Rect::from_min_max(via.min, pos2(x, via.bottom())), 2, DEEP);
    p.vline(via.left() + a01(0.0) * via.width(), r.center().y - 5.0..=r.center().y + 5.0, Stroke::new(1.0, c(0x666666)));
    // Botón redondo con sombra y brillo.
    let o = pos2(x, r.center().y);
    let claro = if resp.hovered() || resp.dragged() { HIGHLIGHT } else { c(0xe2e2e2) };
    p.circle_filled(o + vec2(0.0, 1.2), 6.2, Color32::from_black_alpha(120));
    p.circle_filled(o, 6.0, mezcla(claro, Color32::BLACK, 0.35));
    p.circle_filled(o - vec2(0.0, 0.8), 5.0, claro);
    p.circle_filled(o - vec2(1.5, 2.0), 1.8, Color32::from_white_alpha(140));
    resp
}

// Colores de las etiquetas de los clips.
const TAG_FX: Color32 = c(0xb48cff);
const TAG_NODOS: Color32 = c(0x5fd3c4);
const TAG_ANIM: Color32 = c(0x4fc3f7);
const TAG_TRANS: Color32 = c(0xffd54f);
const TAG_VEL: Color32 = c(0xffa64d);
const TAG_VOL: Color32 = c(0x8bd17c);
const TAG_REV: Color32 = c(0xff8a80);

/// Dónde empezó el clic o arrastre de `r` (o el ratón ahora, si no se sabe).
fn m_o(r: &Response) -> Pos2 {
    r.ctx.input(|i| i.pointer.press_origin()).or(r.interact_pointer_pos()).unwrap_or(r.rect.center())
}

fn translucido(x: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(x.r(), x.g(), x.b(), a)
}

/// Debajo del contenido del clip: su sombra y el cuerpo de su color, de claro a oscuro.
fn clip_fondo(p: &Painter, rc: Rect, fondo: Color32) {
    p.rect_filled(rc.translate(vec2(0.0, 2.0)), 6, Color32::from_black_alpha(70));
    degradado(p, rc, 6, mezcla(fondo, Color32::WHITE, 0.1), mezcla(fondo, Color32::BLACK, 0.28));
}

/// Encima del contenido: un vidrio grueso, con su brillo, su luz de abajo y dos destellos en diagonal.
fn clip_cristal(p: &Painter, rc: Rect, col: Color32) {
    // El brillo termina donde la cinta del nombre: un solo filo.
    vidrio_en(p, rc, 6, col, false, 18.0);
    // Destellos: dos franjas inclinadas, como la luz de una ventana en el cristal.
    if rc.width() > 90.0 {
        let (h, sesgo) = (rc.height(), rc.height() * 0.55);
        for (x, w, a) in [(0.10, 0.5, 13u8), (0.10 + 0.62 * h / rc.width(), 0.16, 9)] {
            let x0 = rc.left() + rc.width().min(420.0) * x + sesgo;
            let w = h * w;
            let q = vec![pos2(x0, rc.top() + 1.0), pos2(x0 + w, rc.top() + 1.0), pos2(x0 + w - sesgo, rc.bottom() - 1.0), pos2(x0 - sesgo, rc.bottom() - 1.0)];
            if q[1].x < rc.right() - 4.0 {
                p.add(Shape::convex_polygon(q, Color32::from_white_alpha(a), Stroke::NONE));
            }
        }
    }
}

/// Etiquetas de colores pegadas a la derecha de `r`, de derecha a izquierda mientras quepan junto al nombre. Da lo que
/// queda de `r` para el nombre.
fn etiquetas(p: &Painter, r: Rect, tags: &[(String, Color32)]) -> Rect {
    let mut x = r.right() - 4.0;
    for (t, col) in tags {
        let g = p.layout_no_wrap(t.clone(), barlow(11.0), c(0x141414));
        let w = g.size().x + 10.0;
        if x - w < r.left() + 60.0 {
            break;
        }
        let caja = Rect::from_min_max(pos2(x - w, r.center().y - 6.5), pos2(x, r.center().y + 6.5));
        p.rect_filled(caja.translate(vec2(0.0, 1.0)), 6, Color32::from_black_alpha(80));
        degradado(p, caja, 6, mezcla(*col, Color32::WHITE, 0.3), *col);
        p.galley(caja.center() - g.size() / 2.0, g, c(0x141414));
        x -= w + 3.0;
    }
    Rect::from_min_max(r.min, pos2(x, r.max.y))
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
