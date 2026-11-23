//! Pestaña NODOS: los efectos del clip elegido conectados con cables; a la izquierda qué agregar, a la derecha los
//! controles del nodo con sus ruedas de color, y medidores de imagen.
use super::estilo::*;
use super::*;
use crate::idioma::{t, tf};
use crate::nodos::{Clase, ENTRADA, Grafo, Nodo, SALIDA};
use crate::nombres;

const ANCHO: f32 = 150.0;
const CABEZA: f32 = 26.0;
const PUERTO: f32 = 22.0;

pub struct Estado {
    pub elegido: Option<u64>,
    zoom: f32,
    centro: Vec2,
    encuadrar: bool,
    /// Cable en el aire: desde qué nodo sale.
    cable: Option<u64>,
    arrastre: Option<u64>,
    pub buscar: String,
    menu: Option<Pos2>,
}

impl Default for Estado {
    fn default() -> Self {
        Estado { elegido: None, zoom: 1.0, centro: Vec2::ZERO, encuadrar: true, cable: None, arrastre: None, buscar: String::new(), menu: None }
    }
}

/// Puntos de partida con varios nodos.
pub const PLANTILLAS: [(&str, &str); 7] = [
    ("Empezar de cero", "Un nodo con ruedas de sombras, medios y luces"),
    ("Color por zonas", "Color general y otro solo dentro de una máscara"),
    ("Aislar un color", "Ese color se queda y lo demás pasa a blanco y negro"),
    ("Resplandor", "Copia desenfocada en modo Pantalla"),
    ("Fondo desenfocado", "Nítido adentro de la máscara, borroso afuera"),
    ("Dos colores en paralelo", "Dos mezclas de color fusionadas"),
    ("Foco de escenario", "Oscurece todo menos una zona suave"),
];

pub fn plantilla(g: &mut Grafo, n: usize, mut fx: impl FnMut(&str) -> Option<FxVideo>) {
    *g = Grafo::default();
    let mut ef = |g: &mut Grafo, t: &str, v: &[(&str, f64)]| {
        let mut f = fx(t).unwrap_or(FxVideo { id: 0, tipo: t.into(), activo: true, p: Default::default() });
        for (k, x) in v {
            if let Some(p) = f.p.get_mut(*k) {
                p.v = Valor::Num(*x);
            }
        }
        g.agregar(Clase::Efecto, Some(f), 0.0, 0.0)
    };
    let modo = |g: &mut Grafo, id: u64, m: u32, op: f64| {
        if let Some(q) = g.nodo_mut(id) {
            q.fx.p.get_mut("modo").unwrap().v = Valor::Op(m);
            q.fx.p.get_mut("opacidad").unwrap().v = Valor::Num(op);
        }
    };
    match n {
        0 => {
            let a = ef(g, "bc:color", &[]);
            g.insertar(ENTRADA, a);
        }
        1 => {
            let a = ef(g, "bc:color", &[("con", 10.0)]);
            let b = ef(g, "bc:color", &[("exp", 0.5), ("temp", 20.0)]);
            let m = g.agregar(Clase::Mascara, None, 0.0, 0.0);
            g.insertar(ENTRADA, a);
            g.insertar(a, b);
            g.conectar(m, b, 1);
        }
        2 => {
            let gris = ef(g, "bc:color", &[("sat", 0.0)]);
            let q = g.agregar(Clase::Calificador, None, 0.0, 0.0);
            if let Some(x) = g.nodo_mut(q) {
                x.fx.p.get_mut("invertir").unwrap().v = Valor::Op(1);
                x.fx.p.get_mut("tono").unwrap().v = Valor::Num(5.0);
            }
            g.insertar(ENTRADA, gris);
            g.conectar(ENTRADA, q, 0);
            g.conectar(q, gris, 1);
        }
        3 => {
            let b = ef(g, "ec.blur.gaussian", &[("blurriness", 60.0)]);
            let c = ef(g, "bc:color", &[("exp", 0.4)]);
            let f = g.agregar(Clase::Fusionar, None, 0.0, 0.0);
            g.conectar(ENTRADA, b, 0);
            g.conectar(b, c, 0);
            g.conectar(ENTRADA, f, 0);
            g.conectar(c, f, 1);
            g.conectar(f, SALIDA, 0);
            modo(g, f, 2, 55.0);
        }
        4 => {
            let b = ef(g, "ec.blur.gaussian", &[("blurriness", 45.0)]);
            let m = g.agregar(Clase::Mascara, None, 0.0, 0.0);
            if let Some(x) = g.nodo_mut(m) {
                x.fx.p.get_mut("invertir").unwrap().v = Valor::Op(1);
            }
            g.insertar(ENTRADA, b);
            g.conectar(m, b, 1);
        }
        5 => {
            let a = ef(g, "bc:color", &[("temp", -40.0)]);
            let b = ef(g, "bc:color", &[("temp", 45.0)]);
            let f = g.agregar(Clase::Fusionar, None, 0.0, 0.0);
            let m = g.agregar(Clase::Mascara, None, 0.0, 0.0);
            if let Some(x) = g.nodo_mut(m) {
                x.fx.p.get_mut("forma").unwrap().v = Valor::Op(2);
            }
            g.conectar(ENTRADA, a, 0);
            g.conectar(ENTRADA, b, 0);
            g.conectar(a, f, 0);
            g.conectar(b, f, 1);
            g.conectar(m, f, 2);
            g.conectar(f, SALIDA, 0);
        }
        _ => {
            let a = ef(g, "bc:color", &[("exp", -2.2), ("sat", 60.0)]);
            let m = g.agregar(Clase::Mascara, None, 0.0, 0.0);
            if let Some(x) = g.nodo_mut(m) {
                x.fx.p.get_mut("invertir").unwrap().v = Valor::Op(1);
                x.fx.p.get_mut("suave").unwrap().v = Valor::Num(60.0);
            }
            g.insertar(ENTRADA, a);
            g.conectar(m, a, 1);
        }
    }
    g.acomodar();
}

fn nombre(n: &Nodo) -> String {
    match n.clase {
        Clase::Entrada => t("Clip").into(),
        Clase::Salida => t("Salida").into(),
        _ => t(nombres::efecto(&n.fx.tipo)).into(),
    }
}

fn tinta(c: Clase) -> Color32 {
    match c {
        Clase::Entrada | Clase::Salida => c_(0x6f7f8f),
        Clase::Efecto => c_(0xb08a3a),
        Clase::Fusionar => c_(0x5f8f6a),
        Clase::Mascara | Clase::Calificador => c_(0x7c6bb0),
        Clase::Fondo => c_(0x9a5a5a),
    }
}

fn c_(x: u32) -> Color32 {
    c(x)
}

/// Cable como curva suave.
fn cable(p: &Painter, a: Pos2, b: Pos2, col: Color32, w: f32) {
    let d = ((b.x - a.x).abs() * 0.5).clamp(24.0, 140.0);
    p.add(Shape::CubicBezier(eframe::egui::epaint::CubicBezierShape::from_points_stroke([a, a + vec2(d, 0.0), b - vec2(d, 0.0), b], false, Color32::TRANSPARENT, Stroke::new(w, col))));
}

impl App {
    pub fn espacio_nodos(&mut self, ui: &mut Ui) {
        // Sin clip elegido, el de arriba en el cursor.
        if self.clip.is_none_or(|id| !self.p.video.iter().any(|c| c.id == id)) {
            let ahora = self.ahora();
            self.clip = (0..self.p.capas.len()).rev().find_map(|i| self.p.video_en(i, ahora)).map(|c| c.id).or(self.p.video.first().map(|c| c.id));
        }
        let id = self.clip.filter(|id| self.p.video.iter().any(|c| c.id == *id));
        let alto = ui.available_height();
        Panel::right("n-der").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(6.0)).resizable(true).default_size(390.0).min_size(300.0).show(ui, |ui| self.panel_nodo(ui, id));
        Panel::left("n-izq").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(8.0)).resizable(true).default_size(250.0).min_size(190.0).show(ui, |ui| self.paleta_nodos(ui, id));
        Panel::bottom("n-grafo").frame(Frame::new().fill(WORKSPACE)).resizable(true).default_size(alto * 0.48).size_range(170.0..=(alto - 160.0).max(200.0)).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            self.lienzo_nodos(ui, id);
        });
        CentralPanel::default().frame(Frame::new().fill(Color32::BLACK)).show(ui, |ui| self.preview(ui));
    }

    fn grafo(&mut self, id: u64) -> Option<&mut Grafo> {
        self.p.clip_video(id).map(|c| c.nodos.get_or_insert_with(Grafo::default))
    }

    /// Agrega un nodo después del elegido (o antes de la salida) y lo deja elegido.
    pub fn agregar_nodo(&mut self, id: u64, clase: Clase, fx: Option<FxVideo>, donde: Option<Vec2>) {
        let sel = self.nodos.elegido;
        let Some(g) = self.grafo(id) else { return };
        let antes = sel.filter(|s| *s != SALIDA && g.nodo(*s).is_some_and(|n| !n.clase.da_mascara())).or_else(|| g.cables.iter().find(|c| c.a == SALIDA).map(|c| c.de)).unwrap_or(ENTRADA);
        let (x, y) = donde.map_or_else(|| g.nodo(antes).map_or((40.0, 120.0), |n| (n.x + 190.0, n.y)), |d| (d.x, d.y));
        let n = g.agregar(clase, fx, x, y);
        match clase {
            Clase::Efecto if donde.is_none() => {
                g.insertar(antes, n);
                g.acomodar();
            }
            // Una máscara nueva se engancha al efecto elegido.
            Clase::Mascara | Clase::Calificador => {
                if clase == Clase::Calificador {
                    g.conectar(ENTRADA, n, 0);
                }
                if let Some(s) = sel {
                    let p = g.nodo(s).map_or(9, |x| if x.clase == Clase::Fusionar { 2 } else if x.clase == Clase::Efecto { 1 } else { 9 });
                    g.conectar(n, s, p);
                }
                if donde.is_none() {
                    g.acomodar();
                }
            }
            Clase::Fusionar if donde.is_none() => {
                g.insertar(antes, n);
                g.acomodar();
            }
            _ => {}
        }
        self.nodos.elegido = Some(n);
        self.nodos.encuadrar = donde.is_none();
    }

    fn paleta_nodos(&mut self, ui: &mut Ui, id: Option<u64>) {
        let Some(id) = id else {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| ui.label(RichText::new(t("Importa un video o imagen para usar nodos")).color(TEXT_FAINT)));
            return;
        };
        ui.add(TextEdit::singleline(&mut self.nodos.buscar).hint_text(t("Buscar nodo…")).margin(Margin::symmetric(8, 5)).desired_width(ui.available_width() - 4.0));
        ui.label(RichText::new(t("Clic: va después del nodo elegido.")).size(12.5).color(TEXT_FAINT));
        let q = self.nodos.buscar.to_lowercase();
        let va = |n: &str| q.is_empty() || t(n).to_lowercase().contains(&q);
        let mut nuevo: Option<(Clase, Option<&str>)> = None;
        let mut pl = None;
        ScrollArea::vertical().id_salt("paleta-nodos").auto_shrink([false, false]).show(ui, |ui| {
            let fila = |ui: &mut Ui, n: &str, ayuda: &str, col: Color32| -> bool {
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width() - 6.0, 25.0), Sense::click());
                resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tf("Nodo {}", &[&t(n)])));
                let p = ui.painter_at(r);
                p.rect_filled(r, 4, if resp.hovered() { BUTTON_HI } else { c(0x363636) });
                p.rect_filled(Rect::from_min_size(r.min, vec2(4.0, r.height())), 2, col);
                p.text(pos2(r.left() + 12.0, r.center().y), Align2::LEFT_CENTER, t(n), fuente(14.0), TEXT);
                let clic = resp.clicked();
                if !ayuda.is_empty() {
                    resp.on_hover_text(t(ayuda));
                }
                clic
            };
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.add_space(4.0);
            ui.label(RichText::new(t("Básicos")).color(MAIN));
            if va("Mezcla de color") && fila(ui, "Mezcla de color", "Ruedas de sombras, medios y luces, exposición, temperatura, tonos y más", tinta(Clase::Efecto)) {
                nuevo = Some((Clase::Efecto, Some("bc:color")));
            }
            for (k, n, a) in Clase::NUEVAS {
                if va(n) && fila(ui, n, a, tinta(k)) {
                    nuevo = Some((k, None));
                }
            }
            ui.add_space(6.0);
            ui.label(RichText::new(t("Listos para empezar")).color(MAIN));
            for (i, (n, a)) in PLANTILLAS.iter().enumerate() {
                if va(n) && fila(ui, n, a, c(0x8a8a8a)) {
                    pl = Some(i);
                }
            }
            let mut g = "";
            for e in nombres::EFECTOS.iter().filter(|e| e.2 != "bc:color" && (va(e.1) || va(e.0))) {
                if g != e.0 {
                    g = e.0;
                    ui.add_space(6.0);
                    ui.label(RichText::new(t(g)).color(MAIN));
                }
                let r = ui.cursor();
                // Solo se dibuja lo que se ve.
                if !ui.is_rect_visible(Rect::from_min_size(r.min, vec2(10.0, 25.0))) {
                    ui.add_space(25.0);
                    continue;
                }
                if fila(ui, e.1, "", tinta(Clase::Efecto)) {
                    nuevo = Some((Clase::Efecto, Some(e.2)));
                }
            }
        });
        if let Some((k, e)) = nuevo {
            let fx = e.and_then(|e| crate::composicion::fx_nuevo(0, e));
            if k != Clase::Efecto || fx.is_some() {
                self.agregar_nodo(id, k, fx, None);
            }
        }
        if let Some(i) = pl {
            if let Some(g) = self.grafo(id) {
                plantilla(g, i, |t| crate::composicion::fx_nuevo(0, t));
                self.nodos.elegido = g.nodos.iter().find(|n| n.clase == Clase::Efecto).map(|n| n.id);
            }
            self.nodos.encuadrar = true;
            self.avisar(tf("«{}» puesto", &[&t(PLANTILLAS[i].0)]), false);
        }
    }

    fn panel_nodo(&mut self, ui: &mut Ui, id: Option<u64>) {
        self.medidores_imagen(ui);
        let ahora = self.ahora();
        let Some(id) = id else { return };
        let sel = self.nodos.elegido;
        let mut quitar = false;
        let Some(c) = self.p.clip_video(id) else { return };
        let tl = (ahora - c.inicio).clamp(0.0, c.dur);
        let Some(g) = c.nodos.as_mut() else {
            ui.label(RichText::new(t("Agrega un nodo de la izquierda para empezar.")).color(TEXT_FAINT));
            return;
        };
        let Some(n) = sel.and_then(|s| g.nodo_mut(s)) else {
            ui.label(RichText::new(t("Elige un nodo para ver sus controles.")).color(TEXT_FAINT));
            return;
        };
        ui.horizontal(|ui| {
            if n.clase != Clase::Entrada && n.clase != Clase::Salida {
                if led(ui, n.fx.activo, "Encender el nodo").clicked() {
                    n.fx.activo = !n.fx.activo;
                }
            }
            ui.label(RichText::new(nombre(n)).size(17.0));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if n.id != ENTRADA && n.id != SALIDA {
                    quitar = boton_color(ui, &tabler::X, false, "Quitar nodo (⌫)", TEXT_DIM, 24.0).clicked();
                }
            });
        });
        ScrollArea::vertical().id_salt("panel-nodo").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            match n.clase {
                Clase::Entrada => {
                    ui.label(RichText::new(t("La imagen del clip, ya con sus efectos normales.")).color(TEXT_FAINT));
                }
                Clase::Salida => {
                    ui.label(RichText::new(t("Lo que llegue aquí es lo que se ve.")).color(TEXT_FAINT));
                }
                _ => inspector::fx_ui(ui, &mut n.fx, tl),
            }
        });
        if quitar {
            self.quitar_nodo();
        }
    }

    pub fn quitar_nodo(&mut self) {
        if let (Some(id), Some(s)) = (self.clip, self.nodos.elegido) {
            if let Some(g) = self.p.clip_video(id).and_then(|c| c.nodos.as_mut()) {
                g.quitar(s);
            }
            self.nodos.elegido = None;
        }
    }

    fn lienzo_nodos(&mut self, ui: &mut Ui, id: Option<u64>) {
        // Barra
        Frame::new().fill(TOOLBAR).inner_margin(Margin::symmetric(8, 4)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.set_min_height(26.0);
                ui.set_min_width(ui.available_width());
                let Some(id) = id else { return };
                let Some(c) = self.p.clip_video(id) else { return };
                let n = animacion::nombre_clip(c);
                let hay = c.nodos.is_some();
                let on = c.nodos.as_ref().is_some_and(|g| g.activo);
                if hay && led(ui, on, "Encender los nodos de este clip").clicked() {
                    c.nodos.as_mut().unwrap().activo = !on;
                }
                ui.label(RichText::new(tf("Nodos de «{}»", &[&n])).color(TEXT_DIM));
                ui.separator();
                if ui.add_enabled(hay, Button::new(t("Acomodar"))).clicked() {
                    c.nodos.as_mut().unwrap().acomodar();
                    self.nodos.encuadrar = true;
                }
                if ui.add_enabled(hay, Button::new(t("Ver todo"))).clicked() {
                    self.nodos.encuadrar = true;
                }
                if ui.add_enabled(hay, Button::new(t("Quitar todos los nodos"))).clicked() {
                    c.nodos = None;
                    self.nodos.elegido = None;
                }
                if ui.available_width() < 470.0 {
                    return;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(t("Arrastra de un círculo a otro para conectar · doble clic apaga · clic derecho agrega")).size(12.5).color(TEXT_FAINT));
                });
            });
        });
        let todo = ui.available_rect_before_wrap();
        let resp = ui.allocate_rect(todo, Sense::click_and_drag());
        let p = ui.painter_at(todo);
        p.rect_filled(todo, 0, c(0x202022));
        let g = id.and_then(|id| self.p.video.iter().find(|c| c.id == id)).map(|c| c.nodos.clone().unwrap_or_default());
        let (Some(id), Some(g)) = (id, g) else {
            p.text(todo.center(), Align2::CENTER_CENTER, t("Elige un clip de video"), fuente(16.0), TEXT_FAINT);
            return;
        };
        if std::mem::take(&mut self.nodos.encuadrar) {
            let mut r = Rect::NOTHING;
            for n in &g.nodos {
                r = r.union(Rect::from_min_size(pos2(n.x, n.y), vec2(ANCHO, alto_nodo(n))));
            }
            let r = r.expand(30.0);
            self.nodos.zoom = (todo.width() / r.width()).min(todo.height() / r.height()).clamp(0.35, 1.25);
            self.nodos.centro = r.center().to_vec2();
        }
        if resp.hovered() {
            let (z, s) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta()));
            self.nodos.zoom = (self.nodos.zoom * z).clamp(0.25, 2.5);
            self.nodos.centro -= s / self.nodos.zoom;
        }
        let (zoom, centro) = (self.nodos.zoom, self.nodos.centro);
        let a_p = move |v: Vec2| todo.center() + (v - centro) * zoom;
        let a_g = move |o: Pos2| (o - todo.center()) / zoom + centro;
        // Cuadrícula de puntos
        let paso = 28.0 * zoom;
        if paso > 8.0 {
            let o = a_p(Vec2::ZERO);
            let (mut y, y0) = (todo.top() + (o.y - todo.top()).rem_euclid(paso), todo.bottom());
            while y < y0 {
                let mut x = todo.left() + (o.x - todo.left()).rem_euclid(paso);
                while x < todo.right() {
                    p.circle_filled(pos2(x, y), 1.0, c(0x303033));
                    x += paso;
                }
                y += paso;
            }
        }
        let caja = |n: &Nodo| Rect::from_min_size(a_p(vec2(n.x, n.y)), vec2(ANCHO, alto_nodo(n)) * zoom);
        let sal = |n: &Nodo| a_p(vec2(n.x + ANCHO, n.y + CABEZA / 2.0));
        let ent = |n: &Nodo, i: usize| a_p(vec2(n.x, n.y + CABEZA + PUERTO * (i as f32 + 0.5)));
        let raton = ui.input(|i| i.pointer.hover_pos()).filter(|m| todo.contains(*m));
        for k in &g.cables {
            if let (Some(a), Some(b)) = (g.nodo(k.de), g.nodo(k.a)) {
                let on = self.nodos.elegido == Some(k.de) || self.nodos.elegido == Some(k.a);
                cable(&p, sal(a), ent(b, k.puerto as usize), if a.clase.da_mascara() { c(0x9a8ad0) } else if on { MAIN } else { c(0x9a9a9a) }, if on { 2.5 } else { 2.0 });
            }
        }
        // Puertos: (posición, nodo, entrada o salida)
        let mut puertos: Vec<(Pos2, u64, Option<u8>)> = vec![];
        for n in &g.nodos {
            let r = caja(n);
            let sel = self.nodos.elegido == Some(n.id);
            p.rect_filled(r.translate(vec2(0.0, 3.0)), 8, Color32::from_black_alpha(70));
            p.rect_filled(r, 8, c(0x343437));
            let cab = Rect::from_min_size(r.min, vec2(r.width(), CABEZA * zoom));
            let col = tinta(n.clase);
            let col = if n.fx.activo { col } else { mezcla(col, c(0x343437), 0.75) };
            p.rect_filled(cab, CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, col);
            let g2 = {
                let mut job = eframe::egui::epaint::text::LayoutJob::simple_singleline(nombre(n), fuente(14.0 * zoom), c(0x171717));
                job.wrap = eframe::egui::epaint::text::TextWrapping { max_width: cab.width() - 16.0 * zoom, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
                p.layout_job(job)
            };
            p.galley(pos2(cab.left() + 8.0 * zoom, cab.center().y - g2.size().y / 2.0), g2, c(0x171717));
            for (i, e) in n.clase.entradas().iter().enumerate() {
                let o = ent(n, i);
                p.text(o + vec2(10.0 * zoom, 0.0), Align2::LEFT_CENTER, t(e), fuente(12.5 * zoom), TEXT_DIM);
                let puesto = g.cables.iter().any(|k| k.a == n.id && k.puerto == i as u8);
                p.circle(o, 5.5 * zoom, if puesto { MAIN } else { c(0x202022) }, Stroke::new(1.5, c(0xbababa)));
                puertos.push((o, n.id, Some(i as u8)));
            }
            if n.clase != Clase::Salida {
                p.circle(sal(n), 6.0 * zoom, if n.clase.da_mascara() { c(0x9a8ad0) } else { c(0xd8d8d8) }, Stroke::new(1.5, c(0x202022)));
                puertos.push((sal(n), n.id, None));
            }
            if sel {
                p.rect_stroke(r, 8, Stroke::new(2.0, MAIN), StrokeKind::Outside);
            }
            ui.interact(r.intersect(todo), Id::new(("nodo", n.id)), Sense::hover()).widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, tf("En el lienzo: {}", &[&nombre(n)])));
        }
        let puerto_en = |m: Pos2| puertos.iter().filter(|x| x.0.distance(m) < 13.0 * zoom.max(0.7)).min_by(|a, b| a.0.distance(m).total_cmp(&b.0.distance(m))).copied();
        let nodo_en = |m: Pos2| g.nodos.iter().rev().find(|n| caja(n).contains(m)).map(|n| n.id);
        let mut cambio: Option<Box<dyn FnOnce(&mut Grafo)>> = None;
        if resp.drag_started() {
            if let Some(m) = ui.input(|i| i.pointer.press_origin()) {
                match puerto_en(m) {
                    Some((_, n, None)) => self.nodos.cable = Some(n),
                    // Jalar de una entrada suelta su cable.
                    Some((_, n, Some(q))) => {
                        if let Some(k) = g.cables.iter().find(|k| k.a == n && k.puerto == q).copied() {
                            self.nodos.cable = Some(k.de);
                            cambio = Some(Box::new(move |g| g.cables.retain(|x| *x != k)));
                        }
                    }
                    None => {
                        self.nodos.arrastre = nodo_en(m);
                        if let Some(n) = self.nodos.arrastre {
                            self.nodos.elegido = Some(n);
                        }
                    }
                }
            }
        }
        if resp.dragged() {
            let d = resp.drag_delta() / zoom;
            if self.nodos.cable.is_none() {
                match self.nodos.arrastre {
                    Some(n) => {
                        cambio = Some(Box::new(move |g| {
                            if let Some(x) = g.nodo_mut(n) {
                                x.x += d.x;
                                x.y += d.y;
                            }
                        }))
                    }
                    None => self.nodos.centro -= d,
                }
            }
        }
        if let (Some(de), Some(m)) = (self.nodos.cable, raton) {
            if let Some(n) = g.nodo(de) {
                let fin = puerto_en(m).filter(|x| x.2.is_some()).map_or(m, |x| x.0);
                cable(&p, sal(n), fin, HIGHLIGHT, 2.5);
            }
        }
        if resp.drag_stopped() {
            if let (Some(de), Some(m)) = (self.nodos.cable.take(), raton) {
                // Sobre un nodo sin apuntar a su círculo: la primera entrada libre.
                let destino = puerto_en(m).and_then(|x| Some((x.1, x.2?))).or_else(|| {
                    let n = g.nodo(nodo_en(m)?)?;
                    let k = (0..n.clase.entradas().len() as u8).find(|q| !g.cables.iter().any(|k| k.a == n.id && k.puerto == *q)).unwrap_or(0);
                    (!n.clase.entradas().is_empty()).then_some((n.id, k))
                });
                if let Some((a, q)) = destino {
                    if g.llega(a, de) {
                        self.avisar(t("Eso haría una vuelta: los nodos van en un solo sentido"), false);
                    } else {
                        cambio = Some(Box::new(move |g| {
                            g.conectar(de, a, q);
                        }));
                    }
                }
            }
            self.nodos.arrastre = None;
        }
        if let Some(m) = raton {
            if resp.clicked() {
                self.nodos.elegido = nodo_en(m);
            }
            if resp.double_clicked() {
                if let Some(n) = nodo_en(m).filter(|n| *n != ENTRADA && *n != SALIDA) {
                    cambio = Some(Box::new(move |g| {
                        if let Some(x) = g.nodo_mut(n) {
                            x.fx.activo = !x.fx.activo;
                        }
                    }));
                }
            }
            if resp.secondary_clicked() {
                self.nodos.menu = Some(m);
                self.nodos.elegido = nodo_en(m).or(self.nodos.elegido);
            }
        }
        let donde = self.nodos.menu.map(a_g);
        resp.context_menu(|ui| {
            let sobre = self.nodos.elegido.filter(|s| *s != ENTRADA && *s != SALIDA);
            if sobre.is_some() && ui.button(t("Quitar nodo")).clicked() {
                self.quitar_nodo();
            }
            if ui.button(t("Mezcla de color")).clicked() {
                let f = crate::composicion::fx_nuevo(0, "bc:color");
                self.agregar_nodo(id, Clase::Efecto, f, donde);
            }
            for (k, n, _) in Clase::NUEVAS {
                if ui.button(t(n)).clicked() {
                    self.agregar_nodo(id, k, None, donde);
                }
            }
            ui.separator();
            let l = nombres::EFECTOS;
            let mut i = 0;
            while i < l.len() {
                let cat = l[i].0;
                let j = i + l[i..].iter().take_while(|e| e.0 == cat).count();
                ui.menu_button(t(cat), |ui| {
                    ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                        for e in &l[i..j] {
                            if ui.button(t(e.1)).clicked() {
                                let f = crate::composicion::fx_nuevo(0, e.2);
                                self.agregar_nodo(id, Clase::Efecto, f, donde);
                                ui.close();
                            }
                        }
                    });
                });
                i = j;
            }
        });
        if let Some(f) = cambio {
            if let Some(g) = self.grafo(id) {
                f(g);
            }
        }
        // Soltar un efecto del catálogo.
        if let Some(s) = resp.dnd_release_payload::<biblioteca::Soltar>() {
            if let biblioteca::Soltar::FxVideo(e) = &*s {
                let f = crate::composicion::fx_nuevo(0, e);
                self.agregar_nodo(id, Clase::Efecto, f, raton.map(a_g));
            }
        }
    }
}

fn alto_nodo(n: &Nodo) -> f32 {
    CABEZA + PUERTO * n.clase.entradas().len().max(1) as f32 + 4.0
}
