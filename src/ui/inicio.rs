//! Menú principal: lo primero que se ve al abrir la app. Proyectos recientes, nuevo video, nueva canción y abrir.
use super::estilo::*;
use super::{App, Confirmar, Espacio, carpeta, nombre_de, tabler};
use crate::idioma::{self, Idioma, t};
use eframe::egui::*;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// Cuántos proyectos recientes se recuerdan.
const TOPE: usize = 12;
/// Tamaño de la ventana del menú y del editor.
pub const MENU: Vec2 = vec2(980.0, 620.0);
pub const EDITOR: Vec2 = vec2(1440.0, 860.0);
/// Lo mínimo que se ve la pantalla de carga.
const CARGA_MIN: Duration = Duration::from_millis(900);

pub struct Reciente {
    pub ruta: PathBuf,
    mini: Option<TextureHandle>,
    cuando: String,
}

#[derive(Default)]
pub struct Estado {
    pub visible: bool,
    /// Se lee al mostrar el menú; `None` = volver a leer.
    lista: Option<Vec<Reciente>>,
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
        ctx.send_viewport_cmd(ViewportCommand::Resizable(!menu));
        ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(if menu { MENU } else { vec2(960.0, 600.0) }));
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

fn archivo_recientes() -> PathBuf {
    carpeta().join("Recientes.json")
}

pub fn recientes() -> Vec<PathBuf> {
    let l: Vec<PathBuf> = std::fs::read(archivo_recientes()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    l.into_iter().filter(|r| r.exists()).collect()
}

/// Pone `ruta` al principio de los recientes.
pub fn anotar(ruta: &Path) {
    let ruta = ruta.canonicalize().unwrap_or_else(|_| ruta.to_path_buf());
    let mut l = recientes();
    l.retain(|r| *r != ruta);
    l.insert(0, ruta);
    l.truncate(TOPE);
    let _ = std::fs::create_dir_all(carpeta());
    if let Ok(b) = serde_json::to_vec_pretty(&l) {
        let _ = std::fs::write(archivo_recientes(), b);
    }
}

fn hace(t: SystemTime) -> String {
    let d = SystemTime::now().duration_since(t).map(|d| d.as_secs() / 86_400).unwrap_or(0);
    match d {
        0 => "Hoy".into(),
        1 => "Ayer".into(),
        2..=59 => crate::idioma::tf("Hace {} días", &[&d]),
        _ => crate::idioma::tf("Hace {} meses", &[&(d / 30)]),
    }
}

fn leer(ctx: &Context) -> Vec<Reciente> {
    recientes()
        .into_iter()
        .map(|ruta| {
            let mini = crate::archivo::miniatura(&ruta)
                .map(|(an, al, q)| ctx.load_texture(format!("reciente:{}", ruta.display()), ColorImage::from_rgba_unmultiplied([an, al], &q), TextureOptions::LINEAR));
            let cuando = std::fs::metadata(&ruta).and_then(|m| m.modified()).map(hace).unwrap_or_default();
            Reciente { ruta, mini, cuando }
        })
        .collect()
}

/// Botón grande del menú: ícono, título y una línea de ayuda.
fn opcion(ui: &mut Ui, i: &tabler::TablerIcon, titulo: &str, ayuda: &str, principal: bool) -> Response {
    let (titulo, ayuda) = (crate::idioma::t(titulo), crate::idioma::t(ayuda));
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 64.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), titulo));
    let p = ui.painter();
    let activo = ui.is_enabled();
    let fondo = if !activo { SUNKEN } else if resp.hovered() { BUTTON_HI } else { BUTTON };
    p.rect_filled(r, 8, fondo);
    if principal && activo {
        p.rect_filled(Rect::from_min_size(r.min, vec2(4.0, r.height())), 2, MAIN);
    }
    let tinta = if activo { TEXT } else { TEXT_FAINT };
    icono(p, i, Rect::from_center_size(pos2(r.left() + 34.0, r.center().y), Vec2::splat(26.0)), if activo { GLYPH } else { TEXT_FAINT });
    p.text(pos2(r.left() + 62.0, r.center().y - 10.0), Align2::LEFT_CENTER, titulo, fuente(18.0), tinta);
    p.text(pos2(r.left() + 62.0, r.center().y + 12.0), Align2::LEFT_CENTER, ayuda, fuente(13.0), TEXT_DIM);
    resp
}

/// Tarjeta de un proyecto reciente.
fn tarjeta(ui: &mut Ui, r: &Reciente) -> Response {
    const AN: f32 = 240.0;
    let al = AN * 9.0 / 16.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(AN, al + 48.0), Sense::click());
    let nombre = nombre_de(&r.ruta);
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &nombre));
    let p = ui.painter();
    let img = Rect::from_min_size(rect.min, vec2(AN, al));
    p.rect_filled(rect, 8, if resp.hovered() { BUTTON } else { WINDOW_TITLE });
    match &r.mini {
        Some(t) => {
            p.image(t.id(), img.shrink(1.0), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        None => {
            p.rect_filled(img.shrink(1.0), 7, WELL);
            icono(p, &tabler::FILE_MUSIC, Rect::from_center_size(img.center(), Vec2::splat(36.0)), TEXT_FAINT);
        }
    }
    if resp.hovered() {
        p.rect_stroke(rect, 8, Stroke::new(1.5, MAIN), StrokeKind::Inside);
    }
    let texto = rect.left() + 10.0;
    let ancho = AN - 20.0;
    let g = p.layout(nombre.clone(), fuente(15.0), TEXT, ancho);
    let g = if g.rows.len() > 1 { p.layout_no_wrap(format!("{}…", nombre.chars().take(26).collect::<String>()), fuente(15.0), TEXT) } else { g };
    p.galley(pos2(texto, img.bottom() + 8.0), g, TEXT);
    p.text(pos2(texto, img.bottom() + 36.0), Align2::LEFT_CENTER, &r.cuando, fuente(12.5), TEXT_DIM);
    resp.on_hover_text(r.ruta.display().to_string())
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
            if ui.button(t("Listo")).clicked() {
                cerrar = true;
            }
        });
        if cerrar {
            self.inicio.config = false;
        }
    }

    /// Arranca Beethoven DAW en su ventana; el menú muestra la carga y después se cierra.
    fn abrir_daw(&mut self, programa: PathBuf) {
        match std::process::Command::new(&programa).env("BEETHOVEN_IDIOMA", idioma::actual().codigo()).spawn() {
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
                if let Some(t) = &self.retrato {
                    ui.add(Image::new(t).fit_to_exact_size(Vec2::splat(128.0)).corner_radius(20));
                }
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
            } else if desde.elapsed() >= CARGA_MIN && !self.ocupada() {
                self.inicio.entrar();
            }
            ctx.request_repaint_after(Duration::from_millis(16));
            return;
        }
        if self.inicio.lista.is_none() {
            self.inicio.lista = Some(if self.sin_dispositivo { vec![] } else { leer(&ctx) });
        }
        Panel::left("inicio-menu").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(28.0)).resizable(false).exact_size(340.0).show(ui, |ui| {
            if let Some(t) = &self.retrato {
                ui.add(Image::new(t).fit_to_exact_size(Vec2::splat(88.0)).corner_radius(14));
            }
            ui.add_space(12.0);
            ui.label(RichText::new(crate::idioma::t("Beethoven Craft")).font(barlow(30.0)).color(TEXT));
            ui.label(RichText::new(crate::idioma::t("Compositor de video y audio")).size(14.0).color(TEXT_DIM));
            ui.add_space(28.0);
            if opcion(ui, &tabler::MOVIE, "Nuevo video", "Beethoven Movie: compón y edita tu video", true).clicked() {
                self.espacio = Espacio::Video;
                self.pedir(Confirmar::Nuevo);
            }
            ui.add_space(8.0);
            let daw = programa_daw();
            ui.add_enabled_ui(daw.is_some(), |ui| {
                let r = opcion(ui, &tabler::PIANO, "Nueva canción", "Beethoven DAW: rack, piano roll y mezcla", false)
                    .on_disabled_hover_text(crate::idioma::t("No encuentro Beethoven DAW. Hay que armarlo con empaque/empacar.sh."));
                if r.clicked() {
                    self.abrir_daw(daw.unwrap());
                }
            });
            ui.add_space(8.0);
            if opcion(ui, &tabler::FOLDER_OPEN, "Abrir proyecto…", "Un archivo .bcraft de tu Mac", false).clicked() {
                self.inicio.pendiente = None;
                self.pedir(Confirmar::Abrir);
            }
            ui.add_space(8.0);
            if opcion(ui, &tabler::SETTINGS, t("Configuración"), t("Idioma y atajos de teclado"), false).clicked() {
                self.inicio.config = true;
            }
            if self.ruta.is_some() || self.sin_guardar() {
                ui.add_space(20.0);
                let n = self.ruta.as_deref().map_or(crate::idioma::t("Proyecto sin nombre").into(), nombre_de);
                if ui.add(Button::new(RichText::new(crate::idioma::tf("← Volver a «{}»", &[&n])).size(14.0)).frame(false)).clicked() {
                    self.inicio.entrar();
                }
            }
        });
        CentralPanel::default().frame(Frame::new().fill(WORKSPACE).inner_margin(Margin::symmetric(36, 28))).show(ui, |ui| {
            ui.label(RichText::new(crate::idioma::t("Recientes")).font(barlow(22.0)).color(TEXT));
            self.mensaje_ui(ui);
            ui.add_space(14.0);
            let lista = self.inicio.lista.as_deref().unwrap_or_default();
            if lista.is_empty() {
                ui.add_space(40.0);
                ui.label(RichText::new(crate::idioma::t("Aún no hay proyectos. Empieza uno nuevo o abre uno que ya tengas.")).size(15.0).color(TEXT_DIM));
                return;
            }
            let mut abrir = None;
            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(18.0, 18.0);
                    for r in lista {
                        if tarjeta(ui, r).clicked() {
                            abrir = Some(r.ruta.clone());
                        }
                    }
                });
            });
            if let Some(r) = abrir {
                self.inicio.pendiente = Some(r);
                self.pedir(Confirmar::Abrir);
            }
        });
        if self.inicio.config {
            self.ventana_config(&ctx);
        }
    }
}
