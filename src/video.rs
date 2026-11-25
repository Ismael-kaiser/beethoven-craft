//! Vista previa: un hilo decodifica con ffmpeg, compone el cuadro y lo entrega; el reloj del audio decide cuál toca.
use crate::composicion::{self, Cuadro, Fuente};
use crate::medios;
use crate::modelo::{ClipVideo, Contenido, Proyecto};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TryRecvError, bounded, unbounded};
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cuadros seguidos de un archivo desde un punto.
pub struct Lector {
    hijo: Child,
    rx: Receiver<Cuadro>,
    /// El archivo que se abrió (el original o su copia ligera).
    ruta: PathBuf,
    tam: (usize, usize),
    ultimo: Option<Arc<Cuadro>>,
    /// Los últimos que pasaron, para los efectos de tiempo.
    cola: std::collections::VecDeque<Arc<Cuadro>>,
    sig: Option<Cuadro>,
    pub fin: bool,
    paso: f64,
    /// Decodifica el chip de video, no el procesador.
    chip: bool,
    /// Lo que entrega primero.
    desde: f64,
}

impl Drop for Lector {
    fn drop(&mut self) {
        let _ = self.hijo.kill();
        let _ = self.hijo.wait();
    }
}

impl Lector {
    pub fn abrir(c: &ClipVideo, desde: f64, tam: (usize, usize), fps_max: f64, m: Modo) -> std::io::Result<Lector> {
        let imagen = c.contenido == Contenido::Imagen;
        let fps = c.fps.min(fps_max).max(1.0);
        let (w, h) = tam;
        // Se alinea a la rejilla de cuadros para que el mismo instante dé siempre el mismo cuadro.
        let desde = ((desde * fps + 0.01).floor() / fps).max(0.0);
        let mut com = medios::ffmpeg();
        let ruta = m.archivo(c);
        let chip = m.chip && !imagen && chip_sirve(&ruta, true) == Some(true);
        // Con el chip, el cuadro se achica en la tarjeta gráfica y baja ya chico: el procesador no carga con el 4K.
        let en_tarjeta = if chip { escala_en_tarjeta(&ruta, false).flatten() } else { None };
        if chip {
            com.args(["-hwaccel", CHIP]);
            if en_tarjeta.is_some() {
                com.args(["-hwaccel_output_format", "videotoolbox_vld"]);
            }
        }
        com.args(["-threads", "4"]);
        if !imagen {
            com.args(["-ss", &format!("{desde:.6}")]);
        }
        com.arg("-i").arg(&ruta).args(["-an", "-sn", "-map", "0:v:0", "-vf"]);
        let yuv = m.yuv && !imagen && w % 2 == 0 && h % 2 == 0 && !medios::con_alfa(&c.ruta);
        let color = if yuv { ":out_color_matrix=bt709:out_range=tv" } else { "" };
        let escala = format!("scale={w}:{h}:flags={}{color},setsar=1", if m.fino { "bicubic" } else { "bilinear" });
        com.arg(match en_tarjeta {
            Some(fmt) => format!("scale_vt=w={w}:h={h},hwdownload,format={fmt},setsar=1,fps={}:start_time=0", medios::fps_fraccion(fps)),
            None if imagen => escala,
            None => format!("fps={}:start_time=0,{escala}", medios::fps_fraccion(fps)),
        });
        if imagen {
            com.args(["-frames:v", "1"]);
        }
        let mut hijo =
            com.args(["-pix_fmt", if yuv { "yuv420p" } else { "rgba" }, "-f", "rawvideo", "pipe:1"]).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
        let mut o = hijo.stdout.take().unwrap();
        let (tx, rx) = bounded(4);
        std::thread::spawn(move || {
            let mut i = 0u64;
            loop {
                let mut d = vec![0u8; if yuv { w * h * 3 / 2 } else { w * h * 4 }];
                if o.read_exact(&mut d).is_err() {
                    return;
                }
                let t = desde + i as f64 / fps;
                i += 1;
                let (rgba, yuv) = if yuv { (vec![], d) } else { (d, vec![]) };
                if tx.send(Cuadro { t, ancho: w, alto: h, rgba, yuv }).is_err() {
                    return;
                }
            }
        });
        Ok(Lector { hijo, rx, ruta, tam, ultimo: None, cola: Default::default(), sig: None, fin: false, paso: 1.0 / fps, chip, desde })
    }

    /// Avanza hasta el cuadro que toca en `t`. Sin `esperar`, entrega lo más nuevo que haya.
    fn hasta(&mut self, t: f64, esperar: bool) -> Option<Arc<Cuadro>> {
        loop {
            if self.sig.is_none() {
                let r = if esperar {
                    self.rx.recv().map_err(|_| TryRecvError::Disconnected)
                } else if self.ultimo.is_none() {
                    // El primero se espera un momento: es el que se pidió ver.
                    self.rx.recv_timeout(Duration::from_secs(4)).map_err(|e| match e {
                        RecvTimeoutError::Timeout => TryRecvError::Empty,
                        RecvTimeoutError::Disconnected => TryRecvError::Disconnected,
                    })
                } else {
                    self.rx.try_recv()
                };
                match r {
                    Ok(c) => self.sig = Some(c),
                    Err(e) => {
                        self.fin |= e == TryRecvError::Disconnected;
                        return self.ultimo.clone();
                    }
                }
            }
            if self.ultimo.is_some() && self.sig.as_ref().unwrap().t > t + 0.001 {
                return self.ultimo.clone();
            }
            self.ultimo = self.sig.take().map(Arc::new);
            if let Some(u) = &self.ultimo {
                self.cola.push_back(u.clone());
                if self.cola.len() > 10 {
                    self.cola.pop_front();
                }
            }
        }
    }

    fn sirve(&self, c: &ClipVideo, t: f64, tam: (usize, usize), tocando: bool, m: &Modo) -> bool {
        if self.ruta != m.archivo(c) || self.tam != tam {
            return false;
        }
        if c.contenido == Contenido::Imagen {
            return true;
        }
        let Some(u) = &self.ultimo else { return !self.fin };
        // Hacia adelante y cerca: más barato seguir leyendo que volver a abrir.
        let lejos = if tocando { 1.5 } else { 0.6 };
        t >= u.t - self.paso * 0.5 && (self.fin || t < u.t + lejos)
    }
}

/// Cómo se decodifica.
#[derive(Clone, Copy, Default)]
pub struct Modo {
    /// Con el chip de video: tarda más en arrancar, pero casi no usa el procesador.
    pub chip: bool,
    /// Planos 4:2:0 en vez de RGBA.
    pub yuv: bool,
    /// Al cambiar de tamaño, mejor calidad.
    pub fino: bool,
    /// De la copia ligera, si hay.
    pub ligera: bool,
}

impl Modo {
    fn archivo(&self, c: &ClipVideo) -> PathBuf {
        if self.ligera && c.contenido == Contenido::Archivo { crate::ligeras::para_ver(&c.ruta) } else { c.ruta.clone() }
    }
}

/// ¿El chip de video de esta máquina sabe leer ese archivo? Se prueba una vez con un cuadro: hay formatos en que ffmpeg falla
/// en vez de seguir con el procesador (AV1 en las Mac M1). Sin `esperar`, `None` mientras se averigua.
fn chip_sirve(ruta: &std::path::Path, esperar: bool) -> Option<bool> {
    use std::sync::Mutex;
    static C: Mutex<Option<HashMap<PathBuf, Option<bool>>>> = Mutex::new(None);
    if SIN_CHIP.load(std::sync::atomic::Ordering::Relaxed) {
        return Some(false);
    }
    match C.lock().unwrap().get_or_insert_with(HashMap::new).get(ruta) {
        Some(Some(si)) => return Some(*si),
        Some(None) if !esperar => return None,
        _ => {}
    }
    let probar = |r: &std::path::Path| {
        let s = medios::ffmpeg().args(["-hwaccel", CHIP, "-i"]).arg(r).args(["-an", "-frames:v", "1", "-vf", "scale=16:16", "-pix_fmt", "rgba", "-f", "rawvideo", "pipe:1"]).stderr(Stdio::null()).output();
        let si = s.is_ok_and(|s| s.status.success() && s.stdout.len() == 16 * 16 * 4);
        C.lock().unwrap().get_or_insert_with(HashMap::new).insert(r.to_path_buf(), Some(si));
        si
    };
    if esperar {
        return Some(probar(ruta));
    }
    C.lock().unwrap().get_or_insert_with(HashMap::new).insert(ruta.to_path_buf(), None);
    let r = ruta.to_path_buf();
    std::thread::spawn(move || probar(&r));
    None
}

const CHIP: &str = if cfg!(target_os = "macos") { "videotoolbox" } else { "auto" };

/// Si los cuadros de ese archivo se pueden achicar en la tarjeta gráfica (Mac): el formato en que bajan (8 o 10 bits), o
/// `Some(None)` si no se puede. Se prueba una vez por archivo; sin `esperar`, la prueba va en segundo plano y mientras
/// tanto da `None` (y se achica con el procesador, como siempre).
fn escala_en_tarjeta(ruta: &std::path::Path, esperar: bool) -> Option<Option<&'static str>> {
    use std::sync::Mutex;
    // Por archivo: `None` mientras se prueba.
    static C: Mutex<Option<HashMap<PathBuf, Option<Option<&'static str>>>>> = Mutex::new(None);
    if !cfg!(target_os = "macos") || SIN_TARJETA.load(std::sync::atomic::Ordering::Relaxed) {
        return Some(None);
    }
    match C.lock().unwrap().get_or_insert_with(HashMap::new).get(ruta) {
        Some(Some(r)) => return Some(*r),
        Some(None) if !esperar => return None,
        _ => {}
    }
    let probar = |r: &std::path::Path| {
        let sirve = |fmt: &str| {
            let s = medios::ffmpeg()
                .args(["-hwaccel", CHIP, "-hwaccel_output_format", "videotoolbox_vld", "-i"])
                .arg(r)
                .args(["-an", "-frames:v", "1", "-vf", &format!("scale_vt=w=16:h=16,hwdownload,format={fmt}"), "-pix_fmt", "rgba", "-f", "rawvideo", "pipe:1"])
                .stderr(Stdio::null())
                .output();
            s.is_ok_and(|s| s.status.success() && s.stdout.len() == 16 * 16 * 4)
        };
        let fmt = ["nv12", "p010le"].into_iter().find(|f| sirve(f));
        C.lock().unwrap().get_or_insert_with(HashMap::new).insert(r.to_path_buf(), Some(fmt));
        fmt
    };
    if esperar {
        return Some(probar(ruta));
    }
    C.lock().unwrap().get_or_insert_with(HashMap::new).insert(ruta.to_path_buf(), None);
    let r = ruta.to_path_buf();
    std::thread::spawn(move || probar(&r));
    None
}

/// Para comparar: achicar siempre con el procesador.
pub static SIN_TARJETA: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// El chip no se usa (o el usuario lo apagó).
pub static SIN_CHIP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Lectores abiertos, uno por clip.
pub struct Lectores {
    v: HashMap<u64, (Lector, u32)>,
    ronda: u32,
    pub fps_max: f64,
    pub tocando: bool,
    /// Exportación: espera cada cuadro en vez de mostrar el más nuevo.
    pub exacto: bool,
    pub modo: Modo,
    /// Se está reproduciendo: los que abrieron con el procesador (arranca antes) le pasan el trabajo al chip.
    pub seguido: bool,
    relevos: HashMap<u64, Lector>,
}

/// Con cuánta anticipación se prepara el relevo del chip (abrirlo con un 4K tarda unos 0,4 s; con menos margen no llega).
const RELEVO: f64 = 1.0;

impl Lectores {
    pub fn nuevos(fps_max: f64, exacto: bool) -> Self {
        Lectores { v: HashMap::new(), ronda: 0, fps_max, tocando: exacto, exacto, modo: Modo::default(), seguido: false, relevos: HashMap::new() }
    }
    pub fn con(mut self, modo: Modo) -> Self {
        self.modo = modo;
        self
    }
    /// Cuántos decodifica el chip de video.
    pub fn con_chip(&self) -> usize {
        self.v.values().filter(|x| x.0.chip).count()
    }
    /// Cierra los que no se usaron en las últimas rondas.
    pub fn limpiar(&mut self) {
        self.ronda += 1;
        let r = self.ronda;
        self.relevos.retain(|id, _| self.v.contains_key(id));
        self.v.retain(|_, (_, u)| r - *u < 40);
    }
}

impl Lectores {
    /// Al reproducir de corrido, se abre otro lector con el chip un poco adelante y se cambia al llegar ahí.
    fn relevar(&mut self, c: &ClipVideo, t: f64, tam: (usize, usize)) {
        let normal = c.contenido == Contenido::Archivo && (c.mov.velocidad - 1.0).abs() < 1e-3;
        if !self.seguido || self.modo.chip || !normal || self.v.get(&c.id).is_none_or(|(l, _)| l.chip || l.fin) {
            return;
        }
        match self.relevos.get(&c.id) {
            None => {
                let ruta = self.modo.archivo(c);
                // Se abre cuando ya se sabe si el chip lee ese archivo y si puede achicar en la tarjeta (en segundo plano).
                if chip_sirve(&ruta, false) != Some(true) || escala_en_tarjeta(&ruta, false).is_none() {
                    return;
                }
                if let Ok(l) = Lector::abrir(c, t + RELEVO, tam, self.fps_max, Modo { chip: true, ..self.modo }) {
                    self.relevos.insert(c.id, l);
                }
            }
            Some(l) if t >= l.desde - 1e-6 || t < l.desde - RELEVO - 0.2 => {
                let l = self.relevos.remove(&c.id).unwrap();
                // Solo si ya entregó algo y sigue en su lugar.
                if t >= l.desde - 1e-6 && t < l.desde + 0.3 && !l.rx.is_empty() && l.tam == tam {
                    if let Some(v) = self.v.get_mut(&c.id) {
                        let cola = std::mem::take(&mut v.0.cola);
                        v.0 = l;
                        v.0.cola = cola;
                    }
                }
            }
            _ => {}
        }
    }
}

impl Fuente for Lectores {
    fn recientes(&self, c: &ClipVideo) -> Vec<Arc<Cuadro>> {
        self.v.get(&c.id).map(|(l, _)| l.cola.iter().cloned().collect()).unwrap_or_default()
    }
    fn cuadro(&mut self, c: &ClipVideo, t: f64, ancho: usize, alto: usize) -> Option<Arc<Cuadro>> {
        let tam = (ancho, alto);
        if !self.v.get(&c.id).is_some_and(|(l, _)| l.sirve(c, t, tam, self.tocando, &self.modo)) {
            self.v.remove(&c.id);
            // Un clip cortado sigue donde iba el de al lado: se toma su lector en vez de abrir otro.
            let libre = self.v.iter().find(|(id, (l, u))| **id != c.id && *u < self.ronda && l.ultimo.is_some() && l.sirve(c, t, tam, self.tocando, &self.modo)).map(|x| *x.0);
            if let Some(l) = libre.and_then(|id| self.v.remove(&id)) {
                self.v.insert(c.id, l);
                let (l, u) = self.v.get_mut(&c.id)?;
                *u = self.ronda;
                return l.hasta(t, self.exacto);
            }
            self.relevos.remove(&c.id);
            // Reproduciendo, se abre de una vez con el chip (y achicando en la tarjeta) si ya se sabe que sirve: con el
            // procesador, un 4K se atrasa en cuanto la máquina tiene otra carga, se reabre y nunca llega al chip.
            let ruta = self.modo.archivo(c);
            // Las pruebas se lanzan (en segundo plano) desde que se ve el clip, para tenerlas al darle play.
            let video = c.contenido == Contenido::Archivo;
            let listo = video && chip_sirve(&ruta, false) == Some(true) && escala_en_tarjeta(&ruta, false).is_some();
            let al_chip = self.seguido && !self.modo.chip && listo;
            let modo = if al_chip { Modo { chip: true, ..self.modo } } else { self.modo };
            self.v.insert(c.id, (Lector::abrir(c, t, tam, self.fps_max, modo).ok()?, 0));
        }
        self.relevar(c, t, tam);
        let (l, u) = self.v.get_mut(&c.id)?;
        *u = self.ronda;
        let q = l.hasta(t, self.exacto);
        if q.is_none() && l.chip && l.fin {
            // El chip no entregó nada: con el procesador.
            *l = Lector::abrir(c, t, tam, self.fps_max, Modo { chip: false, ..self.modo }).ok()?;
            return l.hasta(t, self.exacto);
        }
        q
    }
}

/// Lo que la ventana quiere ver.
pub struct Pedido {
    pub p: Arc<Proyecto>,
    pub version: u64,
    /// Número de cuadro de la vista previa y su segundo.
    i: i64,
    pub t: f64,
    pub tocando: bool,
    pub tam: (usize, usize),
}

/// Escalas de la vista previa mientras se reproduce: si no alcanza, se baja de escalón. El video se decodifica siempre
/// completo y se achica después: cambiar de escalón no vuelve a abrirlo.
const ESCALAS: [f64; 5] = [1.0, 0.8, 0.65, 0.5, 0.4];
/// Cuadros que se componen a la vez.
const OBREROS: usize = 3;
/// Cuánto se calcula por adelantado al reproducir y en pausa, en segundos.
const ADELANTO: f64 = 1.5;
const ADELANTO_PAUSA: f64 = 8.0;
/// Archivo temporal para los cuadros que ya no caben en la RAM; se borra solo.
const DISCO: u64 = 3 << 30;

/// Memoria RAM para los cuadros ya hechos: una parte de la que tiene la máquina.
fn memoria_cuadros() -> usize {
    static M: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *M.get_or_init(|| (medios::memoria_total() / 16).clamp(192 << 20, 2 << 30) as usize)
}

/// El tamaño de la vista previa a la escala `s`, en pares.
pub fn a_escala((w, h): (usize, usize), s: f64) -> (usize, usize) {
    let par = |x: usize| ((x as f64 * s / 2.0).round() as usize * 2).max(2);
    (par(w), par(h))
}

/// Un cuadro decodificado, al tamaño que hace falta.
fn reducir(q: &Arc<Cuadro>, w: usize, h: usize) -> Arc<Cuadro> {
    if (q.ancho, q.alto) == (w, h) {
        return q.clone();
    }
    let d = q.en_rgba();
    let mut rgba = vec![255u8; w * h * 4];
    let tope = |v: usize, n: usize| v.min(n.saturating_sub(1));
    rgba.chunks_exact_mut(w * 4).enumerate().for_each(|(y, fila)| {
        // Dos por dos muestras: alcanza para bajar hasta la cuarta parte sin dientes.
        let ys = [tope((y * 4 + 1) * q.alto / (h * 4), q.alto), tope((y * 4 + 3) * q.alto / (h * 4), q.alto)];
        for (x, o) in fila.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let xs = [tope((x * 4 + 1) * q.ancho / (w * 4), q.ancho), tope((x * 4 + 3) * q.ancho / (w * 4), q.ancho)];
            let mut s = [0u32; 4];
            for yy in ys {
                for xx in xs {
                    let i = (yy * q.ancho + xx) * 4;
                    (0..4).for_each(|k| s[k] += d[i + k] as u32);
                }
            }
            *o = s.map(|v| (v / 4) as u8);
        }
    });
    Arc::new(Cuadro { t: q.t, ancho: w, alto: h, rgba, ..Default::default() })
}

/// Los cuadros de video que necesita una composición, ya decodificados.
struct Surtido {
    cuadros: HashMap<u64, Arc<Cuadro>>,
    pasado: HashMap<u64, Vec<Arc<Cuadro>>>,
}

impl Fuente for Surtido {
    fn cuadro(&mut self, c: &ClipVideo, _: f64, ancho: usize, alto: usize) -> Option<Arc<Cuadro>> {
        self.cuadros.get(&c.id).map(|q| reducir(q, ancho, alto))
    }
    fn cuadro_libre(&mut self, c: &ClipVideo, _: f64, _: usize, _: usize) -> Option<Arc<Cuadro>> {
        self.cuadros.get(&c.id).cloned()
    }
    fn recientes(&self, c: &ClipVideo) -> Vec<Arc<Cuadro>> {
        // Del mismo tamaño que el cuadro de ahora.
        self.pasado.get(&c.id).cloned().unwrap_or_default()
    }
}

/// Un cuadro por componer.
struct Trabajo {
    i: i64,
    firma: u64,
    p: Arc<Proyecto>,
    t: f64,
    tam: (usize, usize),
    nivel: usize,
    surtido: Surtido,
}

struct Hecho {
    i: i64,
    firma: u64,
    tam: (usize, usize),
    nivel: usize,
    q: Arc<Cuadro>,
    seg: f64,
}

struct Guardado {
    firma: u64,
    nivel: usize,
    q: Option<Arc<Cuadro>>,
    /// Dónde quedó en el disco.
    disco: Option<Lugar>,
    /// Última vez que sirvió.
    uso: u64,
}

/// Un cuadro en el disco.
#[derive(Clone, Copy, PartialEq)]
struct Lugar {
    donde: u64,
    ancho: usize,
    alto: usize,
    t: f64,
}

/// Archivo temporal que se llena en círculo: lo nuevo va encima de lo más viejo.
struct Disco {
    f: std::fs::File,
    ruta: PathBuf,
    cursor: u64,
    /// Lo escrito, del más viejo al más nuevo: desde dónde, cuánto y de quién es.
    orden: std::collections::VecDeque<(u64, u64, (i64, (usize, usize)))>,
}

impl Disco {
    fn nuevo() -> Option<Disco> {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let ruta = std::env::temp_dir().join(format!("beethoven-cuadros-{}-{n}", std::process::id()));
        let f = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&ruta).ok()?;
        // Ya abierto se puede borrar del listado: el sistema lo quita al cerrar la app, aunque truene.
        #[cfg(unix)]
        let _ = std::fs::remove_file(&ruta);
        Some(Disco { f, ruta, cursor: 0, orden: Default::default() })
    }

    /// Escribe y devuelve dónde, y los cuadros que se encimaron.
    fn poner(&mut self, k: (i64, (usize, usize)), q: &Cuadro) -> Option<(Lugar, Vec<(i64, (usize, usize))>)> {
        let n = q.rgba.len() as u64;
        if n > DISCO / 4 {
            return None;
        }
        if self.cursor + n > DISCO {
            self.cursor = 0;
        }
        let (a, z) = (self.cursor, self.cursor + n);
        let mut fuera = vec![];
        self.orden.retain(|(o, l, k)| {
            let choca = *o < z && o + l > a;
            if choca {
                fuera.push(*k);
            }
            !choca
        });
        escribir_en(&self.f, &q.rgba, a).ok()?;
        self.orden.push_back((a, n, k));
        self.cursor = z;
        Some((Lugar { donde: a, ancho: q.ancho, alto: q.alto, t: q.t }, fuera))
    }

    fn leer(&self, l: &Lugar) -> Option<Cuadro> {
        let mut rgba = vec![0u8; l.ancho * l.alto * 4];
        leer_de(&self.f, &mut rgba, l.donde).ok()?;
        Some(Cuadro { t: l.t, ancho: l.ancho, alto: l.alto, rgba, ..Default::default() })
    }
}

impl Drop for Disco {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.ruta);
    }
}

#[cfg(unix)]
fn escribir_en(f: &std::fs::File, b: &[u8], donde: u64) -> std::io::Result<()> {
    std::os::unix::fs::FileExt::write_all_at(f, b, donde)
}
#[cfg(unix)]
fn leer_de(f: &std::fs::File, b: &mut [u8], donde: u64) -> std::io::Result<()> {
    std::os::unix::fs::FileExt::read_exact_at(f, b, donde)
}
#[cfg(windows)]
fn escribir_en(f: &std::fs::File, mut b: &[u8], mut donde: u64) -> std::io::Result<()> {
    while !b.is_empty() {
        let n = std::os::windows::fs::FileExt::seek_write(f, b, donde)?;
        (b, donde) = (&b[n..], donde + n as u64);
    }
    Ok(())
}
#[cfg(windows)]
fn leer_de(f: &std::fs::File, mut b: &mut [u8], mut donde: u64) -> std::io::Result<()> {
    while !b.is_empty() {
        let n = std::os::windows::fs::FileExt::seek_read(f, b, donde)?;
        if n == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        (b, donde) = (&mut b[n..], donde + n as u64);
    }
    Ok(())
}

/// Lo que decide cómo se ve el segundo `t`: si no cambia, el cuadro guardado sirve aunque se edite otra parte.
fn firma(p: &Proyecto, t: f64) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (p.ancho, p.alto, p.fps.to_bits()).hash(&mut h);
    p.capas.iter().for_each(|c| c.visible.hash(&mut h));
    // Con los vecinos, por las transiciones.
    for c in p.video.iter().filter(|c| c.inicio < t + 3.0 && c.fin() > t - 3.0) {
        serde_json::to_string(c).unwrap_or_default().hash(&mut h);
    }
    h.finish()
}

/// ¿Cuesta componer ese segundo? Un video solo se copia tal cual y no vale la pena adelantarlo.
fn trabajoso(p: &Proyecto, t: f64) -> bool {
    p.video.iter().any(|c| c.inicio <= t && c.fin() > t && (!c.simple() || c.contenido != Contenido::Archivo))
}

/// ¿Hay una transición en ese segundo? Son lo más caro de componer: se preparan primero.
fn en_transicion(p: &Proyecto, t: f64) -> bool {
    p.video.iter().any(|c| c.inicio <= t && c.fin() > t && (c.entrada.as_ref().is_some_and(|x| t < c.inicio + x.dur) || c.salida.as_ref().is_some_and(|x| t > c.fin() - x.dur)))
}

/// El hilo que decide: decodifica en orden, reparte la composición entre varios hilos, guarda lo hecho y entrega el cuadro
/// que toca.
struct Director {
    lectores: Lectores,
    a_obreros: Sender<Trabajo>,
    sale: Sender<Arc<Cuadro>>,
    pedido: Option<Pedido>,
    guardados: HashMap<(i64, (usize, usize)), Guardado>,
    memoria: usize,
    /// Se abre con el primer cuadro que costó trabajo.
    disco: Option<Disco>,
    reloj: u64,
    /// Firmas ya calculadas de esta versión del proyecto.
    firmas: HashMap<i64, u64>,
    /// En los obreros: cuadro y firma.
    vuelo: Vec<(i64, u64)>,
    sig: i64,
    nivel: usize,
    /// De cuántos en cuántos cuadros se calcula cuando ni así alcanza.
    paso: i64,
    /// Segundos entre un cuadro listo y el siguiente, con los obreros llenos.
    ritmo: f64,
    /// Lo que tarda un cuadro, de que se manda a que regresa.
    tarda: f64,
    desahogo: u32,
    /// Lo último que se entregó: cuadro, firma y escalón.
    entregado: Option<(i64, u64, usize)>,
    cambio: Instant,
    /// La última vez que salió un cuadro.
    visto: Instant,
    /// La última vez que se terminó un cuadro: en pausa se descansa entre uno y otro, para no atrasar la ventana.
    hecho: Instant,
    /// Segundos que cuesta un cuadro completo, según los últimos.
    costo: f64,
    /// Para la barra de la regla: tramos que cuestan (desde, hasta, listo) y cuándo se publicó.
    estado: Arc<std::sync::Mutex<Vec<(f64, f64, bool)>>>,
    publicado: Instant,
}

impl Director {
    fn fps(p: &Proyecto) -> f64 {
        p.fps.clamp(1.0, 60.0)
    }

    fn firma_de(&mut self, i: i64) -> u64 {
        let Some(q) = &self.pedido else { return 0 };
        let (p, f) = (q.p.clone(), Self::fps(&q.p));
        *self.firmas.entry(i).or_insert_with(|| firma(&p, i as f64 / f))
    }

    /// El cuadro guardado de `i` si todavía vale: su escalón.
    fn listo(&mut self, i: i64) -> Option<usize> {
        let tam = self.pedido.as_ref()?.tam;
        let f = self.firma_de(i);
        self.guardados.get(&(i, tam)).filter(|g| g.firma == f).map(|g| g.nivel)
    }

    fn pedir(&mut self, q: Pedido) {
        let antes = self.pedido.take();
        let otra = antes.as_ref().is_none_or(|a| a.version != q.version);
        if otra {
            self.firmas.clear();
        }
        let salto = antes.as_ref().is_none_or(|a| a.tam != q.tam || a.tocando != q.tocando || (q.i - a.i).abs() > 8 || q.i < a.i);
        if salto || otra {
            self.sig = q.i;
        }
        // Al empezar a reproducir, la calidad que se calcula que aguanta: no se espera a que se trabe.
        if q.tocando && antes.as_ref().is_some_and(|a| !a.tocando) && self.costo > 0.0 {
            let k = self.costo * Self::fps(&q.p) / 1.6;
            self.nivel = (0..ESCALAS.len()).find(|n| k * ESCALAS[*n].powi(2) < 0.9).unwrap_or(ESCALAS.len() - 1);
            self.ritmo = 0.0;
            self.paso = 1;
        }
        if antes.as_ref().is_none_or(|a| a.i != q.i || a.version != q.version || a.tam != q.tam || a.tocando != q.tocando) {
            self.cambio = Instant::now();
        }
        self.lectores.tocando = true;
        self.lectores.seguido = q.tocando;
        self.pedido = Some(q);
    }

    fn recibir(&mut self, h: Hecho) {
        self.vuelo.retain(|v| *v != (h.i, h.firma));
        self.hecho = Instant::now();
        self.tarda += (h.seg - self.tarda) * if self.tarda == 0.0 { 1.0 } else { 0.25 };
        let completo = h.seg / ESCALAS[h.nivel].powi(2);
        self.costo += (completo - self.costo) * if self.costo == 0.0 { 1.0 } else { 0.3 };
        // Mientras se mueve un control llegan cuadros de hace un momento: se muestran, para ver el cambio en vivo.
        let vivo = self.pedido.as_ref().is_some_and(|q| !q.tocando && q.i == h.i && q.tam == h.tam);
        if vivo && self.firma_de(h.i) != h.firma {
            self.entregado = None;
            let _ = self.sale.send(h.q.clone());
        }
        // Tan atrasado que no se iba a ver nada: más vale mostrarlo que dejar la imagen congelada.
        let tarde = self.pedido.as_ref().is_some_and(|q| q.tocando && q.tam == h.tam && self.visto.elapsed() > Duration::from_millis(250) && (q.i - h.i).abs() as f64 <= 2.0 * Self::fps(&q.p));
        if tarde && self.entregado.is_none_or(|e| h.i > e.0 || e.0 - h.i > 8) {
            self.entregado = Some((h.i, h.firma, h.nivel));
            self.visto = Instant::now();
            let _ = self.sale.send(h.q.clone());
        }
        // Lo que alcanza a hacer el equipo: lo que tardó entre los que iban a la vez. Esperar al archivo no cuenta.
        let d = h.seg / (self.vuelo.len() + 1).min(OBREROS) as f64;
        self.ritmo += (d - self.ritmo) * if self.ritmo == 0.0 { 1.0 } else { 0.25 };
        self.reloj += 1;
        let k = (h.i, h.tam);
        // Lo que sale barato no se escribe: se vuelve a hacer.
        let mut lugar = None;
        if h.seg > 0.008 {
            if self.disco.is_none() {
                self.disco = Disco::nuevo();
            }
            if let Some((l, fuera)) = self.disco.as_mut().and_then(|d| d.poner(k, &h.q)) {
                lugar = Some(l);
                for x in fuera.into_iter().filter(|x| *x != k) {
                    if let Some(g) = self.guardados.get_mut(&x) {
                        g.disco = None;
                        if g.q.is_none() {
                            self.guardados.remove(&x);
                        }
                    }
                }
            }
        }
        self.memoria += h.q.rgba.len();
        if let Some(v) = self.guardados.insert(k, Guardado { firma: h.firma, nivel: h.nivel, q: Some(h.q), disco: lugar, uso: self.reloj }) {
            self.memoria -= v.q.map_or(0, |q| q.rgba.len());
        }
        self.aligerar();
        self.regular(h.nivel);
    }

    /// Suelta de la RAM primero lo que ya pasó y luego lo que más falta para verse.
    fn aligerar(&mut self) {
        let (ahora, tam) = self.pedido.as_ref().map_or((0, (0, 0)), |q| (q.i, q.tam));
        while self.memoria > memoria_cuadros() {
            let lejos = |k: &(i64, (usize, usize))| if k.1 != tam { i64::MAX } else if k.0 < ahora { 1_000_000 + ahora - k.0 } else { k.0 - ahora };
            let Some(k) = self.guardados.iter().filter(|g| g.1.q.is_some()).max_by_key(|g| lejos(g.0)).map(|g| *g.0) else { break };
            let Some(g) = self.guardados.get_mut(&k) else { break };
            self.memoria -= g.q.take().map_or(0, |q| q.rgba.len());
            if g.disco.is_none() {
                self.guardados.remove(&k);
            }
        }
        // Las fichas sin cuadro pesan poco, pero no se acumulan sin fin.
        if self.guardados.len() > 20_000 {
            self.guardados.retain(|_, g| g.q.is_some());
        }
    }

    /// Sube o baja de calidad según lo que alcanza.
    fn regular(&mut self, nivel: usize) {
        let Some(q) = self.pedido.as_ref().filter(|q| q.tocando) else { return };
        if self.ritmo <= 0.0 || nivel != self.nivel {
            return;
        }
        let k = self.ritmo * Self::fps(&q.p) / self.paso as f64;
        let area = |a: usize, b: usize| (ESCALAS[b] / ESCALAS[a]).powi(2);
        if k > 0.9 {
            self.desahogo = 0;
            // El costo va con el área: se baja de una vez hasta donde alcance. Si ni así, la mitad de los cuadros; y al
            // final, más chico.
            let tope = if self.paso > 1 { ESCALAS.len() - 1 } else { ESCALAS.len() - 2 };
            let meta = (self.nivel..=tope).find(|n| k * area(self.nivel, *n) < 0.7).unwrap_or(tope).max(self.nivel);
            if meta > self.nivel {
                self.ritmo *= area(self.nivel, meta);
                self.nivel = meta;
            } else if self.paso < 4 {
                self.paso += 1;
            }
        } else if k < 0.4 {
            self.desahogo += 1;
            if self.desahogo > 24 {
                self.desahogo = 0;
                if self.paso > 1 && k * (self.paso as f64) / ((self.paso - 1) as f64) < 0.7 {
                    self.paso -= 1;
                } else if self.nivel > 0 && k * area(self.nivel, self.nivel - 1) < 0.7 {
                    self.ritmo *= area(self.nivel, self.nivel - 1);
                    self.nivel -= 1;
                }
            }
        } else {
            self.desahogo = 0;
        }
    }

    /// Manda a la ventana el mejor cuadro que haya para lo que pidió.
    fn entregar(&mut self) {
        let Some((i, tam, tocando)) = self.pedido.as_ref().map(|q| (q.i, q.tam, q.tocando)) else { return };
        let mejor = match self.listo(i) {
            Some(_) => Some(i),
            None if tocando => {
                // El más cercano ya hecho, sin regresar de lo que se está viendo.
                let (r, paso) = (self.paso.max(2) * 3, self.paso);
                let visto = self.entregado.map_or(i64::MIN, |e| e.0);
                let adelante = visto < i - r * 2 || visto > i + r * 2;
                (1..=r).flat_map(|d| [i - d, i + d.min(paso)]).filter(|j| adelante || *j > visto).find(|j| self.listo(*j).is_some())
            }
            None => None,
        };
        let Some(j) = mejor else { return };
        let f = self.firma_de(j);
        self.reloj += 1;
        let Some(g) = self.guardados.get_mut(&(j, tam)) else { return };
        g.uso = self.reloj;
        // Ya se está viendo.
        if self.entregado == Some((j, f, g.nivel)) {
            return;
        }
        let q = match (&g.q, &g.disco, &self.disco) {
            (Some(q), ..) => Some(q.clone()),
            (None, Some(l), Some(d)) => d.leer(l).map(Arc::new),
            _ => None,
        };
        let Some(q) = q else {
            self.guardados.remove(&(j, tam));
            return;
        };
        self.entregado = Some((j, f, g.nivel));
        self.visto = Instant::now();
        let _ = self.sale.send(q);
    }

    /// Cuántos cuadros por adelantado caben en la RAM, dejando lugar a los de atrás; y cuántos más en el disco.
    fn caben(&self) -> i64 {
        let peso = self.pedido.as_ref().map_or(1, |q| q.tam.0 * q.tam.1 * 4).max(1);
        (memoria_cuadros() / peso * 2 / 3).max(4) as i64
    }
    fn caben_disco(&self) -> i64 {
        let peso = self.pedido.as_ref().map_or(1, |q| q.tam.0 * q.tam.1 * 4).max(1);
        (DISCO as usize / 2 / peso) as i64
    }

    /// El siguiente cuadro que conviene hacer, si hay lugar.
    fn siguiente(&mut self) -> Option<(i64, usize)> {
        let (i, tocando, fps, fin, p) = self.pedido.as_ref().map(|q| (q.i, q.tocando, Self::fps(&q.p), q.p.fin(), q.p.clone()))?;
        if !tocando {
            // En pausa, primero el que se ve, completo.
            let f = self.firma_de(i);
            let hay = self.listo(i);
            if hay != Some(0) {
                if self.vuelo.contains(&(i, f)) || !self.vuelo.is_empty() {
                    return None;
                }
                // Primero uno rápido si el completo tarda; el bueno, cuando ya no se mueve.
                let rapido = (0..ESCALAS.len()).find(|n| self.costo * ESCALAS[*n].powi(2) < 0.03).unwrap_or(ESCALAS.len() - 1);
                return match hay {
                    None => Some((i, rapido)),
                    Some(_) if self.cambio.elapsed() > Duration::from_millis(220) => Some((i, 0)),
                    Some(_) => None,
                };
            }
            // Y con calma, todo el proyecto: primero las transiciones (lo más caro), luego lo demás que cueste; cada vez
            // desde el cursor hacia adelante y después desde el principio. Dos hilos, uno queda libre para la ventana.
            if self.cambio.elapsed() < Duration::from_millis(600) || self.vuelo.len() >= OBREROS - 1 {
                return None;
            }
            // Si lo último salió barato, lo demás quizá también; las transiciones no: esas siempre se preparan.
            let barato = self.tarda < 0.012;
            let tope = (self.caben() + self.caben_disco()) as usize;
            let ultimo = (fin * fps).ceil() as i64;
            let orden: Vec<i64> = (i + 1..ultimo).chain(0..i).take(tope).collect();
            let vuelo: Vec<i64> = self.vuelo.iter().map(|v| v.0).collect();
            let falta = |d: &mut Self, j: i64| !vuelo.contains(&j) && d.listo(j).is_none();
            if let Some(j) = orden.iter().copied().find(|j| en_transicion(&p, *j as f64 / fps) && falta(self, *j)) {
                return Some((j, 0));
            }
            if barato {
                return None;
            }
            return orden.into_iter().find(|j| trabajoso(&p, *j as f64 / fps) && falta(self, *j)).map(|j| (j, 0));
        }
        if self.vuelo.len() >= OBREROS {
            return None;
        }
        // Se quedó atrás: se brinca a uno que sí alcance a estar a tiempo.
        if self.sig < i {
            self.sig = i + if self.tarda > 1.0 / fps { (self.tarda * fps).ceil() as i64 } else { 0 };
        }
        // Si viene una transición que no está hecha, se mira más lejos (también con lugar en el disco) para que los
        // hilos la vayan haciendo mientras pasa lo fácil.
        let pronto = i + (ADELANTO_PAUSA * fps) as i64;
        let viene = (i..pronto).any(|j| en_transicion(&p, j as f64 / fps) && self.listo(j).is_none());
        let hasta = if viene { pronto.min(i + self.caben() + self.caben_disco()) } else { i + ((ADELANTO * fps) as i64).min(self.caben()) };
        while self.sig <= hasta {
            let j = self.sig;
            if j as f64 / fps >= fin && j > i {
                return None;
            }
            self.sig += self.paso;
            let f = self.firma_de(j);
            if self.listo(j).is_some() || self.vuelo.contains(&(j, f)) {
                continue;
            }
            // Si no alcanza para todos, un hueco con vecinos hechos se queda como está.
            let paso = self.paso;
            if paso > 1 && (1..paso).any(|d| self.listo(j - d).is_some() || self.listo(j + d).is_some()) {
                continue;
            }
            return Some((j, self.nivel));
        }
        None
    }

    /// Publica qué tramos que cuestan ya están hechos en calidad completa (para la barra de la regla).
    fn publicar(&mut self) {
        self.publicado = Instant::now();
        let Some((fps, fin, p)) = self.pedido.as_ref().map(|q| (Self::fps(&q.p), q.p.fin(), q.p.clone())) else { return };
        let mut v: Vec<(f64, f64, bool)> = vec![];
        // Lo que de verdad cuesta: las transiciones siempre; lo demás, si en esta máquina salió caro.
        let barato = self.tarda < 0.012;
        for j in 0..(fin * fps).ceil() as i64 {
            let t = j as f64 / fps;
            if !(en_transicion(&p, t) || !barato && trabajoso(&p, t)) {
                continue;
            }
            let listo = self.listo(j) == Some(0);
            match v.last_mut() {
                Some(u) if u.2 == listo && (u.1 - t).abs() < 0.5 / fps => u.1 = t + 1.0 / fps,
                _ => v.push((t, t + 1.0 / fps, listo)),
            }
        }
        *self.estado.lock().unwrap() = v;
    }

    /// Decodifica lo que necesita el cuadro `j` y lo manda a componer.
    fn lanzar(&mut self, j: i64, nivel: usize) {
        let firma = self.firma_de(j);
        let Some(q) = &self.pedido else { return };
        let (p, tam, t) = (q.p.clone(), q.tam, j as f64 / Self::fps(&q.p));
        self.lectores.limpiar();
        let de_tiempo = composicion::usa_pasado(&p);
        let mut s = Surtido { cuadros: HashMap::new(), pasado: HashMap::new() };
        for (c, ts, aw, ah) in composicion::pedidos(&p, t, tam.0, tam.1) {
            if let Some(q) = self.lectores.cuadro(c, ts, aw, ah) {
                if de_tiempo {
                    s.pasado.insert(c.id, self.lectores.recientes(c));
                }
                s.cuadros.insert(c.id, q);
            }
        }
        // Los efectos de tiempo mezclan cuadros del mismo tamaño.
        let nivel = if de_tiempo { 0 } else { nivel };
        self.vuelo.push((j, firma));
        let _ = self.a_obreros.send(Trabajo { i: j, firma, p, t, tam, nivel, surtido: s });
    }
}

fn obrero(entra: Receiver<Trabajo>, sale: Sender<Hecho>) {
    while let Ok(mut x) = entra.recv() {
        let ini = Instant::now();
        let (w, h) = a_escala(x.tam, ESCALAS[x.nivel]);
        let rgba = composicion::componer(&x.p, x.t, w, h, &mut x.surtido);
        let q = Arc::new(Cuadro { t: x.t, ancho: w, alto: h, rgba, ..Default::default() });
        if sale.send(Hecho { i: x.i, firma: x.firma, tam: x.tam, nivel: x.nivel, q, seg: ini.elapsed().as_secs_f64() }).is_err() {
            return;
        }
    }
}

/// Vista previa: compone por adelantado y guarda lo hecho. Siempre atiende el pedido más nuevo.
pub struct VistaPrevia {
    tx: Sender<Pedido>,
    rx: Receiver<Arc<Cuadro>>,
    /// Pedidos sin respuesta.
    pub pendientes: u32,
    ultimo: Option<(i64, u64, (usize, usize), bool)>,
    estado: Arc<std::sync::Mutex<Vec<(f64, f64, bool)>>>,
}

impl Default for VistaPrevia {
    fn default() -> Self {
        let (tx, entra) = unbounded::<Pedido>();
        let (sale, rx) = unbounded();
        let estado = Arc::new(std::sync::Mutex::new(vec![]));
        let estado2 = estado.clone();
        std::thread::spawn(move || {
            let (a_obreros, trabajos) = unbounded();
            let (hecho, hechos) = unbounded();
            let mut d = Director {
                lectores: Lectores::nuevos(60.0, true).con(Modo { yuv: true, ligera: true, ..Default::default() }),
                a_obreros,
                sale,
                pedido: None,
                guardados: HashMap::new(),
                memoria: 0,
                disco: None,
                reloj: 0,
                firmas: HashMap::new(),
                vuelo: vec![],
                sig: 0,
                nivel: 0,
                paso: 1,
                ritmo: 0.0,
                tarda: 0.0,
                desahogo: 0,
                entregado: None,
                cambio: Instant::now(),
                visto: Instant::now(),
                hecho: Instant::now(),
                costo: 0.0,
                estado: estado2,
                publicado: Instant::now(),
            };
            // Los obreros se prenden con el primer pedido: hay vistas que nunca se usan.
            let mut prendidos = false;
            loop {
                // Con trabajo pendiente no se espera; si no, hasta que algo pase.
                crossbeam_channel::select! {
                    recv(entra) -> q => match q {
                        Ok(q) => d.pedir(q),
                        Err(_) => return,
                    },
                    recv(hechos) -> h => if let Ok(h) = h { d.recibir(h) },
                    default(Duration::from_millis(if d.pedido.is_some() { 60 } else { 5000 })) => {}
                }
                while let Ok(q) = entra.try_recv() {
                    d.pedir(q);
                }
                while let Ok(h) = hechos.try_recv() {
                    d.recibir(h);
                }
                d.entregar();
                if d.publicado.elapsed() > Duration::from_millis(400) {
                    d.publicar();
                }
                // De uno en uno: entre cada cuadro se vuelve a ver si pidieron otra cosa.
                while let Some((j, nivel)) = d.siguiente() {
                    if !std::mem::replace(&mut prendidos, true) {
                        for _ in 0..OBREROS {
                            let (e, s) = (trabajos.clone(), hecho.clone());
                            std::thread::spawn(move || obrero(e, s));
                        }
                    }
                    d.lanzar(j, nivel);
                    if !entra.is_empty() || !hechos.is_empty() {
                        break;
                    }
                }
            }
        });
        VistaPrevia { tx, rx, pendientes: 0, ultimo: None, estado }
    }
}

impl VistaPrevia {
    /// Tramos que cuestan componer: (desde, hasta, ya hecho en calidad completa). Para la barra de la regla.
    pub fn preparado(&self) -> Vec<(f64, f64, bool)> {
        self.estado.lock().unwrap().clone()
    }

    /// Pide el cuadro de `t` si hace falta y devuelve el más nuevo que esté listo. `version` cambia con cada edición.
    pub fn actualizar(&mut self, p: &Arc<Proyecto>, version: u64, t: f64, tocando: bool, tam: (usize, usize)) -> Option<Arc<Cuadro>> {
        let mut r = None;
        while let Ok(c) = self.rx.try_recv() {
            // En pausa, un cuadro chico es un adelanto: falta el bueno.
            self.pendientes = u32::from(!tocando && c.ancho < tam.0);
            r = Some(c);
        }
        let fps = p.fps.clamp(1.0, 60.0);
        // Un pedido por cuadro de video, no por cuadro de pantalla.
        let i = (t * fps + 0.01).floor() as i64;
        if self.ultimo != Some((i, version, tam, tocando)) {
            self.ultimo = Some((i, version, tam, tocando));
            self.pendientes = 1;
            let _ = self.tx.send(Pedido { p: p.clone(), version, i, t: i as f64 / fps, tocando, tam });
        }
        r
    }
}

/// Tira de miniaturas de un video, una cada `paso` segundos.
pub struct Tira {
    pub paso: f64,
    pub ancho: usize,
    pub alto: usize,
    pub rgba: Vec<Vec<u8>>,
}

pub fn tira(ruta: &std::path::Path, dur: f64, razon: f64) -> Option<Tira> {
    let paso = (dur / 60.0).max(1.0);
    let alto = 40;
    let ancho = ((alto as f64 * razon / 2.0).round() as usize * 2).clamp(16, 120);
    let s = medios::ffmpeg()
        .args(["-skip_frame", "nokey", "-i"])
        .arg(ruta)
        .args(["-an", "-vf", &format!("fps=1/{paso}:round=near,scale={ancho}:{alto}"), "-pix_fmt", "rgba", "-f", "rawvideo", "pipe:1"])
        .output()
        .ok()?;
    let rgba: Vec<Vec<u8>> = s.stdout.chunks_exact(ancho * alto * 4).map(|c| c.to_vec()).collect();
    (!rgba.is_empty()).then_some(Tira { paso, ancho, alto, rgba })
}
