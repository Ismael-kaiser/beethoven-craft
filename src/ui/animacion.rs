//! Pestaña ANIMACIÓN: elementos y animaciones listas a la izquierda, vista previa con trayectoria al centro, valores a
//! la derecha y abajo las pistas: una por propiedad o control de efecto, con sus cuadros clave y curvas, o la gráfica.
use super::biblioteca::pastilla;
use super::estilo::*;
use super::*;
use crate::animaciones::{self, ANIMACIONES, Animacion, Momento, Pista, Propiedad};
use crate::curvas::{Curva, Familia, Sentido};
use crate::idioma::{t, tf};
use crate::nombres;

/// Ancho de la cabecera de cada pista.
const CAB: f32 = 230.0;
const REGLA: f32 = 24.0;
const FILA_CLIP: f32 = 28.0;
const FILA: f32 = 25.0;

/// Cuadro clave: clip, pista y su segundo dentro del clip.
pub type Clave = (u64, Pista, f64);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Panel2 {
    Elementos,
    Listas,
    Texto,
}

pub struct Estado {
    /// El clip elegido en la lista.
    pub clip: Option<u64>,
    pub claves: Vec<Clave>,
    pub panel: Panel2,
    /// Cambiar un valor pone un cuadro clave aunque la propiedad no esté animada.
    pub auto: bool,
    /// Lo mismo en la pestaña Animación, donde viene prendido.
    pub grabar: bool,
    /// Un clic en la barra del objeto pone una llave ahí.
    pub clic_pone: bool,
    /// Lo que cambió en este cuadro no es una edición (deshacer, abrir).
    pub callar: bool,
    pub grafica: bool,
    /// Pista que se ve en la gráfica.
    pub pista: Option<(u64, Pista)>,
    pub zoom: f64,
    pub x0: f64,
    y0: f32,
    cerrados: HashSet<u64>,
    arrastre: Option<Arrastre>,
    copiadas: Vec<(Pista, f64, Valor, Curva)>,
    menu: Option<Clave>,
    /// Clic derecho en una llave o en la barra de un objeto.
    menu_momento: Option<(u64, f64)>,
    encuadrar: bool,
    pub buscar: String,
    pub libre: super::libre::Libre,
}

impl Default for Estado {
    fn default() -> Self {
        Estado {
            clip: None,
            claves: vec![],
            panel: Panel2::Listas,
            auto: false,
            grabar: true,
            clic_pone: false,
            callar: false,
            grafica: false,
            pista: None,
            zoom: 120.0,
            x0: 0.0,
            y0: 0.0,
            cerrados: HashSet::new(),
            arrastre: None,
            copiadas: vec![],
            menu: None,
            menu_momento: None,
            encuadrar: true,
            buscar: String::new(),
            libre: Default::default(),
        }
    }
}

enum Arrastre {
    Cursor,
    /// Caja de selección desde este punto.
    Caja(Pos2),
    /// Claves que se mueven: (clip, pista, segundo original, segundo actual) y el punto de partida.
    Claves(Vec<(Clave, f64)>, Pos2),
    /// En la gráfica: la clave, y el punto de control (0 o 1) de la curva.
    Punto(Clave),
    Control(Clave, usize),
    /// Todas las llaves de un momento del objeto: de dónde.
    Momento(u64, f64),
    /// Mueve o recorta el clip: id, qué (0 mover, 1 izquierda, 2 derecha), desfase.
    Clip(u64, u8, f64),
}

enum Hacer {
    Ir(f64),
    Agregar(u64, Pista, f64),
    /// Llave con todo, en ese segundo del clip.
    Guardar(u64, f64),
    Curva(Curva),
    Borrar,
    Quitar(u64),
}

pub fn nombre_clip(c: &ClipVideo) -> String {
    match &c.contenido {
        Contenido::Titulo(x) => x.texto.lines().next().unwrap_or("").chars().take(28).collect(),
        Contenido::Ajuste => t("Clip vacío").into(),
        Contenido::Color(_) => t("Fondo de color").into(),
        Contenido::Forma(f) => t(TipoForma::TODOS.iter().find(|x| x.0 == f.tipo).map_or("Figura", |x| x.1)).into(),
        _ => nombre_de(&c.ruta),
    }
}

fn nombre_pista(c: &ClipVideo, p: &Pista) -> String {
    match p {
        Pista::Mov(p) => t(p.nombre()).into(),
        Pista::Campo(k) => t(c.campos().iter().find(|x| x.0 == k).map_or("", |x| x.1)).into(),
        Pista::Fx(id, k) => match c.fx.iter().find(|f| f.id == *id) {
            Some(f) => format!("{} · {}", t(nombres::efecto(&f.tipo)), nombres::control(&f.tipo, k)),
            None => k.clone(),
        },
    }
}

fn ambito(p: &Pista) -> (f64, f64) {
    match p {
        Pista::Mov(Propiedad::X | Propiedad::Y) => (-100.0, 100.0),
        Pista::Mov(Propiedad::Giro) => (-180.0, 180.0),
        Pista::Mov(Propiedad::Escala | Propiedad::Ancho | Propiedad::Alto) => (0.0, 200.0),
        _ => (0.0, 100.0),
    }
}

/// Curvas a un clic.
pub fn rapidas() -> [(&'static str, Curva); 8] {
    use Familia::*;
    use Sentido::*;
    [
        ("Lineal", Curva::Lineal),
        ("Suave", Curva::Suave),
        ("Al llegar", Curva::Familia(Cubic, Salir)),
        ("Al arrancar", Curva::Familia(Cubic, Entrar)),
        ("Impulso", Curva::Familia(Back, Salir)),
        ("Rebote", Curva::Familia(Bounce, Salir)),
        ("Elástica", Curva::Familia(Elastic, Salir)),
        ("Salto", Curva::Salto),
    ]
}

fn pinta_curva(p: &Painter, r: Rect, cv: Curva, baja: bool, col: Color32) {
    let pts: Vec<Pos2> = (0..=28)
        .map(|i| {
            let k = i as f64 / 28.0;
            let y = (cv.aplicar(k) as f32).clamp(-0.35, 1.35);
            pos2(r.left() + r.width() * k as f32, r.bottom() - if baja { 1.0 - y } else { y } * r.height())
        })
        .collect();
    p.add(Shape::line(pts, Stroke::new(1.5, col)));
}

fn rombo(p: &Painter, o: Pos2, r: f32, col: Color32) {
    p.add(Shape::convex_polygon(vec![o - vec2(0.0, r), o + vec2(r, 0.0), o + vec2(0.0, r), o - vec2(r, 0.0)], col, Stroke::new(1.0, Color32::BLACK)));
}

impl App {
    /// Manda un clip a la pestaña de animación y la abre.
    pub fn animar(&mut self, id: u64) {
        if let Some(c) = self.p.clip_video(id) {
            c.animado = true;
        }
        if !self.pestanas.contains(&Espacio::Animacion) {
            self.pestanas.push(Espacio::Animacion);
        }
        self.espacio = Espacio::Animacion;
        self.anim.clip = Some(id);
        self.anim.encuadrar = true;
        self.clip = Some(id);
        self.anim.panel = Panel2::Listas;
    }

    pub fn espacio_animacion(&mut self, ui: &mut Ui) {
        // El clip elegido en cualquier pestaña es el que se anima; el de la lista se recuerda.
        if let Some(id) = self.clip.filter(|id| self.p.video.iter().any(|c| c.id == *id)) {
            self.anim.clip = Some(id);
        }
        self.anim.clip = self.anim.clip.filter(|id| self.p.video.iter().any(|c| c.id == *id));
        if self.anim.clip.is_none() {
            self.anim.clip = self.p.video.iter().find(|c| c.animado).map(|c| c.id);
        }
        self.clip = self.anim.clip.or(self.clip);
        let alto = ui.available_height();
        egui::Panel::bottom("anim-pistas").frame(Frame::new().fill(WORKSPACE)).resizable(true).default_size(alto * 0.46).size_range(190.0..=(alto - 170.0).max(220.0)).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            self.barra_animacion(ui);
            if self.anim.grafica {
                self.grafica_animacion(ui);
            } else {
                self.pistas_animacion(ui);
            }
        });
        egui::Panel::left("anim-izq").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(8.0)).resizable(true).default_size(330.0).min_size(260.0).show(ui, |ui| self.panel_animacion(ui));
        egui::Panel::right("anim-der").frame(Frame::new().fill(WINDOW_TITLE)).resizable(true).default_size(350.0).min_size(270.0).show(ui, |ui| {
            if self.anim.clip.is_some() {
                self.inspector(ui);
            } else {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| ui.label(RichText::new(t("Elige un elemento para ver sus valores")).color(TEXT_FAINT)));
            }
        });
        CentralPanel::default().frame(Frame::new().fill(Color32::BLACK)).show(ui, |ui| self.preview(ui));
        if self.anim.grabar {
            self.claves_automaticas();
        }
    }

    /// Lo que se le cambió al objeto elegido desde el cuadro pasado (mover, girar, color, un control…) queda en llaves.
    pub fn claves_automaticas(&mut self) {
        if std::mem::take(&mut self.anim.callar) {
            return;
        }
        let Some(a) = self.aplicado.clone() else { return };
        let Some(antes) = self.clip.and_then(|id| a.video.iter().find(|c| c.id == id)) else { return };
        // Recortar o mover el clip en el tiempo no es animarlo.
        let tl = self.a_cuadro(self.ahora() - antes.inicio).clamp(0.0, antes.dur);
        let Some(c) = self.p.clip_video(antes.id).filter(|c| c.inicio == antes.inicio && c.dur == antes.dur) else { return };
        if animaciones::grabar_cambios(antes, c, tl) {
            self.avisar(tf("◆ Llave guardada en {}", &[&tiempo(antes.inicio + tl)]), false);
        }
    }

    // ---- izquierda

    fn panel_animacion(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            for (p, n) in [(Panel2::Elementos, "Elementos"), (Panel2::Listas, "Animaciones"), (Panel2::Texto, "Texto")] {
                if pastilla(ui, n, self.anim.panel == p).clicked() {
                    self.anim.panel = p;
                }
            }
        });
        ui.add_space(6.0);
        ScrollArea::vertical().id_salt(("anim-panel", self.anim.panel as u8)).auto_shrink([false, false]).show(ui, |ui| match self.anim.panel {
            Panel2::Elementos => self.panel_elementos(ui),
            Panel2::Listas => self.panel_listas(ui),
            Panel2::Texto => self.panel_texto(ui),
        });
    }

    fn panel_elementos(&mut self, ui: &mut Ui) {
        ui.label(RichText::new(t("Agregar")).font(barlow(17.0)).color(TEXT));
        ui.add_space(3.0);
        let mut nuevo = None;
        ui.horizontal_wrapped(|ui| {
            if ui.button(t("Texto")).clicked() {
                let t0 = self.ahora();
                self.poner_titulo(0, t0, None);
                nuevo = self.clip;
            }
            if ui.button(t("Imagen…")).clicked() {
                self.menu_importar();
            }
            ui.menu_button(t("Figura"), |ui| {
                for (f, n) in TipoForma::TODOS {
                    if ui.button(t(n)).clicked() {
                        nuevo = Some(self.poner_forma(f));
                    }
                }
            });
            if ui.button(t("Fondo de color")).clicked() {
                let (t0, dur) = (self.ahora(), 5.0);
                let capa = self.capa_libre(t0, dur);
                let mut c = self.p.nuevo_video("Color".into(), Contenido::Color([0.16, 0.2, 0.36, 1.0]), t0, dur);
                c.capa = capa;
                nuevo = Some(c.id);
                self.p.video.push(c);
            }
            if ui.button(t("Clip vacío")).on_hover_text(t(linea::AYUDA_VACIO)).clicked() {
                let (t0, dur) = self.hueco_vacio();
                nuevo = Some(self.clip_vacio(t0, dur));
            }
        });
        if let Some(id) = nuevo {
            self.animar(id);
            self.anim.panel = Panel2::Elementos;
        }
        ui.add_space(12.0);
        ui.label(RichText::new(t("En el proyecto")).font(barlow(17.0)).color(TEXT));
        ui.label(RichText::new(t("Clic para animarlo.")).size(12.5).color(TEXT_FAINT));
        ui.add_space(3.0);
        let mut v: Vec<u64> = self.p.video.iter().map(|c| c.id).collect();
        v.sort_by(|a, b| {
            let ini = |id: &u64| self.p.video.iter().find(|c| c.id == *id).map_or(0.0, |c| c.inicio);
            ini(a).total_cmp(&ini(b))
        });
        if v.is_empty() {
            ui.label(RichText::new(t("Aún no hay nada. Agrega algo de arriba.")).color(TEXT_FAINT));
        }
        let ancho = ui.available_width() - 8.0;
        let cols = (ancho / 96.0).floor().max(2.0);
        let w = (ancho - (cols - 1.0) * 6.0) / cols;
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        let mut elige = None;
        ui.horizontal_wrapped(|ui| {
            for id in v {
                let Some(c) = self.p.video.iter().find(|c| c.id == id) else { continue };
                let on = self.anim.clip == Some(id);
                let n = nombre_clip(c);
                let llaves = animaciones::momentos(c).len();
                let tex = self.mini_clip(c, c.dur * 0.3);
                let r = super::nodos::carta(ui, w, &n, "", &format!("Animar {n}"), |p, r| super::nodos::pinta_fuente(p, r, c, tex));
                let img = Rect::from_min_size(r.rect.min, vec2(w, (w * 9.0 / 16.0).round()));
                let p = ui.painter();
                if llaves > 0 {
                    // Cuántas llaves tiene.
                    let g = p.layout_no_wrap(llaves.to_string(), barlow(11.5), COLORED_DARK);
                    let q = Rect::from_min_size(img.min + vec2(4.0, 4.0), g.size() + vec2(20.0, 2.0));
                    encendido(p, q, 5, MAIN);
                    rombo(p, pos2(q.left() + 8.5, q.center().y), 3.8, COLORED_DARK);
                    p.galley(pos2(q.left() + 15.0, q.center().y - g.size().y / 2.0), g, COLORED_DARK);
                }
                if on {
                    p.rect_stroke(img.expand(1.0), 7, Stroke::new(2.0, MAIN), StrokeKind::Outside);
                }
                if r.on_hover_text(tiempo(c.inicio)).clicked() {
                    elige = Some((id, c.inicio, c.fin()));
                }
            }
        });
        if let Some((id, a, b)) = elige {
            self.anim.clip = Some(id);
            self.clip = Some(id);
            self.anim.encuadrar = true;
            if let Some(c) = self.p.clip_video(id) {
                c.animado = true;
            }
            let ahora = self.ahora();
            if ahora < a || ahora >= b {
                self.ir(a.max(0.0));
            }
        }
    }

    pub fn poner_forma(&mut self, f: TipoForma) -> u64 {
        let (t0, dur) = (self.ahora(), 5.0);
        let capa = self.capa_libre(t0, dur);
        let mut c = self.p.nuevo_video("Figura".into(), Contenido::Forma(Forma { tipo: f, ..Default::default() }), t0, dur);
        c.capa = capa;
        let id = c.id;
        self.p.video.push(c);
        self.clip = Some(id);
        id
    }

    pub fn poner_animacion(&mut self, id: u64, a: &Animacion) {
        let tl = self.ahora();
        let mut ids: Vec<u64> = (0..a.fx.len()).map(|_| self.p.id()).collect();
        if let Some(c) = self.p.clip_video(id) {
            let tl = tl - c.inicio;
            animaciones::poner_en(c, a, tl, || ids.pop().unwrap_or(0));
            c.animado = true;
        }
        self.avisar(tf("Animación «{}» puesta", &[&t(a.nombre)]), false);
    }

    fn panel_listas(&mut self, ui: &mut Ui) {
        let Some(id) = self.anim.clip else {
            ui.label(RichText::new(t("Agrega o elige un elemento y después una animación.")).color(TEXT_FAINT));
            if ui.button(t("Ver elementos")).clicked() {
                self.anim.panel = Panel2::Elementos;
            }
            return;
        };
        ui.add(TextEdit::singleline(&mut self.anim.buscar).hint_text(t("Buscar animación…")).margin(Margin::symmetric(8, 5)).desired_width(ui.available_width() - 8.0));
        ui.label(RichText::new(t("Clic para ponerla; puedes juntar varias. Pasa el ratón para verla.")).size(12.5).color(TEXT_FAINT));
        let q = self.anim.buscar.to_lowercase();
        let ancho = ui.available_width() - 6.0;
        let n = (ancho / 104.0).floor().max(1.0);
        let w = (ancho - (n - 1.0) * 6.0) / n;
        let mut poner = None;
        for (m, titulo) in [(Momento::Entrada, "Al entrar"), (Momento::Enfasis, "Énfasis en el cursor"), (Momento::Bucle, "Todo el tiempo"), (Momento::Salida, "Al salir")] {
            let v: Vec<&Animacion> = ANIMACIONES.iter().filter(|a| a.momento == m && (q.is_empty() || t(a.nombre).to_lowercase().contains(&q))).collect();
            if v.is_empty() {
                continue;
            }
            ui.add_space(7.0);
            ui.label(RichText::new(t(titulo)).color(MAIN).size(14.5));
            for fila in v.chunks(n as usize) {
                let (r, _) = ui.allocate_exact_size(vec2(ancho, 74.0), Sense::hover());
                if !ui.is_rect_visible(r) {
                    continue;
                }
                for (k, a) in fila.iter().enumerate() {
                    let caja = Rect::from_min_size(pos2(r.left() + k as f32 * (w + 6.0), r.top()), vec2(w, 70.0));
                    if tarjeta_anim(ui, caja, a) {
                        poner = Some(*a);
                    }
                }
            }
        }
        if let Some(a) = poner {
            self.poner_animacion(id, a);
        }
        ui.add_space(10.0);
        if ui.button(t("Quitar toda la animación de este clip")).clicked() {
            let ahora = self.ahora();
            if let Some(c) = self.p.clip_video(id) {
                let tl = (ahora - c.inicio).clamp(0.0, c.dur);
                animaciones::quitar_todo(c, tl);
            }
            self.anim.claves.clear();
        }
    }

    fn panel_texto(&mut self, ui: &mut Ui) {
        let id = self.anim.clip;
        let Some(c) = id.and_then(|id| self.p.clip_video(id)) else {
            ui.label(RichText::new(t("Elige un texto para animarlo letra por letra.")).color(TEXT_FAINT));
            return;
        };
        let Contenido::Titulo(x) = &mut c.contenido else {
            ui.label(RichText::new(t("Este elemento no es texto. Elige un texto para animarlo letra por letra.")).color(TEXT_FAINT));
            return;
        };
        ui.label(RichText::new(t("Texto")).font(barlow(17.0)).color(TEXT));
        ui.add(TextEdit::multiline(&mut x.texto).desired_rows(2).desired_width(ui.available_width() - 8.0).font(fuente(16.0)));
        ui.add_space(8.0);
        ui.label(RichText::new(t("Ajustes rápidos")).color(MAIN).size(14.5));
        let base = AnimTexto::default();
        let listos: [(&str, AnimTexto); 10] = [
            ("Máquina de escribir", AnimTexto { entrada: MovTexto::Maquina, dur: 0.05, paso: 0.07, ..base }),
            ("Cascada", AnimTexto { entrada: MovTexto::Bajar, ..base }),
            ("Olas", AnimTexto { entrada: MovTexto::Aparecer, bucle: BucleTexto::Ola, ..base }),
            ("Letras que crecen", AnimTexto { entrada: MovTexto::Crecer, ..base }),
            ("Rebotes", AnimTexto { entrada: MovTexto::Rebotar, dur: 0.9, paso: 0.07, ..base }),
            ("Palabra por palabra", AnimTexto { entrada: MovTexto::Subir, por: PorTexto::Palabra, paso: 0.22, ..base }),
            ("Renglones de lado", AnimTexto { entrada: MovTexto::Lado, salida: MovTexto::Lado, por: PorTexto::Renglon, paso: 0.2, dur: 0.6, ..base }),
            ("Terror", AnimTexto { entrada: MovTexto::Azar, bucle: BucleTexto::Temblor, paso: 0.09, ..base }),
            ("Fiesta", AnimTexto { entrada: MovTexto::Girar, bucle: BucleTexto::Arcoiris, ..base }),
            ("Entra y sale", AnimTexto { entrada: MovTexto::Subir, salida: MovTexto::Bajar, ..base }),
        ];
        ui.horizontal_wrapped(|ui| {
            for (n, a) in listos {
                if pastilla(ui, n, x.anim == Some(a)).clicked() {
                    x.anim = Some(a);
                }
            }
            if pastilla(ui, "Sin animación de letras", x.anim.is_none()).clicked() {
                x.anim = None;
            }
        });
        ui.add_space(10.0);
        let mut a = x.anim.unwrap_or(AnimTexto { entrada: MovTexto::Ninguno, ..base });
        let antes = a;
        ui.label(RichText::new(t("A tu gusto")).color(MAIN).size(14.5));
        Grid::new("anim-texto").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
            let nombre = |m: MovTexto| t(MovTexto::TODOS.iter().find(|x| x.0 == m).map_or("", |x| x.1));
            for (n, m) in [("Al entrar", &mut a.entrada), ("Al salir", &mut a.salida)] {
                ui.label(t(n));
                ComboBox::from_id_salt(("at", n)).width(170.0).selected_text(nombre(*m)).show_ui(ui, |ui| {
                    for (v, n) in MovTexto::TODOS {
                        ui.selectable_value(m, v, t(n));
                    }
                });
                ui.end_row();
            }
            ui.label(t("Mientras"));
            ComboBox::from_id_salt("at-bucle").width(170.0).selected_text(t(BucleTexto::TODOS.iter().find(|x| x.0 == a.bucle).map_or("", |x| x.1))).show_ui(ui, |ui| {
                for (v, n) in BucleTexto::TODOS {
                    ui.selectable_value(&mut a.bucle, v, t(n));
                }
            });
            ui.end_row();
            ui.label(t("Por"));
            ui.horizontal(|ui| {
                for (v, n) in [(PorTexto::Letra, "Letra"), (PorTexto::Palabra, "Palabra"), (PorTexto::Renglon, "Renglón")] {
                    if pastilla(ui, n, a.por == v).clicked() {
                        a.por = v;
                    }
                }
            });
            ui.end_row();
            for (n, v, max) in [("Cada pieza tarda", &mut a.dur, 3.0), ("Entre piezas", &mut a.paso, 1.0)] {
                ui.label(t(n));
                ui.add(Slider::new(v, 0.01..=max).suffix(" s").max_decimals(2));
                ui.end_row();
            }
            ui.label(t("Fuerza"));
            ui.add(Slider::new(&mut a.fuerza, 0.0..=4.0).max_decimals(2));
            ui.end_row();
        });
        if a != antes {
            x.anim = Some(a);
            c.animado = true;
        }
    }

    // ---- abajo

    fn barra_animacion(&mut self, ui: &mut Ui) {
        Frame::new().fill(TOOLBAR).inner_margin(Margin::symmetric(8, 4)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.set_min_height(28.0);
                ui.set_min_width(ui.available_width());
                if pastilla(ui, "Pistas", !self.anim.grafica).clicked() {
                    self.anim.grafica = false;
                }
                if pastilla(ui, "Gráfica", self.anim.grafica).clicked() {
                    self.anim.grafica = true;
                }
                ui.separator();
                if boton_color(ui, &iconos::CARET_LEFT, false, "Cuadro clave anterior", GLYPH, 26.0).clicked() {
                    self.saltar_clave(-1);
                }
                // El botón grande: guarda el objeto como está.
                let hay = self.anim.clip.is_some();
                let n = t("Guardar llave");
                let gal = ui.painter().layout_no_wrap(n.to_string(), fuente(14.5), COLORED_DARK);
                let (r, resp) = ui.allocate_exact_size(vec2(gal.size().x + 38.0, 26.0), if hay { Sense::click() } else { Sense::hover() });
                resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, hay, n));
                if hay && !resp.is_pointer_button_down_on() { encendido(ui.painter(), r, 6, if resp.hovered() { HIGHLIGHT } else { MAIN }) } else { relieve(ui.painter(), r, 6, if hay { DEEP } else { BUTTON }, false, hay) }
                rombo(ui.painter(), pos2(r.left() + 15.0, r.center().y), 5.5, if hay { COLORED_DARK } else { TEXT_FAINT });
                ui.painter().galley(pos2(r.left() + 28.0, r.center().y - gal.size().y / 2.0), gal, if hay { COLORED_DARK } else { TEXT_FAINT });
                if resp.on_hover_text(t("Guarda en el cursor todo lo que tiene el objeto: lugar, tamaño, giro, transparencia, color, recorte y efectos animados (Shift+K)")).clicked() {
                    self.clave_de_todo();
                }
                if boton_color(ui, &iconos::CARET_RIGHT, false, "Cuadro clave siguiente", GLYPH, 26.0).clicked() {
                    self.saltar_clave(1);
                }
                let on = self.anim.clic_pone;
                if pastilla(ui, "◆ Con clic", on).on_hover_text(t("Con esto prendido, cada clic en la barra del objeto guarda una llave ahí. Apagado: doble clic.")).clicked() {
                    self.anim.clic_pone = !on;
                }
                let on = self.anim.grabar;
                let (r, resp) = ui.allocate_exact_size(vec2(122.0, 26.0), Sense::click());
                resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, on, t("Grabar claves")));
                relieve(ui.painter(), r, 6, if on { mezcla(RECORD, BUTTON, 0.55) } else { BUTTON }, resp.hovered(), on || resp.is_pointer_button_down_on());
                ui.painter().circle_filled(pos2(r.left() + 14.0, r.center().y), 5.0, if on { RECORD } else { mezcla(RECORD, BUTTON, 0.6) });
                if on {
                    ui.painter().circle_filled(pos2(r.left() + 14.0, r.center().y), 9.0, RECORD.gamma_multiply(0.18));
                }
                ui.painter().text(pos2(r.left() + 26.0, r.center().y), Align2::LEFT_CENTER, t("Grabar claves"), fuente(14.0), TEXT);
                if resp.on_hover_text(t("Con esto prendido, cualquier cambio (mover en la vista, girar, un control de efecto) pone un cuadro clave en el cursor")).clicked() {
                    self.anim.grabar = !on;
                }
                ui.separator();
                // Curva de lo elegido
                let elegida = self.anim.claves.first().cloned().and_then(|k| {
                    let c = self.p.video.iter().find(|c| c.id == k.0)?;
                    Some(k.1.de(c)?.curva_en(k.2))
                });
                for (n, cv) in rapidas() {
                    let (r, resp) = ui.allocate_exact_size(vec2(30.0, 26.0), if elegida.is_some() { Sense::click() } else { Sense::hover() });
                    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, elegida.is_some(), format!("Curva {n}")));
                    let on = elegida == Some(cv);
                    if on { encendido(ui.painter(), r, 5, MAIN) } else { relieve(ui.painter(), r, 5, BUTTON, resp.hovered(), resp.is_pointer_button_down_on()) }
                    pinta_curva(&ui.painter_at(r), r.shrink2(vec2(6.0, 7.0)), cv, false, if on { COLORED_DARK } else if elegida.is_some() { TEXT } else { TEXT_FAINT });
                    if resp.on_hover_text(tf("{}: hasta el siguiente cuadro clave", &[&t(n)])).clicked() {
                        self.curva_a_elegidas(cv);
                    }
                }
                ui.menu_button(t("Más curvas"), |ui| {
                    ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                        for cv in Curva::menu() {
                            if ui.add_enabled(elegida.is_some(), Button::selectable(elegida == Some(cv), cv.nombre())).clicked() {
                                self.curva_a_elegidas(cv);
                            }
                        }
                    });
                });
                ui.separator();
                let hay = !self.anim.claves.is_empty();
                if ui.add_enabled(hay, Button::new(t("Copiar"))).on_hover_text(t("Copia los cuadros clave elegidos")).clicked() {
                    self.copiar_claves();
                }
                if ui.add_enabled(!self.anim.copiadas.is_empty(), Button::new(t("Pegar"))).on_hover_text(t("En el cursor, en el clip elegido")).clicked() {
                    self.pegar_claves();
                }
                if ui.add_enabled(hay, Button::new(t("Borrar"))).clicked() {
                    self.borrar_claves();
                }
                self.menu_animar_fx(ui);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button(t("Ver clip")).on_hover_text(t("Ajusta la vista al clip elegido")).clicked() {
                        self.anim.encuadrar = true;
                    }
                    ui.spacing_mut().slider_width = 80.0;
                    let mut z = self.anim.zoom.ln();
                    if ui.add(Slider::new(&mut z, 2f64.ln()..=1500f64.ln()).show_value(false)).changed() {
                        self.anim.zoom = z.exp();
                    }
                });
            });
        });
    }

    /// Menú para animar cualquier control de los efectos del clip.
    fn menu_animar_fx(&mut self, ui: &mut Ui) {
        let Some(c) = self.anim.clip.and_then(|id| self.p.video.iter().find(|c| c.id == id)).cloned() else { return };
        ui.menu_button(t("Animar efecto"), |ui| {
            if c.fx.is_empty() {
                ui.label(RichText::new(t("Este clip no tiene efectos. Ponle uno en la pestaña Efectos.")).color(TEXT_FAINT));
                if ui.button(t("Ir a Efectos")).clicked() {
                    self.espacio = Espacio::Efectos;
                }
            }
            for f in &c.fx {
                ui.menu_button(t(nombres::efecto(&f.tipo)), |ui| {
                    ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                        for (k, p) in &f.p {
                            // Las opciones de lista y los textos saltan; se animan números, colores, puntos y encendidos.
                            if matches!(p.v, Valor::Texto(_)) {
                                continue;
                            }
                            let on = !p.claves.is_empty();
                            if ui.selectable_label(on, nombres::control(&f.tipo, k)).clicked() {
                                let tl = (self.ahora() - c.inicio).clamp(0.0, c.dur);
                                let pista = Pista::Fx(f.id, k.clone());
                                if let Some(q) = self.p.clip_video(c.id).and_then(|c| pista.de_mut(c)) {
                                    if q.clave_en(tl).is_none() {
                                        q.alternar_clave(tl);
                                    }
                                }
                                self.anim.pista = Some((c.id, pista));
                                if let Some(c) = self.p.clip_video(c.id) {
                                    c.animado = true;
                                }
                                ui.close();
                            }
                        }
                    });
                });
            }
        });
    }

    pub fn saltar_clave(&mut self, dir: i32) {
        let Some(c) = self.anim.clip.and_then(|id| self.p.video.iter().find(|c| c.id == id)) else { return };
        let t0 = self.ahora();
        let mut v: Vec<f64> = Pista::todas(c).iter().filter_map(|p| p.de(c)).flat_map(|p| p.claves.iter().map(|k| k.0 + c.inicio)).collect();
        v.sort_by(f64::total_cmp);
        let x = if dir > 0 { v.into_iter().find(|x| *x > t0 + 0.02) } else { v.into_iter().rev().find(|x| *x < t0 - 0.02) };
        if let Some(x) = x {
            self.ir(x);
        }
    }

    pub fn clave_de_todo(&mut self) {
        let t0 = self.ahora();
        let Some(c) = self.anim.clip.and_then(|id| self.p.clip_video(id)) else { return };
        let tl = (t0 - c.inicio).clamp(0.0, c.dur);
        let id = c.id;
        self.guardar_llave_en(id, tl);
    }

    /// Guarda todo lo que el objeto tiene en `tl` (segundo del clip).
    pub fn guardar_llave_en(&mut self, id: u64, tl: f64) {
        let tl = self.a_cuadro(tl);
        let Some(c) = self.p.clip_video(id) else { return };
        let tl = tl.clamp(0.0, c.dur);
        animaciones::guardar_llave(c, tl);
        let t0 = c.inicio + tl;
        self.avisar(tf("◆ Llave guardada en {}", &[&tiempo(t0)]), false);
    }

    fn curva_a_elegidas(&mut self, cv: Curva) {
        for (id, p, tk) in self.anim.claves.clone() {
            if let Some(q) = self.p.clip_video(id).and_then(|c| p.de_mut(c)) {
                q.poner_curva(tk, cv);
            }
        }
    }

    pub fn copiar_claves(&mut self) {
        let v: Vec<(Pista, f64, Valor, Curva)> = self
            .anim
            .claves
            .iter()
            .filter_map(|(id, p, tk)| {
                let q = p.de(self.p.video.iter().find(|c| c.id == *id)?)?;
                Some((p.clone(), *tk, q.claves[q.clave_en(*tk)?].1.clone(), q.curva_en(*tk)))
            })
            .collect();
        let t0 = v.iter().map(|x| x.1).fold(f64::MAX, f64::min);
        self.anim.copiadas = v.into_iter().map(|(p, tk, v, c)| (p, tk - t0, v, c)).collect();
    }

    pub fn pegar_claves(&mut self) {
        let t0 = self.ahora();
        let copia = self.anim.copiadas.clone();
        let Some(c) = self.anim.clip.and_then(|id| self.p.clip_video(id)) else { return };
        let tl = t0 - c.inicio;
        let (id, dur) = (c.id, c.dur);
        let mut nuevas = vec![];
        for (p, dt, v, cv) in copia {
            let tk = tl + dt;
            if !(0.0..=dur).contains(&tk) {
                continue;
            }
            // Un control de efecto se pega en el mismo efecto si el clip lo tiene.
            if let Some(q) = p.de_mut(c) {
                match q.clave_en(tk) {
                    Some(i) => q.claves[i].1 = v,
                    None => {
                        q.claves.push((tk, v));
                        q.claves.sort_by(|a, b| a.0.total_cmp(&b.0));
                    }
                }
                q.poner_curva(tk, cv);
                nuevas.push((id, p, tk));
            }
        }
        c.animado = true;
        self.anim.claves = nuevas;
    }

    pub fn borrar_claves(&mut self) {
        for (id, p, tk) in std::mem::take(&mut self.anim.claves) {
            if let Some(q) = self.p.clip_video(id).and_then(|c| p.de_mut(c)) {
                if q.clave_en(tk).is_some() {
                    q.alternar_clave(tk);
                }
            }
        }
    }

    /// Escala de tiempo, regla y cursor compartidos por las pistas y la gráfica.
    fn regla_animacion(&mut self, ui: &mut Ui, todo: Rect, resp: &Response) -> Rect {
        let carril = Rect::from_min_max(pos2(todo.left() + CAB, todo.top()), todo.max);
        if std::mem::take(&mut self.anim.encuadrar) {
            let (a, b) = self.anim.clip.and_then(|id| self.p.video.iter().find(|c| c.id == id)).map_or((0.0, self.p.fin().max(5.0)), |c| (c.inicio, c.fin()));
            let d = (b - a).max(0.5);
            self.anim.zoom = (carril.width() as f64 * 0.9 / d).clamp(2.0, 1500.0);
            self.anim.x0 = (a - d * 0.05).max(-0.3);
        }
        if resp.contains_pointer() {
            let (z, s) = ui.input(|i| (i.zoom_delta() as f64, i.smooth_scroll_delta()));
            if z != 1.0 {
                if let Some(m) = ui.input(|i| i.pointer.hover_pos()) {
                    let tm = self.anim.x0 + (m.x - carril.left()) as f64 / self.anim.zoom;
                    self.anim.zoom = (self.anim.zoom * z).clamp(2.0, 1500.0);
                    self.anim.x0 = tm - (m.x - carril.left()) as f64 / self.anim.zoom;
                }
            }
            self.anim.x0 = (self.anim.x0 - s.x as f64 / self.anim.zoom).max(-0.3);
            self.anim.y0 = (self.anim.y0 - s.y).max(0.0);
        }
        let (zoom, x0) = (self.anim.zoom, self.anim.x0);
        let p = ui.painter_at(todo);
        p.rect_filled(todo, 0, WORKSPACE);
        let regla = Rect::from_min_max(pos2(carril.left(), todo.top()), pos2(carril.right(), todo.top() + REGLA));
        p.rect_filled(Rect::from_min_max(todo.min, regla.max), 0, RULER);
        p.text(pos2(todo.left() + 10.0, regla.center().y), Align2::LEFT_CENTER, tiempo(self.ahora()), fuente(15.0), MAIN);
        let pr = ui.painter_at(carril);
        let paso = [0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0].into_iter().find(|s| s * zoom > 64.0).unwrap_or(120.0);
        let mut k = (x0 / paso).floor().max(0.0);
        while ((k * paso - x0) * zoom) < carril.width() as f64 {
            let x = carril.left() + ((k * paso - x0) * zoom) as f32;
            pr.vline(x, regla.bottom() - 6.0..=todo.bottom(), Stroke::new(1.0, c(0x222222)));
            let s = k * paso;
            pr.text(pos2(x + 3.0, regla.center().y - 2.0), Align2::LEFT_CENTER, if paso < 1.0 { format!("{s:.2}") } else { tiempo(s) }, fuente(12.0), RULER_TEXT);
            k += 1.0;
        }
        carril
    }

    fn cursor_animacion(&self, ui: &Ui, carril: Rect) {
        let x = carril.left() + ((self.ahora() - self.anim.x0) * self.anim.zoom) as f32;
        if x >= carril.left() - 1.0 {
            let p = ui.painter_at(carril);
            p.vline(x, carril.y_range(), Stroke::new(1.5, MAIN));
            p.add(Shape::convex_polygon(vec![pos2(x - 6.0, carril.top() + 3.0), pos2(x + 6.0, carril.top() + 3.0), pos2(x, carril.top() + 13.0)], MAIN, Stroke::NONE));
        }
    }

    /// Pega un segundo (del proyecto) al cursor, a las orillas del clip o a otro cuadro clave cercano.
    fn iman_animacion(&self, s: f64, c: &ClipVideo, propias: &[f64]) -> f64 {
        if !self.linea.iman {
            return s;
        }
        let tol = 7.0 / self.anim.zoom;
        let mut v = vec![self.ahora(), c.inicio, c.fin()];
        for p in Pista::todas(c) {
            if let Some(q) = p.de(c) {
                v.extend(q.claves.iter().map(|k| k.0 + c.inicio).filter(|x| !propias.iter().any(|y| (y - x).abs() < 0.02)));
            }
        }
        v.into_iter().filter(|x| (x - s).abs() < tol).min_by(|a, b| (a - s).abs().total_cmp(&(b - s).abs())).unwrap_or(s)
    }

    fn pistas_animacion(&mut self, ui: &mut Ui) {
        let todo = ui.available_rect_before_wrap();
        let resp = ui.allocate_rect(todo, Sense::click_and_drag());
        let carril = self.regla_animacion(ui, todo, &resp);
        let (zoom, x0) = (self.anim.zoom, self.anim.x0);
        let a_x = move |s: f64| carril.left() + ((s - x0) * zoom) as f32;
        let a_t = move |x: f32| x0 + (x - carril.left()) as f64 / zoom;
        let cuerpo = Rect::from_min_max(pos2(todo.left(), todo.top() + REGLA), todo.max);
        let zona = Rect::from_min_max(pos2(carril.left(), cuerpo.top()), todo.max);
        let p = ui.painter_at(cuerpo);
        let pz = ui.painter_at(zona);
        let elegido = self.anim.clip;
        let mut clips: Vec<ClipVideo> = self.p.video.iter().filter(|c| c.animado || Some(c.id) == elegido).cloned().collect();
        // El elegido arriba, abierto; los demás después, por tiempo.
        clips.sort_by(|a, b| (Some(b.id) == elegido).cmp(&(Some(a.id) == elegido)).then(a.inicio.total_cmp(&b.inicio)));
        if clips.is_empty() {
            p.text(cuerpo.center(), Align2::CENTER_CENTER, t("Agrega un texto, imagen, figura o clip vacío en «Elementos»"), fuente(16.0), TEXT_FAINT);
        }
        let raton = ui.input(|i| i.pointer.hover_pos()).filter(|m| cuerpo.contains(*m));
        let mut hacer = vec![];
        let mut rombos: Vec<(Rect, Clave)> = vec![];
        let mut carriles: Vec<(Rect, u64, Pista)> = vec![];
        let mut barras: Vec<(Rect, u64)> = vec![];
        let mut momentos: Vec<(Rect, u64, f64)> = vec![];
        let mut y = cuerpo.top() - self.anim.y0;
        for c in &clips {
            let on = elegido == Some(c.id);
            let cab = Rect::from_min_size(pos2(todo.left(), y), vec2(todo.width(), FILA_CLIP));
            degradado(&p, cab, 0, if on { c_fila(0x4a4a4a) } else { c_fila(0x383838) }, if on { c_fila(0x333333) } else { c_fila(0x2a2a2a) });
            p.hline(cab.x_range(), cab.top() + 0.5, Stroke::new(1.0, Color32::from_white_alpha(20)));
            let abierto = on != self.anim.cerrados.contains(&c.id);
            let fl = Rect::from_center_size(pos2(cab.left() + 14.0, cab.center().y), vec2(16.0, 16.0));
            let o = fl.center();
            p.add(Shape::convex_polygon(if abierto { vec![o + vec2(-4.0, -2.5), o + vec2(4.0, -2.5), o + vec2(0.0, 3.5)] } else { vec![o + vec2(-2.5, -4.0), o + vec2(3.5, 0.0), o + vec2(-2.5, 4.0)] }, TEXT_DIM, Stroke::NONE));
            ui.painter_at(Rect::from_min_max(cab.min, pos2(carril.left() - 26.0, cab.bottom())).intersect(cuerpo)).text(pos2(cab.left() + 28.0, cab.center().y), Align2::LEFT_CENTER, nombre_clip(c), fuente(14.5), if on { TEXT } else { TEXT_DIM });
            let x = Rect::from_center_size(pos2(carril.left() - 14.0, cab.center().y), vec2(18.0, 18.0));
            let sobre_x = raton.is_some_and(|m| x.contains(m));
            p.text(x.center(), Align2::CENTER_CENTER, "✕", fuente(13.0), if sobre_x { TEXT } else { TEXT_FAINT });
            if resp.clicked() && raton.is_some_and(|m| cab.contains(m) && m.x < carril.left()) {
                if sobre_x {
                    hacer.push(Hacer::Quitar(c.id));
                } else if on || raton.is_some_and(|m| fl.contains(m)) {
                    if !self.anim.cerrados.remove(&c.id) {
                        self.anim.cerrados.insert(c.id);
                    }
                } else {
                    self.anim.clip = Some(c.id);
                    self.clip = Some(c.id);
                }
            }
            // El tramo del clip en la línea de tiempo.
            let b = Rect::from_min_max(pos2(a_x(c.inicio), cab.top() + 4.0), pos2(a_x(c.fin()).max(a_x(c.inicio) + 3.0), cab.bottom() - 4.0));
            pz.rect_filled(b, 5, if on { mezcla(c_fila(0x3d3d3d), MAIN, 0.4) } else { c_fila(0x4a4636) });
            if on {
                for e in [b.left() + 2.5, b.right() - 2.5] {
                    pz.vline(e, b.top() + 4.0..=b.bottom() - 4.0, Stroke::new(2.0, MAIN));
                }
            }
            if b.intersects(zona) {
                vidrio(&pz, b, 5, if on { MAIN } else { c_fila(0x8a8a8a) }, false);
            }
            // Sus llaves, por momento: se arrastran juntas.
            let mo: Vec<f64> = animaciones::momentos(c).into_iter().map(|m| m.0).filter(|k| *k >= -0.01 && *k <= c.dur + 0.01).collect();
            for k in mo {
                let o = pos2(a_x(c.inicio + k), cab.center().y);
                if zona.expand(8.0).contains(o) {
                    let r = Rect::from_center_size(o, vec2(15.0, 20.0));
                    let sobre = raton.is_some_and(|m| r.contains(m));
                    rombo(&pz, o + vec2(0.0, 1.0), 6.5, Color32::from_black_alpha(120));
                    rombo(&pz, o, 6.5, if sobre { Color32::WHITE } else if on { HIGHLIGHT } else { Color32::from_rgb(0xc9, 0xb2, 0x5a) });
                    momentos.push((r, c.id, k));
                }
            }
            barras.push((b, c.id));
            y += FILA_CLIP;
            let pistas = Pista::todas(c);
            if !abierto {
                continue;
            }
            for pista in pistas {
                let Some(pr) = pista.de(c) else { continue };
                let fila = Rect::from_min_size(pos2(todo.left(), y), vec2(todo.width(), FILA));
                y += FILA;
                if !fila.intersects(cuerpo) {
                    continue;
                }
                let activa = self.anim.pista.as_ref().is_some_and(|x| x.0 == c.id && x.1 == pista);
                if activa {
                    p.rect_filled(fila, 0, Color32::from_white_alpha(10));
                }
                let tl = (self.ahora() - c.inicio).clamp(0.0, c.dur);
                // Rombo de la cabecera: pone o quita en el cursor.
                let rk = Rect::from_center_size(pos2(fila.left() + 20.0, fila.center().y), vec2(18.0, 18.0));
                let (hay, aqui) = (!pr.claves.is_empty(), pr.clave_en(tl).is_some());
                let col = if raton.is_some_and(|m| rk.contains(m)) { HIGHLIGHT } else if hay { MAIN } else { TEXT_FAINT };
                if aqui {
                    rombo(&p, rk.center(), 5.0, col);
                } else {
                    let o = rk.center();
                    p.add(Shape::closed_line(vec![o - vec2(0.0, 5.0), o + vec2(5.0, 0.0), o + vec2(0.0, 5.0), o - vec2(5.0, 0.0)], Stroke::new(1.2, col)));
                }
                let valor = match pr.en(tl) {
                    Valor::Num(v) => format!("{v:.1}"),
                    Valor::Punto(v) => format!("{:.2}, {:.2}", v[0], v[1]),
                    Valor::Si(b) => t(if b { "Sí" } else { "No" }).into(),
                    Valor::Op(i) => i.to_string(),
                    _ => String::new(),
                };
                let rv = p.text(pos2(carril.left() - 10.0, fila.center().y), Align2::RIGHT_CENTER, valor, fuente(12.5), if hay { MAIN } else { TEXT_FAINT });
                ui.painter_at(Rect::from_min_max(fila.min, pos2(rv.left() - 6.0, fila.bottom())).intersect(cuerpo)).text(pos2(fila.left() + 36.0, fila.center().y), Align2::LEFT_CENTER, nombre_pista(c, &pista), fuente(13.0), if hay { TEXT } else { TEXT_DIM });
                if resp.clicked() && raton.is_some_and(|m| fila.contains(m) && m.x < carril.left()) {
                    self.anim.pista = Some((c.id, pista.clone()));
                    if raton.is_some_and(|m| rk.contains(m)) {
                        hacer.push(Hacer::Agregar(c.id, pista.clone(), tl));
                    } else {
                        // Clic en el nombre: elige todos sus cuadros clave.
                        self.anim.claves = pr.claves.iter().map(|k| (c.id, pista.clone(), k.0)).collect();
                    }
                }
                if resp.double_clicked() && raton.is_some_and(|m| fila.contains(m) && m.x < carril.left() && !rk.contains(m)) {
                    self.anim.grafica = true;
                }
                let franja = Rect::from_min_max(pos2(a_x(c.inicio), fila.top() + 2.0), pos2(a_x(c.fin()), fila.bottom() - 2.0));
                pz.rect_filled(franja, 2, STRIP_DARK);
                pz.hline(fila.x_range(), fila.bottom() - 0.5, Stroke::new(1.0, GRID_GAP));
                for w in pr.claves.windows(2) {
                    let r = Rect::from_min_max(pos2(a_x(c.inicio + w[0].0), fila.top() + 5.0), pos2(a_x(c.inicio + w[1].0), fila.bottom() - 5.0));
                    if r.right() > zona.left() && r.left() < zona.right() {
                        pinta_curva(&pz, r, pr.curva_en(w[0].0), w[1].1.num() < w[0].1.num(), MAIN.gamma_multiply(0.6));
                    }
                }
                for k in &pr.claves {
                    let o = pos2(a_x(c.inicio + k.0), fila.center().y);
                    if !zona.expand(8.0).contains(o) {
                        continue;
                    }
                    let r = Rect::from_center_size(o, vec2(15.0, 17.0));
                    let elegida = self.anim.claves.iter().any(|e| e.0 == c.id && e.1 == pista && (e.2 - k.0).abs() < 0.02);
                    let sobre = raton.is_some_and(|m| r.contains(m));
                    rombo(&pz, o, 6.0, if elegida { MAIN } else if sobre { HIGHLIGHT } else { TEXT_DIM });
                    rombos.push((r, (c.id, pista.clone(), k.0)));
                }
                carriles.push((Rect::from_min_max(pos2(carril.left(), fila.top()), fila.max), c.id, pista));
            }
        }
        let alto = y + self.anim.y0 - cuerpo.top();
        self.anim.y0 = self.anim.y0.clamp(0.0, (alto - cuerpo.height() + 10.0).max(0.0));
        self.cursor_animacion(ui, carril);
        let todo_raton = ui.input(|i| i.pointer.hover_pos());
        let en_rombo = |m: Pos2| rombos.iter().find(|r| r.0.contains(m)).map(|r| r.1.clone());
        let mods = ui.input(|i| i.modifiers);
        if resp.drag_started() {
            if let Some(m) = ui.input(|i| i.pointer.press_origin()) {
                self.anim.arrastre = if m.y < cuerpo.top() && m.x >= carril.left() {
                    Some(Arrastre::Cursor)
                } else if let Some(k) = en_rombo(m) {
                    if !self.anim.claves.contains(&k) {
                        self.anim.claves = vec![k];
                    }
                    Some(Arrastre::Claves(self.anim.claves.iter().map(|k| (k.clone(), k.2)).collect(), m))
                } else if let Some((_, id, k)) = momentos.iter().find(|x| x.0.contains(m)) {
                    self.clip = Some(*id);
                    self.anim.clip = Some(*id);
                    Some(Arrastre::Momento(*id, *k))
                } else if let Some((b, id)) = barras.iter().find(|b| b.0.expand(3.0).contains(m)) {
                    let que = if (m.x - b.left()).abs() < 7.0 { 1 } else if (m.x - b.right()).abs() < 7.0 { 2 } else { 0 };
                    let ini = self.p.video.iter().find(|c| c.id == *id).map_or(0.0, |c| c.inicio);
                    self.clip = Some(*id);
                    Some(Arrastre::Clip(*id, que, a_t(m.x) - ini))
                } else if m.x >= carril.left() {
                    Some(Arrastre::Caja(m))
                } else {
                    None
                };
            }
        }
        if let (true, Some(m)) = (resp.dragged(), todo_raton) {
            match self.anim.arrastre.take() {
                Some(Arrastre::Cursor) => {
                    self.ir(a_t(m.x).max(0.0));
                    self.anim.arrastre = Some(Arrastre::Cursor);
                }
                Some(Arrastre::Caja(o)) => {
                    let r = Rect::from_two_pos(o, m);
                    pz.rect_filled(r, 2, MAIN.gamma_multiply(0.12));
                    pz.rect_stroke(r, 2, Stroke::new(1.0, MAIN), StrokeKind::Inside);
                    self.anim.claves = rombos.iter().filter(|x| r.contains(x.0.center())).map(|x| x.1.clone()).collect();
                    self.anim.arrastre = Some(Arrastre::Caja(o));
                }
                Some(Arrastre::Claves(v, o)) => {
                    let mut dt = (m.x - o.x) as f64 / zoom;
                    // La primera manda: se pega a lo cercano.
                    if let Some(((id, _, _), t0)) = v.first() {
                        if let Some(c) = clips.iter().find(|c| c.id == *id) {
                            let propias: Vec<f64> = v.iter().map(|x| x.0.2 + c.inicio).collect();
                            dt = self.iman_animacion(c.inicio + t0 + dt, c, &propias) - c.inicio - t0;
                        }
                    }
                    self.anim.arrastre = Some(Arrastre::Claves(self.mover_claves(v, dt), o));
                }
                Some(Arrastre::Momento(id, de)) => {
                    let ini = clips.iter().find(|c| c.id == id).map_or(0.0, |c| c.inicio);
                    let a = clips.iter().find(|c| c.id == id).map_or(0.0, |c| self.iman_animacion(a_t(m.x), c, &[ini + de])) - ini;
                    let a = self.a_cuadro(a);
                    let quedo = self.p.clip_video(id).is_some_and(|c| {
                        let a = a.clamp(0.0, c.dur);
                        (a - de).abs() > 1e-6 && animaciones::mover_momento(c, de, a)
                    });
                    self.anim.arrastre = Some(Arrastre::Momento(id, if quedo { a.max(0.0) } else { de }));
                }
                Some(Arrastre::Clip(id, que, des)) => {
                    let s = a_t(m.x);
                    if let Some(c) = self.p.clip_video(id) {
                        // Los que no salen de un archivo se alargan libremente.
                        let libre = c.contenido != Contenido::Archivo;
                        match que {
                            0 => c.inicio = (s - des).max(0.0),
                            1 if libre => {
                                let fin = c.fin();
                                let ini = s.clamp(0.0, fin - 0.1);
                                let d = c.inicio - ini;
                                c.animables().for_each(|p| p.correr(d));
                                (c.inicio, c.dur) = (ini, fin - ini);
                                c.total = c.total.max(c.dur);
                            }
                            2 if libre => {
                                c.dur = (s - c.inicio).max(0.1);
                                c.total = c.total.max(c.dur);
                            }
                            _ => {}
                        }
                    }
                    self.anim.arrastre = Some(Arrastre::Clip(id, que, des));
                }
                x => self.anim.arrastre = x,
            }
        }
        if resp.drag_stopped() {
            self.anim.arrastre = None;
        }
        if let Some(m) = todo_raton.filter(|m| m.x >= carril.left()) {
            let en_momento = momentos.iter().find(|x| x.0.contains(m)).map(|x| (x.1, x.2));
            let en_barra = barras.iter().find(|b| b.0.contains(m)).map(|b| b.1);
            if let (true, Some((id, k))) = (resp.clicked() && !resp.double_clicked(), en_momento) {
                // Clic en una llave: se elige entera y el cursor va a ella.
                self.anim.clip = Some(id);
                self.clip = Some(id);
                if let Some(c) = clips.iter().find(|c| c.id == id) {
                    self.anim.claves = Pista::todas(c).into_iter().filter(|p| p.de(c).is_some_and(|q| q.clave_en(k).is_some())).map(|p| (id, p, k)).collect();
                    hacer.push(Hacer::Ir(c.inicio + k));
                }
            } else if let (true, Some(id), None) = (resp.double_clicked() || (resp.clicked() && self.anim.clic_pone), en_barra, en_momento) {
                // Llave con todo lo que tiene ahí.
                self.anim.clip = Some(id);
                self.clip = Some(id);
                if let Some(c) = clips.iter().find(|c| c.id == id) {
                    let s = a_t(m.x).clamp(c.inicio, c.fin());
                    hacer.push(Hacer::Ir(s));
                    hacer.push(Hacer::Guardar(id, s - c.inicio));
                }
            } else if resp.double_clicked() && en_rombo(m).is_none() {
                if let Some((_, id, pista)) = carriles.iter().find(|c| c.0.contains(m)) {
                    if let Some(c) = clips.iter().find(|c| c.id == *id) {
                        let tk = a_t(m.x) - c.inicio;
                        if (0.0..=c.dur).contains(&tk) {
                            hacer.push(Hacer::Agregar(*id, pista.clone(), tk));
                        }
                    }
                }
            } else if resp.clicked() {
                match en_rombo(m) {
                    Some(k) => {
                        if mods.shift || mods.command {
                            match self.anim.claves.iter().position(|x| *x == k) {
                                Some(i) => drop(self.anim.claves.remove(i)),
                                None => self.anim.claves.push(k.clone()),
                            }
                        } else {
                            self.anim.claves = vec![k.clone()];
                        }
                        self.anim.pista = Some((k.0, k.1.clone()));
                        if let Some(c) = clips.iter().find(|c| c.id == k.0) {
                            hacer.push(Hacer::Ir(c.inicio + k.2));
                        }
                    }
                    None => {
                        if let Some((_, id)) = barras.iter().find(|b| b.0.contains(m)) {
                            self.anim.clip = Some(*id);
                            self.clip = Some(*id);
                        }
                        if !mods.shift {
                            self.anim.claves.clear();
                        }
                        hacer.push(Hacer::Ir(a_t(m.x).max(0.0)));
                    }
                }
            }
            if resp.secondary_clicked() {
                self.anim.menu_momento = en_momento.or(en_barra.map(|id| (id, a_t(m.x) - clips.iter().find(|c| c.id == id).map_or(0.0, |c| c.inicio))));
                self.anim.menu = en_rombo(m);
                if let Some(k) = &self.anim.menu {
                    if !self.anim.claves.contains(k) {
                        self.anim.claves = vec![k.clone()];
                    }
                }
            }
        }
        self.menu_clave(&resp, &mut hacer);
        for h in hacer {
            match h {
                Hacer::Ir(s) => self.ir(s),
                Hacer::Guardar(id, tl) => self.guardar_llave_en(id, tl),
                Hacer::Agregar(id, pista, tk) => {
                    let mut queda = false;
                    if let Some(c) = self.p.clip_video(id) {
                        c.animado = true;
                        if let Some(q) = pista.de_mut(c) {
                            q.alternar_clave(tk);
                            queda = q.clave_en(tk).is_some();
                        }
                    }
                    self.anim.claves = if queda { vec![(id, pista.clone(), tk)] } else { vec![] };
                    self.anim.pista = Some((id, pista));
                }
                Hacer::Curva(cv) => self.curva_a_elegidas(cv),
                Hacer::Borrar => self.borrar_claves(),
                Hacer::Quitar(id) => {
                    if let Some(c) = self.p.clip_video(id) {
                        c.animado = false;
                    }
                    if self.anim.clip == Some(id) {
                        self.anim.clip = None;
                        self.clip = None;
                    }
                }
            }
        }
    }

    fn menu_clave(&mut self, resp: &Response, hacer: &mut Vec<Hacer>) {
        if let (Some((id, tl)), None) = (self.anim.menu_momento, &self.anim.menu) {
            let ini = self.p.video.iter().find(|c| c.id == id).map_or(0.0, |c| c.inicio);
            let abierto = resp.context_menu(|ui| {
                ui.set_min_width(210.0);
                if ui.button(RichText::new(t("◆ Guardar llave con todo")).color(MAIN)).clicked() {
                    hacer.push(Hacer::Ir(ini + tl));
                    hacer.push(Hacer::Guardar(id, tl));
                    ui.close();
                }
                self.menu_llaves(ui, id, ini + tl);
            });
            if abierto.is_none() {
                self.anim.menu_momento = None;
            }
            return;
        }
        let Some((id, pista, tk)) = self.anim.menu.clone() else { return };
        let actual = self.p.video.iter().find(|c| c.id == id).and_then(|c| pista.de(c)).map(|p| p.curva_en(tk));
        let abierto = resp.context_menu(|ui| {
            ui.label(RichText::new(t("Curva hasta el siguiente cuadro clave")).color(TEXT_DIM).size(12.5));
            ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for cv in Curva::menu() {
                    if ui.selectable_label(actual == Some(cv), cv.nombre()).clicked() {
                        hacer.push(Hacer::Curva(cv));
                        ui.close();
                    }
                }
            });
            ui.separator();
            if ui.button(t("Copiar")).clicked() {
                self.copiar_claves();
                ui.close();
            }
            if ui.button(t("Borrar cuadro clave")).clicked() {
                hacer.push(Hacer::Borrar);
                ui.close();
            }
        });
        if abierto.is_none() {
            self.anim.menu = None;
        }
    }

    /// Mueve las claves arrastradas `dt` segundos desde donde estaban; devuelve dónde quedaron.
    fn mover_claves(&mut self, v: Vec<(Clave, f64)>, dt: f64) -> Vec<(Clave, f64)> {
        let mut fuera = vec![];
        // Primero se sacan todas, para que no choquen entre sí.
        for ((id, p, tk), t0) in &v {
            if let Some(c) = self.p.clip_video(*id) {
                let dur = c.dur;
                if let Some(q) = p.de_mut(c) {
                    if let Some(i) = q.clave_en(*tk) {
                        let cv = q.curva_en(*tk);
                        let (_, val) = q.claves.remove(i);
                        q.curvas.retain(|x| (x.0 - tk).abs() >= 0.02);
                        fuera.push((*id, p.clone(), (t0 + dt).clamp(0.0, dur), val, cv, *t0));
                    }
                }
            }
        }
        let mut r = vec![];
        for (id, p, tn, val, cv, t0) in fuera {
            if let Some(q) = self.p.clip_video(id).and_then(|c| p.de_mut(c)) {
                q.claves.retain(|k| (k.0 - tn).abs() >= 0.02);
                q.claves.push((tn, val));
                q.claves.sort_by(|a, b| a.0.total_cmp(&b.0));
                q.poner_curva(tn, cv);
                r.push(((id, p, tn), t0));
            }
        }
        self.anim.claves = r.iter().map(|x| x.0.clone()).collect();
        r
    }

    /// Valor contra tiempo de una pista, con los puntos de control de sus curvas.
    fn grafica_animacion(&mut self, ui: &mut Ui) {
        let todo = ui.available_rect_before_wrap();
        let resp = ui.allocate_rect(todo, Sense::click_and_drag());
        let carril = self.regla_animacion(ui, todo, &resp);
        let cuerpo = Rect::from_min_max(pos2(carril.left(), todo.top() + REGLA), todo.max).shrink2(vec2(0.0, 12.0));
        let p = ui.painter_at(Rect::from_min_max(pos2(carril.left(), todo.top() + REGLA), todo.max));
        let Some(clip) = self.anim.clip.and_then(|id| self.p.video.iter().find(|c| c.id == id)).cloned() else {
            p.text(cuerpo.center(), Align2::CENTER_CENTER, t("Elige un elemento"), fuente(15.0), TEXT_FAINT);
            return;
        };
        // Lista de pistas a la izquierda.
        let pistas = Pista::todas(&clip);
        let raton = ui.input(|i| i.pointer.hover_pos());
        let pc = ui.painter_at(Rect::from_min_max(todo.min, pos2(carril.left(), todo.bottom())));
        if self.anim.pista.as_ref().is_none_or(|x| x.0 != clip.id || x.1.de(&clip).is_none()) {
            let p = pistas.iter().find(|p| p.de(&clip).is_some_and(|q| !q.claves.is_empty())).or(pistas.first()).cloned();
            self.anim.pista = p.map(|p| (clip.id, p));
        }
        for (i, q) in pistas.iter().enumerate() {
            let r = Rect::from_min_size(pos2(todo.left(), todo.top() + REGLA + i as f32 * FILA), vec2(CAB, FILA));
            let on = self.anim.pista.as_ref().is_some_and(|x| x.1 == *q);
            let hay = q.de(&clip).is_some_and(|q| !q.claves.is_empty());
            pc.rect_filled(r, 0, if on { BUTTON } else { WORKSPACE });
            pc.text(pos2(r.left() + 14.0, r.center().y), Align2::LEFT_CENTER, nombre_pista(&clip, q), fuente(13.0), if on { TEXT } else if hay { MAIN } else { TEXT_FAINT });
            if resp.clicked() && raton.is_some_and(|m| r.contains(m)) {
                self.anim.pista = Some((clip.id, q.clone()));
                self.anim.claves.clear();
            }
        }
        let Some((_, pista)) = self.anim.pista.clone() else { return };
        let Some(pr) = pista.de(&clip).cloned() else { return };
        if !matches!(pr.v, Valor::Num(_)) {
            p.text(cuerpo.center(), Align2::CENTER_CENTER, t("La gráfica es para valores numéricos"), fuente(15.0), TEXT_FAINT);
            return;
        }
        let (zoom, x0) = (self.anim.zoom, self.anim.x0);
        let a_x = move |s: f64| carril.left() + ((s - x0) * zoom) as f32;
        let a_t = move |x: f32| x0 + (x - carril.left()) as f64 / zoom;
        // Escala vertical: lo que hay, con aire; fija mientras se arrastra.
        let k = Id::new(("graf", clip.id));
        let (mut lo, mut hi) = ambito(&pista);
        if pr.claves.len() > 1 {
            let n = 80;
            let v: Vec<f64> = (0..=n).map(|i| pr.en(clip.dur * i as f64 / n as f64).num()).chain(pr.claves.iter().map(|k| k.1.num())).collect();
            (lo, hi) = (v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max));
            let aire = ((hi - lo) * 0.18).max(1.0);
            (lo, hi) = (lo - aire, hi + aire);
        }
        let guardada: Option<(f64, f64)> = ui.data(|d| d.get_temp(k));
        let (lo, hi) = match (&self.anim.arrastre, guardada) {
            (Some(Arrastre::Punto(_) | Arrastre::Control(..)), Some(g)) => g,
            _ => (lo, hi),
        };
        ui.data_mut(|d| d.insert_temp(k, (lo, hi)));
        let a_y = move |v: f64| cuerpo.bottom() - ((v - lo) / (hi - lo)) as f32 * cuerpo.height();
        let a_v = move |y: f32| lo + ((cuerpo.bottom() - y) / cuerpo.height()) as f64 * (hi - lo);
        let paso = [0.1, 0.5, 1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0].into_iter().find(|s| s / (hi - lo) * cuerpo.height() as f64 > 26.0).unwrap_or(500.0);
        let mut v = (lo / paso).ceil() * paso;
        while v <= hi {
            p.hline(carril.x_range(), a_y(v), Stroke::new(1.0, if v.abs() < 1e-9 { c(0x3a3a3a) } else { c(0x262626) }));
            p.text(pos2(carril.left() + 5.0, a_y(v)), Align2::LEFT_BOTTOM, format!("{v:.0}"), fuente(11.0), TEXT_FAINT);
            v += paso;
        }
        for x in [a_x(clip.inicio), a_x(clip.fin())] {
            p.vline(x, cuerpo.y_range(), Stroke::new(1.0, MAIN.gamma_multiply(0.35)));
        }
        let (xa, xb) = (a_x(clip.inicio).max(carril.left()), a_x(clip.fin()).min(carril.right()));
        if xb > xa {
            let n = ((xb - xa) / 2.0).clamp(2.0, 900.0) as usize;
            let pts = (0..=n).map(|i| {
                let x = xa + (xb - xa) * i as f32 / n as f32;
                pos2(x, a_y(pr.en(a_t(x) - clip.inicio).num()))
            });
            p.add(Shape::line(pts.collect(), Stroke::new(2.0, MAIN)));
        }
        // Puntos y controles
        let mut puntos: Vec<(Pos2, Clave)> = vec![];
        let mut controles: Vec<(Pos2, Clave, usize)> = vec![];
        for (i, q) in pr.claves.iter().enumerate() {
            let o = pos2(a_x(clip.inicio + q.0), a_y(q.1.num()));
            let clave = (clip.id, pista.clone(), q.0);
            let elegida = self.anim.claves.contains(&clave);
            if let (true, Some(s)) = (elegida, pr.claves.get(i + 1)) {
                let b = match pr.curva_en(q.0) {
                    Curva::Bezier(a, b, c, d) => [a, b, c, d],
                    _ => [0.33, 0.0, 0.67, 1.0],
                };
                let fin = pos2(a_x(clip.inicio + s.0), a_y(s.1.num()));
                for (j, (u, v)) in [(b[0], b[1]), (b[2], b[3])].into_iter().enumerate() {
                    let h = pos2(o.x + (fin.x - o.x) * u, o.y + (fin.y - o.y) * v);
                    p.line_segment([if j == 0 { o } else { fin }, h], Stroke::new(1.0, TEXT_DIM));
                    p.circle(h, 5.0, BUTTON_HI, Stroke::new(1.5, HIGHLIGHT));
                    controles.push((h, clave.clone(), j));
                }
            }
            rombo(&p, o, 7.0, if elegida { HIGHLIGHT } else { MAIN });
            puntos.push((o, clave));
        }
        if pr.claves.is_empty() {
            p.text(cuerpo.center(), Align2::CENTER_CENTER, t("Doble clic para poner un cuadro clave"), fuente(15.0), TEXT_FAINT);
        }
        self.cursor_animacion(ui, carril);
        let cerca = |m: Pos2| puntos.iter().find(|x| x.0.distance(m) < 10.0).map(|x| x.1.clone());
        if resp.drag_started() {
            if let Some(m) = ui.input(|i| i.pointer.press_origin()).filter(|m| m.x >= carril.left()) {
                self.anim.arrastre = if m.y < todo.top() + REGLA {
                    Some(Arrastre::Cursor)
                } else if let Some((_, k, j)) = controles.iter().find(|x| x.0.distance(m) < 9.0) {
                    Some(Arrastre::Control(k.clone(), *j))
                } else if let Some(k) = cerca(m) {
                    self.anim.claves = vec![k.clone()];
                    Some(Arrastre::Punto(k))
                } else {
                    Some(Arrastre::Cursor)
                };
            }
        }
        if let (true, Some(m)) = (resp.dragged(), raton) {
            match self.anim.arrastre.take() {
                Some(Arrastre::Cursor) => {
                    self.ir(a_t(m.x).max(0.0));
                    self.anim.arrastre = Some(Arrastre::Cursor);
                }
                Some(Arrastre::Punto((id, q, tk))) => {
                    let s = self.iman_animacion(a_t(m.x), &clip, &[clip.inicio + tk]);
                    let tn = (s - clip.inicio).clamp(0.0, clip.dur);
                    let v = self.mover_claves(vec![((id, q.clone(), tk), tn)], 0.0);
                    let tn = v.first().map_or(tk, |x| x.0.2);
                    if let Some(x) = self.p.clip_video(id).and_then(|c| q.de_mut(c)) {
                        if let Some(i) = x.clave_en(tn) {
                            x.claves[i].1 = Valor::Num((a_v(m.y) * 100.0).round() / 100.0);
                        }
                    }
                    self.anim.arrastre = Some(Arrastre::Punto((id, q, tn)));
                }
                Some(Arrastre::Control((id, q, tk), j)) => {
                    if let Some(i) = pr.clave_en(tk) {
                        if let Some(s) = pr.claves.get(i + 1) {
                            let (o, fin) = (pos2(a_x(clip.inicio + tk), a_y(pr.claves[i].1.num())), pos2(a_x(clip.inicio + s.0), a_y(s.1.num())));
                            let mut b = match pr.curva_en(tk) {
                                Curva::Bezier(a, b, c, d) => [a, b, c, d],
                                _ => [0.33, 0.0, 0.67, 1.0],
                            };
                            b[j * 2] = ((m.x - o.x) / (fin.x - o.x).max(1.0)).clamp(0.0, 1.0);
                            if (fin.y - o.y).abs() > 2.0 {
                                b[j * 2 + 1] = ((m.y - o.y) / (fin.y - o.y)).clamp(-1.5, 2.5);
                            }
                            if let Some(x) = self.p.clip_video(id).and_then(|c| q.de_mut(c)) {
                                x.poner_curva(tk, Curva::Bezier(b[0], b[1], b[2], b[3]));
                            }
                        }
                    }
                    self.anim.arrastre = Some(Arrastre::Control((id, q, tk), j));
                }
                x => self.anim.arrastre = x,
            }
        }
        if resp.drag_stopped() {
            self.anim.arrastre = None;
        }
        let mut hacer = vec![];
        if let Some(m) = raton.filter(|m| m.x >= carril.left()) {
            if resp.double_clicked() && cerca(m).is_none() {
                let tk = a_t(m.x) - clip.inicio;
                if (0.0..=clip.dur).contains(&tk) {
                    if let Some(q) = self.p.clip_video(clip.id).and_then(|c| {
                        c.animado = true;
                        pista.de_mut(c)
                    }) {
                        q.alternar_clave(tk);
                        if let Some(i) = q.clave_en(tk) {
                            q.claves[i].1 = Valor::Num((a_v(m.y) * 100.0).round() / 100.0);
                        }
                    }
                    self.anim.claves = vec![(clip.id, pista.clone(), tk)];
                }
            } else if resp.clicked() {
                match cerca(m) {
                    Some(k) => {
                        self.ir(clip.inicio + k.2);
                        self.anim.claves = vec![k];
                    }
                    None => self.ir(a_t(m.x).max(0.0)),
                }
            }
            if resp.secondary_clicked() {
                self.anim.menu = cerca(m);
                if let Some(k) = &self.anim.menu {
                    self.anim.claves = vec![k.clone()];
                }
            }
        }
        self.menu_clave(&resp, &mut hacer);
        for h in hacer {
            match h {
                Hacer::Curva(cv) => self.curva_a_elegidas(cv),
                Hacer::Borrar => self.borrar_claves(),
                _ => {}
            }
        }
    }
}

fn c_fila(x: u32) -> Color32 {
    c(x)
}

/// Tarjeta de animación lista; con el ratón encima se ve moverse.
fn tarjeta_anim(ui: &mut Ui, r: Rect, a: &Animacion) -> bool {
    let n = t(a.nombre);
    let resp = ui.interact(r, Id::new(("anim", a.nombre)), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, n));
    let hov = resp.hovered();
    let p = ui.painter_at(r);
    p.rect_filled(r, 6, if hov { BUTTON_HI } else { c(0x3a3a3a) });
    let vista = Rect::from_min_max(r.min + vec2(4.0, 4.0), pos2(r.right() - 4.0, r.bottom() - 22.0));
    p.rect_filled(vista, 4, c(0x232323));
    // Se calcula con los mismos cuadros clave que pondrá.
    let mut proy = Proyecto::nuevo();
    let dur = match a.momento {
        Momento::Bucle => a.dur.max(1.2),
        _ => a.dur + 0.6,
    };
    let mut cl = proy.nuevo_video("m".into(), Contenido::Color([1.0; 4]), 0.0, dur);
    animaciones::poner_en(&mut cl, a, 0.2, || 1);
    let tl = if hov {
        ui.ctx().request_repaint();
        ui.input(|i| i.time) % (dur + 0.35)
    } else {
        match a.momento {
            Momento::Entrada => a.dur * 0.35,
            Momento::Salida => dur - a.dur * 0.4,
            _ => a.dur * 0.3,
        }
    }
    .min(dur);
    let g = |x: Propiedad| x.de(&cl).en(tl).num() as f32;
    let o = vista.center() + vec2(g(Propiedad::X) / 100.0 * vista.width() * 0.75, g(Propiedad::Y) / 100.0 * vista.height());
    let e = g(Propiedad::Escala) / 100.0;
    let lado = vista.height() * 0.26;
    let m = vec2(lado * 1.35 * e * g(Propiedad::Ancho) / 100.0, lado * e * g(Propiedad::Alto) / 100.0);
    let giro = emath::Rot2::from_angle(g(Propiedad::Giro).to_radians());
    let alfa = (g(Propiedad::Opacidad) / 100.0).clamp(0.0, 1.0);
    let esq = [vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(1.0, 1.0), vec2(-1.0, 1.0)].map(|v| o + giro * (v * m));
    let pv = ui.painter_at(vista);
    if !a.fx.is_empty() {
        // Con efecto: un halo que se abre según qué tan fuera está.
        let f = cl.fx.first().and_then(|f| f.p.get(a.fx[0].1)).map_or(0.0, |p| ((p.en(tl).num() - a.fx[0].3) / (a.fx[0].2 - a.fx[0].3)).clamp(0.0, 1.0)) as f32;
        for (dx, col) in [(-6.0 * f, c(0xff5a6e)), (6.0 * f, c(0x4fc3ff))] {
            pv.add(Shape::convex_polygon(esq.iter().map(|q| *q + vec2(dx, 0.0)).collect(), col.gamma_multiply(0.5 * alfa.max(f)), Stroke::NONE));
        }
    }
    pv.add(Shape::convex_polygon(esq.to_vec(), MAIN.gamma_multiply(alfa), Stroke::NONE));
    let g = {
        let mut job = eframe::egui::epaint::text::LayoutJob::simple_singleline(n.into(), fuente(12.5), TEXT);
        job.wrap = eframe::egui::epaint::text::TextWrapping { max_width: r.width() - 8.0, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
        p.layout_job(job)
    };
    p.galley(pos2(r.center().x - g.size().x / 2.0, r.bottom() - 19.0), g, TEXT);
    if hov {
        p.rect_stroke(r, 6, Stroke::new(1.5, MAIN), StrokeKind::Inside);
    }
    let clic = resp.clicked();
    resp.on_hover_text(t(a.ayuda));
    clic
}
