//! El archivo de proyecto `.bcraft`: binario propio por bloques, comprimido y con suma de control.
//!
//! `BCRAFT` + versión (u16), y luego bloques: etiqueta de 4 letras, largo (u64), CRC-32 (u32) y datos.
//! `PROY` es el proyecto (MessagePack comprimido), `MINI` una miniatura y `MEDI` un archivo incluido.
use crate::modelo::{Contenido, Instrumento, Proyecto};
use miniz_oxide::deflate::compress_to_vec;
use miniz_oxide::inflate::decompress_to_vec_with_limit;
use std::collections::BTreeSet;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const MAGIA: &[u8; 6] = b"BCRAFT";
const VERSION: u16 = 1;
/// Un proyecto descomprimido más grande que esto está dañado.
const TOPE: usize = 256 << 20;

pub struct Crc(u32);

impl Crc {
    pub fn nuevo() -> Self {
        Crc(!0)
    }
    pub fn meter(&mut self, b: &[u8]) {
        for x in b {
            self.0 ^= *x as u32;
            for _ in 0..8 {
                self.0 = (self.0 >> 1) ^ (0xEDB8_8320 & (self.0 & 1).wrapping_neg());
            }
        }
    }
    pub fn fin(&self) -> u32 {
        !self.0
    }
}

fn crc(b: &[u8]) -> u32 {
    let mut c = Crc::nuevo();
    c.meter(b);
    c.fin()
}

/// Archivos que usa el proyecto.
pub fn medios(p: &Proyecto) -> Vec<PathBuf> {
    let mut v: BTreeSet<PathBuf> = BTreeSet::new();
    v.extend(p.video.iter().filter(|c| matches!(c.contenido, Contenido::Archivo | Contenido::Imagen)).map(|c| c.ruta.clone()));
    v.extend(p.pistas.iter().flat_map(|t| &t.clips).map(|c| c.ruta.clone()));
    v.extend(p.pistas.iter().filter_map(|t| match &t.instrumento {
        Some(Instrumento::Muestra { ruta, .. }) => Some(ruta.clone()),
        _ => None,
    }));
    v.into_iter().collect()
}

fn cambiar(p: &mut Proyecto, de: &Path, a: &Path) {
    p.video.iter_mut().filter(|c| c.ruta == de).for_each(|c| c.ruta = a.into());
    for t in &mut p.pistas {
        t.clips.iter_mut().filter(|c| c.ruta == de).for_each(|c| c.ruta = a.into());
        if let Some(Instrumento::Muestra { ruta, .. }) = &mut t.instrumento {
            if ruta == de {
                *ruta = a.into();
            }
        }
    }
}

fn bloque(w: &mut impl Write, tag: &[u8; 4], d: &[u8]) -> std::io::Result<()> {
    w.write_all(tag)?;
    w.write_all(&(d.len() as u64).to_le_bytes())?;
    w.write_all(&crc(d).to_le_bytes())?;
    w.write_all(d)
}

/// `mini`: ancho, alto y RGBA. `incluir`: mete también los medios.
pub fn guardar(p: &Proyecto, ruta: &Path, mini: Option<(u16, u16, &[u8])>, incluir: bool) -> Result<(), String> {
    let tmp = ruta.with_extension("parcial");
    let r = (|| -> Result<(), String> {
        let e = |e: std::io::Error| e.to_string();
        let mut w = BufWriter::new(std::fs::File::create(&tmp).map_err(e)?);
        w.write_all(MAGIA).map_err(e)?;
        w.write_all(&VERSION.to_le_bytes()).map_err(e)?;
        let m = rmp_serde::to_vec_named(p).map_err(|e| e.to_string())?;
        bloque(&mut w, b"PROY", &compress_to_vec(&m, 6)).map_err(e)?;
        if let Some((an, al, q)) = mini {
            let mut d = an.to_le_bytes().to_vec();
            d.extend(al.to_le_bytes());
            d.extend(compress_to_vec(q, 6));
            bloque(&mut w, b"MINI", &d).map_err(e)?;
        }
        if incluir {
            for f in medios(p) {
                let mut r = BufReader::new(std::fs::File::open(&f).map_err(|x| format!("{}: {x}", f.display()))?);
                let n = f.to_string_lossy().into_owned().into_bytes();
                let largo = 4 + n.len() as u64 + r.get_ref().metadata().map_err(e)?.len();
                w.write_all(b"MEDI").map_err(e)?;
                w.write_all(&largo.to_le_bytes()).map_err(e)?;
                // La suma se escribe al final, cuando ya se leyó todo.
                let donde = w.stream_position().map_err(e)?;
                w.write_all(&[0; 4]).map_err(e)?;
                let mut c = Crc::nuevo();
                let mut pon = |w: &mut BufWriter<std::fs::File>, b: &[u8]| {
                    c.meter(b);
                    w.write_all(b)
                };
                pon(&mut w, &(n.len() as u32).to_le_bytes()).map_err(e)?;
                pon(&mut w, &n).map_err(e)?;
                let mut b = vec![0u8; 1 << 20];
                let mut total = 4 + n.len() as u64;
                loop {
                    let k = r.read(&mut b).map_err(e)?;
                    if k == 0 {
                        break;
                    }
                    total += k as u64;
                    pon(&mut w, &b[..k]).map_err(e)?;
                }
                if total != largo {
                    return Err(crate::idioma::tf("{} cambió mientras se guardaba", &[&(f.display())]));
                }
                let fin = w.stream_position().map_err(e)?;
                w.seek(SeekFrom::Start(donde)).map_err(e)?;
                w.write_all(&c.fin().to_le_bytes()).map_err(e)?;
                w.seek(SeekFrom::Start(fin)).map_err(e)?;
            }
        }
        w.flush().map_err(e)?;
        w.get_ref().sync_all().map_err(e)
    })();
    match r {
        Ok(()) => std::fs::rename(&tmp, ruta).map_err(|e| e.to_string()),
        Err(x) => {
            let _ = std::fs::remove_file(&tmp);
            Err(x)
        }
    }
}

/// Lee solo la miniatura de un proyecto: ancho, alto y RGBA.
pub fn miniatura(ruta: &Path) -> Option<(usize, usize, Vec<u8>)> {
    let mut f = BufReader::new(std::fs::File::open(ruta).ok()?);
    let mut cab = [0u8; 8];
    f.read_exact(&mut cab).ok()?;
    if &cab[..6] != MAGIA {
        return None;
    }
    loop {
        let mut h = [0u8; 16];
        f.read_exact(&mut h).ok()?;
        let largo = u64::from_le_bytes(h[4..12].try_into().unwrap());
        if &h[..4] != b"MINI" {
            f.seek(SeekFrom::Current(i64::try_from(largo).ok()?)).ok()?;
            continue;
        }
        if largo < 4 || largo > TOPE as u64 {
            return None;
        }
        let mut d = vec![0u8; largo as usize];
        f.read_exact(&mut d).ok()?;
        if crc(&d) != u32::from_le_bytes(h[12..16].try_into().unwrap()) {
            return None;
        }
        let (an, al) = (u16::from_le_bytes([d[0], d[1]]) as usize, u16::from_le_bytes([d[2], d[3]]) as usize);
        let q = decompress_to_vec_with_limit(&d[4..], TOPE).ok()?;
        return (q.len() == an * al * 4).then_some((an, al, q));
    }
}

/// Abre un proyecto. Los medios incluidos que ya no existan en su lugar se sacan a `carpeta`.
pub fn abrir(ruta: &Path, carpeta: &Path) -> Result<Proyecto, String> {
    let e = |e: std::io::Error| e.to_string();
    let mut f = BufReader::new(std::fs::File::open(ruta).map_err(e)?);
    let mut cab = [0u8; 8];
    let leidos = f.read(&mut cab).map_err(e)?;
    if leidos < 8 || &cab[..6] != MAGIA {
        // Los primeros proyectos eran texto JSON.
        let s = std::fs::read_to_string(ruta).map_err(|_| "el archivo no es un proyecto de Beethoven Craft")?;
        return serde_json::from_str(&s).map_err(|_| crate::idioma::t("el archivo no es un proyecto de Beethoven Craft").into());
    }
    if u16::from_le_bytes([cab[6], cab[7]]) > VERSION {
        return Err(crate::idioma::t("lo guardó una versión más nueva de Beethoven Craft").into());
    }
    let mut p: Option<Proyecto> = None;
    let mut sacados: Vec<(PathBuf, PathBuf)> = vec![];
    let total = f.get_ref().metadata().map_err(e)?.len();
    loop {
        let mut h = [0u8; 16];
        match f.read_exact(&mut h) {
            Ok(()) => {}
            Err(x) if x.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(x) => return Err(x.to_string()),
        }
        let largo = u64::from_le_bytes(h[4..12].try_into().unwrap());
        let suma = u32::from_le_bytes(h[12..16].try_into().unwrap());
        if f.stream_position().map_err(e)?.checked_add(largo).is_none_or(|x| x > total) {
            return Err(crate::idioma::t("el archivo está incompleto").into());
        }
        match &h[..4] {
            b"PROY" => {
                let mut d = vec![0u8; largo as usize];
                f.read_exact(&mut d).map_err(e)?;
                if crc(&d) != suma {
                    return Err(crate::idioma::t("el archivo está dañado").into());
                }
                let m = decompress_to_vec_with_limit(&d, TOPE).map_err(|_| "el archivo está dañado")?;
                p = Some(rmp_serde::from_slice(&m).map_err(|_| "el archivo está dañado")?);
            }
            b"MEDI" => {
                let mut n = [0u8; 4];
                f.read_exact(&mut n).map_err(e)?;
                let k = u32::from_le_bytes(n) as u64;
                if k + 4 > largo || k > 4096 {
                    return Err(crate::idioma::t("el archivo está dañado").into());
                }
                let mut nombre = vec![0u8; k as usize];
                f.read_exact(&mut nombre).map_err(e)?;
                let orig = PathBuf::from(String::from_utf8_lossy(&nombre).into_owned());
                let resto = largo - 4 - k;
                if orig.exists() {
                    f.seek(SeekFrom::Current(resto as i64)).map_err(e)?;
                    continue;
                }
                // Solo el nombre: nunca se escribe fuera de la carpeta, diga lo que diga el archivo.
                let base = orig.file_name().map(|x| x.to_os_string()).unwrap_or("medio".into());
                std::fs::create_dir_all(carpeta).map_err(e)?;
                let mut d = carpeta.join(&base);
                let mut v = 2;
                while d.exists() && std::fs::metadata(&d).map(|m| m.len()).unwrap_or(0) != resto {
                    d = carpeta.join(format!("{v}-{}", base.to_string_lossy()));
                    v += 1;
                }
                let mut c = Crc::nuevo();
                c.meter(&n);
                c.meter(&nombre);
                let tmp = d.with_extension("parcial");
                let mut w = BufWriter::new(std::fs::File::create(&tmp).map_err(e)?);
                let mut falta = resto;
                let mut b = vec![0u8; 1 << 20];
                while falta > 0 {
                    let k = (falta as usize).min(b.len());
                    f.read_exact(&mut b[..k]).map_err(e)?;
                    c.meter(&b[..k]);
                    w.write_all(&b[..k]).map_err(e)?;
                    falta -= k as u64;
                }
                w.flush().map_err(e)?;
                drop(w);
                if c.fin() != suma {
                    let _ = std::fs::remove_file(&tmp);
                    return Err(crate::idioma::tf("{} está dañado dentro del proyecto", &[&(base.to_string_lossy())]));
                }
                std::fs::rename(&tmp, &d).map_err(e)?;
                sacados.push((orig, d));
            }
            // Bloques que esta versión no conoce se saltan.
            _ => {
                f.seek(SeekFrom::Current(largo as i64)).map_err(e)?;
            }
        }
    }
    let mut p = p.ok_or("el archivo no trae el proyecto")?;
    for (de, a) in sacados {
        cambiar(&mut p, &de, &a);
    }
    Ok(p)
}
