// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Los guiones de Lua disponibles (de fábrica y del usuario) y sus sonidos ya armados.
use crate::idioma::t;
use crate::lua::{Ficha, Tipo, armar_con};
use crate::muestrario::Muestra;
use crate::paquete::Archivos;
use crate::receta::Receta;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

macro_rules! fabrica {
    ($($dir:literal: $($n:literal),*;)*) => { &[$($(($n, include_str!(concat!("../", $dir, "/", $n, ".lua")))),*),*] };
}
/// Cada efecto vive en su carpeta: `efectos/eco/eco.lua`.
macro_rules! en_carpetas {
    ($dir:literal: $($n:literal),*) => { &[$(($n, include_str!(concat!("../", $dir, "/", $n, "/", $n, ".lua")))),*] };
}

const FABRICA: &[(&str, &str)] = fabrica! {
    "instrumentos": "muestra";
    "herramientas": "acorde", "arpegio", "cuadrar", "humano", "reves";
};
const EFECTOS: &[(&str, &str)] = en_carpetas! {
    "efectos": "vozencaja", "vozgruesa", "ecualizador", "filtro", "aire", "gravesextra", "compresor", "limitador", "compuerta", "multibanda", "deseseo", "transitorios", "eco", "pingpong", "reverb", "coro", "flanger", "faser", "vibrato", "tremolo", "autopan", "ancho", "fuego", "cinta", "trituradora", "lofi", "anillo"
};
/// Lo demás que traen las carpetas de fábrica: (efecto, archivo, contenido).
const EXTRAS: &[(&str, &str, &[u8])] = &[
    ("vozgruesa", "ayuda.lua", include_bytes!("../efectos/vozgruesa/ayuda.lua")),
    ("vozgruesa", "logo.png", include_bytes!("../efectos/vozgruesa/logo.png")),
    ("vozgruesa", "fondo.png", include_bytes!("../efectos/vozgruesa/fondo.png")),
];

/// Los archivos de fábrica de un efecto.
fn extras(clave: &str) -> Archivos {
    let mut a = Archivos::nuevos();
    for (c, n, b) in EXTRAS.iter().filter(|x| x.0 == clave) {
        let _ = c;
        match n.strip_suffix(".lua") {
            Some(m) => _ = a.lua.insert(m.to_string(), String::from_utf8_lossy(b).into_owned()),
            None => a.poner_imagen(n, b.to_vec()).expect("las imágenes de fábrica sirven"),
        }
    }
    a
}

pub struct Guion {
    pub clave: String,
    pub fuente: String,
    pub ficha: Ficha,
    pub receta: Option<Arc<Receta>>,
    pub propio: bool,
    /// Lo que trae su carpeta: otros `.lua` e imágenes.
    pub archivos: Arc<Archivos>,
    /// Su carpeta, si es del usuario y vive en una.
    pub carpeta: Option<PathBuf>,
}

pub struct Biblioteca {
    pub guiones: Vec<Guion>,
    pub avisos: Vec<String>,
    pub sr: f32,
    pub carpeta: Option<PathBuf>,
    pub rack: crate::plugin::Rack,
    muestras: HashMap<(String, usize), Option<Arc<Muestra>>>,
}

impl Biblioteca {
    /// `carpeta`: de dónde se leen (y dónde se guardan) los guiones del usuario.
    pub fn nueva(sr: f32, carpeta: Option<PathBuf>) -> Biblioteca {
        let mut b = Biblioteca { guiones: vec![], avisos: vec![], sr, carpeta, rack: Default::default(), muestras: HashMap::new() };
        for (clave, fuente) in FABRICA {
            b.poner(clave, fuente, false, Archivos::nuevos(), None).expect("los guiones de fábrica sirven");
        }
        for (clave, fuente) in EFECTOS {
            b.poner(clave, fuente, false, extras(clave), None).expect("los efectos de fábrica sirven");
        }
        b.leer_carpeta();
        b
    }

    /// Lee los guiones del usuario: cada uno en su carpeta (con sus imágenes y otros `.lua`) o, como antes, un `.lua`
    /// suelto. Los que fallan quedan en `avisos`.
    pub fn leer_carpeta(&mut self) {
        let Some(dir) = self.carpeta.clone() else { return };
        let mut v: Vec<PathBuf> = std::fs::read_dir(&dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
        v.sort();
        for p in v {
            let clave = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let r = if p.is_dir() {
                crate::paquete::leer(&p).and_then(|(f, a)| self.poner(&clave, &f, true, a, Some(p.clone())))
            } else if p.extension().is_some_and(|e| e == "lua") {
                std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|f| self.poner(&clave, &f, true, Archivos::nuevos(), None))
            } else {
                continue;
            };
            if let Err(e) = r {
                self.avisos.push(format!("{clave}: {e}"));
            }
        }
    }

    fn poner(&mut self, clave: &str, fuente: &str, propio: bool, archivos: Archivos, carpeta: Option<PathBuf>) -> Result<(), String> {
        let (ficha, receta) = armar_con(fuente, self.sr, &archivos)?;
        if let Some(l) = ficha.logo.as_ref().filter(|l| archivos.imagen(l).is_none()) {
            return Err(crate::idioma::tf("el logo «{}» no está en la carpeta", &[l]));
        }
        let archivos = Arc::new(archivos);
        // Que la interfaz, si trae, también esté bien escrita.
        crate::interfaz::cargar_con(fuente, archivos.clone())?;
        let g = Guion { clave: clave.into(), fuente: fuente.into(), ficha, receta: receta.map(Arc::new), propio, archivos, carpeta };
        match self.guiones.iter_mut().find(|x| x.clave == clave) {
            // Cambiar de tipo dejaría canales apuntando a un efecto.
            Some(x) if x.ficha.tipo != g.ficha.tipo => return Err(t("ya hay uno de otro tipo con ese nombre").into()),
            Some(x) => *x = g,
            None => self.guiones.push(g),
        }
        Ok(())
    }

    /// Guarda el guion del usuario en su carpeta (`Lua/<clave>/<clave>.lua`, junto a sus imágenes) y lo deja sonando.
    /// Si tiene errores no cambia nada. Uno que era un `.lua` suelto pasa a su carpeta.
    pub fn guardar(&mut self, clave: &str, fuente: &str) -> Result<(), String> {
        let previo = self.ver(clave).filter(|g| g.propio);
        let Some(d) = self.carpeta.clone() else {
            let a = previo
                .map(|g| Archivos { id: Archivos::nuevos().id, lua: g.archivos.lua.clone(), imagenes: g.archivos.imagenes.clone() })
                .unwrap_or_else(Archivos::nuevos);
            return self.poner(clave, fuente, true, a, None);
        };
        let carpeta = previo.and_then(|g| g.carpeta.clone()).unwrap_or_else(|| d.join(clave));
        // El principal se llama como la carpeta. Lo demás se vuelve a leer: así entran las imágenes que pusiste.
        let nombre = carpeta.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| clave.into());
        // Tu copia de uno de fábrica se lleva sus imágenes y sus otros `.lua`.
        if !carpeta.is_dir() {
            if let Some(g) = self.ver(clave) {
                let a = g.archivos.clone();
                std::fs::create_dir_all(&carpeta).map_err(|e| e.to_string())?;
                for (n, f) in &a.lua {
                    std::fs::write(carpeta.join(format!("{n}.lua")), f).map_err(|e| e.to_string())?;
                }
                for (n, b) in &a.imagenes {
                    let r = carpeta.join(n);
                    if let Some(p) = r.parent() {
                        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
                    }
                    std::fs::write(r, b.as_slice()).map_err(|e| e.to_string())?;
                }
            }
        }
        let archivos = crate::paquete::extras(&carpeta, &nombre)?;
        self.poner(clave, fuente, true, archivos, Some(carpeta.clone()))?;
        std::fs::create_dir_all(&carpeta).map_err(|e| e.to_string())?;
        std::fs::write(carpeta.join(format!("{nombre}.lua")), fuente).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(d.join(format!("{clave}.lua")));
        Ok(())
    }

    /// La carpeta de un guion del usuario (se crea si no está): ahí se ponen sus imágenes.
    pub fn carpeta_de(&self, clave: &str) -> Option<PathBuf> {
        let d = self.ver(clave).and_then(|g| g.carpeta.clone()).or_else(|| self.carpeta.as_ref().map(|c| c.join(clave)))?;
        std::fs::create_dir_all(&d).ok()?;
        Some(d)
    }

    /// Una clave libre a partir de un nombre.
    pub fn clave_libre(&self, nombre: &str) -> String {
        let base: String = nombre.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        let base = if base.trim_matches('_').is_empty() { "mio".to_string() } else { base };
        (1..).map(|i| if i == 1 { base.clone() } else { format!("{base}_{i}") }).find(|c| self.ver(c).is_none()).unwrap()
    }

    pub fn ver(&self, clave: &str) -> Option<&Guion> {
        self.guiones.iter().find(|g| g.clave == clave)
    }

    pub fn de_tipo(&self, t: Tipo) -> impl Iterator<Item = &Guion> {
        self.guiones.iter().filter(move |g| g.ficha.tipo == t)
    }

    /// Agrupados para mostrar.
    /// Los guiones de ese tipo por grupo. Los instrumentos de fábrica son grabaciones (`banco`): de Lua solo
    /// quedan los tuyos.
    pub fn grupos(&self, t: Tipo) -> BTreeMap<&str, Vec<&Guion>> {
        let mut m: BTreeMap<&str, Vec<&Guion>> = BTreeMap::new();
        for g in self.de_tipo(t).filter(|g| t != Tipo::Instrumento || g.propio) {
            m.entry(g.ficha.grupo.ver()).or_default().push(g);
        }
        m
    }

    /// Si esa clave es un instrumento grabado. Uno tuyo de Lua con el mismo nombre gana.
    pub fn del_banco(&self, clave: &str) -> Option<&'static crate::banco::Instrumento> {
        crate::banco::de(clave).filter(|_| !self.ver(clave).is_some_and(|g| g.propio))
    }

    /// El guion que hace sonar ese instrumento: el suyo, o el muestreador si es grabado.
    pub fn sonido(&self, clave: &str) -> Option<&Guion> {
        self.ver(if self.del_banco(clave).is_some() { "muestra" } else { clave })
    }

    /// Cómo se llama un instrumento, en el idioma de ahora.
    pub fn nombre(&self, clave: &str) -> String {
        match self.del_banco(clave) {
            Some(x) => t(x.nombre).to_string(),
            None => self.ver(clave).map_or(clave.to_string(), |g| g.ficha.nombre.ver().to_string()),
        }
    }

    /// La grabación de un instrumento de fábrica. Sin el banco junto al programa, nada.
    pub fn banco(&mut self, x: &crate::banco::Instrumento) -> Option<Arc<Muestra>> {
        let r = crate::banco::archivo()?.to_string_lossy().to_string();
        self.muestra(&r, crate::banco::indice(x.preset)?)
    }

    pub fn muestra(&mut self, ruta: &str, preset: usize) -> Option<Arc<Muestra>> {
        let clave = (ruta.to_string(), preset);
        if let Some(m) = self.muestras.get(&clave) {
            return m.clone();
        }
        let m = match crate::muestrario::abrir(std::path::Path::new(ruta), preset) {
            Ok(m) => Some(Arc::new(m)),
            Err(e) => {
                self.avisos.push(format!("{ruta}: {e}"));
                None
            }
        };
        self.muestras.insert(clave, m.clone());
        m
    }
}
