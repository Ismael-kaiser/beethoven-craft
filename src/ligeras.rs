//! Copias ligeras de los videos pesados, solo para editar: se hacen en segundo plano, una vez por archivo, y la exportación
//! siempre usa el original.
use crate::medios;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, PartialEq)]
enum Estado {
    Espera,
    Lista(PathBuf),
    /// No le hace falta, o no se pudo.
    No,
}

static ESTADOS: Mutex<Option<HashMap<PathBuf, Estado>>> = Mutex::new(None);
static COLA: OnceLock<crossbeam_channel::Sender<PathBuf>> = OnceLock::new();
/// El usuario las apagó.
pub static APAGADAS: AtomicBool = AtomicBool::new(false);
/// Avance de la que se está haciendo: 1 a 1000; 0, ninguna.
static AVANCE: AtomicU32 = AtomicU32::new(0);
/// Lado más largo.
const LADO: u32 = 1920;
/// Espacio para guardarlas.
const TOPE: u64 = 12 << 30;

pub fn avance() -> Option<u32> {
    Some(AVANCE.load(Relaxed)).filter(|a| *a > 0)
}

/// El archivo que conviene leer para la vista previa.
pub fn para_ver(ruta: &Path) -> PathBuf {
    if !APAGADAS.load(Relaxed) {
        if let Some(Estado::Lista(p)) = ESTADOS.lock().unwrap().as_ref().and_then(|m| m.get(ruta)) {
            return p.clone();
        }
    }
    ruta.to_path_buf()
}

/// Pide la copia ligera de un video si le hace falta. Se puede llamar a cada rato.
pub fn pedir(ruta: &Path, dir: &Path) {
    if APAGADAS.load(Relaxed) {
        return;
    }
    let mut e = ESTADOS.lock().unwrap();
    let m = e.get_or_insert_with(HashMap::new);
    if m.contains_key(ruta) {
        return;
    }
    m.insert(ruta.to_path_buf(), Estado::Espera);
    drop(e);
    let dir = dir.to_path_buf();
    let tx = COLA.get_or_init(|| {
        let (tx, rx) = crossbeam_channel::unbounded::<PathBuf>();
        std::thread::spawn(move || {
            let _ = std::fs::create_dir_all(&dir);
            limpiar(&dir);
            for r in rx {
                let q = if pesado(&r) { hacer(&r, &dir) } else { None };
                AVANCE.store(0, Relaxed);
                if let Some(m) = ESTADOS.lock().unwrap().as_mut() {
                    m.insert(r, q.map_or(Estado::No, Estado::Lista));
                }
            }
        });
        tx
    });
    let _ = tx.send(ruta.to_path_buf());
}

/// Más grande que 1080p, o de un formato que se decodifica todo con el procesador.
pub fn pesado(ruta: &Path) -> bool {
    let s = medios::comando(medios::herramienta("ffprobe"))
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=codec_name,width,height", "-of", "csv=p=0"])
        .arg(ruta)
        .output();
    let s = s.map(|s| String::from_utf8_lossy(&s.stdout).trim().to_string()).unwrap_or_default();
    let x: Vec<&str> = s.lines().next().unwrap_or("").split(',').collect();
    if x.len() < 3 || medios::con_alfa(ruta) {
        return false;
    }
    let lado = |i: usize| x[i].parse::<u32>().unwrap_or(0);
    lado(1).max(lado(2)) > LADO || (x[0] == "av1" && lado(1).max(lado(2)) > 1280)
}

/// Un nombre que cambia si el archivo cambia.
fn nombre(ruta: &Path) -> Option<String> {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let m = std::fs::metadata(ruta).ok()?;
    let mut h = DefaultHasher::new();
    (ruta, m.len(), m.modified().ok()).hash(&mut h);
    Some(format!("{:016x}.mp4", h.finish()))
}

pub fn hacer(ruta: &Path, dir: &Path) -> Option<PathBuf> {
    let sale = dir.join(nombre(ruta)?);
    if sale.exists() {
        return Some(sale);
    }
    let dur = medios::sondear(ruta).ok()?.dur.max(0.1);
    let tmp = sale.with_extension("haciendo.mp4");
    let video: &[&str] = if cfg!(target_os = "macos") {
        &["-c:v", "h264_videotoolbox", "-q:v", "62", "-profile:v", "high"]
    } else {
        &["-c:v", "libx264", "-preset", "veryfast", "-crf", "20"]
    };
    // Con calma: no le quita procesador a la edición.
    let mut c = if cfg!(unix) {
        let mut c = medios::comando("nice");
        c.args(["-n", "15"]).arg(medios::herramienta("ffmpeg"));
        c
    } else {
        medios::comando(medios::herramienta("ffmpeg"))
    };
    let mut hijo = c
        .args(["-hide_banner", "-nostdin", "-v", "error", "-y", "-progress", "pipe:1", "-threads", "3", "-i"])
        .arg(ruta)
        .args(["-an", "-sn", "-map", "0:v:0", "-fps_mode", "passthrough", "-vf"])
        .arg(format!("scale=w='min({LADO},iw)':h='min({LADO},ih)':force_original_aspect_ratio=decrease:force_divisible_by=2:flags=bicubic:out_color_matrix=bt709:out_range=tv,format=yuv420p,setsar=1"))
        .args(video)
        // Un cuadro completo cada doce: brincar a cualquier punto es inmediato.
        .args(["-g", "12", "-bf", "0", "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709"])
        .arg(&tmp)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    AVANCE.store(1, Relaxed);
    for l in BufReader::new(hijo.stdout.take()?).lines().map_while(Result::ok) {
        if let Some(us) = l.strip_prefix("out_time_us=").and_then(|x| x.parse::<f64>().ok()) {
            AVANCE.store(((us / 1e6 / dur * 1000.0) as u32).clamp(1, 1000), Relaxed);
        }
        if APAGADAS.load(Relaxed) {
            let _ = hijo.kill();
        }
    }
    let bien = hijo.wait().is_ok_and(|s| s.success()) && std::fs::rename(&tmp, &sale).is_ok();
    if !bien {
        let _ = std::fs::remove_file(&tmp);
    }
    bien.then_some(sale)
}

/// Quita las que quedaron a medias y, si ya ocupan mucho, las más viejas.
fn limpiar(dir: &Path) {
    let Ok(d) = std::fs::read_dir(dir) else { return };
    let mut v: Vec<(std::time::SystemTime, u64, PathBuf)> = vec![];
    for e in d.flatten() {
        let p = e.path();
        if p.to_string_lossy().ends_with(".haciendo.mp4") {
            let _ = std::fs::remove_file(p);
        } else if let Ok(m) = e.metadata() {
            v.push((m.accessed().or(m.modified()).unwrap_or(std::time::UNIX_EPOCH), m.len(), p));
        }
    }
    v.sort();
    let mut total: u64 = v.iter().map(|x| x.1).sum();
    for (_, n, p) in v {
        if total <= TOPE {
            break;
        }
        total -= n;
        let _ = std::fs::remove_file(p);
    }
}
