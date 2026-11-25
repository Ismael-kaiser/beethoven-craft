// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Los lados: la biblioteca, el instrumento elegido y el editor de Lua.
use super::*;
use crate::lua::Ficha;
use crate::plugin::Plugin;

pub struct Editor {
    clave: String,
    texto: String,
    error: Option<String>,
    /// Canal que se quedará con el guion al guardar, si es nuevo.
    canal: Option<u32>,
}

/// Para qué se elige un instrumento.
#[derive(Clone, Copy, PartialEq)]
enum Para {
    /// Cambiar el del canal elegido.
    Cambiar,
    /// Un canal más.
    Canal,
    /// Un patrón nuevo.
    Patron,
    /// Una pista de la playlist (o la primera libre), con su patrón.
    Pista(Option<usize>),
}

/// El cuadro que pregunta con qué instrumento.
pub struct Elegir {
    para: Para,
    buscar: String,
}

impl Elegir {
    pub fn canal() -> Elegir {
        Elegir { para: Para::Canal, buscar: String::new() }
    }
    pub fn cambiar() -> Elegir {
        Elegir { para: Para::Cambiar, buscar: String::new() }
    }
    pub fn patron() -> Elegir {
        Elegir { para: Para::Patron, buscar: String::new() }
    }
    pub fn pista(i: Option<usize>) -> Elegir {
        Elegir { para: Para::Pista(i), buscar: String::new() }
    }
}

/// Lo que se puede elegir.
#[derive(Clone)]
pub enum Cual {
    Canal(u32),
    Lua(String),
    Plugin(PathBuf),
    Archivo(PathBuf),
}

/// Un plugin completo para empezar: con su ventana, sus preajustes y todo explicado.
const NUEVO_EFECTO: &str = include_str!("../../plantillas/efecto.lua");
const NUEVO: &str = include_str!("../../plantillas/instrumento.lua");

/// La grabación resumida en 256 picos, de 0 a 1.
pub fn picos(m: &crate::muestrario::Muestra) -> Vec<f32> {
    let Some(z) = m.zonas.first() else { return vec![] };
    let x = &m.datos[z.ini..z.fin];
    let n = x.len().max(1);
    let v: Vec<f32> = (0..256)
        .map(|k| x[(k * n / 256).min(x.len())..((k + 1) * n / 256).min(x.len())].iter().step_by(1 + n / 100_000).fold(0f32, |m, v| m.max(v.abs())))
        .collect();
    let alto = v.iter().cloned().fold(1e-6, f32::max);
    v.into_iter().map(|x| x / alto).collect()
}

pub fn controles(ui: &mut Ui, f: &Ficha, v: &mut BTreeMap<String, f32>, c: Color32) {
    ui.horizontal_wrapped(|ui| {
        for k in &f.controles {
            let mut x = v.get(&k.clave).copied().unwrap_or(k.valor);
            if perilla(ui, k.nombre.ver(), &mut x, k.min, k.max, k.valor, c, 38.0).changed() {
                if x == k.valor {
                    v.remove(&k.clave);
                } else {
                    v.insert(k.clave.clone(), x);
                }
            }
        }
    });
}

pub fn controles_plugin(ui: &mut Ui, p: &Plugin) {
    let n = p.controles.len();
    if n == 0 {
        return;
    }
    CollapsingHeader::new(tf("{} controles", &[&n])).id_salt(p as *const Plugin as usize).show(ui, |ui| {
        ScrollArea::vertical().max_height(320.0).show_rows(ui, 38.0, n, |ui, filas| {
            for c in &p.controles[filas] {
                let mut v = p.valor(c.id);
                let x = p.texto(c.id, v);
                ui.label(RichText::new(format!("{}: {x}", c.nombre)).size(11.5).color(TENUE));
                let mut s = Slider::new(&mut v, c.min..=c.max).show_value(false);
                if c.entero && c.max - c.min >= 1.0 {
                    s = s.step_by(1.0);
                }
                if ui.add(s).changed() {
                    p.mover(c.id, v);
                }
            }
        });
    });
}

/// Un renglón de la biblioteca.
/// Abre la carpeta en el explorador del sistema.
fn mostrar_carpeta(d: &std::path::Path) {
    let programa = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(programa).arg(d).spawn();
}

fn renglon(ui: &mut Ui, s: &str, c: Color32, logo: Option<TextureHandle>) -> Response {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 23.0), Sense::click());
    if resp.hovered() {
        relieve(ui.painter(), r, 4, CAJA, true, resp.is_pointer_button_down_on());
    }
    // Con logo, el logo; si no, un puntito del color de su tipo.
    match logo {
        Some(t) => App::pinta_logo(ui.painter(), Rect::from_center_size(r.left_center() + vec2(9.0, 0.0), Vec2::splat(16.0)), &t),
        None => _ = ui.painter().circle_filled(r.left_center() + vec2(8.0, 0.0), 3.0, c.gamma_multiply(if resp.hovered() { 1.0 } else { 0.7 })),
    }
    rotulo(ui.painter(), Rect::from_min_max(r.min + vec2(18.0, 0.0), r.max), s, 15.0, TEXTO);
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, s));
    resp
}

impl App {
    pub(super) fn biblioteca(&mut self, ui: &mut Ui) {
        ui.add_space(6.0);
        ui.add(TextEdit::singleline(&mut self.buscar).hint_text(t("Buscar…")).desired_width(f32::INFINITY));
        let q = self.buscar.to_lowercase();
        let pasa = |s: &str| q.is_empty() || s.to_lowercase().contains(&q);
        let (mut ins, mut fx, mut plug, mut edita, mut nuevo) = (None, None, None, None, None);
        let (mut tuyo, mut instala) = (None::<(String, u8)>, 0u8);
        let mut notas = None;
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            CollapsingHeader::new(t("Instrumentos")).id_salt("Instrumentos").default_open(true).show(ui, |ui| {
                if crate::banco::archivo().is_none() {
                    ui.label(RichText::new(t("Falta el banco de instrumentos grabados (sonidos/MuseScore_General.sf3)")).size(11.0).color(ROJO));
                }
                // Los sintetizadores de fábrica, antes que las grabaciones.
                for (g, v) in self.bib.grupos(Tipo::Instrumento) {
                    let v: Vec<_> = v.into_iter().filter(|x| !x.propio && (pasa(x.ficha.nombre.ver()) || pasa(g))).collect();
                    if v.is_empty() {
                        continue;
                    }
                    CollapsingHeader::new(g).id_salt("sintes").open(if q.is_empty() { None } else { Some(true) }).show(ui, |ui| {
                        for x in v {
                            let b = renglon(ui, x.ficha.nombre.ver(), ACENTO, None);
                            if b.clicked() {
                                ins = Some(x.clave.clone());
                            }
                            b.context_menu(|ui| {
                                if ui.button(t("Editar en Lua")).clicked() {
                                    edita = Some(x.clave.clone());
                                }
                            });
                            if b.hovered() {
                                self.pista_ayuda = t("Clic: agregar como canal").into();
                            }
                        }
                    });
                }
                for (fam, v) in crate::banco::familias() {
                    let v: Vec<_> = v.into_iter().filter(|x| pasa(t(x.nombre)) || pasa(t(fam))).collect();
                    if v.is_empty() {
                        continue;
                    }
                    CollapsingHeader::new(t(fam)).id_salt(fam).open(if q.is_empty() { None } else { Some(true) }).show(ui, |ui| {
                        for x in v {
                            let b = renglon(ui, t(x.nombre), ACENTO, None);
                            if b.clicked() {
                                ins = Some(x.clave.to_string());
                            }
                            if b.hovered() {
                                self.pista_ayuda = t("Clic: agregar como canal").into();
                            }
                        }
                    });
                }
            });
            // Los de fábrica arriba; los tuyos, en «Tus plugins».
            CollapsingHeader::new(t("Efectos")).id_salt("Efectos").default_open(true).show(ui, |ui| {
                for (g, v) in self.bib.grupos(Tipo::Efecto) {
                    let v: Vec<_> = v.into_iter().filter(|x| !x.propio && (pasa(x.ficha.nombre.ver()) || pasa(g))).collect();
                    if v.is_empty() {
                        continue;
                    }
                    ui.label(RichText::new(g).size(11.0).color(TENUE));
                    for x in v {
                        let logo = self.logo_de(ui.ctx(), &x.clave);
                        let b = renglon(ui, x.ficha.nombre.ver(), Color32::from_rgb(0x8c, 0xb8, 0xff), logo);
                        if b.clicked() {
                            fx = Some(x.clave.clone());
                        }
                        b.context_menu(|ui| {
                            if ui.button(t("Editar en Lua")).clicked() {
                                edita = Some(x.clave.clone());
                            }
                        });
                        if b.hovered() {
                            self.pista_ayuda = t("Clic: poner en la pista de mezcla elegida · clic derecho: editar su guion de Lua").into();
                        }
                    }
                }
            });
            CollapsingHeader::new(t("Tus plugins")).id_salt("crear").default_open(true).show(ui, |ui| {
                ui.label(
                    RichText::new(t("Los que instalas o haces en Lua. Suelta aquí una carpeta, un .zip o un .lua y queda listo."))
                        .size(11.0)
                        .color(TENUE),
                );
                let tuyos: Vec<_> = self
                    .bib
                    .guiones
                    .iter()
                    .filter(|g| g.propio && !matches!(g.ficha.tipo, Tipo::Herramienta) && pasa(g.ficha.nombre.ver()))
                    .collect();
                for x in tuyos {
                    let es_ins = x.ficha.tipo == Tipo::Instrumento;
                    let logo = self.logo_de(ui.ctx(), &x.clave);
                    let b = renglon(ui, x.ficha.nombre.ver(), if es_ins { ACENTO } else { Color32::from_rgb(0x8c, 0xb8, 0xff) }, logo);
                    if b.clicked() {
                        if x.ficha.tipo == Tipo::Notas {
                            notas = Some(x.clave.clone());
                        } else if es_ins {
                            ins = Some(x.clave.clone());
                        } else {
                            fx = Some(x.clave.clone());
                        }
                    }
                    b.context_menu(|ui| {
                        for (s, que) in
                            [("Editar en Lua", 0), ("Abrir su carpeta", 1), ("Empacar para compartir…", 2), ("Quitar (va a «Quitados»)", 3)]
                        {
                            if ui.button(t(s)).clicked() {
                                tuyo = Some((x.clave.clone(), que));
                                ui.close();
                            }
                        }
                    });
                    if b.hovered() {
                        self.pista_ayuda = t(if es_ins {
                            "Clic: agregar como canal · clic derecho: editar su guion de Lua"
                        } else {
                            "Clic: poner en la pista de mezcla elegida · clic derecho: editar su guion de Lua"
                        })
                        .into();
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button(t("+ Instrumento en Lua")).clicked() {
                        nuevo = Some(NUEVO);
                    }
                    if ui.button(t("+ Efecto en Lua")).clicked() {
                        nuevo = Some(NUEVO_EFECTO);
                    }
                    if ui.button(t("Instalar plugin…")).clicked() {
                        instala = 1;
                    }
                    if ui
                        .button(t("Abrir la carpeta"))
                        .on_hover_text(t("Lo que pongas ahí se carga solo, y al guardar un cambio se oye de una vez"))
                        .clicked()
                    {
                        instala = 2;
                    }
                });
            });
            if !self.plugins.is_empty() {
                CollapsingHeader::new(t("Plugins")).id_salt("plugins").default_open(false).show(ui, |ui| {
                    for r in &self.plugins {
                        let n = r.file_stem().unwrap_or_default().to_string_lossy();
                        if !pasa(&n) {
                            continue;
                        }
                        let tipo = r.extension().unwrap_or_default().to_string_lossy().to_uppercase();
                        if renglon(ui, &format!("{n}  ·  {tipo}"), VERDE, None).clicked() {
                            plug = Some(r.clone());
                        }
                    }
                });
            }
            ui.add_space(8.0);
            if ui.button(t("Buscar un plugin…")).clicked() {
                self.elegir_plugin(None);
            }
        });
        if let Some(k) = ins {
            self.poner_instrumento(&k);
            self.mostrar(Vista::Ritmo);
        }
        if let Some(k) = fx {
            self.p.mezcla[self.pista].efectos.push(EfectoPuesto::nuevo(&k));
            self.mostrar(Vista::Mezcla);
            self.abrir_efecto(self.pista, self.p.mezcla[self.pista].efectos.len() - 1);
        }
        if let Some(r) = plug {
            self.ofrecer(&r, None);
        }
        if let Some(k) = edita {
            self.editar(&k);
        }
        if let (Some(k), Some(c)) = (notas, self.p.canales.iter_mut().find(|c| c.id == self.canal)) {
            c.notas = Some(k);
            self.mostrar(Vista::Canal);
        }
        match tuyo {
            Some((k, 0)) => self.editar(&k),
            Some((k, 1)) => _ = self.bib.carpeta_de(&k).map(|d| mostrar_carpeta(&d)),
            Some((k, 2)) => self.empacar_plugin(&k),
            Some((k, _)) => match self.bib.quitar(&k) {
                Ok(()) => {
                    self.cambio_bib += 1;
                    self.avisar(tf("«{}» se quitó; quedó en la carpeta «Quitados»", &[&k]), false);
                }
                Err(e) => self.avisar(e, true),
            },
            None => {}
        }
        match instala {
            1 => {
                if let Some(r) = rfd::FileDialog::new().add_filter("Plugin", &["zip", "bplugin", "lua"]).pick_file() {
                    self.instalar_plugin(&r);
                }
            }
            2 => {
                if let Some(d) = self.bib.carpeta.clone() {
                    let _ = std::fs::create_dir_all(&d);
                    mostrar_carpeta(&d);
                }
            }
            _ => {}
        }
        if let Some(texto) = nuevo {
            let clave = self.bib.clave_libre(if idioma::actual() == Idioma::En { "my_sound" } else { "mi_sonido" });
            self.editor = Some(Editor { clave, texto: texto.into(), error: None, canal: (texto == NUEVO).then_some(0) });
        }
    }

    /// Instala una carpeta, un paquete o un guion, y dice cómo quedó.
    pub fn instalar_plugin(&mut self, r: &Path) {
        match self.bib.instalar(r) {
            Ok((clave, inf)) => {
                self.cambio_bib += 1;
                let Some(g) = self.bib.ver(&clave) else { return };
                let (nombre, autor) = (g.ficha.nombre.ver().to_string(), g.ficha.autor.clone());
                let quien = if autor.is_empty() { nombre } else { format!("{nombre} · {autor}") };
                self.avisar(tf("«{}» quedó instalado: está en «Tus plugins» ({} % de procesador)", &[&quien, &format!("{:.1}", inf.cpu)]), false);
                inf.avisos.into_iter().for_each(|a| self.avisar(a, true));
            }
            Err(e) => self.avisar(tf("No se instaló «{}»: {}", &[&r.file_name().unwrap_or_default().to_string_lossy(), &e]), true),
        }
    }

    fn empacar_plugin(&mut self, clave: &str) {
        let b = match self.bib.empacar(clave) {
            Ok(b) => b,
            Err(e) => return self.avisar(e, true),
        };
        let Some(r) = rfd::FileDialog::new().set_file_name(format!("{clave}.zip")).add_filter("Plugin", &["zip", "bplugin"]).save_file() else {
            return;
        };
        match std::fs::write(&r, b) {
            Ok(()) => self.avisar(t("Listo para compartir: quien lo reciba solo tiene que soltarlo en el programa"), false),
            Err(e) => self.avisar(tf("No se pudo guardar: {}", &[&e]), true),
        }
    }

    /// Carga lo que cambió en la carpeta de plugins.
    pub fn vigilar_plugins(&mut self) {
        use crate::instalar::Cambio;
        for c in self.bib.vigilar() {
            self.cambio_bib += 1;
            match c {
                Cambio::Nuevo(k) => self.avisar(tf("Plugin nuevo: «{}»", &[&self.bib.nombre(&k)]), false),
                Cambio::Mejor(k) => self.avisar(tf("«{}» se actualizó", &[&self.bib.nombre(&k)]), false),
                Cambio::Fuera(k) => self.avisar(tf("«{}» ya no está en la carpeta", &[&k]), false),
                Cambio::Mal(k, e) => self.avisar(format!("{k}: {e}"), true),
            }
        }
    }

    fn editar(&mut self, clave: &str) {
        if let Some(g) = self.bib.ver(clave) {
            self.editor = Some(Editor { clave: clave.into(), texto: g.fuente.clone(), error: None, canal: None });
        }
    }

    /// El botón del Piano roll que dice qué instrumento suena y deja cambiarlo.
    pub(super) fn boton_instrumento(&mut self, ui: &mut Ui) {
        let Some(c) = self.p.canales.iter().find(|c| c.id == self.canal) else { return };
        let n = match &c.plugin {
            Some(x) => x.nombre.clone(),
            None => self.bib.nombre(&c.instrumento),
        };
        if ui.button(RichText::new(tf("Suena: {}", &[&n])).color(ACENTO)).on_hover_text(t("Cambiar el instrumento de este canal")).clicked() {
            self.elegir = Some(Elegir::cambiar());
        }
    }

    pub(super) fn ver_elegir(&mut self, ui: &mut Ui) {
        let Some(mut e) = self.elegir.take() else { return };
        let mut cual = None;
        let r = Modal::new(Id::new("elegir")).show(ui.ctx(), |ui| {
            ui.set_width(560.0);
            ui.label(
                RichText::new(t(match e.para {
                    Para::Cambiar => "¿Con qué instrumento suena?",
                    Para::Canal => "¿Qué instrumento agrego?",
                    Para::Patron => "¿Qué instrumento toca este patrón?",
                    Para::Pista(_) => "¿Qué instrumento va en la pista?",
                }))
                .size(19.0)
                .color(ACENTO),
            );
            ui.add(TextEdit::singleline(&mut e.buscar).hint_text(t("Buscar…")).desired_width(f32::INFINITY));
            let q = e.buscar.to_lowercase();
            let pasa = |s: &str| q.is_empty() || s.to_lowercase().contains(&q);
            let mut teja = |ui: &mut Ui, s: &str, c: Color32, x: Cual| {
                let (r, resp) = ui.allocate_exact_size(vec2(176.0, 34.0), Sense::click());
                relieve(ui.painter(), r, 6, CAJA, resp.hovered(), resp.is_pointer_button_down_on());
                ui.painter().rect_filled(Rect::from_min_size(r.min + vec2(4.0, 6.0), vec2(4.0, r.height() - 12.0)), 2.0, c);
                rotulo(ui.painter(), r.shrink2(vec2(14.0, 0.0)), s, 14.5, TEXTO);
                resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, s));
                if resp.clicked() {
                    cual = Some(x);
                }
            };
            ScrollArea::vertical().max_height(380.0).auto_shrink([false, true]).show(ui, |ui| {
                if e.para == Para::Patron && !self.p.canales.is_empty() {
                    ui.label(RichText::new(t("Los que ya tienes")).size(12.0).color(TENUE));
                    ui.horizontal_wrapped(|ui| {
                        for c in self.p.canales.iter().filter(|c| pasa(&c.nombre)) {
                            teja(ui, &c.nombre, color(c.color), Cual::Canal(c.id));
                        }
                    });
                }
                for (fam, v) in crate::banco::familias() {
                    let v: Vec<_> = v.into_iter().filter(|x| pasa(t(x.nombre)) || pasa(t(fam))).collect();
                    if v.is_empty() {
                        continue;
                    }
                    ui.label(RichText::new(t(fam)).size(12.0).color(TENUE));
                    ui.horizontal_wrapped(|ui| {
                        for x in v {
                            teja(ui, t(x.nombre), ACENTO, Cual::Lua(x.clave.into()));
                        }
                    });
                }
                for (g, v) in self.bib.grupos(Tipo::Instrumento) {
                    let v: Vec<_> = v.into_iter().filter(|x| pasa(x.ficha.nombre.ver()) || pasa(g)).collect();
                    if v.is_empty() {
                        continue;
                    }
                    ui.label(RichText::new(g).size(12.0).color(TENUE));
                    ui.horizontal_wrapped(|ui| {
                        for x in v {
                            teja(ui, x.ficha.nombre.ver(), ACENTO, Cual::Lua(x.clave.clone()));
                        }
                    });
                }
                let plugins: Vec<&PathBuf> = self.plugins.iter().filter(|r| pasa(&r.file_stem().unwrap_or_default().to_string_lossy())).collect();
                if !plugins.is_empty() {
                    ui.label(RichText::new(t("Plugins")).size(12.0).color(TENUE));
                    ui.horizontal_wrapped(|ui| {
                        for r in plugins {
                            let n = format!(
                                "{} · {}",
                                r.file_stem().unwrap_or_default().to_string_lossy(),
                                r.extension().unwrap_or_default().to_string_lossy()
                            );
                            teja(ui, &n, VERDE, Cual::Plugin(r.clone()));
                        }
                    });
                }
            });
            ui.horizontal(|ui| {
                if ui.button(t("Audio, SF2 o SFZ…")).clicked()
                    && let Some(r) = rfd::FileDialog::new()
                        .add_filter("Audio, SF2, SF3, SFZ", &[crate::decodificar::EXTENSIONES, &["sf2", "sf3", "sfz"]].concat())
                        .pick_file()
                {
                    cual = Some(Cual::Archivo(r));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.button(t("Cancelar")).clicked()).inner
            })
            .inner
        });
        if let Some(x) = cual {
            self.elegido(e.para, x);
        } else if !r.inner && !r.should_close() {
            self.elegir = Some(e);
        }
    }

    /// Agrega un canal con eso y dice cuál quedó. Nada si no se pudo o no era instrumento.
    fn canal_con(&mut self, x: &Cual) -> Option<u32> {
        let n = self.p.canales.len();
        match x {
            Cual::Canal(id) => return Some(*id),
            Cual::Lua(k) => self.poner_instrumento(k),
            Cual::Plugin(r) => self.ofrecer(r, None),
            Cual::Archivo(r) => {
                let nombre = r.file_stem().unwrap_or_default().to_string_lossy().to_string();
                if let Err(e) = self.poner_muestra(r, &nombre) {
                    self.avisar(format!("{nombre}: {e}"), true);
                }
            }
        }
        (self.p.canales.len() > n).then_some(self.canal)
    }

    fn elegido(&mut self, para: Para, x: Cual) {
        if para == Para::Cambiar {
            // El canal conserva su nombre, notas y mezcla; solo cambia con qué suena.
            let viejo = self.canal;
            if let (Cual::Lua(k), Some(c)) = (&x, self.p.canales.iter_mut().find(|c| c.id == viejo)) {
                c.instrumento = k.clone();
                c.controles.clear();
                c.plugin = None;
                return;
            }
            let Some(nuevo) = self.canal_con(&x) else { return };
            let Some(i) = self.p.canales.iter().position(|c| c.id == nuevo) else { return };
            let n = self.p.canales.remove(i);
            if let Some(c) = self.p.canales.iter_mut().find(|c| c.id == viejo) {
                (c.instrumento, c.controles, c.plugin, c.muestra, c.preset) = (n.instrumento, n.controles, n.plugin, n.muestra, n.preset);
            }
            self.canal = viejo;
            return;
        }
        let Some(canal) = self.canal_con(&x) else { return };
        self.canal = canal;
        self.clip = None;
        let nombre = self.p.canales.iter().find(|c| c.id == canal).map_or(String::new(), |c| c.nombre.clone());
        if para != Para::Canal {
            self.patron = self.p.agregar_patron();
            let y = self.p.patrones.last_mut().unwrap();
            y.nombre = nombre.clone();
            if let Para::Pista(i) = para {
                y.compases = 4;
                let libre = |p: &Proyecto, i: usize| p.clips.iter().all(|c| c.pista != i) && p.pistas[i].nombre.is_empty();
                let i = i.or_else(|| (0..PISTAS_ARREGLO).find(|i| libre(&self.p, *i))).unwrap_or(0);
                let tick = self.donde() / COMPAS * COMPAS;
                if self.p.pistas[i].nombre.is_empty() {
                    self.p.pistas[i].nombre = nombre;
                }
                self.p.clips.push(Clip::de_patron(self.patron, i, tick, 4 * COMPAS));
            }
        }
        self.notas.centrar();
        self.mostrar(Vista::Notas);
    }

    /// Los ajustes del clip de audio elegido.
    fn ver_clip(&mut self, ui: &mut Ui, i: usize) {
        let mut ver = None;
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            let Some(a) = &mut self.p.clips[i].audio else { return };
            ui.add(TextEdit::singleline(&mut a.nombre).font(letra(17.0)).desired_width(f32::INFINITY));
            let antes = a.velocidad;
            ui.horizontal_wrapped(|ui| {
                perilla(ui, t("Volumen"), &mut a.volumen, 0.0, 2.0, 1.0, ACENTO, 38.0);
                perilla(ui, t("Lado"), &mut a.pan, -1.0, 1.0, 0.0, ACENTO, 38.0);
                perilla(ui, t("Entrada s"), &mut a.entra, 0.0, 10.0, 0.0, ACENTO, 38.0);
                perilla(ui, t("Salida s"), &mut a.sale, 0.0, 10.0, 0.0, ACENTO, 38.0);
                perilla(ui, t("Velocidad"), &mut a.velocidad, 0.25, 4.0, 1.0, ACENTO, 38.0).on_hover_text(t("Más rápido suena más agudo"));
            });
            let (nueva, mezcla) = (a.velocidad, a.mezcla);
            if nueva != antes {
                // El clip sigue abarcando el mismo pedazo de audio.
                let c = &mut self.p.clips[i];
                c.dur = ((c.dur as f32 * antes / nueva).round() as u32).max(1);
            }
            ui.separator();
            ui.label(RichText::new(t("Mezcla en")).size(12.0).color(TENUE));
            ui.horizontal(|ui| {
                let nombre = self.p.mezcla[mezcla].nombre.clone();
                ComboBox::from_id_salt("pista_clip").selected_text(nombre).width(150.0).show_ui(ui, |ui| {
                    for k in 0..self.p.mezcla.len() {
                        let n = self.p.mezcla[k].nombre.clone();
                        if let Some(a) = &mut self.p.clips[i].audio {
                            ui.selectable_value(&mut a.mezcla, k, n);
                        }
                    }
                });
                if ui.button(t("Ver en el Mixer")).clicked() {
                    ver = Some(mezcla);
                }
            });
        });
        if let Some(k) = ver {
            self.pista = k;
            self.mostrar(Vista::Mezcla);
        }
    }

    /// Los efectos que pueden mover controles de otros.
    pub(super) fn controladores(&self) -> Vec<((usize, usize), String)> {
        let mut v = vec![];
        for (i, m) in self.p.mezcla.iter().enumerate() {
            for (k, e) in m.efectos.iter().enumerate().filter(|(_, e)| e.activo) {
                let Some(g) = self.bib.ver(&e.efecto) else { continue };
                if g.receta.as_ref().is_some_and(|r| r.nombres_medidas().any(|m| m == "control")) {
                    v.push(((i, k), format!("{} · {}", g.ficha.nombre.ver(), m.nombre)));
                }
            }
        }
        v
    }

    /// El `id` de un efecto; si no tenía, se le pone.
    pub(super) fn id_de(&mut self, (i, k): (usize, usize)) -> u64 {
        let nuevo = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
        let e = &mut self.p.mezcla[i].efectos[k];
        if e.id == 0 {
            e.id = nuevo;
        }
        e.id
    }

    /// Le pone un audio a un pad.
    pub fn poner_pad(&mut self, canal: usize, pad: usize, ruta: String) {
        let c = &mut self.p.canales[canal];
        c.pads.resize(crate::motor::PADS, String::new());
        c.pads[pad] = ruta;
    }

    /// Escribe en el patrón una nota por corte, donde cae cada uno en el audio.
    pub fn escribir_cortes(&mut self, canal: usize) {
        let c = self.p.canales[canal].clone();
        let Some(m) = c.muestra.as_ref().and_then(|r| self.bib.cortes(r, &c.controles)) else { return };
        let seg = c.muestra.as_ref().and_then(|r| self.bib.muestra(r, 0)).map_or(0.0, |m| m.segundos());
        let total = self.p.pulsos(seg);
        let Some(x) = self.p.patrones.iter_mut().find(|x| x.id == self.patron) else { return };
        let en = |f: f32| (f * total as f32).round() as u32;
        let notas = m.cortes.iter().enumerate().map(|(i, a)| {
            let fin = m.cortes.get(i + 1).map_or(total, |b| en(*b));
            Nota { tick: en(*a), dur: fin.saturating_sub(en(*a)).max(1), tono: 48 + i as u8, fuerza: 0.9 }
        });
        x.notas.insert(c.id, notas.collect());
        x.compases = x.compases.max(total.div_ceil(COMPAS));
        let nombre = x.nombre.clone();
        self.avisar(tf("{} cortes escritos en «{}»", &[&m.cortes.len(), &nombre]), false);
    }

    pub(super) fn ver_canal(&mut self, ui: &mut Ui) {
        if let Some(i) = self.clip.filter(|_| self.clip_audio().is_some()) {
            return self.ver_clip(ui, i);
        }
        let Some(k) = self.p.canales.iter().position(|c| c.id == self.canal) else { return };
        let (mut edita, mut abrir) = (None, None);
        let clave = self.p.canales[k].instrumento.clone();
        self.lienzo(&clave);
        let mut momento = self.momento(ui, self.p.canales[k].pista);
        if let Some(n) = self.p.canales[k].notas.clone() {
            self.lienzo(&n);
        }
        if let Some(r) = self.p.canales[k].muestra.clone() {
            momento.archivo = Path::new(&r).file_name().unwrap_or_default().to_string_lossy().into();
            if self.lienzo(&clave).is_some() {
                momento.huella = self.bib.muestra(&r, self.p.canales[k].preset).map_or(vec![], |m| picos(&m));
                if self.bib.ver(&clave).is_some_and(|g| g.ficha.muestras == crate::lua::Muestras::Cortes) {
                    let c = self.p.canales[k].controles.clone();
                    momento.cortes = self.bib.cortes(&r, &c).map_or(vec![], |m| m.cortes.clone());
                }
            }
        }
        let mut ent = lienzo::Entorno {
            pads: self.p.canales[k].pads.clone(),
            controladores: self.controladores(),
            vinculados: self.p.canales[k].vinculos.keys().cloned().collect(),
            ..Default::default()
        };
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            let c = &mut self.p.canales[k];
            let col = color(c.color);
            ui.horizontal(|ui| {
                ui.color_edit_button_srgb(&mut c.color);
                ui.add(TextEdit::singleline(&mut c.nombre).font(letra(17.0)).desired_width(f32::INFINITY));
            });
            ui.horizontal(|ui| {
                perilla(ui, t("Volumen"), &mut c.volumen, 0.0, 1.2, 0.8, ACENTO, 38.0);
                perilla(ui, t("Lado"), &mut c.pan, -1.0, 1.0, 0.0, ACENTO, 38.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(t("Mezcla en")).size(11.0).color(TENUE));
                    let n = self.p.mezcla[c.pista].nombre.clone();
                    ComboBox::from_id_salt("pista").selected_text(n).width(110.0).show_ui(ui, |ui| {
                        for (i, m) in self.p.mezcla.iter().enumerate() {
                            ui.selectable_value(&mut c.pista, i, &m.nombre);
                        }
                    });
                });
            });
            ui.separator();
            if let Some(x) = &c.plugin {
                ui.label(RichText::new(&x.nombre).color(col));
                match self.bib.rack.ver(x.uid) {
                    Some(p) => {
                        if ui.button(t("Abrir ventana")).clicked() {
                            abrir = Some(x.uid);
                        }
                        controles_plugin(ui, p);
                    }
                    None => _ = ui.label(RichText::new(t("No se pudo cargar")).color(ROJO)),
                }
                return;
            }
            let nombre = self.bib.nombre(&c.instrumento);
            ui.horizontal(|ui| {
                ComboBox::from_id_salt("instrumento").selected_text(nombre).width(150.0).show_ui(ui, |ui| {
                    for (fam, v) in crate::banco::familias() {
                        ui.label(RichText::new(t(fam)).size(11.0).color(TENUE));
                        for x in v {
                            ui.selectable_value(&mut c.instrumento, x.clave.to_string(), t(x.nombre));
                        }
                    }
                    for (g, v) in self.bib.grupos(Tipo::Instrumento) {
                        ui.label(RichText::new(g).size(11.0).color(TENUE));
                        for x in v {
                            ui.selectable_value(&mut c.instrumento, x.clave.clone(), x.ficha.nombre.ver());
                        }
                    }
                });
                // Los grabados no tienen guion que editar.
                if self.bib.del_banco(&c.instrumento).is_none() && ui.button("Lua").on_hover_text(t("Editar en Lua")).clicked() {
                    edita = Some(c.instrumento.clone());
                }
            });
            if let Some(g) = self.bib.sonido(&c.instrumento) {
                if !g.ficha.presets.is_empty() {
                    ui.menu_button(t("Preajustes"), |ui| {
                        for (nombre, v) in &g.ficha.presets {
                            if ui.button(nombre.ver()).clicked() {
                                c.controles = v.clone();
                                ui.close();
                            }
                        }
                    });
                }
                match self.lienzos.get_mut(&g.clave) {
                    // Entero en lo que se ve; en ventanas muy chaparras, lo que sobra se recorre.
                    Some((_, Some(it))) => {
                        let ancho = ui.available_width().max(220.0);
                        let alto = (ui.clip_rect().bottom() - ui.cursor().top() - 8.0).max(230.0);
                        let tam = vec2(it.ancho, it.alto) * (ancho / it.ancho).min(alto / it.alto);
                        ui.allocate_ui(tam, |ui| {
                            ui.set_min_size(tam);
                            {
                                ent.datos = Some(c.datos.clone());
                                lienzo::pintar(ui, it, &g.ficha, &mut c.controles, momento.clone(), &mut ent);
                                if let Some(d) = ent.datos.take() {
                                    c.datos = d;
                                }
                            }
                        });
                    }
                    _ => controles(ui, &g.ficha, &mut c.controles, ACENTO),
                }
            }
            if c.muestra.is_some()
                || self
                    .bib
                    .ver(&c.instrumento)
                    .is_some_and(|g| g.ficha.muestras == crate::lua::Muestras::Archivo && g.receta.as_ref().is_some_and(|r| r.usa_muestra()))
            {
                ui.separator();
                let n = c
                    .muestra
                    .as_ref()
                    .map_or(t("Ningún archivo").into(), |r| Path::new(r).file_name().unwrap_or_default().to_string_lossy().to_string());
                ui.add(Label::new(RichText::new(n).color(TENUE)).truncate());
                if ui.button(t("Elegir audio, SF2 o SFZ…")).clicked()
                    && let Some(r) = rfd::FileDialog::new()
                        .add_filter("Audio, SF2, SF3, SFZ", &[crate::decodificar::EXTENSIONES, &["sf2", "sf3", "sfz"]].concat())
                        .pick_file()
                {
                    c.muestra = Some(r.to_string_lossy().into());
                    c.preset = 0;
                }
                if let Some(r) = c.muestra.clone().filter(|r| {
                    let r = r.to_lowercase();
                    r.ends_with(".sf2") || r.ends_with(".sf3")
                }) {
                    let v = self.presets.entry(r.clone()).or_insert_with(|| crate::muestrario::presets(Path::new(&r)));
                    let actual = v.get(c.preset).cloned().unwrap_or_default();
                    ComboBox::from_id_salt("preset").selected_text(actual).width(200.0).show_ui(ui, |ui| {
                        for (i, n) in v.iter().enumerate() {
                            ui.selectable_value(&mut c.preset, i, n);
                        }
                    });
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(RichText::new(t("Notas en vivo")).size(11.0).color(TENUE));
                let actual = c.notas.as_ref().map_or(t("Nada").to_string(), |k| self.bib.nombre(k));
                ComboBox::from_id_salt("notas-en-vivo").selected_text(actual).width(170.0).show_ui(ui, |ui| {
                    ui.selectable_value(&mut c.notas, None, t("Nada"));
                    for g in self.bib.de_tipo(Tipo::Notas) {
                        ui.selectable_value(&mut c.notas, Some(g.clave.clone()), g.ficha.nombre.ver());
                    }
                });
            });
            if let Some(g) = c.notas.as_ref().and_then(|k| self.bib.ver(k)) {
                match self.lienzos.get_mut(&g.clave) {
                    Some((_, Some(it))) => {
                        let ancho = ui.available_width().clamp(220.0, 520.0);
                        let tam = vec2(ancho, ancho * it.alto / it.ancho);
                        ui.allocate_ui(tam, |ui| {
                            ui.set_min_size(tam);
                            lienzo::pintar(ui, it, &g.ficha, &mut c.notas_controles, Default::default(), &mut Default::default())
                        });
                    }
                    _ => controles(ui, &g.ficha, &mut c.notas_controles, ACENTO),
                }
            }
            ui.separator();
            if let Some(m) = self.teclados.as_ref().filter(|m| !m.nombres.is_empty()) {
                ui.add(Label::new(RichText::new(format!("MIDI: {}", m.nombres.join(", "))).size(12.0).color(VERDE)).truncate());
            }
            ui.label(RichText::new(t("Octava del teclado")).size(11.0).color(TENUE));
            ui.add(Slider::new(&mut self.octava, 2..=8));
        });
        for (tono, si) in std::mem::take(&mut ent.toca) {
            if si { self.sonar(self.canal, tono, 0.9) } else { self.callar(self.canal, tono) }
        }
        let audio = |ui: &Ui| {
            let _ = ui;
            rfd::FileDialog::new().add_filter("Audio", crate::decodificar::EXTENSIONES).pick_file().map(|r| r.to_string_lossy().to_string())
        };
        if let Some(i) = ent.pad_archivo
            && let Some(r) = audio(ui)
        {
            self.poner_pad(k, i, r);
        }
        if ent.elegir
            && let Some(r) = audio(ui)
        {
            (self.p.canales[k].muestra, self.p.canales[k].preset) = (Some(r), 0);
        }
        if ent.quitar {
            self.p.canales[k].muestra = None;
        }
        if ent.escribir {
            self.escribir_cortes(k);
        }
        if let Some((c, de)) = ent.vincular.take() {
            let id = de.map(|d| self.id_de(d));
            let v = &mut self.p.canales[k].vinculos;
            match id {
                Some(id) => _ = v.insert(c, id),
                None => _ = v.remove(&c),
            }
        }
        if let Some(x) = edita {
            self.editar(&x);
        }
        if let Some(u) = abrir {
            self.ventana(u);
        }
    }

    pub(super) fn ver_editor(&mut self, ui: &mut Ui) {
        let Some(mut e) = self.editor.take() else { return };
        let mut abierta = true;
        let (mut guardar, mut probar) = (false, false);
        Window::new(format!("{}.lua", e.clave)).id(Id::new("editor")).open(&mut abierta).default_size([620.0, 460.0]).show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                guardar = ui.button(RichText::new(t("Guardar y oír")).color(ACENTO)).clicked();
                probar = ui.button(t("Revisar")).clicked();
                if ui.button(t("Abrir su carpeta")).on_hover_text(t("Pon ahí sus imágenes PNG y otros .lua; al guardar se vuelven a leer")).clicked()
                {
                    match self.bib.carpeta_de(&e.clave) {
                        Some(d) => mostrar_carpeta(&d),
                        None => e.error = Some(t("No hay carpeta para guardar guiones").into()),
                    }
                }
                ui.label(RichText::new(t("El original de fábrica no se pierde: borra tu carpeta y regresa")).size(11.0).color(TENUE));
            });
            if let Some(x) = &e.error {
                ui.label(RichText::new(x).color(if x == t("Todo bien") { VERDE } else { ROJO }));
            }
            ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                ui.add(TextEdit::multiline(&mut e.texto).code_editor().desired_width(f32::INFINITY).desired_rows(26));
            });
        });
        if probar {
            let archivos = self.bib.ver(&e.clave).map(|g| g.archivos.clone()).unwrap_or_default();
            e.error = Some(match crate::instalar::revisar(&e.texto, &archivos, self.bib.sr) {
                Ok((_, inf)) if inf.avisos.is_empty() => t("Todo bien").into(),
                Ok((_, inf)) => inf.avisos.join(" · "),
                Err(x) => x,
            });
        }
        if guardar {
            match self.bib.guardar(&e.clave, &e.texto) {
                Ok(()) => {
                    self.cambio_bib += 1;
                    e.error = Some(t("Todo bien").into());
                    if e.canal.take().is_some() {
                        self.poner_instrumento(&e.clave.clone());
                    }
                    self.sonar(self.canal, 60, 0.9);
                    self.callar(self.canal, 60);
                }
                Err(x) => e.error = Some(x),
            }
        }
        if abierta {
            self.editor = Some(e);
        }
    }
}
