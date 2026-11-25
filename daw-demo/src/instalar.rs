// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Instalar y compartir plugins de Lua: una carpeta, un `.zip` (o `.bplugin`, que es lo mismo) o un `.lua` suelto. Antes
//! de entrar se revisa todo: nombres, tamaños, que arme, que suene sin reventar, cuánto cuesta y que su ventana dibuje.
use crate::biblioteca::Biblioteca;
use crate::idioma::{t, tf};
use crate::lua::{Ficha, Tipo};
use crate::medir;
use crate::paquete::{self, Archivos};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Topes de un paquete.
const MAX_ARCHIVOS: usize = 256;
const MAX_TOTAL: usize = 160 << 20;
/// Lo que puede venir dentro; lo demás se ignora.
const CONOCIDAS: &[&str] = &["lua", "png", "wav", "ogg", "flac", "mp3", "md", "txt"];
/// Más de esto del tiempo real y no se instala: cortaría el sonido.
const MAX_CPU: f32 = 70.0;

/// Cómo salió la revisión.
#[derive(Clone, Debug)]
pub struct Informe {
    /// Porcentaje de un procesador con una pista o una nota.
    pub cpu: f32,
    pub avisos: Vec<String>,
}

/// Los archivos de un plugin, con su ruta dentro de la carpeta.
pub struct Paquete {
    pub nombre: String,
    pub archivos: BTreeMap<String, Vec<u8>>,
}

fn roto() -> String {
    t("el paquete está dañado").into()
}

/// Un nombre de archivo que se queda dentro de su carpeta: sin rutas del sistema, sin «..», hasta una subcarpeta.
fn sana(ruta: &str) -> Option<String> {
    let r = ruta.replace('\\', "/");
    let partes: Vec<&str> = r.split('/').collect();
    let bien = |p: &str| {
        !p.is_empty()
            && p.len() <= 80
            && !p.starts_with('.')
            && !p.ends_with([' ', '.'])
            && p.chars().all(|c| c.is_alphanumeric() || " ._-()".contains(c))
    };
    (partes.len() <= 2 && partes.iter().all(|p| bien(p))).then_some(r)
}

/// Lee un zip sin escribir nada. Solo los métodos de siempre (guardado y deflate).
pub fn leer_zip(b: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let u16_ = |i: usize| b.get(i..i + 2).map(|x| u16::from_le_bytes([x[0], x[1]]) as usize).ok_or_else(roto);
    let u32_ = |i: usize| b.get(i..i + 4).map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as usize).ok_or_else(roto);
    // El índice está al final, quizá con un comentario después.
    let fin = (0..b.len().saturating_sub(21)).rev().take(66_000).find(|i| b[*i..].starts_with(b"PK\x05\x06")).ok_or_else(roto)?;
    let (cuantos, mut i) = (u16_(fin + 10)?, u32_(fin + 16)?);
    if cuantos > MAX_ARCHIVOS * 4 {
        return Err(t("el paquete trae demasiados archivos").into());
    }
    let mut v: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut total = 0usize;
    for _ in 0..cuantos {
        if b.get(i..i + 4) != Some(b"PK\x01\x02") {
            return Err(roto());
        }
        let (metodo, comprimido, largo) = (u16_(i + 10)?, u32_(i + 20)?, u32_(i + 24)?);
        let (n, extra, coment, lugar) = (u16_(i + 28)?, u16_(i + 30)?, u16_(i + 32)?, u32_(i + 42)?);
        let nombre = String::from_utf8_lossy(b.get(i + 46..i + 46 + n).ok_or_else(roto)?).to_string();
        i += 46 + n + extra + coment;
        if nombre.ends_with('/') || nombre.starts_with("__MACOSX") || nombre.split('/').any(|p| p.starts_with("._") || p == ".DS_Store") {
            continue;
        }
        total = total.saturating_add(largo);
        if total > MAX_TOTAL || v.len() >= MAX_ARCHIVOS {
            return Err(t("el paquete pesa demasiado").into());
        }
        if b.get(lugar..lugar + 4) != Some(b"PK\x03\x04") {
            return Err(roto());
        }
        let ini = lugar + 30 + u16_(lugar + 26)? + u16_(lugar + 28)?;
        let datos = b.get(ini..ini.checked_add(comprimido).ok_or_else(roto)?).ok_or_else(roto)?;
        let x = match metodo {
            0 => datos.to_vec(),
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(datos, largo).map_err(|_| roto())?,
            _ => return Err(roto()),
        };
        if x.len() != largo {
            return Err(roto());
        }
        v.insert(nombre, x);
    }
    Ok(v)
}

/// Un zip sin comprimir, que abre cualquier programa.
pub fn escribir_zip(v: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let crc = |x: &[u8]| {
        !x.iter().fold(!0u32, |mut c, b| {
            c ^= *b as u32;
            (0..8).for_each(|_| c = if c & 1 != 0 { c >> 1 ^ 0xEDB8_8320 } else { c >> 1 });
            c
        })
    };
    let (mut b, mut indice) = (vec![], vec![]);
    for (n, x) in v {
        let lugar = b.len() as u32;
        // Versión, marca de nombres en UTF-8, guardado sin comprimir, y la fecha en 1980.
        let mut cab = vec![];
        for w in [20u16, 0x800, 0, 0, 0x21] {
            cab.extend(w.to_le_bytes());
        }
        for w in [crc(x), x.len() as u32, x.len() as u32] {
            cab.extend(w.to_le_bytes());
        }
        cab.extend((n.len() as u16).to_le_bytes());
        cab.extend([0, 0]);
        b.extend(b"PK\x03\x04");
        b.extend(&cab);
        b.extend(n.as_bytes());
        b.extend(x);
        indice.extend(b"PK\x01\x02\x14\x00");
        indice.extend(&cab);
        indice.extend([0u8; 10]);
        indice.extend(lugar.to_le_bytes());
        indice.extend(n.as_bytes());
    }
    let lugar = b.len() as u32;
    b.extend(&indice);
    b.extend(b"PK\x05\x06\0\0\0\0");
    b.extend((v.len() as u16).to_le_bytes());
    b.extend((v.len() as u16).to_le_bytes());
    b.extend((indice.len() as u32).to_le_bytes());
    b.extend(lugar.to_le_bytes());
    b.extend([0, 0]);
    b
}

/// Si eso se puede instalar: una carpeta con guiones, un paquete o un guion.
pub fn es_plugin(r: &Path) -> bool {
    let ext = r.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if r.is_dir() {
        // Los VST3 y CLAP de Mac también son carpetas.
        return !matches!(ext.as_str(), "vst3" | "clap" | "app")
            && std::fs::read_dir(r).is_ok_and(|d| d.flatten().any(|e| e.path().extension().is_some_and(|x| x == "lua")));
    }
    matches!(ext.as_str(), "lua" | "zip" | "bplugin")
}

/// Junta los archivos del plugin, ya revisados sus nombres y tamaños.
pub fn abrir(r: &Path) -> Result<Paquete, String> {
    let nombre = r.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let mut v: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    if r.is_dir() {
        let mut total = 0;
        let mut pendientes = vec![(r.to_path_buf(), String::new())];
        while let Some((d, pre)) = pendientes.pop() {
            for e in std::fs::read_dir(&d).map_err(|e| e.to_string())?.flatten() {
                let (tipo, n) = (e.file_type().map_err(|e| e.to_string())?, format!("{pre}{}", e.file_name().to_string_lossy()));
                if tipo.is_dir() && pre.is_empty() && !n.starts_with('.') {
                    pendientes.push((e.path(), format!("{n}/")));
                } else if tipo.is_file() && conocida(&n) {
                    total += e.metadata().map_or(0, |m| m.len() as usize);
                    if total > MAX_TOTAL || v.len() >= MAX_ARCHIVOS {
                        return Err(t("el paquete pesa demasiado").into());
                    }
                    v.insert(n, std::fs::read(e.path()).map_err(|e| e.to_string())?);
                }
            }
        }
        return limpiar(r.file_name().unwrap_or_default().to_string_lossy().to_string(), v);
    }
    let b = std::fs::metadata(r).map_err(|e| e.to_string()).and_then(|m| {
        if m.len() as usize > MAX_TOTAL { Err(t("el paquete pesa demasiado").to_string()) } else { std::fs::read(r).map_err(|e| e.to_string()) }
    })?;
    if r.extension().is_some_and(|e| e.eq_ignore_ascii_case("lua")) {
        v.insert(format!("{nombre}.lua"), b);
        return limpiar(nombre, v);
    }
    let mut v = leer_zip(&b)?;
    // Una ruta que se sale es mala señal: no se instala nada. Se mira antes de quitar la carpeta de afuera.
    if let Some(k) = v.keys().find(|k| k.split(['/', '\\']).any(|p| p == "..") || k.starts_with(['/', '\\']) || k.contains(':')) {
        return Err(tf("«{}» se sale de su carpeta", &[k]));
    }
    // Los zip suelen traer todo dentro de una carpeta.
    let mut nombre = nombre;
    let raiz = v.keys().next().and_then(|k| k.split_once('/')).map(|x| x.0.to_string());
    if let Some(x) = raiz.filter(|x| v.keys().all(|k| k.starts_with(&format!("{x}/")))) {
        v = v.into_iter().map(|(k, b)| (k[x.len() + 1..].to_string(), b)).collect();
        nombre = x;
    }
    limpiar(nombre, v)
}

fn conocida(n: &str) -> bool {
    Path::new(n).extension().is_some_and(|e| CONOCIDAS.contains(&e.to_string_lossy().to_lowercase().as_str()))
}

fn limpiar(nombre: String, v: BTreeMap<String, Vec<u8>>) -> Result<Paquete, String> {
    let mut archivos = BTreeMap::new();
    for (k, b) in v {
        if conocida(&k) {
            archivos.insert(sana(&k).ok_or_else(|| tf("nombre de archivo no permitido: «{}»", &[&k]))?, b);
        }
    }
    Ok(Paquete { nombre, archivos })
}

impl Paquete {
    /// El guion principal: el que se llama como el paquete, `plugin.lua`, o el único que haya.
    pub fn principal(&self) -> Result<String, String> {
        let luas: Vec<&String> = self.archivos.keys().filter(|k| k.ends_with(".lua") && !k.contains('/')).collect();
        let hay = |n: String| luas.iter().find(|k| k.to_lowercase() == n.to_lowercase()).map(|k| k.to_string());
        // «mi-plugin-1.2.zip» también vale para «mi-plugin.lua».
        hay(format!("{}.lua", self.nombre))
            .or_else(|| hay("plugin.lua".into()))
            .or_else(|| (luas.len() == 1).then(|| luas[0].clone()))
            .or_else(|| luas.iter().find(|k| self.nombre.to_lowercase().starts_with(&k.to_lowercase()[..k.len() - 4])).map(|k| k.to_string()))
            .ok_or_else(|| {
                if luas.is_empty() {
                    t("no trae ningún guion .lua").into()
                } else {
                    t("no se sabe cuál es el guion principal: llámalo plugin.lua").into()
                }
            })
    }
}

/// Pone a prueba un plugin: que arme, que suene, que no reviente, cuánto cuesta y que su ventana dibuje.
pub fn revisar(fuente: &str, a: &Arc<Archivos>, sr: f32) -> Result<(Ficha, Informe), String> {
    let (ficha, receta) = crate::lua::armar_con(fuente, sr, a)?;
    let mut inf = Informe { cpu: 0.0, avisos: vec![] };
    let falta = |n: &String| !a.audios.contains_key(n);
    if let Some(n) = ficha.audios().into_iter().find(falta) {
        return Err(tf("falta el audio «{}» en la carpeta", &[&n]));
    }
    let Some(r) = receta else { return Ok((ficha, inf)) };
    let ctl: Vec<f32> = r.controles.iter().map(|c| c.valor).collect();
    let seg = 1.0;
    let n = (sr * seg) as usize;
    let vivo_cpu = if r.vivo { en_vivo(fuente, a, &ficha, &r.controles, sr)? } else { 0.0 };
    let ini = std::time::Instant::now();
    let y = if matches!(ficha.tipo, Tipo::Efecto | Tipo::Notas) {
        // Un tono con ruido: despierta a casi cualquier efecto.
        let mut s = 7u32;
        let x: Vec<f32> = (0..n)
            .map(|i| {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                (i as f32 * 0.06).sin() * 0.4 + (s as f32 / u32::MAX as f32 - 0.5) * 0.2
            })
            .collect();
        medir::correr(&r, &ctl, &[x.clone(), x])
    } else {
        medir::nota(&r, &ctl, 220.0, n)
    };
    inf.cpu = ini.elapsed().as_secs_f32() / seg * 100.0 + vivo_cpu;
    if inf.cpu > MAX_CPU {
        return Err(tf("pide demasiado procesador: {} % con una sola voz", &[&(inf.cpu as u32)]));
    }
    if inf.cpu > 20.0 {
        inf.avisos.push(tf("usa {} % del procesador", &[&(inf.cpu as u32)]));
    }
    if y.iter().flatten().any(|v| !v.is_finite()) {
        return Err(t("su sonido se vuelve infinito").into());
    }
    let pico = y.iter().flatten().fold(0f32, |m, v| m.max(v.abs()));
    if pico > 8.0 {
        return Err(tf("suena demasiado fuerte: {} veces el tope", &[&(pico as u32)]));
    }
    if pico > 2.0 {
        inf.avisos.push(t("suena muy fuerte").into());
    }
    let callado = pico < 1e-5 && !r.vivo && !r.usa_muestra() && !r.con_huella() && !r.con_lateral();
    if callado {
        inf.avisos.push(t("con sus controles de fábrica no suena").into());
    }
    if let Some(mut i) = crate::interfaz::cargar_con(fuente, a.clone())? {
        let v = ficha.controles.iter().map(|c| (c.clave.clone(), c.valor)).collect();
        let m = crate::interfaz::Momento {
            activo: true,
            nombre: ficha.nombre.ver().into(),
            nombres: ficha.controles.iter().map(|c| (c.clave.clone(), c.nombre.ver().to_string())).collect(),
            ..Default::default()
        };
        i.cuadro(&v, &m);
        if let Some(e) = i.error {
            return Err(tf("su ventana falla: {}", &[&e]));
        }
    }
    Ok((ficha, inf))
}

/// Corre un segundo de lo que hace en vivo y dice cuánto procesador pide.
fn en_vivo(fuente: &str, a: &Arc<Archivos>, ficha: &Ficha, controles: &[crate::receta::Control], sr: f32) -> Result<f32, String> {
    use crate::plugin::Evento;
    use crate::receta::BLOQUE;
    let claves: Vec<String> = controles.iter().map(|c| c.clave.clone()).collect();
    let ctl: Vec<f32> = controles.iter().map(|c| c.valor).collect();
    let v = crate::vivo::Vivo::nuevo(fuente, a, sr, &claves, &Default::default(), (&ficha.medidas, None, 0))?;
    let mut notas = Vec::with_capacity(64);
    let ini = std::time::Instant::now();
    let bloques = (sr as usize) / BLOQUE;
    let mut s = 3u32;
    for b in 0..bloques {
        let mut x = [[0.0; BLOQUE]; 2];
        if ficha.tipo == Tipo::Efecto {
            for i in 0..BLOQUE {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                let y = ((b * BLOQUE + i) as f32 * 0.03).sin() * 0.4 + (s as f32 / u32::MAX as f32 - 0.5) * 0.1;
                (x[0][i], x[1][i]) = (y, y);
            }
        }
        let ev: &[Evento] = match b {
            2 => &[Evento::Nota { tono: 60, fuerza: 0.9 }, Evento::Nota { tono: 64, fuerza: 0.7 }],
            20 => &[Evento::Rueda(0.5), Evento::Mando { n: 1, v: 0.5 }],
            b if b == bloques / 2 => &[Evento::Suelta { tono: 60 }, Evento::Suelta { tono: 64 }],
            _ => &[],
        };
        let t = crate::vivo::Momento { bpm: 120.0, pos: b as f64 * BLOQUE as f64 / sr as f64 * 2.0, tocando: true };
        v.procesar(ev, &ctl, &t, &mut x, &[[0.0; BLOQUE]; 2], BLOQUE, &mut notas);
        notas.clear();
        if let Some(e) = v.falla() {
            return Err(e);
        }
        if x.iter().flatten().any(|v| v.abs() > 8.0) {
            return Err(tf("suena demasiado fuerte: {} veces el tope", &[&8]));
        }
    }
    Ok(ini.elapsed().as_secs_f32() * 100.0)
}

/// Cómo estaba cada plugin del disco: así se sabe cuál cambió.
pub type Vista = BTreeMap<PathBuf, u64>;

/// La huella de cada cosa en la carpeta de plugins.
pub fn mirar(dir: &Path) -> Vista {
    use std::hash::{DefaultHasher, Hash, Hasher};
    fn sumar(p: &Path, hondo: u8, h: &mut DefaultHasher) {
        let Ok(m) = std::fs::symlink_metadata(p) else { return };
        p.hash(h);
        if m.is_dir() && hondo > 0 {
            let mut v: Vec<PathBuf> = std::fs::read_dir(p).into_iter().flatten().flatten().map(|e| e.path()).collect();
            v.sort();
            v.iter().take(MAX_ARCHIVOS * 2).for_each(|x| sumar(x, hondo - 1, h));
        } else if m.is_file() {
            (m.len(), m.modified().ok()).hash(h);
        }
    }
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.file_name().is_some_and(|n| n != QUITADOS) || p.extension().is_some_and(|e| e == "lua"))
        .map(|p| {
            let mut h = DefaultHasher::new();
            sumar(&p, 2, &mut h);
            (p, h.finish())
        })
        .collect()
}

/// Donde queda lo que se quita o se reemplaza, por si se quiere de vuelta.
pub const QUITADOS: &str = "Quitados";

/// Lo que pasó al mirar la carpeta.
pub enum Cambio {
    Nuevo(String),
    Mejor(String),
    Fuera(String),
    Mal(String, String),
}

impl Biblioteca {
    fn apartar(&self, p: &Path) -> Result<(), String> {
        let d = self.carpeta.as_ref().ok_or_else(|| t("No hay carpeta para guardar guiones").to_string())?.join(QUITADOS);
        std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
        std::fs::rename(p, d.join(format!("{}-{n}", p.file_name().unwrap_or_default().to_string_lossy()))).map_err(|e| e.to_string())
    }

    /// Instala un plugin y dice con qué clave quedó. Si ya estaba ese mismo, lo actualiza; a los de fábrica no los toca.
    pub fn instalar(&mut self, r: &Path) -> Result<(String, Informe), String> {
        let paq = abrir(r)?;
        let principal = paq.principal()?;
        let dir = self.carpeta.clone().ok_or_else(|| t("No hay carpeta para guardar guiones").to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        // Se arma primero en una carpeta aparte: si algo está mal, no queda nada a medias.
        let obra = dir.join(format!(".instalando-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&obra);
        let base = principal.trim_end_matches(".lua").to_string();
        let listo = (|| -> Result<(String, Informe), String> {
            for (n, b) in &paq.archivos {
                let destino = obra.join(n);
                std::fs::create_dir_all(destino.parent().unwrap_or(&obra)).map_err(|e| e.to_string())?;
                std::fs::write(destino, b).map_err(|e| e.to_string())?;
            }
            let fuente = String::from_utf8(paq.archivos[&principal].clone()).map_err(|_| t("el guion no es texto").to_string())?;
            let extras = Arc::new(paquete::extras(&obra, &base)?);
            let (ficha, inf) = revisar(&fuente, &extras, self.sr)?;
            // El mismo plugin se actualiza; uno distinto con el mismo nombre de archivo toma otra clave.
            let propia = self.clave_de(&base);
            let clave = match self.ver(&propia) {
                None => propia,
                Some(g) if g.propio && g.ficha.nombre == ficha.nombre && g.ficha.tipo == ficha.tipo && g.ficha.autor == ficha.autor => propia,
                Some(_) => self.clave_libre(&base),
            };
            std::fs::rename(obra.join(&principal), obra.join(format!("{clave}.lua"))).map_err(|e| e.to_string())?;
            let destino = dir.join(&clave);
            for viejo in [destino.clone(), dir.join(format!("{clave}.lua"))] {
                if viejo.exists() {
                    self.apartar(&viejo)?;
                }
            }
            std::fs::rename(&obra, &destino).map_err(|e| e.to_string())?;
            self.leer_uno(&destino)?;
            Ok((clave, inf))
        })();
        let _ = std::fs::remove_dir_all(&obra);
        self.mirado();
        listo
    }

    /// El plugin en un solo archivo, para compartirlo.
    pub fn empacar(&self, clave: &str) -> Result<Vec<u8>, String> {
        let g = self.ver(clave).filter(|g| g.propio).ok_or_else(|| t("Solo se empacan los tuyos").to_string())?;
        let mut v = match &g.carpeta {
            Some(d) => abrir(d)?.archivos,
            None => BTreeMap::new(),
        };
        v.entry(format!("{clave}.lua")).or_insert_with(|| g.fuente.clone().into_bytes());
        Ok(escribir_zip(&v.into_iter().map(|(k, b)| (format!("{clave}/{k}"), b)).collect()))
    }

    /// Lo aparta a «Quitados». Si tapaba a uno de fábrica, ese regresa.
    pub fn quitar(&mut self, clave: &str) -> Result<(), String> {
        let g = self.ver(clave).filter(|g| g.propio).ok_or_else(|| t("Los de fábrica no se quitan").to_string())?;
        let suelto = self.carpeta.as_ref().map(|d| d.join(format!("{clave}.lua")));
        if let Some(p) = g.carpeta.clone().or(suelto).filter(|p| p.exists()) {
            self.apartar(&p)?;
        }
        self.olvidar(clave);
        self.mirado();
        Ok(())
    }

    /// Revisa la carpeta y carga lo que cambió: se llama seguido, mientras el programa corre.
    pub fn vigilar(&mut self) -> Vec<Cambio> {
        let Some(dir) = self.carpeta.clone() else { return vec![] };
        let ahora = mirar(&dir);
        if ahora == self.visto {
            return vec![];
        }
        let antes = std::mem::replace(&mut self.visto, ahora.clone());
        let clave = |p: &Path| p.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let mut v = vec![];
        for (p, h) in &ahora {
            if antes.get(p) == Some(h) || p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
                continue;
            }
            let estaba = self.ver(&clave(p)).is_some_and(|g| g.propio);
            // Si está mal escrito sigue sonando la versión anterior.
            v.push(match self.leer_uno(p) {
                Ok(()) if estaba => Cambio::Mejor(clave(p)),
                Ok(()) => Cambio::Nuevo(clave(p)),
                Err(e) => Cambio::Mal(clave(p), e),
            });
        }
        for p in antes.keys().filter(|p| !ahora.contains_key(*p)) {
            let k = clave(p);
            let queda = ahora.keys().any(|x| clave(x) == k);
            if !queda && self.ver(&k).is_some_and(|g| g.propio) {
                self.olvidar(&k);
                v.push(Cambio::Fuera(k));
            }
        }
        v
    }
}
