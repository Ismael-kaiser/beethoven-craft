//! Pestaña NODOS: los efectos del clip elegido conectados con cables; a la izquierda qué agregar, a la derecha los
//! controles del nodo con sus ruedas de color, y medidores de imagen.
use super::estilo::*;
use super::*;
use crate::idioma::{t, tf};
use crate::nodos::{Clase, ENTRADA, Grafo, Nodo, SALIDA};
use crate::nombres;

pub const ANCHO: f32 = 160.0;
pub const CABEZA: f32 = 26.0;
pub const PUERTO: f32 = 22.0;
/// La imagen de cada nodo.
pub const MINI: f32 = 78.0;

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
    /// Qué se ve a la izquierda.
    pub seccion: usize,
}

const SECCIONES: [&str; 4] = ["Todo", "Clips", "Nodos", "Efectos"];

impl Default for Estado {
    fn default() -> Self {
        Estado { elegido: None, zoom: 1.0, centro: Vec2::ZERO, encuadrar: true, cable: None, arrastre: None, buscar: String::new(), menu: None, seccion: 0 }
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
        Clase::Medio => n.medio.as_ref().map_or(t("Clip o imagen").into(), |c| animacion::nombre_clip(c)),
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
        Clase::Medio => c_(0x4f8fa8),
    }
}

fn c_(x: u32) -> Color32 {
    c(x)
}

/// Cable como curva suave.
fn cable(p: &Painter, a: Pos2, b: Pos2, col: Color32, w: f32) {
    let d = ((b.x - a.x).abs() * 0.5).clamp(24.0, 140.0);
    let raya = |dy: f32, w: f32, col: Color32| {
        let o = vec2(0.0, dy);
        p.add(Shape::CubicBezier(eframe::egui::epaint::CubicBezierShape::from_points_stroke([a + o, a + o + vec2(d, 0.0), b + o - vec2(d, 0.0), b + o], false, Color32::TRANSPARENT, Stroke::new(w, col))));
    };
    // Cable con sombra, forro oscuro, color y un filo de luz.
    raya(2.5, w + 3.0, Color32::from_black_alpha(70));
    raya(0.0, w + 2.5, mezcla(col, Color32::BLACK, 0.7));
    raya(0.0, w, col);
    raya(-w * 0.28, (w * 0.35).max(0.7), Color32::from_white_alpha(110));
}

/// Conector: aro de metal y, si tiene cable, su luz.
fn enchufe(p: &Painter, o: Pos2, r: f32, luz: Option<Color32>) {
    p.circle_filled(o + vec2(0.0, r * 0.25), r + 1.0, Color32::from_black_alpha(130));
    p.circle_filled(o, r, c_(0x2a2a2a));
    p.circle_filled(o - vec2(0.0, r * 0.12), r * 0.92, c(0x8a8a8a));
    p.circle_filled(o, r * 0.72, c_(0x161616));
    if let Some(l) = luz {
        p.circle_filled(o, r * 0.58, mezcla(l, Color32::BLACK, 0.15));
        p.circle_filled(o - vec2(r * 0.12, r * 0.18), r * 0.26, Color32::from_white_alpha(150));
    }
}

const TEAL: Color32 = Color32::from_rgb(0x5f, 0xd3, 0xc4);

/// Texto de un renglón, con puntos suspensivos si no cabe.
fn texto_corto(p: &Painter, r: Rect, n: &str, tam: f32, col: Color32) {
    let mut job = eframe::egui::epaint::text::LayoutJob::simple_singleline(n.to_string(), fuente(tam), col);
    job.wrap = eframe::egui::epaint::text::TextWrapping { max_width: r.width(), max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    let g = p.layout_job(job);
    p.galley(pos2(r.left(), r.center().y - g.size().y / 2.0), g, col);
}
fn texto_centro(p: &Painter, r: Rect, n: &str, tam: f32, col: Color32) {
    let g = p.layout(n.to_string(), fuente(tam), col, r.width() - 6.0);
    p.galley(r.center() - g.size() / 2.0, g, col);
}

/// Tarjeta con su dibujo arriba y el nombre abajo.
pub(super) fn carta(ui: &mut Ui, w: f32, n: &str, ayuda: &str, etiqueta: &str, pinta: impl FnOnce(&Painter, Rect)) -> Response {
    let alto = (w * 9.0 / 16.0).round();
    let (r, resp) = ui.allocate_exact_size(vec2(w, alto + 20.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, etiqueta));
    if ui.is_rect_visible(r) {
        let p = ui.painter_at(r.expand(3.0));
        let img = Rect::from_min_size(r.min, vec2(w, alto));
        p.rect_filled(img.translate(vec2(0.0, 1.5)), 6, Color32::from_black_alpha(90));
        pinta(&p, img);
        vidrio(&p, img, 6, if resp.hovered() { MAIN } else { c_(0x909090) }, resp.hovered());
        if resp.hovered() {
            p.rect_stroke(img, 6, Stroke::new(2.0, MAIN), StrokeKind::Outside);
        }
        texto_corto(&p, Rect::from_min_max(pos2(r.left() + 2.0, img.bottom() + 1.0), r.max), n, 12.0, if resp.hovered() { MAIN } else { TEXT });
    }
    if ayuda.is_empty() { resp.on_hover_text(n) } else { resp.on_hover_text(t(ayuda)) }
}

fn pinta_tex(p: &Painter, r: Rect, tex: Option<TextureId>) {
    let rad = if r.height() < 40.0 { 4 } else { 6 };
    match tex {
        Some(id) => {
            p.add(epaint::RectShape::filled(r, rad, Color32::WHITE).with_texture(id, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))));
        }
        None => hueco(p, r, rad, c_(0x1c1c1c)),
    }
}

/// Un clip en chiquito: su cuadro, o un dibujo de lo que es.
pub(super) fn pinta_fuente(p: &Painter, r: Rect, c: &ClipVideo, tex: Option<TextureId>) {
    if tex.is_some() {
        return pinta_tex(p, r, tex);
    }
    let (a, b) = match &c.contenido {
        Contenido::Color(q) => {
            let x = Color32::from_rgb((q[0] * 255.0) as u8, (q[1] * 255.0) as u8, (q[2] * 255.0) as u8);
            (mezcla(x, Color32::WHITE, 0.1), mezcla(x, Color32::BLACK, 0.15))
        }
        Contenido::Titulo(_) => (c_(0x8a3f78), c_(0x43203a)),
        Contenido::Forma(_) => (c_(0x8a5432), c_(0x43281a)),
        Contenido::Ajuste => (c_(0x43608c), c_(0x223046)),
        _ => (c_(0x3c3c3c), c_(0x1e1e1e)),
    };
    degradado(p, r, 6, a, b);
    let medio = r.center();
    match &c.contenido {
        Contenido::Titulo(x) => texto_centro(p, r, x.texto.lines().next().unwrap_or("T"), (r.height() * 0.26).clamp(8.0, 18.0), Color32::WHITE),
        Contenido::Forma(_) => {
            p.circle_filled(medio, r.height() * 0.28, Color32::from_white_alpha(190));
        }
        Contenido::Ajuste => icono(p, &tabler::ADJUSTMENTS, Rect::from_center_size(medio, Vec2::splat(r.height() * 0.5)), Color32::from_white_alpha(200)),
        Contenido::Archivo | Contenido::Imagen => icono(p, &tabler::MOVIE, Rect::from_center_size(medio, Vec2::splat(r.height() * 0.45)), TEXT_FAINT),
        _ => {}
    }
}

/// Dibujos de los nodos básicos.
fn pinta_basico(p: &Painter, r: Rect, k: Clase) {
    degradado(p, r, 6, c_(0x4a4a4a), c_(0x242424));
    let (o, u) = (r.center(), r.height() * 0.28);
    match k {
        Clase::Efecto => {
            // Las tres ruedas.
            for (i, x) in [-1.0f32, 0.0, 1.0].into_iter().enumerate() {
                let q = o + vec2(x * u * 1.75, 0.0);
                let n = 18;
                for j in 0..n {
                    let a = j as f32 / n as f32 * std::f32::consts::TAU;
                    arco(p, q, u * 0.72, a, std::f32::consts::TAU / n as f32 * 1.1, u * 0.22, Color32::from(eframe::egui::ecolor::Hsva::new(j as f32 / n as f32, 0.75, 0.95, 1.0)));
                }
                p.circle_filled(q + Vec2::angled(i as f32 * 2.1) * u * 0.25, u * 0.17, Color32::WHITE);
            }
        }
        Clase::Fusionar => {
            p.rect_filled(Rect::from_center_size(o - vec2(u * 0.5, u * 0.3), vec2(u * 2.0, u * 1.5)), 3, c_(0x5f8f6a));
            p.rect_filled(Rect::from_center_size(o + vec2(u * 0.5, u * 0.3), vec2(u * 2.0, u * 1.5)), 3, Color32::from_rgba_unmultiplied(255, 224, 102, 190));
        }
        Clase::Mascara => {
            p.add(Shape::ellipse_filled(o, vec2(u * 1.6, u * 1.05), Color32::from_white_alpha(60)));
            p.add(Shape::ellipse_filled(o, vec2(u * 1.25, u * 0.8), Color32::from_white_alpha(230)));
        }
        Clase::Calificador => {
            let b = Rect::from_center_size(o, vec2(r.width() * 0.76, u * 0.9));
            let n = 24;
            for i in 0..n {
                let x = b.left() + b.width() * i as f32 / n as f32;
                p.rect_filled(Rect::from_min_size(pos2(x, b.top()), vec2(b.width() / n as f32 + 0.6, b.height())), 0, Color32::from(eframe::egui::ecolor::Hsva::new(i as f32 / n as f32, 0.8, 0.9, 1.0)));
            }
            p.rect_stroke(Rect::from_center_size(b.center() - vec2(b.width() * 0.2, 0.0), vec2(b.width() * 0.22, b.height() + 6.0)), 3, Stroke::new(2.0, Color32::WHITE), StrokeKind::Middle);
        }
        Clase::Fondo => {
            p.rect_filled(Rect::from_center_size(o, vec2(u * 2.6, u * 1.8)), 4, c_(0x9a5a5a));
        }
        _ => icono(p, &tabler::MOVIE, Rect::from_center_size(o, Vec2::splat(u * 1.8)), TEXT_DIM),
    }
}

/// La forma de una máscara, tal como está.
fn pinta_mascara(p: &Painter, r: Rect, n: &Nodo, tl: f64) {
    let (w, h) = (36u32, 20u32);
    p.rect_filled(r, 4, Color32::BLACK);
    let m = crate::nodos::mascara(n, tl, w, h);
    let mut malla = Mesh::default();
    let (dx, dy) = (r.width() / w as f32, r.height() / h as f32);
    for y in 0..h {
        for x in 0..w {
            let v = (m.data[(y * w + x) as usize][3] * 235.0) as u8;
            if v > 4 {
                malla.add_colored_rect(Rect::from_min_size(r.min + vec2(x as f32 * dx, y as f32 * dy), vec2(dx + 0.3, dy + 0.3)), Color32::from_gray(v));
            }
        }
    }
    p.add(Shape::mesh(malla));
}

/// El grafo de una plantilla, en chiquito.
fn pinta_plantilla(p: &Painter, r: Rect, i: usize) {
    degradado(p, r, 6, c_(0x3c3c3c), c_(0x1e1e1e));
    let mut g = Grafo::default();
    plantilla(&mut g, i, |_| None);
    let (mut a, mut b) = (pos2(f32::MAX, f32::MAX), pos2(f32::MIN, f32::MIN));
    for n in &g.nodos {
        (a, b) = (a.min(pos2(n.x, n.y)), b.max(pos2(n.x + 120.0, n.y + 60.0)));
    }
    let dentro = r.shrink2(vec2(r.width() * 0.08, r.height() * 0.2));
    let k = (dentro.width() / (b.x - a.x)).min(dentro.height() / (b.y - a.y)).min(0.16);
    let o = |x: f32, y: f32| r.center() + vec2(x - (a.x + b.x) / 2.0, y - (a.y + b.y) / 2.0) * k;
    for q in &g.cables {
        if let (Some(m), Some(n)) = (g.nodo(q.de), g.nodo(q.a)) {
            p.line_segment([o(m.x + 120.0, m.y + 30.0), o(n.x, n.y + 30.0)], Stroke::new(1.2, if m.clase.da_mascara() { c(0x9a8ad0) } else { c(0xb0b0b0) }));
        }
    }
    for n in &g.nodos {
        let q = Rect::from_min_max(o(n.x, n.y), o(n.x + 120.0, n.y + 60.0));
        p.rect_filled(q.translate(vec2(0.0, 1.0)), 2, Color32::from_black_alpha(120));
        degradado(p, q, 2, mezcla(tinta(n.clase), Color32::WHITE, 0.25), tinta(n.clase));
    }
}

impl App {
    pub fn espacio_nodos(&mut self, ui: &mut Ui) {
        // Las tarjetas de efectos enseñan cómo se ve cada uno.
        if !self.muestras_pedidas && !self.sin_dispositivo {
            self.pedir_muestras();
        }
        // Sin clip elegido, el de arriba en el cursor.
        if self.clip.is_none_or(|id| !self.p.video.iter().any(|c| c.id == id)) {
            let ahora = self.ahora();
            self.clip = (0..self.p.capas.len()).rev().find_map(|i| self.p.video_en(i, ahora)).map(|c| c.id).or(self.p.video.first().map(|c| c.id));
        }
        let id = self.clip.filter(|id| self.p.video.iter().any(|c| c.id == *id));
        let alto = ui.available_height();
        Panel::right("n-der").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(6.0)).resizable(true).default_size(390.0).min_size(300.0).show(ui, |ui| self.panel_nodo(ui, id));
        Panel::left("n-izq").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(8.0)).resizable(true).default_size(310.0).min_size(210.0).show(ui, |ui| self.paleta_nodos(ui, id));
        Panel::bottom("n-grafo").frame(Frame::new().fill(WORKSPACE)).resizable(true).default_size(alto * 0.6).size_range(170.0..=(alto - 160.0).max(200.0)).show(ui, |ui| {
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

    /// Los clips, imágenes y demás que se pueden usar como nodo, sin repetir archivo.
    fn fuentes_de_nodos(&self) -> Vec<ClipVideo> {
        let mut v: Vec<ClipVideo> = vec![];
        for c in self.p.video.iter().filter(|c| c.contenido != Contenido::Ajuste) {
            let archivo = matches!(c.contenido, Contenido::Archivo | Contenido::Imagen);
            if !v.iter().any(|x| if archivo { x.ruta == c.ruta && x.contenido == c.contenido } else { x.contenido == c.contenido }) {
                let mut c = c.clone();
                // Solo lo que se ve: sin sus efectos ni su movimiento. Un video, desde su principio.
                (c.fx, c.nodos, c.entrada, c.salida, c.mov, c.clase, c.grupo) = (vec![], None, None, None, Default::default(), None, None);
                if c.contenido == Contenido::Archivo {
                    c.desde = 0.0;
                }
                v.push(c);
            }
        }
        v
    }

    /// Cómo se ve un clip, en chiquito: textura y pedazo. `tl`: segundo del clip.
    pub(super) fn mini_clip(&self, c: &ClipVideo, tl: f64) -> Option<TextureId> {
        let tl_ = self.tiras.get(&c.ruta).and_then(|x| x.as_ref()).filter(|_| matches!(c.contenido, Contenido::Archivo | Contenido::Imagen))?;
        let k = ((c.desde + tl * c.mov.velocidad).max(0.0) / tl_.paso) as usize;
        tl_.tex.get(k.min(tl_.tex.len().saturating_sub(1))).map(|t| t.id())
    }

    /// Pone en el grafo de `id` un nodo con la fuente `c`. Con un clip ya en medio, se le encima chico para que se vea de una vez.
    pub fn agregar_medio(&mut self, id: u64, c: ClipVideo, donde: Option<Vec2>) {
        let Some(g) = self.grafo(id) else { return };
        let antes = g.cables.iter().find(|k| k.a == SALIDA).map(|k| k.de).unwrap_or(ENTRADA);
        let (x, y) = donde.map_or((40.0, 300.0), |d| (d.x, d.y));
        let n = g.agregar(Clase::Medio, None, x, y);
        if let Some(q) = g.nodo_mut(n) {
            q.medio = Some(Box::new(c));
        }
        if donde.is_none() {
            let k = g.agregar(Clase::Fusionar, None, x + 220.0, y);
            g.insertar(antes, k);
            g.conectar(n, k, 1);
            if let Some(p) = g.nodo_mut(n).and_then(|q| q.fx.p.get_mut("escala")) {
                p.v = Valor::Num(45.0);
            }
            g.acomodar();
        }
        self.nodos.elegido = Some(n);
        self.nodos.encuadrar = donde.is_none();
    }

    /// Un archivo de la computadora como fuente de nodo.
    fn fuente_de_archivo(&mut self, r: PathBuf) -> Option<ClipVideo> {
        let imagen = es_imagen(&r);
        match crate::medios::sondear(&r) {
            Ok(i) if i.video.is_some() => {
                let (w, al, fps) = i.video?;
                let dur = if imagen { 5.0 } else { i.dur.max(0.1) };
                let mut c = self.p.nuevo_video(r, if imagen { Contenido::Imagen } else { Contenido::Archivo }, 0.0, dur);
                (c.ancho, c.alto, c.fps, c.total) = (w, al, fps, dur);
                Some(c)
            }
            Ok(_) => {
                self.avisar(t("Ese archivo no tiene imagen"), true);
                None
            }
            Err(e) => {
                self.avisar(e, true);
                None
            }
        }
    }

    fn abrir_fuente(&mut self) -> Option<ClipVideo> {
        let r = rfd::FileDialog::new().set_title(t("Elige un video o una imagen")).pick_file()?;
        self.fuente_de_archivo(r)
    }

    /// Cambia de qué sale un nodo Medio.
    pub fn cambiar_medio(&mut self, id: u64, nodo: u64, c: ClipVideo) {
        if let Some(q) = self.grafo(id).and_then(|g| g.nodo_mut(nodo)) {
            q.medio = Some(Box::new(c));
        }
    }

    fn paleta_nodos(&mut self, ui: &mut Ui, id: Option<u64>) {
        let Some(id) = id else {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| ui.label(RichText::new(t("Importa un video o imagen para usar nodos")).color(TEXT_FAINT)));
            return;
        };
        ui.add(TextEdit::singleline(&mut self.nodos.buscar).hint_text(t("Buscar nodo…")).margin(Margin::symmetric(8, 5)).desired_width(ui.available_width() - 4.0));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for (i, n) in SECCIONES.into_iter().enumerate() {
                if biblioteca::pastilla(ui, n, self.nodos.seccion == i).clicked() {
                    self.nodos.seccion = i;
                }
            }
        });
        let q = self.nodos.buscar.to_lowercase();
        let va = |n: &str| q.is_empty() || t(n).to_lowercase().contains(&q);
        let sec = self.nodos.seccion;
        let mut nuevo: Option<(Clase, Option<&str>)> = None;
        let mut pl = None;
        let mut medio = None;
        let mut abrir = false;
        let fuentes = self.fuentes_de_nodos();
        ScrollArea::vertical().id_salt(("paleta-nodos", sec)).auto_shrink([false, false]).show(ui, |ui| {
            let ancho = ui.available_width() - 8.0;
            let cols = (ancho / 88.0).floor().max(2.0);
            let w = (ancho - (cols - 1.0) * 6.0) / cols;
            let titulo = |ui: &mut Ui, n: &str| {
                ui.add_space(4.0);
                ui.label(RichText::new(t(n)).color(MAIN));
            };
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            if sec <= 1 {
                titulo(ui, "Clips e imágenes");
                ui.horizontal_wrapped(|ui| {
                    for c in fuentes.iter().filter(|c| va(&animacion::nombre_clip(c))) {
                        let n = animacion::nombre_clip(c);
                        let tex = self.mini_clip(c, c.total * 0.3);
                        if carta(ui, w, &n, "Pone este clip o imagen como nodo", &tf("Nodo {}", &[&n]), |p, r| pinta_fuente(p, r, c, tex)).clicked() {
                            medio = Some(c.clone());
                        }
                    }
                    if va("Abrir archivo…") && carta(ui, w, t("Abrir archivo…"), "Un video o imagen de tu computadora", t("Abrir archivo…"), |p, r| {
                        hueco(p, r, 6, c_(0x202020));
                        icono(p, &tabler::PLUS, Rect::from_center_size(r.center(), Vec2::splat(22.0)), MAIN);
                    }).clicked() {
                        abrir = true;
                    }
                });
            }
            if sec == 0 || sec == 2 {
                titulo(ui, "Básicos");
                ui.horizontal_wrapped(|ui| {
                    if va("Mezcla de color") && carta(ui, w, t("Mezcla de color"), "Ruedas de sombras, medios y luces, exposición, temperatura, tonos y más", &tf("Nodo {}", &[&t("Mezcla de color")]), |p, r| pinta_basico(p, r, Clase::Efecto)).clicked() {
                        nuevo = Some((Clase::Efecto, Some("bc:color")));
                    }
                    for (k, n, a) in Clase::NUEVAS {
                        if va(n) && carta(ui, w, t(n), a, &tf("Nodo {}", &[&t(n)]), |p, r| pinta_basico(p, r, k)).clicked() {
                            nuevo = Some((k, None));
                        }
                    }
                });
                titulo(ui, "Listos para empezar");
                ui.horizontal_wrapped(|ui| {
                    for (i, (n, a)) in PLANTILLAS.iter().enumerate() {
                        if va(n) && carta(ui, w, t(n), a, &tf("Nodo {}", &[&t(n)]), |p, r| pinta_plantilla(p, r, i)).clicked() {
                            pl = Some(i);
                        }
                    }
                });
            }
            if sec == 0 || sec == 3 {
                let l: Vec<_> = nombres::EFECTOS.iter().filter(|e| e.2 != "bc:color" && (va(e.1) || va(e.0))).collect();
                let mut i = 0;
                while i < l.len() {
                    let g = l[i].0;
                    let j = i + l[i..].iter().take_while(|e| e.0 == g).count();
                    titulo(ui, g);
                    // Por renglones, y solo se dibuja lo que se ve.
                    for fila in l[i..j].chunks(cols as usize) {
                        let alto = w * 9.0 / 16.0 + 20.0;
                        if !ui.is_rect_visible(Rect::from_min_size(ui.cursor().min, vec2(10.0, alto))) {
                            ui.add_space(alto);
                            continue;
                        }
                        ui.horizontal(|ui| {
                            for e in fila {
                                let tex = self.muestras.get(e.2).map(|t| t.id());
                                if carta(ui, w, t(e.1), "", &tf("Nodo {}", &[&t(e.1)]), |p, r| pinta_tex(p, r, tex)).clicked() {
                                    nuevo = Some((Clase::Efecto, Some(e.2)));
                                }
                            }
                        });
                    }
                    i = j;
                }
            }
        });
        if abrir {
            medio = self.abrir_fuente();
        }
        if let Some(c) = medio {
            self.agregar_medio(id, c, None);
        }
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

    /// Tarjetas con todo lo que puede ser fuente. Da la que se elige.
    fn elegir_fuente(&mut self, ui: &mut Ui, actual: Option<&ClipVideo>) -> Option<ClipVideo> {
        let mut r = None;
        let mut abrir = false;
        let ancho = ui.available_width() - 8.0;
        let w = (ancho - 12.0) / 3.0;
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        let fuentes = self.fuentes_de_nodos();
        ui.horizontal_wrapped(|ui| {
            for c in &fuentes {
                let n = animacion::nombre_clip(c);
                let tex = self.mini_clip(c, c.total * 0.3);
                let es = actual.is_some_and(|a| a.contenido == c.contenido && a.ruta == c.ruta);
                let resp = carta(ui, w, &n, "", &tf("Usar {}", &[&n]), |p, r| pinta_fuente(p, r, c, tex));
                if es {
                    ui.painter().rect_stroke(Rect::from_min_size(resp.rect.min, vec2(w, w * 9.0 / 16.0)), 6, Stroke::new(2.0, MAIN), StrokeKind::Outside);
                }
                if resp.clicked() {
                    r = Some(c.clone());
                }
            }
            if carta(ui, w, t("Abrir archivo…"), "Un video o imagen de tu computadora", t("Elegir archivo"), |p, r| {
                hueco(p, r, 6, c_(0x202020));
                icono(p, &tabler::PLUS, Rect::from_center_size(r.center(), Vec2::splat(22.0)), MAIN);
            }).clicked() {
                abrir = true;
            }
        });
        if abrir { self.abrir_fuente() } else { r }
    }

    /// La tira de clips de arriba del lienzo: a cuál se le están poniendo nodos.
    fn tira_de_clips(&mut self, ui: &mut Ui, id: Option<u64>) {
        let mut v: Vec<(u64, f64, usize)> = self.p.video.iter().map(|c| (c.id, c.inicio, c.capa)).collect();
        v.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.2.cmp(&b.2)));
        let mut elige = None;
        Frame::new().fill(c_(0x222222)).inner_margin(Margin::symmetric(8, 5)).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ScrollArea::horizontal().id_salt("tira-clips").auto_shrink([false, true]).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (k, ..) in v {
                        let Some(c) = self.p.video.iter().find(|c| c.id == k) else { continue };
                        let n = animacion::nombre_clip(c);
                        let (r, resp) = ui.allocate_exact_size(vec2(84.0, 47.0), Sense::click());
                        resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, id == Some(k), tf("Nodos del clip {}", &[&n])));
                        if !ui.is_rect_visible(r) {
                            continue;
                        }
                        let p = ui.painter_at(r.expand(4.0));
                        pinta_fuente(&p, r, c, self.mini_clip(c, c.dur * 0.3));
                        // Con nodos, un puntito; el nombre, abajo sobre una sombra.
                        let abajo = Rect::from_min_max(pos2(r.left(), r.bottom() - 17.0), r.max);
                        degradado_en(&p, abajo, CornerRadius { nw: 0, ne: 0, sw: 6, se: 6 }, Color32::from_black_alpha(0), Color32::from_black_alpha(220));
                        texto_corto(&p, abajo.shrink2(vec2(5.0, 1.0)), &n, 11.5, TEXT);
                        if c.nodos.is_some() {
                            p.circle(pos2(r.right() - 8.0, r.top() + 8.0), 4.0, TEAL, Stroke::new(1.0, Color32::from_black_alpha(160)));
                        }
                        vidrio(&p, r, 6, if id == Some(k) { MAIN } else { c_(0x909090) }, resp.hovered());
                        if id == Some(k) {
                            p.rect_stroke(r.expand(1.0), 7, Stroke::new(2.0, MAIN), StrokeKind::Outside);
                        }
                        if resp.on_hover_text(&n).clicked() {
                            elige = Some(k);
                        }
                    }
                });
            });
        });
        if let Some(k) = elige {
            self.clip = Some(k);
            self.seleccion.clear();
            self.seleccion.insert(k);
            self.nodos.elegido = None;
            self.nodos.encuadrar = true;
            // El cursor va al clip si no está en él, para ver lo que se le hace.
            if let Some((a, b)) = self.p.video.iter().find(|c| c.id == k).map(|c| (c.inicio, c.fin())) {
                let ahora = self.ahora();
                if ahora < a || ahora >= b {
                    self.ir(a + (b - a) * 0.3);
                }
            }
        }
    }

    fn panel_nodo(&mut self, ui: &mut Ui, id: Option<u64>) {
        self.medidores_imagen(ui);
        let ahora = self.ahora();
        let Some(id) = id else { return };
        let sel = self.nodos.elegido;
        let mut quitar = false;
        let mut elegir = false;
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
        let medio = n.clase == Clase::Medio;
        ScrollArea::vertical().id_salt("panel-nodo").auto_shrink([false, medio]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            match n.clase {
                Clase::Entrada => {
                    ui.label(RichText::new(t("La imagen del clip, ya con sus efectos normales.")).color(TEXT_FAINT));
                    ui.add_space(6.0);
                    ui.label(RichText::new(t("Para usar otro clip o imagen aparte, agrégalo de la izquierda en «Clips e imágenes». Para ponerle nodos a otro clip, elígelo en la tira de arriba del lienzo.")).color(TEXT_DIM));
                }
                Clase::Medio => elegir = true,
                Clase::Salida => {
                    ui.label(RichText::new(t("Lo que llegue aquí es lo que se ve.")).color(TEXT_FAINT));
                }
                _ => inspector::fx_ui(ui, &mut n.fx, tl),
            }
        });
        if elegir {
            self.panel_medio(ui, id, tl);
        }
        if quitar {
            self.quitar_nodo();
        }
    }

    /// Controles de un nodo «Clip o imagen»: qué pone y cómo.
    fn panel_medio(&mut self, ui: &mut Ui, id: u64, tl: f64) {
        let Some(s) = self.nodos.elegido else { return };
        ScrollArea::vertical().id_salt("panel-medio").auto_shrink([false, false]).show(ui, |ui| {
            let actual = self.p.clip_video(id).and_then(|c| c.nodos.as_mut()).and_then(|g| g.nodo_mut(s)).and_then(|n| n.medio.clone());
            ui.label(RichText::new(t("De dónde sale")).color(MAIN));
            if let Some(c) = self.elegir_fuente(ui, actual.as_deref()) {
                self.cambiar_medio(id, s, c);
            }
            ui.add_space(6.0);
            if let Some(n) = self.p.clip_video(id).and_then(|c| c.nodos.as_mut()).and_then(|g| g.nodo_mut(s)) {
                inspector::fx_ui(ui, &mut n.fx, tl);
            }
        });
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
        self.tira_de_clips(ui, id);
        let todo = ui.available_rect_before_wrap();
        let resp = ui.allocate_rect(todo, Sense::click_and_drag());
        let p = ui.painter_at(todo);
        // Mesa de trabajo: metida en el tablero, más oscura hacia abajo.
        degradado(&p, todo, 0, c_(0x2c2c2c), c_(0x1a1a1a));
        degradado(&p, Rect::from_min_size(todo.min, vec2(todo.width(), 7.0)), 0, Color32::from_black_alpha(130), Color32::from_black_alpha(0));
        let host = id.and_then(|id| self.p.video.iter().find(|c| c.id == id)).cloned();
        let (Some(id), Some(host)) = (id, host) else {
            p.text(todo.center(), Align2::CENTER_CENTER, t("Elige un clip de video"), fuente(16.0), TEXT_FAINT);
            return;
        };
        let g = host.nodos.clone().unwrap_or_default();
        let dueno = animacion::nombre_clip(&host);
        let tl = (self.ahora() - host.inicio).clamp(0.0, host.dur);
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
                    p.circle_filled(pos2(x, y), 1.0, Color32::from_white_alpha(14));
                    x += paso;
                }
                y += paso;
            }
        }
        let caja = |n: &Nodo| Rect::from_min_size(a_p(vec2(n.x, n.y)), vec2(ANCHO, alto_nodo(n)) * zoom);
        let sal = |n: &Nodo| a_p(vec2(n.x + ANCHO, n.y + CABEZA / 2.0));
        let ent = |n: &Nodo, i: usize| a_p(vec2(n.x, n.y + CABEZA + MINI + 10.0 + PUERTO * (i as f32 + 0.5)));
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
            let col = tinta(n.clase);
            let col = if n.fx.activo { col } else { mezcla(col, c_(0x505050), 0.75) };
            let rad = (9.0 * zoom).clamp(3.0, 12.0) as u8;
            // Caja de metal con la sombra larga; elegida, con su resplandor.
            for (e, a) in [(7.0, 26), (4.0, 40), (2.0, 60)] {
                p.rect_filled(r.translate(vec2(0.0, 3.0 * zoom)).expand(e * zoom), CornerRadius::same(rad + (e * zoom) as u8), Color32::from_black_alpha(a));
            }
            if sel {
                for (e, a) in [(6.0, 0.06), (3.5, 0.10), (2.0, 0.16)] {
                    p.rect_filled(r.expand(e), CornerRadius::same(rad + e as u8), MAIN.gamma_multiply(a));
                }
            }
            relieve(&p, r, rad, c_(0x4a4a4a), sel, false);
            let cab = Rect::from_min_size(r.min, vec2(r.width(), CABEZA * zoom)).shrink(1.0);
            degradado_en(&p, cab, CornerRadius { nw: rad, ne: rad, sw: 0, se: 0 }, mezcla(col, Color32::WHITE, 0.22), mezcla(col, Color32::BLACK, 0.18));
            // Reflejo de vidrio en la mitad de arriba.
            degradado_en(&p, Rect::from_min_max(cab.min, pos2(cab.right(), cab.center().y)), CornerRadius { nw: rad, ne: rad, sw: 0, se: 0 }, Color32::from_white_alpha(80), Color32::from_white_alpha(18));
            p.hline(cab.x_range(), cab.bottom(), Stroke::new(1.0, Color32::from_black_alpha(150)));
            p.hline(cab.x_range(), cab.bottom() + 1.0, Stroke::new(1.0, Color32::from_white_alpha(22)));
            let quien = if n.clase == Clase::Entrada { tf("Clip: {}", &[&dueno]) } else { nombre(n) };
            texto_corto(&p, cab.shrink2(vec2(8.0 * zoom, 0.0)).translate(vec2(0.0, 1.0)), &quien, 13.5 * zoom, Color32::from_white_alpha(70));
            texto_corto(&p, cab.shrink2(vec2(8.0 * zoom, 0.0)), &quien, 13.5 * zoom, c_(0x141414));
            // La imagen del nodo, detrás de un vidrio.
            let img = Rect::from_min_size(a_p(vec2(n.x + 8.0, n.y + CABEZA + 5.0)), vec2(ANCHO - 16.0, MINI) * zoom);
            let ri = (5.0 * zoom).clamp(2.0, 7.0) as u8;
            match n.clase {
                Clase::Entrada => pinta_fuente(&p, img, &host, self.mini_clip(&host, tl)),
                Clase::Medio => match &n.medio {
                    Some(c) => pinta_fuente(&p, img, c, self.mini_clip(c, if c.contenido == Contenido::Archivo { tl.rem_euclid(c.total.max(0.1)) } else { 0.0 })),
                    None => {
                        hueco(&p, img, ri, c_(0x1c1c1c));
                        texto_centro(&p, img, t("Elige un clip o imagen"), 12.0 * zoom, TEXT_FAINT);
                    }
                },
                Clase::Salida => pinta_tex(&p, img, self.textura.as_ref().map(|t| t.id())),
                Clase::Efecto if n.fx.tipo != "bc:color" => pinta_tex(&p, img, self.muestras.get(&n.fx.tipo).map(|t| t.id())),
                Clase::Fondo => {
                    let q = match n.fx.p.get("color").map(|p| p.en(tl)) {
                        Some(Valor::Color(q)) => q,
                        _ => [0.0, 0.0, 0.0, 1.0],
                    };
                    p.rect_filled(img, ri, Color32::from_rgb((q[0] * 255.0) as u8, (q[1] * 255.0) as u8, (q[2] * 255.0) as u8));
                }
                Clase::Mascara => pinta_mascara(&p, img, n, tl),
                k => pinta_basico(&p, img, k),
            }
            if !n.fx.activo {
                p.rect_filled(img, ri, Color32::from_black_alpha(150));
                texto_centro(&p, img, t("Apagado"), 13.0 * zoom, TEXT_DIM);
            }
            vidrio(&p, img, ri, col, false);
            for (i, e) in n.clase.entradas().iter().enumerate() {
                let o = ent(n, i);
                p.text(o + vec2(10.0 * zoom, 0.0), Align2::LEFT_CENTER, t(e), fuente(12.5 * zoom), TEXT_DIM);
                let puesto = g.cables.iter().any(|k| k.a == n.id && k.puerto == i as u8);
                enchufe(&p, o, 6.0 * zoom, if puesto { Some(MAIN) } else { None });
                puertos.push((o, n.id, Some(i as u8)));
            }
            if n.clase != Clase::Salida {
                enchufe(&p, sal(n), 6.5 * zoom, Some(if n.clase.da_mascara() { c(0x9a8ad0) } else { c(0xe8e8e8) }));
                puertos.push((sal(n), n.id, None));
            }
            if sel {
                p.rect_stroke(r, rad, Stroke::new(1.5, MAIN), StrokeKind::Outside);
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
            ui.menu_button(t("Clip o imagen"), |ui| {
                for c in self.fuentes_de_nodos() {
                    if ui.button(animacion::nombre_clip(&c)).clicked() {
                        self.agregar_medio(id, c, donde);
                    }
                }
                if ui.button(RichText::new(t("Abrir archivo…")).color(MAIN)).clicked() {
                    if let Some(c) = self.abrir_fuente() {
                        self.agregar_medio(id, c, donde);
                    }
                }
            });
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
            // Un archivo del panel: entra como clip o imagen.
            let rutas = match &*s {
                biblioteca::Soltar::Medios(v) => v.clone(),
                biblioteca::Soltar::Medio(r) => vec![r.clone()],
                _ => vec![],
            };
            for r in rutas {
                if let Some(c) = self.fuente_de_archivo(r) {
                    self.agregar_medio(id, c, raton.map(a_g));
                }
            }
        }
    }
}

fn alto_nodo(n: &Nodo) -> f32 {
    CABEZA + MINI + 10.0 + PUERTO * n.clase.entradas().len() as f32 + if n.clase.entradas().is_empty() { 2.0 } else { 4.0 }
}
