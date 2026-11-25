// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! La ventana.
mod arreglo;
pub mod asistente;
mod ecualizador;
pub mod lado;
mod lienzo;
mod metal;
mod mezcla;
mod notas;
mod piel;
mod ritmo;

use crate::biblioteca::Biblioteca;
use crate::idioma::{self, Idioma, t, tf};
use crate::lua::Tipo;
use crate::modelo::*;
use crate::motor::{Escena, Estado, Mando, Modo, Motor, Orden, a_audio};
use crate::plugin::{self, Oferta, ventana::Ventana};
use crate::{audio, flp, midi, wav};
use eframe::egui::{self, *};
use metal::*;
use piel::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vista {
    Arreglo,
    Ritmo,
    Notas,
    Mezcla,
    Canal,
    Asistente,
}

const VISTAS: usize = 6;

/// De fábrica: Playlist y Mixer son pestañas; lo demás, ventanas encima.
const FIJAS: [bool; VISTAS] = [true, false, false, true, false, false];

impl Vista {
    pub const TODAS: [Vista; VISTAS] = [Vista::Arreglo, Vista::Ritmo, Vista::Notas, Vista::Mezcla, Vista::Canal, Vista::Asistente];

    fn nombre(self) -> &'static str {
        match self {
            Vista::Arreglo => "Playlist",
            Vista::Ritmo => "Ritmo",
            Vista::Notas => "Piano roll",
            Vista::Mezcla => "Mixer",
            Vista::Canal => "Instrumento",
            Vista::Asistente => "Asistente",
        }
    }

    fn tecla(self) -> (Key, &'static str) {
        match self {
            Vista::Arreglo => (Key::F5, "F5"),
            Vista::Ritmo => (Key::F6, "F6"),
            Vista::Notas => (Key::F7, "F7"),
            Vista::Mezcla => (Key::F9, "F9"),
            Vista::Canal => (Key::F8, "F8"),
            Vista::Asistente => (Key::F10, "F10"),
        }
    }

    /// Su lugar al acomodar todo.
    fn lugar(self, z: Rect) -> Rect {
        let (w, h) = (z.width(), z.height());
        let r = |x: f32, y: f32, a: f32, b: f32| Rect::from_min_size(z.min + vec2(w * x, h * y), vec2(w * a, h * b)).shrink(3.0);
        match self {
            Vista::Arreglo => z,
            Vista::Ritmo => r(0.0, 0.5, 0.64, 0.5),
            Vista::Canal => r(0.64, 0.5, 0.36, 0.5),
            Vista::Notas => r(0.06, 0.14, 0.88, 0.84),
            Vista::Mezcla => r(0.04, 0.2, 0.92, 0.78),
            Vista::Asistente => r(0.62, 0.02, 0.38, 0.96),
        }
    }
}

pub struct App {
    pub p: Proyecto,
    pub bib: Biblioteca,
    /// Qué ventanas están abiertas, cuál se ve completa y cuál tiene el teclado.
    abiertas: [bool; VISTAS],
    /// Cuáles son pestañas en vez de ventanas, y cuál pestaña se ve. La playlist siempre es el fondo.
    fijas: [bool; VISTAS],
    pub pestana: Vista,
    completa: Option<Vista>,
    pub activa: Vista,
    /// Cambia al acomodar: las ventanas vuelven a su lugar.
    vez: u32,
    metronomo: bool,
    cabezal: f64,
    /// Los saltos que el motor debe haber hecho, y desde cuándo se espera.
    salto: Option<(u32, std::time::Instant)>,
    biblioteca_abierta: bool,
    /// Los teclados MIDI, a qué canal tocan y lo que llega de ellos.
    teclados: Option<crate::teclados::Teclados>,
    canal_midi: Arc<std::sync::atomic::AtomicU32>,
    toques: crossbeam_channel::Receiver<crate::teclados::Toque>,
    anotar: bool,
    anotando: BTreeMap<u8, (u32, u32, f32)>,
    grabando: Option<(audio::Entrada, u32)>,
    asistente: asistente::Asistente,
    prueba: bool,
    /// Las interfaces en Lua ya cargadas, por guion. `None`: ese guion no trae.
    lienzos: BTreeMap<String, (u64, Option<crate::interfaz::Interfaz>)>,
    /// El cuadro para elegir instrumento.
    pub elegir: Option<lado::Elegir>,
    /// El clip elegido en la playlist.
    pub clip: Option<usize>,
    /// Las ventanas de efectos abiertas: pista y ranura.
    pub fx_abiertos: Vec<(usize, usize)>,
    fx_frente: Option<(usize, usize)>,
    al_frente: Option<Vista>,
    capas: [Option<LayerId>; VISTAS],
    /// El área de las pestañas.
    fondo: Rect,
    /// Dónde quedó cada tira del Mixer.
    tiras: Vec<(usize, Rect)>,
    altos: [f32; PISTAS_MEZCLA],
    pub canal: u32,
    pub patron: u32,
    pub cancion: bool,
    pista: usize,
    archivo: Option<PathBuf>,
    guardado: Proyecto,
    // Historial: el último estado firme y lo hecho y deshecho.
    firme: Proyecto,
    hecho: Vec<Proyecto>,
    deshecho: Vec<Proyecto>,
    // Lo último que se le mandó al motor.
    enviado: Option<(Proyecto, Modo, u64)>,
    /// Sube cuando cambia la biblioteca.
    cambio_bib: u64,
    mando: Mando,
    estado: Arc<Estado>,
    _salida: Option<audio::Salida>,
    /// Sin salida de audio el motor se queda aquí.
    _motor: Option<Motor>,
    picos: [[f32; 2]; PISTAS_MEZCLA],
    avisos: Vec<(String, bool, f64)>,
    pista_ayuda: String,
    /// La del cuadro anterior: la barra se dibuja antes que lo demás.
    pista_ayuda_vista: String,
    teclas: BTreeMap<Key, (u32, u8)>,
    octava: u8,
    ventanas: BTreeMap<u64, Ventana>,
    plugins: Vec<PathBuf>,
    ofertas: Option<(Vec<Oferta>, Option<usize>)>,
    presets: BTreeMap<String, Vec<String>>,
    editor: Option<lado::Editor>,
    buscar: String,
    pub notas: notas::Notas,
    pub arreglo: arreglo::Arreglo,
    ritmo: ritmo::Ritmo,
    carpeta: Option<PathBuf>,
    cerrar: bool,
    /// Lo que se hará al descartar los cambios sin guardar.
    pendiente: Option<Pendiente>,
}

#[derive(Clone, Copy, PartialEq)]
enum Pendiente {
    Nuevo,
    Abrir,
    Cerrar,
}

/// La carpeta del usuario.
pub fn carpeta() -> PathBuf {
    let casa = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    casa.unwrap_or_else(std::env::temp_dir).join("Music").join("Beethoven DAW Demo")
}

fn ajustes(dir: &Path) -> PathBuf {
    dir.join("ajustes.json")
}

/// El idioma: lo que diga quien abrió el programa, lo elegido antes o el del sistema.
pub fn poner_idioma(dir: Option<&Path>) {
    let pedido = std::env::var("BEETHOVEN_IDIOMA").ok().and_then(|s| Idioma::de_codigo(&s));
    let elegido = || {
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(ajustes(dir?)).ok()?).ok()?;
        Idioma::de_codigo(v["idioma"].as_str()?)
    };
    idioma::poner(pedido.or_else(elegido).unwrap_or_else(Idioma::del_sistema));
}

impl App {
    /// `prueba`: sin audio y sin leer ni escribir la carpeta del usuario.
    pub fn nueva(ctx: &Context, prueba: bool) -> App {
        vestir(ctx);
        let carpeta = (!prueba).then(carpeta);
        if !prueba {
            poner_idioma(carpeta.as_deref());
        }
        let sr = if prueba { 48000.0 } else { audio::frecuencia().unwrap_or(48000.0) };
        let mut bib = Biblioteca::nueva(sr, carpeta.as_ref().map(|c| c.join("Lua")));
        // Las pruebas usan la canción de muestra; el programa abre vacío.
        let p = if prueba { Proyecto::demo() } else { Proyecto::default() };
        let (motor, mando) = Motor::nuevo(Escena::nueva(&p, &mut bib, Modo::Cancion));
        let estado = mando.estado.clone();
        let (manda, toques) = crossbeam_channel::bounded(256);
        let canal_midi = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let teclados = (!prueba).then(|| crate::teclados::abrir(canal_midi.clone(), mando.ordenes(), manda));
        let mut avisos: Vec<(String, bool, f64)> = bib.avisos.drain(..).map(|a| (a, true, 12.0)).collect();
        let (mut salida, mut guardado_motor) = (None, None);
        if prueba {
            guardado_motor = Some(motor);
        } else {
            match audio::abrir(motor) {
                Ok(s) => salida = Some(s),
                Err(e) => avisos.push((tf("Sin sonido: {}", &[&e]), true, 20.0)),
            }
        }
        App {
            canal: p.canales.first().map_or(0, |c| c.id),
            patron: p.patrones[0].id,
            abiertas: [false; VISTAS],
            fijas: FIJAS,
            pestana: Vista::Arreglo,
            completa: None,
            activa: Vista::Arreglo,
            vez: 0,
            metronomo: false,
            cabezal: 0.0,
            salto: None,
            biblioteca_abierta: true,
            teclados,
            canal_midi,
            toques,
            anotar: false,
            anotando: BTreeMap::new(),
            grabando: None,
            asistente: asistente::Asistente::nuevo(ctx, carpeta.as_ref().map(|c| c.join("asistente.json"))),
            prueba,
            lienzos: BTreeMap::new(),
            elegir: None,
            clip: None,
            fx_abiertos: vec![],
            fx_frente: None,
            al_frente: None,
            capas: [None; VISTAS],
            fondo: Rect::NOTHING,
            tiras: vec![],
            altos: [0.0; PISTAS_MEZCLA],
            cancion: !p.clips.is_empty(),
            pista: 1,
            archivo: None,
            guardado: p.clone(),
            firme: p.clone(),
            hecho: vec![],
            deshecho: vec![],
            enviado: None,
            cambio_bib: 0,
            mando,
            estado,
            _salida: salida,
            _motor: guardado_motor,
            picos: Default::default(),
            avisos,
            pista_ayuda: String::new(),
            pista_ayuda_vista: String::new(),
            teclas: BTreeMap::new(),
            octava: 5,
            ventanas: BTreeMap::new(),
            plugins: if prueba { vec![] } else { plugin::buscar() },
            ofertas: None,
            presets: BTreeMap::new(),
            editor: None,
            buscar: String::new(),
            notas: Default::default(),
            arreglo: Default::default(),
            ritmo: Default::default(),
            carpeta,
            cerrar: false,
            pendiente: None,
            p,
            bib,
        }
    }

    pub fn avisar(&mut self, s: impl Into<String>, malo: bool) {
        self.avisos.push((s.into(), malo, if malo { 12.0 } else { 4.0 }));
    }

    /// Texto de ayuda de abajo: qué hace el ratón aquí.
    fn ayuda(&mut self, s: &str) {
        self.pista_ayuda = t(s).into();
    }

    fn modo(&self) -> Modo {
        if self.cancion { Modo::Cancion } else { Modo::Patron(self.patron) }
    }

    pub fn tocando(&self) -> bool {
        self.estado.tocando.load(Relaxed)
    }

    fn tick(&self) -> f32 {
        self.cabezal as f32
    }

    /// Lleva el cabezal al lugar que se oye, sin brincos: entre bloque y bloque de audio avanza con el reloj.
    fn mover_cabezal(&mut self, dt: f64) {
        // Recién saltó: se queda donde se le dijo hasta que el motor llegue.
        if let Some((saltos, cuando)) = self.salto {
            if self.estado.saltos.load(Relaxed) < saltos && cuando.elapsed().as_millis() < 300 {
                return;
            }
            self.salto = None;
        }
        let paso = self.p.bpm as f64 * PPQ as f64 / 60.0;
        let mut meta = self.estado.tick() as f64;
        if !self.tocando() {
            self.cabezal = meta;
            return;
        }
        let largo = match self.modo() {
            Modo::Cancion => self.p.largo(),
            Modo::Patron(i) => self.p.patron(i).map_or(COMPAS, |x| x.largo()),
        } as f64;
        meta += self.estado.hace().min(0.2) * paso;
        if meta >= largo {
            meta -= largo;
        }
        self.cabezal += dt * paso;
        let falta = meta - self.cabezal;
        // Lejos (dio la vuelta o empezó): va directo. Cerca: se corrige de a poco.
        self.cabezal = if falta.abs() > paso * 0.15 { meta } else { self.cabezal + falta * (1.0 - (-dt * 6.0).exp()) };
    }

    pub fn tocar(&mut self, si: bool) {
        self.sincronizar();
        self.mando.mandar(Orden::Tocar(si));
    }

    fn ir(&mut self, tick: f64) {
        // El motor tarda un bloque en llegar; mientras, el cabezal ya está donde se puso.
        let faltan = self.salto.map_or(self.estado.saltos.load(Relaxed), |s| s.0.max(self.estado.saltos.load(Relaxed)));
        if self.mando.mandar(Orden::Ir(tick)) {
            self.salto = Some((faltan + 1, std::time::Instant::now()));
            self.cabezal = tick.max(0.0);
        }
    }

    fn sonar(&mut self, canal: u32, tono: u8, fuerza: f32) {
        self.sincronizar();
        self.mando.mandar(Orden::Nota { canal, tono, fuerza });
    }

    fn callar(&mut self, canal: u32, tono: u8) {
        self.mando.mandar(Orden::Soltar { canal, tono });
    }

    /// Le manda al motor el proyecto si cambió.
    pub fn sincronizar(&mut self) {
        let modo = self.modo();
        if self.enviado.as_ref().is_some_and(|(p, m, b)| *p == self.p && *m == modo && *b == self.cambio_bib) {
            return;
        }
        let e = Box::new(Escena::nueva(&self.p, &mut self.bib, modo));
        if self.mando.mandar(Orden::Escena(e)) {
            self.enviado = Some((self.p.clone(), modo, self.cambio_bib));
        }
        let nuevos: Vec<String> = self.bib.avisos.drain(..).collect();
        nuevos.into_iter().for_each(|a| self.avisar(a, true));
    }

    /// Anota un paso del historial cuando se suelta el ratón.
    fn historial(&mut self, ui: &Ui) {
        if self.p != self.firme && !ui.input(|i| i.pointer.any_down()) {
            self.hecho.push(std::mem::replace(&mut self.firme, self.p.clone()));
            if self.hecho.len() > 200 {
                self.hecho.remove(0);
            }
            self.deshecho.clear();
        }
    }

    pub fn deshacer(&mut self) {
        if let Some(x) = self.hecho.pop() {
            self.deshecho.push(std::mem::replace(&mut self.firme, x));
            self.p = self.firme.clone();
            self.revisar();
        }
    }

    pub fn rehacer(&mut self) {
        if let Some(x) = self.deshecho.pop() {
            self.hecho.push(std::mem::replace(&mut self.firme, x));
            self.p = self.firme.clone();
            self.revisar();
        }
    }

    /// Que lo elegido exista.
    fn revisar(&mut self) {
        if !self.p.canales.iter().any(|c| c.id == self.canal) {
            self.canal = self.p.canales.first().map_or(0, |c| c.id);
        }
        if self.p.patron(self.patron).is_none() {
            self.patron = self.p.patrones[0].id;
        }
        self.notas.soltar();
        self.arreglo.soltar();
        if self.clip.is_some_and(|i| i >= self.p.clips.len()) {
            self.clip = None;
        }
    }

    fn cambiar_proyecto(&mut self, p: Proyecto, archivo: Option<PathBuf>) {
        self.mando.mandar(Orden::Tocar(false));
        self.mando.mandar(Orden::Silencio);
        self.ventanas.clear();
        self.fx_abiertos.clear();
        self.clip = None;
        self.p = p.clone();
        self.firme = p.clone();
        self.guardado = p;
        self.hecho.clear();
        self.deshecho.clear();
        self.archivo = archivo;
        self.bib.rack = Default::default();
        self.cambio_bib += 1;
        self.patron = self.p.patrones[0].id;
        self.canal = self.p.canales.first().map_or(0, |c| c.id);
        self.cancion = !self.p.clips.is_empty();
        self.revisar();
        self.sincronizar();
        self.ir(0.0);
        let malos: Vec<String> = self
            .plugins_puestos()
            .into_iter()
            .filter_map(|x| self.bib.rack.dar(&x, self.bib.sr).err().map(|e| tf("No se pudo cargar «{}»: {}", &[&x.nombre, &e])))
            .collect();
        malos.into_iter().for_each(|a| self.avisar(a, true));
    }

    fn plugins_puestos(&self) -> Vec<PluginPuesto> {
        let c = self.p.canales.iter().filter_map(|c| c.plugin.clone());
        c.chain(self.p.mezcla.iter().flat_map(|m| m.efectos.iter().filter_map(|e| e.plugin.clone()))).collect()
    }

    /// El proyecto con el estado actual de sus plugins.
    fn completo(&self) -> Proyecto {
        let mut p = self.p.clone();
        let c = p.canales.iter_mut().filter_map(|c| c.plugin.as_mut());
        for x in c.chain(p.mezcla.iter_mut().flat_map(|m| m.efectos.iter_mut().filter_map(|e| e.plugin.as_mut()))) {
            if let Some(e) = self.bib.rack.estado(x.uid) {
                x.estado = e;
            }
        }
        p
    }

    fn escribir(&mut self, r: &Path) -> Result<(), String> {
        // Primero a un lado: si algo falla, el archivo anterior queda entero.
        let parcial = r.with_extension("parcial");
        let b = serde_json::to_vec_pretty(&self.completo()).map_err(|e| e.to_string())?;
        std::fs::write(&parcial, b).and_then(|_| std::fs::rename(&parcial, r)).map_err(|e| e.to_string())
    }

    pub fn guardar(&mut self, otro: bool) -> bool {
        let r = match self.archivo.clone().filter(|_| !otro) {
            Some(r) => r,
            None => {
                let d = rfd::FileDialog::new().add_filter("Beethoven DAW Demo", &["bdaw"]).set_file_name(format!("{}.bdaw", self.p.nombre));
                let Some(r) = d.save_file() else { return false };
                r
            }
        };
        if let Some(n) = r.file_stem() {
            self.p.nombre = n.to_string_lossy().into();
        }
        match self.escribir(&r) {
            Ok(()) => {
                self.guardado = self.p.clone();
                self.archivo = Some(r);
                self.avisar(t("Guardado"), false);
                true
            }
            Err(e) => {
                self.avisar(tf("No se pudo guardar: {}", &[&e]), true);
                false
            }
        }
    }

    /// Abre cualquier archivo que el programa entienda.
    pub fn abrir(&mut self, r: &Path) {
        let ext = r.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let nombre = r.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let res: Result<(), String> = (|| {
            let leer = || std::fs::read(r).map_err(|e| e.to_string());
            match ext.as_str() {
                "bdaw" => {
                    let p = Proyecto::de_json(&String::from_utf8_lossy(&leer()?))?;
                    self.cambiar_proyecto(p, Some(r.into()));
                }
                "flp" => {
                    let (p, avisos) = flp::leer(&leer()?)?;
                    self.cambiar_proyecto(p, None);
                    avisos.into_iter().for_each(|a| self.avisar(a, true));
                }
                "mid" | "midi" => {
                    self.patron = midi::importar(&mut self.p, &leer()?)?;
                    self.canal = self.p.canales.last().map_or(0, |c| c.id);
                    self.cancion = false;
                    self.mostrar(Vista::Ritmo);
                }
                x if crate::decodificar::EXTENSIONES.contains(&x) => self.poner_audio(r, None, self.donde(), None)?,
                "sf2" | "sf3" | "sfz" => {
                    self.poner_muestra(r, &nombre)?;
                    self.p.canales.last_mut().unwrap().controles.insert("baja".into(), 0.3);
                    self.mostrar(Vista::Canal);
                }
                "lua" => {
                    let f = std::fs::read_to_string(r).map_err(|e| e.to_string())?;
                    self.bib.guardar(&nombre, &f)?;
                    self.cambio_bib += 1;
                    self.avisar(tf("«{}» quedó en la biblioteca", &[&nombre]), false);
                }
                "vst3" | "clap" => self.ofrecer(r, None),
                _ => return Err(t("No conozco ese tipo de archivo").into()),
            }
            Ok(())
        })();
        if let Err(e) = res {
            self.avisar(format!("{nombre}: {e}"), true);
        }
    }

    fn pegar(&mut self) -> bool {
        let v = arboard::Clipboard::new().and_then(|mut c| c.get().file_list()).unwrap_or_default();
        v.iter().for_each(|r| self.abrir(r));
        !v.is_empty()
    }

    /// Carga el archivo de un plugin. `pista`: como efecto en esa pista; si no, como instrumento.
    fn ofrecer(&mut self, r: &Path, pista: Option<usize>) {
        // Un plugin ajeno puede tronar: antes se guarda una copia.
        if let Some(d) = self.carpeta.clone() {
            let _ = std::fs::create_dir_all(&d);
            let _ = self.escribir(&d.join("respaldo.bdaw"));
        }
        match plugin::ofertas(r) {
            Ok(v) if v.is_empty() => self.avisar(t("El archivo no trae plugins"), true),
            Ok(v) if v.len() == 1 => self.poner_plugin(&v[0], pista),
            Ok(v) => self.ofertas = Some((v, pista)),
            Err(e) => self.avisar(tf("No se pudo cargar «{}»: {}", &[&r.file_stem().unwrap_or_default().to_string_lossy(), &e]), true),
        }
    }

    pub fn poner_plugin(&mut self, o: &Oferta, pista: Option<usize>) {
        let uid = self.plugins_puestos().iter().map(|x| x.uid).max().unwrap_or(0).max(self.hecho.len() as u64)
            + std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_micros() as u64);
        let x = PluginPuesto { uid, ruta: o.ruta.clone(), id: o.id.clone(), nombre: o.nombre.clone(), estado: String::new() };
        if let Err(e) = self.bib.rack.dar(&x, self.bib.sr) {
            return self.avisar(tf("No se pudo cargar «{}»: {}", &[&o.nombre, &e]), true);
        }
        if o.instrumento {
            self.canal = self.p.agregar_canal("clasico", &o.nombre);
            let pista = self.p.pista_para(&o.nombre);
            let c = self.p.canales.last_mut().unwrap();
            (c.plugin, c.pista) = (Some(x), pista);
            self.clip = None;
        } else {
            let i = pista.unwrap_or(self.pista);
            self.p.mezcla[i].efectos.push(EfectoPuesto { plugin: Some(x), ..EfectoPuesto::nuevo("") });
            self.pista = i;
            self.avisar(tf("«{}» es un efecto: se puso en «{}»", &[&o.nombre, &self.p.mezcla[i].nombre]), false);
        }
    }

    /// Abre o trae al frente la ventana del plugin.
    fn ventana(&mut self, uid: u64) {
        if let Some(v) = self.ventanas.get(&uid) {
            return v.al_frente();
        }
        let Some(p) = self.bib.rack.ver(uid).cloned() else { return };
        match Ventana::abrir(&p) {
            Some(v) => _ = self.ventanas.insert(uid, v),
            None => self.avisar(t("Este plugin no tiene ventana aquí; usa sus controles"), true),
        }
    }

    /// El logo de un guion (`logo = "logo.png"` en su carpeta), ya en la tarjeta.
    pub(super) fn logo_de(&self, ctx: &Context, clave: &str) -> Option<TextureHandle> {
        let g = self.bib.ver(clave)?;
        lienzo::textura(ctx, &g.archivos, g.ficha.logo.as_deref()?)
    }

    /// Un logo chiquito en un recuadro hundido (listas, títulos).
    pub(super) fn pinta_logo(p: &Painter, r: Rect, t: &TextureHandle) {
        hueco(p, r.expand(1.0), 4, HUECO);
        p.image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }

    /// La interfaz en Lua de un guion, si trae. Se vuelve a cargar cuando cambia la biblioteca.
    fn lienzo(&mut self, clave: &str) -> Option<&mut crate::interfaz::Interfaz> {
        let viejo = self.lienzos.get(clave).is_none_or(|x| x.0 != self.cambio_bib);
        if viejo {
            let i = self.bib.ver(clave).and_then(|g| crate::interfaz::cargar_con(&g.fuente, g.archivos.clone()).ok().flatten());
            self.lienzos.insert(clave.into(), (self.cambio_bib, i));
        }
        self.lienzos.get_mut(clave)?.1.as_mut()
    }

    fn momento(&self, ui: &Ui, pista: usize) -> crate::interfaz::Momento {
        let nivel = self.picos[pista][0].max(self.picos[pista][1]);
        crate::interfaz::Momento { tiempo: ui.input(|i| i.time), nivel, tocando: self.tocando() || nivel > 1e-3, raton: None }
    }

    pub fn abrir_ventanas(&mut self) {
        self.plugins_puestos().iter().for_each(|x| self.ventana(x.uid));
    }

    /// Otra biblioteca: los plugins del proyecto se copian en vez de usar los que suenan en vivo.
    fn aparte(&self) -> Biblioteca {
        Biblioteca::nueva(self.bib.sr, self.bib.carpeta.clone())
    }

    fn exportar_audio(&mut self) {
        let mut d = rfd::FileDialog::new().set_file_name(format!("{}.wav", self.p.nombre));
        for e in ["wav", "mp3", "flac", "ogg", "m4a"] {
            d = d.add_filter(e.to_uppercase(), &[e]);
        }
        let Some(r) = d.save_file() else { return };
        self.tocar(false);
        let x = a_audio(&self.completo(), &mut self.aparte(), self.modo());
        match crate::decodificar::escribir(&r, self.bib.sr as u32, &x) {
            Ok(()) => self.avisar(tf("Exportado: {} s de audio", &[&(x.len() as u32 / self.bib.sr as u32)]), false),
            Err(e) => self.avisar(tf("No se pudo exportar: {}", &[&e]), true),
        }
    }

    /// El proyecto con una sola pista de mezcla sonando; nada si está callada o no tiene qué sonar.
    fn solo_pista(p: &Proyecto, i: usize) -> Option<Proyecto> {
        let mut q = p.clone();
        q.mezcla[..PRIMER_ENVIO].iter_mut().enumerate().for_each(|(k, m)| m.solo = k == i);
        // Lo que va derecho a la general no es de ninguna pista.
        q.canales.retain(|c| c.pista == i);
        q.clips.retain(|c| c.audio.as_ref().is_none_or(|a| a.mezcla == i));
        let hay = !q.canales.is_empty() || q.clips.iter().any(|c| c.audio.is_some());
        (hay && !q.mezcla[i].mudo).then_some(q)
    }

    /// Un WAV por cada pista de mezcla que suene.
    fn exportar_pistas(&mut self) {
        let Some(dir) = rfd::FileDialog::new().pick_folder() else { return };
        self.tocar(false);
        match pistas(&self.completo(), &mut self.aparte(), self.modo(), &dir) {
            Ok(n) => self.avisar(tf("Exportado: {} pistas", &[&n]), false),
            Err(e) => self.avisar(tf("No se pudo exportar: {}", &[&e]), true),
        }
    }

    fn exportar_midi(&mut self) {
        let d = rfd::FileDialog::new().add_filter("MIDI", &["mid"]).set_file_name(format!("{}.mid", self.p.nombre));
        let Some(r) = d.save_file() else { return };
        match std::fs::write(r, midi::exportar(&self.p, self.patron)) {
            Ok(()) => self.avisar(t("MIDI exportado"), false),
            Err(e) => self.avisar(tf("No se pudo exportar: {}", &[&e]), true),
        }
    }

    fn hacer(&mut self, x: Pendiente) {
        match x {
            Pendiente::Nuevo => {
                let mut p = Proyecto::default();
                for (i, n) in [("bombo", "Bombo"), ("caja", "Caja"), ("platillo", "Platillo"), ("piano", "Piano")] {
                    p.agregar_canal(i, t(n));
                }
                self.cambiar_proyecto(p, None);
                self.mostrar(Vista::Ritmo);
            }
            Pendiente::Abrir => {
                let d = rfd::FileDialog::new().add_filter("Beethoven DAW Demo / FLP", &["bdaw", "flp"]);
                if let Some(r) = d.pick_file() {
                    self.abrir(&r);
                }
            }
            Pendiente::Cerrar => self.cerrar = true,
        }
    }

    /// Pregunta antes si hay cambios sin guardar.
    fn pedir(&mut self, x: Pendiente) {
        if self.p != self.guardado { self.pendiente = Some(x) } else { self.hacer(x) }
    }

    fn cambiar_idioma(&mut self, i: Idioma) {
        idioma::poner(i);
        if let Some(d) = &self.carpeta {
            let _ = std::fs::create_dir_all(d);
            let _ = std::fs::write(ajustes(d), serde_json::json!({ "idioma": i.codigo() }).to_string());
        }
    }

    /// Va a una pestaña, o abre una ventana y la trae al frente.
    pub fn mostrar(&mut self, v: Vista) {
        self.activa = v;
        if self.fijas[v as usize] {
            self.pestana = v;
            return;
        }
        // Las ventanas viven encima de la playlist.
        self.pestana = Vista::Arreglo;
        self.abiertas[v as usize] = true;
        self.al_frente = Some(v);
        if self.completa.is_some_and(|c| c != v) {
            self.completa = None;
        }
    }

    pub fn abierta(&self, v: Vista) -> bool {
        if self.fijas[v as usize] { self.pestana == v } else { self.abiertas[v as usize] && self.pestana == Vista::Arreglo }
    }

    /// Como las teclas de FL: la abre, la trae al frente o la cierra si ya lo estaba.
    fn alternar(&mut self, v: Vista) {
        if v != Vista::Arreglo && self.abierta(v) && self.activa == v {
            self.cerrar_ventana(v);
        } else {
            self.mostrar(v);
        }
    }

    /// Al cerrar queda la playlist completa.
    fn cerrar_ventana(&mut self, v: Vista) {
        self.abiertas[v as usize] = false;
        if self.completa == Some(v) {
            self.completa = None;
        }
        if self.pestana == v {
            self.pestana = Vista::Arreglo;
        }
        // Las que de fábrica son pestañas vuelven a serlo.
        self.fijas[v as usize] = FIJAS[v as usize];
        self.activa = Vista::Arreglo;
    }

    /// Las pestañas, como de navegador, y a la derecha las ventanas.
    fn pestanas(&mut self, ui: &mut Ui) {
        let todo = ui.max_rect();
        degradado(ui.painter(), todo, 0, Color32::from_rgb(0x12, 0x13, 0x15), Color32::from_rgb(0x1a, 0x1b, 0x1e));
        let mut x = todo.left() + 6.0;
        for v in Vista::TODAS.into_iter().filter(|v| self.fijas[*v as usize]).collect::<Vec<_>>() {
            let r = Rect::from_min_max(pos2(x, todo.top() + 3.0), pos2(x + 168.0, todo.bottom()));
            x = r.right() + 3.0;
            let el = self.pestana == v;
            let b = ui.interact(r, Id::new(("pestana", v as u8)), Sense::click());
            let cr = CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 };
            let (a, z) = if el {
                (0x36, 0x1e)
            } else if b.hovered() {
                (0x2e, 0x1c)
            } else {
                (0x24, 0x17)
            };
            degradado_en(ui.painter(), r, cr, Color32::from_gray(a), Color32::from_gray(z));
            ui.painter().hline(r.left() + 6.0..=r.right() - 6.0, r.top() + 0.75, Stroke::new(1.0, Color32::from_white_alpha(38)));
            if el {
                ui.painter().hline(r.left() + 8.0..=r.right() - 8.0, r.top() + 1.0, Stroke::new(2.0, ACENTO));
            }
            rotulo(
                ui.painter(),
                Rect::from_min_max(r.min + vec2(14.0, 2.0), r.max - vec2(30.0, 0.0)),
                t(v.nombre()),
                15.5,
                if el { ACENTO } else { TEXTO },
            );
            b.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, el, t(v.nombre())));
            if v != Vista::Arreglo {
                let q = Rect::from_center_size(r.right_center() - vec2(16.0, -1.0), vec2(20.0, 18.0));
                let s = ui.interact(q, Id::new(("sacar", v as u8)), Sense::click()).on_hover_text(t("Sacar como ventana"));
                if s.hovered() {
                    relieve(ui.painter(), q, 4, CAJA, true, s.is_pointer_button_down_on());
                }
                let o = q.center();
                let st = Stroke::new(1.3, if s.hovered() { ACENTO } else { TENUE });
                ui.painter().rect_stroke(Rect::from_min_size(o + vec2(-5.0, -1.5), vec2(7.0, 6.0)), 1.0, st, StrokeKind::Inside);
                ui.painter().rect_stroke(Rect::from_min_size(o + vec2(-1.5, -5.0), vec2(7.0, 6.0)), 1.0, st, StrokeKind::Inside);
                if s.clicked() {
                    self.fijas[v as usize] = false;
                    self.mostrar(v);
                    continue;
                }
            }
            if b.clicked() {
                self.mostrar(v);
            }
        }
    }

    fn acomodar(&mut self) {
        self.vez += 1;
        self.completa = None;
        self.abiertas = [false; VISTAS];
        self.fijas = FIJAS;
        self.mostrar(Vista::Arreglo);
    }

    /// Lo que va dentro de una vista, sea pestaña o ventana.
    fn contenido(&mut self, ui: &mut Ui, v: Vista) {
        match v {
            Vista::Arreglo => self.ver_arreglo(ui),
            Vista::Ritmo => self.ver_ritmo(ui),
            Vista::Notas => self.ver_notas(ui),
            Vista::Mezcla => self.ver_mezcla(ui),
            Vista::Canal => self.ver_canal(ui),
            Vista::Asistente => self.ver_asistente(ui),
        }
    }

    /// Pone un audio en la playlist, en su propia pista del Mixer si no se dice cuál.
    pub fn poner_audio(&mut self, r: &Path, pista: Option<usize>, tick: u32, mezcla: Option<usize>) -> Result<(), String> {
        let m = crate::muestrario::abrir(r, 0)?;
        let seg = m.zonas.iter().map(|z| (z.fin - z.ini) as f32 / z.sr).fold(0.0, f32::max);
        let dur = self.p.pulsos(seg);
        let libre = |p: &Proyecto, i: usize| p.clips.iter().all(|c| c.pista != i || c.tick >= tick + dur || c.tick + c.dur <= tick);
        let pista = pista.or_else(|| (0..PISTAS_ARREGLO).find(|i| libre(&self.p, *i))).unwrap_or(0);
        let nombre = r.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let ruta = r.to_string_lossy().to_string();
        // El mismo archivo otra vez va a la pista que ya tiene.
        let ya = self.p.clips.iter().filter_map(|c| c.audio.as_ref()).find(|a| a.ruta == ruta).map(|a| a.mezcla);
        let mezcla = mezcla.or(ya).unwrap_or_else(|| self.p.pista_para(&nombre));
        self.p.clips.push(Clip {
            desfase: 0,
            patron: 0,
            pista,
            tick,
            dur,
            audio: Some(ClipAudio { ruta, nombre: nombre.clone(), mezcla, ..Default::default() }),
        });
        self.clip = Some(self.p.clips.len() - 1);
        self.arreglo.elegir(self.p.clips.len() - 1);
        self.pista = mezcla;
        self.cancion = true;
        self.mostrar(Vista::Arreglo);
        self.avisar(tf("«{}» está en la Playlist y en la pista «{}» del Mixer", &[&nombre, &self.p.mezcla[mezcla].nombre]), false);
        Ok(())
    }

    /// Empieza a grabar el micrófono mientras suena la canción, o termina y pone el clip.
    fn grabar(&mut self) {
        if let Some((mut e, tick)) = self.grabando.take() {
            self.tocar(false);
            let x = e.juntar().to_vec();
            if x.is_empty() {
                return self.avisar(t("No llegó sonido del micrófono"), true);
            }
            let dir = self.carpeta.clone().unwrap_or_else(std::env::temp_dir).join("Grabaciones");
            let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            let r = dir.join(format!("grabacion-{n}.wav"));
            let res = std::fs::create_dir_all(&dir).map_err(|e| e.to_string()).and_then(|_| wav::escribir(&r, e.sr, &x));
            if let Err(e) = res.and_then(|_| self.poner_audio(&r, None, tick, None)) {
                self.avisar(tf("No se pudo guardar: {}", &[&e]), true);
            }
            return;
        }
        match audio::Entrada::abrir() {
            Ok(e) => {
                self.cancion = true;
                let tick = self.tick().max(0.0) as u32;
                self.grabando = Some((e, tick));
                self.tocar(true);
            }
            Err(e) => self.avisar(tf("No se puede grabar: {}", &[&e]), true),
        }
    }

    /// Arriba: archivo, transporte, pantallas y ventanas.
    fn barra(&mut self, ui: &mut Ui) {
        let r = ui.max_rect().expand(8.0);
        placa(ui.painter(), r, 0);
        ui.painter().hline(r.x_range(), r.bottom() - 1.0, Stroke::new(1.0, Color32::BLACK));
        ui.horizontal_centered(|ui| {
            ui.menu_button(t("Archivo"), |ui| {
                ui.set_min_width(200.0);
                let b = |ui: &mut Ui, s: &str, k: &str| ui.add(Button::new(t(s)).shortcut_text(k)).clicked();
                let cmd = if cfg!(target_os = "macos") { "⌘" } else { "Ctrl+" };
                if b(ui, "Nuevo", &format!("{cmd}N")) {
                    self.pedir(Pendiente::Nuevo);
                }
                if b(ui, "Abrir (.bdaw, .flp)…", &format!("{cmd}O")) {
                    self.pedir(Pendiente::Abrir);
                }
                if b(ui, "Importar…", "")
                    && let Some(r) = rfd::FileDialog::new()
                        .add_filter(
                            "Audio, MIDI, SF2, SFZ, Lua",
                            &[crate::decodificar::EXTENSIONES, &["mid", "midi", "sf2", "sf3", "sfz", "lua"]].concat(),
                        )
                        .pick_file()
                {
                    self.abrir(&r);
                }
                if b(ui, "Pegar archivos copiados", &format!("{cmd}V")) && !self.pegar() {
                    self.avisar(t("No hay archivos copiados"), true);
                }
                if b(ui, "Buscar un plugin…", "") {
                    self.elegir_plugin(None);
                }
                ui.separator();
                if b(ui, "Guardar", &format!("{cmd}S")) {
                    self.guardar(false);
                }
                if b(ui, "Guardar como…", "") {
                    self.guardar(true);
                }
                ui.separator();
                if b(ui, "Exportar audio…", "") {
                    self.exportar_audio();
                }
                if b(ui, "Exportar pistas separadas…", "") {
                    self.exportar_pistas();
                }
                if b(ui, "Exportar MIDI…", "") {
                    self.exportar_midi();
                }
                ui.separator();
                ui.label(RichText::new(t("Idioma")).color(TENUE));
                for i in Idioma::TODOS {
                    if ui.radio(idioma::actual() == i, i.nombre()).clicked() {
                        self.cambiar_idioma(i);
                    }
                }
            });
            ui.menu_button(t("Ventana"), |ui| {
                ui.set_min_width(210.0);
                for v in Vista::TODAS {
                    let b = Button::selectable(self.abierta(v), t(v.nombre())).shortcut_text(v.tecla().1);
                    if ui.add(b).clicked() {
                        self.alternar(v);
                        ui.close();
                    }
                }
                if ui.add(Button::selectable(self.biblioteca_abierta, t("Biblioteca")).shortcut_text("F4")).clicked() {
                    self.biblioteca_abierta = !self.biblioteca_abierta;
                    ui.close();
                }
                ui.separator();
                if ui.button(t("Acomodar las ventanas")).clicked() {
                    self.acomodar();
                    ui.close();
                }
            });
            if ui.add_enabled(!self.hecho.is_empty(), Button::new("↺")).on_hover_text(t("Deshacer")).clicked() {
                self.deshacer();
            }
            if ui.add_enabled(!self.deshecho.is_empty(), Button::new("↻")).on_hover_text(t("Rehacer")).clicked() {
                self.rehacer();
            }
            ui.add_space(6.0);
            for (c, s) in [(false, "Patrón"), (true, "Canción")] {
                if foco(ui, self.cancion == c, t(s), ACENTO).on_hover_text(t("Qué se toca: solo el patrón elegido o toda la canción")).clicked()
                    && self.cancion != c
                {
                    self.cancion = c;
                    self.ir(0.0);
                }
            }
            let toca = self.tocando();
            let b = boton(ui, vec2(40.0, 32.0), toca, VERDE, t("Tocar o pausar (espacio)"), |p, o, c| {
                if toca {
                    for x in [-6.0, 2.0] {
                        p.rect_filled(Rect::from_min_size(o + vec2(x, -7.0), vec2(4.0, 14.0)), 1.0, c);
                    }
                } else {
                    p.add(Shape::convex_polygon(vec![o + vec2(-5.0, -8.0), o + vec2(8.0, 0.0), o + vec2(-5.0, 8.0)], c, Stroke::NONE));
                }
            });
            if b.clicked() {
                if self.grabando.is_some() { self.grabar() } else { self.tocar(!toca) }
            }
            let b = boton(ui, vec2(36.0, 32.0), false, TEXTO, t("Parar y volver al inicio"), |p, o, c| {
                _ = p.rect_filled(Rect::from_center_size(o, vec2(12.0, 12.0)), 1.0, c)
            });
            if b.clicked() {
                if self.grabando.is_some() {
                    self.grabar();
                }
                self.tocar(false);
                self.ir(0.0);
            }
            let g = self.grabando.is_some();
            if boton(ui, vec2(36.0, 32.0), g, ROJO, t("Grabar el micrófono en una pista de audio"), |p, o, c| _ = p.circle_filled(o, 6.5, c))
                .clicked()
            {
                self.grabar();
            }
            let k = self.tick().max(0.0) as u32;
            pantalla(ui, &format!("{:03}:{}:{}", k / COMPAS + 1, k % COMPAS / PPQ + 1, k % PPQ / PASO + 1), 108.0, 21.0)
                .on_hover_text(t("Compás : tiempo : paso"));
            let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
            esfera(ui.painter(), r.center(), 14.0, k as f64 / PPQ as f64 * 60.0 / self.p.bpm as f64, ACENTO);
            ui.add(DragValue::new(&mut self.p.bpm).range(20.0..=400.0).speed(0.25).max_decimals(1).suffix(" bpm"))
                .on_hover_text(t("Tempo: arrastra o haz clic para escribir"));
            let mut s = self.p.swing;
            if perilla(ui, "", &mut s, 0.0, 1.0, 0.0, ACENTO, 30.0).on_hover_text("Swing").changed() {
                self.p.swing = s;
            }
            let m = boton(ui, vec2(32.0, 32.0), self.metronomo, ACENTO, t("Metrónomo"), |p, o, c| {
                p.add(Shape::closed_line(
                    vec![o + vec2(-7.0, 8.0), o + vec2(-3.5, -8.0), o + vec2(3.5, -8.0), o + vec2(7.0, 8.0)],
                    Stroke::new(1.6, c),
                ));
                p.line_segment([o + vec2(0.0, 5.0), o + vec2(6.0, -7.0)], Stroke::new(1.6, c));
            });
            if m.clicked() {
                self.metronomo = !self.metronomo;
                self.mando.mandar(Orden::Metronomo(self.metronomo));
            }
            ui.add_space(6.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (r, _) = ui.allocate_exact_size(vec2(16.0, 34.0), Sense::hover());
                nivel(ui.painter(), r, self.picos[0]);
                let c = self.estado.carga.load(Relaxed) as f32 / 10.0;
                let tono = if c > 80.0 { ROJO } else { TENUE };
                ui.add(
                    Label::new(RichText::new(tf("{}% · {} voces", &[&format!("{c:.0}"), &self.estado.voces.load(Relaxed)])).size(12.0).color(tono))
                        .truncate(),
                )
                .on_hover_text(t("Cuánto del tiempo disponible usa el audio"));
                let sin = if self.p != self.guardado { " •" } else { "" };
                ui.add(Label::new(RichText::new(format!("{}{sin}", self.p.nombre)).color(TENUE)).truncate());
            });
        });
    }

    /// Una ventana de metal dentro del programa.
    fn marco(&mut self, ui: &mut Ui, z: Rect, v: Vista) {
        let id = Id::new(("ventana", v as u8, self.vez));
        let completa = self.completa == Some(v);
        let w = Window::new(t(v.nombre())).id(id).title_bar(false).frame(Frame::NONE).constrain_to(z);
        let mut w = w.min_size(vec2(260.0, 150.0)).default_rect(v.lugar(z));
        w = if completa { w.fixed_rect(z.shrink(3.0)) } else { w.resizable(true) };
        let r = w.show(ui.ctx(), |ui| {
            let (todo, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
            let nombre = match v {
                Vista::Canal if self.clip_audio().is_some() => format!("{} · {}", t("Audio"), self.clip_audio().map_or("", |a| a.nombre.as_str())),
                Vista::Notas | Vista::Canal => {
                    let c = self.p.canales.iter().find(|c| c.id == self.canal).map_or("", |c| c.nombre.as_str());
                    format!("{} · {c}", t(v.nombre()))
                }
                _ => t(v.nombre()).to_string(),
            };
            let pide = titulo(ui, todo, &nombre, self.activa == v, id, true);
            if pide.cerrar {
                self.cerrar_ventana(v);
            }
            if pide.agrandar {
                self.completa = if completa { None } else { Some(v) };
                self.activa = v;
            }
            if pide.fijar {
                self.abiertas[v as usize] = false;
                self.completa = None;
                self.fijas[v as usize] = true;
                self.mostrar(v);
            }
            let dentro = Rect::from_min_max(pos2(todo.left() + 6.0, todo.top() + TITULO + 5.0), todo.max - vec2(6.0, 6.0));
            let mut c = ui.new_child(UiBuilder::new().max_rect(dentro).id_salt(v as u8));
            c.set_clip_rect(dentro.expand(2.0).intersect(ui.clip_rect()));
            self.contenido(&mut c, v);
        });
        if let Some(r) = r {
            let capa = r.response.layer_id;
            let pos = ui.input(|i| i.pointer.interact_pos()).unwrap_or_default();
            if ui.input(|i| i.pointer.any_pressed()) && ui.ctx().layer_id_at(pos) == Some(capa) {
                self.activa = v;
            }
            if self.al_frente == Some(v) {
                ui.ctx().move_to_top(capa);
                self.al_frente = None;
            }
            self.capas[v as usize] = Some(capa);
        }
    }

    /// La ventana que está debajo de ese punto.
    fn vista_en(&self, ctx: &Context, o: Pos2) -> Option<Vista> {
        let capa = ctx.layer_id_at(o)?;
        let ventana = Vista::TODAS.into_iter().find(|v| !self.fijas[*v as usize] && self.abierta(*v) && self.capas[*v as usize] == Some(capa));
        // Un aviso u otra cosa encima no cuenta: lo que sigue es la pestaña.
        ventana.or_else(|| self.fondo.contains(o).then_some(self.pestana))
    }

    /// Un archivo soltado encima del programa: cae donde está el ratón.
    pub fn soltar(&mut self, ctx: &Context, r: &Path, o: Option<Pos2>) {
        if !crate::decodificar::es_audio(r) {
            return self.abrir(r);
        }
        let vista = o.and_then(|o| self.vista_en(ctx, o));
        let nombre = r.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let res = match (vista, o) {
            (Some(Vista::Ritmo | Vista::Canal), _) => self.poner_muestra(r, &nombre),
            (Some(Vista::Arreglo), Some(o)) => {
                let (pista, tick) = self.arreglo.lugar(o);
                self.poner_audio(r, pista, tick, None)
            }
            (Some(Vista::Mezcla), Some(o)) => {
                let i = self.tiras.iter().find(|x| x.1.contains(o)).map(|x| x.0);
                let r = self.poner_audio(r, None, self.donde(), i);
                self.mostrar(Vista::Mezcla);
                r
            }
            _ => self.poner_audio(r, None, self.donde(), None),
        };
        if let Err(e) = res {
            self.avisar(format!("{nombre}: {e}"), true);
        }
    }

    /// Donde va lo que no dice su lugar: el cabezal, en el tiempo más cercano.
    fn donde(&self) -> u32 {
        if self.cancion { self.tick().max(0.0) as u32 / PPQ * PPQ } else { 0 }
    }

    /// Un audio como instrumento, en un canal nuevo.
    fn poner_muestra(&mut self, r: &Path, nombre: &str) -> Result<(), String> {
        crate::muestrario::abrir(r, 0)?;
        self.canal = self.p.agregar_canal("muestra", nombre);
        let pista = self.p.pista_para(nombre);
        let c = self.p.canales.last_mut().unwrap();
        c.pista = pista;
        c.muestra = Some(r.to_string_lossy().into());
        self.clip = None;
        Ok(())
    }

    /// El clip de audio elegido en la playlist.
    fn clip_audio(&self) -> Option<&ClipAudio> {
        self.p.clips.get(self.clip?)?.audio.as_ref()
    }

    fn elegir_plugin(&mut self, pista: Option<usize>) {
        let d = rfd::FileDialog::new().add_filter("VST3 / CLAP", &["vst3", "clap"]);
        let d = match plugin::carpetas().into_iter().find(|c| c.is_dir()) {
            Some(c) => d.set_directory(c),
            None => d,
        };
        // En Mac los plugins son paquetes y se eligen como archivos.
        if let Some(r) = d.pick_file() {
            self.ofrecer(&r, pista);
        }
    }

    fn atajos(&mut self, ui: &mut Ui) {
        let escribe = ui.ctx().text_edit_focused();
        use egui::Key as K;
        let cmd = |k| KeyboardShortcut::new(Modifiers::COMMAND, k);
        let usa = |ui: &mut Ui, s: KeyboardShortcut| ui.input_mut(|i| i.consume_shortcut(&s));
        if usa(ui, cmd(K::S)) {
            self.guardar(false);
        }
        if usa(ui, cmd(K::O)) {
            self.pedir(Pendiente::Abrir);
        }
        if usa(ui, cmd(K::N)) {
            self.pedir(Pendiente::Nuevo);
        }
        if escribe {
            self.soltar_teclas();
            return;
        }
        if usa(ui, KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, K::Z)) || usa(ui, cmd(K::Y)) {
            self.rehacer();
        }
        if usa(ui, cmd(K::Z)) {
            self.deshacer();
        }
        let toco = |ui: &mut Ui, k| ui.input_mut(|i| i.consume_key(Modifiers::NONE, k));
        if toco(ui, K::Space) {
            let t = self.tocando();
            self.tocar(!t);
        }
        if toco(ui, K::F4) {
            self.biblioteca_abierta = !self.biblioteca_abierta;
        }
        for v in Vista::TODAS {
            if toco(ui, v.tecla().0) {
                self.alternar(v);
            }
        }
        if toco(ui, K::L) {
            self.cancion = !self.cancion;
            self.ir(0.0);
        }
        // El teclado de la computadora toca notas: dos octavas, como un piano.
        const ABAJO: [Key; 12] = [K::Z, K::S, K::X, K::D, K::C, K::V, K::G, K::B, K::H, K::N, K::J, K::M];
        const ARRIBA: [Key; 12] = [K::Q, K::Num2, K::W, K::Num3, K::E, K::R, K::Num5, K::T, K::Num6, K::Y, K::Num7, K::U];
        let (mut abajo, mut sueltas) = (vec![], vec![]);
        ui.input(|i| {
            for e in &i.events {
                if let Event::Key { key, pressed, repeat, modifiers, .. } = e {
                    if *pressed && !*repeat && !modifiers.any() {
                        abajo.push(*key);
                    } else if !*pressed {
                        sueltas.push(*key);
                    }
                }
            }
        });
        for k in abajo {
            let j = ABAJO.iter().position(|x| *x == k).map(|j| j as u8).or_else(|| ARRIBA.iter().position(|x| *x == k).map(|j| j as u8 + 12));
            if let Some(j) = j.filter(|_| self.canal != 0) {
                let tono = (self.octava * 12 + j).min(127);
                self.sonar(self.canal, tono, 0.9);
                self.teclas.insert(k, (self.canal, tono));
                self.toque(self.canal, tono, 0.9);
            }
        }
        for k in sueltas {
            if let Some((c, tono)) = self.teclas.remove(&k) {
                self.callar(c, tono);
                self.toque(c, tono, 0.0);
            }
        }
    }

    /// Anota en el patrón lo que se toca mientras suena.
    fn toque(&mut self, canal: u32, tono: u8, fuerza: f32) {
        if !self.anotar || self.cancion {
            return;
        }
        let k = self.tick().max(0.0) as u32;
        if fuerza > 0.0 {
            if self.tocando() {
                self.anotando.insert(tono, (canal, k, fuerza));
            }
        } else if let Some((canal, ini, fuerza)) = self.anotando.remove(&tono) {
            let rej = self.notas.rejilla();
            let Some(x) = self.p.patrones.iter_mut().find(|x| x.id == self.patron) else { return };
            let largo = x.largo();
            let tick = (ini + rej / 2) / rej * rej % largo;
            // Si dio la vuelta mientras estaba apretada, llega hasta el final.
            let fin = if k > ini { k } else { largo };
            let dur = ((fin - ini + rej / 2) / rej * rej).max(rej);
            x.notas.entry(canal).or_default().push(Nota { tick, dur, tono, fuerza });
        }
    }

    fn soltar_teclas(&mut self) {
        for (c, tono) in std::mem::take(&mut self.teclas).into_values() {
            self.callar(c, tono);
        }
    }

    fn dialogos(&mut self, ui: &mut Ui) {
        if let Some(x) = self.pendiente {
            let r = Modal::new(Id::new("pendiente")).show(ui.ctx(), |ui| {
                ui.heading(t("Hay cambios sin guardar"));
                ui.label(t("¿Qué hago con ellos?"));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let g = ui.button(t("Guardar")).clicked();
                    let d = ui.button(t("Descartar")).clicked();
                    let c = ui.button(t("Cancelar")).clicked();
                    (g, d, c)
                })
                .inner
            });
            let (g, d, c) = r.inner;
            if d || (g && self.guardar(false)) {
                self.pendiente = None;
                self.hacer(x);
            } else if c || r.should_close() {
                self.pendiente = None;
            }
        }
        if let Some((v, pista)) = self.ofertas.clone() {
            let r = Modal::new(Id::new("ofertas")).show(ui.ctx(), |ui| {
                ui.heading(t("¿Cuál plugin?"));
                let mut sale = None;
                ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    for o in &v {
                        let clase = t(if o.instrumento { "instrumento" } else { "efecto" });
                        if ui.button(format!("{} · {clase}", o.nombre)).clicked() {
                            sale = Some(o.clone());
                        }
                    }
                });
                sale
            });
            if let Some(o) = &r.inner {
                self.poner_plugin(o, pista);
            }
            if r.inner.is_some() || r.should_close() {
                self.ofertas = None;
            }
        }
    }

    fn mensajes(&mut self, ui: &mut Ui) {
        let dt = ui.input(|i| i.stable_dt).min(0.3) as f64;
        self.avisos.iter_mut().for_each(|a| a.2 -= dt);
        self.avisos.retain(|a| a.2 > 0.0);
        if self.avisos.is_empty() {
            return;
        }
        // No atrapan el ratón: lo de abajo se sigue usando, y se van solos.
        let area = Area::new(Id::new("avisos")).anchor(Align2::CENTER_BOTTOM, vec2(0.0, -40.0)).order(Order::Tooltip).interactable(false);
        area.show(ui.ctx(), |ui| {
            for (s, malo, _) in self.avisos.iter().rev().take(4) {
                let f = Frame::new().fill(CAJA).corner_radius(8).inner_margin(10).stroke(Stroke::new(1.0, if *malo { ROJO } else { VERDE }));
                f.show(ui, |ui| {
                    ui.set_max_width(460.0);
                    ui.label(s)
                });
            }
        });
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        self.pista_ayuda.clear();
        let soltados: Vec<PathBuf> = ui.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        if !soltados.is_empty() {
            let o = raton(ui.ctx());
            for r in soltados {
                self.soltar(ui.ctx(), &r, o);
            }
        }
        // Archivos copiados en el Finder o el Explorador.
        if !ui.ctx().text_edit_focused() && ui.input(|i| i.events.iter().any(|e| matches!(e, Event::Paste(_)))) {
            self.pegar();
        }
        for (i, p) in self.picos.iter_mut().enumerate() {
            let nuevo = self.estado.pico(i);
            for l in 0..2 {
                p[l] = nuevo[l].max(p[l] * 0.88);
            }
        }
        let dt = ui.input(|i| i.stable_dt).min(0.1) as f64;
        self.mover_cabezal(dt);
        self.atender_ia();
        self.bib.rack.todos().for_each(|p| p.atender());
        self.ventanas.retain(|_, v| v.sigue());
        self.canal_midi.store(self.canal, Relaxed);
        while let Ok((c, tono, f)) = self.toques.try_recv() {
            self.toque(c, tono, f);
        }
        self.atajos(ui);

        let placa_panel = Frame::new().fill(PANEL).inner_margin(8);
        Panel::top("barra").exact_size(48.0).frame(placa_panel).show(ui, |ui| self.barra(ui));
        Panel::bottom("ayuda").exact_size(26.0).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(10, 2))).show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let s = if self.pista_ayuda_vista.is_empty() {
                    t("Arrastra o pega aquí audio, .flp, MIDI, SoundFont, plugins o guiones de Lua · el teclado toca notas").to_string()
                } else {
                    self.pista_ayuda_vista.clone()
                };
                ui.label(RichText::new(s).size(13.5).color(TENUE));
            });
        });
        if self.biblioteca_abierta {
            Panel::left("biblioteca").exact_size(206.0).frame(placa_panel).show(ui, |ui| {
                let r = ui.max_rect().expand(8.0);
                degradado(ui.painter(), r, 0, Color32::from_rgb(0x25, 0x26, 0x2a), Color32::from_rgb(0x17, 0x18, 0x1b));
                self.biblioteca(ui)
            });
        }
        Panel::top("pestanas").exact_size(32.0).frame(Frame::NONE).show(ui, |ui| self.pestanas(ui));
        let z = CentralPanel::default().frame(Frame::new().fill(FONDO)).show(ui, |ui| {
            let z = ui.max_rect();
            placa(ui.painter(), z, 0);
            let pos = ui.input(|i| i.pointer.interact_pos()).unwrap_or_default();
            if ui.input(|i| i.pointer.any_pressed()) && z.contains(pos) && ui.ctx().layer_id_at(pos) == Some(LayerId::background()) {
                self.activa = self.pestana;
            }
            let dentro = z.shrink2(vec2(6.0, 6.0));
            let mut c = ui.new_child(UiBuilder::new().max_rect(dentro).id_salt(("fondo", self.pestana as u8)));
            c.set_clip_rect(z);
            self.contenido(&mut c, self.pestana);
            z
        });
        let z = z.inner;
        self.fondo = z;
        for v in Vista::TODAS {
            if !self.fijas[v as usize] && self.abierta(v) && self.completa.is_none_or(|c| c == v) {
                self.marco(ui, z, v);
            }
        }
        if let Some((e, _)) = &mut self.grabando {
            e.juntar();
            ui.ctx().request_repaint();
        }
        self.ver_efectos(ui, z);
        self.ver_editor(ui);
        self.ver_elegir(ui);
        self.dialogos(ui);
        self.mensajes(ui);
        self.pista_ayuda_vista = std::mem::take(&mut self.pista_ayuda);
        self.revisar_borrados();
        self.historial(ui);
        self.sincronizar();
        metalizar(ui.ctx());
        if self.tocando() || !self.teclas.is_empty() || self.picos.iter().flatten().any(|x| *x > 1e-3) {
            ui.ctx().request_repaint();
        } else if self.teclados.as_ref().is_some_and(|t| !t.nombres.is_empty()) || !self.ventanas.is_empty() || self.bib.rack.todos().next().is_some()
        {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
        }
        if ui.input(|i| i.viewport().close_requested()) && !self.cerrar && self.p != self.guardado {
            ui.ctx().send_viewport_cmd(ViewportCommand::CancelClose);
            self.pendiente = Some(Pendiente::Cerrar);
        } else if self.cerrar {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
    }

    fn revisar_borrados(&mut self) {
        // Sin instrumentos no hay canal que elegir: revisar cada cuadro soltaría lo que se arrastra.
        let sin_canal = !self.p.canales.is_empty() && !self.p.canales.iter().any(|c| c.id == self.canal);
        if self.p.patron(self.patron).is_none() || sin_canal {
            self.revisar();
        }
    }

    /// Los clips y las notas de lo que se borró también se van.
    fn borrar_canal(&mut self, id: u32) {
        self.p.canales.retain(|c| c.id != id);
        for p in &mut self.p.patrones {
            p.notas.remove(&id);
        }
    }

    fn borrar_patron(&mut self, id: u32) {
        if self.p.patrones.len() > 1 {
            self.p.patrones.retain(|x| x.id != id);
            self.p.clips.retain(|c| c.patron != id);
        }
    }

    /// Agrega un instrumento de Lua como canal nuevo.
    pub fn poner_instrumento(&mut self, clave: &str) {
        if self.bib.del_banco(clave).is_none() && !self.bib.ver(clave).is_some_and(|g| g.ficha.tipo == Tipo::Instrumento) {
            return;
        }
        let n = self.bib.nombre(clave);
        self.canal = self.p.agregar_canal(clave, &n);
        let pista = self.p.pista_para(&n);
        self.p.canales.last_mut().unwrap().pista = pista;
        self.clip = None;
    }
}

/// Un WAV por cada pista de mezcla que suene, con los efectos de la general. Devuelve cuántas.
pub fn pistas(p: &Proyecto, bib: &mut Biblioteca, modo: Modo, dir: &Path) -> Result<usize, String> {
    let mut n = 0;
    for i in 1..PRIMER_ENVIO {
        let Some(q) = App::solo_pista(p, i) else { continue };
        let x = a_audio(&q, bib, modo);
        if x.iter().flatten().all(|v| v.abs() < 1e-5) {
            continue;
        }
        let limpio: String = q.mezcla[i].nombre.chars().map(|c| if c.is_alphanumeric() || " -_".contains(c) { c } else { '_' }).collect();
        wav::escribir(&dir.join(format!("{i:02} {limpio}.wav")), bib.sr as u32, &x)?;
        n += 1;
    }
    Ok(n)
}

/// Dónde está el ratón dentro de la ventana. Mientras se arrastra un archivo, macOS no avisa: se le pregunta.
fn raton(ctx: &Context) -> Option<Pos2> {
    #[cfg(target_os = "macos")]
    if let (Some(mtm), Some(v)) = (objc2::MainThreadMarker::new(), ctx.input(|i| i.viewport().inner_rect)) {
        let o = objc2_app_kit::NSEvent::mouseLocation();
        // La pantalla principal manda: sus coordenadas crecen hacia arriba.
        let alto = objc2_app_kit::NSScreen::screens(mtm).firstObject()?.frame().size.height;
        return Some(pos2(o.x as f32, (alto - o.y) as f32) - v.min.to_vec2());
    }
    ctx.input(|i| i.pointer.latest_pos())
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _: &mut eframe::Frame) {
        App::ui(self, ui);
    }

    fn on_exit(&mut self) {
        // Antes de soltar los plugins se cierran sus ventanas.
        self.ventanas.clear();
    }
}
