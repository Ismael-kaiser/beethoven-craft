// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Plugins VST3 y CLAP. Se cargan, se abren y se sueltan en el hilo de la ventana; `procesar` corre en el de audio.
use crate::idioma::t;
mod clap;
pub mod ventana;
mod vst3;

use crate::modelo::PluginPuesto;
use crate::receta::{BLOQUE, Buf};
use crossbeam_channel::{Receiver, Sender, bounded};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering::*};

/// Eventos por bloque.
pub const EVENTOS: usize = 128;

#[derive(Clone, Copy, Debug)]
pub enum Evento {
    Nota { tono: u8, fuerza: f32 },
    Suelta { tono: u8 },
    Control { id: u32, valor: f64 },
}

#[derive(Clone, Copy, Default)]
pub struct Tiempo {
    pub bpm: f64,
    /// En negras.
    pub pos: f64,
    pub tocando: bool,
    pub sr: f64,
}

#[derive(Clone, Debug)]
pub struct ControlP {
    pub id: u32,
    pub nombre: String,
    pub min: f64,
    pub max: f64,
    pub entero: bool,
}

/// Lo que se puede elegir de un archivo.
#[derive(Clone, Debug, PartialEq)]
pub struct Oferta {
    pub ruta: String,
    pub id: String,
    pub nombre: String,
    pub instrumento: bool,
}

/// Las entradas y salidas de audio del plugin, ya con su memoria.
struct Puertos {
    datos: Vec<Vec<Vec<f32>>>,
    punteros: Vec<Vec<*mut f32>>,
}

impl Puertos {
    fn nuevos(canales: &[usize]) -> Puertos {
        let mut datos: Vec<Vec<Vec<f32>>> = canales.iter().map(|c| vec![vec![0.0; BLOQUE]; *c]).collect();
        let punteros = datos.iter_mut().map(|p| p.iter_mut().map(|c| c.as_mut_ptr()).collect()).collect();
        Puertos { datos, punteros }
    }

    fn llenar(&mut self, x: &[Buf; 2], n: usize) {
        for (i, p) in self.datos.iter_mut().enumerate() {
            let uno = p.len() == 1;
            for (j, c) in p.iter_mut().enumerate() {
                match (i, j) {
                    (0, 0) if uno => (0..n).for_each(|k| c[k] = 0.5 * (x[0][k] + x[1][k])),
                    (0, 0 | 1) => c[..n].copy_from_slice(&x[j][..n]),
                    _ => c[..n].fill(0.0),
                }
            }
        }
    }

    fn vaciar(&self, x: &mut [Buf; 2], n: usize) {
        match self.datos.first() {
            Some(p) if !p.is_empty() => {
                x[0][..n].copy_from_slice(&p[0][..n]);
                x[1][..n].copy_from_slice(&p[if p.len() > 1 { 1 } else { 0 }][..n]);
            }
            _ => x.iter_mut().for_each(|c| c[..n].fill(0.0)),
        }
    }
}

enum Fondo {
    Clap(Box<clap::Clap>),
    Vst3(Box<vst3::Vst3>),
}

pub struct Plugin {
    pub nombre: String,
    pub controles: Vec<ControlP>,
    fondo: UnsafeCell<Fondo>,
    cambios: Sender<(u32, f64)>,
    llegan: Receiver<(u32, f64)>,
    ocupado: AtomicBool,
}

// El lado de audio solo se toca dentro de `procesar`, que se cuida con `ocupado`; lo demás es del hilo de la ventana,
// como piden las dos normas.
unsafe impl Send for Plugin {}
unsafe impl Sync for Plugin {}

impl Plugin {
    /// Solo desde el hilo de la ventana.
    pub fn cargar(ruta: &str, id: &str, sr: f64, estado: &[u8]) -> Result<Plugin, String> {
        let (cambios, llegan) = bounded(EVENTOS * 4);
        let p = Path::new(ruta);
        let (fondo, controles, nombre) = if extension(p) == "clap" {
            let mut c = clap::Clap::cargar(p, id, sr)?;
            if !estado.is_empty() {
                c.poner_estado(estado);
            }
            (c.controles(), c.nombre(), Fondo::Clap(c))
        } else {
            let mut v = vst3::Vst3::cargar(p, id, sr, cambios.clone())?;
            if !estado.is_empty() {
                v.poner_estado(estado);
            }
            (v.controles(), v.nombre.clone(), Fondo::Vst3(v))
        }
        .acomodar();
        Ok(Plugin { nombre, controles, fondo: UnsafeCell::new(fondo), cambios, llegan, ocupado: AtomicBool::new(false) })
    }

    #[allow(clippy::mut_from_ref)]
    fn f(&self) -> &mut Fondo {
        unsafe { &mut *self.fondo.get() }
    }

    /// Hilo de audio. `ev` es lo que pasa al inicio del bloque.
    pub fn procesar(&self, ev: &[Evento], t: &Tiempo, entra: &[Buf; 2], sale: &mut [Buf; 2], n: usize) {
        if self.ocupado.swap(true, Acquire) {
            return sale.iter_mut().for_each(|c| c[..n].fill(0.0));
        }
        let mut todo = [Evento::Suelta { tono: 0 }; EVENTOS];
        let mut k = ev.len().min(EVENTOS);
        todo[..k].copy_from_slice(&ev[..k]);
        while k < EVENTOS {
            let Ok((id, valor)) = self.llegan.try_recv() else { break };
            todo[k] = Evento::Control { id, valor };
            k += 1;
        }
        match self.f() {
            Fondo::Clap(c) => c.procesar(&todo[..k], t, entra, sale, n),
            Fondo::Vst3(v) => v.procesar(&todo[..k], t, entra, sale, n),
        }
        self.ocupado.store(false, Release);
    }

    pub fn valor(&self, id: u32) -> f64 {
        match self.f() {
            Fondo::Clap(c) => c.valor(id),
            Fondo::Vst3(v) => v.valor(id),
        }
    }

    pub fn mover(&self, id: u32, valor: f64) {
        if let Fondo::Vst3(v) = self.f() {
            v.mover(id, valor);
        }
        let _ = self.cambios.try_send((id, valor));
    }

    /// El valor como lo escribe el plugin («-6.0 dB»).
    pub fn texto(&self, id: u32, valor: f64) -> String {
        match self.f() {
            Fondo::Clap(c) => c.texto(id, valor),
            Fondo::Vst3(v) => v.texto(id, valor),
        }
    }

    pub fn estado(&self) -> Vec<u8> {
        match self.f() {
            Fondo::Clap(c) => c.estado(),
            Fondo::Vst3(v) => v.estado(),
        }
    }

    /// Lo que el plugin pidió hacer en el hilo de la ventana. Se llama en cada cuadro.
    pub fn atender(&self) {
        if let Fondo::Clap(c) = self.f() {
            c.atender();
        }
    }

    fn pegar(&self, vista: *mut std::ffi::c_void, ventana: *mut std::ffi::c_void) -> Option<(u32, u32)> {
        match self.f() {
            Fondo::Clap(c) => c.pegar(vista),
            Fondo::Vst3(v) => v.pegar(vista, ventana),
        }
    }

    fn despegar(&self) {
        match self.f() {
            Fondo::Clap(c) => c.despegar(),
            Fondo::Vst3(v) => v.despegar(),
        }
    }

    /// El tamaño que pidió el plugin, si pidió uno.
    fn tamano_pedido(&self) -> Option<(u32, u32)> {
        match self.f() {
            Fondo::Clap(c) => c.tamano_pedido(),
            Fondo::Vst3(_) => None,
        }
    }
}

trait Acomodar {
    fn acomodar(self) -> (Fondo, Vec<ControlP>, String);
}
impl Acomodar for (Vec<ControlP>, String, Fondo) {
    fn acomodar(self) -> (Fondo, Vec<ControlP>, String) {
        (self.2, self.0, self.1)
    }
}

fn extension(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

pub fn es_plugin(p: &Path) -> bool {
    matches!(extension(p).as_str(), "vst3" | "clap")
}

/// Carpetas donde cada sistema guarda los plugins.
pub fn carpetas() -> Vec<PathBuf> {
    let casa = |s: &str| std::env::var_os(if cfg!(windows) { "LOCALAPPDATA" } else { "HOME" }).map(|h| PathBuf::from(h).join(s));
    let mut v: Vec<Option<PathBuf>> = vec![];
    if cfg!(target_os = "macos") {
        for k in ["VST3", "CLAP"] {
            v.push(Some(PathBuf::from("/Library/Audio/Plug-Ins").join(k)));
            v.push(casa(&format!("Library/Audio/Plug-Ins/{k}")));
        }
    } else if cfg!(windows) {
        for k in ["VST3", "CLAP"] {
            v.push(std::env::var_os("COMMONPROGRAMFILES").map(|h| PathBuf::from(h).join(k)));
            v.push(casa(&format!("Programs/Common/{k}")));
        }
    } else {
        for k in ["vst3", "clap"] {
            v.push(Some(PathBuf::from("/usr/lib").join(k)));
            v.push(casa(&format!(".{k}")));
        }
    }
    v.into_iter().flatten().collect()
}

/// Los archivos que hay, sin abrirlos. CLAP primero si están los dos.
pub fn buscar() -> Vec<PathBuf> {
    fn ver(d: &Path, hondo: u8, v: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
            let p = e.path();
            if es_plugin(&p) {
                v.push(p);
            } else if hondo > 0 && p.is_dir() {
                ver(&p, hondo - 1, v);
            }
        }
    }
    let mut v = vec![];
    carpetas().iter().for_each(|d| ver(d, 2, &mut v));
    v.sort_by_key(|p| (p.file_stem().map(|s| s.to_string_lossy().to_lowercase()), extension(p)));
    v
}

/// Qué plugins trae un archivo. Lo carga.
pub fn ofertas(ruta: &Path) -> Result<Vec<Oferta>, String> {
    if extension(ruta) == "clap" { clap::ofertas(ruta) } else { vst3::ofertas(ruta) }
}

/// El archivo ejecutable dentro del paquete.
fn binario(p: &Path) -> Result<PathBuf, String> {
    if p.is_file() {
        return Ok(p.into());
    }
    let sub = if cfg!(target_os = "macos") {
        "MacOS".to_string()
    } else if cfg!(windows) {
        format!("{}-win", std::env::consts::ARCH)
    } else {
        format!("{}-linux", std::env::consts::ARCH)
    };
    let d = p.join("Contents").join(sub);
    let nombre = p.file_stem().unwrap_or_default();
    let mut v: Vec<PathBuf> = std::fs::read_dir(&d).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
    v.sort_by_key(|x| x.file_stem() != Some(nombre));
    v.into_iter().next().ok_or_else(|| t("El plugin no trae versión para este sistema").into())
}

/// Los plugins que están vivos, por su `uid`.
#[derive(Default)]
pub struct Rack {
    vivos: HashMap<u64, Result<Arc<Plugin>, String>>,
}

impl Rack {
    /// El que pide el proyecto; si no está, se carga.
    pub fn dar(&mut self, p: &PluginPuesto, sr: f32) -> Result<Arc<Plugin>, String> {
        self.vivos.entry(p.uid).or_insert_with(|| Plugin::cargar(&p.ruta, &p.id, sr as f64, &de_hex(&p.estado)).map(Arc::new)).clone()
    }

    pub fn ver(&self, uid: u64) -> Option<&Arc<Plugin>> {
        self.vivos.get(&uid)?.as_ref().ok()
    }

    pub fn todos(&self) -> impl Iterator<Item = &Arc<Plugin>> {
        self.vivos.values().flatten()
    }

    pub fn estado(&self, uid: u64) -> Option<String> {
        Some(hex(&self.ver(uid)?.estado()))
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn de_hex(s: &str) -> Vec<u8> {
    s.as_bytes().chunks_exact(2).filter_map(|c| u8::from_str_radix(std::str::from_utf8(c).ok()?, 16).ok()).collect()
}
