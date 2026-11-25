// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Abre audio de cualquier formato común: MP3, OGG, FLAC, M4A, AIFF, CAF, WAV… Lo que no (Opus, WMA, el sonido de un
//! video) se le pide a ffmpeg si está instalado.
use crate::idioma::{t, tf};
use crate::wav::Wav;
use std::path::{Path, PathBuf};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub const EXTENSIONES: &[&str] =
    &["wav", "mp3", "ogg", "oga", "opus", "flac", "m4a", "mp4", "aac", "aif", "aiff", "caf", "wma", "webm", "mka", "mov", "mkv", "amr", "3gp"];

/// Tope de lo que se abre: dos horas.
const MAX_SEG: usize = 2 * 60 * 60;

pub fn es_audio(p: &Path) -> bool {
    p.extension().is_some_and(|e| EXTENSIONES.contains(&e.to_string_lossy().to_lowercase().as_str()))
}

pub fn leer(ruta: &Path) -> Result<Wav, String> {
    let ext = ruta.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    // El WAV va directo; si trae algo raro adentro, sigue con los demás.
    if ext == "wav"
        && let Ok(w) = crate::wav::leer(ruta)
    {
        return Ok(w);
    }
    match simphonia(ruta, &ext) {
        Ok(w) => Ok(w),
        Err(e) => con_ffmpeg(ruta).map_err(|f| f.unwrap_or(e)),
    }
}

fn simphonia(ruta: &Path, ext: &str) -> Result<Wav, String> {
    let f = std::fs::File::open(ruta).map_err(|e| e.to_string())?;
    decodificar(MediaSourceStream::new(Box::new(f), Default::default()), ext)
}

/// Audio que ya está en memoria, como cada grabación de un SoundFont .sf3 (Ogg Vorbis).
pub fn de_bytes(b: Vec<u8>, ext: &str) -> Result<Wav, String> {
    decodificar(MediaSourceStream::new(Box::new(std::io::Cursor::new(b)), Default::default()), ext)
}

fn decodificar(mss: MediaSourceStream, ext: &str) -> Result<Wav, String> {
    let raro = || t("no se reconoce el formato de audio").to_string();
    let mut hint = Hint::new();
    hint.with_extension(ext);
    let gapless = FormatOptions { enable_gapless: true, ..Default::default() };
    let p = symphonia::default::get_probe().format(&hint, mss, &gapless, &MetadataOptions::default()).map_err(|_| raro())?;
    let mut f = p.format;
    let pista = f.tracks().iter().find(|t| t.codec_params.sample_rate.is_some()).ok_or_else(raro)?;
    let id = pista.id;
    let mut sr = pista.codec_params.sample_rate.unwrap_or(44100);
    let mut d = symphonia::default::get_codecs().make(&pista.codec_params, &DecoderOptions::default()).map_err(|_| raro())?;
    let (mut izq, mut der) = (vec![], vec![]);
    let mut b: Option<SampleBuffer<f32>> = None;
    // Se acaba con un error de fin de archivo; lo ya leído sirve aunque el final esté cortado.
    while let Ok(q) = f.next_packet() {
        if q.track_id() != id {
            continue;
        }
        let x = match d.decode(&q) {
            Ok(x) => x,
            Err(Error::DecodeError(_)) => continue,
            Err(_) => break,
        };
        sr = x.spec().rate;
        let canales = x.spec().channels.count().max(1);
        if b.as_ref().is_none_or(|b| b.capacity() < x.capacity() * canales) {
            b = Some(SampleBuffer::new(x.capacity() as u64, *x.spec()));
        }
        let b = b.as_mut().unwrap();
        b.copy_interleaved_ref(x);
        for c in b.samples().chunks_exact(canales) {
            izq.push(c[0]);
            der.push(c[if canales > 1 { 1 } else { 0 }]);
        }
        if izq.len() > MAX_SEG * sr as usize {
            return Err(t("el audio dura más de dos horas").into());
        }
    }
    if izq.is_empty() { Err(t("el archivo no trae sonido").into()) } else { Ok(Wav { izq, der, sr: sr as f32 }) }
}

/// Dónde está ffmpeg. Una app abierta desde el Finder no hereda el PATH de la terminal.
pub fn ffmpeg() -> Option<PathBuf> {
    let nombre = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    let del_path = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default();
    let fijas = ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"].map(PathBuf::from);
    del_path.into_iter().chain(fijas).map(|d| d.join(nombre)).find(|p| p.is_file())
}

/// `Err(None)`: no hay ffmpeg.
fn con_ffmpeg(ruta: &Path) -> Result<Wav, Option<String>> {
    let f = ffmpeg().ok_or(None)?;
    let o = std::process::Command::new(f)
        .args(["-v", "error", "-nostdin", "-i"])
        .arg(ruta)
        .args(["-vn", "-t", &MAX_SEG.to_string(), "-f", "f32le", "-ac", "2", "-ar", "48000", "-"])
        .output()
        .map_err(|e| Some(e.to_string()))?;
    if !o.status.success() || o.stdout.is_empty() {
        return Err(Some(t("no se reconoce el formato de audio").into()));
    }
    let (mut izq, mut der) = (vec![], vec![]);
    for c in o.stdout.chunks_exact(8) {
        izq.push(f32::from_le_bytes([c[0], c[1], c[2], c[3]]));
        der.push(f32::from_le_bytes([c[4], c[5], c[6], c[7]]));
    }
    Ok(Wav { izq, der, sr: 48000.0 })
}

/// Guarda en el formato que diga la extensión. WAV es propio; los demás los hace ffmpeg.
pub fn escribir(ruta: &Path, sr: u32, x: &[[f32; 2]]) -> Result<(), String> {
    let ext = ruta.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if ext == "wav" {
        return crate::wav::escribir(ruta, sr, x);
    }
    let f = ffmpeg().ok_or_else(|| tf("Para exportar {} hace falta ffmpeg", &[&ext.to_uppercase()]))?;
    // En orden de preferencia: no todos los ffmpeg traen los mismos.
    let calidad: &[&[&str]] = match ext.as_str() {
        "mp3" => &[&["-codec:a", "libmp3lame", "-b:a", "320k"]],
        "flac" => &[&["-codec:a", "flac"]],
        "ogg" => &[&["-codec:a", "libvorbis", "-q:a", "7"], &["-codec:a", "libopus", "-b:a", "192k"]],
        "m4a" => &[&["-codec:a", "aac", "-b:a", "256k"]],
        _ => return Err(t("No conozco ese tipo de archivo").into()),
    };
    let temporal = std::env::temp_dir().join(format!("bdaw-{}.wav", std::process::id()));
    crate::wav::escribir(&temporal, sr, x)?;
    let mut r = Ok(());
    for c in calidad {
        let o = std::process::Command::new(&f).args(["-v", "error", "-nostdin", "-y", "-i"]).arg(&temporal).args(*c).arg(ruta).output();
        r = match o {
            Ok(o) if o.status.success() => Ok(()),
            Ok(o) => Err(String::from_utf8_lossy(&o.stderr).trim().to_string()),
            Err(e) => Err(e.to_string()),
        };
        if r.is_ok() {
            break;
        }
    }
    let _ = std::fs::remove_file(&temporal);
    r
}
