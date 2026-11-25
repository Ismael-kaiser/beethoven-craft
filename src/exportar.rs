//! Exportación: video con la mezcla, o solo la mezcla.
use crate::composicion::{self, Fuente};
use crate::medios::{self, Wav};
use crate::modelo::{Proyecto, SR};
use crate::motor::{self, Almacen};
use crate::video::{Lectores, Modo as Decodificar};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Stdio;
use crossbeam_channel::bounded;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};

/// El codificador de la máquina no sirvió: se usa el de ffmpeg.
static SIN_ACELERADOR: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Modo {
    Video,
    Audio,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Codec {
    H264,
    H265,
    ProRes,
}

#[derive(Clone, Debug)]
pub struct Opciones {
    /// Alto y ancho; `None` = los del proyecto.
    pub tam: Option<(u32, u32)>,
    /// Cuadros por segundo; `None` = los del proyecto.
    pub fps: Option<f64>,
    pub codec: Codec,
    /// 0 ligera, 1 normal, 2 alta.
    pub calidad: usize,
    /// Solo el tramo marcado.
    pub tramo: Option<(f64, f64)>,
}

impl Default for Opciones {
    fn default() -> Self {
        Opciones { tam: None, fps: None, codec: Codec::H264, calidad: 1, tramo: None }
    }
}

impl Opciones {
    fn video(&self) -> Vec<String> {
        let s = |x: &[&str]| x.iter().map(|y| y.to_string()).collect::<Vec<_>>();
        match self.codec {
            // El codificador de la Mac va mucho más rápido y deja libre el procesador para componer.
            Codec::H264 if cfg!(target_os = "macos") && !SIN_ACELERADOR.load(Relaxed) => {
                s(&["-c:v", "h264_videotoolbox", "-profile:v", "high", "-q:v", ["52", "70", "84"][self.calidad.min(2)], "-pix_fmt", "yuv420p"])
            }
            Codec::H264 => s(&["-c:v", "libx264", "-preset", "veryfast", "-crf", ["24", "18", "14"][self.calidad.min(2)], "-pix_fmt", "yuv420p"]),
            // En Mac, el codificador de la máquina; en Windows y Linux, x265.
            Codec::H265 if cfg!(target_os = "macos") && !SIN_ACELERADOR.load(Relaxed) => {
                s(&["-c:v", "hevc_videotoolbox", "-q:v", ["52", "70", "84"][self.calidad.min(2)], "-tag:v", "hvc1", "-pix_fmt", "yuv420p"])
            }
            Codec::H265 => s(&["-c:v", "libx265", "-preset", "fast", "-crf", ["28", "23", "19"][self.calidad.min(2)], "-tag:v", "hvc1", "-pix_fmt", "yuv420p"]),
            Codec::ProRes if cfg!(target_os = "macos") && !SIN_ACELERADOR.load(Relaxed) => s(&["-c:v", "prores_videotoolbox", "-profile:v", ["1", "2", "3"][self.calidad.min(2)], "-pix_fmt", "p210le"]),
            Codec::ProRes => s(&["-c:v", "prores_ks", "-profile:v", ["1", "2", "3"][self.calidad.min(2)], "-pix_fmt", "yuv422p10le"]),
        }
    }
    fn audio(&self) -> [&'static str; 4] {
        if self.codec == Codec::ProRes { ["-c:a", "pcm_s24le", "-ar", "48000"] } else { ["-c:a", "aac", "-b:a", "256k"] }
    }
    fn simple(&self) -> bool {
        self.tam.is_none() && self.tramo.is_none()
    }
}

/// Con las opciones de siempre. `avance` va de 0 a 1000.
pub fn exportar(
    p: &Proyecto,
    almacen: &Almacen,
    salida: &Path,
    modo: Modo,
    avance: &AtomicU32,
    cancelar: &AtomicBool,
) -> Result<(), String> {
    exportar_con(p, almacen, salida, modo, &Opciones::default(), avance, cancelar)
}

/// La exportación corre con prioridad baja: la interfaz y el resto del sistema tienen el procesador primero.
/// Los hilos que se crean después lo heredan. Igual en los tres sistemas: en macOS con la clase de calidad
/// `utility`; en Windows y Linux con la prioridad mínima del hilo (`SetThreadPriority` / `nice`).
fn baja_prioridad() {
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn pthread_set_qos_class_self_np(qos: u32, relativa: i32) -> i32;
        }
        const QOS_CLASS_UTILITY: u32 = 0x11;
        // SAFETY: llamada de sistema sin punteros; solo cambia la clase del hilo actual.
        unsafe {
            pthread_set_qos_class_self_np(QOS_CLASS_UTILITY, 0);
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Min);
    }
}

pub fn exportar_con(
    p: &Proyecto,
    almacen: &Almacen,
    salida: &Path,
    modo: Modo,
    op: &Opciones,
    avance: &AtomicU32,
    cancelar: &AtomicBool,
) -> Result<(), String> {
    baja_prioridad();
    let otro = op.fps.filter(|f| (f - p.fps).abs() > 1e-6).map(|fps| Proyecto { fps, ..p.clone() });
    let p = otro.as_ref().unwrap_or(p);
    for c in p.pistas.iter().flat_map(|t| &t.clips) {
        if !almacen.contains_key(&c.clave()) {
            return Err(crate::idioma::tf("Falta el audio de «{}»", &[&c.nombre]));
        }
    }
    let dur = match modo {
        Modo::Video => p.fin(),
        Modo::Audio => p.fin_audio() + motor::cola(p),
    };
    let (ini, dur) = match op.tramo {
        Some((a, b)) => (a, b.min(dur.max(p.fin())) - a),
        None => (0.0, dur),
    };
    if dur <= 0.0 || (modo == Modo::Audio && p.fin_audio() <= 0.0) {
        return Err(crate::idioma::t("No hay nada que exportar todavía").into());
    }
    if modo == Modo::Video && p.video.is_empty() {
        return Err(crate::idioma::t("El proyecto no tiene video").into());
    }
    // Se escribe a un archivo aparte y se renombra al final: si algo falla no queda un archivo a medias.
    let nombre = salida.file_name().and_then(|n| n.to_str()).ok_or("Nombre inválido")?;
    let tmp = salida.with_file_name(format!(".toma-uno-{nombre}"));
    let hacer = || {
        if modo == Modo::Video && (!p.video_simple() || !op.simple()) {
            componer(p, almacen, &tmp, op, ini, dur, avance, cancelar)
        } else {
            escribir(p, almacen, &tmp, modo, op, ini, dur, avance, cancelar)
        }
    };
    let mut r = hacer();
    // Si falló con el codificador de la máquina, otra vez con el de ffmpeg.
    if r.is_err() && modo == Modo::Video && cfg!(target_os = "macos") && !cancelar.load(Relaxed) && !SIN_ACELERADOR.swap(true, Relaxed) {
        avance.store(0, Relaxed);
        r = hacer();
    }
    match r {
        Ok(()) => std::fs::rename(&tmp, salida).map_err(|e| e.to_string()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Con capas, efectos, títulos o transiciones: cada cuadro se compone aquí, igual que en la vista previa.
#[allow(clippy::too_many_arguments)]
fn componer(
    p: &Proyecto,
    almacen: &Almacen,
    tmp: &Path,
    op: &Opciones,
    ini: f64,
    dur: f64,
    avance: &AtomicU32,
    cancelar: &AtomicBool,
) -> Result<(), String> {
    // Cada exportación con el suyo: puede haber dos a la vez.
    static N: AtomicU32 = AtomicU32::new(0);
    let wav = std::env::temp_dir().join(format!("beethoven-craft-{}-{}.wav", std::process::id(), N.fetch_add(1, Relaxed)));
    let r = componer_con(p, almacen, tmp, &wav, op, ini, dur, avance, cancelar);
    let _ = std::fs::remove_file(&wav);
    r
}

#[allow(clippy::too_many_arguments)]
fn componer_con(
    p: &Proyecto,
    almacen: &Almacen,
    tmp: &Path,
    wav: &Path,
    op: &Opciones,
    ini: f64,
    dur: f64,
    avance: &AtomicU32,
    cancelar: &AtomicBool,
) -> Result<(), String> {
    let e = |e: std::io::Error| crate::idioma::tf("No pude escribir: {}", &[&e]);
    let mut w = Wav::crear(wav, 2, SR).map_err(e)?;
    let mut mal = None;
    let todo = motor::mezclar_desde(p, almacen, motor::cuadros(ini), motor::cuadros(dur), |_, b| {
        mal = w.escribir(b).err();
        mal.is_none() && !cancelar.load(Relaxed)
    });
    if let Some(x) = mal {
        return Err(e(x));
    }
    if !todo {
        return Err(crate::idioma::t("Exportación cancelada").into());
    }
    w.cerrar().map_err(e)?;
    let (an, al) = op.tam.unwrap_or((p.ancho, p.alto));
    let (an, al) = (an as usize / 2 * 2, al as usize / 2 * 2);
    // Casi todo se codifica a 4:2:0: los cuadros van así desde el archivo hasta el codificador, sin convertir.
    let yuv = op.codec != Codec::ProRes;
    let modo = Decodificar { chip: true, yuv, fino: true, ligera: false };
    let (fr, fps) = (medios::fps_fraccion(p.fps), medios::fps_exacto(p.fps));
    let mut hijo = medios::ffmpeg()
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .args(["-y", "-f", "rawvideo", "-pix_fmt", if yuv { "yuv420p" } else { "rgba" }, "-color_range", if yuv { "tv" } else { "pc" }, "-s", &format!("{an}x{al}"), "-r", &fr, "-i", "pipe:0", "-i"])
        .arg(wav)
        .args(["-map", "0:v", "-map", "1:a", "-vf", if yuv { "setparams=range=tv:colorspace=bt709:color_primaries=bt709:color_trc=bt709" } else { "scale=out_color_matrix=bt709:out_range=tv" }])
        .args(op.video())
        .args(["-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709"])
        .args(op.audio())
        .args(["-movflags", "+faststart", "-t", &format!("{dur:.6}")])
        .arg(tmp)
        .spawn()
        .map_err(|e| crate::idioma::tf("No pude ejecutar ffmpeg: {}", &[&e]))?;
    let mut ent = hijo.stdin.take().unwrap();
    let mut err = hijo.stderr.take().unwrap();
    let lector = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });
    let total = (dur * fps).ceil() as u64;
    // Casi al principio del cuadro no hay duda de cuál toca.
    let t_de = |n: u64| ini + (n as f64 + 0.02) / fps;
    en_paralelo(p, total, &t_de, (an, al), modo, &mut ent, avance, cancelar);
    drop(ent);
    if cancelar.load(Relaxed) {
        let _ = hijo.kill();
    }
    let estado = hijo.wait().map_err(|e| e.to_string())?;
    let texto = lector.join().unwrap_or_default();
    if cancelar.load(Relaxed) {
        return Err(crate::idioma::t("Exportación cancelada").into());
    }
    if !estado.success() {
        return Err(crate::idioma::tf("ffmpeg falló: {}", &[&(texto.lines().last().unwrap_or("sin detalle"))]));
    }
    Ok(())
}

/// Un hilo decodifica en orden, varios componen a la vez y aquí se escriben en orden a ffmpeg.
#[allow(clippy::too_many_arguments)]
fn en_paralelo(
    p: &Proyecto,
    total: u64,
    t_de: &(dyn Fn(u64) -> f64 + Sync),
    (an, al): (usize, usize),
    modo: Decodificar,
    ent: &mut impl Write,
    avance: &AtomicU32,
    cancelar: &AtomicBool,
) {
    let hilos = std::thread::available_parallelism().map_or(4, |n| n.get() / 2).clamp(2, 4);
    // Cuántos cuadros puede ir adelantado el decodificador: acota la memoria.
    let ventana = hilos as u64 * 3;
    let escritos = AtomicU64::new(0);
    let parar = AtomicBool::new(false);
    std::thread::scope(|s| {
        let (tx, trabajos) = bounded::<(u64, composicion::Prestado)>(hilos);
        let (listo, hechos) = bounded::<(u64, Vec<u8>)>(hilos);
        let (escritos, parar) = (&escritos, &parar);
        s.spawn(move || {
            let mut l = Lectores::nuevos(240.0, true).con(modo);
            for n in 0..total {
                while n >= escritos.load(Relaxed) + ventana && !parar.load(Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                if parar.load(Relaxed) || cancelar.load(Relaxed) {
                    break;
                }
                l.limpiar();
                // Los mismos cuadros que pediría la composición, y para los efectos de tiempo, lo que el lector tiene
                // en su cola en este momento: así componer en otro hilo da lo mismo que leer en orden.
                let pedidos = composicion::pedidos(p, t_de(n), an, al);
                let cuadros = pedidos.iter().filter_map(|&(c, ts, aw, ah)| Some((c.id, l.cuadro(c, ts, aw, ah)?))).collect();
                let recientes = pedidos.iter().filter(|x| composicion::de_tiempo(x.0)).map(|x| (x.0.id, l.recientes(x.0))).collect();
                if tx.send((n, composicion::Prestado { cuadros, recientes })).is_err() {
                    break;
                }
            }
        });
        for _ in 0..hilos {
            let (trabajos, listo) = (trabajos.clone(), listo.clone());
            s.spawn(move || {
                for (n, mut m) in trabajos {
                    let cuadro = if modo.yuv { composicion::componer_yuv(p, t_de(n), an, al, &mut m) } else { composicion::componer(p, t_de(n), an, al, &mut m) };
                    if listo.send((n, cuadro)).is_err() {
                        break;
                    }
                }
            });
        }
        drop((trabajos, listo));
        let mut espera = BTreeMap::new();
        let mut sig = 0;
        'fuera: for (n, cuadro) in hechos {
            espera.insert(n, cuadro);
            while let Some(cuadro) = espera.remove(&sig) {
                if cancelar.load(Relaxed) || ent.write_all(&cuadro).is_err() {
                    break 'fuera;
                }
                sig += 1;
                escritos.store(sig, Relaxed);
                avance.store((sig * 1000 / total) as u32, Relaxed);
            }
        }
        // Al salir antes de tiempo, los demás hilos se enteran y terminan.
        parar.store(true, Relaxed);
    });
}

/// El único clip, si va completo, sin cambios y ya está en el formato que se pide.
fn copiable<'a>(p: &'a Proyecto, op: &Opciones, dur: f64) -> Option<&'a crate::modelo::ClipVideo> {
    let [v] = p.video.as_slice() else { return None };
    let entero = v.inicio < 1e-3 && v.desde < 1e-3 && v.fin() >= dur - 0.05 && (v.ancho, v.alto) == (p.ancho, p.alto) && (v.fps - p.fps).abs() < 0.01;
    if !entero || op.codec == Codec::ProRes || op.calidad == 0 {
        return None;
    }
    let s = medios::comando(medios::herramienta("ffprobe"))
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=codec_name,pix_fmt,start_time", "-of", "csv=p=0"])
        .arg(&v.ruta)
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&s.stdout).trim().to_string();
    let x: Vec<&str> = s.split(',').collect();
    let codec = if op.codec == Codec::H264 { "h264" } else { "hevc" };
    (x.len() >= 3 && x[0] == codec && x[1] == "yuv420p" && x[2].parse::<f64>().is_ok_and(|t| t.abs() < 0.05)).then_some(v)
}

#[allow(clippy::too_many_arguments)]
fn escribir(
    p: &Proyecto,
    almacen: &Almacen,
    tmp: &Path,
    modo: Modo,
    op: &Opciones,
    ini: f64,
    dur: f64,
    avance: &AtomicU32,
    cancelar: &AtomicBool,
) -> Result<(), String> {
    let mut c = medios::ffmpeg();
    c.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());
    c.args(["-y", "-f", "f32le", "-ar", &SR.to_string(), "-ac", "2", "-i", "pipe:0"]);
    let d = format!("{dur:.6}");
    if modo == Modo::Video {
        let (w, h, fps) = (p.ancho / 2 * 2, p.alto / 2 * 2, medios::fps_fraccion(p.fps));
        if let Some(v) = copiable(p, op, dur) {
            // Nada cambió en la imagen: se pasa tal cual, sin volver a codificar.
            c.arg("-i").arg(&v.ruta).args(["-map", "1:v:0", "-map", "0:a", "-c:v", "copy"]).args(op.audio()).args(["-movflags", "+faststart"]);
        } else {
        let mut g = format!("color=black:s={w}x{h}:r={fps}:d={d},format=yuv420p[b0]");
        // Un solo clip que cubre todo no necesita el fondo negro.
        let solo = matches!(p.video.as_slice(), [v] if v.inicio < 1e-3 && v.fin() >= dur - 1e-3);
        for (i, v) in p.video.iter().enumerate() {
            let (n, fin) = (i + 1, v.inicio + v.dur);
            // El clip se mueve antes de `fps`: así se redondea una sola vez, a la rejilla de la salida.
            c.args(["-ss", &format!("{:.6}", v.desde), "-t", &format!("{:.6}", v.dur), "-i"]).arg(&v.ruta);
            g += &format!(
                ";[{n}:v:0]setpts=PTS+{:.6}/TB,fps={fps}:start_time={:.6},scale={w}:{h}:force_original_aspect_ratio=decrease,\
                 pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,format=yuv420p[v{n}]\
                 ;[b{i}][v{n}]overlay=eof_action=pass:enable='gte(t,{:.6})*lt(t,{fin:.6})'[b{n}]",
                v.inicio, v.inicio, v.inicio
            );
        }
        if solo {
            g = format!("[1:v:0]setpts=PTS-STARTPTS,fps={fps}:start_time=0,scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,format=yuv420p[b1]");
        }
        c.args(["-filter_complex", &g, "-map", &format!("[b{}]", p.video.len()), "-map", "0:a"]);
        c.args(op.video()).args(op.audio()).args(["-movflags", "+faststart"]);
        }
    } else {
        match tmp.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase().as_str() {
            "wav" => c.args(["-c:a", "pcm_s24le"]),
            "flac" => c.args(["-c:a", "flac"]),
            "mp3" => c.args(["-c:a", "libmp3lame", "-b:a", "320k"]),
            "m4a" => c.args(["-c:a", "aac", "-b:a", "256k"]),
            e => return Err(crate::idioma::tf("No sé exportar audio en formato «{}»", &[&e])),
        };
    }
    c.args(["-t", &d]).arg(tmp);
    let mut hijo = c.spawn().map_err(|e| crate::idioma::tf("No pude ejecutar ffmpeg: {}", &[&e]))?;
    let mut ent = hijo.stdin.take().unwrap();
    let mut err = hijo.stderr.take().unwrap();
    let lector = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });
    let total = motor::cuadros(dur);
    let mut bytes = Vec::new();
    let mut falla = false;
    let completo = motor::mezclar_desde(p, almacen, motor::cuadros(ini), total, |t, b| {
        bytes.clear();
        b.iter().for_each(|x| bytes.extend_from_slice(&x.to_le_bytes()));
        falla = ent.write_all(&bytes).is_err();
        avance.store((t * 1000 / total) as u32, Relaxed);
        !falla && !cancelar.load(Relaxed)
    });
    drop(ent);
    if !completo && !falla {
        let _ = hijo.kill();
    }
    // Espera a ffmpeg sin dejar de atender la cancelación.
    let estado = loop {
        if let Some(e) = hijo.try_wait().map_err(|e| e.to_string())? {
            break e;
        }
        if cancelar.load(Relaxed) {
            let _ = hijo.kill();
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let texto = lector.join().unwrap_or_default();
    if cancelar.load(Relaxed) {
        return Err(crate::idioma::t("Exportación cancelada").into());
    }
    if !estado.success() {
        return Err(crate::idioma::tf("ffmpeg falló: {}", &[&(texto.lines().last().unwrap_or("sin detalle"))]));
    }
    Ok(())
}
