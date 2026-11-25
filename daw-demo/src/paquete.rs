// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! La carpeta de un guion: `voz gruesa/voz gruesa.lua` es el principal; los demás `.lua` se pueden traer con
//! `incluir("otro")` y las imágenes PNG se ven con `logo = "logo.png"` y `d.imagen(…)`. Todo se lee aquí, con topes de
//! tamaño, antes de darlo al guion: el guion sigue sin poder abrir archivos.
use crate::idioma::{t as tr, tf};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// Tope de cada imagen y de todo lo que trae una carpeta.
pub const MAX_IMAGEN: usize = 8 << 20;
pub const MAX_TOTAL: usize = 32 << 20;
/// Lado máximo de una imagen, en píxeles.
pub const MAX_LADO: u32 = 2048;

/// Lo que trae la carpeta de un guion, además del principal.
#[derive(Debug, Default)]
pub struct Archivos {
    /// Cada carpeta leída tiene el suyo: así las imágenes ya mostradas se reconocen aunque se vuelvan a leer.
    pub id: u64,
    /// Otros guiones, por nombre sin «.lua».
    pub lua: BTreeMap<String, String>,
    /// Imágenes por nombre con su extensión («logo.png», «fondos/noche.png»).
    pub imagenes: BTreeMap<String, Arc<Vec<u8>>>,
}

static SIG: AtomicU64 = AtomicU64::new(1);

impl Archivos {
    pub fn nuevos() -> Archivos {
        Archivos { id: SIG.fetch_add(1, Relaxed), ..Default::default() }
    }

    pub fn imagen(&self, nombre: &str) -> Option<&Arc<Vec<u8>>> {
        self.imagenes.get(nombre)
    }

    /// Agrega una imagen revisando que sea un PNG que se puede leer y que no pase de los topes.
    pub fn poner_imagen(&mut self, nombre: &str, b: Vec<u8>) -> Result<(), String> {
        if b.len() > MAX_IMAGEN {
            return Err(tf("«{}» pesa más de 8 MB", &[&nombre]));
        }
        let total: usize = self.imagenes.values().map(|x| x.len()).sum::<usize>() + b.len();
        if total > MAX_TOTAL {
            return Err(tr("las imágenes de la carpeta pasan de 32 MB").into());
        }
        let (w, h, _) = rgba(&b).ok_or_else(|| tf("«{}» no es un PNG que se pueda leer", &[&nombre]))?;
        if w > MAX_LADO || h > MAX_LADO {
            return Err(tf("«{}» mide más de 2048 píxeles de lado", &[&nombre]));
        }
        self.imagenes.insert(nombre.to_string(), Arc::new(b));
        Ok(())
    }
}

/// Un PNG a RGBA de 8 bits: ancho, alto y píxeles. `None` si no se puede leer.
pub fn rgba(b: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut d = png::Decoder::new(std::io::Cursor::new(b));
    d.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = d.read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()?];
    let info = r.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let px = (w * h) as usize;
    let datos = &buf[..info.buffer_size()];
    let v = match info.color_type {
        png::ColorType::Rgba => datos.to_vec(),
        png::ColorType::Rgb => datos.chunks_exact(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => datos.chunks_exact(2).flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
        png::ColorType::Grayscale => datos.iter().flat_map(|g| [*g, *g, *g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    (v.len() == px * 4).then_some((w, h, v))
}

/// Lee la carpeta de un guion: da su principal y lo demás. El principal se llama como la carpeta
/// («voz gruesa/voz gruesa.lua»); si no hay, sirve el único `.lua` que tenga.
pub fn leer(dir: &Path) -> Result<(String, Archivos), String> {
    let nombre = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let (mut luas, mut a) = todo(dir)?;
    let principal = if luas.contains_key(&nombre) {
        nombre.clone()
    } else if luas.len() == 1 {
        luas.keys().next().cloned().unwrap_or_default()
    } else {
        return Err(tf("a la carpeta le falta «{}.lua»", &[&nombre]));
    };
    let fuente = luas.remove(&principal).unwrap_or_default();
    a.lua = luas;
    Ok((fuente, a))
}

/// Lo que hay en la carpeta además del principal (`principal`, sin «.lua»). Una carpeta que no existe no trae nada.
pub fn extras(dir: &Path, principal: &str) -> Result<Archivos, String> {
    if !dir.is_dir() {
        return Ok(Archivos::nuevos());
    }
    let (mut luas, mut a) = todo(dir)?;
    luas.remove(principal);
    a.lua = luas;
    Ok(a)
}

/// Todos los `.lua` (por nombre, sin «.lua») y las imágenes de la carpeta y de sus subcarpetas directas.
fn todo(dir: &Path) -> Result<(BTreeMap<String, String>, Archivos), String> {
    let mut a = Archivos::nuevos();
    let mut luas: BTreeMap<String, String> = BTreeMap::new();
    // Una subcarpeta de profundidad (para «fondos/noche.png»), sin seguir enlaces.
    let mut pendientes = vec![(dir.to_path_buf(), String::new())];
    while let Some((d, pre)) = pendientes.pop() {
        for e in std::fs::read_dir(&d).map_err(|e| e.to_string())?.flatten() {
            let (p, tipo) = (e.path(), e.file_type().map_err(|e| e.to_string())?);
            let n = format!("{pre}{}", e.file_name().to_string_lossy());
            if tipo.is_dir() && pre.is_empty() {
                pendientes.push((p, format!("{n}/")));
            } else if tipo.is_file() {
                match p.extension().map(|x| x.to_string_lossy().to_lowercase()).as_deref() {
                    Some("lua") if pre.is_empty() => {
                        let f = std::fs::read_to_string(&p).map_err(|e| format!("{n}: {e}"))?;
                        luas.insert(n.trim_end_matches(".lua").to_string(), f);
                    }
                    Some("png") => {
                        let b = std::fs::read(&p).map_err(|e| format!("{n}: {e}"))?;
                        a.poner_imagen(&n, b)?;
                    }
                    _ => {}
                }
            }
        }
    }
    Ok((luas, a))
}
