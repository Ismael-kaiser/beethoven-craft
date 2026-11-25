//! Menú principal: lo primero que se ve al abrir la app. Proyectos recientes, nuevo video, nueva canción y abrir.
use super::estilo::*;
use super::{App, Confirmar, Espacio, biblioteca, carpeta, nombre_de, tabler};
use crate::explorador::{llano, natural, peso};
use crate::idioma::{self, Idioma, t, tf};
use crate::proyectos::{self, Borrado, Ficha, Libro, Tipo};
use eframe::egui::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// Tamaño de la ventana del menú y del editor.
pub const MENU: Vec2 = vec2(1180.0, 740.0);
pub const EDITOR: Vec2 = vec2(1440.0, 860.0);
/// Lo mínimo que se ve la pantalla de carga.
const CARGA_MIN: Duration = Duration::from_millis(300);
/// Lo más que se espera a que terminen de leerse los medios: lo que falte sigue en segundo plano, ya en el editor.
const CARGA_MAX: Duration = Duration::from_millis(1500);
/// Color de Beethoven DAW.
const VIOLETA: Color32 = c(0x8f86ff);

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Vista {
    #[default]
    Inicio,
    Papelera,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum Orden {
    #[default]
    Nuevos,
    Nombre,
    Peso,
}

#[derive(Default)]
pub struct Estado {
    pub visible: bool,
    /// Se lee al mostrar el menú; `None` = volver a leer.
    pub lista: Option<Vec<Ficha>>,
    pub borrados: Vec<Borrado>,
    /// Miniaturas ya leídas (`None`: no tiene).
    minis: HashMap<PathBuf, Option<TextureHandle>>,
    pub vista: Vista,
    pub buscar: String,
    filtro: Option<Tipo>,
    orden: Orden,
    renglones: bool,
    pub elegidos: HashSet<PathBuf>,
    /// Proyecto al que se le está cambiando el nombre, y el nombre que lleva.
    pub renombrando: Option<(PathBuf, String)>,
    /// Los que se van a mandar a la papelera, esperando el sí.
    pub por_eliminar: Vec<PathBuf>,
    /// Lo que se borraría para siempre, esperando el sí.
    por_borrar: Vec<Borrado>,
    /// Dónde están los recientes y la papelera; `None`: la carpeta de la app.
    pub base: Option<PathBuf>,
    /// Tamaño con que empieza el video nuevo.
    pub formato: Option<(u32, u32)>,
    /// Reciente que se abre después de confirmar que se pierden los cambios.
    pub pendiente: Option<PathBuf>,
    /// Pantalla de carga: desde cuándo se ve.
    pub cargando: Option<Instant>,
    /// La carga es de Beethoven DAW: al terminar se cierra esta ventana.
    pub daw: bool,
    /// Tamaño que tiene que tomar la ventana.
    tam: Option<Vec2>,
    /// La ventana de configuración abierta.
    pub config: bool,
}

impl Estado {
    /// Cierra lo que haya y muestra el menú en su ventana chica.
    pub fn mostrar(&mut self) {
        self.visible = true;
        self.lista = None;
        self.minis.clear();
        self.elegidos.clear();
        self.vista = Vista::Inicio;
        self.cargando = None;
        self.daw = false;
        self.tam = Some(MENU);
    }
    /// Del menú a la pantalla de carga (se llama al poner un proyecto).
    pub fn cargar(&mut self) {
        if self.visible {
            self.cargando = Some(Instant::now());
        }
    }
    /// Cierra el menú y abre el editor.
    pub fn entrar(&mut self) {
        self.visible = false;
        self.cargando = None;
        self.tam = Some(EDITOR);
    }
    /// Cambia el tamaño de la ventana si hace falta y la centra.
    pub fn ajustar_ventana(&mut self, ctx: &Context) {
        let Some(t) = self.tam.take() else { return };
        let menu = t == MENU;
        ctx.send_viewport_cmd(ViewportCommand::Title(if menu { "Beethoven Craft" } else { "Beethoven Movie" }.into()));
        ctx.send_viewport_cmd(ViewportCommand::Resizable(true));
        ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(if menu { vec2(900.0, 620.0) } else { vec2(960.0, 600.0) }));
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(t));
        if let Some(m) = ctx.input(|i| i.viewport().monitor_size) {
            let t = vec2(t.x.min(m.x), t.y.min(m.y));
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(((m - t) / 2.0).to_pos2()));
        }
    }
}

/// El programa de Beethoven DAW: junto a este (dentro del .app) o, al probar desde el código, en `daw/target/release`.
pub fn programa_daw() -> Option<PathBuf> {
    let exe = std::env::consts::EXE_SUFFIX;
    let junto = std::env::current_exe().ok()?.with_file_name(format!("beethoven-daw{exe}"));
    let codigo = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("daw/target/release/beethoven_daw{exe}"));
    [junto, codigo].into_iter().find(|r| r.is_file())
}

fn archivo_ajustes() -> PathBuf {
    carpeta().join("Ajustes.json")
}

/// Lee los ajustes guardados (idioma y atajos de teclado) y los pone en marcha.
pub fn cargar_ajustes() {
    let v: serde_json::Value = std::fs::read(archivo_ajustes()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    if let Some(i) = v["idioma"].as_str().and_then(Idioma::de_codigo) {
        idioma::poner(i);
    }
    crate::atajos::de_json(&v["atajos"]);
}

pub fn guardar_ajustes() {
    let _ = std::fs::create_dir_all(carpeta());
    let v = serde_json::json!({ "idioma": idioma::actual().codigo(), "atajos": crate::atajos::a_json() });
    if let Ok(b) = serde_json::to_vec_pretty(&v) {
        let _ = std::fs::write(archivo_ajustes(), b);
    }
}

/// Pone `ruta` al principio de los recientes.
pub fn anotar(ruta: &Path) {
    Libro { base: carpeta() }.anotar(ruta);
}

fn t_(s: &str) -> String {
    t(s).to_string()
}

fn hace(t: SystemTime) -> String {
    let d = SystemTime::now().duration_since(t).map(|d| d.as_secs() / 86_400).unwrap_or(0);
    match d {
        0 => t_("Hoy"),
        1 => t_("Ayer"),
        2..=59 => crate::idioma::tf("Hace {} días", &[&d]),
        _ => crate::idioma::tf("Hace {} meses", &[&(d / 30)]),
    }
}

/// Lo que se puede hacer con un proyecto desde el menú.
enum Hacer {
    Abrir(PathBuf),
    Renombrar(PathBuf),
    Duplicar(PathBuf),
    Mostrar(PathBuf),
    Quitar(Vec<PathBuf>),
    Eliminar(Vec<PathBuf>),
    Elegir(PathBuf),
    Restaurar(Borrado),
    Borrar(Vec<Borrado>),
}

/// Renglón de la barra de la izquierda.
fn nav(ui: &mut Ui, i: &tabler::TablerIcon, n: &str, on: bool, cuenta: usize) -> Response {
    let n = t(n);
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, n));
    let p = ui.painter();
    if on || resp.hovered() {
        p.rect_filled(r, 8, if on { BUTTON } else { SUNKEN });
    }
    if on {
        p.rect_filled(Rect::from_min_size(r.min + vec2(0.0, 9.0), vec2(3.0, 20.0)), 2, MAIN);
    }
    icono(p, i, Rect::from_center_size(pos2(r.left() + 22.0, r.center().y), Vec2::splat(18.0)), if on { MAIN } else { TEXT_DIM });
    p.text(pos2(r.left() + 42.0, r.center().y), Align2::LEFT_CENTER, n, fuente(15.5), if on { TEXT } else { TEXT_DIM });
    if cuenta > 0 {
        let g = p.layout_no_wrap(cuenta.to_string(), fuente(12.5), TEXT_DIM);
        let caja = Rect::from_center_size(pos2(r.right() - 20.0, r.center().y), vec2((g.size().x + 12.0).max(22.0), 18.0));
        p.rect_filled(caja, 9, WORKSPACE);
        p.galley(caja.center() - g.size() / 2.0, g, TEXT_DIM);
    }
    resp
}

/// Tarjeta grande de «crear». El dibujo del fondo lo pone `arte`.
fn crear(ui: &mut Ui, r: Rect, titulo: &str, sub: &str, marca: &str, col: Color32, i: &tabler::TablerIcon, arte: impl FnOnce(&Painter, Rect)) -> Response {
    let titulo = t(titulo);
    let activo = ui.is_enabled();
    let resp = ui.interact(r, Id::new(("crear", marca)), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, activo, titulo));
    let k = ui.ctx().animate_bool(resp.id, resp.hovered() && activo);
    let p = ui.painter_at(r.expand(2.0));
    let fondo = mezcla(WINDOW_TITLE, col, 0.10 + 0.07 * k);
    p.rect_filled(r, 14, fondo);
    arte(&ui.painter_at(r), r);
    p.rect_stroke(r, 14, Stroke::new(1.0 + 0.8 * k, mezcla(BUTTON_HI, col, 0.25 + 0.6 * k)), StrokeKind::Inside);
    let ic = Rect::from_min_size(r.min + vec2(20.0, 20.0), Vec2::splat(44.0));
    p.rect_filled(ic, 12, if activo { col } else { BUTTON });
    icono(&p, i, ic.shrink(10.0), if activo { c(0x14141a) } else { TEXT_FAINT });
    p.text(pos2(ic.right() + 14.0, ic.center().y - 10.0), Align2::LEFT_CENTER, marca, fuente(13.0), mezcla(TEXT_DIM, col, 0.6));
    p.text(pos2(ic.right() + 14.0, ic.center().y + 11.0), Align2::LEFT_CENTER, titulo, barlow(22.0), if activo { TEXT } else { TEXT_FAINT });
    p.text(pos2(ic.left(), ic.bottom() + 16.0), Align2::LEFT_CENTER, t(sub), fuente(14.0), TEXT_DIM);
    resp
}

/// Una línea de tiempo de adorno.
fn arte_video(p: &Painter, r: Rect) {
    let x0 = r.left() + r.width() * 0.52;
    for (k, (a, b, f, col)) in [(0.02, 0.30, 0.30, MAIN), (0.34, 0.62, 0.30, DEEP), (0.66, 1.05, 0.30, MAIN), (0.10, 0.48, 0.46, c(0x5fb3ff)), (0.55, 0.95, 0.46, c(0x6fdc8c)), (0.02, 0.80, 0.62, c(0xff8fb1))].into_iter().enumerate() {
        let w = r.right() - x0;
        let y = r.top() + r.height() * f;
        let q = Rect::from_min_max(pos2(x0 + w * a, y), pos2(x0 + w * b, y + r.height() * 0.12));
        p.rect_filled(q, 4, col.gamma_multiply(0.16 + 0.02 * (k % 3) as f32));
    }
    p.vline(x0 + (r.right() - x0) * 0.44, (r.top() + r.height() * 0.22)..=(r.top() + r.height() * 0.80), Stroke::new(1.5, HIGHLIGHT.gamma_multiply(0.5)));
}

/// Unas teclas y una onda de adorno.
fn arte_musica(p: &Painter, r: Rect) {
    let n = 46;
    for i in 0..n {
        let x = r.left() + r.width() * (0.56 + 0.44 * i as f32 / n as f32);
        let k = i as f32;
        let h = r.height() * (0.05 + 0.15 * ((k * 0.7).sin() * (k * 0.23 + 1.0).cos()).abs());
        p.vline(x, (r.top() + r.height() * 0.36 - h)..=(r.top() + r.height() * 0.36 + h), Stroke::new(2.4, VIOLETA.gamma_multiply(0.28)));
    }
    let (w, y0) = (17.0, r.bottom() - r.height() * 0.30);
    let mut x = r.right() - 4.0;
    let mut k = 0;
    while x > r.left() + r.width() * 0.56 {
        p.rect_filled(Rect::from_min_max(pos2(x - w + 1.5, y0), pos2(x, r.bottom() + 8.0)), 3, Color32::from_white_alpha(26));
        if ![2, 6].contains(&(k % 7)) {
            p.rect_filled(Rect::from_min_max(pos2(x - w - 5.0, y0), pos2(x - w + 6.0, y0 + r.height() * 0.17)), 2, c(0x14141a));
        }
        x -= w;
        k += 1;
    }
}

impl App {
    /// Configuración: el idioma de Beethoven Movie y de Beethoven DAW.
    fn ventana_config(&mut self, ctx: &Context) {
        let mut cerrar = false;
        Modal::new(Id::new("config")).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.heading(t("Configuración"));
            ui.add_space(10.0);
            ui.label(RichText::new(t("Idioma")).color(TEXT_DIM));
            for i in Idioma::TODOS {
                if ui.radio(idioma::actual() == i, i.nombre()).clicked() && idioma::actual() != i {
                    idioma::poner(i);
                    if !self.sin_dispositivo {
                        guardar_ajustes();
                    }
                }
            }
            ui.add_space(4.0);
            ui.label(RichText::new(t("Vale para Beethoven Movie y Beethoven DAW.")).size(12.5).color(TEXT_FAINT));
            ui.add_space(10.0);
            ui.label(RichText::new(t("Teclado")).color(TEXT_DIM));
            if ui.button(t("Atajos de teclado…")).clicked() {
                self.atajos.abierto = true;
                cerrar = true;
            }
            ui.add_space(10.0);
            ui.label(RichText::new(t("Actualizaciones")).color(TEXT_DIM));
            let mut si = crate::actualizar::buscar_al_abrir(&carpeta());
            if ui.checkbox(&mut si, t("Buscar versiones nuevas al abrir")).changed() && !self.sin_dispositivo {
                crate::actualizar::poner_buscar_al_abrir(&carpeta(), si);
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new(tf("Tienes la {}", &[&crate::actualizar::version()])).size(12.5).color(TEXT_FAINT));
                if ui.small_button(t("Buscar ahora")).clicked() {
                    self.buscar_version(true);
                }
            });
            ui.add_space(10.0);
            if ui.button(t("Listo")).clicked() {
                cerrar = true;
            }
        });
        if cerrar {
            self.inicio.config = false;
        }
    }

    /// Arranca Beethoven DAW en su ventana, con una canción o vacío; el menú muestra la carga y después se cierra.
    fn abrir_daw(&mut self, programa: PathBuf, cancion: Option<&Path>) {
        match std::process::Command::new(&programa).args(cancion).env("BEETHOVEN_IDIOMA", idioma::actual().codigo()).spawn() {
            Ok(_) => {
                self.inicio.daw = true;
                self.inicio.cargando = Some(Instant::now());
            }
            Err(e) => self.avisar(crate::idioma::tf("No pude abrir Beethoven DAW: {}", &[&e]), true),
        }
    }

    /// Mientras se prepara el proyecto, entre el menú y el editor.
    fn pantalla_carga(&mut self, ui: &mut Ui) {
        let texto = match &self.ruta {
            _ if self.inicio.daw => crate::idioma::t("Abriendo Beethoven DAW…").into(),
            Some(r) => crate::idioma::tf("Abriendo «{}»…", &[&(nombre_de(r))]),
            None => crate::idioma::t("Preparando Beethoven Movie…").into(),
        };
        CentralPanel::default().frame(Frame::new().fill(WORKSPACE)).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space((ui.available_height() - 300.0).max(0.0) / 2.0);
                let (r, _) = ui.allocate_exact_size(Vec2::splat(128.0), Sense::hover());
                logo(ui, r);
                ui.add_space(18.0);
                ui.label(RichText::new(crate::idioma::t("Beethoven Craft")).font(barlow(34.0)).color(TEXT));
                ui.add_space(22.0);
                ui.add(Spinner::new().size(26.0).color(MAIN));
                ui.add_space(10.0);
                ui.label(RichText::new(crate::idioma::t(&texto)).size(15.0).color(TEXT_DIM));
            });
        });
    }

    /// Dibuja el menú principal en toda la ventana.
    pub fn pantalla_inicio(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if let Some(desde) = self.inicio.cargando {
            self.pantalla_carga(ui);
            if desde.elapsed() >= CARGA_MIN && self.inicio.daw {
                // Beethoven DAW ya arrancó en su ventana: el menú se cierra.
                self.inicio.cargando = None;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            } else if desde.elapsed() >= CARGA_MIN && (!self.ocupada() || desde.elapsed() >= CARGA_MAX) {
                self.inicio.entrar();
            }
            ctx.request_repaint_after(Duration::from_millis(16));
            return;
        }
        if self.inicio.lista.is_none() {
            self.leer_proyectos();
        }
        let mut hacer = vec![];
        Panel::left("inicio-menu").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(Margin::symmetric(16, 22))).resizable(false).exact_size(232.0).show(ui, |ui| self.barra_inicio(ui));
        CentralPanel::default().frame(Frame::new().fill(WORKSPACE).inner_margin(Margin { left: 30, right: 18, top: 24, bottom: 14 })).show(ui, |ui| match self.inicio.vista {
            Vista::Inicio => self.vista_inicio(ui, &mut hacer),
            Vista::Papelera => self.vista_papelera(ui, &mut hacer),
        });
        for h in hacer {
            self.hacer_inicio(h);
        }
        self.dialogos_inicio(&ctx);
        if self.inicio.config {
            self.ventana_config(&ctx);
        }
    }

    fn libro(&self) -> Libro {
        Libro { base: self.inicio.base.clone().unwrap_or_else(carpeta) }
    }

    fn leer_proyectos(&mut self) {
        // Las pruebas solo ven la carpeta que ponen ellas.
        if self.sin_dispositivo && self.inicio.base.is_none() {
            self.inicio.lista = Some(vec![]);
            return;
        }
        let l = self.libro();
        crate::tiempos::marca("leer proyectos: empieza");
        let canciones = if self.sin_dispositivo { vec![] } else { proyectos::canciones_del_daw() };
        crate::tiempos::marca("leer proyectos: canciones del DAW");
        self.inicio.lista = Some(l.listar(&canciones));
        crate::tiempos::marca("leer proyectos: lista");
        self.inicio.borrados = l.papelera();
        crate::tiempos::marca("leer proyectos: papelera");
        let vivos: HashSet<PathBuf> = self.inicio.lista.iter().flatten().map(|f| f.ruta.clone()).collect();
        self.inicio.elegidos.retain(|r| vivos.contains(r));
    }

    fn barra_inicio(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(56.0), Sense::hover());
            logo(ui, r);
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.add_space(6.0);
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.label(RichText::new("Beethoven").font(barlow(21.0)).color(TEXT));
                ui.label(RichText::new("Craft").font(barlow(21.0)).color(MAIN));
                // La versión, en una pastillita hundida; clic abre «Acerca de».
                ui.add_space(4.0);
                let txt = format!("v{}", crate::actualizar::version());
                let g = ui.painter().layout_no_wrap(txt.clone(), barlow(12.0), TEXT_DIM);
                let (r, resp) = ui.allocate_exact_size(g.size() + vec2(12.0, 4.0), Sense::click());
                resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tf("Versión {}", &[&crate::actualizar::version()])));
                hueco(ui.painter(), r, 6, if resp.hovered() { c(0x2a2a2a) } else { c(0x161616) });
                ui.painter().galley(r.center() - g.size() / 2.0, g, if resp.hovered() { MAIN } else { TEXT_DIM });
                if resp.on_hover_text(t("Acerca de Beethoven Craft")).clicked() {
                    self.acerca = true;
                }
            });
        });
        ui.add_space(24.0);
        ui.spacing_mut().item_spacing.y = 4.0;
        let n = self.inicio.lista.as_ref().map_or(0, Vec::len);
        if nav(ui, &tabler::FOLDERS, "Proyectos", self.inicio.vista == Vista::Inicio, n).clicked() {
            self.inicio.vista = Vista::Inicio;
        }
        if nav(ui, &tabler::TRASH, "Papelera", self.inicio.vista == Vista::Papelera, self.inicio.borrados.len()).clicked() {
            self.inicio.vista = Vista::Papelera;
        }
        ui.add_space(10.0);
        if nav(ui, &tabler::FOLDER_OPEN, "Abrir proyecto…", false, 0).clicked() {
            self.inicio.pendiente = None;
            self.pedir(Confirmar::Abrir);
        }
        if nav(ui, &tabler::KEYBOARD, "Atajos de teclado", false, 0).clicked() {
            self.atajos.abierto = true;
        }
        if nav(ui, &tabler::SETTINGS, "Configuración", false, 0).clicked() {
            self.inicio.config = true;
        }
        if self.nueva.is_some() {
            ui.add_space(10.0);
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
            self.insignia_nueva(ui, r);
        }
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            let r = ui.add(Label::new(RichText::new(t("Creado por mixter-ha99")).size(12.5).color(TEXT_FAINT)).sense(Sense::click()));
            if r.on_hover_text(t("Acerca de Beethoven Craft")).clicked() {
                self.acerca = true;
            }
            if self.ruta.is_some() || self.sin_guardar() {
                ui.add_space(8.0);
                let n = self.ruta.as_deref().map_or(t("Proyecto sin nombre").into(), nombre_de);
                let b = Button::new(RichText::new(tf("← Volver a «{}»", &[&n])).size(14.0).color(MAIN)).truncate().fill(BUTTON).min_size(vec2(ui.available_width(), 36.0));
                if ui.add(b).clicked() {
                    self.inicio.entrar();
                }
            }
        });
    }

    fn vista_inicio(&mut self, ui: &mut Ui, hacer: &mut Vec<Hacer>) {
        ui.label(RichText::new(t("¿Qué vas a crear?")).font(barlow(26.0)).color(TEXT));
        ui.add_space(4.0);
        self.mensaje_ui(ui);
        ui.add_space(8.0);
        // Tarjetas grandes
        let ancho = ui.available_width() - 12.0;
        let (fila, _) = ui.allocate_exact_size(vec2(ancho, 156.0), Sense::hover());
        let w = (ancho - 16.0) / 2.0;
        let izq = Rect::from_min_size(fila.min, vec2(w, fila.height()));
        let der = izq.translate(vec2(w + 16.0, 0.0));
        let nuevo = crear(ui, izq, "Nuevo video", "Edita, anima y mezcla tu video", "Beethoven Movie", MAIN, &tabler::MOVIE, arte_video);
        // Formatos a un clic, dentro de la tarjeta
        let mut pie = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(pos2(izq.left() + 20.0, izq.bottom() - 42.0), izq.max - vec2(12.0, 10.0))).layout(Layout::left_to_right(Align::Center)));
        pie.spacing_mut().item_spacing.x = 6.0;
        let mut formato = None;
        for (n, w, h) in [("16:9", 1920, 1080), ("9:16", 1080, 1920), ("1:1", 1080, 1080), ("4:5", 1080, 1350)] {
            if biblioteca::pastilla(&mut pie, n, false).on_hover_text(tf("Video nuevo de {}×{}", &[&w, &h])).clicked() {
                formato = Some((w, h));
            }
        }
        if nuevo.clicked() || formato.is_some() {
            self.espacio = Espacio::Video;
            self.inicio.formato = formato;
            self.pedir(Confirmar::Nuevo);
        }
        let daw = programa_daw();
        let mut u = ui.new_child(UiBuilder::new().max_rect(der));
        if daw.is_none() {
            u.disable();
        }
        let r = crear(&mut u, der, "Nueva canción", "Compón, escribe partituras y mezcla", "Beethoven DAW", VIOLETA, &tabler::PIANO, arte_musica)
            .on_disabled_hover_text(t("No encuentro Beethoven DAW. Hay que armarlo con empaque/empacar.sh."));
        if r.clicked() {
            if let Some(d) = daw {
                self.abrir_daw(d, None);
            }
        }
        ui.add_space(22.0);
        // Herramientas de la lista
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(RichText::new(t("Tus proyectos")).font(barlow(21.0)).color(TEXT));
            ui.add_space(8.0);
            for (f, n) in [(None, "Todos"), (Some(Tipo::Video), "Videos"), (Some(Tipo::Cancion), "Canciones")] {
                if biblioteca::pastilla(ui, n, self.inicio.filtro == f).clicked() {
                    self.inicio.filtro = f;
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(12.0);
                let ren = self.inicio.renglones;
                if boton_color(ui, if ren { &tabler::GRID_DOTS } else { &tabler::LAYOUT_ROWS }, false, if ren { "Ver como tarjetas" } else { "Ver como lista" }, GLYPH, 28.0).clicked() {
                    self.inicio.renglones = !ren;
                }
                let o = [(Orden::Nuevos, "Más nuevo"), (Orden::Nombre, "Nombre"), (Orden::Peso, "Más pesado")];
                ComboBox::from_id_salt("orden-proyectos").width(104.0).selected_text(t(o.iter().find(|x| x.0 == self.inicio.orden).unwrap().1)).show_ui(ui, |ui| {
                    for (x, n) in o {
                        ui.selectable_value(&mut self.inicio.orden, x, t(n));
                    }
                });
                ui.add(TextEdit::singleline(&mut self.inicio.buscar).hint_text(t("Buscar proyecto…")).desired_width(170.0).margin(Margin::symmetric(8, 5)));
            });
        });
        ui.add_space(8.0);
        // Los que se ven
        let q = llano(&self.inicio.buscar);
        let todos = self.inicio.lista.as_ref().map_or(0, Vec::len);
        let mut v: Vec<Ficha> = self.inicio.lista.iter().flatten().filter(|f| self.inicio.filtro.is_none_or(|x| x == f.tipo) && llano(&f.nombre).contains(&q)).cloned().collect();
        match self.inicio.orden {
            Orden::Nuevos => {}
            Orden::Nombre => v.sort_by(|a, b| natural(&a.nombre, &b.nombre)),
            Orden::Peso => v.sort_by_key(|f| std::cmp::Reverse(f.tam)),
        }
        // Varios elegidos
        if !self.inicio.elegidos.is_empty() {
            let sel: Vec<PathBuf> = v.iter().map(|f| f.ruta.clone()).filter(|r| self.inicio.elegidos.contains(r)).collect();
            Frame::new().fill(mezcla(WINDOW_TITLE, MAIN, 0.10)).corner_radius(9).inner_margin(Margin::symmetric(12, 6)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tf("{} elegidos", &[&sel.len()])).color(TEXT));
                    if ui.button(RichText::new(t("Eliminar")).color(RECORD)).clicked() {
                        hacer.push(Hacer::Eliminar(sel.clone()));
                    }
                    if ui.button(t("Quitar de la lista")).on_hover_text(t("El archivo se queda donde está")).clicked() {
                        hacer.push(Hacer::Quitar(sel.clone()));
                    }
                    if ui.button(t("Elegir todos")).clicked() {
                        self.inicio.elegidos = v.iter().map(|f| f.ruta.clone()).collect();
                    }
                    if ui.button(t("Cancelar")).clicked() {
                        self.inicio.elegidos.clear();
                    }
                });
            });
            ui.add_space(8.0);
        }
        if v.is_empty() {
            let (r, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
            let o = r.center() - vec2(0.0, 24.0);
            icono(ui.painter(), &tabler::FOLDER_PLUS, Rect::from_center_size(o - vec2(0.0, 34.0), Vec2::splat(46.0)), TEXT_FAINT);
            let texto = if todos == 0 { "Aún no hay proyectos. Empieza uno nuevo o abre uno que ya tengas." } else { "No hay proyectos con ese nombre" };
            ui.painter().text(o + vec2(0.0, 12.0), Align2::CENTER_CENTER, t(texto), fuente(15.0), TEXT_DIM);
            return;
        }
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            if self.inicio.renglones {
                ui.spacing_mut().item_spacing.y = 3.0;
                for f in &v {
                    self.renglon_proyecto(ui, f, hacer);
                }
            } else {
                let ancho = ui.available_width() - 12.0;
                let n = (ancho / 210.0).floor().max(2.0);
                let w = (ancho - (n - 1.0) * 14.0) / n;
                ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                for fila in v.chunks(n as usize) {
                    ui.horizontal(|ui| {
                        for f in fila {
                            self.tarjeta_proyecto(ui, f, w, hacer);
                        }
                    });
                }
            }
        });
    }

    fn mini_de(&mut self, ctx: &Context, f: &Ficha) -> Option<TextureHandle> {
        self.inicio
            .minis
            .entry(f.ruta.clone())
            .or_insert_with(|| {
                if f.tipo == Tipo::Cancion {
                    return None;
                }
                let (an, al, q) = crate::archivo::miniatura(&f.ruta)?;
                Some(ctx.load_texture(format!("reciente:{}", f.ruta.display()), ColorImage::from_rgba_unmultiplied([an, al], &q), TextureOptions::LINEAR))
            })
            .clone()
    }

    /// La imagen del proyecto, o un dibujo según lo que es.
    fn pinta_mini(&mut self, ui: &Ui, f: &Ficha, r: Rect, rad: u8) {
        let tex = self.mini_de(ui.ctx(), f);
        let p = ui.painter_at(r);
        let col = if f.tipo == Tipo::Cancion { VIOLETA } else { MAIN };
        p.rect_filled(r, rad, mezcla(WELL, col, 0.12));
        match tex {
            Some(t) => {
                // Llena el recuadro sin deformarse.
                let s = t.size_vec2();
                let k = (r.width() / s.x).max(r.height() / s.y);
                let (u, v) = (r.width() / (s.x * k) / 2.0, r.height() / (s.y * k) / 2.0);
                let uv = Rect::from_min_max(pos2(0.5 - u, 0.5 - v), pos2(0.5 + u, 0.5 + v));
                Image::new(&t).uv(uv).corner_radius(rad).paint_at(ui, r);
            }
            None => {
                if f.tipo == Tipo::Cancion && r.width() > 80.0 {
                    arte_musica(&p, Rect::from_min_max(pos2(r.left() - r.width() * 0.85, r.top()), r.max));
                }
                icono(&p, if f.tipo == Tipo::Cancion { &tabler::MUSIC } else { &tabler::MOVIE }, Rect::from_center_size(r.center(), Vec2::splat((r.height() * 0.34).min(40.0))), col.gamma_multiply(0.75));
            }
        }
    }

    fn menu_proyecto(ui: &mut Ui, f: &Ficha, hacer: &mut Vec<Hacer>) {
        ui.set_min_width(190.0);
        let mut b = |ui: &mut Ui, n: &str, h: Hacer| {
            if ui.button(t(n)).clicked() {
                hacer.push(h);
                ui.close();
            }
        };
        b(ui, "Abrir", Hacer::Abrir(f.ruta.clone()));
        b(ui, "Cambiar nombre…", Hacer::Renombrar(f.ruta.clone()));
        b(ui, "Duplicar", Hacer::Duplicar(f.ruta.clone()));
        b(ui, if cfg!(target_os = "macos") { "Mostrar en el Finder" } else { "Mostrar en su carpeta" }, Hacer::Mostrar(f.ruta.clone()));
        b(ui, "Elegir varios", Hacer::Elegir(f.ruta.clone()));
        ui.separator();
        b(ui, "Quitar de la lista", Hacer::Quitar(vec![f.ruta.clone()]));
        if ui.button(RichText::new(t("Eliminar…")).color(RECORD)).clicked() {
            hacer.push(Hacer::Eliminar(vec![f.ruta.clone()]));
            ui.close();
        }
    }

    /// Clic, elegir varios, menú y «⋯», igual en tarjetas y en lista.
    fn atender_proyecto(&mut self, ui: &mut Ui, f: &Ficha, r: Rect, resp: &Response, puntos: Rect, hacer: &mut Vec<Hacer>) {
        let elegido = self.inicio.elegidos.contains(&f.ruta);
        let rp = ui.interact(puntos, Id::new(("mas-proyecto", &f.ruta)), Sense::click());
        rp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tf("Más de {}", &[&f.nombre])));
        let sobre = resp.hovered() || rp.hovered() || rp.context_menu_opened();
        if sobre {
            ui.painter().rect_filled(puntos, 6, if rp.hovered() { BUTTON_HI } else { Color32::from_black_alpha(170) });
            for k in -1..=1 {
                ui.painter().circle_filled(puntos.center() + vec2(6.0 * k as f32, 0.0), 1.9, TEXT);
            }
        }
        Popup::menu(&rp).show(|ui| Self::menu_proyecto(ui, f, hacer));
        resp.context_menu(|ui| Self::menu_proyecto(ui, f, hacer));
        if elegido || sobre {
            ui.painter().rect_stroke(r, 10, Stroke::new(if elegido { 2.0 } else { 1.2 }, if elegido { MAIN } else { BUTTON_HI }), StrokeKind::Inside);
        }
        if resp.clicked() {
            // Con ⌘, o si ya hay elegidos, el clic elige en vez de abrir.
            if ui.input(|i| i.modifiers.command) || !self.inicio.elegidos.is_empty() {
                hacer.push(Hacer::Elegir(f.ruta.clone()));
            } else {
                hacer.push(Hacer::Abrir(f.ruta.clone()));
            }
        }
    }

    fn tarjeta_proyecto(&mut self, ui: &mut Ui, f: &Ficha, w: f32, hacer: &mut Vec<Hacer>) {
        let al = (w * 9.0 / 16.0).round();
        let (r, resp) = ui.allocate_exact_size(vec2(w, al + 56.0), Sense::click());
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &f.nombre));
        if !ui.is_rect_visible(r) {
            return;
        }
        ui.painter().rect_filled(r, 10, if resp.hovered() { BUTTON } else { WINDOW_TITLE });
        let img = Rect::from_min_size(r.min + vec2(5.0, 5.0), vec2(w - 10.0, al - 5.0));
        self.pinta_mini(ui, f, img, 7);
        let (col, n) = if f.tipo == Tipo::Cancion { (VIOLETA, "Canción") } else { (MAIN, "Video") };
        let g = ui.painter().layout_no_wrap(t(n).into(), fuente(11.5), c(0x14141a));
        let caja = Rect::from_min_size(img.min + vec2(7.0, 7.0), g.size() + vec2(12.0, 4.0));
        ui.painter().rect_filled(caja, 5, col);
        ui.painter().galley(caja.min + vec2(6.0, 2.0), g, c(0x14141a));
        let p = ui.painter_at(r.shrink2(vec2(10.0, 0.0)));
        let mut job = text::LayoutJob::simple_singleline(f.nombre.clone(), fuente(15.0), TEXT);
        job.wrap = text::TextWrapping { max_width: w - 22.0, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
        p.galley(pos2(r.left() + 11.0, img.bottom() + 8.0), p.layout_job(job), TEXT);
        p.text(pos2(r.left() + 11.0, img.bottom() + 38.0), Align2::LEFT_CENTER, format!("{} · {}", hace(f.fecha), peso(f.tam)), fuente(12.5), TEXT_DIM);
        let puntos = Rect::from_min_size(pos2(img.right() - 36.0, img.top() + 6.0), vec2(30.0, 22.0));
        self.atender_proyecto(ui, f, r, &resp, puntos, hacer);
        if !resp.context_menu_opened() {
            resp.on_hover_text(f.ruta.display().to_string());
        }
    }

    fn renglon_proyecto(&mut self, ui: &mut Ui, f: &Ficha, hacer: &mut Vec<Hacer>) {
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width() - 12.0, 48.0), Sense::click());
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &f.nombre));
        if !ui.is_rect_visible(r) {
            return;
        }
        ui.painter().rect_filled(r, 9, if resp.hovered() { BUTTON } else { WINDOW_TITLE });
        let img = Rect::from_min_size(r.min + vec2(6.0, 6.0), vec2(64.0, 36.0));
        self.pinta_mini(ui, f, img, 5);
        let p = ui.painter_at(r);
        p.text(pos2(img.right() + 14.0, r.center().y - 8.0), Align2::LEFT_CENTER, &f.nombre, fuente(15.0), TEXT);
        let carpeta = f.ruta.parent().map(|c| c.display().to_string()).unwrap_or_default();
        p.text(pos2(img.right() + 14.0, r.center().y + 11.0), Align2::LEFT_CENTER, carpeta, fuente(12.0), TEXT_FAINT);
        let fin = r.right() - 46.0;
        p.rect_filled(Rect::from_min_max(pos2(fin - 210.0, r.top() + 1.0), pos2(r.right() - 1.0, r.bottom() - 1.0)), 9, if resp.hovered() { BUTTON } else { WINDOW_TITLE });
        p.text(pos2(fin, r.center().y), Align2::RIGHT_CENTER, format!("{} · {}", hace(f.fecha), peso(f.tam)), fuente(13.0), TEXT_DIM);
        let puntos = Rect::from_center_size(pos2(r.right() - 24.0, r.center().y), vec2(30.0, 24.0));
        self.atender_proyecto(ui, f, r, &resp, puntos, hacer);
    }

    fn vista_papelera(&mut self, ui: &mut Ui, hacer: &mut Vec<Hacer>) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(t("Papelera")).font(barlow(26.0)).color(TEXT));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(12.0);
                if !self.inicio.borrados.is_empty() && ui.button(RichText::new(t("Vaciar papelera…")).color(RECORD)).clicked() {
                    hacer.push(Hacer::Borrar(self.inicio.borrados.clone()));
                }
            });
        });
        ui.label(RichText::new(t("Solo se guarda el archivo del proyecto. Tus videos, sonidos e imágenes nunca se tocan.")).color(TEXT_DIM).size(14.0));
        self.mensaje_ui(ui);
        ui.add_space(14.0);
        if self.inicio.borrados.is_empty() {
            let (r, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
            ui.painter().text(r.center(), Align2::CENTER_CENTER, t("La papelera está vacía"), fuente(15.0), TEXT_DIM);
            return;
        }
        let l = self.libro();
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for b in &self.inicio.borrados {
                Frame::new().fill(WINDOW_TITLE).corner_radius(9).inner_margin(Margin::symmetric(14, 9)).outer_margin(Margin { right: 12, ..Default::default() }).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.set_width(ui.available_width());
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.label(RichText::new(nombre_de(&b.original)).size(15.5).color(TEXT));
                            let tam = std::fs::metadata(l.archivo_de(b)).map_or(0, |m| m.len());
                            let cuando = hace(SystemTime::UNIX_EPOCH + Duration::from_secs(b.cuando));
                            ui.label(RichText::new(format!("{cuando} · {} · {}", peso(tam), b.original.parent().map(|c| c.display().to_string()).unwrap_or_default())).size(12.5).color(TEXT_FAINT));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button(RichText::new(t("Borrar para siempre")).color(RECORD)).clicked() {
                                hacer.push(Hacer::Borrar(vec![b.clone()]));
                            }
                            if ui.button(RichText::new(t("Restaurar")).color(MAIN)).clicked() {
                                hacer.push(Hacer::Restaurar(b.clone()));
                            }
                        });
                    });
                });
            }
        });
    }

    fn hacer_inicio(&mut self, h: Hacer) {
        let l = self.libro();
        match h {
            Hacer::Abrir(r) if proyectos::es_cancion(&r) => match programa_daw() {
                Some(d) => self.abrir_daw(d, Some(&r)),
                None => self.avisar(t("No encuentro Beethoven DAW. Hay que armarlo con empaque/empacar.sh."), true),
            },
            Hacer::Abrir(r) => {
                self.inicio.pendiente = Some(r);
                self.pedir(Confirmar::Abrir);
            }
            Hacer::Renombrar(r) => {
                let n = nombre_de(&r);
                self.inicio.renombrando = Some((r, n));
            }
            Hacer::Duplicar(r) => {
                match l.duplicar(&r) {
                    Ok(a) => self.avisar(tf("Se hizo «{}»", &[&nombre_de(&a)]), false),
                    Err(e) => self.avisar(tf("No se pudo duplicar: {}", &[&e]), true),
                }
                self.inicio.lista = None;
            }
            Hacer::Mostrar(r) => super::archivos::mostrar(&r),
            Hacer::Quitar(v) => {
                v.iter().for_each(|r| l.quitar(r));
                self.inicio.lista = None;
            }
            Hacer::Eliminar(v) => self.inicio.por_eliminar = v,
            Hacer::Elegir(r) => {
                if !self.inicio.elegidos.remove(&r) {
                    self.inicio.elegidos.insert(r);
                }
            }
            Hacer::Restaurar(b) => {
                match l.restaurar(&b) {
                    Ok(a) => self.avisar(tf("«{}» regresó a tus proyectos", &[&nombre_de(&a)]), false),
                    Err(e) => self.avisar(tf("No se pudo restaurar: {}", &[&e]), true),
                }
                self.inicio.lista = None;
            }
            Hacer::Borrar(v) => self.inicio.por_borrar = v,
        }
    }

    fn dialogos_inicio(&mut self, ctx: &Context) {
        let l = self.libro();
        if let Some((r, mut n)) = self.inicio.renombrando.take() {
            let (mut listo, mut cerrar) = (false, false);
            let m = Modal::new(Id::new("renombrar")).show(ctx, |ui| {
                ui.set_width(380.0);
                ui.heading(t("Cambiar nombre"));
                ui.add_space(8.0);
                let e = ui.add(TextEdit::singleline(&mut n).desired_width(f32::INFINITY).margin(Margin::symmetric(8, 6)));
                if !e.has_focus() && !ui.input(|i| i.key_pressed(Key::Enter)) {
                    e.request_focus();
                }
                listo = ui.input(|i| i.key_pressed(Key::Enter));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    listo |= ui.button(RichText::new(t("Guardar nombre")).color(MAIN)).clicked();
                    cerrar = ui.button(t("Cancelar")).clicked();
                });
            });
            cerrar |= m.should_close();
            if listo {
                match l.renombrar(&r, &n) {
                    Ok(a) => {
                        // El proyecto abierto sigue siendo el mismo archivo.
                        if self.ruta.as_ref() == Some(&r) {
                            self.ruta = Some(a);
                        }
                        self.inicio.lista = None;
                    }
                    Err(e) => {
                        self.avisar(e, true);
                        self.inicio.renombrando = Some((r, n));
                    }
                }
            } else if !cerrar {
                self.inicio.renombrando = Some((r, n));
            }
        }
        if !self.inicio.por_eliminar.is_empty() {
            let v = self.inicio.por_eliminar.clone();
            let m = Modal::new(Id::new("eliminar-proyectos")).show(ctx, |ui| {
                ui.set_width(400.0);
                ui.heading(if v.len() == 1 { tf("¿Eliminar «{}»?", &[&nombre_de(&v[0])]) } else { tf("¿Eliminar {} proyectos?", &[&v.len()]) });
                ui.add_space(4.0);
                ui.label(RichText::new(t("Se va a la papelera del menú y lo puedes restaurar. Tus videos, sonidos e imágenes no se borran.")).color(TEXT_DIM));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let si = ui.button(RichText::new(t("Sí, eliminar")).color(RECORD)).clicked();
                    (si, ui.button(t("Cancelar")).clicked())
                })
                .inner
            });
            if m.inner.0 {
                let mut mal = None;
                for r in &v {
                    match l.eliminar(r) {
                        // Si estaba abierto, queda como proyecto sin archivo: no se pierde lo que se ve.
                        Ok(()) if self.ruta.as_ref() == Some(r) => self.ruta = None,
                        Ok(()) => {}
                        Err(e) => mal = Some(e),
                    }
                }
                match mal {
                    Some(e) => self.avisar(tf("No se pudo eliminar: {}", &[&e]), true),
                    None => self.avisar(t("Se fue a la papelera"), false),
                }
                self.inicio.elegidos.clear();
                self.inicio.lista = None;
            }
            if m.inner.0 || m.inner.1 || m.should_close() {
                self.inicio.por_eliminar.clear();
            }
        }
        if !self.inicio.por_borrar.is_empty() {
            let v = self.inicio.por_borrar.clone();
            let m = Modal::new(Id::new("borrar-proyectos")).show(ctx, |ui| {
                ui.set_width(400.0);
                ui.heading(if v.len() == 1 { tf("¿Borrar «{}» para siempre?", &[&nombre_de(&v[0].original)]) } else { tf("¿Borrar {} proyectos para siempre?", &[&v.len()]) });
                ui.add_space(4.0);
                ui.label(RichText::new(t("Esto no se puede deshacer.")).color(TEXT_DIM));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let si = ui.button(RichText::new(t("Sí, borrar para siempre")).color(RECORD)).clicked();
                    (si, ui.button(t("Cancelar")).clicked())
                })
                .inner
            });
            if m.inner.0 {
                if let Some(e) = v.iter().filter_map(|b| l.borrar(b).err()).next_back() {
                    self.avisar(tf("No se pudo eliminar: {}", &[&e]), true);
                }
                self.inicio.lista = None;
            }
            if m.inner.0 || m.inner.1 || m.should_close() {
                self.inicio.por_borrar.clear();
            }
        }
    }
}

/// El retrato de la app en `r`, nítido: se reduce una sola vez al tamaño exacto en pixeles, promediando todo lo que cae
/// en cada uno. En chico se ve la cara más de cerca.
pub fn logo(ui: &Ui, r: Rect) {
    let lado = (r.width() * ui.ctx().pixels_per_point()).round().max(1.0) as usize;
    let id = Id::new(("logo", lado));
    let tex: TextureHandle = ui.data(|d| d.get_temp(id)).unwrap_or_else(|| {
        let img = ColorImage::from_rgba_unmultiplied([lado, lado], &reducir_logo(super::ICONO, lado, r.width() <= 72.0));
        let x = ui.ctx().load_texture(format!("logo{lado}"), img, TextureOptions::LINEAR);
        ui.data_mut(|d| d.insert_temp(id, x.clone()));
        x
    });
    Image::new(&tex).corner_radius((r.width() * 0.2) as u8).paint_at(ui, r);
}

/// Área promediada en luz lineal. `cerca`: solo la cara.
fn reducir_logo(q: &[u8], lado: usize, cerca: bool) -> Vec<u8> {
    const N: usize = 512;
    // El dibujo ocupa el centro; alrededor es margen transparente.
    let (x0, y0, l) = if cerca { (104.0f32, 88.0f32, 280.0f32) } else { (52.0, 52.0, 408.0) };
    let k = l / lado as f32;
    let lineal: Vec<f32> = (0..256).map(|v| (v as f32 / 255.0).powf(2.2)).collect();
    let mut o = vec![255u8; lado * lado * 4];
    for y in 0..lado {
        for x in 0..lado {
            let (a, b) = (x0 + x as f32 * k, x0 + (x + 1) as f32 * k);
            let (d, e) = (y0 + y as f32 * k, y0 + (y + 1) as f32 * k);
            let (mut s, mut peso) = ([0.0f32; 3], 0.0f32);
            for yy in d as usize..(e.ceil() as usize).min(N) {
                let py = (e.min(yy as f32 + 1.0) - d.max(yy as f32)).max(0.0);
                for xx in a as usize..(b.ceil() as usize).min(N) {
                    let p = py * (b.min(xx as f32 + 1.0) - a.max(xx as f32)).max(0.0);
                    let i = (yy * N + xx) * 4;
                    (0..3).for_each(|j| s[j] += lineal[q[i + j] as usize] * p);
                    peso += p;
                }
            }
            let i = (y * lado + x) * 4;
            (0..3).for_each(|j| o[i + j] = ((s[j] / peso.max(1e-6)).powf(1.0 / 2.2) * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cada pixel del logo es el promedio de lo que tapa: sin dientes ni saltos.
    #[test]
    fn el_logo_se_promedia() {
        let rayas: Vec<u8> = (0..512 * 512).flat_map(|i| if (i % 512) % 2 == 0 { [255, 255, 255, 255] } else { [0, 0, 0, 255] }).collect();
        let o = reducir_logo(&rayas, 64, false);
        assert!(o.chunks(4).all(|p| (p[0] as i32 - 186).abs() < 14), "rayas finas quedan en un gris parejo");
        let blanco = vec![200u8; 512 * 512 * 4];
        for (lado, cerca) in [(1, true), (56, true), (112, true), (256, false), (600, false)] {
            let o = reducir_logo(&blanco, lado, cerca);
            assert!(o.len() == lado * lado * 4 && o.chunks(4).all(|p| p[0].abs_diff(200) < 2));
        }
    }
}
