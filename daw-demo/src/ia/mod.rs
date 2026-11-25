// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! El asistente de IA. Aquí no hay nada del DAW: recibe unas herramientas, habla con la IA que haya y manda cada llamada
//! por un canal al programa, que es quien toca el proyecto (así se puede usar igual en otro programa).
pub mod acp;
mod api;
mod claude;
pub mod elegir;
pub mod mcp;

use crate::idioma::{t, tf};
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::time::Duration;

/// Vueltas de herramientas que puede dar en una sola petición.
pub const MAX_VUELTAS: usize = 150;

#[derive(Clone)]
pub struct Herramienta {
    pub nombre: &'static str,
    pub descripcion: &'static str,
    /// JSON Schema del objeto de entrada.
    pub esquema: Value,
}

/// La IA pide algo; el programa contesta.
pub struct Llamada {
    pub nombre: String,
    pub datos: Value,
    responde: Sender<Result<String, String>>,
}

impl Llamada {
    pub fn responder(self, r: Result<String, String>) {
        let _ = self.responde.send(r);
    }
}

/// El lado de la IA: pide y espera.
#[derive(Clone)]
pub struct Puerta {
    manda: Sender<Llamada>,
    /// Despierta a la ventana.
    avisa: Arc<dyn Fn() + Send + Sync>,
}

impl Puerta {
    pub fn nueva(avisa: Arc<dyn Fn() + Send + Sync>) -> (Puerta, Receiver<Llamada>) {
        let (manda, recibe) = bounded(64);
        (Puerta { manda, avisa }, recibe)
    }

    pub fn pedir(&self, nombre: &str, datos: Value) -> Result<String, String> {
        let (responde, r) = bounded(1);
        self.manda
            .send_timeout(Llamada { nombre: nombre.into(), datos, responde }, Duration::from_secs(5))
            .map_err(|_| t("el programa está ocupado").to_string())?;
        (self.avisa)();
        r.recv_timeout(Duration::from_secs(300)).unwrap_or_else(|_| Err(t("el programa no contestó").into()))
    }
}

/// Una herramienta contesta con una imagen PNG poniendo esto y luego la imagen en base64.
pub const IMAGEN: &str = "\u{1}png;base64,";

/// Los bytes en base64.
pub fn base64(b: &[u8]) -> String {
    const L: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(b.len().div_ceil(3) * 4);
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            s.push(if k <= c.len() { L[(n >> (18 - 6 * k)) as usize & 63] as char } else { '=' });
        }
    }
    s
}

/// Lo que va pasando en una respuesta.
#[derive(Clone, Debug, PartialEq)]
pub enum Evento {
    Texto(String),
    /// Un pedazo de lo que va diciendo.
    Parte(String),
    /// Lo que iba diciendo por partes ya está completo.
    Cierra,
    /// Usó una herramienta.
    Uso(String),
    Error(String),
    Fin,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Clase {
    /// Claude Code instalado: usa la suscripción de quien lo tiene.
    ClaudeCode,
    /// La API de Anthropic, con clave.
    Anthropic,
    /// Un agente instalado que sigue ACP: OpenCode, Cline, Gemini CLI…
    Acp,
    /// Cualquier servidor que hable como el de OpenAI: Ollama, LM Studio, OpenAI, Gemini…
    Compatible,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Proveedor {
    pub nombre: String,
    pub clase: Clase,
    /// Dirección del servidor, con su puerto, o el programa instalado.
    pub url: String,
    pub modelo: String,
    #[serde(default)]
    pub clave: String,
    /// Con qué se abre, si es un programa.
    #[serde(default)]
    pub args: Vec<String>,
    /// Los modelos que dijo tener.
    #[serde(skip)]
    pub modelos: Vec<String>,
    /// Lo encontró solo: no se guarda.
    #[serde(skip)]
    pub hallado: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Ajustes {
    #[serde(default)]
    pub elegido: String,
    #[serde(default)]
    pub propios: Vec<Proveedor>,
    /// El modelo elegido de cada IA, por su nombre.
    #[serde(default)]
    pub modelos: std::collections::BTreeMap<String, String>,
    /// 0 es cualquiera libre.
    #[serde(default)]
    pub puerto_mcp: u16,
    #[serde(default)]
    pub token: String,
    /// Dejar abierto el servidor MCP para otros programas.
    #[serde(default)]
    pub mcp_abierto: bool,
    /// Dejar que busque archivos por nombre en las carpetas de la persona.
    #[serde(default)]
    pub buscar: bool,
    /// Dejar que vea cuadros del video.
    #[serde(default)]
    pub ver: bool,
}

impl Ajustes {
    /// De las que hay, la elegida, con su modelo.
    pub fn actual(&self, hallados: &[Proveedor]) -> Option<Proveedor> {
        let v = || hallados.iter().chain(&self.propios);
        let mut p = v().find(|p| p.nombre == self.elegido).or(v().next())?.clone();
        if let Some(m) = self.modelos.get(&p.nombre) {
            p.modelo = m.clone();
        }
        Some(p)
    }

    pub fn leer(ruta: &Path) -> Ajustes {
        let mut a: Ajustes = std::fs::read_to_string(ruta).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        if a.token.len() < 32 {
            a.token = token();
        }
        a
    }

    /// Lleva claves: solo su dueño lo puede leer.
    pub fn guardar(&self, ruta: &Path) -> Result<(), String> {
        let b = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        privado(ruta, &b)
    }
}

/// Escribe un archivo que solo puede leer su dueño.
pub fn privado(ruta: &Path, b: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let parcial = ruta.with_extension("parcial");
    let _ = std::fs::remove_file(&parcial);
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
    o.open(&parcial).and_then(|mut f| f.write_all(b)).and_then(|_| std::fs::rename(&parcial, ruta)).map_err(|e| e.to_string())
}

fn token() -> String {
    let mut b = [0u8; 24];
    getrandom::fill(&mut b).expect("el sistema da azar");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Dónde se buscan los programas. Las apps abiertas desde el Finder no heredan el PATH de la terminal.
fn carpetas_de_programas() -> Vec<PathBuf> {
    let casa = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from).unwrap_or_default();
    let mut v: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    for s in [".local/bin", ".claude/local", ".opencode/bin", ".cargo/bin", ".bun/bin", ".npm-global/bin", ".volta/bin", "AppData/Roaming/npm"] {
        v.push(casa.join(s));
    }
    v.extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"].map(PathBuf::from));
    // Los de nvm, del más nuevo al más viejo.
    let mut n: Vec<PathBuf> =
        std::fs::read_dir(casa.join(".nvm/versions/node")).into_iter().flatten().flatten().map(|e| e.path().join("bin")).collect();
    n.sort();
    v.extend(n.into_iter().rev());
    let mut ya = std::collections::HashSet::new();
    v.retain(|x| ya.insert(x.clone()));
    v
}

pub(crate) fn ruta_de_programas() -> std::ffi::OsString {
    std::env::join_paths(carpetas_de_programas()).unwrap_or_default()
}

/// Un programa instalado.
pub fn programa(nombre: &str) -> Option<PathBuf> {
    let nombres: Vec<String> = if cfg!(windows) { vec![format!("{nombre}.exe"), format!("{nombre}.cmd")] } else { vec![nombre.into()] };
    carpetas_de_programas().iter().flat_map(|d| nombres.iter().map(move |n| d.join(n))).find(|p| p.is_file())
}

/// Las IA que hay en esta computadora. Tarda un momento: pregunta a los servidores locales.
pub fn detectar() -> Vec<Proveedor> {
    let nuevo = |nombre: &str, clase, url: &str, modelo: &str, clave: String| Proveedor {
        nombre: nombre.into(),
        clase,
        url: url.into(),
        modelo: modelo.into(),
        clave,
        args: vec![],
        modelos: vec![],
        hallado: true,
    };
    let mut v = vec![];
    if let Some(p) = programa("claude") {
        // El más capaz de los que trae su plan; se puede cambiar.
        v.push(nuevo("Claude Code", Clase::ClaudeCode, &p.to_string_lossy(), "opus", String::new()));
    }
    // Los agentes ACP (`acp::CONOCIDOS`) todavía no reciben bien las herramientas: aún no se ofrecen.
    let var = |k: &str| std::env::var(k).ok().filter(|s| !s.trim().is_empty());
    if let Some(k) = var("ANTHROPIC_API_KEY") {
        v.push(nuevo("Claude API", Clase::Anthropic, api::ANTHROPIC, api::MODELO, k));
    }
    if let Some(k) = var("OPENAI_API_KEY") {
        v.push(nuevo("OpenAI", Clase::Compatible, "https://api.openai.com/v1", "", k));
    }
    if let Some(k) = var("GEMINI_API_KEY") {
        v.push(nuevo("Gemini", Clase::Compatible, "https://generativelanguage.googleapis.com/v1beta/openai", "", k));
    }
    let locales = [("Ollama", 11434), ("LM Studio", 1234), ("llama.cpp", 8080)];
    let hilos = locales.map(|(n, p)| {
        std::thread::spawn(move || {
            let url = format!("http://127.0.0.1:{p}/v1");
            api::modelos(&url, "", Duration::from_millis(700)).ok().map(|m| (n, url, m))
        })
    });
    for h in hilos {
        if let Ok(Some((n, url, m))) = h.join()
            && !m.is_empty()
        {
            let mut p = nuevo(n, Clase::Compatible, &url, &m[0], String::new());
            p.modelos = m;
            v.push(p);
        }
    }
    v
}

/// Los modelos que ofrece un servidor.
pub fn modelos(p: &Proveedor) -> Result<Vec<String>, String> {
    match p.clase {
        Clase::ClaudeCode => Ok(vec![]),
        Clase::Acp => Ok(acp::Vivo::abrir(p, None, &std::env::temp_dir().join("beethoven-ia"))?.modelos.clone()),
        Clase::Anthropic => api::modelos_anthropic(&p.url, &p.clave),
        Clase::Compatible => api::modelos(&p.url, &p.clave, Duration::from_secs(10)),
    }
}

/// La plática con una IA. Guarda su propia memoria.
pub struct Platica {
    sistema: String,
    herramientas: Arc<Vec<Herramienta>>,
    puerta: Puerta,
    memoria: Arc<std::sync::Mutex<Memoria>>,
    parar: Arc<AtomicBool>,
    pub eventos: Receiver<Evento>,
    manda: Sender<Evento>,
    pub ocupada: bool,
}

/// Lo que cada clase necesita para seguir la plática.
#[derive(Default)]
struct Memoria {
    con: String,
    mensajes: Vec<Value>,
    sesion: Option<String>,
    acp: Option<acp::Vivo>,
}

/// Cómo llega Claude Code a las herramientas.
#[derive(Clone)]
pub struct Enlace {
    pub url: String,
    pub token: String,
    pub carpeta: PathBuf,
}

impl Platica {
    pub fn nueva(sistema: String, herramientas: Vec<Herramienta>, puerta: Puerta) -> Platica {
        let (manda, eventos) = unbounded();
        Platica {
            sistema,
            herramientas: Arc::new(herramientas),
            puerta,
            memoria: Default::default(),
            parar: Default::default(),
            eventos,
            manda,
            ocupada: false,
        }
    }

    pub fn herramientas(&self) -> Arc<Vec<Herramienta>> {
        self.herramientas.clone()
    }

    pub fn puerta(&self) -> Puerta {
        self.puerta.clone()
    }

    pub fn olvidar(&mut self) {
        self.parar();
        self.memoria = Default::default();
    }

    pub fn parar(&mut self) {
        self.parar.store(true, Relaxed);
        self.parar = Default::default();
        // Lo que siga llegando de la anterior ya no se escucha.
        let (manda, eventos) = unbounded();
        (self.manda, self.eventos) = (manda, eventos);
        self.ocupada = false;
    }

    /// Manda un mensaje; la respuesta llega por `eventos`.
    pub fn decir(&mut self, p: &Proveedor, mensaje: &str, enlace: Option<Enlace>) {
        self.ocupada = true;
        let (p, mensaje, sistema) = (p.clone(), mensaje.to_string(), self.sistema.clone());
        let (h, puerta, memoria, parar, manda) =
            (self.herramientas.clone(), self.puerta.clone(), self.memoria.clone(), self.parar.clone(), self.manda.clone());
        std::thread::spawn(move || {
            let mut m = memoria.lock().unwrap_or_else(|e| e.into_inner());
            // Cambiar de IA empieza otra plática: las memorias no se entienden entre sí.
            let modelo = if p.clase == Clase::Acp { "" } else { &p.modelo };
            let con = format!("{:?} {} {modelo}", p.clase, p.url);
            if m.con != con {
                *m = Memoria { con, ..Default::default() };
            }
            let avisa = puerta.avisa.clone();
            let di = |e: Evento| {
                let _ = manda.send(e);
                avisa();
            };
            let r = match p.clase {
                Clase::ClaudeCode => match &enlace {
                    Some(e) => claude::decir(&p, &sistema, &mensaje, &mut m.sesion, e, &parar, &di),
                    None => Err(t("no se pudo abrir el servidor de herramientas").into()),
                },
                Clase::Acp => (|| {
                    let e = enlace.as_ref().ok_or_else(|| t("no se pudo abrir el servidor de herramientas").to_string())?;
                    if m.acp.is_none() {
                        m.acp = Some(acp::Vivo::abrir(&p, Some(e), &e.carpeta)?);
                    }
                    let nombres: Vec<&str> = h.iter().map(|h| h.nombre).collect();
                    let v = m.acp.as_mut().ok_or("acp")?;
                    let r = v.decir(&p, &sistema, &mensaje, &nombres, &parar, &di);
                    // Si algo falló, la próxima vez se abre de nuevo.
                    if r.is_err() {
                        m.acp = None;
                    }
                    r
                })(),
                Clase::Anthropic | Clase::Compatible => {
                    // Si falla, la memoria vuelve a como estaba: no queda una pregunta sin respuesta.
                    let antes = m.mensajes.len();
                    let c = api::Caso { p: &p, sistema: &sistema, herramientas: &h, puerta: &puerta, parar: &parar, di: &di };
                    let r =
                        if p.clase == Clase::Anthropic { c.anthropic(&mut m.mensajes, &mensaje) } else { c.compatible(&mut m.mensajes, &mensaje) };
                    if r.is_err() {
                        m.mensajes.truncate(antes);
                    }
                    r
                }
            };
            if let Err(e) = r {
                di(Evento::Error(e));
            }
            di(Evento::Fin);
        });
    }
}

/// Arregla lo que las IA chicas suelen mandar mal: números y listas como texto, enteros con punto.
pub fn acomodar(esquema: &Value, d: &mut Value) {
    if let Value::String(s) = d
        && matches!(esquema["type"].as_str(), Some("number" | "integer" | "boolean" | "array" | "object"))
        && let Ok(v) = serde_json::from_str::<Value>(s.trim())
        && !v.is_string()
    {
        *d = v;
    }
    match d {
        Value::Number(n) if esquema["type"] == "integer" && n.as_u64().is_none() && n.as_i64().is_none() => {
            if let Some(x) = n.as_f64().filter(|x| x.fract() == 0.0 && x.abs() < 1e15) {
                *d = (x as i64).into();
            }
        }
        // A veces mandan uno solo donde va una lista.
        Value::Array(v) => v.iter_mut().for_each(|x| acomodar(&esquema["items"], x)),
        Value::Object(o) => {
            let extra = &esquema["additionalProperties"];
            for (k, x) in o.iter_mut() {
                let e = &esquema["properties"][k.as_str()];
                acomodar(if e.is_null() && extra.is_object() { extra } else { e }, x);
            }
        }
        x if esquema["type"] == "array" && !x.is_null() => *x = Value::Array(vec![x.take()]),
        _ => {}
    }
}

/// Lo mismo para una llamada entera, con lo que lleve dentro si es «varias».
pub fn acomodar_llamada(h: &[Herramienta], nombre: &str, d: &mut Value) {
    if let Some(x) = h.iter().find(|x| x.nombre == nombre) {
        acomodar(&x.esquema, d);
    }
    if nombre == "varias" {
        for a in d["acciones"].as_array_mut().into_iter().flatten() {
            let n = a["herramienta"].as_str().unwrap_or("").to_string();
            if n != "varias" && a["datos"].is_object() {
                acomodar_llamada(h, &n, &mut a["datos"]);
            }
        }
    }
}

/// Un error de red en pocas palabras, sin la clave.
fn de_red(e: impl std::fmt::Display) -> String {
    tf("No se pudo conectar: {}", &[&e])
}
