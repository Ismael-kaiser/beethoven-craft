//! La ventana: barra, línea de tiempo, vista previa, consola y editor de audio.
pub mod animacion;
pub mod archivos;
pub mod atajos;
pub mod biblioteca;
pub mod color;
pub mod consola;
pub mod editor;
pub mod inicio;
pub mod estilo;
pub mod inspector;
pub mod libre;
pub mod linea;
pub mod muestras;
pub mod nodos;
pub mod musica;
pub mod origen;
pub mod pestanas;
pub mod proyecto;
#[rustfmt::skip]
#[allow(dead_code)]
mod tabler;

use crate::audio::{self, Grabacion, Monitor, Reloj, Salida};
use crate::exportar::{self, Modo};
use crate::medios::{self, Audio, Info};
use crate::modelo::*;
use crate::motor::{self, Almacen, Motor};
use crate::separar;
use crate::video::{self, Tira, VistaPrevia};
use crossbeam_channel::{Receiver, Sender, unbounded};
use eframe::egui::{self, *};
use estilo::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Canal {
    Pista(u64),
    /// Mezclador.
    Bus(u64),
    Maestro,
}

/// Pestañas principales.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Espacio {
    Video,
    Audio,
    Playlist,
    Efectos,
    Animacion,
    Nodos,
}

impl Espacio {
    /// Índice en `App::ventanas` si se puede sacar a su ventana.
    pub fn ventana(self) -> Option<usize> {
        match self {
            Espacio::Video => Some(0),
            Espacio::Audio => Some(1),
            Espacio::Efectos => Some(2),
            Espacio::Playlist | Espacio::Animacion | Espacio::Nodos => None,
        }
    }
}

/// Panel de la izquierda en la pestaña de video.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PanelIzq {
    Origen,
    Controles,
    Color,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Pestana {
    Rack,
    Piano,
    Consola,
    Editor,
}

#[derive(Clone, Copy)]
enum Confirmar {
    Salir,
    Nuevo,
    Abrir,
}

/// Cómo tratar el audio de un video al importarlo.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AudioVideo {
    Junto,
    Separado,
}

enum Evento {
    Sondeado(PathBuf, Result<Info, String>),
    Audio { tarea: u64, ruta: PathBuf, res: Result<Audio, String>, destino: Destino2 },
    Separado { tarea: u64, ruta: PathBuf, res: Result<(PathBuf, PathBuf), String>, inicio: f64, molde: Option<Clip>, con: Vec<u64> },
    Estirado { clave: PathBuf, res: Result<Audio, String> },
    Congelado { clip: u64, t: f64, ruta: PathBuf, ok: bool },
    Tira(PathBuf, Option<Tira>),
    Muestra(String, Vec<u8>),
    Exportado { tarea: u64, ruta: PathBuf, res: Result<(), String> },
}

/// Qué hacer con un audio ya leído.
enum Destino2 {
    /// Solo al almacén (proyecto abierto).
    Nada,
    /// `con`: clips con los que queda vinculado.
    PistaNueva { nombre: String, inicio: f64, molde: Option<Clip>, con: Vec<u64> },
    Grabacion { pista: u64, inicio: f64, desde: f64 },
}

/// Miniaturas de un video en la línea de tiempo.
pub struct TiraLista {
    pub paso: f64,
    pub tex: Vec<TextureHandle>,
}

struct Tarea {
    nombre: String,
    avance: Arc<AtomicU32>,
    cancelar: Arc<AtomicBool>,
}

struct GrabacionActiva {
    g: Grabacion,
    pista: u64,
    inicio: f64,
    arrancada: bool,
    abierta: Instant,
    /// Par del reloj tomado mientras suena: ubica la toma en la línea de tiempo.
    reloj: Option<Reloj>,
    nivel: f32,
}

pub struct App {
    pub p: Proyecto,
    /// Menú principal.
    pub inicio: inicio::Estado,
    /// Pestaña de animación.
    pub anim: animacion::Estado,
    pub atajos: atajos::Estado,
    pub archivos: archivos::Estado,
    pub ajustes_proyecto: Option<proyecto::Ajustes>,
    pub nodos: nodos::Estado,
    aplicado: Option<Arc<Proyecto>>,
    /// Versión del proyecto que ya está en el historial, y si difiere de lo guardado (con la versión en que se vio).
    v_historial: u64,
    sucio: std::cell::Cell<(u64, bool)>,
    respaldo: Sender<Proyecto>,
    ultimo: Proyecto,
    deshacer: Vec<Foto>,
    rehacer: Vec<Foto>,
    guardado: Proyecto,
    pub almacen: Almacen,
    pub motor: Arc<Mutex<Motor>>,
    pub reloj: Arc<Mutex<Reloj>>,
    salida: Option<Salida>,
    sin_dispositivo: bool,
    muda: bool,
    ruta: Option<PathBuf>,
    pub canal: Canal,
    pub clip: Option<u64>,
    pub pestana: Pestana,
    pub linea: linea::Estado,
    pub editor: editor::Estado,
    vista: VistaPrevia,
    vista_p: Arc<Proyecto>,
    vista_v: u64,
    textura: Option<TextureHandle>,
    /// Alto de la vista previa en pixeles.
    /// Alto de la vista previa; 0: el de la pantalla.
    calidad: usize,
    /// Pixeles de alto que ocupa la imagen en la pantalla.
    alto_panel: usize,
    pub tiras: HashMap<PathBuf, Option<TiraLista>>,
    /// Último cuadro compuesto, para los medidores de color.
    cuadro: Option<Arc<crate::composicion::Cuadro>>,
    cuadro_v: u64,
    medidor_img: usize,
    medidor_v: u64,
    medidor_tex: Option<TextureHandle>,
    pub espacio: Espacio,
    pub panel_izq: PanelIzq,
    /// Pestañas abiertas, en orden.
    pub pestanas: Vec<Espacio>,
    /// Pestañas sacadas a su propia ventana: Video, Audio, Efectos.
    pub ventanas: [bool; 3],
    /// Miniaturas de efectos, transiciones y estilos.
    pub muestras: HashMap<String, TextureHandle>,
    muestras_hechas: usize,
    muestras_pedidas: bool,
    /// Se está reproduciendo.
    suena: Arc<AtomicBool>,
    /// Cuándo se preparó el audio que sigue.
    audio_listo: Instant,
    pub origen: Option<origen::Origen>,
    pub seleccion: HashSet<u64>,
    pidiendo: HashSet<PathBuf>,
    exportar_op: Option<exportar::Opciones>,
    teclado: Option<crate::midi::Teclado>,
    midi_rec: bool,
    midi_abiertas: Vec<(u8, usize)>,
    pub lib: biblioteca::Estado,
    pub musica: musica::Estado,
    metronomo: bool,
    pub escuchar: bool,
    monitor_auto: bool,
    tx: Sender<Evento>,
    rx: Receiver<Evento>,
    tareas: HashMap<u64, Tarea>,
    sig_tarea: u64,
    pub preguntar: Option<(PathBuf, Info)>,
    mensaje: Option<(String, bool, Instant)>,
    grab: Option<GrabacionActiva>,
    monitor: Option<Monitor>,
    avisar_bocinas: bool,
    mics: Vec<String>,
    mic: Option<String>,
    picos: HashMap<u64, [f32; 2]>,
    pico_maestro: [f32; 2],
    confirmar: Option<Confirmar>,
    acerca: bool,
    cerrar_ya: bool,
}

/// Ícono de la app, 512 × 512 RGBA.
pub const ICONO: &[u8] = include_bytes!("../../recursos/icono512.rgba");

/// Grabaciones, voces separadas y autoguardado: en la carpeta de videos del usuario de cada sistema.
pub fn carpeta() -> PathBuf {
    let var = |v: &str| std::env::var_os(v).filter(|x| !x.is_empty()).map(PathBuf::from);
    let videos = if cfg!(windows) {
        var("USERPROFILE").map(|h| h.join("Videos"))
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Movies"))
    } else {
        var("XDG_VIDEOS_DIR").or_else(|| var("HOME").map(|h| h.join("Videos")))
    };
    videos.unwrap_or_else(std::env::temp_dir).join("Beethoven Craft")
}

impl App {
    /// `sin_dispositivo`: no abre la tarjeta de sonido (pruebas). `muda`: la abre pero manda silencio.
    pub fn nueva(ctx: &Context, sin_dispositivo: bool, muda: bool) -> App {
        // Se abre desde ya, sin detener la ventana.
        std::thread::spawn(crate::vgpu::gpu);
        estilo::aplicar(ctx);
        if !sin_dispositivo {
            inicio::cargar_ajustes();
        }
        let (tx, rx) = unbounded();
        let p = Proyecto::nuevo();
        let canal = p.pistas.first().map_or(Canal::Maestro, |x| Canal::Pista(x.id));
        let mut a = App {
            inicio: inicio::Estado::default(),
            anim: animacion::Estado::default(),
            atajos: Default::default(),
            archivos: Default::default(),
            ajustes_proyecto: None,
            nodos: Default::default(),
            aplicado: None,
            v_historial: 0,
            sucio: Default::default(),
            respaldo: hilo_respaldo(),
            ultimo: p.clone(),
            guardado: p.clone(),
            p,
            deshacer: vec![],
            rehacer: vec![],
            almacen: HashMap::new(),
            motor: Arc::new(Mutex::new(Motor::default())),
            reloj: Arc::new(Mutex::new(Reloj::nuevo())),
            salida: None,
            sin_dispositivo,
            muda,
            ruta: None,
            canal,
            clip: None,
            pestana: Pestana::Consola,
            linea: linea::Estado::default(),
            editor: editor::Estado::default(),
            vista: VistaPrevia::default(),
            vista_p: Arc::new(Proyecto::nuevo()),
            vista_v: 0,
            textura: None,
            calidad: 0,
            alto_panel: 540,
            tiras: HashMap::new(),
            cuadro: None,
            cuadro_v: 0,
            medidor_img: 0,
            medidor_v: u64::MAX,
            medidor_tex: None,
            espacio: Espacio::Playlist,
            panel_izq: PanelIzq::Controles,
            pestanas: vec![Espacio::Playlist, Espacio::Video, Espacio::Audio, Espacio::Efectos, Espacio::Animacion, Espacio::Nodos],
            ventanas: [false; 3],
            muestras: HashMap::new(),
            muestras_hechas: 0,
            muestras_pedidas: false,
            suena: Default::default(),
            audio_listo: Instant::now(),
            origen: None,
            seleccion: HashSet::new(),
            pidiendo: HashSet::new(),
            exportar_op: None,
            teclado: None,
            midi_rec: false,
            midi_abiertas: vec![],
            lib: biblioteca::Estado::default(),
            musica: musica::Estado::default(),
            metronomo: false,
            escuchar: true,
            monitor_auto: false,
            tx,
            rx,
            tareas: HashMap::new(),
            sig_tarea: 1,
            preguntar: None,
            mensaje: None,
            grab: None,
            monitor: None,
            avisar_bocinas: false,
            mics: vec![],
            mic: None,
            picos: HashMap::new(),
            pico_maestro: [0.0; 2],
            confirmar: None,
            acerca: false,
            cerrar_ya: false,
        };
        if !sin_dispositivo {
            a.archivos.cargar();
        }
        a.abrir_salida();
        a
    }

    fn abrir_salida(&mut self) {
        if self.sin_dispositivo {
            return;
        }
        self.salida = None;
        match audio::abrir_salida(self.motor.clone(), self.reloj.clone(), self.muda) {
            Ok(s) => self.salida = Some(s),
            Err(e) => self.avisar(e, true),
        }
    }

    pub fn avisar(&mut self, s: impl Into<String>, error: bool) {
        self.mensaje = Some((s.into(), error, Instant::now()));
    }

    // ---- transporte

    pub fn tocando(&self) -> bool {
        self.motor.lock().unwrap().tocando
    }

    pub fn ahora(&self) -> f64 {
        let r = self.reloj.lock().unwrap();
        r.ahora().min(self.p.fin().max(r.pos))
    }

    pub fn ir(&mut self, t: f64) {
        if self.grab.is_some() {
            return;
        }
        let t = t.max(0.0);
        let mut m = self.motor.lock().unwrap();
        m.ir(motor::cuadros(t));
        let mut r = self.reloj.lock().unwrap();
        *r = Reloj { pos: t, instante: Instant::now(), tocando: r.tocando, desde: t };
    }

    pub fn alternar(&mut self) {
        if self.grab.is_some() {
            self.parar_grabacion();
            return;
        }
        self.parar_origen();
        if self.tocando() {
            self.pausar();
        } else {
            if self.ahora() >= self.p.fin() - 0.02 {
                self.ir(0.0);
            }
            self.sincronizar();
            self.preparar_audio();
            self.motor.lock().unwrap().tocando = true;
        }
    }

    fn pausar(&mut self) {
        let t = self.ahora();
        self.motor.lock().unwrap().tocando = false;
        self.ir(t);
    }

    fn detener(&mut self) {
        if self.grab.is_some() {
            self.parar_grabacion();
        }
        self.motor.lock().unwrap().tocando = false;
        self.ir(0.0);
    }

    // ---- grabación

    pub fn grabar(&mut self) {
        if self.grab.is_some() {
            self.parar_grabacion();
            return;
        }
        if self.tocando() {
            self.pausar();
        }
        let pista = match self.p.pistas.iter().find(|x| x.armada) {
            Some(x) => x.id,
            None => self.p.nueva_pista_grabacion(),
        };
        self.canal = Canal::Pista(pista);
        let seg = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let ruta = carpeta().join("Grabaciones").join(format!("toma-{seg}.wav"));
        match Grabacion::iniciar(self.mic.as_deref(), &ruta) {
            Ok(g) => {
                if self.escuchar && self.monitor.is_none() {
                    if self.por_bocinas() {
                        self.avisar(crate::idioma::t("Conecta audífonos para escucharte mientras grabas"), false);
                    } else {
                        self.encender_monitor();
                        self.monitor_auto = self.monitor.is_some();
                    }
                }
                self.grab = Some(GrabacionActiva {
                    g,
                    pista,
                    inicio: self.ahora(),
                    arrancada: false,
                    abierta: Instant::now(),
                    reloj: None,
                    nivel: 0.0,
                });
            }
            Err(e) => self.avisar(e, true),
        }
    }

    fn apagar_monitor(&mut self) {
        self.monitor = None;
        self.motor.lock().unwrap().vivo = None;
    }

    /// Micrófono y bocinas juntos chillan.
    fn por_bocinas(&self) -> bool {
        let s = self.salida.as_ref().map(|s| s.nombre.to_lowercase()).unwrap_or_default();
        ["bocina", "altavo", "speaker"].iter().any(|b| s.contains(b))
    }

    /// Enciende o apaga «Escucharme».
    fn alternar_monitor(&mut self) {
        self.monitor_auto = false;
        if self.monitor.is_some() {
            self.apagar_monitor();
        } else if self.por_bocinas() {
            self.avisar_bocinas = true;
        } else {
            self.encender_monitor();
        }
    }

    fn encender_monitor(&mut self) {
        if !self.p.pistas.iter().any(|p| p.armada) {
            let id = match self.canal {
                Canal::Pista(id) => id,
                _ => self.p.nueva_pista_grabacion(),
            };
            self.p.armar(id);
        }
        match Monitor::iniciar(self.mic.as_deref()) {
            Ok(m) => {
                self.motor.lock().unwrap().vivo = Some(m.vivo.clone());
                self.monitor = Some(m);
            }
            Err(e) => self.avisar(e, true),
        }
    }

    /// Arranca la música cuando el micrófono ya entrega sonido y guarda el par de relojes.
    fn atender_grabacion(&mut self) {
        let Some(a) = &mut self.grab else { return };
        a.nivel = a.g.nivel().max(a.nivel * 0.85);
        if !a.arrancada {
            if a.g.primera().is_some() {
                a.arrancada = true;
                let mut m = self.motor.lock().unwrap();
                m.grabando = true;
                m.tocando = true;
            } else if a.abierta.elapsed() > Duration::from_secs(20) {
                self.grab = None;
                self.avisar(crate::idioma::t("El micrófono no entrega sonido. Revisa el permiso en Ajustes del Sistema → Privacidad y seguridad → Micrófono."), true);
            }
            return;
        }
        let r = *self.reloj.lock().unwrap();
        if r.tocando && a.reloj.is_none() && r.pos > a.inicio + 0.2 {
            a.reloj = Some(r);
        }
    }

    pub fn parar_grabacion(&mut self) {
        let Some(mut a) = self.grab.take() else { return };
        if std::mem::take(&mut self.monitor_auto) {
            self.apagar_monitor();
        }
        let t = self.ahora();
        {
            let mut m = self.motor.lock().unwrap();
            m.tocando = false;
            m.grabando = false;
        }
        self.ir(t);
        if let Err(e) = a.g.terminar() {
            self.avisar(e, true);
            return;
        }
        let (Some(r), Some(cap)) = (a.reloj, a.g.primera()) else {
            let _ = std::fs::remove_file(&a.g.ruta);
            self.avisar(crate::idioma::t("La toma fue demasiado corta"), true);
            return;
        };
        // Segundo de la canción que se oía cuando entró la primera muestra.
        let t0 = r.en(cap) - self.p.ajuste_ms as f64 / 1000.0;
        let (inicio, desde) = if t0 < a.inicio { (a.inicio, a.inicio - t0) } else { (t0, 0.0) };
        let ruta = a.g.ruta.clone();
        self.leer_audio(ruta, Destino2::Grabacion { pista: a.pista, inicio, desde }, "Leyendo la toma");
    }

    // ---- tareas

    fn tarea(&mut self, nombre: &str) -> (u64, Arc<AtomicU32>, Arc<AtomicBool>) {
        let id = self.sig_tarea;
        self.sig_tarea += 1;
        let (a, c) = (Arc::new(AtomicU32::new(0)), Arc::new(AtomicBool::new(false)));
        self.tareas.insert(id, Tarea { nombre: nombre.into(), avance: a.clone(), cancelar: c.clone() });
        (id, a, c)
    }

    pub fn ocupada(&self) -> bool {
        !self.tareas.is_empty()
    }

    fn leer_audio(&mut self, ruta: PathBuf, destino: Destino2, nombre: &str) {
        let (tarea, ..) = self.tarea(nombre);
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let res = medios::decodificar(&ruta);
            let _ = tx.send(Evento::Audio { tarea, ruta, res, destino });
        });
    }

    pub fn importar(&mut self, ruta: PathBuf) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let r = medios::sondear(&ruta);
            let _ = tx.send(Evento::Sondeado(ruta, r));
        });
    }

    /// Agrega el video y su audio. Lo llama el diálogo de importación.
    pub fn importar_video(&mut self, ruta: PathBuf, info: Info, modo: AudioVideo) {
        let Some((ancho, alto, fps)) = info.video else { return };
        let inicio = self.p.video.iter().filter(|c| c.capa == 0).map(|c| c.fin()).fold(0.0, f64::max);
        if self.p.video.is_empty() && !self.p.fijo {
            (self.p.ancho, self.p.alto, self.p.fps) = (ancho, alto, fps);
        }
        let mut c = self.p.nuevo_video(ruta.clone(), Contenido::Archivo, inicio, info.dur);
        (c.ancho, c.alto, c.fps) = (ancho, alto, fps);
        self.clip = Some(c.id);
        let con = vec![c.id];
        self.p.video.push(c);
        self.linea.encuadrar = true;
        if !info.audio {
            return;
        }
        match modo {
            AudioVideo::Junto => self.leer_audio(
                ruta,
                Destino2::PistaNueva { nombre: crate::idioma::t("Audio del video").into(), inicio, molde: None, con },
                "Leyendo el audio",
            ),
            AudioVideo::Separado => self.separar(ruta, inicio, None, con),
        }
    }

    /// Pone el audio de un clip de video en una pista, alineado y con el mismo recorte.
    pub fn traer_audio(&mut self, c: &ClipVideo) {
        let molde = Clip {
            id: 0,
            ruta: c.ruta.clone(),
            nombre: String::new(),
            inicio: c.inicio,
            desde: c.desde,
            dur: c.dur,
            total: c.total,
            ganancia_db: 0.0,
            fade_in: 0.0,
            fade_out: 0.0,
            invertido: false,
            grupo: None,
            velocidad: c.mov.velocidad, bus: None, color: None
        };
        let d = Destino2::PistaNueva { nombre: crate::idioma::t("Audio del video").into(), inicio: c.inicio, molde: Some(molde), con: vec![c.id] };
        self.leer_audio(c.ruta.clone(), d, "Leyendo el audio");
    }

    pub fn cargar_muestra(&mut self, ruta: PathBuf) {
        if !self.almacen.contains_key(&ruta) {
            self.leer_audio(ruta, Destino2::Nada, "Leyendo el sonido");
        }
    }

    /// Separa voz e instrumental de un archivo en dos pistas nuevas. Con `molde` conserva el recorte de ese clip.
    pub fn separar(&mut self, ruta: PathBuf, inicio: f64, molde: Option<Clip>, con: Vec<u64>) {
        let (tarea, av, ca) = self.tarea("Separando voz e instrumental");
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let res = separar::separar_archivo(&ruta, &carpeta().join("Separados"), &av, &ca);
            let _ = tx.send(Evento::Separado { tarea, ruta, res, inicio, molde, con });
        });
    }

    pub fn exportar_a(&mut self, ruta: PathBuf, modo: Modo) {
        let (tarea, av, ca) = self.tarea(if modo == Modo::Video { "Exportando video" } else { "Exportando audio" });
        let (tx, p, al) = (self.tx.clone(), self.p.clone(), self.almacen.clone());
        std::thread::spawn(move || {
            let res = exportar::exportar(&p, &al, &ruta, modo, &av, &ca);
            let _ = tx.send(Evento::Exportado { tarea, ruta, res });
        });
    }

    /// `clip` es el principal y siempre está dentro de la selección; lo que ya no existe se olvida.
    fn cuadrar_seleccion(&mut self) {
        let vivos: HashSet<u64> = self.p.tramos().iter().map(|t| t.id).collect();
        self.seleccion.retain(|x| vivos.contains(x));
        self.clip = self.clip.filter(|x| vivos.contains(x));
        match self.clip {
            Some(id) if !self.seleccion.contains(&id) => {
                self.seleccion.clear();
                self.seleccion.insert(id);
            }
            None if !self.seleccion.is_empty() => self.clip = self.seleccion.iter().copied().next(),
            _ => {}
        }
    }

    /// Guarda como imagen el cuadro del cursor y lo pone encima.
    pub fn congelar(&mut self, id: u64) {
        let t = self.ahora();
        let Some(c) = self.p.clip_video(id).filter(|c| t >= c.inicio && t < c.fin()).cloned() else {
            self.avisar(crate::idioma::t("Pon el cursor sobre el clip"), false);
            return;
        };
        let tx = self.tx.clone();
        let ts = c.fuente(t);
        let dir = carpeta().join("Cuadros");
        let ruta = dir.join(format!("cuadro-{}-{:.0}.png", self.p.sig_id, ts * 1000.0));
        std::thread::spawn(move || {
            let _ = std::fs::create_dir_all(&dir);
            let s = medios::ffmpeg().args(["-y", "-ss", &format!("{ts:.4}"), "-i"]).arg(&c.ruta).args(["-frames:v", "1"]).arg(&ruta).status();
            let ok = s.is_ok_and(|s| s.success()) && ruta.exists();
            let _ = tx.send(Evento::Congelado { clip: id, t, ruta, ok });
        });
    }

    /// Prepara el sonido de los clips con otra velocidad.
    fn pedir_velocidades(&mut self) {
        let faltan: Vec<(PathBuf, PathBuf, f64)> = self
            .p
            .pistas
            .iter()
            .flat_map(|p| &p.clips)
            .filter(|c| c.clave() != c.ruta && !self.almacen.contains_key(&c.clave()))
            .map(|c| (c.clave(), c.ruta.clone(), c.velocidad))
            .collect();
        for (clave, ruta, v) in faltan {
            if self.pidiendo.insert(clave.clone()) {
                let tx = self.tx.clone();
                std::thread::spawn(move || {
                    let res = medios::decodificar_a_velocidad(&ruta, v);
                    let _ = tx.send(Evento::Estirado { clave, res });
                });
            }
        }
    }

    pub fn importar_srt(&mut self) {
        let Some(r) = rfd::FileDialog::new().add_filter(crate::idioma::t("Subtítulos"), &["srt"]).pick_file() else { return };
        let Ok(s) = std::fs::read_to_string(&r) else {
            self.avisar(crate::idioma::t("No pude leer los subtítulos"), true);
            return;
        };
        self.poner_subtitulos(&s);
    }

    pub fn poner_subtitulos(&mut self, s: &str) {
        let v = crate::edicion::leer_srt(s);
        if v.is_empty() {
            self.avisar(crate::idioma::t("Ese archivo no trae subtítulos SRT"), true);
            return;
        }
        let capa = self.p.capas.len();
        self.p.capas.push(Capa { nombre: "Subtítulos".into(), visible: true, bloqueada: false });
        let (_, base, y) = crate::titulos::plantilla(2);
        let n = v.len();
        for (t, dur, texto) in v {
            let mut c = self.p.nuevo_video("Subtítulo".into(), Contenido::Titulo(Titulo { texto, ..base.clone() }), t, dur);
            (c.capa, c.mov.y) = (capa, Prop::num(y));
            self.p.video.push(c);
        }
        self.avisar(crate::idioma::tf("{} subtítulos en su propia pista", &[&n]), false);
    }

    pub fn muestras_listas(&self) -> bool {
        self.muestras_hechas >= crate::estilos::ESTILOS.len() + crate::filtros::FILTROS.len() + crate::nombres::EFECTOS.len() + crate::nombres::TRANSICIONES.len()
    }

    /// Rehace las miniaturas con el cuadro que se está viendo.
    pub fn pedir_muestras(&mut self) {
        let base = match &self.cuadro {
            Some(q) if !self.p.video.is_empty() => muestras::reducir(q.ancho, q.alto, &q.rgba),
            _ => muestras::de_prueba(),
        };
        // En un cuadro casi negro no se ve nada.
        let base = if base.iter().map(|x| *x as u64).sum::<u64>() / (base.len() as u64) < 64 + 16 { muestras::de_prueba() } else { base };
        self.muestras_pedidas = true;
        self.muestras_hechas = 0;
        let tx = self.tx.clone();
        let suena = self.suena.clone();
        std::thread::spawn(move || {
            muestras::calcular(base, |k, q| {
                // Las miniaturas no le quitan procesador a la reproducción.
                while suena.load(Relaxed) {
                    std::thread::sleep(Duration::from_millis(120));
                }
                tx.send(Evento::Muestra(k, q)).is_ok()
            })
        });
    }

    fn importar_imagen(&mut self, ruta: PathBuf, info: Info) {
        let Some((ancho, alto, _)) = info.video else { return };
        let t = self.ahora();
        let capa = if self.p.video.is_empty() { 0 } else { self.capa_libre(t, 5.0) };
        let mut c = self.p.nuevo_video(ruta, Contenido::Imagen, t, 5.0);
        (c.ancho, c.alto, c.capa) = (ancho, alto, capa);
        self.clip = Some(c.id);
        self.p.video.push(c);
    }

    /// Pide las miniaturas que falten.
    fn pedir_tiras(&mut self) {
        for c in self.p.video.iter().filter(|c| c.contenido == Contenido::Archivo) {
            if !self.tiras.contains_key(&c.ruta) {
                self.tiras.insert(c.ruta.clone(), None);
                let (tx, r, dur, razon) = (self.tx.clone(), c.ruta.clone(), c.total, c.ancho as f64 / c.alto.max(1) as f64);
                std::thread::spawn(move || {
                    let t = video::tira(&r, dur, razon);
                    let _ = tx.send(Evento::Tira(r, t));
                });
            }
        }
    }

    fn eventos(&mut self, ctx: &Context) {
        while let Ok(e) = self.rx.try_recv() {
            match e {
                Evento::Sondeado(_, Err(e)) => self.avisar(e, true),
                Evento::Muestra(k, q) => {
                    let img = ColorImage::from_rgba_unmultiplied([muestras::AN, muestras::AL], &q);
                    self.muestras_hechas += 1;
                    match self.muestras.get_mut(&k) {
                        Some(t) => t.set(img, TextureOptions::LINEAR),
                        None => {
                            self.muestras.insert(k, ctx.load_texture("muestra", img, TextureOptions::LINEAR));
                        }
                    }
                }
                Evento::Tira(ruta, t) => {
                    let lista = t.map(|t| TiraLista {
                        paso: t.paso,
                        tex: t
                            .rgba
                            .iter()
                            .map(|q| ctx.load_texture("mini", ColorImage::from_rgba_unmultiplied([t.ancho, t.alto], q), TextureOptions::LINEAR))
                            .collect(),
                    });
                    self.tiras.insert(ruta, lista);
                }
                Evento::Sondeado(ruta, Ok(info)) => {
                    if es_imagen(&ruta) {
                        self.importar_imagen(ruta, info);
                    } else if info.video.is_some() && info.audio {
                        self.preguntar = Some((ruta, info));
                    } else if info.video.is_some() {
                        self.importar_video(ruta, info, AudioVideo::Junto);
                    } else {
                        let nombre = nombre_de(&ruta);
                        self.leer_audio(
                            ruta,
                            Destino2::PistaNueva { nombre, inicio: 0.0, molde: None, con: vec![] },
                            "Leyendo el audio",
                        );
                    }
                }
                Evento::Audio { tarea, ruta, res, destino } => {
                    self.tareas.remove(&tarea);
                    match res {
                        Err(e) => self.avisar(format!("{}: {e}", nombre_de(&ruta)), true),
                        Ok(a) => {
                            let a = Arc::new(a);
                            self.almacen.insert(ruta.clone(), a.clone());
                            self.aplicado = None;
                            self.colocar(ruta, &a, destino);
                        }
                    }
                }
                Evento::Separado { tarea, ruta, res, inicio, molde, con } => {
                    self.tareas.remove(&tarea);
                    match res {
                        Err(e) => {
                            self.avisar(crate::idioma::tf("{}. Dejé el audio completo.", &[&e]), true);
                            if molde.is_none() {
                                let d = Destino2::PistaNueva { nombre: crate::idioma::t("Audio del video").into(), inicio, molde: None, con };
                                self.leer_audio(ruta, d, "Leyendo el audio");
                            }
                        }
                        Ok((voz, ins)) => {
                            for (r, n) in [(ins, "Instrumental"), (voz, "Voz original")] {
                                let d = Destino2::PistaNueva { nombre: n.into(), inicio, molde: molde.clone(), con: con.clone() };
                                self.leer_audio(r, d, "Leyendo el audio");
                            }
                            // Las dos pistas reemplazan al clip completo (se puede deshacer).
                            if let Some(m) = molde {
                                self.p.borrar_clip(m.id);
                            }
                        }
                    }
                }
                Evento::Estirado { clave, res } => match res {
                    Ok(a) => {
                        self.almacen.insert(clave, Arc::new(a));
                        self.aplicado = None;
                    }
                    Err(e) => self.avisar(crate::idioma::tf("No pude cambiar la velocidad del audio: {}", &[&e]), true),
                },
                Evento::Congelado { clip, t, ruta, ok } => {
                    let c = self.p.clip_video(clip).cloned();
                    match c.filter(|_| ok) {
                        Some(c) => {
                            let capa = self.capa_libre(t, 2.0);
                            let mut n = self.p.nuevo_video(ruta, Contenido::Imagen, t, 2.0);
                            (n.ancho, n.alto, n.capa, n.mov, n.fx) = (c.ancho, c.alto, capa, c.mov.clone(), c.fx.clone());
                            for f in &mut n.fx {
                                f.id = self.p.id();
                            }
                            self.clip = Some(n.id);
                            self.p.video.push(n);
                        }
                        None => self.avisar(crate::idioma::t("No pude congelar el cuadro"), true),
                    }
                }
                Evento::Exportado { tarea, ruta, res } => {
                    self.tareas.remove(&tarea);
                    match res {
                        Ok(()) => self.avisar(crate::idioma::tf("Listo: {}", &[&(ruta.display())]), false),
                        Err(e) => self.avisar(e, true),
                    }
                }
            }
        }
    }

    fn colocar(&mut self, ruta: PathBuf, a: &Audio, destino: Destino2) {
        let nombre = nombre_de(&ruta);
        let total = a.dur();
        let mut clip = Clip {
            id: self.p.id(),
            ruta,
            nombre,
            inicio: 0.0,
            desde: 0.0,
            dur: total,
            total,
            ganancia_db: 0.0,
            fade_in: 0.0,
            fade_out: 0.0,
            invertido: false,
            grupo: None,
            velocidad: 1.0, bus: None, color: None
        };
        let mut vincular: Vec<u64> = vec![];
        let pista = match destino {
            Destino2::Nada => return,
            Destino2::PistaNueva { nombre, inicio, molde, con } => {
                vincular = con;
                clip.inicio = inicio;
                clip.nombre = crate::idioma::tf("{} · {}", &[&nombre, &clip.nombre]);
                if let Some(m) = molde {
                    clip = Clip { id: clip.id, ruta: clip.ruta, nombre: clip.nombre, total, grupo: None, ..m };
                    clip.dur = clip.dur.min((total - clip.desde) / clip.velocidad).max(0.05);
                }
                self.p.nueva_pista(&nombre)
            }
            Destino2::Grabacion { pista, inicio, desde } => {
                if total - desde < 0.1 {
                    self.avisar(crate::idioma::t("La toma fue demasiado corta"), true);
                    return;
                }
                (clip.inicio, clip.desde, clip.dur) = (inicio, desde, total - desde);
                clip.nombre = crate::idioma::tf("Toma {}", &[&(self.p.pista(pista).map_or(1, |p| p.clips.len() + 1))]);
                // Pudo borrarse la pista mientras se leía la toma.
                if self.p.pista(pista).is_some() { pista } else { self.p.nueva_pista("Grabación") }
            }
        };
        self.clip = Some(clip.id);
        self.canal = Canal::Pista(pista);
        let id = clip.id;
        if let Some(p) = self.p.pista(pista) {
            p.clips.push(clip);
        }
        if !vincular.is_empty() {
            // Se suma al grupo que ya tenga el video (voz e instrumental comparten).
            let g = self.p.tramo_de(vincular[0]).and_then(|t| t.grupo);
            match g {
                Some(g) => {
                    if let Some(c) = self.p.clip(id) {
                        c.grupo = Some(g);
                    }
                }
                None => {
                    vincular.push(id);
                    self.p.vincular(&vincular);
                }
            }
        }
        self.linea.encuadrar = true;
    }

    // ---- proyecto

    /// Entrega los cambios al motor.
    pub fn sincronizar(&mut self) {
        if self.aplicado.as_deref() == Some(&self.p) {
            return;
        }
        // Un cambio de imagen no vuelve a preparar el sonido.
        if !self.aplicado.as_deref().is_some_and(|a| misma_mezcla(a, &self.p)) {
            let ids = self.motor.lock().unwrap().ids_fx();
            let n = motor::preparar(&self.p, &self.almacen, &ids);
            let _viejo = self.motor.lock().unwrap().aplicar(n);
        }
        self.vista_p = Arc::new(self.p.clone());
        self.aplicado = Some(self.vista_p.clone());
        self.vista_v += 1;
    }

    /// Como `p != guardado`, sin comparar otra vez si nada cambió.
    fn por_guardar(&self) -> bool {
        let (v, r) = self.sucio.get();
        if v == self.vista_v {
            return r;
        }
        let r = self.p != self.guardado;
        self.sucio.set((self.vista_v, r));
        r
    }

    fn historial(&mut self, ctx: &Context) {
        // Va después de `sincronizar`: si la versión es la misma, nada cambió.
        if self.v_historial == self.vista_v || ctx.input(|i| i.pointer.any_down()) {
            return;
        }
        self.v_historial = self.vista_v;
        if self.p == self.ultimo {
            return;
        }
        let antes = std::mem::replace(&mut self.ultimo, self.p.clone());
        self.deshacer.push(Foto::de(&antes));
        self.rehacer.clear();
        // Con un tope de pasos y de memoria.
        let mut peso: usize = self.deshacer.iter().map(|f| f.0.len()).sum();
        while self.deshacer.len() > 200 || (peso > 24 << 20 && self.deshacer.len() > 20) {
            peso -= self.deshacer.remove(0).0.len();
        }
        // Respaldo por si la app se cierra de golpe, en su propio hilo.
        if !self.sin_dispositivo {
            let _ = self.respaldo.send(self.p.clone());
        }
    }

    fn volver(&mut self, atras: bool) {
        let (de, a) = if atras { (&mut self.deshacer, &mut self.rehacer) } else { (&mut self.rehacer, &mut self.deshacer) };
        if let Some(q) = de.pop().and_then(|f| f.abrir()) {
            a.push(Foto::de(&std::mem::replace(&mut self.p, q)));
            self.p.sig_id = self.p.sig_id.max(self.ultimo.sig_id);
            self.ultimo = self.p.clone();
        }
    }

    pub fn guardar_en(&mut self, ruta: PathBuf) {
        let mini = self.cuadro.as_ref().filter(|_| !self.p.video.is_empty()).map(|q| muestras::reducir(q.ancho, q.alto, &q.rgba));
        let mini = mini.as_ref().map(|q| (muestras::AN as u16, muestras::AL as u16, q.as_slice()));
        match crate::archivo::guardar(&self.p, &ruta, mini, false) {
            Ok(()) => {
                self.guardado = self.p.clone();
                self.sucio.set((0, false));
                self.avisar(crate::idioma::tf("Guardado en {}", &[&(ruta.display())]), false);
                if !self.sin_dispositivo {
                    inicio::anotar(&ruta);
                }
                self.ruta = Some(ruta);
            }
            Err(e) => self.avisar(crate::idioma::tf("No pude guardar: {}", &[&e]), true),
        }
    }

    pub fn abrir(&mut self, ruta: PathBuf) {
        match crate::archivo::abrir(&ruta, &carpeta().join("Medios").join(nombre_de(&ruta))) {
            Err(e) => self.avisar(crate::idioma::tf("No pude abrir: {}", &[&e]), true),
            Ok(p) => {
                self.detener();
                self.poner(p);
                if !self.sin_dispositivo {
                    inicio::anotar(&ruta);
                }
                self.ruta = Some(ruta);
                let mut rutas: Vec<PathBuf> = self.p.pistas.iter().flat_map(|t| &t.clips).map(|c| c.ruta.clone()).collect();
                rutas.extend(self.p.pistas.iter().filter_map(|t| match &t.instrumento {
                    Some(Instrumento::Muestra { ruta, .. }) => Some(ruta.clone()),
                    _ => None,
                }));
                rutas.sort();
                rutas.dedup();
                for r in rutas {
                    if !self.almacen.contains_key(&r) {
                        self.leer_audio(r, Destino2::Nada, "Leyendo el audio");
                    }
                }
            }
        }
    }

    fn poner(&mut self, p: Proyecto) {
        self.canal = p.pistas.first().map_or(Canal::Maestro, |x| Canal::Pista(x.id));
        self.clip = None;
        self.ultimo = p.clone();
        self.guardado = p.clone();
        self.sucio.set((0, false));
        self.p = p;
        self.deshacer.clear();
        self.rehacer.clear();
        self.linea.encuadrar = true;
        self.aplicado = None;
        self.inicio.cargar();
    }

    fn dialogo(&self) -> rfd::FileDialog {
        rfd::FileDialog::new()
    }

    fn sin_guardar(&self) -> bool {
        self.p != self.guardado || self.ocupada() || self.grab.is_some()
    }

    fn pedir(&mut self, que: Confirmar) {
        if self.sin_guardar() {
            self.confirmar = Some(que);
        } else {
            self.hacer(que, &Context::default());
        }
    }

    fn hacer(&mut self, que: Confirmar, ctx: &Context) {
        match que {
            Confirmar::Salir => {
                self.cerrar_ya = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            Confirmar::Nuevo => {
                self.detener();
                let mut p = Proyecto::nuevo();
                if let Some((w, h)) = self.inicio.formato.take() {
                    (p.ancho, p.alto, p.fijo) = (w, h, true);
                }
                self.poner(p);
                self.ruta = None;
            }
            Confirmar::Abrir => {
                if let Some(r) = self.inicio.pendiente.take() {
                    self.abrir(r);
                } else if let Some(r) = self.dialogo().add_filter(crate::idioma::t("Proyecto de Beethoven Craft"), &["bcraft", "toma"]).pick_file() {
                    self.abrir(r);
                }
            }
        }
    }

    fn menu_guardar(&mut self, como: bool) {
        let r = match (&self.ruta, como) {
            (Some(r), false) => Some(r.clone()),
            _ => self.dialogo().set_file_name(crate::idioma::t("Mi proyecto.bcraft")).add_filter(crate::idioma::t("Proyecto de Beethoven Craft"), &["bcraft", "toma"]).save_file(),
        };
        if let Some(r) = r {
            self.guardar_en(r.with_extension("bcraft"));
        }
    }

    /// Un solo archivo con el proyecto y todos sus videos y audios adentro.
    fn menu_guardar_todo(&mut self) {
        let Some(r) = self.dialogo().set_file_name(crate::idioma::t("Mi proyecto completo.bcraft")).add_filter(crate::idioma::t("Proyecto de Beethoven Craft"), &["bcraft"]).save_file() else { return };
        let r = r.with_extension("bcraft");
        let (tarea, ..) = self.tarea("Guardando el proyecto con todo incluido");
        let (tx, p) = (self.tx.clone(), self.p.clone());
        std::thread::spawn(move || {
            let res = crate::archivo::guardar(&p, &r, None, true);
            let _ = tx.send(Evento::Exportado { tarea, ruta: r, res });
        });
    }

    pub fn menu_importar(&mut self) {
        let f = ["mp4", "mov", "mkv", "webm", "avi", "m4v", "png", "jpg", "jpeg", "webp", "mp3", "wav", "m4a", "flac", "ogg", "opus", "aac", "aif", "aiff"];
        for r in self.dialogo().add_filter(crate::idioma::t("Audio, video e imágenes"), &f).pick_files().unwrap_or_default() {
            self.importar(r);
        }
    }

    fn menu_exportar(&mut self, modo: Modo) {
        let d = self.dialogo();
        let d = match modo {
            Modo::Video => d.set_file_name(crate::idioma::t("Mi video.mp4")).add_filter(crate::idioma::t("Video MP4"), &["mp4"]),
            Modo::Audio => d
                .set_file_name(crate::idioma::t("Mi audio.mp3"))
                .add_filter(crate::idioma::t("MP3"), &["mp3"])
                .add_filter(crate::idioma::t("WAV 24 bits"), &["wav"])
                .add_filter(crate::idioma::t("FLAC"), &["flac"])
                .add_filter("M4A", &["m4a"]),
        };
        if let Some(mut r) = d.save_file() {
            if r.extension().is_none() {
                r.set_extension(if modo == Modo::Video { "mp4" } else { "mp3" });
            }
            self.exportar_a(r, modo);
        }
    }

    pub fn dividir(&mut self) {
        let t = self.ahora();
        let bajo = |x: &crate::edicion::Tramo| t > x.inicio + 0.01 && t < x.fin() - 0.01;
        let ids: Vec<u64> = match self.clip {
            Some(id) => vec![id],
            // Sin nada elegido: lo de la pista elegida, o todo lo visible en el cursor.
            None => match self.canal {
                Canal::Pista(p) => self.p.tramos().iter().filter(|x| x.carril == crate::edicion::Carril::Audio(p) && bajo(x)).map(|x| x.id).collect(),
                _ => self.p.tramos().iter().filter(|x| bajo(x)).map(|x| x.id).collect(),
            },
        };
        let hecho = ids.into_iter().fold(false, |h, id| self.p.dividir_grupo(id, t) || h);
        if !hecho {
            self.avisar(crate::idioma::t("Elige un clip y pon el cursor de reproducción sobre él para dividirlo"), false);
        }
    }

    pub fn borrar_clip(&mut self) {
        self.borrar_elegidos(false);
    }

    pub fn borrar_elegidos(&mut self, cerrar: bool) {
        let mut ids: Vec<u64> = self.seleccion.drain().collect();
        ids.extend(self.clip.take());
        ids.sort();
        ids.dedup();
        for id in ids {
            if cerrar {
                self.p.borrar_ondulado(id);
            } else {
                self.p.borrar_grupo(id);
            }
        }
    }

    /// Salta al corte anterior o siguiente.
    fn saltar_corte(&mut self, d: i32) {
        let t = self.ahora();
        let v = self.p.cortes();
        let n = if d > 0 { v.iter().find(|x| **x > t + 0.01) } else { v.iter().rev().find(|x| **x < t - 0.01) };
        if let Some(x) = n.copied() {
            self.ir(x);
        }
    }

    /// Recorta hasta el cursor la orilla del clip que está debajo y cierra el hueco.
    fn recortar_al_cursor(&mut self, o: crate::edicion::Orilla) {
        use crate::edicion::{Carril, Orilla};
        let t = self.ahora();
        let v = self.p.tramos();
        let x = self.clip.and_then(|id| v.iter().find(|x| x.id == id)).filter(|x| t > x.inicio && t < x.fin()).or_else(|| {
            v.iter().find(|x| matches!(x.carril, Carril::Video(_)) && t > x.inicio + 0.01 && t < x.fin() - 0.01)
        });
        let Some(x) = x.copied() else { return };
        match o {
            Orilla::Izq => {
                self.p.recorte_ondulado(x.id, o, t - x.inicio);
                self.ir(x.inicio);
            }
            Orilla::Der => {
                self.p.recorte_ondulado(x.id, o, t - x.fin());
            }
        }
    }

    /// Disolvencia cruzada en la entrada del clip elegido.
    fn transicion_rapida(&mut self) {
        match self.clip.filter(|id| self.p.video.iter().any(|c| c.id == *id)) {
            Some(id) => self.poner_transicion(id, "cross_dissolve", false),
            None => self.avisar(crate::idioma::t("Elige un clip de video"), false),
        }
    }

    fn teclas(&mut self, ctx: &Context) {
        if ctx.egui_wants_keyboard_input() || self.preguntar.is_some() || self.atajos.abierto {
            return;
        }
        for id in ctx.input_mut(|i| crate::atajos::atender(&mut i.events)) {
            self.hacer_atajo(id);
        }
    }

    /// Lo que hace cada atajo de teclado (`crate::atajos::ACCIONES`).
    pub fn hacer_atajo(&mut self, id: &str) {
        let paso = 1.0 / self.p.fps.max(1.0);
        let ahora = self.ahora();
        let animando = self.espacio == Espacio::Animacion;
        match id {
            "guardar" => self.menu_guardar(false),
            "guardar_como" => self.menu_guardar(true),
            "abrir" => self.pedir(Confirmar::Abrir),
            "importar" => self.menu_importar(),
            "exportar" => self.exportar_op = Some(Default::default()),
            "menu" => {
                self.pausar();
                self.inicio.mostrar();
            }
            "atajos" => self.atajos.abierto = true,
            "ajustes" => self.abrir_ajustes_proyecto(),
            "archivos" => self.archivos.rec.abierto = !self.archivos.rec.abierto,
            "tocar" => self.alternar(),
            "pausa" if self.tocando() => self.alternar(),
            "seguir" if !self.tocando() => self.alternar(),
            "regresar5" => self.ir(ahora - 5.0),
            "detener" => self.detener(),
            "grabar" => self.grabar(),
            "cuadro_atras" => self.ir(ahora - paso),
            "cuadro_adelante" => self.ir(ahora + paso),
            "cinco_atras" => self.ir(ahora - 5.0 * paso),
            "cinco_adelante" => self.ir(ahora + 5.0 * paso),
            "corte_antes" => self.saltar_corte(-1),
            "corte_despues" => self.saltar_corte(1),
            "inicio" => self.ir(0.0),
            "fin" => self.ir(self.p.fin()),
            "deshacer" => self.volver(true),
            "rehacer" => self.volver(false),
            "dividir" => self.dividir(),
            "borrar" if animando && !self.anim.claves.is_empty() => self.borrar_claves(),
            "borrar" if self.espacio == Espacio::Nodos => self.quitar_nodo(),
            "borrar" => self.borrar_elegidos(false),
            "borrar_cerrar" => self.borrar_elegidos(true),
            "todo" => self.seleccion = self.p.tramos().iter().map(|x| x.id).collect(),
            "elegir" => self.clip = (0..self.p.capas.len()).rev().find_map(|i| self.p.video_en(i, ahora)).map(|c| c.id),
            "recortar_izq" => self.recortar_al_cursor(crate::edicion::Orilla::Izq),
            "recortar_der" => self.recortar_al_cursor(crate::edicion::Orilla::Der),
            "transicion" => self.transicion_rapida(),
            "marca" => {
                let n = self.p.marcas.len() + 1;
                self.p.marcas.push(Marca { t: ahora, nombre: crate::idioma::tf("Marca {}", &[&n]) });
            }
            "entrada" => self.p.tramo.0 = Some(ahora),
            "salida" => self.p.tramo.1 = Some(ahora),
            "sin_tramo" => self.p.tramo = (None, None),
            "levantar" => {
                if let (Some(a), Some(b)) = self.p.tramo {
                    self.p.quitar_rango(a, b, false);
                }
            }
            "extraer" => {
                if let (Some(a), Some(b)) = self.p.tramo {
                    self.p.quitar_rango(a, b, true);
                    self.p.tramo = (None, None);
                    self.ir(a);
                }
            }
            "vacio" => {
                let (t, dur) = self.hueco_vacio();
                self.clip_vacio(t, dur);
            }
            "titulo" => self.poner_titulo(0, ahora, None),
            "acercar" => self.acercar(1.5, 300.0),
            "alejar" => self.acercar(0.66, 300.0),
            "ver_todo" => self.linea.encuadrar = true,
            "iman" => self.linea.iman = !self.linea.iman,
            "p_playlist" => self.abrir_pestana(Espacio::Playlist),
            "p_video" => self.abrir_pestana(Espacio::Video),
            "p_audio" => self.abrir_pestana(Espacio::Audio),
            "p_efectos" => self.abrir_pestana(Espacio::Efectos),
            "p_animacion" => self.abrir_pestana(Espacio::Animacion),
            "p_nodos" => self.abrir_pestana(Espacio::Nodos),
            "animar" => {
                if let Some(id) = self.clip.filter(|id| self.p.video.iter().any(|c| c.id == *id)) {
                    self.animar(id);
                }
            }
            "clave" => self.clave_de_todo(),
            "clave_antes" => self.saltar_clave(-1),
            "clave_despues" => self.saltar_clave(1),
            "auto" => self.anim.auto = !self.anim.auto,
            "copiar_claves" if animando => self.copiar_claves(),
            "pegar_claves" if animando => self.pegar_claves(),
            h => {
                if let Some(x) = h.strip_prefix('h').and_then(|n| n.parse::<usize>().ok()).and_then(|n| linea::HERRAMIENTAS.get(n)) {
                    self.linea.herramienta = x.0;
                }
            }
        }
    }

    fn abrir_pestana(&mut self, e: Espacio) {
        if !self.pestanas.contains(&e) {
            self.pestanas.push(e);
        }
        self.espacio = e;
    }

    /// Los audios largos viven en disco: lo que va a sonar en los próximos segundos se trae antes, en otro hilo.
    fn preparar_audio(&self) {
        let t = self.ahora();
        let v: Vec<(Arc<Audio>, f64, f64)> = self
            .p
            .pistas
            .iter()
            .flat_map(|p| &p.clips)
            .filter(|c| c.inicio < t + 8.0 && c.inicio + c.dur > t)
            .filter_map(|c| {
                // Una versión con otra velocidad o al revés corre por su cuenta: se trae con holgura.
                let a = self.almacen.get(&c.clave())?.clone();
                let d = c.desde / c.velocidad.max(0.01) + (t - c.inicio).max(0.0);
                let d = if c.invertido { (c.desde + c.dur - (t - c.inicio).max(0.0) - 8.0).max(0.0) } else { d };
                Some((a, d - 0.5, d + 8.5))
            })
            .collect();
        if !v.is_empty() {
            std::thread::spawn(move || v.iter().for_each(|(a, i, j)| a.preparar(*i, *j)));
        }
    }

    // ---- dibujo

    pub fn ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        self.cuadrar_seleccion();
        self.eventos(&ctx);
        self.pedir_tiras();
        self.pedir_velocidades();
        if !self.inicio.visible {
            self.atender_teclado();
        }
        self.atender_grabacion();
        if self.salida.as_ref().is_some_and(|s| s.rota.load(Relaxed)) {
            self.abrir_salida();
        }
        for r in ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().into()).collect::<Vec<PathBuf>>()) {
            if es_proyecto(&r) {
                self.abrir(r);
            } else {
                self.importar(r);
            }
        }
        if !self.inicio.visible {
            self.teclas(&ctx);
        }
        {
            let (p, m) = self.motor.lock().unwrap().picos();
            let baja = |v: &mut [f32; 2], n: [f32; 2]| (0..2).for_each(|k| v[k] = n[k].max(v[k] * 0.86));
            for (id, n) in p {
                baja(self.picos.entry(id).or_default(), n);
            }
            baja(&mut self.pico_maestro, m);
        }
        let suena = self.tocando();
        self.suena.store(suena, Relaxed);
        if suena && self.audio_listo.elapsed() > Duration::from_millis(800) {
            self.audio_listo = Instant::now();
            self.preparar_audio();
        }
        self.inicio.ajustar_ventana(&ctx);
        if self.inicio.visible {
            self.pantalla_inicio(ui);
        } else {
            Panel::top("tira").frame(Frame::NONE).show_separator_line(false).show(ui, |ui| self.tira_pestanas(ui));
            Panel::top("barra").frame(Frame::new().fill(TOOLBAR).inner_margin(6.0)).show(ui, |ui| self.barra(ui));
            self.refrescar_vista(&ctx);
            self.panel_archivos(ui);
            // Una pestaña sacada a su ventana deja aquí la Playlist.
            match self.espacio {
                Espacio::Video if !self.ventanas[0] => self.espacio_video(ui),
                Espacio::Audio if !self.ventanas[1] => self.espacio_audio(ui, true),
                Espacio::Efectos if !self.ventanas[2] => self.espacio_efectos(ui, true),
                Espacio::Animacion => self.espacio_animacion(ui),
                Espacio::Nodos => self.espacio_nodos(ui),
                _ => self.espacio_playlist(ui),
            }
            self.ventanas_ui(&ctx);
        }
        self.cuadrar_seleccion();
        self.ventana_atajos(&ctx);
        self.dialogos(&ctx);
        self.sincronizar();
        self.historial(&ctx);
        if self.tocando() || self.grab.is_some() || self.ocupada() || self.monitor.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else if self.pico_maestro[0] > 0.001 {
            ctx.request_repaint_after(Duration::from_millis(33));
        } else {
            ctx.request_repaint_after(Duration::from_millis(120));
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.cerrar_ya {
            if self.sin_guardar() {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                self.confirmar = Some(Confirmar::Salir);
            }
        }
    }

    fn transporte(&mut self, ui: &mut Ui) {
        let tocando = self.tocando();
        let grabando = self.grab.is_some();
        if boton(ui, &tabler::PLAYER_PLAY, tocando, "Reproducir / pausa (espacio)").clicked() {
            self.alternar();
        }
        if boton(ui, &tabler::PLAYER_STOP, false, "Detener y volver al inicio").clicked() {
            self.detener();
        }
        if boton_rec(ui, grabando).clicked() {
            self.grabar();
        }
    }

    fn barra(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.set_min_height(36.0);
            ui.menu_button(crate::idioma::t("ARCHIVO"), |ui| {
                ui.set_min_width(220.0);
                if ui.button(crate::idioma::t("Menú principal")).clicked() {
                    self.pausar();
                    self.inicio.mostrar();
                }
                ui.separator();
                if ui.button(crate::idioma::t("Proyecto nuevo")).clicked() {
                    self.pedir(Confirmar::Nuevo);
                }
                if ui.button(crate::idioma::t("Abrir proyecto…")).clicked() {
                    self.pedir(Confirmar::Abrir);
                }
                if ui.add(Button::new(crate::idioma::t("Guardar")).shortcut_text(crate::atajos::menu("guardar"))).clicked() {
                    self.menu_guardar(false);
                }
                if ui.button(crate::idioma::t("Guardar como…")).clicked() {
                    self.menu_guardar(true);
                }
                if ui.button(crate::idioma::t("Guardar con todo incluido…")).on_hover_text(crate::idioma::t("Un solo archivo con tus videos y audios adentro, para respaldar o llevar a otra Mac")).clicked() {
                    self.menu_guardar_todo();
                }
                if ui.add(Button::new(crate::idioma::t("Ajustes del proyecto…")).shortcut_text(crate::atajos::menu("ajustes"))).on_hover_text(crate::idioma::t("Resolución y fotogramas por segundo")).clicked() {
                    self.abrir_ajustes_proyecto();
                }
                ui.separator();
                if ui.add(Button::new(crate::idioma::t("Importar video o audio…")).shortcut_text(crate::atajos::menu("importar"))).clicked() {
                    self.menu_importar();
                }
                ui.separator();
                if ui.add(Button::new(crate::idioma::t("Exportar video con mi audio…")).shortcut_text(crate::atajos::menu("exportar"))).clicked() {
                    self.exportar_op = Some(Default::default());
                }
                if ui.button(crate::idioma::t("Exportar solo el audio…")).clicked() {
                    self.menu_exportar(Modo::Audio);
                }
            });
            ui.menu_button(crate::idioma::t("EDITAR"), |ui| {
                ui.set_min_width(220.0);
                if ui.add(Button::new(crate::idioma::t("Deshacer")).shortcut_text(crate::atajos::menu("deshacer"))).clicked() {
                    self.volver(true);
                }
                if ui.add(Button::new(crate::idioma::t("Rehacer")).shortcut_text(crate::atajos::menu("rehacer"))).clicked() {
                    self.volver(false);
                }
                ui.separator();
                if ui.add(Button::new(crate::idioma::t("Dividir en el cursor")).shortcut_text(crate::atajos::menu("dividir"))).clicked() {
                    self.dividir();
                }
                if ui.add(Button::new(crate::idioma::t("Borrar clip")).shortcut_text(crate::atajos::menu("borrar"))).clicked() {
                    self.borrar_clip();
                }
                ui.separator();
                if ui.add(Button::new(crate::idioma::t("Atajos de teclado…")).shortcut_text(crate::atajos::menu("atajos"))).clicked() {
                    self.atajos.abierto = true;
                }
            });
            ui.menu_button(crate::idioma::t("AGREGAR"), |ui| {
                ui.set_min_width(220.0);
                if ui.add(Button::new(crate::idioma::t("Clip vacío (efectos para todo)")).shortcut_text(crate::atajos::menu("vacio"))).on_hover_text(crate::idioma::t(linea::AYUDA_VACIO)).clicked() {
                    self.hacer_atajo("vacio");
                }
                if ui.add(Button::new(crate::idioma::t("Texto")).shortcut_text(crate::atajos::menu("titulo"))).clicked() {
                    self.hacer_atajo("titulo");
                }
                ui.menu_button(crate::idioma::t("Figura"), |ui| {
                    for (f, n) in TipoForma::TODOS {
                        if ui.button(crate::idioma::t(n)).clicked() {
                            self.poner_forma(f);
                        }
                    }
                });
                ui.separator();
                if ui.button(crate::idioma::t("Pista de grabación (con efectos de voz)")).clicked() {
                    let id = self.p.nueva_pista_grabacion();
                    self.canal = Canal::Pista(id);
                }
                ui.menu_button(crate::idioma::t("Instrumento"), |ui| self.menu_instrumentos(ui));
                if ui.button(crate::idioma::t("Pista de audio vacía")).clicked() {
                    let n = self.p.pistas.len() + 1;
                    let id = self.p.nueva_pista(&crate::idioma::tf("Pista {}", &[&n]));
                    self.canal = Canal::Pista(id);
                }
            });
            ui.menu_button(crate::idioma::t("AYUDA"), |ui| {
                if ui.button(crate::idioma::t("Acerca de Beethoven Craft")).clicked() {
                    self.acerca = true;
                }
            });
            ui.separator();
            self.transporte(ui);
            let grabando = self.grab.is_some();
            // Reloj
            let (r, _) = ui.allocate_exact_size(vec2(150.0, 34.0), Sense::hover());
            ui.painter().rect_filled(r, 6, c(0x1b1b1b));
            ui.painter().rect_filled(r.shrink(1.0), 5, TIME_PANEL);
            let col = if grabando { RECORD } else { MAIN };
            ui.painter().text(r.center(), Align2::CENTER_CENTER, tiempo(self.ahora()), fuente(27.0), col);
            ui.separator();
            if boton(ui, &tabler::ARROW_BACK_UP, false, "Deshacer (⌘Z)").clicked() {
                self.volver(true);
            }
            if boton(ui, &tabler::SLICE, false, "Dividir el clip en el cursor (S)").clicked() {
                self.dividir();
            }
            self.tempo_ui(ui);
            ui.separator();
            let iman = self.linea.iman;
            if boton(ui, &tabler::MAGNET, iman, "Imán: pega los clips a los bordes cercanos").clicked() {
                self.linea.iman = !iman;
            }
            if boton(ui, &tabler::DEVICE_FLOPPY, false, "Guardar (⌘S)").clicked() {
                self.menu_guardar(false);
            }
            ui.separator();
            if let Some(g) = &self.grab {
                let (r, _) = ui.allocate_exact_size(vec2(10.0, 32.0), Sense::hover());
                medidor(ui.painter(), r, [g.nivel; 2]);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (r, _) = ui.allocate_exact_size(vec2(16.0, 34.0), Sense::hover());
                medidor(ui.painter(), r, self.pico_maestro);
                perilla(ui, &mut self.p.maestro_db, -40.0, 6.0, 0.0, false, 16.0, "Volumen general")
                    .on_hover_text(crate::idioma::tf("Volumen general: {} dB", &[&format!("{:.1}", self.p.maestro_db)]));
            });
        });
        // Segundo renglón: tareas y avisos.
        if !self.tareas.is_empty() {
            let mut lista: Vec<_> = self.tareas.iter().collect();
            lista.sort_by_key(|t| t.0);
            ui.horizontal_wrapped(|ui| {
                for (_, t) in lista {
                    let a = t.avance.load(Relaxed) as f32 / 1000.0;
                    ui.add(Spinner::new().size(14.0).color(MAIN));
                    ui.label(crate::idioma::t(&t.nombre));
                    if a > 0.0 {
                        ui.add(ProgressBar::new(a).desired_width(160.0).text(format!("{:.0} %", a * 100.0)).fill(DEEP));
                        if ui.small_button(crate::idioma::t("Cancelar")).clicked() {
                            t.cancelar.store(true, Relaxed);
                        }
                    }
                }
            });
        }
        self.mensaje_ui(ui);
    }

    /// El último aviso, mientras dura.
    fn mensaje_ui(&self, ui: &mut Ui) {
        if let Some((m, error, t)) = &self.mensaje {
            let dura = if *error { 12.0 } else { 6.0 };
            if t.elapsed().as_secs_f32() < dura {
                let col = if *error { c(0xff8a80) } else { HIGHLIGHT };
                ui.colored_label(col, m.as_str());
            }
        }
    }

    /// Ganancia y fundidos de un clip.
    pub fn perillas_clip(&mut self, ui: &mut Ui, id: u64) {
        let Some(k) = self.p.clip(id) else { return };
        let (mut g, mut fi, mut fo) = (k.ganancia_db, k.fade_in as f32, k.fade_out as f32);
        let max = (k.dur as f32).min(20.0);
        ui.horizontal(|ui| {
            for (n, v, min, max, txt) in [
                ("Ganancia", &mut g, -24.0, 24.0, " dB"),
                ("Entrada", &mut fi, 0.0, max, " s"),
                ("Salida", &mut fo, 0.0, max, " s"),
            ] {
                ui.allocate_ui_with_layout(vec2(80.0, 58.0), Layout::top_down(Align::Center), |ui| {
                    ui.set_width(80.0);
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.label(RichText::new(crate::idioma::t(&n)).size(12.0).color(TEXT_DIM));
                    perilla(ui, v, min, max, 0.0, false, 14.0, n);
                    let s = if min < 0.0 { format!("{v:+.1}{txt}") } else { format!("{v:.2}{txt}") };
                    ui.label(RichText::new(crate::idioma::t(&s)).size(12.0));
                });
            }
        });
        k.ganancia_db = g;
        k.fade_in = (fi as f64).min(k.dur);
        k.fade_out = (fo as f64).min(k.dur - k.fade_in);
    }

    /// Pide el cuadro que toca y sube el que haya llegado.
    fn refrescar_vista(&mut self, ctx: &Context) {
        let t = self.ahora();
        // El último cuadro sigue visible al llegar al final.
        let t = if self.p.fin() > 0.0 { t.min(self.p.fin() - 0.001).max(0.0) } else { t };
        let quiere = if self.calidad > 0 {
            self.calidad
        } else {
            // El escalón que cubre lo que se ve; sin tarjeta gráfica, uno que aguante el procesador.
            let tope = if crate::vgpu::gpu().is_some() { 1440 } else { 540 };
            [360, 540, 720, 900, 1080, 1440].into_iter().find(|h| *h >= self.alto_panel).unwrap_or(1440).min(tope)
        };
        let h = quiere.min(self.p.alto.max(2) as usize) / 2 * 2;
        let w = (h as f64 * self.p.ancho as f64 / self.p.alto.max(1) as f64 / 2.0).round() as usize * 2;
        let tocando = self.tocando();
        if let Some(q) = self.vista.actualizar(&self.vista_p, self.vista_v, t, tocando, (w.max(2), h.max(2))) {
            // El cuadro es opaco: se copia de un jalón.
            let img = ColorImage::new([q.ancho, q.alto], bytemuck::cast_slice(&q.rgba).to_vec());
            self.cuadro_v += 1;
            match &mut self.textura {
                Some(x) => x.set(img, TextureOptions::LINEAR),
                None => self.textura = Some(ctx.load_texture("video", img, TextureOptions::LINEAR)),
            }
            self.cuadro = Some(q);
        }
        if self.vista.pendientes > 0 {
            ctx.request_repaint_after(Duration::from_millis(12));
        }
    }

    fn preview(&mut self, ui: &mut Ui) {
        let todo = ui.available_rect_before_wrap();
        let barra = Rect::from_min_max(pos2(todo.left(), todo.bottom() - 34.0), todo.max);
        let r = Rect::from_min_max(todo.min, pos2(todo.right(), barra.top()));
        let resp = ui.allocate_rect(todo, Sense::hover());
        // Archivos soltados en la imagen: en el cursor.
        let trae = resp.dnd_hover_payload::<biblioteca::Soltar>().is_some_and(|s| matches!(*s, biblioteca::Soltar::Medios(_)));
        if let Some(s) = resp.dnd_release_payload::<biblioteca::Soltar>() {
            if let biblioteca::Soltar::Medios(v) = &*s {
                let t = self.ahora();
                self.poner_archivos(v, t, None, None);
            }
        }
        let razon = self.p.ancho as f32 / self.p.alto.max(1) as f32;
        let k = (r.width() / razon).min(r.height());
        let d = Rect::from_center_size(r.center(), vec2(k * razon, k));
        self.alto_panel = (d.height() * ui.ctx().pixels_per_point()).round().max(2.0) as usize;
        match &self.textura {
            Some(x) if !self.p.video.is_empty() => {
                ui.painter().image(x.id(), d, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            _ => {
                ui.painter().text(r.center(), Align2::CENTER_CENTER, crate::idioma::t("Arrastra aquí tu video, canción o imagen"), fuente(18.0), TEXT_FAINT);
            }
        }
        if trae {
            ui.painter().rect_stroke(r.shrink(6.0), 8, Stroke::new(2.0, MAIN), StrokeKind::Inside);
        }
        // En la pestaña de animación lo hace ella, con todo lo que cambie.
        let antes = (self.anim.auto && self.espacio != Espacio::Animacion).then(|| self.clip.and_then(|id| self.p.video.iter().find(|c| c.id == id).cloned())).flatten();
        self.mover_en_vista(ui, d);
        self.camino(ui, d);
        self.barra_libre(ui, r);
        if let Some(a) = antes {
            self.claves_automaticas(&a);
        }
        ui.painter().rect_filled(barra, 0, WINDOW_TITLE);
        let mut u = ui.new_child(UiBuilder::new().max_rect(barra.shrink2(vec2(8.0, 3.0))).layout(Layout::left_to_right(Align::Center)));
        u.label(RichText::new(tiempo(self.ahora())).color(MAIN).size(15.0));
        let paso = 1.0 / self.p.fps.max(1.0);
        let t = self.ahora();
        u.add_space((barra.width() / 2.0 - 190.0).max(4.0));
        if boton_color(&mut u, &tabler::CARET_LEFT, false, "Un cuadro atrás (←)", GLYPH, 26.0).clicked() {
            self.ir(t - paso);
        }
        let tocando = self.tocando();
        if boton_color(&mut u, &tabler::PLAYER_PLAY, tocando, "Reproducir / pausa", GLYPH, 26.0).clicked() {
            self.alternar();
        }
        if boton_color(&mut u, &tabler::CARET_RIGHT, false, "Un cuadro adelante (→)", GLYPH, 26.0).clicked() {
            self.ir(t + paso);
        }
        u.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ComboBox::from_id_salt("calidad").width(86.0).selected_text(match self.calidad {
                0 => crate::idioma::t("Automática"),
                360 => crate::idioma::t("Rápida"),
                720 => crate::idioma::t("Nítida"),
                1080 => crate::idioma::t("Completa"),
                _ => crate::idioma::t("Normal"),
            }).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.calidad, 0, crate::idioma::t("Automática")).on_hover_text(crate::idioma::t("Tan nítida como se ve en tu pantalla"));
                ui.selectable_value(&mut self.calidad, 360, crate::idioma::t("Rápida"));
                ui.selectable_value(&mut self.calidad, 540, crate::idioma::t("Normal"));
                ui.selectable_value(&mut self.calidad, 720, crate::idioma::t("Nítida"));
                ui.selectable_value(&mut self.calidad, 1080, crate::idioma::t("Completa"));
                ui.separator();
                let mut usa = crate::vgpu::encendida();
                if ui.checkbox(&mut usa, crate::idioma::t("Usar la tarjeta gráfica")).changed() {
                    crate::vgpu::apagar(!usa);
                    crate::video::SIN_CHIP.store(!usa, Relaxed);
                    self.vista_v += 1;
                }
                ui.label(RichText::new(crate::vgpu::gpu().map_or("", |g| g.nombre())).size(12.0).color(TEXT_FAINT));
            }).response.on_hover_text(crate::idioma::t("Calidad de la vista previa. No cambia lo que se exporta."));
            ui.label(RichText::new(tiempo(self.p.fin())).color(TEXT_DIM));
            if ui.available_width() > 130.0 && ui.add(Button::new(RichText::new(format!("{}×{} · {}", self.p.ancho, self.p.alto, proyecto::nombre_fps((self.p.fps * 100.0).round() / 100.0))).size(13.0).color(TEXT_DIM)).frame(false)).on_hover_text(crate::idioma::t("Resolución y fotogramas por segundo del proyecto")).clicked() {
                self.abrir_ajustes_proyecto();
            }
        });
    }

    /// Mover y escalar el clip elegido directamente sobre la imagen.
    fn mover_en_vista(&mut self, ui: &mut Ui, d: Rect) {
        let ahora = self.ahora();
        let Some(id) = self.clip else { return };
        let (pw, ph) = (self.p.ancho as usize, self.p.alto as usize);
        let Some(c) = self.p.clip_video(id).filter(|c| ahora >= c.inicio && ahora <= c.fin()) else { return };
        let t = ahora - c.inicio;
        let (aw, ah) = match c.contenido {
            Contenido::Archivo | Contenido::Imagen => crate::composicion::ajuste(c, pw, ph),
            _ => (pw, ph),
        };
        let e = c.mov.escala.en(t).num() as f32 / 100.0;
        let est = vec2(c.mov.ancho.en(t).num() as f32 / 100.0, c.mov.alto.en(t).num() as f32 / 100.0);
        let (aw, ah) = match &c.contenido {
            Contenido::Forma(f) => ((f.tam[0] / 100.0 * ph as f32) as usize, (f.tam[1] / 100.0 * ph as f32) as usize),
            _ => (aw, ah),
        };
        let k = d.width() / pw.max(1) as f32;
        let centro = d.center() + vec2(c.mov.x.en(t).num() as f32 / 100.0 * d.width(), c.mov.y.en(t).num() as f32 / 100.0 * d.height());
        let medio = vec2(aw as f32, ah as f32) * k * e / 2.0;
        let giro = emath::Rot2::from_angle((c.mov.giro.en(t).num() as f32).to_radians());
        let esq = [vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(1.0, 1.0), vec2(-1.0, 1.0)].map(|v| centro + giro * (v * medio * est));
        let p = ui.painter_at(d.expand(1.0));
        p.add(Shape::closed_line(esq.to_vec(), Stroke::new(1.0, MAIN.gamma_multiply(0.7))));
        let asa = Rect::from_center_size(d.clamp(esq[2]), Vec2::splat(14.0));
        p.rect_filled(asa.shrink(3.0), 2, MAIN);
        // Asa de giro, arriba del clip.
        let arriba = d.clamp(centro + giro * vec2(0.0, -medio.y * est.y - 22.0));
        let asa_giro = Rect::from_center_size(arriba, Vec2::splat(16.0));
        p.circle(arriba, 5.0, WINDOW_TITLE, Stroke::new(1.5, MAIN));
        let cuerpo = ui.interact(d, Id::new("vista-mover").with(ui.id()), Sense::click_and_drag());
        cuerpo.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, crate::idioma::t("Imagen")));
        cuerpo.context_menu(|ui| {
            ui.set_min_width(210.0);
            self.menu_llaves(ui, id, ahora);
        });
        if self.anim.libre.modo != libre::Modo::Mover {
            return self.vista_libre(ui, d, &cuerpo);
        }
        let Some(c) = self.p.clip_video(id) else { return };
        let resp = ui.interact(asa, Id::new("vista-escala").with(ui.id()), Sense::drag());
        let rgiro = ui.interact(asa_giro, Id::new("vista-giro").with(ui.id()), Sense::drag()).on_hover_text(crate::idioma::t("Girar (Shift: de 15 en 15 grados)"));
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeNwSe);
        }
        if let (true, Some(m)) = (rgiro.dragged(), rgiro.interact_pointer_pos()) {
            let v = m - centro;
            let mut g = (v.x.atan2(-v.y)).to_degrees() as f64;
            if ui.input(|i| i.modifiers.shift) {
                g = (g / 15.0).round() * 15.0;
            }
            // La vuelta más cercana a la que ya llevaba.
            let antes = c.mov.giro.en(t).num();
            g += ((antes - g) / 360.0).round() * 360.0;
            c.mov.giro.poner(t, Valor::Num(g));
        } else if let (true, Some(m)) = (resp.dragged(), resp.interact_pointer_pos()) {
            let base = (vec2(aw as f32, ah as f32) * k / 2.0).length().max(1.0);
            c.mov.escala.poner(t, Valor::Num(((m - centro).length() / base * 100.0).clamp(1.0, 500.0) as f64));
        } else if cuerpo.dragged() {
            let q = cuerpo.drag_delta();
            let (x, y) = (c.mov.x.en(t).num(), c.mov.y.en(t).num());
            c.mov.x.poner(t, Valor::Num(x + (q.x / d.width()) as f64 * 100.0));
            c.mov.y.poner(t, Valor::Num(y + (q.y / d.height()) as f64 * 100.0));
        }
    }

    /// Pestañas sacadas a su propia ventana. Comparten el mismo proyecto.
    fn ventanas_ui(&mut self, ctx: &Context) {
        for (i, n) in ["Video", "Audio", "Efectos"].into_iter().enumerate() {
            if !self.ventanas[i] {
                continue;
            }
            let b = ViewportBuilder::default().with_title(crate::idioma::tf("Beethoven Craft · {}", &[&crate::idioma::t(n)])).with_inner_size([1180.0, 700.0]).with_min_inner_size([640.0, 420.0]);
            ctx.show_viewport_immediate(ViewportId::from_hash_of(n), b, |ui, _| {
                let vctx = ui.ctx().clone();
                self.teclas(&vctx);
                match i {
                    0 => self.ventana_video(ui),
                    1 => self.espacio_audio(ui, false),
                    _ => self.espacio_efectos(ui, false),
                }
                if vctx.input(|i| i.viewport().close_requested()) {
                    self.ventanas[i] = false;
                }
            });
        }
    }

    /// Pestaña de VIDEO: controles a la izquierda, programa al centro, biblioteca a la derecha y abajo las pistas con
    /// sus herramientas y medidores.
    fn espacio_video(&mut self, ui: &mut Ui) {
        self.zona_pistas(ui, linea::Que::Todo, 0.46);
        Panel::left("v-izq").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(Margin::symmetric(6, 5))).resizable(true).default_size(370.0).min_size(280.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                for (p, n) in [(PanelIzq::Origen, "Origen"), (PanelIzq::Controles, "Controles"), (PanelIzq::Color, "Color")] {
                    if biblioteca::pastilla(ui, n, self.panel_izq == p).clicked() {
                        self.panel_izq = p;
                    }
                }
            });
            ui.add_space(3.0);
            match self.panel_izq {
                PanelIzq::Origen => self.ui_origen(ui),
                PanelIzq::Controles => self.inspector(ui),
                PanelIzq::Color => self.ui_color(ui),
            }
        });
        Panel::right("v-der").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(8.0)).resizable(true).default_size(290.0).min_size(200.0).show(ui, |ui| self.biblioteca(ui, false));
        CentralPanel::default().frame(Frame::new().fill(Color32::BLACK)).show(ui, |ui| self.preview(ui));
    }

    /// Pistas con la tira de herramientas a la izquierda y el medidor general a la derecha.
    fn zona_pistas(&mut self, ui: &mut Ui, que: linea::Que, parte: f32) {
        let alto = ui.available_height();
        Panel::bottom("pistas").frame(Frame::new().fill(WORKSPACE)).resizable(true).default_size((alto * parte).max(220.0)).size_range(140.0..=(alto - 160.0).max(200.0)).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            Panel::left("herramientas").frame(Frame::new().fill(TOOLBAR).inner_margin(Margin::symmetric(4, 4))).resizable(false).exact_size(38.0).show(ui, |ui| {
                ScrollArea::vertical().id_salt("herr").show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for (n, (h, i, _, ayuda)) in linea::HERRAMIENTAS.into_iter().enumerate() {
                        // «Cuchilla (C): corta» con la tecla que tenga ahora.
                        let ayuda = crate::idioma::t(ayuda);
                        let tecla = crate::atajos::menu(&format!("h{n}"));
                        let ayuda = match (ayuda.split_once(" ("), ayuda.split_once("):")) {
                            (Some((a, _)), Some((_, b))) if tecla.is_empty() => format!("{a}:{b}"),
                            (Some((a, _)), Some((_, b))) => format!("{a} ({tecla}):{b}"),
                            _ => ayuda.to_string(),
                        };
                        if boton_color(ui, i, self.linea.herramienta == h, &ayuda, GLYPH, 28.0).clicked() {
                            self.linea.herramienta = h;
                        }
                    }
                });
            });
            Panel::right("medidor-gral").frame(Frame::new().fill(TOOLBAR).inner_margin(5.0)).resizable(false).exact_size(34.0).show(ui, |ui| {
                let (r, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
                medidor(ui.painter(), r, self.pico_maestro);
            });
            Panel::bottom("pie-linea").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(Margin::symmetric(6, 3))).resizable(false).exact_size(30.0).show(ui, |ui| self.pie_linea(ui));
            CentralPanel::default().frame(Frame::new().fill(GRID_GAP)).show(ui, |ui| self.ui_linea(ui, que));
        });
    }

    /// Pestaña de AUDIO: consola, rack, piano roll y editor, con las pistas de sonido abajo.
    fn espacio_audio(&mut self, ui: &mut Ui, con_pistas: bool) {
        Panel::top("audio-barra").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(5.0)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.set_min_height(32.0);
                musica::pestanas(ui, &mut self.pestana);
                ui.separator();
                self.tempo_ui(ui);
                ui.menu_button(RichText::new(crate::idioma::t("+ Instrumento")).color(MAIN), |ui| self.menu_instrumentos(ui));
                ui.separator();
                if led(ui, self.escuchar, "Escucharme al grabar").clicked() {
                    self.escuchar = !self.escuchar;
                }
                ui.label(crate::idioma::t("Escucharme al grabar"));
                let on = self.monitor.is_some();
                if biblioteca::pastilla(ui, "Escucharme ahora", on).clicked() {
                    self.alternar_monitor();
                }
            });
        });
        if con_pistas {
            self.zona_pistas(ui, linea::Que::Audio, 0.36);
        }
        Panel::right("audio-fx").frame(Frame::new().fill(WINDOW_TITLE)).resizable(true).default_size(350.0).show(ui, |ui| {
            if con_pistas && !self.p.video.is_empty() {
                // El video siempre a la vista, en chico.
                let w = ui.available_width();
                let (r, _) = ui.allocate_exact_size(vec2(w, w * self.p.alto as f32 / self.p.ancho.max(1) as f32), Sense::hover());
                ui.painter().rect_filled(r, 0, Color32::BLACK);
                if let Some(x) = &self.textura {
                    ui.painter().image(x.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                }
            }
            if self.pestana != Pestana::Editor {
                self.instrumento_ui(ui);
            }
            if let Some(id) = self.clip.filter(|id| self.p.clip(*id).is_some()) {
                Frame::new().inner_margin(Margin::symmetric(8, 4)).show(ui, |ui| self.ruta_clip(ui, id));
            }
            self.panel_fx(ui)
        });
        if matches!(self.pestana, Pestana::Rack | Pestana::Piano) {
            Panel::left("lista-ins").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(6.0)).resizable(true).default_size(176.0).min_size(130.0).show(ui, |ui| self.lista_instrumentos(ui));
        }
        CentralPanel::default().frame(Frame::new().fill(WORKSPACE)).show(ui, |ui| match self.pestana {
            Pestana::Consola => self.ui_consola(ui),
            Pestana::Editor => self.ui_editor(ui),
            Pestana::Rack => self.ui_rack(ui),
            Pestana::Piano => self.ui_piano(ui),
        });
    }

    /// Pestaña de EFECTOS: catálogo con muestras, vista previa y la cadena de lo elegido.
    fn espacio_efectos(&mut self, ui: &mut Ui, con_pistas: bool) {
        if con_pistas {
            self.zona_pistas(ui, linea::Que::Todo, 0.32);
        }
        if !self.muestras_pedidas {
            self.pedir_muestras();
        }
        Panel::left("e-cat").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(8.0)).resizable(true).default_size(300.0).min_size(240.0).show(ui, |ui| self.catalogo(ui));
        Panel::right("e-cad").frame(Frame::new().fill(WINDOW_TITLE)).resizable(true).default_size(360.0).min_size(280.0).show(ui, |ui| self.inspector(ui));
        CentralPanel::default().frame(Frame::new().fill(Color32::BLACK)).show(ui, |ui| self.preview(ui));
    }

    /// Pestaña PLAYLIST: el resultado final en grande con todas las pistas.
    fn espacio_playlist(&mut self, ui: &mut Ui) {
        self.zona_pistas(ui, linea::Que::Todo, 0.5);
        CentralPanel::default().frame(Frame::new().fill(Color32::BLACK)).show(ui, |ui| self.preview(ui));
    }

    fn ventana_video(&mut self, ui: &mut Ui) {
        Panel::left("video-insp").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(6.0)).resizable(true).default_size(390.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                for (p, n) in [(PanelIzq::Controles, "Controles"), (PanelIzq::Color, "Color")] {
                    if biblioteca::pastilla(ui, n, self.panel_izq == p).clicked() {
                        self.panel_izq = p;
                    }
                }
            });
            if self.panel_izq == PanelIzq::Color { self.ui_color(ui) } else { self.inspector(ui) }
        });
        CentralPanel::default().frame(Frame::new().fill(Color32::BLACK)).show(ui, |ui| self.preview(ui));
    }

    /// Opciones de exportación de video.
    fn dialogo_exportar(&mut self, ctx: &Context) {
        let Some(mut op) = self.exportar_op.clone() else { return };
        let (mut cerrar, mut ir) = (false, false);
        Modal::new(Id::new("exportar")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading(crate::idioma::t("Exportar video con mi audio"));
            ui.add_space(6.0);
            let fila = |ui: &mut Ui, n: &str, f: &mut dyn FnMut(&mut Ui)| {
                ui.horizontal(|ui| {
                    ui.add_sized([110.0, 22.0], Label::new(RichText::new(crate::idioma::t(&n)).color(TEXT_DIM)));
                    f(ui);
                });
            };
            use exportar::Codec;
            fila(ui, "Formato", &mut |ui| {
                for (c, n) in [(Codec::H264, "H.264 (MP4)"), (Codec::H265, "H.265 (MP4)"), (Codec::ProRes, "ProRes (MOV)")] {
                    if biblioteca::pastilla(ui, n, op.codec == c).clicked() {
                        op.codec = c;
                    }
                }
            });
            fila(ui, "Calidad", &mut |ui| {
                for (i, n) in ["Ligera", "Normal", "Alta"].into_iter().enumerate() {
                    if biblioteca::pastilla(ui, n, op.calidad == i).clicked() {
                        op.calidad = i;
                    }
                }
            });
            let (w, h) = (self.p.ancho, self.p.alto);
            let alto = |a: u32| Some(((w as f64 * a as f64 / h.max(1) as f64 / 2.0).round() as u32 * 2, a));
            let tams = [("Original", None), ("720p", alto(720)), ("1080p", alto(1080)), ("480p", alto(480)), ("1440p", alto(1440)), ("4K", alto(2160)), ("Vertical 1080×1920", Some((1080, 1920))), ("Cuadrado 1080", Some((1080, 1080)))];
            fila(ui, "Tamaño", &mut |ui| {
                let n = tams.iter().find(|t| t.1 == op.tam).map_or("Original", |t| t.0);
                ComboBox::from_id_salt("tam").selected_text(n).show_ui(ui, |ui| {
                    for (n, t) in tams {
                        ui.selectable_value(&mut op.tam, t, n);
                    }
                });
            });
            fila(ui, "Fotogramas", &mut |ui| {
                let propios = crate::idioma::tf("Los del proyecto ({})", &[&proyecto::nombre_fps(self.p.fps)]);
                ComboBox::from_id_salt("fps-exp").selected_text(op.fps.map_or(propios.clone(), proyecto::nombre_fps)).show_ui(ui, |ui| {
                    ui.selectable_value(&mut op.fps, None, propios);
                    for f in proyecto::FPS {
                        ui.selectable_value(&mut op.fps, Some(f), proyecto::nombre_fps(f));
                    }
                });
            });
            let marcado = match self.p.tramo {
                (Some(a), Some(b)) if b > a => Some((a, b)),
                _ => None,
            };
            fila(ui, "Tramo", &mut |ui| {
                if biblioteca::pastilla(ui, "Todo", op.tramo.is_none()).clicked() {
                    op.tramo = None;
                }
                match marcado {
                    Some(t) => {
                        if biblioteca::pastilla(ui, &format!("{} – {}", tiempo(t.0), tiempo(t.1)), op.tramo.is_some()).clicked() {
                            op.tramo = Some(t);
                        }
                    }
                    None => {
                        ui.label(RichText::new(crate::idioma::t("marca entrada (I) y salida (O) para exportar un pedazo")).color(TEXT_FAINT).size(12.5));
                    }
                }
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.add_sized([150.0, 32.0], Button::new(RichText::new(crate::idioma::t("Exportar…")).color(MAIN).size(16.0))).clicked() {
                    ir = true;
                }
                if ui.add_sized([100.0, 32.0], Button::new(crate::idioma::t("Cancelar"))).clicked() {
                    cerrar = true;
                }
            });
        });
        self.exportar_op = (!cerrar && !ir).then(|| op.clone());
        if ir {
            let ext = if op.codec == exportar::Codec::ProRes { "mov" } else { "mp4" };
            if let Some(r) = self.dialogo().set_file_name(crate::idioma::tf("Mi video.{}", &[&ext])).add_filter(crate::idioma::t("Video"), &[ext]).save_file() {
                self.exportar_video_a(r.with_extension(ext), op);
            }
        }
    }

    pub fn exportar_video_a(&mut self, ruta: PathBuf, op: exportar::Opciones) {
        let (tarea, av, ca) = self.tarea("Exportando video");
        let (tx, p, al) = (self.tx.clone(), self.p.clone(), self.almacen.clone());
        std::thread::spawn(move || {
            let res = exportar::exportar_con(&p, &al, &ruta, Modo::Video, &op, &av, &ca);
            let _ = tx.send(Evento::Exportado { tarea, ruta, res });
        });
    }

    fn dialogos(&mut self, ctx: &Context) {
        self.dialogo_exportar(ctx);
        self.dialogo_proyecto(ctx);
        if let Some((ruta, info)) = self.preguntar.clone() {
            let mut elegido = None;
            let mut cerrar = false;
            Modal::new(Id::new("importar")).show(ctx, |ui| {
                ui.set_width(460.0);
                ui.heading(crate::idioma::t("Importar video"));
                ui.label(RichText::new(nombre_de(&ruta)).color(TEXT_DIM));
                ui.add_space(8.0);
                ui.label(crate::idioma::t("¿Cómo quieres el audio?"));
                ui.add_space(4.0);
                let b = |ui: &mut Ui, t: &str, d: &str| {
                    let r = ui.add_sized([460.0, 34.0], Button::new(RichText::new(t).size(17.0)));
                    ui.label(RichText::new(d).color(TEXT_DIM).size(13.0));
                    ui.add_space(6.0);
                    r.clicked()
                };
                if b(ui, "Todo junto", "Una sola pista con el audio completo del video.") {
                    elegido = Some(AudioVideo::Junto);
                }
                let dura = format!("{}:{:02}", info.dur as u64 / 60, info.dur as u64 % 60);
                if b(
                    ui,
                    "Separar voz e instrumental",
                    &crate::idioma::tf("Dos pistas: instrumental y voz original. Lo hace tu Mac con IA, sin internet. El video dura {}; tarda más o menos lo mismo.", &[&dura]),
                ) {
                    elegido = Some(AudioVideo::Separado);
                }
                if ui.button(crate::idioma::t("Cancelar")).clicked() {
                    cerrar = true;
                }
            });
            if let Some(m) = elegido {
                self.importar_video(ruta, info, m);
                cerrar = true;
            }
            if cerrar {
                self.preguntar = None;
            }
        }
        if self.avisar_bocinas {
            Modal::new(Id::new("bocinas")).show(ctx, |ui| {
                ui.set_width(380.0);
                ui.heading(crate::idioma::t("Ponte audífonos"));
                ui.label(crate::idioma::t("El sonido sale por bocinas. Si te escuchas por ahí, el micrófono lo vuelve a captar y puede chillar fuerte."));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(crate::idioma::t("Cancelar")).clicked() {
                        self.avisar_bocinas = false;
                    }
                    if ui.button(crate::idioma::t("Activar de todos modos")).clicked() {
                        self.avisar_bocinas = false;
                        self.encender_monitor();
                    }
                });
            });
        }
        if let Some(que) = self.confirmar {
            Modal::new(Id::new("salir")).show(ctx, |ui| {
                ui.set_width(340.0);
                ui.heading(crate::idioma::t("¿Seguir sin guardar?"));
                ui.label(crate::idioma::t("Tienes cambios sin guardar o un trabajo en curso."));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(crate::idioma::t("Guardar")).clicked() {
                        self.confirmar = None;
                        self.menu_guardar(false);
                    }
                    if ui.button(crate::idioma::t("Seguir sin guardar")).clicked() {
                        self.confirmar = None;
                        self.hacer(que, ctx);
                    }
                    if ui.button(crate::idioma::t("Cancelar")).clicked() {
                        self.confirmar = None;
                    }
                });
            });
        }
        if self.acerca {
            Modal::new(Id::new("acerca")).show(ctx, |ui| {
                ui.set_width(420.0);
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(96.0), Sense::hover());
                    inicio::logo(ui, r);
                    ui.vertical(|ui| {
                        ui.heading(crate::idioma::t("Beethoven Craft"));
                        ui.label(crate::idioma::t("Compositor de video y audio."));
                        ui.label(RichText::new(crate::idioma::t("Creado por mixter-ha99")).color(MAIN).size(17.0));
                    });
                });
                ui.add_space(6.0);
                ui.label(crate::idioma::t("Software libre, GPL-3.0 o posterior."));
                ui.label(crate::idioma::t("Efectos de audio y diseño: © 2026 ItsAlphaNeon y colaboradores (GPL-3.0)."));
                ui.label(crate::idioma::t("Efectos de video: EffectCraft; transiciones: FilmCraft; curvas de animación: AniCraft; del equipo de ArtCraft (MIT o Apache-2.0)."));
                ui.label(crate::idioma::t("Separación de voz: modelo MDX-Net Voc FT de Ultimate Vocal Remover (MIT). Letras Cuprum y Barlow (OFL). Íconos Tabler (MIT). Video y audio: ffmpeg."));
                ui.add_space(8.0);
                if ui.button(crate::idioma::t("Cerrar")).clicked() {
                    self.acerca = false;
                }
            });
        }
    }
}

/// Un paso del historial: el proyecto comprimido. Completos llenarían la memoria.
struct Foto(Vec<u8>);

impl Foto {
    fn de(p: &Proyecto) -> Foto {
        Foto(miniz_oxide::deflate::compress_to_vec(&rmp_serde::to_vec_named(p).unwrap_or_default(), 1))
    }
    fn abrir(&self) -> Option<Proyecto> {
        rmp_serde::from_slice(&miniz_oxide::inflate::decompress_to_vec(&self.0).ok()?).ok()
    }
}

/// Guarda el respaldo sin detener la ventana: siempre el más nuevo, y no más de uno cada tres segundos.
fn hilo_respaldo() -> Sender<Proyecto> {
    let (tx, rx) = unbounded::<Proyecto>();
    std::thread::spawn(move || {
        while let Ok(mut p) = rx.recv() {
            std::thread::sleep(Duration::from_millis(1200));
            while let Ok(n) = rx.try_recv() {
                p = n;
            }
            let _ = std::fs::create_dir_all(carpeta());
            let _ = crate::archivo::guardar(&p, &carpeta().join("Autoguardado.bcraft"), None, false);
            std::thread::sleep(Duration::from_millis(1800));
        }
    });
    tx
}

/// ¿Suenan igual? Lo de la imagen no cuenta.
fn misma_mezcla(a: &Proyecto, b: &Proyecto) -> bool {
    a.pistas == b.pistas && a.buses == b.buses && a.maestro_db == b.maestro_db && a.fx_maestro == b.fx_maestro && a.bpm == b.bpm && a.patrones == b.patrones && a.ajuste_ms == b.ajuste_ms && a.fin() == b.fin()
}

pub fn es_imagen(r: &Path) -> bool {
    let e = r.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    ["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff"].contains(&e.as_str())
}

pub fn es_proyecto(r: &Path) -> bool {
    r.extension().is_some_and(|e| e == "bcraft" || e == "toma")
}

pub fn nombre_de(r: &Path) -> String {
    r.file_stem().and_then(|s| s.to_str()).unwrap_or("archivo").to_string()
}

fn boton_rec(ui: &mut Ui, on: bool) -> Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, "Grabar"));
    let p = ui.painter();
    p.rect_filled(r.shrink(1.0), 6, if on { RECORD } else if resp.hovered() { BUTTON_HI } else { BUTTON });
    p.circle_filled(r.center(), 7.0, if on { Color32::WHITE } else { RECORD });
    resp.on_hover_text(crate::idioma::t("Grabar en la pista armada (R)"))
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _: &mut eframe::Frame) {
        App::ui(self, ui);
    }
}
