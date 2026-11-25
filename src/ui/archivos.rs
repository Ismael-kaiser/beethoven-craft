//! Panel de archivos: carpetas y subcarpetas del equipo, búsqueda, miniaturas y vista previa. Todo se arrastra a las pistas.
use super::biblioteca::Soltar;
use super::estilo::*;
use super::*;
use crate::explorador::{self as ex, Entrada, Mini, Orden, Recuerdo, Taller, Tipo};
use crate::idioma::{t, tf};
use std::collections::HashSet;
use std::sync::atomic::AtomicU64;

/// Cada cuánto se revisa si cambió lo que hay en una carpeta abierta.
const REVISAR: Duration = Duration::from_secs(3);
/// Segundos de sonido que se leen para oír un archivo.
const OIR: f64 = 180.0;
const FILA: f32 = 27.0;

struct Listado {
    v: Result<Arc<Vec<Entrada>>, String>,
    fecha: Option<std::time::SystemTime>,
    revisado: Instant,
}

struct MiniLista {
    info: Option<Info>,
    tex: Option<TextureHandle>,
    onda: Vec<f32>,
}

/// El archivo que se ve y se oye abajo.
struct Vista {
    ruta: PathBuf,
    info: Info,
    t: f64,
    /// Con el ratón encima se recorre el video sin moverlo de donde va.
    paseo: Option<f64>,
    tocando: bool,
    /// Se pidió tocar y el sonido se está leyendo.
    espera: bool,
    audio: Option<(Arc<Audio>, f64)>,
    p: Arc<Proyecto>,
    vista: VistaPrevia,
    tex: Option<TextureHandle>,
}

enum Aviso {
    Lista(PathBuf, Option<std::time::SystemTime>, Result<Vec<Entrada>, String>),
    Igual(PathBuf),
    Busqueda(u64, Vec<Entrada>),
    Mini(PathBuf, Mini),
    Sonido(PathBuf, f64, Option<Audio>),
}

pub struct Estado {
    pub rec: Recuerdo,
    guardado: Recuerdo,
    listados: HashMap<PathBuf, Listado>,
    pidiendo: HashSet<PathBuf>,
    abiertas: HashSet<PathBuf>,
    pub buscar: String,
    buscada: Option<(PathBuf, String)>,
    hallados: Option<Arc<Vec<Entrada>>>,
    turno: Arc<AtomicU64>,
    pub filtro: Option<Tipo>,
    pub orden: Orden,
    pub elegidos: Vec<PathBuf>,
    ancla: Option<PathBuf>,
    minis: HashMap<PathBuf, MiniLista>,
    pedidas: HashSet<PathBuf>,
    taller: Option<Taller>,
    atras: Vec<Option<PathBuf>>,
    adelante: Vec<Option<PathBuf>>,
    vista: Option<Vista>,
    tx: Sender<Aviso>,
    rx: Receiver<Aviso>,
}

impl Default for Estado {
    fn default() -> Self {
        let (tx, rx) = unbounded();
        Estado {
            rec: Recuerdo::default(),
            guardado: Recuerdo::default(),
            listados: HashMap::new(),
            pidiendo: HashSet::new(),
            abiertas: HashSet::new(),
            buscar: String::new(),
            buscada: None,
            hallados: None,
            turno: Arc::new(AtomicU64::new(0)),
            filtro: None,
            orden: Orden::Nombre,
            elegidos: vec![],
            ancla: None,
            minis: HashMap::new(),
            pedidas: HashSet::new(),
            taller: None,
            atras: vec![],
            adelante: vec![],
            vista: None,
            tx,
            rx,
        }
    }
}

fn archivo_recuerdo() -> PathBuf {
    carpeta().join("Explorador.json")
}

impl Estado {
    pub fn cargar(&mut self) {
        self.rec = Recuerdo::leer(&archivo_recuerdo());
        self.rec.carpeta = self.rec.carpeta.take().filter(|c| c.is_dir());
        self.guardado = self.rec.clone();
    }

    fn recordar(&mut self) {
        if self.rec != self.guardado {
            self.guardado = self.rec.clone();
            self.rec.guardar(&archivo_recuerdo());
        }
    }

    /// Cuánto dura un archivo ya visto.
    pub fn dur(&self, r: &Path) -> Option<f64> {
        let i = self.minis.get(r)?.info.as_ref()?;
        Some(if es_imagen(r) { 5.0 } else { i.dur })
    }

    pub fn ir_a(&mut self, c: Option<PathBuf>) {
        if c != self.rec.carpeta {
            self.atras.push(std::mem::replace(&mut self.rec.carpeta, c));
            self.adelante.clear();
            self.al_llegar();
        }
    }

    fn al_llegar(&mut self) {
        self.buscar.clear();
        self.elegidos.clear();
        self.ancla = None;
        if let Some(l) = self.rec.carpeta.as_ref().and_then(|c| self.listados.get_mut(c)) {
            l.revisado = Instant::now() - REVISAR * 2;
        }
    }

    fn volver(&mut self, atras: bool) {
        let (de, a) = if atras { (&mut self.atras, &mut self.adelante) } else { (&mut self.adelante, &mut self.atras) };
        if let Some(c) = de.pop() {
            a.push(std::mem::replace(&mut self.rec.carpeta, c));
            self.al_llegar();
        }
    }

    fn subir(&mut self) {
        let arriba = self.rec.carpeta.as_ref().and_then(|c| c.parent()).map(Path::to_path_buf);
        self.ir_a(arriba);
    }

    /// Lo de una carpeta, pidiéndolo aparte si falta o si ya toca revisarlo.
    fn lista(&mut self, c: &Path) -> Option<Result<Arc<Vec<Entrada>>, String>> {
        let (v, fecha, toca) = match self.listados.get(c) {
            Some(l) => (Some(l.v.clone()), l.fecha, l.revisado.elapsed() > REVISAR),
            None => (None, None, true),
        };
        if toca && self.pidiendo.insert(c.to_path_buf()) {
            let (tx, c) = (self.tx.clone(), c.to_path_buf());
            std::thread::spawn(move || {
                let ahora = std::fs::metadata(&c).and_then(|m| m.modified()).ok();
                let _ = tx.send(if ahora.is_some() && ahora == fecha {
                    Aviso::Igual(c)
                } else {
                    let l = ex::listar(&c);
                    Aviso::Lista(c, ahora, l)
                });
            });
        }
        v
    }

    fn reordenar(&mut self) {
        for l in self.listados.values_mut() {
            if let Ok(v) = &mut l.v {
                ex::ordenar(Arc::make_mut(v).as_mut_slice(), self.orden);
            }
        }
        if let Some(v) = &mut self.hallados {
            ex::ordenar(Arc::make_mut(v).as_mut_slice(), self.orden);
        }
    }

    fn recibir(&mut self, ctx: &Context) {
        while let Ok(a) = self.rx.try_recv() {
            match a {
                Aviso::Lista(c, fecha, v) => {
                    self.pidiendo.remove(&c);
                    let v = v.map(|mut v| {
                        ex::ordenar(&mut v, self.orden);
                        Arc::new(v)
                    });
                    self.listados.insert(c, Listado { v, fecha, revisado: Instant::now() });
                }
                Aviso::Igual(c) => {
                    self.pidiendo.remove(&c);
                    if let Some(l) = self.listados.get_mut(&c) {
                        l.revisado = Instant::now();
                    }
                }
                Aviso::Busqueda(n, mut v) => {
                    if n == self.turno.load(Relaxed) {
                        ex::ordenar(&mut v, self.orden);
                        self.hallados = Some(Arc::new(v));
                    }
                }
                Aviso::Mini(r, m) => {
                    if self.minis.len() > 900 {
                        self.minis.clear();
                        self.pedidas.clear();
                    }
                    let tex = m.imagen.map(|(w, h, q)| ctx.load_texture(format!("mini:{}", r.display()), ColorImage::from_rgba_unmultiplied([w, h], &q), TextureOptions::LINEAR));
                    self.minis.insert(r, MiniLista { info: m.info, tex, onda: m.onda });
                }
                Aviso::Sonido(r, desde, a) => {
                    if let Some(v) = self.vista.as_mut().filter(|v| v.ruta == r) {
                        // Sin sonido, uno vacío lleva la cuenta.
                        v.audio = Some((Arc::new(a.unwrap_or_else(|| Audio::nuevo(vec![]))), desde));
                    }
                }
            }
        }
    }

    fn pedir_mini(&mut self, e: &Entrada) {
        if !e.tipo.es_medio() || self.minis.contains_key(&e.ruta) || !self.pedidas.insert(e.ruta.clone()) {
            return;
        }
        let tx = self.tx.clone();
        let taller = self.taller.get_or_insert_with(|| {
            Taller::nuevo(move |r, m| {
                let _ = tx.send(Aviso::Mini(r, m));
            })
        });
        for r in taller.pedir(e.ruta.clone()) {
            self.pedidas.remove(&r);
        }
    }

    fn pasa(&self, e: &Entrada) -> bool {
        match self.filtro {
            Some(f) => e.tipo == f,
            None => self.rec.todos || e.tipo != Tipo::Otro,
        }
    }

    /// Lo que se ve, en orden: nivel dentro del árbol, y la entrada o un aviso.
    fn filas(&mut self) -> Vec<(usize, Result<Entrada, String>)> {
        let Some(c) = self.rec.carpeta.clone() else { return vec![] };
        let mut v = vec![];
        if !self.buscar.trim().is_empty() {
            let q = (c.clone(), self.buscar.trim().to_string());
            if self.buscada.as_ref() != Some(&q) {
                let n = self.turno.fetch_add(1, Relaxed) + 1;
                self.buscada = Some(q.clone());
                self.hallados = None;
                let (tx, turno) = (self.tx.clone(), self.turno.clone());
                std::thread::spawn(move || {
                    // Se espera a que termine de escribir.
                    std::thread::sleep(Duration::from_millis(180));
                    let r = ex::buscar(&q.0, &q.1, &turno, n);
                    let _ = tx.send(Aviso::Busqueda(n, r));
                });
            }
            match self.hallados.clone() {
                None => v.push((0, Err(t("Buscando…").into()))),
                Some(h) => {
                    v.extend(h.iter().filter(|e| self.pasa(e) || e.tipo == Tipo::Carpeta).map(|e| (0, Ok(e.clone()))));
                    if v.is_empty() {
                        v.push((0, Err(t("No hay nada con ese nombre aquí").into())));
                    }
                }
            }
            return v;
        }
        if self.buscada.take().is_some() {
            self.turno.fetch_add(1, Relaxed);
            self.hallados = None;
        }
        let plano = self.rec.cuadricula;
        self.ramas(&c, 0, plano, &mut v);
        v
    }

    fn ramas(&mut self, c: &Path, nivel: usize, plano: bool, v: &mut Vec<(usize, Result<Entrada, String>)>) {
        match self.lista(c) {
            None => v.push((nivel, Err(t("Leyendo…").into()))),
            Some(Err(e)) => v.push((nivel, Err(e))),
            Some(Ok(l)) => {
                let antes = v.len();
                for e in l.iter() {
                    if e.tipo == Tipo::Carpeta {
                        v.push((nivel, Ok(e.clone())));
                        // Un enlace que regresa a una carpeta de arriba no se abre sin fin.
                        if !plano && nivel < 24 && self.abiertas.contains(&e.ruta) {
                            self.ramas(&e.ruta, nivel + 1, plano, v);
                        }
                    } else if self.pasa(e) {
                        v.push((nivel, Ok(e.clone())));
                    }
                }
                if v.len() == antes {
                    v.push((nivel, Err(t("Nada que mostrar aquí").into())));
                }
            }
        }
    }

    /// Clic en un archivo, con ⌘ o Shift para elegir varios.
    fn elegir(&mut self, r: &Path, m: Modifiers, filas: &[(usize, Result<Entrada, String>)]) {
        if m.shift {
            let pos = |x: &Path| filas.iter().position(|f| f.1.as_ref().is_ok_and(|e| e.ruta == x));
            if let (Some(a), Some(b)) = (self.ancla.as_deref().and_then(pos), pos(r)) {
                self.elegidos = filas[a.min(b)..=a.max(b)].iter().filter_map(|f| f.1.as_ref().ok()).filter(|e| e.tipo != Tipo::Carpeta).map(|e| e.ruta.clone()).collect();
                return;
            }
        }
        if m.command {
            match self.elegidos.iter().position(|x| x == r) {
                Some(i) => _ = self.elegidos.remove(i),
                None => self.elegidos.push(r.into()),
            }
        } else {
            self.elegidos = vec![r.into()];
        }
        self.ancla = Some(r.into());
    }

    pub fn parar_vista(&mut self) {
        if let Some(v) = &mut self.vista {
            (v.tocando, v.espera) = (false, false);
        }
    }
}

fn dibujo(tipo: Tipo, abierta: bool) -> &'static tabler::TablerIcon {
    match tipo {
        Tipo::Carpeta if abierta => &tabler::FOLDER_OPEN,
        Tipo::Carpeta => &tabler::FOLDER,
        Tipo::Video => &tabler::MOVIE,
        Tipo::Audio => &tabler::FILE_MUSIC,
        Tipo::Imagen => &tabler::SQUARES,
        Tipo::Proyecto => &tabler::FILE_STAR,
        Tipo::Midi => &tabler::PIANO,
        Tipo::Subtitulos => &tabler::FILE_TEXT,
        Tipo::Otro => &tabler::FILE,
    }
}

/// Recorta un texto por en medio para que se vea la terminación.
fn recortado(ui: &Ui, s: &str, f: &FontId, ancho: f32) -> String {
    let mide = |s: &str| ui.painter().layout_no_wrap(s.into(), f.clone(), TEXT).size().x;
    if mide(s) <= ancho {
        return s.into();
    }
    let c: Vec<char> = s.chars().collect();
    let cola: String = c[c.len().saturating_sub(7)..].iter().collect();
    let (mut a, mut b) = (0, c.len().saturating_sub(7));
    while a + 1 < b {
        let m = (a + b) / 2;
        let x: String = c[..m].iter().collect::<String>() + "…" + &cola;
        if mide(&x) <= ancho { a = m } else { b = m }
    }
    c[..a].iter().collect::<String>() + "…" + &cola
}

/// La forma de un sonido, de orilla a orilla.
fn pintar_onda(p: &Painter, r: Rect, onda: &[f32], col: Color32) {
    let n = onda.len().max(1) as f32;
    let pico = onda.iter().copied().fold(0.02, f32::max);
    for (i, v) in onda.iter().enumerate() {
        let x = r.left() + (i as f32 + 0.5) / n * r.width();
        let h = (v / pico * r.height() / 2.0).max(0.5);
        p.vline(x, (r.center().y - h)..=(r.center().y + h), Stroke::new((r.width() / n * 0.7).clamp(0.6, 3.0), col));
    }
}

/// La mayor área con la forma de la imagen.
fn cabe(r: Rect, w: f32, h: f32) -> Rect {
    let k = (r.width() / w.max(1.0)).min(r.height() / h.max(1.0));
    Rect::from_center_size(r.center(), vec2(w, h) * k)
}

const TODA: Rect = Rect { min: pos2(0.0, 0.0), max: pos2(1.0, 1.0) };

impl App {
    /// El panel, a la izquierda de cualquier pestaña.
    pub fn panel_archivos(&mut self, ui: &mut Ui) {
        self.archivos.recibir(ui.ctx());
        if self.archivos.rec.abierto && !self.archivos.rec.anclado {
            self.ventana_archivos(ui);
        } else if self.archivos.rec.abierto {
            let ancho = ui.available_width();
            Panel::left("archivos").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(Margin::symmetric(7, 6))).resizable(true).default_size(256.0).size_range(190.0..=(ancho * 0.45).max(200.0)).show(ui, |ui| self.ui_archivos(ui));
        } else if self.archivos.vista.as_ref().is_some_and(|v| v.tocando) {
            self.parar_origen();
        }
        if !self.sin_dispositivo && !ui.input(|i| i.pointer.any_down()) {
            self.archivos.recordar();
        }
    }

    /// Flotando encima de todo: se mueve de donde no hay nada y se estira de las orillas.
    fn ventana_archivos(&mut self, ui: &mut Ui) {
        let pantalla = ui.ctx().content_rect();
        let nuevo = [pantalla.left() + 24.0, pantalla.top() + 90.0, 760.0f32.min(pantalla.width() - 60.0), (pantalla.height() - 150.0).clamp(380.0, 560.0)];
        // Las ventanas guardadas de antes eran una columna angosta: se abren con el tamaño nuevo.
        let [x, y, w, h] = self.archivos.rec.lugar.filter(|l| l[2] >= 500.0).unwrap_or(nuevo);
        let sombra = Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(140) };
        let marco = Frame::new().fill(WINDOW_BODY).corner_radius(10.0).inner_margin(Margin::symmetric(8, 8)).stroke(Stroke::new(1.0, c(0x151515))).shadow(sombra);
        let r = Window::new("archivos")
            .id(Id::new("ventana-archivos"))
            .title_bar(false)
            .frame(marco)
            .resizable(true)
            .default_pos(pos2(x, y))
            .default_size(vec2(w, h))
            .min_size(vec2(300.0, 340.0))
            .constrain_to(pantalla)
            .show(ui.ctx(), |ui| {
                ui.set_min_size(ui.available_size());
                self.ui_archivos(ui);
            });
        if let Some(r) = r {
            let q = r.response.rect;
            self.archivos.rec.lugar = Some([q.left(), q.top(), q.width() - 20.0, q.height() - 18.0].map(f32::round));
        }
    }

    fn ui_archivos(&mut self, ui: &mut Ui) {
        ui.set_min_width(ui.available_width());
        // Ancha (en su ventana): lugares siempre a la izquierda, lista al centro y vista a la derecha.
        let amplio = !self.archivos.rec.anclado && ui.available_width() >= 540.0;
        // Barra de herramientas sobre una placa de metal.
        let (placa, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
        relieve(ui.painter(), placa, 7, c(0x3a3a3a), false, false);
        let mut cab = ui.new_child(UiBuilder::new().max_rect(placa.shrink2(vec2(5.0, 4.0))).layout(Layout::left_to_right(Align::Center)));
        self.cabecera_archivos(&mut cab, amplio);
        ui.add_space(5.0);
        if !amplio {
            self.ruta_ui(ui);
            if self.archivos.rec.carpeta.is_none() {
                return self.lugares_ui(ui);
            }
        }
        if amplio {
            Panel::left("archivos-lado").frame(Frame::new().fill(c(0x232323)).corner_radius(6).inner_margin(Margin::symmetric(4, 4))).resizable(true).default_size(176.0).size_range(140.0..=260.0).show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                self.lugares_ui(ui);
            });
            if self.archivos.rec.carpeta.is_none() {
                let r = ui.available_rect_before_wrap();
                ui.painter().text(r.center(), Align2::CENTER_CENTER, t("Elige un lugar a la izquierda"), fuente(15.0), TEXT_FAINT);
                return;
            }
        }
        // Buscar y filtros.
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.archivos.buscar).hint_text(t("Buscar aquí y en subcarpetas…")).desired_width(if amplio { 220.0 } else { f32::INFINITY }));
            if amplio {
                self.filtros_archivos(ui);
            }
        });
        if !amplio {
            ui.horizontal_wrapped(|ui| self.filtros_archivos(ui));
        }
        ui.add_space(4.0);
        let vista_der = amplio && ui.available_width() >= 620.0;
        if vista_der {
            Panel::right("archivos-vista-der").frame(Frame::new().inner_margin(Margin { left: 8, ..Default::default() })).resizable(true).default_size(250.0).size_range(200.0..=380.0).show(ui, |ui| self.vista_ui(ui));
        } else {
            Panel::bottom("archivos-vista").frame(Frame::NONE).resizable(true).default_size(210.0).size_range(110.0..=(ui.available_height() - 110.0).max(120.0)).show(ui, |ui| self.vista_ui(ui));
        }
        let filas = self.archivos.filas();
        if self.archivos.rec.cuadricula { self.cuadricula_ui(ui, &filas) } else { self.lista_ui(ui, &filas) }
    }

    /// Atrás, adelante, subir, lugares; en la ventana ancha también la ruta. A la derecha: orden, cuadrícula, pegar y
    /// cerrar.
    fn cabecera_archivos(&mut self, ui: &mut Ui, amplio: bool) {
        ui.spacing_mut().item_spacing.x = 3.0;
        let a = &mut self.archivos;
        if boton_color(ui, &tabler::ARROW_LEFT, false, "Carpeta anterior", if a.atras.is_empty() { TEXT_FAINT } else { GLYPH }, 24.0).clicked() {
            a.volver(true);
        }
        if boton_color(ui, &tabler::CARET_RIGHT, false, "Carpeta siguiente", if a.adelante.is_empty() { TEXT_FAINT } else { GLYPH }, 24.0).clicked() {
            a.volver(false);
        }
        if boton_color(ui, &tabler::CARET_UP, false, "Subir una carpeta", if a.rec.carpeta.is_none() { TEXT_FAINT } else { GLYPH }, 24.0).clicked() {
            a.subir();
        }
        if !amplio && boton_color(ui, &tabler::FOLDERS, a.rec.carpeta.is_none(), "Lugares y favoritos", GLYPH, 24.0).clicked() {
            a.ir_a(None);
        }
        let derecha = 4.0 * 27.0;
        if amplio {
            let mut u = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(ui.cursor().min + vec2(6.0, 0.0), pos2(ui.max_rect().right() - derecha, ui.max_rect().bottom()))).layout(Layout::left_to_right(Align::Center)));
            self.ruta_ui(&mut u);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if boton_color(ui, &tabler::X, false, "Cerrar el panel de archivos", GLYPH, 24.0).clicked() {
                self.archivos.rec.abierto = false;
            }
            let a = &mut self.archivos;
            if boton_color(ui, &tabler::LAYOUT_ROWS, a.rec.anclado, if a.rec.anclado { "Que flote en su ventana" } else { "Pegar a la izquierda" }, GLYPH, 24.0).clicked() {
                a.rec.anclado = !a.rec.anclado;
            }
            if boton_color(ui, &tabler::GRID_DOTS, a.rec.cuadricula, "Ver como cuadrícula", GLYPH, 24.0).clicked() {
                a.rec.cuadricula = !a.rec.cuadricula;
            }
            let r = boton_color(ui, &tabler::ADJUSTMENTS_HORIZONTAL, false, "Orden y qué se muestra", GLYPH, 24.0);
            Popup::menu(&r).show(|ui| {
                ui.set_min_width(190.0);
                ui.label(RichText::new(t("Ordenar por")).color(TEXT_FAINT).size(13.0));
                for (o, n) in Orden::TODOS {
                    if ui.selectable_label(a.orden == o, t(n)).clicked() {
                        a.orden = o;
                        a.reordenar();
                    }
                }
                ui.separator();
                ui.checkbox(&mut a.rec.todos, t("Mostrar todos los archivos"));
                if let Some(c) = a.rec.carpeta.clone() {
                    let ya = a.rec.favoritos.contains(&c);
                    if ui.button(t(if ya { "Quitar esta carpeta de favoritos" } else { "Agregar esta carpeta a favoritos" })).clicked() {
                        if ya { a.rec.favoritos.retain(|f| *f != c) } else { a.rec.favoritos.push(c) }
                    }
                }
            });
        });
    }

    fn filtros_archivos(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
        for (f, n) in [(None, "Todo"), (Some(Tipo::Video), "Video"), (Some(Tipo::Audio), "Audio"), (Some(Tipo::Imagen), "Imagen")] {
            if biblioteca::chip(ui, t(n), self.archivos.filtro == f).clicked() {
                self.archivos.filtro = f;
            }
        }
    }

    /// «Videos › Viaje › Día 1»: cada parte se puede abrir.
    fn ruta_ui(&mut self, ui: &mut Ui) {
        let Some(c) = self.archivos.rec.carpeta.clone() else {
            ui.label(RichText::new(t("Archivos")).size(16.0).color(TEXT));
            return;
        };
        let partes: Vec<PathBuf> = c.ancestors().map(Path::to_path_buf).collect();
        let nombre = |r: &Path| r.file_name().map_or_else(|| r.display().to_string(), |n| n.to_string_lossy().into_owned());
        let mut ir = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            let n = partes.len().min(3);
            if partes.len() > n {
                ui.menu_button("…", |ui| {
                    for p in partes[n..].iter() {
                        if ui.button(nombre(p)).clicked() {
                            ir = Some(p.clone());
                        }
                    }
                });
            }
            let cada = (ui.available_width() - 12.0 * n as f32) / n as f32;
            for (i, p) in partes[..n].iter().enumerate().rev() {
                let f = fuente(14.0);
                let texto = recortado(ui, &nombre(p), &f, if i == 0 { cada * 1.4 } else { cada * 0.8 });
                let r = ui.add(Label::new(RichText::new(texto).font(f).color(if i == 0 { TEXT } else { TEXT_DIM })).sense(Sense::click())).on_hover_text(p.display().to_string());
                if r.clicked() {
                    ir = Some(p.clone());
                }
                if i > 0 {
                    ui.label(RichText::new("›").color(TEXT_FAINT).size(13.0));
                }
            }
        });
        if let Some(p) = ir {
            self.archivos.ir_a(Some(p));
        }
    }

    fn lugares_ui(&mut self, ui: &mut Ui) {
        let id = ui.id().with("lugares");
        // Los discos se revisan de vez en cuando, no en cada cuadro.
        let (discos, cuando): (Vec<PathBuf>, f64) = ui.data(|d| d.get_temp(id)).unwrap_or((vec![], -10.0));
        let ahora = ui.input(|i| i.time);
        let discos = if ahora - cuando > 3.0 {
            let v = if self.sin_dispositivo { vec![] } else { ex::discos() };
            ui.data_mut(|d| d.insert_temp(id, (v.clone(), ahora)));
            v
        } else {
            discos
        };
        let mut ir = None;
        let mut quitar = None;
        let actual = self.archivos.rec.carpeta.clone();
        ScrollArea::vertical().id_salt("archivos-lugares").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            let mut grupo = |ui: &mut Ui, titulo: &str, v: Vec<(String, PathBuf, &'static tabler::TablerIcon)>, fav: bool| {
                if v.is_empty() {
                    return;
                }
                ui.add_space(6.0);
                ui.label(RichText::new(t(titulo).to_uppercase()).color(TEXT_FAINT).size(11.0));
                for (n, r, ic) in v {
                    let (rc, resp) = ui.allocate_exact_size(vec2(ui.available_width(), FILA), Sense::click());
                    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &n));
                    let aqui = actual.as_deref() == Some(r.as_path());
                    if aqui {
                        ui.painter().rect_filled(rc, 5, MAIN.gamma_multiply(0.14));
                        ui.painter().rect_filled(Rect::from_min_size(rc.min + vec2(0.0, 4.0), vec2(3.0, rc.height() - 8.0)), 1, MAIN);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(rc, 5, BUTTON);
                    }
                    icono(ui.painter(), ic, Rect::from_center_size(pos2(rc.left() + 14.0, rc.center().y), Vec2::splat(16.0)), MAIN);
                    ui.painter_at(rc).text(pos2(rc.left() + 30.0, rc.center().y), Align2::LEFT_CENTER, &n, fuente(14.0), if aqui { MAIN } else { TEXT });
                    if resp.clicked() {
                        ir = Some(r.clone());
                    }
                    let resp = resp.on_hover_text(r.display().to_string());
                    if fav {
                        resp.context_menu(|ui| {
                            if ui.button(t("Quitar de favoritos")).clicked() {
                                quitar = Some(r.clone());
                            }
                        });
                    }
                }
            };
            let nombre = |r: &Path| r.file_name().map_or_else(|| r.display().to_string(), |n| n.to_string_lossy().into_owned());
            let mut proyecto = vec![];
            if let Some(c) = self.ruta.as_ref().and_then(|r| r.parent()) {
                proyecto.push((t("Carpeta del proyecto").to_string(), c.to_path_buf(), &tabler::FILE_STAR));
            }
            let nuestra = carpeta();
            if nuestra.is_dir() {
                proyecto.push(("Beethoven Craft".into(), nuestra, &tabler::FOLDER));
            }
            grupo(ui, "Proyecto", proyecto, false);
            grupo(ui, "Favoritos", self.archivos.rec.favoritos.iter().map(|r| (nombre(r), r.clone(), &tabler::STAR)).collect(), true);
            grupo(ui, "Lugares", ex::lugares().into_iter().map(|(n, r)| (t(n).to_string(), r, &tabler::FOLDER)).collect(), false);
            grupo(ui, "Discos", discos.iter().map(|r| (nombre(r), r.clone(), &tabler::BOX)).collect(), false);
        });
        if let Some(r) = quitar {
            self.archivos.rec.favoritos.retain(|f| *f != r);
        }
        if let Some(r) = ir {
            self.archivos.ir_a(Some(r));
        }
    }

    fn lista_ui(&mut self, ui: &mut Ui, filas: &[(usize, Result<Entrada, String>)]) {
        ui.spacing_mut().item_spacing.y = 1.0;
        // Encabezado de columnas: clic ordena por esa columna.
        let con_fecha = ui.available_width() > 430.0;
        let (enc, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
        degradado(ui.painter(), enc, 4, c(0x363636), c(0x2a2a2a));
        ui.painter().hline(enc.x_range(), enc.bottom() - 0.5, Stroke::new(1.0, Color32::from_black_alpha(140)));
        let columnas = [(Orden::Nombre, "Nombre", enc.left() + 8.0, Align2::LEFT_CENTER), (Orden::Fecha, "Fecha", enc.right() - 92.0, Align2::RIGHT_CENTER), (Orden::Tamano, "Duración / peso", enc.right() - 8.0, Align2::RIGHT_CENTER)];
        for (o, n, x, al) in columnas {
            if o == Orden::Fecha && !con_fecha {
                continue;
            }
            let on = self.archivos.orden == o;
            let r = ui.painter().text(pos2(x, enc.center().y), al, format!("{}{}", t(n), if on { " ▾" } else { "" }), fuente(12.0), if on { MAIN } else { TEXT_DIM });
            let rr = ui.interact(r.expand(3.0), Id::new(("orden-col", o as u8)), Sense::click());
            rr.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tf("Ordenar por {}", &[&t(n)])));
            if rr.clicked() && !on {
                self.archivos.orden = o;
                self.archivos.reordenar();
            }
        }
        ui.add_space(2.0);
        ScrollArea::vertical().id_salt(("archivos-lista", &self.archivos.rec.carpeta)).auto_shrink([false, false]).show_rows(ui, FILA, filas.len(), |ui, vis| {
            let desde = vis.start;
            for (k, (nivel, f)) in filas[vis].iter().enumerate() {
                let (rc, resp) = ui.allocate_exact_size(vec2(ui.available_width(), FILA), Sense::click_and_drag());
                // Renglones alternados: se sigue la fila con la vista.
                if (desde + k) % 2 == 1 {
                    ui.painter().rect_filled(rc, 0, Color32::from_white_alpha(4));
                }
                let x = rc.left() + 4.0 + *nivel as f32 * 13.0;
                let e = match f {
                    Ok(e) => e,
                    Err(s) => {
                        ui.painter_at(rc).text(pos2(x + 8.0, rc.center().y), Align2::LEFT_CENTER, s, fuente(13.5), TEXT_FAINT);
                        continue;
                    }
                };
                resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &e.nombre));
                let a = &mut self.archivos;
                a.pedir_mini(e);
                let dir = e.tipo == Tipo::Carpeta;
                let abierta = dir && a.abiertas.contains(&e.ruta);
                let on = a.elegidos.contains(&e.ruta);
                if on {
                    ui.painter().rect_filled(rc, 4, MAIN.gamma_multiply(0.16));
                    ui.painter().rect_filled(Rect::from_min_size(rc.min + vec2(0.0, 3.0), vec2(3.0, rc.height() - 6.0)), 1, MAIN);
                } else if resp.hovered() {
                    ui.painter().rect_filled(rc, 4, BUTTON);
                }
                // Flecha para ver la subcarpeta aquí mismo
                let fl = Rect::from_center_size(pos2(x + 6.0, rc.center().y), Vec2::splat(18.0));
                if dir && a.buscar.trim().is_empty() {
                    let rf = ui.interact(fl, Id::new(("abrir-rama", &e.ruta)), Sense::click());
                    rf.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tf("Ver dentro de {}", &[&e.nombre])));
                    let (c, s) = (fl.center(), 3.5);
                    let tri = if abierta { [vec2(-s, -s * 0.6), vec2(s, -s * 0.6), vec2(0.0, s * 0.8)] } else { [vec2(-s * 0.6, -s), vec2(s * 0.8, 0.0), vec2(-s * 0.6, s)] };
                    ui.painter().add(Shape::convex_polygon(tri.map(|v| c + v).to_vec(), if rf.hovered() { TEXT } else { TEXT_DIM }, Stroke::NONE));
                    if rf.clicked() && !a.abiertas.remove(&e.ruta) {
                        a.abiertas.insert(e.ruta.clone());
                    }
                }
                let m = a.minis.get(&e.ruta);
                let im = Rect::from_min_size(pos2(x + 16.0, rc.top() + 3.5), vec2(34.0, 20.0));
                let col = match e.tipo {
                    Tipo::Carpeta => MAIN,
                    Tipo::Otro => TEXT_FAINT,
                    _ => TEXT_DIM,
                };
                let mut tx = x + 36.0;
                match m {
                    Some(MiniLista { tex: Some(x), .. }) => {
                        let s = x.size_vec2();
                        ui.painter().rect_filled(im, 2, Color32::BLACK);
                        ui.painter().image(x.id(), cabe(im, s.x, s.y), TODA, Color32::WHITE);
                        tx = im.right() + 7.0;
                    }
                    Some(MiniLista { onda, .. }) if !onda.is_empty() => {
                        ui.painter().rect_filled(im, 2, WELL);
                        pintar_onda(&ui.painter_at(im), im.shrink(2.0), &onda.iter().step_by(6).copied().collect::<Vec<_>>(), DEEP);
                        tx = im.right() + 7.0;
                    }
                    _ => icono(ui.painter(), dibujo(e.tipo, abierta), Rect::from_center_size(pos2(x + 24.0, rc.center().y), Vec2::splat(16.0)), col),
                }
                let dato = match m.and_then(|m| m.info.as_ref()) {
                    Some(i) if e.tipo != Tipo::Imagen && i.dur > 0.0 => tiempo(i.dur),
                    _ if dir => String::new(),
                    _ => ex::peso(e.tam),
                };
                let mut fin = rc.right() - 6.0;
                if rc.width() > 215.0 && !dato.is_empty() {
                    fin = ui.painter().text(pos2(fin, rc.center().y), Align2::RIGHT_CENTER, dato, fuente(12.0), if on { TEXT } else { TEXT_FAINT }).left() - 6.0;
                }
                if rc.width() > 430.0 {
                    fin = fin.min(rc.right() - 180.0);
                    ui.painter().text(pos2(rc.right() - 92.0, rc.center().y), Align2::RIGHT_CENTER, fecha_corta(e.fecha), fuente(12.0), TEXT_FAINT);
                }
                let f = fuente(14.5);
                let n = recortado(ui, &e.nombre, &f, fin - tx);
                ui.painter_at(rc).text(pos2(tx, rc.center().y), Align2::LEFT_CENTER, n, f, if e.tipo == Tipo::Otro { TEXT_FAINT } else { TEXT });
                self.atender_archivo(ui, resp, e, filas);
            }
        });
    }

    fn cuadricula_ui(&mut self, ui: &mut Ui, filas: &[(usize, Result<Entrada, String>)]) {
        let ancho = ui.available_width() - 10.0;
        let n = (ancho / 112.0).floor().max(1.0);
        let (w, h) = ((ancho - (n - 1.0) * 6.0) / n, 92.0);
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        let filas_n = filas.len().div_ceil(n as usize);
        ScrollArea::vertical().id_salt(("archivos-cuadros", &self.archivos.rec.carpeta)).auto_shrink([false, false]).show_rows(ui, h, filas_n, |ui, vis| {
            for k in vis {
                ui.horizontal(|ui| {
                    for (_, f) in filas.iter().skip(k * n as usize).take(n as usize) {
                        let e = match f {
                            Ok(e) => e,
                            Err(s) => {
                                ui.label(RichText::new(s).color(TEXT_FAINT).size(13.5));
                                continue;
                            }
                        };
                        let (rc, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click_and_drag());
                        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &e.nombre));
                        let a = &mut self.archivos;
                        a.pedir_mini(e);
                        let on = a.elegidos.contains(&e.ruta);
                        ui.painter().rect_filled(rc, 6, if on { MAIN.gamma_multiply(0.14) } else if resp.hovered() { BUTTON } else { c(0x262626) });
                        if on || resp.hovered() {
                            ui.painter().rect_stroke(rc, 6, Stroke::new(if on { 2.0 } else { 1.0 }, if on { MAIN } else { BUTTON_HI }), StrokeKind::Inside);
                        }
                        let im = Rect::from_min_max(rc.min + vec2(4.0, 4.0), pos2(rc.right() - 4.0, rc.bottom() - 22.0));
                        match a.minis.get(&e.ruta) {
                            Some(MiniLista { tex: Some(x), .. }) => {
                                let s = x.size_vec2();
                                ui.painter().rect_filled(im, 4, Color32::BLACK);
                                ui.painter().image(x.id(), cabe(im, s.x, s.y), TODA, Color32::WHITE);
                            }
                            Some(MiniLista { onda, .. }) if !onda.is_empty() => {
                                ui.painter().rect_filled(im, 4, WELL);
                                pintar_onda(&ui.painter_at(im), im.shrink(4.0), &onda.iter().step_by(3).copied().collect::<Vec<_>>(), DEEP);
                            }
                            _ => icono(ui.painter(), dibujo(e.tipo, false), Rect::from_center_size(im.center(), Vec2::splat(30.0)), if e.tipo == Tipo::Carpeta { MAIN } else { TEXT_FAINT }),
                        }
                        if let Some(i) = a.minis.get(&e.ruta).and_then(|m| m.info.as_ref()).filter(|i| e.tipo != Tipo::Imagen && i.dur > 0.0) {
                            let g = ui.painter().layout_no_wrap(tiempo(i.dur), fuente(11.5), TEXT);
                            let c = Rect::from_min_size(im.max - g.size() - vec2(8.0, 4.0), g.size() + vec2(6.0, 2.0));
                            ui.painter().rect_filled(c, 3, Color32::from_black_alpha(190));
                            ui.painter().galley(c.min + vec2(3.0, 1.0), g, TEXT);
                        }
                        let f = fuente(13.0);
                        let nombre = recortado(ui, &e.nombre, &f, rc.width() - 10.0);
                        ui.painter_at(rc).text(pos2(rc.center().x, rc.bottom() - 11.0), Align2::CENTER_CENTER, nombre, f, TEXT);
                        self.atender_archivo(ui, resp, e, filas);
                    }
                });
            }
        });
    }

    /// Clic, doble clic, arrastre y menú de un archivo.
    fn atender_archivo(&mut self, ui: &Ui, resp: Response, e: &Entrada, filas: &[(usize, Result<Entrada, String>)]) {
        let dir = e.tipo == Tipo::Carpeta;
        if e.tipo.es_medio() {
            let v = if self.archivos.elegidos.len() > 1 && self.archivos.elegidos.contains(&e.ruta) { self.archivos.elegidos.clone() } else { vec![e.ruta.clone()] };
            if resp.dragged() {
                if let Some(p) = ui.ctx().pointer_interact_pos() {
                    let s = if v.len() > 1 { tf("{} archivos", &[&v.len()]) } else { e.nombre.clone() };
                    let capa = ui.ctx().layer_painter(LayerId::new(Order::Tooltip, Id::new("arrastrando")));
                    let g = capa.layout_no_wrap(s, fuente(14.0), COLORED_DARK);
                    let caja = Rect::from_min_size(p + vec2(12.0, 6.0), g.size() + vec2(14.0, 8.0));
                    capa.rect_filled(caja, 6, MAIN);
                    capa.galley(caja.min + vec2(7.0, 4.0), g, COLORED_DARK);
                }
            }
            resp.dnd_set_drag_payload(Soltar::Medios(v));
        }
        if resp.double_clicked() {
            if dir {
                self.archivos.ir_a(Some(e.ruta.clone()));
            } else {
                let t = self.ahora();
                self.poner_archivos(&[e.ruta.clone()], t, None, None);
            }
        } else if resp.clicked() && !dir {
            let m = ui.input(|i| i.modifiers);
            self.archivos.elegir(&e.ruta, m, filas);
            if e.tipo.es_medio() && !m.any() {
                self.ver_archivo(&e.ruta);
            }
        } else if resp.clicked() && self.archivos.rec.cuadricula {
            self.archivos.elegidos = vec![e.ruta.clone()];
        } else if resp.clicked() && self.archivos.buscar.trim().is_empty() && !self.archivos.abiertas.remove(&e.ruta) {
            self.archivos.abiertas.insert(e.ruta.clone());
        }
        resp.context_menu(|ui| {
            ui.set_min_width(200.0);
            if dir {
                if ui.button(t("Abrir")).clicked() {
                    self.archivos.ir_a(Some(e.ruta.clone()));
                }
                let ya = self.archivos.rec.favoritos.contains(&e.ruta);
                if ui.button(t(if ya { "Quitar de favoritos" } else { "Agregar a favoritos" })).clicked() {
                    if ya { self.archivos.rec.favoritos.retain(|f| *f != e.ruta) } else { self.archivos.rec.favoritos.push(e.ruta.clone()) }
                }
            } else {
                if e.tipo != Tipo::Otro && ui.button(t("Poner en el cursor")).clicked() {
                    let t = self.ahora();
                    self.poner_archivos(&[e.ruta.clone()], t, None, None);
                }
                if e.tipo.es_medio() && ui.button(t("Abrir en Origen")).on_hover_text(t("Para marcar un tramo y meter solo eso")).clicked() {
                    self.abrir_origen(&e.ruta);
                    self.espacio = Espacio::Video;
                }
                if !self.archivos.buscar.trim().is_empty() && ui.button(t("Ir a su carpeta")).clicked() {
                    self.archivos.ir_a(e.ruta.parent().map(Path::to_path_buf));
                }
            }
            ui.separator();
            if ui.button(t(if cfg!(target_os = "macos") { "Mostrar en el Finder" } else { "Mostrar en su carpeta" })).clicked() {
                mostrar(&e.ruta);
            }
            if ui.button(t("Copiar la ruta")).clicked() {
                ui.ctx().copy_text(e.ruta.display().to_string());
            }
        });
        if !resp.dragged() && !resp.context_menu_opened() {
            resp.on_hover_ui_at_pointer(|ui| {
                ui.set_max_width(280.0);
                ui.label(RichText::new(&e.nombre).color(TEXT));
                if dir {
                    ui.label(RichText::new(t("Clic: ver dentro. Doble clic: abrirla.")).size(12.5).color(TEXT_FAINT));
                } else if e.tipo != Tipo::Otro {
                    ui.label(RichText::new(t("Arrástralo a las pistas, o doble clic para ponerlo en el cursor")).size(12.5).color(TEXT_FAINT));
                }
            });
        }
    }

    // ---- vista previa

    pub fn ver_archivo(&mut self, r: &Path) {
        if self.archivos.vista.as_ref().is_some_and(|v| v.ruta == r) {
            return;
        }
        if self.archivos.vista.as_ref().is_some_and(|v| v.tocando) {
            self.parar_origen();
        }
        self.archivos.vista = None;
        // Se arma cuando llega su miniatura, que trae los datos del archivo.
    }

    fn tocar_vista(&mut self) {
        let tocando = self.archivos.vista.as_ref().is_some_and(|v| v.tocando || v.espera);
        self.parar_origen();
        self.archivos.parar_vista();
        let Some(v) = self.archivos.vista.as_mut().filter(|_| !tocando) else { return };
        if v.t >= v.info.dur - 0.05 {
            v.t = 0.0;
        }
        v.espera = true;
        // Lo ya leído sirve si el punto cae adentro.
        if v.audio.as_ref().is_some_and(|(a, d)| v.t >= *d && (v.t < d + a.dur() - 1.0 || !v.info.audio)) {
            return;
        }
        v.audio = None;
        let (tx, r, d, hay) = (self.archivos.tx.clone(), v.ruta.clone(), v.t, v.info.audio);
        std::thread::spawn(move || {
            let a = if hay { medios::decodificar_tramo(&r, d, OIR).ok() } else { None };
            let _ = tx.send(Aviso::Sonido(r, d, a));
        });
    }

    fn vista_ui(&mut self, ui: &mut Ui) {
        ui.set_min_size(ui.available_size());
        let todo = ui.available_rect_before_wrap();
        ui.allocate_rect(todo, Sense::hover());
        let Some(r) = self.archivos.elegidos.last().filter(|r| Tipo::de(r).es_medio()).cloned() else {
            ui.painter().text(todo.center(), Align2::CENTER_CENTER, t("Elige un archivo para verlo aquí"), fuente(13.5), TEXT_FAINT);
            return;
        };
        let Some(info) = self.archivos.minis.get(&r).map(|m| m.info.clone()) else {
            ui.painter().text(todo.center(), Align2::CENTER_CENTER, t("Leyendo…"), fuente(13.5), TEXT_FAINT);
            if !self.archivos.pedidas.contains(&r) {
                self.archivos.pedir_mini(&Entrada { tipo: Tipo::de(&r), nombre: nombre_de(&r), tam: 0, fecha: std::time::SystemTime::UNIX_EPOCH, ruta: r });
            }
            return;
        };
        let Some(info) = info else {
            ui.painter().text(todo.center(), Align2::CENTER_CENTER, t("No pude leer ese archivo"), fuente(13.5), TEXT_FAINT);
            return;
        };
        if self.archivos.vista.as_ref().is_none_or(|v| v.ruta != r) {
            if self.archivos.vista.as_ref().is_some_and(|v| v.tocando) {
                self.parar_origen();
            }
            let mut p = Proyecto::nuevo();
            if let Some((w, h, fps)) = info.video {
                (p.ancho, p.alto, p.fps) = (w, h, fps);
                let con = if es_imagen(&r) { Contenido::Imagen } else { Contenido::Archivo };
                let c = p.nuevo_video(r.clone(), con, 0.0, info.dur.max(5.0));
                p.video.push(c);
            }
            self.archivos.vista = Some(Vista { ruta: r.clone(), info: info.clone(), t: 0.0, paseo: None, tocando: false, espera: false, audio: None, p: Arc::new(p), vista: VistaPrevia::default(), tex: None });
        }
        // Arranca cuando el sonido ya está.
        let listo = self.archivos.vista.as_ref().filter(|v| v.espera).and_then(|v| v.audio.clone().map(|a| (a, v.t)));
        if let Some(((a, d), t0)) = listo {
            let mut m = self.motor.lock().unwrap();
            m.tocando = false;
            m.origen = Some((a, ((t0 - d) * SR as f64) as i64));
            let v = self.archivos.vista.as_mut().unwrap();
            (v.tocando, v.espera) = (true, false);
        }
        let pos = self.motor.lock().unwrap().origen.as_ref().map(|o| o.1 as f64 / SR as f64);
        let imagen = es_imagen(&r);
        let mut acabo = false;
        let v = self.archivos.vista.as_mut().unwrap();
        if v.tocando {
            let d = v.audio.as_ref().map_or(0.0, |a| a.1);
            v.t = pos.map_or(v.t, |p| p + d).min(info.dur);
            acabo = v.t >= info.dur - 0.01 || pos.is_none() || (info.audio && v.t - d > OIR);
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
        let pie = 74.0;
        let img = Rect::from_min_max(todo.min + vec2(0.0, 4.0), pos2(todo.right(), todo.bottom() - pie));
        ui.painter().rect_filled(img, 5, Color32::BLACK);
        let sobre = ui.interact(img, Id::new("archivos-paseo"), Sense::click());
        sobre.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, t("Vista previa del archivo")));
        // Pasear el ratón por la imagen recorre el video.
        v.paseo = sobre.hover_pos().filter(|_| !v.tocando && !imagen && info.video.is_some()).map(|m| ((m.x - img.left()) / img.width()).clamp(0.0, 1.0) as f64 * info.dur);
        let tv = v.paseo.unwrap_or(v.t);
        if info.video.is_some() {
            let h = 240.min(v.p.alto.max(2) as usize) / 2 * 2;
            let w = (h as f64 * v.p.ancho as f64 / v.p.alto.max(1) as f64 / 2.0).round() as usize * 2;
            if let Some(q) = v.vista.actualizar(&v.p, 1, tv.min((info.dur - 0.05).max(0.0)), v.tocando, (w.max(2), h.max(2))) {
                let i = ColorImage::from_rgba_unmultiplied([q.ancho, q.alto], &q.rgba);
                match &mut v.tex {
                    Some(x) => x.set(i, TextureOptions::LINEAR),
                    None => v.tex = Some(ui.ctx().load_texture("archivos-vista", i, TextureOptions::LINEAR)),
                }
            }
            if v.vista.pendientes > 0 {
                ui.ctx().request_repaint_after(Duration::from_millis(12));
            }
            if let Some(x) = v.tex.as_ref().or(self.archivos.minis.get(&r).and_then(|m| m.tex.as_ref())) {
                ui.painter().image(x.id(), cabe(img, v.p.ancho as f32, v.p.alto as f32), TODA, Color32::WHITE);
            }
        } else if let Some(m) = self.archivos.minis.get(&r) {
            pintar_onda(&ui.painter_at(img), img.shrink(6.0), &m.onda, DEEP);
        }
        let mut tocar = sobre.clicked() && !imagen && !acabo;
        let mut poner = false;
        let mut saltar = None;
        // Barra de tiempo
        let barra = Rect::from_min_size(pos2(todo.left(), img.bottom() + 5.0), vec2(todo.width(), 12.0));
        if !imagen {
            let x_de = |s: f64| barra.left() + (s / info.dur.max(0.01)) as f32 * barra.width();
            ui.painter().rect_filled(barra, 4, WELL);
            ui.painter().rect_filled(Rect::from_min_max(barra.min, pos2(x_de(tv), barra.bottom())), 4, MAIN.gamma_multiply(0.45));
            ui.painter().vline(x_de(tv), barra.y_range(), Stroke::new(2.0, MAIN));
            let rb = ui.interact(barra, Id::new("archivos-barra"), Sense::click_and_drag());
            if let (true, Some(m)) = (rb.dragged() || rb.clicked(), rb.interact_pointer_pos()) {
                saltar = Some(((m.x - barra.left()) / barra.width()).clamp(0.0, 1.0) as f64 * info.dur);
            }
        }
        let abajo = Rect::from_min_max(pos2(todo.left(), barra.bottom() + 4.0), todo.max);
        let mut u = ui.new_child(UiBuilder::new().max_rect(abajo));
        u.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            if !imagen && boton_color(ui, if v.tocando || v.espera { &tabler::PLAYER_STOP } else { &tabler::PLAYER_PLAY }, v.tocando || v.espera, "Ver y oír este archivo", GLYPH, 24.0).clicked() {
                tocar = true;
            }
            if ui.button(RichText::new(t("Poner en el cursor")).color(MAIN).size(13.5)).clicked() {
                poner = true;
            }
            if !imagen {
                ui.label(RichText::new(tiempo(tv)).color(TEXT_DIM).size(13.0));
            }
        });
        let mut datos = vec![];
        if let Some((w, h, fps)) = info.video {
            datos.push(format!("{w}×{h}"));
            if !imagen {
                datos.push(tf("{} cps", &[&format!("{fps:.2}").trim_end_matches('0').trim_end_matches('.')]));
            }
        }
        if !imagen {
            datos.push(tiempo(info.dur));
        }
        if info.video.is_some() && !imagen && !info.audio {
            datos.push(t("sin sonido").into());
        }
        u.add(Label::new(RichText::new(datos.join(" · ")).color(TEXT_FAINT).size(12.5)).truncate());
        if let Some(s) = saltar {
            v.t = s;
            // Tocando, se sigue desde ahí si ese sonido ya se leyó; si no, se vuelve a leer.
            let dentro = v.audio.as_ref().filter(|(a, d)| s >= *d && s < d + a.dur().max(0.1)).map(|a| a.1);
            match (v.tocando, dentro) {
                (true, Some(d)) => {
                    if let Some(x) = &mut self.motor.lock().unwrap().origen {
                        x.1 = ((s - d) * SR as f64) as i64;
                    }
                }
                (true, None) => {
                    self.parar_origen();
                    self.tocar_vista();
                }
                _ => {}
            }
        }
        if acabo {
            self.parar_origen();
        }
        if tocar {
            self.tocar_vista();
        }
        if poner {
            let t = self.ahora();
            let v = self.archivos.elegidos.clone();
            self.poner_archivos(&v, t, None, None);
        }
    }

    // ---- a las pistas

    /// Pone archivos uno tras otro desde `t0`. Sin `capa`, la imagen va a la primera pista libre; con `pista`, el sonido
    /// va a esa (y de un video, solo su sonido).
    pub fn poner_archivos(&mut self, rutas: &[PathBuf], t0: f64, capa: Option<usize>, pista: Option<u64>) {
        let mut t0 = t0;
        for r in rutas {
            match Tipo::de(r) {
                Tipo::Proyecto if self.sin_guardar() => self.avisar(t("Guarda tu proyecto antes de abrir otro"), false),
                Tipo::Proyecto => return self.abrir(r.clone()),
                Tipo::Midi => self.importar_midi(r),
                Tipo::Subtitulos => match std::fs::read_to_string(r) {
                    Ok(s) => self.poner_subtitulos(&s),
                    Err(_) => self.avisar(t("No pude leer los subtítulos"), true),
                },
                Tipo::Carpeta | Tipo::Otro => self.avisar(t("Ese archivo no se puede poner en las pistas"), false),
                // Un video solo (no a una pista de sonido): se revisa en segundo plano y pregunta por el audio.
                Tipo::Video if rutas.len() == 1 && pista.is_none() => self.importar_en(r.clone(), Some(Lugar { t: t0, capa, pista })),
                _ => {
                    let Ok(info) = medios::sondear(r) else {
                        self.avisar(t("No pude leer ese archivo"), true);
                        continue;
                    };
                    t0 += self.poner_ya(r, &info, Lugar { t: t0, capa, pista }, AudioVideo::Junto);
                }
            }
        }
    }

    /// Pone un archivo ya leído donde se soltó. Con `Separado`, el video va solo y su sonido se separa en voz e
    /// instrumental. Da cuánto dura lo que se puso.
    pub fn poner_ya(&mut self, r: &Path, info: &Info, l: Lugar, modo: AudioVideo) -> f64 {
        let (t0, capa, pista) = (l.t, l.capa, l.pista);
        let dur = if es_imagen(r) { 5.0 } else { info.dur };
        if let Some(p) = pista {
            self.canal = Canal::Pista(p);
        }
        let solo_sonido = pista.is_some() && info.audio;
        let capa = match capa {
            Some(c) => c,
            None => {
                let ocupada = self.p.video.iter().any(|c| c.capa == 0 && c.inicio < t0 + dur && c.fin() > t0);
                if info.video.is_some() && !solo_sonido && ocupada { self.capa_libre(t0, dur) } else { 0 }
            }
        };
        let separar = modo == AudioVideo::Separado && info.video.is_some() && info.audio && !solo_sonido;
        // El sonido, a una pista con ese tramo libre: así no tapa el que ya está.
        if info.audio && pista.is_none() && !separar {
            let libre = |p: &Pista| p.instrumento.is_none() && !p.bloqueada && !p.armada && !p.clips.iter().any(|c| c.inicio < t0 + dur && c.inicio + c.dur > t0);
            let id = self.p.pistas.iter().find(|p| libre(p)).map(|p| p.id).unwrap_or_else(|| self.p.nueva_pista("Audio"));
            self.canal = Canal::Pista(id);
        }
        let antes = self.p.video.is_empty();
        let sin_audio = Info { audio: info.audio && !separar, ..info.clone() };
        self.poner_medio_info(r, &sin_audio, t0, if solo_sonido { None } else { Some(capa) }, None, false);
        if separar {
            let con: Vec<u64> = self.clip.into_iter().collect();
            self.separar(r.to_path_buf(), t0, None, con);
        }
        if antes {
            self.linea.encuadrar = true;
        }
        dur
    }
}

/// Fecha de un archivo para la lista: «hoy», «ayer» o día/mes/año.
fn fecha_corta(f: std::time::SystemTime) -> String {
    let dias = |x: std::time::SystemTime| x.duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() / 86_400) as i64;
    let (d, hoy) = (dias(f), dias(std::time::SystemTime::now()));
    if d <= 0 {
        return String::new();
    }
    match hoy - d {
        0 => return t("hoy").into(),
        1 => return t("ayer").into(),
        _ => {}
    }
    // Días desde 1970 a año, mes y día (calendario civil).
    let z = d + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dia = doy - (153 * mp + 2) / 5 + 1;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 };
    let ano = yoe + era * 400 + i64::from(mes <= 2);
    format!("{dia:02}/{mes:02}/{ano}")
}

/// Abre el explorador del sistema con el archivo elegido.
pub fn mostrar(r: &Path) {
    let _ = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg("-R").arg(r).spawn()
    } else if cfg!(windows) {
        std::process::Command::new("explorer").arg(format!("/select,{}", r.display())).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(r.parent().unwrap_or(r)).spawn()
    };
}
