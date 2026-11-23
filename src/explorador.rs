//! El explorador de archivos por dentro: qué hay en una carpeta, búsqueda en subcarpetas, miniaturas y lo que se recuerda
//! (carpeta, favoritos). Nada aquí cambia archivos del usuario.
use crate::medios::{self, Info};
use std::cmp::Ordering;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Condvar, Mutex};
use std::time::SystemTime;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tipo {
    Carpeta,
    Video,
    Audio,
    Imagen,
    Proyecto,
    Midi,
    Subtitulos,
    Otro,
}

impl Tipo {
    pub fn de(r: &Path) -> Tipo {
        let e = r.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        match e.as_str() {
            "mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" | "mpg" | "mpeg" | "mts" | "m2ts" | "3gp" | "wmv" | "flv" | "gif" => Tipo::Video,
            "mp3" | "wav" | "m4a" | "flac" | "ogg" | "opus" | "aac" | "aif" | "aiff" | "wma" => Tipo::Audio,
            "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tif" | "tiff" => Tipo::Imagen,
            "bcraft" | "toma" => Tipo::Proyecto,
            "mid" | "midi" => Tipo::Midi,
            "srt" => Tipo::Subtitulos,
            _ => Tipo::Otro,
        }
    }

    /// Lo que se puede poner en la línea de tiempo.
    pub fn es_medio(self) -> bool {
        matches!(self, Tipo::Video | Tipo::Audio | Tipo::Imagen)
    }
}

#[derive(Clone, Debug)]
pub struct Entrada {
    pub ruta: PathBuf,
    pub nombre: String,
    pub tipo: Tipo,
    pub tam: u64,
    pub fecha: SystemTime,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Orden {
    #[default]
    Nombre,
    Fecha,
    Tamano,
    Tipo,
}

impl Orden {
    pub const TODOS: [(Orden, &str); 4] = [(Orden::Nombre, "Nombre"), (Orden::Fecha, "Más nuevo"), (Orden::Tamano, "Más pesado"), (Orden::Tipo, "Tipo")];
}

/// Compara como se lee: «toma 2» va antes que «toma 10».
pub fn natural(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    let numero = |i: &mut std::iter::Peekable<std::str::Chars>| {
        let mut n = String::new();
        while let Some(d) = i.next_if(char::is_ascii_digit) {
            n.push(d);
        }
        let n = n.trim_start_matches('0').to_string();
        (n.len(), n)
    };
    loop {
        let o = match (a.peek(), b.peek()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => numero(&mut a).cmp(&numero(&mut b)),
            _ => {
                let baja = |c: char| c.to_lowercase().next().unwrap_or(c);
                baja(a.next().unwrap()).cmp(&baja(b.next().unwrap()))
            }
        };
        if o != Ordering::Equal {
            return o;
        }
    }
}

/// Las carpetas van siempre primero.
pub fn ordenar(v: &mut [Entrada], orden: Orden) {
    v.sort_by(|a, b| {
        let carpeta = |e: &Entrada| e.tipo != Tipo::Carpeta;
        let por = match orden {
            Orden::Nombre => Ordering::Equal,
            Orden::Fecha => b.fecha.cmp(&a.fecha),
            Orden::Tamano => b.tam.cmp(&a.tam),
            Orden::Tipo => (a.tipo as u8).cmp(&(b.tipo as u8)),
        };
        carpeta(a).cmp(&carpeta(b)).then(por).then_with(|| natural(&a.nombre, &b.nombre))
    });
}

/// Paquetes que el sistema muestra como un solo archivo.
fn es_paquete(n: &str) -> bool {
    let n = n.to_lowercase();
    [".app", ".photoslibrary", ".fcpbundle", ".logicx", ".band", ".imovielibrary", ".bundle", ".framework"].iter().any(|e| n.ends_with(e))
}

fn entrada(d: &std::fs::DirEntry) -> Option<Entrada> {
    let nombre = d.file_name().to_string_lossy().into_owned();
    if nombre.starts_with('.') {
        return None;
    }
    let ruta = d.path();
    // Sigue los enlaces: un alias a una carpeta es una carpeta.
    let m = std::fs::metadata(&ruta).ok()?;
    if m.is_dir() && es_paquete(&nombre) {
        return None;
    }
    let tipo = if m.is_dir() { Tipo::Carpeta } else { Tipo::de(&ruta) };
    Some(Entrada { tipo, nombre, tam: if m.is_dir() { 0 } else { m.len() }, fecha: m.modified().unwrap_or(SystemTime::UNIX_EPOCH), ruta })
}

pub fn listar(carpeta: &Path) -> Result<Vec<Entrada>, String> {
    let l = std::fs::read_dir(carpeta).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => crate::idioma::t("El sistema no dio permiso para ver esta carpeta").to_string(),
        std::io::ErrorKind::NotFound => crate::idioma::t("Esa carpeta ya no está").to_string(),
        _ => e.to_string(),
    })?;
    Ok(l.flatten().filter_map(|d| entrada(&d)).collect())
}

/// Sin acentos ni mayúsculas, para buscar.
pub fn llano(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            c => c,
        })
        // En Mac los nombres pueden traer el acento como carácter aparte.
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .collect()
}

pub const TOPE_BUSQUEDA: usize = 400;

/// Busca en la carpeta y sus subcarpetas. Para si `vigente` deja de valer `turno`.
pub fn buscar(raiz: &Path, texto: &str, vigente: &AtomicU64, turno: u64) -> Vec<Entrada> {
    let q = llano(texto);
    let mut pendientes = std::collections::VecDeque::from([(raiz.to_path_buf(), 0)]);
    let (mut v, mut vistos) = (vec![], 0);
    while let Some((c, nivel)) = pendientes.pop_front() {
        for d in std::fs::read_dir(&c).into_iter().flatten().flatten() {
            vistos += 1;
            if vistos > 60_000 || v.len() >= TOPE_BUSQUEDA || vigente.load(Relaxed) != turno {
                return v;
            }
            let Some(e) = entrada(&d) else { continue };
            // Un enlace a una carpeta no se recorre: podría dar vueltas.
            if e.tipo == Tipo::Carpeta && nivel < 8 && d.file_type().is_ok_and(|t| t.is_dir()) {
                pendientes.push_back((e.ruta.clone(), nivel + 1));
            }
            if llano(&e.nombre).contains(&q) {
                v.push(e);
            }
        }
    }
    v
}

/// Las carpetas de siempre del usuario, las que existan.
pub fn lugares() -> Vec<(&'static str, PathBuf)> {
    let Some(h) = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from) else { return vec![] };
    let videos = if cfg!(target_os = "macos") { "Movies" } else { "Videos" };
    let mut v = vec![("Inicio", h.clone())];
    v.extend([("Escritorio", "Desktop"), ("Descargas", "Downloads"), ("Documentos", "Documents"), ("Videos", videos), ("Música", "Music"), ("Imágenes", "Pictures")].map(|(n, c)| (n, h.join(c))));
    v.retain(|l| l.1.is_dir());
    v
}

/// Discos y memorias conectados.
pub fn discos() -> Vec<PathBuf> {
    if cfg!(windows) {
        return ('C'..='Z').map(|l| PathBuf::from(format!("{l}:\\"))).filter(|r| r.is_dir()).collect();
    }
    let dentro = |c: PathBuf| std::fs::read_dir(c).into_iter().flatten().flatten().map(|d| d.path()).filter(|r| r.is_dir()).collect::<Vec<_>>();
    if cfg!(target_os = "macos") {
        // El disco de arranque aparece como enlace a «/».
        return dentro("/Volumes".into()).into_iter().filter(|r| std::fs::read_link(r).is_err()).collect();
    }
    let u = std::env::var("USER").unwrap_or_default();
    [dentro(format!("/media/{u}").into()), dentro(format!("/run/media/{u}").into())].concat()
}

pub fn peso(b: u64) -> String {
    let b = b as f64;
    match b {
        _ if b < 1e3 => format!("{b:.0} B"),
        _ if b < 1e6 => format!("{:.0} KB", b / 1e3),
        _ if b < 1e9 => format!("{:.1} MB", b / 1e6),
        _ => format!("{:.2} GB", b / 1e9),
    }
}

// ---- Miniaturas

/// Lo que se sabe de un archivo: sus datos, una imagen chica o la forma de su sonido (0 a 1).
#[derive(Clone, Debug, Default)]
pub struct Mini {
    pub info: Option<Info>,
    pub imagen: Option<(usize, usize, Vec<u8>)>,
    pub onda: Vec<f32>,
}

pub const ONDA: usize = 240;
const MINI: (f64, f64) = (256.0, 144.0);

pub fn miniatura(ruta: &Path) -> Mini {
    let Ok(info) = medios::sondear(ruta) else { return Mini::default() };
    let mut m = Mini { info: Some(info.clone()), ..Default::default() };
    if let Some((w, h, _)) = info.video {
        let k = (MINI.0 / w as f64).min(MINI.1 / h as f64).min(1.0);
        let (w, h) = (((w as f64 * k) as usize).max(2), ((h as f64 * k) as usize).max(2));
        let mut c = medios::ffmpeg();
        if Tipo::de(ruta) == Tipo::Video {
            c.args(["-ss", &format!("{:.3}", (info.dur * 0.15).min(8.0))]);
        }
        let o = c.arg("-i").arg(ruta).args(["-an", "-map", "0:v:0", "-frames:v", "1", "-vf", &format!("scale={w}:{h},setsar=1"), "-pix_fmt", "rgba", "-f", "rawvideo", "pipe:1"]).output();
        m.imagen = o.ok().map(|o| o.stdout).filter(|q| q.len() >= w * h * 4).map(|mut q| {
            q.truncate(w * h * 4);
            (w, h, q)
        });
    } else if info.audio {
        m.onda = onda(ruta, info.dur);
    }
    m
}

/// La forma del sonido en `ONDA` partes.
fn onda(ruta: &Path, dur: f64) -> Vec<f32> {
    const HZ: usize = 4000;
    let hijo = medios::ffmpeg().args(["-t", "3600", "-i"]).arg(ruta).args(["-vn", "-map", "0:a:0", "-ac", "1", "-ar", &HZ.to_string(), "-f", "s16le", "pipe:1"]).stdout(Stdio::piped()).stderr(Stdio::null()).spawn();
    let Ok(mut h) = hijo else { return vec![] };
    let por = ((dur.clamp(0.05, 3600.0) * HZ as f64 / ONDA as f64).ceil() as usize).max(1);
    let mut v = vec![0f32; ONDA];
    let mut o = std::io::BufReader::new(h.stdout.take().unwrap());
    let mut b = vec![0u8; por * 2];
    for x in &mut v {
        let mut n = 0;
        while n < b.len() {
            match o.read(&mut b[n..]) {
                Ok(0) | Err(_) => break,
                Ok(k) => n += k,
            }
        }
        *x = b[..n / 2 * 2].chunks_exact(2).map(|c| (i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0).abs()).fold(0.0, f32::max);
    }
    let _ = h.kill();
    let _ = h.wait();
    v
}

/// Dos hilos que hacen miniaturas; la última pedida va primero, porque es la que se está viendo.
pub struct Taller {
    cola: Arc<(Mutex<Vec<PathBuf>>, Condvar)>,
    fin: Arc<AtomicBool>,
}

impl Taller {
    pub fn nuevo(entregar: impl Fn(PathBuf, Mini) + Send + Sync + 'static) -> Taller {
        let cola = Arc::new((Mutex::new(Vec::<PathBuf>::new()), Condvar::new()));
        let fin = Arc::new(AtomicBool::new(false));
        let entregar = Arc::new(entregar);
        for _ in 0..2 {
            let (cola, fin, entregar) = (cola.clone(), fin.clone(), entregar.clone());
            std::thread::spawn(move || {
                loop {
                    let mut q = cola.0.lock().unwrap();
                    while q.is_empty() && !fin.load(Relaxed) {
                        q = cola.1.wait(q).unwrap();
                    }
                    if fin.load(Relaxed) {
                        return;
                    }
                    let Some(r) = q.pop() else { continue };
                    drop(q);
                    let m = miniatura(&r);
                    entregar(r, m);
                }
            });
        }
        Taller { cola, fin }
    }

    /// Pide una miniatura. Devuelve las que ya no caben en la fila: se volverán a pedir si se ven.
    pub fn pedir(&self, r: PathBuf) -> Vec<PathBuf> {
        let mut q = self.cola.0.lock().unwrap();
        q.push(r);
        let sobra = q.len().saturating_sub(80);
        let fuera = q.drain(..sobra).collect();
        self.cola.1.notify_one();
        fuera
    }
}

impl Drop for Taller {
    fn drop(&mut self) {
        self.fin.store(true, Relaxed);
        self.cola.1.notify_all();
    }
}

// ---- Lo que se recuerda

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Recuerdo {
    pub abierto: bool,
    pub carpeta: Option<PathBuf>,
    pub favoritos: Vec<PathBuf>,
    pub cuadricula: bool,
    pub todos: bool,
}

impl Recuerdo {
    pub fn leer(archivo: &Path) -> Recuerdo {
        let r: Option<Recuerdo> = std::fs::read(archivo).ok().and_then(|b| serde_json::from_slice(&b).ok());
        r.unwrap_or(Recuerdo { abierto: true, ..Default::default() })
    }

    pub fn guardar(&self, archivo: &Path) {
        if let Some(c) = archivo.parent() {
            let _ = std::fs::create_dir_all(c);
        }
        if let Ok(b) = serde_json::to_vec_pretty(self) {
            let _ = std::fs::write(archivo, b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orden_natural() {
        let mut v = ["toma 10", "Toma 2", "toma 1", "toma 02b", "álbum", "a"];
        v.sort_by(|a, b| natural(a, b));
        assert_eq!(v, ["a", "toma 1", "Toma 2", "toma 02b", "toma 10", "álbum"]);
        assert_eq!(llano("Canción ÑANDÚ"), "cancion ñandu");
        assert_eq!(llano("cancio\u{301}n"), "cancion");
    }

    #[test]
    fn carpetas_y_busqueda() {
        let d = std::env::temp_dir().join(format!("explorador-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("B carpeta/dentro")).unwrap();
        std::fs::create_dir_all(d.join("Juego.app")).unwrap();
        for (n, k) in [("z.mp4", 10), ("a.WAV", 30), ("foto.png", 20), (".oculto.mp4", 1), ("nota.txt", 5), ("B carpeta/dentro/Canción z.mp3", 1)] {
            std::fs::write(d.join(n), vec![0u8; k]).unwrap();
        }
        let nombres = |o| {
            let mut v = listar(&d).unwrap();
            ordenar(&mut v, o);
            v.into_iter().map(|e| e.nombre).collect::<Vec<_>>()
        };
        assert_eq!(nombres(Orden::Nombre), ["B carpeta", "a.WAV", "foto.png", "nota.txt", "z.mp4"], "carpetas primero; ni ocultos ni paquetes");
        assert_eq!(nombres(Orden::Tamano), ["B carpeta", "a.WAV", "foto.png", "z.mp4", "nota.txt"]);
        assert_eq!(nombres(Orden::Tipo), ["B carpeta", "z.mp4", "a.WAV", "foto.png", "nota.txt"]);
        let turno = AtomicU64::new(3);
        let v = buscar(&d, "cancion", &turno, 3);
        assert_eq!(v.iter().map(|e| e.nombre.as_str()).collect::<Vec<_>>(), ["Canción z.mp3"]);
        assert!(buscar(&d, "z", &turno, 2).is_empty(), "una búsqueda vieja se detiene");
        assert!(listar(&d.join("no está")).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn recuerda() {
        let a = std::env::temp_dir().join(format!("recuerdo-{}.json", std::process::id()));
        assert!(Recuerdo::leer(&a).abierto, "la primera vez sale abierto");
        let r = Recuerdo { abierto: false, carpeta: Some("/x".into()), favoritos: vec!["/y".into()], cuadricula: true, todos: true };
        r.guardar(&a);
        assert_eq!(Recuerdo::leer(&a), r);
        let _ = std::fs::remove_file(a);
        assert_eq!((peso(999), peso(1_500_000), peso(2_340_000_000)), ("999 B".into(), "1.5 MB".into(), "2.34 GB".into()));
    }
}
