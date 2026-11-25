//! Videos que el chip de video de esta máquina no sabe leer (AV1 en una Mac M1, por ejemplo): se detecta al importarlos y,
//! si la persona quiere, se convierten una vez a H.264, que el chip sí lee. Cada máquina es distinta: se prueba aquí mismo
//! con un cuadro del archivo, no con una lista fija de formatos.
use crate::medios;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

/// Lo que se le cuenta a la persona antes de decidir.
#[derive(Clone, Debug)]
pub struct Aviso {
    /// El formato del archivo, como se le dice a la gente (AV1, VP9…).
    pub formato: String,
    /// Cuánto pesaría la copia, en bytes (aproximado).
    pub peso: u64,
    /// Cuánto tardaría en convertirse, en segundos (aproximado).
    pub tarda: f64,
}

/// Cómo se le pide a ffmpeg que decodifique con el chip y deje los cuadros en el chip: si el chip no puede, falla en vez
/// de seguir calladito con el procesador.
fn chip() -> Option<&'static [&'static str]> {
    if cfg!(target_os = "macos") {
        Some(&["-hwaccel", "videotoolbox", "-hwaccel_output_format", "videotoolbox_vld"])
    } else if cfg!(windows) {
        Some(&["-hwaccel", "d3d11va", "-hwaccel_output_format", "d3d11"])
    } else {
        None
    }
}

/// ¿El chip de video lee este archivo? `None`: esta máquina no tiene cómo usar el chip.
pub fn lo_lee_el_chip(ruta: &Path) -> Option<bool> {
    let args = chip()?;
    Some(["nv12", "p010le"].into_iter().any(|fmt| {
        let s = medios::ffmpeg()
            .args(args)
            .arg("-i")
            .arg(ruta)
            .args(["-an", "-sn", "-frames:v", "1", "-vf", &format!("hwdownload,format={fmt}"), "-f", "null", "-"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        s.is_ok_and(|s| s.success())
    }))
}

/// ¿El chip lee H.264? Se prueba una vez por sesión con un video chiquito hecho aquí: si tampoco, convertir no ayuda.
fn lee_h264() -> bool {
    static SI: OnceLock<bool> = OnceLock::new();
    *SI.get_or_init(|| {
        let r = std::env::temp_dir().join(format!("beethoven-craft-prueba-{}.mp4", std::process::id()));
        let hecho = medios::ffmpeg()
            .args(["-y", "-f", "lavfi", "-i", "testsrc2=size=320x240:rate=24:duration=0.25", "-pix_fmt", "yuv420p"])
            .args(codificador_h264())
            .arg(&r)
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        let si = hecho && lo_lee_el_chip(&r) == Some(true);
        let _ = std::fs::remove_file(&r);
        si
    })
}

fn codificador_h264() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["-c:v", "h264_videotoolbox", "-q:v", "75", "-profile:v", "high"]
    } else {
        &["-c:v", "libx264", "-preset", "veryfast", "-crf", "18"]
    }
}

fn formato(ruta: &Path) -> String {
    let s = medios::comando(medios::herramienta("ffprobe"))
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=codec_name", "-of", "csv=p=0"])
        .arg(ruta)
        .output();
    let c = s.map(|s| String::from_utf8_lossy(&s.stdout).trim().to_string()).unwrap_or_default();
    match c.as_str() {
        "av1" => "AV1".into(),
        "vp9" => "VP9".into(),
        "vp8" => "VP8".into(),
        "hevc" => "HEVC (H.265)".into(),
        "h264" => "H.264".into(),
        "prores" => "ProRes".into(),
        "" => "?".into(),
        x => x.to_uppercase(),
    }
}

/// Si conviene ofrecer la conversión: el chip no lee el archivo pero sí leería la copia en H.264.
/// `video`: ancho, alto y cuadros por segundo; `dur` en segundos.
pub fn revisar(ruta: &Path, video: (u32, u32, f64), dur: f64) -> Option<Aviso> {
    if lo_lee_el_chip(ruta)? || !lee_h264() {
        return None;
    }
    let (w, h, fps) = video;
    let px = w as f64 * h as f64;
    // Medido en una Mac M1: un 4K en AV1 a H.264 sale a unos 0,06 bits por píxel y va a la mitad de la velocidad normal.
    let peso = (px * fps * dur * 0.06 / 8.0 + dur * 128_000.0 / 8.0) as u64;
    let tarda = dur * (px / (3840.0 * 2160.0)).max(0.1) * 0.5;
    Some(Aviso { formato: formato(ruta), peso, tarda })
}

/// Dónde queda la copia: junto a las demás, con el nombre del original.
pub fn destino(ruta: &Path, dir: &Path) -> PathBuf {
    let nombre = ruta.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "video".into());
    dir.join(format!("{nombre} (H.264).mp4"))
}

/// Convierte a H.264 con el mismo tamaño y los mismos cuadros. El audio se copia tal cual; si el MP4 no lo acepta, va en
/// AAC. `avance` de 0 a 1000. Si la copia ya existe y es más nueva que el original, se usa esa.
pub fn convertir(ruta: &Path, dir: &Path, dur: f64, avance: &AtomicU32, cancelar: &AtomicBool) -> Result<PathBuf, String> {
    let sale = destino(ruta, dir);
    let fecha = |r: &Path| std::fs::metadata(r).and_then(|m| m.modified()).ok();
    if sale.exists() && fecha(&sale) >= fecha(ruta) {
        return Ok(sale);
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = sale.with_extension("haciendo.mp4");
    let mut r = Err(String::new());
    for audio in [&["-c:a", "copy"][..], &["-c:a", "aac", "-b:a", "320k"]] {
        r = una_vez(ruta, &tmp, dur, audio, avance, cancelar);
        if r.is_ok() || cancelar.load(Relaxed) {
            break;
        }
    }
    match r {
        Ok(()) => std::fs::rename(&tmp, &sale).map(|_| sale).map_err(|e| e.to_string()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(if cancelar.load(Relaxed) { crate::idioma::t("Conversión cancelada").into() } else { e })
        }
    }
}

fn una_vez(ruta: &Path, tmp: &Path, dur: f64, audio: &[&str], avance: &AtomicU32, cancelar: &AtomicBool) -> Result<(), String> {
    let mut hijo = medios::ffmpeg()
        .args(["-y", "-progress", "pipe:1", "-i"])
        .arg(ruta)
        .args(["-map", "0:v:0", "-map", "0:a?", "-sn", "-fps_mode", "passthrough", "-pix_fmt", "yuv420p"])
        .args(codificador_h264())
        // Un cuadro completo cada 2 s: brincar a cualquier punto es rápido.
        .args(["-g", "48"])
        .args(audio)
        .args(["-movflags", "+faststart"])
        .arg(tmp)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| crate::idioma::tf("No pude ejecutar ffmpeg: {}", &[&e]))?;
    let mut err = hijo.stderr.take().unwrap();
    let lector = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = std::io::Read::read_to_string(&mut err, &mut s);
        s
    });
    for l in BufReader::new(hijo.stdout.take().unwrap()).lines().map_while(Result::ok) {
        if let Some(us) = l.strip_prefix("out_time_us=").and_then(|x| x.parse::<f64>().ok()) {
            avance.store(((us / 1e6 / dur.max(0.1) * 1000.0) as u32).min(999), Relaxed);
        }
        if cancelar.load(Relaxed) {
            let _ = hijo.kill();
        }
    }
    let bien = hijo.wait().is_ok_and(|s| s.success());
    let texto = lector.join().unwrap_or_default();
    if bien {
        Ok(())
    } else {
        Err(crate::idioma::tf("ffmpeg falló: {}", &[&(texto.lines().last().unwrap_or("sin detalle"))]))
    }
}
