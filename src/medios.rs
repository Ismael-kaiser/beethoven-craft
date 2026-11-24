//! Lectura de archivos con ffmpeg / ffprobe.
use crate::modelo::SR;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Muestras por punto de la forma de onda.
pub const PUNTO: usize = 256;

/// Las muestras de un audio. Las largas viven en un archivo temporal y el sistema trae a la RAM solo lo que se va usando.
pub enum Cuadros {
    Ram(Vec<[f32; 2]>),
    Disco(memmap2::Mmap),
}

impl std::ops::Deref for Cuadros {
    type Target = [[f32; 2]];
    fn deref(&self) -> &[[f32; 2]] {
        match self {
            Cuadros::Ram(v) => v,
            // SEGURO: el archivo lo escribió esta app con pares de f32, mide un múltiplo de 8 bytes, la memoria mapeada
            // empieza al inicio de una página y cualquier patrón de bits es un f32 válido.
            Cuadros::Disco(m) => unsafe { std::slice::from_raw_parts(m.as_ptr().cast(), m.len() / 8) },
        }
    }
}

/// Desde cuántos cuadros se van al disco (20 s).
const AL_DISCO: usize = SR as usize * 20;

/// Archivo temporal para un audio. En Mac y Linux se borra del listado al abrirlo; en Windows, al soltarlo.
struct Temporal {
    f: std::io::BufWriter<std::fs::File>,
    n: usize,
    #[cfg(not(unix))]
    ruta: PathBuf,
}

impl Temporal {
    fn nuevo() -> Option<Temporal> {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let ruta = std::env::temp_dir().join(format!("beethoven-audio-{}-{n}", std::process::id()));
        let f = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&ruta).ok()?;
        #[cfg(unix)]
        let _ = std::fs::remove_file(&ruta);
        Some(Temporal {
            f: std::io::BufWriter::with_capacity(1 << 18, f),
            n: 0,
            #[cfg(not(unix))]
            ruta,
        })
    }
    fn escribir(&mut self, v: &[[f32; 2]]) -> std::io::Result<()> {
        for x in v {
            self.f.write_all(&x[0].to_ne_bytes())?;
            self.f.write_all(&x[1].to_ne_bytes())?;
        }
        self.n += v.len();
        Ok(())
    }
    fn mapa(mut self) -> Option<Cuadros> {
        self.f.flush().ok()?;
        // SEGURO: nadie más tiene este archivo; nadie lo cambia ni lo recorta mientras esté mapeado.
        let m = unsafe { memmap2::Mmap::map(self.f.get_ref()).ok()? };
        // En Windows el archivo mapeado no se puede borrar todavía: lo limpia el sistema con los demás temporales.
        #[cfg(not(unix))]
        let _ = std::fs::remove_file(&self.ruta);
        (m.len() == self.n * 8).then_some(Cuadros::Disco(m))
    }
}

pub struct Audio {
    pub cuadros: Cuadros,
    /// (mínimo, máximo) cada `PUNTO` cuadros.
    pub onda: Vec<(f32, f32)>,
}

impl Audio {
    pub fn nuevo(cuadros: Vec<[f32; 2]>) -> Self {
        Self::con(Cuadros::Ram(cuadros))
    }
    /// Trae a la RAM lo que está por sonar, para que leerlo del disco no atrase el sonido.
    pub fn preparar(&self, de: f64, a: f64) {
        if let Cuadros::Disco(_) = self.cuadros {
            let i = ((de.max(0.0) * SR as f64) as usize).min(self.cuadros.len());
            let j = ((a.max(0.0) * SR as f64) as usize).clamp(i, self.cuadros.len());
            // Una muestra por página basta.
            std::hint::black_box(self.cuadros[i..j].iter().step_by(512).map(|x| x[0]).sum::<f32>());
        }
    }
    fn con(cuadros: Cuadros) -> Self {
        let onda = cuadros
            .chunks(PUNTO)
            .map(|c| c.iter().flatten().fold((0f32, 0f32), |(a, b), &x| (a.min(x), b.max(x))))
            .collect();
        Audio { cuadros, onda }
    }
    pub fn dur(&self) -> f64 {
        self.cuadros.len() as f64 / SR as f64
    }
    /// Pico entre dos segundos del archivo.
    pub fn pico(&self, a: f64, b: f64) -> f32 {
        let i = ((a * SR as f64) as usize).min(self.cuadros.len());
        let j = ((b * SR as f64) as usize).clamp(i, self.cuadros.len());
        self.cuadros[i..j].iter().flatten().fold(0.0, |m, x| m.max(x.abs()))
    }
}

/// Cuadros por segundo como fracción: el promedio de un archivo casi nunca es exacto (23.97607 es 24000/1001).
pub fn fps_fraccion(fps: f64) -> String {
    let n = fps.round();
    for (v, t) in [(n * 1000.0 / 1001.0, format!("{:.0}/1001", n * 1000.0)), (n, format!("{n:.0}"))] {
        if (fps - v).abs() < v * 0.0004 {
            return t;
        }
    }
    format!("{fps:.5}")
}

/// El número que corresponde a `fps_fraccion`.
pub fn fps_exacto(fps: f64) -> f64 {
    let f = fps_fraccion(fps);
    match f.split_once('/') {
        Some((a, b)) => a.parse::<f64>().unwrap_or(fps) / b.parse::<f64>().unwrap_or(1.0),
        None => f.parse().unwrap_or(fps),
    }
}

/// Ruta de una herramienta: una app abierta desde Finder no hereda el PATH de la terminal.
/// Un programa de ayuda (ffmpeg, ffprobe): junto a la app, en Homebrew (Mac) o, si no, el del PATH.
pub fn herramienta(nombre: &str) -> PathBuf {
    let archivo = format!("{nombre}{}", std::env::consts::EXE_SUFFIX);
    let junto = std::env::current_exe().ok().and_then(|e| Some(e.parent()?.join(&archivo)));
    let homebrew = ["/opt/homebrew/bin", "/usr/local/bin"].iter().filter(|_| cfg!(target_os = "macos")).map(|d| Path::new(d).join(&archivo));
    junto.into_iter().chain(homebrew).find(|p| p.is_file()).unwrap_or_else(|| archivo.into())
}

/// Un proceso sin ventana: Windows abre una consola por cada programa de consola que se lanza.
pub fn comando(programa: impl AsRef<std::ffi::OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(programa);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

pub fn ffmpeg() -> Command {
    let mut c = comando(herramienta("ffmpeg"));
    c.args(["-hide_banner", "-nostdin", "-v", "error"]).stdin(Stdio::null());
    c
}

#[derive(Clone, Debug, Default)]
pub struct Info {
    pub audio: bool,
    /// ancho, alto, fps
    pub video: Option<(u32, u32, f64)>,
    pub dur: f64,
}

/// Memoria RAM de la máquina, en bytes.
pub fn memoria_total() -> u64 {
    let de = || -> Option<u64> {
        if cfg!(target_os = "macos") {
            let s = comando("sysctl").args(["-n", "hw.memsize"]).output().ok()?;
            return String::from_utf8_lossy(&s.stdout).trim().parse().ok();
        }
        let s = std::fs::read_to_string("/proc/meminfo").ok()?;
        s.lines().find(|l| l.starts_with("MemTotal"))?.split_whitespace().nth(1)?.parse::<u64>().ok().map(|k| k * 1024)
    };
    de().unwrap_or(8 << 30)
}

/// ¿El video trae transparencia? Se pregunta una vez por archivo.
pub fn con_alfa(ruta: &Path) -> bool {
    static C: std::sync::Mutex<Vec<(PathBuf, bool)>> = std::sync::Mutex::new(vec![]);
    if let Some(x) = C.lock().unwrap().iter().find(|x| x.0 == ruta) {
        return x.1;
    }
    let s = comando(herramienta("ffprobe"))
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=pix_fmt:stream_tags=alpha_mode", "-of", "csv=p=0"])
        .arg(ruta)
        .output();
    let s = s.map(|s| String::from_utf8_lossy(&s.stdout).to_string()).unwrap_or_default();
    // Formatos con canal de transparencia, y los WebM que lo traen aparte.
    let si = s.split([',', '\n']).any(|x| ["yuva", "rgba", "bgra", "argb", "abgr", "ya8", "ya16", "gbrap", "pal8", "ayuv"].iter().any(|a| x.trim().starts_with(a)) || x.trim() == "1");
    C.lock().unwrap().push((ruta.to_path_buf(), si));
    si
}

pub fn sondear(ruta: &Path) -> Result<Info, String> {
    let s = comando(herramienta("ffprobe"))
        .args(["-v", "error", "-show_streams", "-show_format", "-of", "json"])
        .arg(ruta)
        .output()
        .map_err(|e| crate::idioma::tf("No pude ejecutar ffprobe: {}", &[&e]))?;
    let v: serde_json::Value =
        serde_json::from_slice(&s.stdout).map_err(|_| crate::idioma::t("No reconozco ese archivo").to_string())?;
    let mut i = Info { dur: num(&v["format"]["duration"]).unwrap_or(0.0), ..Default::default() };
    for st in v["streams"].as_array().into_iter().flatten() {
        match st["codec_type"].as_str() {
            Some("audio") => i.audio = true,
            // Las carátulas de los MP3 aparecen como video.
            Some("video") if st["disposition"]["attached_pic"] != 1 && i.video.is_none() => {
                let fps = st["avg_frame_rate"]
                    .as_str()
                    .and_then(|r| r.split_once('/'))
                    .and_then(|(a, b)| Some(a.parse::<f64>().ok()? / b.parse::<f64>().ok()?))
                    .filter(|f| f.is_finite() && *f > 1.0)
                    .unwrap_or(30.0);
                let (mut w, mut h) =
                    (st["width"].as_u64().unwrap_or(0) as u32, st["height"].as_u64().unwrap_or(0) as u32);
                // Videos de celular: la imagen viene girada y ffmpeg la endereza al leerla.
                let giro = st["side_data_list"]
                    .as_array()
                    .and_then(|l| l.iter().find_map(|d| d["rotation"].as_i64()))
                    .unwrap_or(0);
                if giro.rem_euclid(180) == 90 {
                    (w, h) = (h, w);
                }
                if w > 0 && h > 0 {
                    i.video = Some((w, h, fps.min(60.0)));
                }
            }
            _ => {}
        }
    }
    if !i.audio && i.video.is_none() {
        return Err(crate::idioma::t("Ese archivo no tiene audio ni video").into());
    }
    Ok(i)
}

fn num(v: &serde_json::Value) -> Option<f64> {
    v.as_str()?.parse().ok()
}

pub fn decodificar(ruta: &Path) -> Result<Audio, String> {
    // Respeta el arranque del audio dentro del video (algunos empiezan tarde).
    leer_audio(&mut ffmpeg(), ruta, "aresample=async=1:first_pts=0")
}

/// Como `leer_f32`, pero lo largo va cayendo al disco mientras se lee.
fn leer_audio(ffmpeg: &mut Command, ruta: &Path, filtro: &str) -> Result<Audio, String> {
    let mut ram: Vec<[f32; 2]> = vec![];
    let mut disco: Option<Temporal> = None;
    let mut roto = false;
    leer_f32_en(ffmpeg, ruta, SR, filtro, &mut |v| {
        ram.extend_from_slice(v);
        if ram.len() >= AL_DISCO && !roto {
            if disco.is_none() {
                disco = Temporal::nuevo();
            }
            match disco.as_mut().map(|d| d.escribir(&ram)) {
                Some(Ok(())) => ram.clear(),
                // Sin disco (lleno o sin permiso) se sigue en RAM.
                _ => roto = true,
            }
        }
    })?;
    match disco {
        Some(mut d) if !roto => {
            d.escribir(&ram).map_err(|e| e.to_string())?;
            d.mapa().map(Audio::con).ok_or_else(|| crate::idioma::t("No pude guardar el audio temporal").to_string())
        }
        // Se llenó a medias: se lee otra vez, completo en RAM.
        Some(_) => decodificar_a(ruta, SR).map(Audio::nuevo),
        None => Ok(Audio::nuevo(ram)),
    }
}

/// Como `decodificar`, a otra velocidad y con el mismo tono.
pub fn decodificar_a_velocidad(ruta: &Path, v: f64) -> Result<Audio, String> {
    // `atempo` acepta de 0.5 en adelante: más lento se hace en dos pasos.
    let f = if v < 0.5 { format!("atempo={},atempo={}", v.sqrt(), v.sqrt()) } else { format!("atempo={v}") };
    leer_audio(&mut ffmpeg(), ruta, &format!("aresample=async=1:first_pts=0,{f}"))
}

/// Audio estéreo de un archivo a la frecuencia `sr`.
pub fn decodificar_a(ruta: &Path, sr: u32) -> Result<Vec<[f32; 2]>, String> {
    // Respeta el arranque del audio dentro del video (algunos empiezan tarde).
    decodificar_con(ruta, sr, "aresample=async=1:first_pts=0")
}

/// Un tramo del audio, para oírlo sin leer todo el archivo.
pub fn decodificar_tramo(ruta: &Path, desde: f64, dur: f64) -> Result<Audio, String> {
    let corte = ["-ss".to_string(), format!("{desde:.3}"), "-t".into(), format!("{dur:.3}")];
    leer_f32(ffmpeg().args(corte), ruta, SR, "aresample=async=1").map(Audio::nuevo)
}

fn decodificar_con(ruta: &Path, sr: u32, filtro: &str) -> Result<Vec<[f32; 2]>, String> {
    leer_f32(&mut ffmpeg(), ruta, sr, filtro)
}

fn leer_f32(ffmpeg: &mut Command, ruta: &Path, sr: u32, filtro: &str) -> Result<Vec<[f32; 2]>, String> {
    let mut v = vec![];
    leer_f32_en(ffmpeg, ruta, sr, filtro, &mut |x| v.extend_from_slice(x))?;
    Ok(v)
}

fn leer_f32_en(ffmpeg: &mut Command, ruta: &Path, sr: u32, filtro: &str, dar: &mut dyn FnMut(&[[f32; 2]])) -> Result<(), String> {
    let mut h = ffmpeg
        .arg("-i")
        .arg(ruta)
        .args(["-vn", "-map", "0:a:0"])
        .args(["-af", filtro])
        .args(["-ac", "2", "-ar", &sr.to_string(), "-f", "f32le", "pipe:1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| crate::idioma::tf("No pude ejecutar ffmpeg: {}", &[&e]))?;
    let mut o = h.stdout.take().unwrap();
    let mut v: Vec<[f32; 2]> = vec![];
    let mut total = 0;
    let mut b = vec![0u8; 1 << 16];
    let mut lleno = 0;
    loop {
        let n = o.read(&mut b[lleno..]).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        lleno += n;
        let usar = lleno / 8 * 8;
        v.clear();
        v.extend(b[..usar].chunks_exact(8).map(|c| {
            [f32::from_le_bytes(c[..4].try_into().unwrap()), f32::from_le_bytes(c[4..].try_into().unwrap())]
        }));
        dar(&v);
        total += v.len();
        b.copy_within(usar..lleno, 0);
        lleno -= usar;
    }
    let mut e = String::new();
    let _ = h.stderr.take().unwrap().read_to_string(&mut e);
    let _ = h.wait();
    if total == 0 {
        return Err(crate::idioma::tf("No pude leer el audio: {}", &[&(e.lines().next().unwrap_or("archivo vacío"))]));
    }
    Ok(())
}

/// Escribe un WAV de 32 bits flotantes por partes; el encabezado se corrige al cerrar.
pub struct Wav {
    f: std::io::BufWriter<std::fs::File>,
    datos: u32,
}

impl Wav {
    pub fn crear(ruta: &Path, canales: u16, sr: u32) -> std::io::Result<Wav> {
        let mut f = std::io::BufWriter::new(std::fs::File::create(ruta)?);
        let ba = canales * 4;
        f.write_all(b"RIFF\0\0\0\0WAVEfmt ")?;
        f.write_all(&16u32.to_le_bytes())?;
        f.write_all(&3u16.to_le_bytes())?;
        f.write_all(&canales.to_le_bytes())?;
        f.write_all(&sr.to_le_bytes())?;
        f.write_all(&(sr * ba as u32).to_le_bytes())?;
        f.write_all(&ba.to_le_bytes())?;
        f.write_all(&32u16.to_le_bytes())?;
        f.write_all(b"data\0\0\0\0")?;
        Ok(Wav { f, datos: 0 })
    }
    pub fn escribir(&mut self, m: &[f32]) -> std::io::Result<()> {
        for x in m {
            self.f.write_all(&x.to_le_bytes())?;
        }
        self.datos += 4 * m.len() as u32;
        Ok(())
    }
    pub fn cerrar(mut self) -> std::io::Result<()> {
        use std::io::{Seek, SeekFrom};
        self.f.seek(SeekFrom::Start(4))?;
        self.f.write_all(&(36 + self.datos).to_le_bytes())?;
        self.f.seek(SeekFrom::Start(40))?;
        self.f.write_all(&self.datos.to_le_bytes())?;
        self.f.flush()
    }
}
