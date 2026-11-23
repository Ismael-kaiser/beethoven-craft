//! Lectura de archivos con ffmpeg / ffprobe.
use crate::modelo::SR;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Muestras por punto de la forma de onda.
pub const PUNTO: usize = 256;

pub struct Audio {
    pub cuadros: Vec<[f32; 2]>,
    /// (mínimo, máximo) cada `PUNTO` cuadros.
    pub onda: Vec<(f32, f32)>,
}

impl Audio {
    pub fn nuevo(cuadros: Vec<[f32; 2]>) -> Self {
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
    decodificar_a(ruta, SR).map(Audio::nuevo)
}

/// Como `decodificar`, a otra velocidad y con el mismo tono.
pub fn decodificar_a_velocidad(ruta: &Path, v: f64) -> Result<Audio, String> {
    // `atempo` acepta de 0.5 en adelante: más lento se hace en dos pasos.
    let f = if v < 0.5 { format!("atempo={},atempo={}", v.sqrt(), v.sqrt()) } else { format!("atempo={v}") };
    decodificar_con(ruta, SR, &format!("aresample=async=1:first_pts=0,{f}")).map(Audio::nuevo)
}

/// Audio estéreo de un archivo a la frecuencia `sr`.
pub fn decodificar_a(ruta: &Path, sr: u32) -> Result<Vec<[f32; 2]>, String> {
    // Respeta el arranque del audio dentro del video (algunos empiezan tarde).
    decodificar_con(ruta, sr, "aresample=async=1:first_pts=0")
}

fn decodificar_con(ruta: &Path, sr: u32, filtro: &str) -> Result<Vec<[f32; 2]>, String> {
    let mut h = ffmpeg()
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
    let mut b = vec![0u8; 1 << 16];
    let mut lleno = 0;
    loop {
        let n = o.read(&mut b[lleno..]).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        lleno += n;
        let usar = lleno / 8 * 8;
        v.extend(b[..usar].chunks_exact(8).map(|c| {
            [f32::from_le_bytes(c[..4].try_into().unwrap()), f32::from_le_bytes(c[4..].try_into().unwrap())]
        }));
        b.copy_within(usar..lleno, 0);
        lleno -= usar;
    }
    let mut e = String::new();
    let _ = h.stderr.take().unwrap().read_to_string(&mut e);
    let _ = h.wait();
    if v.is_empty() {
        return Err(crate::idioma::tf("No pude leer el audio: {}", &[&(e.lines().next().unwrap_or("archivo vacío"))]));
    }
    Ok(v)
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
