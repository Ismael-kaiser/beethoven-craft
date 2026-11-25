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
    "instrumentos": "muestra", "tresondas", "frecuencias", "muralla", "acido", "retumbo", "pulsada", "barras", "muestreador", "pads", "cortador", "monoanalogico", "arpegiador", "acordes";
    "herramientas": "acorde", "arpegio", "cuadrar", "humano", "reves";
};
const EFECTOS: &[(&str, &str)] = en_carpetas! {
    "efectos": "vozencaja", "vozgruesa", "ecualizador", "filtro", "aire", "gravesextra", "compresor", "limitador", "compuerta", "multibanda", "deseseo", "transitorios", "eco", "pingpong", "reverb", "coro", "flanger", "faser", "vibrato", "tremolo", "autopan", "ancho", "fuego", "cinta", "trituradora", "lofi", "anillo",
    "pasos", "empujon", "filtrovivo", "pedalera", "cambiatono", "visor",
    "balance", "distorsion", "recortador", "matriz", "ducker", "vocoder", "filtropatron", "convolucion", "ctlnivel", "ctllfo", "ctlformula",
    "escalera", "afinador", "quitaruido", "congelar"
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

/// Los valores que un guion deja ver con `mide`, del hilo de audio a la ventana.
pub type Medidas = [std::sync::atomic::AtomicU32; crate::lua::MAX_MEDIDAS];

pub struct Biblioteca {
    pub guiones: Vec<Guion>,
    pub avisos: Vec<String>,
    pub sr: f32,
    pub carpeta: Option<PathBuf>,
    pub rack: crate::plugin::Rack,
    muestras: HashMap<(String, usize), Option<Arc<Muestra>>>,
    huellas: HashMap<(String, usize, u32, bool), Arc<Muestra>>,
    armadas: HashMap<String, Option<Arc<Muestra>>>,
    /// Los guiones en vivo, por su lugar: siguen siendo los mismos aunque se mueva una perilla.
    vivos: HashMap<String, (Arc<Receta>, BTreeMap<String, String>, Arc<crate::vivo::Vivo>)>,
    /// Cómo estaba la carpeta de plugins la última vez que se miró.
    pub(crate) visto: crate::instalar::Vista,
    /// Lo que mide cada efecto puesto, por pista y ranura.
    pub medidas: HashMap<(usize, usize), Arc<Medidas>>,
}

impl Biblioteca {
    /// `carpeta`: de dónde se leen (y dónde se guardan) los guiones del usuario.
    fn vacia(sr: f32, carpeta: Option<PathBuf>) -> Biblioteca {
        Biblioteca {
            guiones: vec![],
            avisos: vec![],
            sr,
            carpeta,
            rack: Default::default(),
            muestras: HashMap::new(),
            huellas: HashMap::new(),
            armadas: HashMap::new(),
            vivos: HashMap::new(),
            visto: Default::default(),
            medidas: HashMap::new(),
        }
    }

    pub fn nueva(sr: f32, carpeta: Option<PathBuf>) -> Biblioteca {
        let mut b = Biblioteca::vacia(sr, carpeta);
        for (clave, fuente) in FABRICA {
            b.poner(clave, fuente, false, Archivos::nuevos(), None).expect("los guiones de fábrica sirven");
        }
        for (clave, fuente) in EFECTOS {
            b.poner(clave, fuente, false, extras(clave), None).expect("los efectos de fábrica sirven");
        }
        b.leer_carpeta();
        b
    }

    /// Solo lo que hace falta para uno: así un plugin en otro programa abre al instante.
    pub fn solo(sr: f32, carpeta: Option<PathBuf>, clave: &str) -> Option<Biblioteca> {
        let mut b = Biblioteca::vacia(sr, carpeta);
        for (k, fuente) in FABRICA.iter().filter(|x| x.0 == clave || x.0 == "muestra").chain(EFECTOS.iter().filter(|x| x.0 == clave)) {
            b.poner(k, fuente, false, extras(k), None).ok()?;
        }
        if let Some(d) = b.carpeta.clone() {
            for p in [d.join(clave), d.join(format!("{clave}.lua"))].into_iter().filter(|p| p.exists()) {
                let _ = b.leer_uno(&p);
            }
        }
        b.ver(clave)?;
        Some(b)
    }

    /// Lee los guiones del usuario: cada uno en su carpeta (con sus imágenes y otros `.lua`) o, como antes, un `.lua`
    /// suelto. Los que fallan quedan en `avisos`.
    pub fn leer_carpeta(&mut self) {
        let Some(dir) = self.carpeta.clone() else { return };
        let mut v: Vec<PathBuf> = std::fs::read_dir(&dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
        v.sort();
        for p in v.into_iter().filter(|p| p.is_dir() || p.extension().is_some_and(|e| e == "lua")) {
            let n = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if n.starts_with('.') || n == crate::instalar::QUITADOS {
                continue;
            }
            if let Err(e) = self.leer_uno(&p) {
                self.avisos.push(format!("{}: {e}", p.file_stem().unwrap_or_default().to_string_lossy()));
            }
        }
        self.mirado();
    }

    /// Carga un plugin del disco. Si está mal, el que ya estuviera se queda como estaba.
    pub(crate) fn leer_uno(&mut self, p: &std::path::Path) -> Result<(), String> {
        let clave = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
        if p.is_dir() {
            let (f, a) = crate::paquete::leer(p)?;
            return self.poner(&clave, &f, true, a, Some(p.to_path_buf()));
        }
        let f = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
        self.poner(&clave, &f, true, Archivos::nuevos(), None)
    }

    pub(crate) fn mirado(&mut self) {
        self.visto = self.carpeta.as_ref().map(|d| crate::instalar::mirar(d)).unwrap_or_default();
    }

    /// Lo quita de la lista; si tapaba a uno de fábrica, ese regresa.
    pub(crate) fn olvidar(&mut self, clave: &str) {
        self.guiones.retain(|g| !(g.propio && g.clave == clave));
        for (k, fuente) in FABRICA.iter().chain(EFECTOS).filter(|x| x.0 == clave) {
            let _ = self.poner(k, fuente, false, extras(k), None);
        }
    }

    fn poner(&mut self, clave: &str, fuente: &str, propio: bool, archivos: Archivos, carpeta: Option<PathBuf>) -> Result<(), String> {
        let (ficha, receta) = armar_con(fuente, self.sr, &archivos)?;
        if let Some(l) = ficha.logo.as_ref().filter(|l| archivos.imagen(l).is_none()) {
            return Err(crate::idioma::tf("el logo «{}» no está en la carpeta", &[l]));
        }
        if let Some(n) = ficha.audios().into_iter().find(|n| !archivos.audios.contains_key(n)) {
            return Err(crate::idioma::tf("falta el audio «{}» en la carpeta", &[&n]));
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
                .map(|g| Archivos {
                    id: Archivos::nuevos().id,
                    lua: g.archivos.lua.clone(),
                    imagenes: g.archivos.imagenes.clone(),
                    audios: g.archivos.audios.clone(),
                })
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
        self.mirado();
        Ok(())
    }

    /// La carpeta de un guion del usuario (se crea si no está): ahí se ponen sus imágenes.
    pub fn carpeta_de(&self, clave: &str) -> Option<PathBuf> {
        let d = self.ver(clave).and_then(|g| g.carpeta.clone()).or_else(|| self.carpeta.as_ref().map(|c| c.join(clave)))?;
        std::fs::create_dir_all(&d).ok()?;
        Some(d)
    }

    /// La clave que le toca a un nombre, esté libre o no.
    pub fn clave_de(&self, nombre: &str) -> String {
        let base: String = nombre.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        if base.trim_matches('_').is_empty() { "mio".to_string() } else { base }
    }

    /// Una clave libre a partir de un nombre. Tampoco choca con las de los instrumentos grabados.
    pub fn clave_libre(&self, nombre: &str) -> String {
        let base = self.clave_de(nombre);
        (1..)
            .map(|i| if i == 1 { base.clone() } else { format!("{base}_{i}") })
            .find(|c| self.ver(c).is_none() && crate::banco::de(c).is_none())
            .unwrap()
    }

    pub fn ver(&self, clave: &str) -> Option<&Guion> {
        self.guiones.iter().find(|g| g.clave == clave)
    }

    pub fn de_tipo(&self, t: Tipo) -> impl Iterator<Item = &Guion> {
        self.guiones.iter().filter(move |g| g.ficha.tipo == t)
    }

    /// Agrupados para mostrar.
    /// Los guiones de ese tipo por grupo. El muestreador no se ofrece: toca las grabaciones del `banco`.
    pub fn grupos(&self, t: Tipo) -> BTreeMap<&str, Vec<&Guion>> {
        let mut m: BTreeMap<&str, Vec<&Guion>> = BTreeMap::new();
        for g in self.de_tipo(t).filter(|g| !g.ficha.oculto) {
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

    /// La huella para un efecto de convolución: la de su archivo o, si no tiene, la del lugar que diga su control
    /// «espacio». También mira «largo» y «reves».
    pub fn huella(&mut self, e: &crate::modelo::EfectoPuesto) -> Option<Arc<Muestra>> {
        use crate::impulso::{Impulso, LUGARES, recortar};
        let ficha = &self.ver(&e.efecto)?.ficha;
        let lee = |k: &str, v: f32| e.controles.get(k).copied().or(ficha.controles.iter().find(|c| c.clave == k).map(|c| c.valor)).unwrap_or(v);
        let (cual, reves) = ((lee("espacio", 0.0).round().max(0.0) as usize).min(LUGARES - 1), lee("reves", 0.0) > 0.5);
        // En pasos de 2 %: mover la perilla no llena la memoria.
        let largo = (lee("largo", 1.0) * 50.0).round().clamp(1.0, 50.0) as u32;
        // Su audio, o el que traiga el plugin, o uno de los lugares de fábrica.
        let g = self.ver(&e.efecto)?;
        let suya = ficha.huella.as_ref().and_then(|n| g.archivos.audios.get(n)).map(|p| p.to_string_lossy().to_string());
        let ruta = e.muestra.clone().or(suya).unwrap_or_default();
        let clave = (ruta.clone(), if ruta.is_empty() { cual } else { 0 }, largo, reves);
        if let Some(m) = self.huellas.get(&clave) {
            return Some(m.clone());
        }
        let k = largo as f32 / 50.0;
        let i = if ruta.is_empty() {
            Impulso::de_fabrica(cual, self.sr, k, reves)
        } else {
            match crate::decodificar::leer(std::path::Path::new(&ruta)) {
                Ok(w) => {
                    let x: Vec<[f32; 2]> = w.izq.iter().zip(&w.der).take((w.sr * crate::impulso::MAX_SEG) as usize).map(|(a, b)| [*a, *b]).collect();
                    let x = crate::editar::remuestrear(&x, (x.len() as f64 * (self.sr / w.sr) as f64) as usize);
                    Impulso::nuevo(recortar([x.iter().map(|v| v[0]).collect(), x.iter().map(|v| v[1]).collect()], k, reves), self.sr)
                }
                Err(x) => {
                    self.avisos.push(format!("{ruta}: {x}"));
                    Impulso::de_fabrica(cual, self.sr, k, reves)
                }
            }
        };
        if self.huellas.len() >= 24 {
            self.huellas.clear();
        }
        let m = Arc::new(Muestra { impulso: Some(Box::new(i)), ..Default::default() });
        self.huellas.insert(clave, m.clone());
        Some(m)
    }

    /// Los audios de los pads, cada uno en su tecla.
    pub fn pads(&mut self, rutas: &[String]) -> Option<Arc<Muestra>> {
        let clave = format!("pads:{}", rutas.join("\n"));
        if let Some(m) = self.armadas.get(&clave) {
            return m.clone();
        }
        let mut m = Muestra::default();
        for (i, r) in rutas.iter().enumerate().take(crate::motor::PADS).filter(|(_, r)| !r.is_empty()) {
            if let Some(x) = self.muestra(r, 0) {
                m.agregar_en(&x, crate::motor::PRIMER_PAD + i as u8);
            }
        }
        let m = Some(Arc::new(m));
        self.guardar_armada(clave, m.clone());
        m
    }

    /// El audio cortado en partes, una por tecla. Iguales, o donde hay golpes.
    pub fn cortes(&mut self, ruta: &str, v: &BTreeMap<String, f32>) -> Option<Arc<Muestra>> {
        let lee = |k: &str, x: f32| v.get(k).copied().unwrap_or(x);
        let (partes, golpes, fino) =
            (lee("partes", 16.0).round().clamp(2.0, 32.0) as usize, lee("donde", 1.0) > 0.5, (lee("fino", 0.5) * 20.0).round() / 20.0);
        let clave = format!("cortes:{partes}:{golpes}:{fino}:{ruta}");
        if let Some(m) = self.armadas.get(&clave) {
            return m.clone();
        }
        let m = self.muestra(ruta, 0).map(|x| Arc::new(x.cortada(partes, golpes, fino)));
        self.guardar_armada(clave, m.clone());
        m
    }

    /// Los audios que trae un instrumento en su carpeta.
    pub fn propias(&mut self, clave: &str) -> Option<Arc<Muestra>> {
        let g = self.ver(clave)?;
        let crate::lua::Muestras::Propias(v) = g.ficha.muestras.clone() else { return None };
        let a = g.archivos.clone();
        let nombre = format!("propias:{}:{clave}", a.id);
        if let Some(m) = self.armadas.get(&nombre) {
            return m.clone();
        }
        let mut m = Muestra::default();
        for x in &v {
            let Some(s) = a.audios.get(&x.archivo).and_then(|p| self.muestra(&p.to_string_lossy(), 0)) else { continue };
            m.agregar_entre(&s, x.nota, x.teclas, x.fuerzas);
        }
        let m = Some(Arc::new(m));
        self.guardar_armada(nombre, m.clone());
        m
    }

    /// El guion en vivo de ese lugar; se arma la primera vez, o si cambió.
    pub fn vivo(
        &mut self,
        lugar: &str,
        clave: &str,
        datos: &BTreeMap<String, String>,
        medidas: Option<Arc<Medidas>>,
    ) -> Option<Arc<crate::vivo::Vivo>> {
        let g = self.ver(clave)?;
        let r = g.receta.clone().filter(|r| r.vivo)?;
        if let Some((a, b, v)) = self.vivos.get(lugar)
            && Arc::ptr_eq(a, &r)
            && b == datos
        {
            return Some(v.clone());
        }
        let claves: Vec<String> = r.controles.iter().map(|c| c.clave.clone()).collect();
        match crate::vivo::Vivo::nuevo(&g.fuente, &g.archivos, self.sr, &claves, datos, (&r.medidas_vivo, medidas, r.medidas.len())) {
            Ok(v) => {
                let v = Arc::new(v);
                self.vivos.insert(lugar.into(), (r, datos.clone(), v.clone()));
                Some(v)
            }
            Err(e) => {
                self.avisos.push(crate::vivo::mensaje(g.ficha.nombre.ver(), &e).to_string());
                None
            }
        }
    }

    /// Los guiones en vivo que hay, con el nombre de su lugar.
    pub fn vivos(&self) -> impl Iterator<Item = (&str, &Arc<crate::vivo::Vivo>)> {
        self.vivos.iter().map(|(k, v)| (k.as_str(), &v.2))
    }

    pub fn soltar_vivos(&mut self) {
        self.vivos.clear();
    }

    fn guardar_armada(&mut self, clave: String, m: Option<Arc<Muestra>>) {
        if self.armadas.len() >= 16 {
            self.armadas.clear();
        }
        self.armadas.insert(clave, m);
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
