// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Lua: arma los sonidos y corre las herramientas de notas. Nunca corre en el hilo de audio.
use crate::idioma::{Texto, t as tr, tf};
use crate::modelo::{Nota, PASO};
use crate::paquete::Archivos;
use crate::receta::{ALCANCE, Banda, Control, Filtro, MAX_RETARDO, N, Nodo, Onda, PUNTOS, Receta, Tabla};
use mlua::{
    ChunkMode, FromLua, Function, HookTriggers, Lua, LuaOptions, MetaMethod, MultiValue, StdLib, Table, UserData, UserDataMethods, Value, VmState,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

const MAX_NODOS: usize = 8000;
/// Ondas y curvas propias por guion.
const MAX_TABLAS: usize = 32;
/// Números que pueden guardar todas las voces juntas (64 MB).
const MAX_MEMORIA: usize = 16 << 20;
const MAX_PASOS: u32 = 80_000_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tipo {
    Instrumento,
    Efecto,
    Herramienta,
    /// Cambia las notas antes de que lleguen al instrumento, en vivo.
    Notas,
}

/// Con qué suena `muestra()` en un instrumento.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Muestras {
    /// El audio que elija quien lo usa.
    #[default]
    Archivo,
    /// Dieciséis audios, uno por tecla.
    Pads,
    /// El audio elegido, partido en pedazos.
    Cortes,
    /// Los audios que trae el plugin en su carpeta.
    Propias(Vec<Grabacion>),
}

/// Un audio del plugin: en qué nota suena tal cual, y de qué tecla a cuál y con qué fuerzas se usa.
#[derive(Clone, Debug, PartialEq)]
pub struct Grabacion {
    pub archivo: String,
    pub nota: u8,
    pub teclas: (u8, u8),
    pub fuerzas: (u8, u8),
}

/// La versión de lo que puede usar un guion. Sube cuando se agrega algo.
pub const API: u32 = 1;

#[derive(Clone, Debug)]
pub struct Ficha {
    pub autor: String,
    pub version: String,
    pub descripcion: Texto,
    pub muestras: Muestras,
    /// El audio de su carpeta que usa de huella en la convolución.
    pub huella: Option<String>,
    /// No se ofrece en las listas.
    pub oculto: bool,
    /// Suena sin fin a propósito: el motor no lo apaga aunque no le entre nada.
    pub sostiene: bool,
    /// Corre mientras suena, y lo que deja ver desde ahí.
    pub vivo: bool,
    pub medidas: Vec<String>,
    pub nombre: Texto,
    pub tipo: Tipo,
    pub grupo: Texto,
    pub controles: Vec<Control>,
    /// Imagen de su carpeta que hace de logo («logo.png»).
    pub logo: Option<String>,
    /// Ajustes ya hechos: nombre y los controles que cambian; lo demás va como de fábrica.
    pub presets: Vec<(Texto, BTreeMap<String, f32>)>,
}

impl Ficha {
    /// Los audios de su carpeta que necesita.
    pub fn audios(&self) -> Vec<String> {
        let mut v: Vec<String> = self.huella.iter().cloned().collect();
        if let Muestras::Propias(g) = &self.muestras {
            v.extend(g.iter().map(|g| g.archivo.clone()));
        }
        v
    }
}

type Grafo = Rc<RefCell<Vec<Nodo>>>;

#[derive(Clone)]
struct Senal(N, Grafo);

/// Lo que aceptan las funciones de sonido: un número o una señal.
enum Arg {
    Num(f32),
    Sen(Senal),
}

impl FromLua for Arg {
    fn from_lua(v: Value, _: &Lua) -> mlua::Result<Self> {
        match v {
            Value::Integer(i) => Ok(Arg::Num(i as f32)),
            Value::Number(x) if x.is_finite() => Ok(Arg::Num(x as f32)),
            Value::UserData(u) => Ok(Arg::Sen(u.borrow::<Senal>()?.clone())),
            v => Err(mlua::Error::runtime(tf("se esperaba un número o un sonido y llegó «{}»", &[&v.type_name()]))),
        }
    }
}

/// Un texto, o `{ es = "…", en = "…" }`.
impl FromLua for Texto {
    fn from_lua(v: Value, lua: &Lua) -> mlua::Result<Self> {
        match v {
            Value::Table(t) => Ok(Texto { es: t.get("es")?, en: t.get("en")? }),
            v => Ok(Texto { es: String::from_lua(v, lua)?, en: None }),
        }
    }
}

fn poner(g: &Grafo, n: Nodo) -> mlua::Result<Senal> {
    let mut v = g.borrow_mut();
    if v.len() >= MAX_NODOS {
        return Err(mlua::Error::runtime(tr("el sonido es demasiado grande")));
    }
    v.push(n);
    Ok(Senal(v.len() as N - 1, g.clone()))
}

fn idx(g: &Grafo, a: Arg) -> mlua::Result<N> {
    match a {
        Arg::Sen(s) => Ok(s.0),
        Arg::Num(x) => Ok(poner(g, Nodo::Const(x))?.0),
    }
}

impl UserData for Senal {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        let ops: [(MetaMethod, fn(N, N) -> Nodo); 5] = [
            (MetaMethod::Add, Nodo::Suma),
            (MetaMethod::Sub, Nodo::Resta),
            (MetaMethod::Mul, Nodo::Por),
            (MetaMethod::Div, Nodo::Entre),
            (MetaMethod::Pow, Nodo::Pot),
        ];
        for (mm, f) in ops {
            m.add_meta_function(mm, move |_, (a, b): (Arg, Arg)| {
                let g = match (&a, &b) {
                    (Arg::Sen(s), _) | (_, Arg::Sen(s)) => s.1.clone(),
                    _ => unreachable!("Lua solo llama esto con un sonido"),
                };
                let (a, b) = (idx(&g, a)?, idx(&g, b)?);
                poner(&g, f(a, b))
            });
        }
        m.add_meta_method(MetaMethod::Unm, |_, s, ()| {
            let k = poner(&s.1, Nodo::Const(-1.0))?;
            poner(&s.1, Nodo::Por(s.0, k.0))
        });
    }
}

/// Un Lua sin archivos ni sistema, con tope de memoria y de pasos.
pub fn lua_cerrado() -> mlua::Result<Lua> {
    let lua = Lua::new_with(StdLib::MATH | StdLib::STRING | StdLib::TABLE, LuaOptions::new())?;
    lua.set_memory_limit(64 << 20)?;
    for f in ["dofile", "loadfile", "load", "collectgarbage", "print", "require"] {
        lua.globals().set(f, Value::Nil)?;
    }
    // Buscar con patrones corre en C y no cuenta pasos: con textos largos podría no acabar nunca.
    let cadena: Table = lua.globals().get("string")?;
    for (nombre, patron) in [("find", true), ("match", true), ("gmatch", true), ("gsub", true), ("rep", false)] {
        let f: Function = cadena.get(nombre)?;
        cadena.set(
            nombre,
            lua.create_function(move |_, a: MultiValue| {
                let largo = |i: usize| a.get(i).and_then(|v| v.as_string()).map_or(0, |s| s.as_bytes().len());
                if patron {
                    let abiertos = a.get(1).and_then(|v| v.as_string()).map_or(0, |s| s.as_bytes().iter().filter(|c| b"*+-?".contains(c)).count());
                    if largo(0) > 2000 || largo(1) > 64 || abiertos > 2 && largo(0) > 200 || abiertos > 4 {
                        return Err(mlua::Error::runtime(tr("ese texto es demasiado largo para buscar en él")));
                    }
                } else {
                    let veces = a.get(1).and_then(|v| v.as_number().or(v.as_integer().map(|i| i as f64))).unwrap_or(0.0);
                    if largo(0) as f64 * veces > 1e6 {
                        return Err(mlua::Error::runtime(tr("ese texto es demasiado largo para buscar en él")));
                    }
                }
                f.call::<MultiValue>(a)
            })?,
        )?;
    }
    presupuesto(&lua, MAX_PASOS)?;
    Ok(lua)
}

/// Cuántos pasos puede dar de aquí en adelante.
pub fn presupuesto(lua: &Lua, pasos: u32) -> mlua::Result<()> {
    let cuenta = std::cell::Cell::new(0u32);
    lua.set_hook(HookTriggers::new().every_nth_instruction(10_000), move |_, _| {
        cuenta.set(cuenta.get() + 10_000);
        if cuenta.get() > pasos { Err(mlua::Error::runtime(tr("el guion tardó demasiado (¿un ciclo sin fin?)"))) } else { Ok(VmState::Continue) }
    })
}

/// Llama a la función del guion en cada punto y guarda lo que da.
fn llenar(f: &Function, n: usize, punto: impl Fn(usize) -> f32) -> mlua::Result<Vec<f32>> {
    (0..n)
        .map(|i| {
            let v = f.call::<f32>(punto(i))?;
            Ok(if v.is_finite() { v.clamp(-16.0, 16.0) } else { 0.0 })
        })
        .collect()
}

fn guardar(t: &RefCell<Vec<Tabla>>, x: Tabla) -> mlua::Result<u16> {
    let mut t = t.borrow_mut();
    if t.len() >= MAX_TABLAS {
        return Err(mlua::Error::runtime(tr("demasiadas ondas y curvas propias")));
    }
    t.push(x);
    Ok(t.len() as u16 - 1)
}

/// Medidas por guion.
pub const MAX_MEDIDAS: usize = 8;

/// Lo que da más libertad: curvas y ondas propias, lazos con lo que quieras dentro, el tempo y más cuentas.
fn registrar_propias(lua: &Lua, g: &Grafo, tablas: &Rc<RefCell<Vec<Tabla>>>, medidas: &Rc<RefCell<Vec<String>>>) -> mlua::Result<()> {
    let t = lua.globals();
    let (gg, mm) = (g.clone(), medidas.clone());
    t.set(
        "mide",
        lua.create_function(move |_, (nombre, x): (String, Arg)| {
            let mut m = mm.borrow_mut();
            if m.len() >= MAX_MEDIDAS {
                return Err(mlua::Error::runtime(tr("demasiadas medidas")));
            }
            m.push(nombre);
            let x = idx(&gg, x)?;
            poner(&gg, Nodo::Mide(x, m.len() as u16 - 1))
        })?,
    )?;
    let gg = g.clone();
    t.set(
        "convolucion",
        lua.create_function(move |_, (x, lado): (Arg, Option<String>)| {
            let x = idx(&gg, x)?;
            poner(&gg, Nodo::Convolucion(x, (lado.as_deref() == Some("der")) as u8))
        })?,
    )?;
    let (gg, tt) = (g.clone(), tablas.clone());
    t.set(
        "curva",
        lua.create_function(move |_, (x, f, como): (Arg, Function, Option<String>)| {
            let n = 2 * PUNTOS + 1;
            let v = llenar(&f, n, |i| -ALCANCE + 2.0 * ALCANCE * i as f32 / (n - 1) as f32)?;
            let k = guardar(&tt, Tabla::curva(v))?;
            let x = idx(&gg, x)?;
            poner(&gg, Nodo::Curva(x, k, como.as_deref() == Some("suave")))
        })?,
    )?;
    let (gg, tt) = (g.clone(), tablas.clone());
    t.set(
        "onda",
        lua.create_function(move |_, (hz, f, fase): (Arg, Function, Option<Arg>)| {
            let v = llenar(&f, PUNTOS, |i| i as f32 / PUNTOS as f32)?;
            let k = guardar(&tt, Tabla::onda(&v))?;
            let (hz, fase) = (idx(&gg, hz)?, idx(&gg, fase.unwrap_or(Arg::Num(0.0)))?);
            poner(&gg, Nodo::Propia(hz, fase, k))
        })?,
    )?;
    let gg = g.clone();
    t.set(
        "lazo",
        lua.create_function(move |_, (seg, f, tope): (Arg, Function, Option<f32>)| {
            let tope = match (&seg, tope) {
                (_, Some(m)) => m,
                (Arg::Num(v), None) => *v,
                _ => 1.0,
            };
            let tope = if tope.is_finite() { tope.clamp(0.002, MAX_RETARDO) } else { 1.0 };
            let seg = idx(&gg, seg)?;
            let vuelta = poner(&gg, Nodo::Lazo(seg, tope))?;
            let entra = idx(&gg, f.call::<Arg>(vuelta.clone())?)?;
            poner(&gg, Nodo::Cierra(vuelta.0, entra))
        })?,
    )?;
    let gg = g.clone();
    t.set("posicion", lua.create_function(move |_, ()| poner(&gg, Nodo::Posicion))?)?;
    let gg = g.clone();
    t.set("tempo", lua.create_function(move |_, ()| poner(&gg, Nodo::Bpm))?)?;
    let gg = g.clone();
    t.set(
        "negras",
        lua.create_function(move |_, n: Arg| {
            // Segundos que duran esas negras al tempo de la canción.
            let sesenta = poner(&gg, Nodo::Const(60.0))?;
            let bpm = poner(&gg, Nodo::Bpm)?;
            let una = poner(&gg, Nodo::Entre(sesenta.0, bpm.0))?;
            let n = idx(&gg, n)?;
            poner(&gg, Nodo::Por(una.0, n))
        })?,
    )?;
    for (nombre, f) in [("raiz", Nodo::Raiz as fn(N) -> Nodo), ("ln", Nodo::Log), ("exp", Nodo::Exp), ("piso", Nodo::Piso)] {
        let g = g.clone();
        t.set(
            nombre,
            lua.create_function(move |_, x: Arg| {
                let x = idx(&g, x)?;
                poner(&g, f(x))
            })?,
        )?;
    }
    for (nombre, f) in [("mayor", Nodo::Mayor as fn(N, N) -> Nodo), ("resto", Nodo::Resto)] {
        let g = g.clone();
        t.set(
            nombre,
            lua.create_function(move |_, (a, b): (Arg, Arg)| {
                let (a, b) = (idx(&g, a)?, idx(&g, b)?);
                poner(&g, f(a, b))
            })?,
        )?;
    }
    let gg = g.clone();
    t.set(
        "si",
        lua.create_function(move |_, (cual, a, b): (Arg, Arg, Arg)| {
            // `a` donde `cual` pasa de 0.5; `b` donde no.
            let (cual, a, b) = (idx(&gg, cual)?, idx(&gg, a)?, idx(&gg, b)?);
            let medio = poner(&gg, Nodo::Const(0.5))?;
            let es = poner(&gg, Nodo::Mayor(cual, medio.0))?;
            let d = poner(&gg, Nodo::Resta(a, b))?;
            let d = poner(&gg, Nodo::Por(d.0, es.0))?;
            poner(&gg, Nodo::Suma(b, d.0))
        })?,
    )
}

fn registrar(lua: &Lua, g: &Grafo) -> mlua::Result<()> {
    let t = lua.globals();
    macro_rules! fun {
        ($n:literal, |$g:ident, $a:tt : $ty:ty| $cuerpo:expr) => {{
            let $g = g.clone();
            t.set($n, lua.create_function(move |_, $a: $ty| -> mlua::Result<Senal> { $cuerpo })?)?;
        }};
    }
    type O = Option<Arg>;
    let o = |a: O, v: f32| a.unwrap_or(Arg::Num(v));
    fn osc(g: &Grafo, onda: Onda, hz: Arg, fase: Arg, ancho: Arg) -> mlua::Result<Senal> {
        let (a, b, c) = (idx(g, hz)?, idx(g, fase)?, idx(g, ancho)?);
        poner(g, Nodo::Osc(onda, a, b, c))
    }
    fun!("seno", |g, (hz, fase): (Arg, O)| osc(&g, Onda::Seno, hz, o(fase, 0.0), Arg::Num(0.5)));
    fun!("sierra", |g, (hz, fase): (Arg, O)| osc(&g, Onda::Sierra, hz, o(fase, 0.0), Arg::Num(0.5)));
    fun!("triangulo", |g, (hz, fase): (Arg, O)| osc(&g, Onda::Triangulo, hz, o(fase, 0.0), Arg::Num(0.5)));
    fun!("pulso", |g, (hz, ancho): (Arg, O)| osc(&g, Onda::Pulso, hz, Arg::Num(0.0), o(ancho, 0.5)));
    fun!("ruido", |g, _: ()| poner(&g, Nodo::Ruido));
    fun!("adsr", |g, (a, d, s, r): (Arg, Arg, Arg, Arg)| {
        let n = Nodo::Adsr(idx(&g, a)?, idx(&g, d)?, idx(&g, s)?, idx(&g, r)?);
        poner(&g, n)
    });
    fun!("golpe", |g, s: Arg| {
        let n = Nodo::Golpe(idx(&g, s)?);
        poner(&g, n)
    });
    for (nombre, f) in [("pasabajos", Filtro::Bajo), ("pasaaltos", Filtro::Alto), ("pasabanda", Filtro::Banda)] {
        let g = g.clone();
        t.set(
            nombre,
            lua.create_function(move |_, (x, hz, q): (Arg, Arg, O)| {
                let n = Nodo::Svf(f, idx(&g, x)?, idx(&g, hz)?, idx(&g, q.unwrap_or(Arg::Num(0.1)))?);
                poner(&g, n)
            })?,
        )?;
    }
    for (nombre, b) in [("campana", Banda::Campana), ("graves", Banda::Graves), ("agudos", Banda::Agudos)] {
        let g = g.clone();
        t.set(
            nombre,
            lua.create_function(move |_, (x, hz, db, q): (Arg, Arg, Arg, O)| {
                let n = Nodo::Eq(b, idx(&g, x)?, idx(&g, hz)?, idx(&g, db)?, idx(&g, q.unwrap_or(Arg::Num(0.707)))?);
                poner(&g, n)
            })?,
        )?;
    }
    fun!("muesca", |g, (x, hz, q): (Arg, Arg, O)| {
        let n = Nodo::Eq(Banda::Muesca, idx(&g, x)?, idx(&g, hz)?, idx(&g, Arg::Num(0.0))?, idx(&g, o(q, 2.0))?);
        poner(&g, n)
    });
    fun!("seguidor", |g, (x, a, s): (Arg, Arg, Arg)| {
        let n = Nodo::Seguidor(idx(&g, x)?, idx(&g, a)?, idx(&g, s)?);
        poner(&g, n)
    });
    for (nombre, f) in [("escalon", Nodo::Escalon as fn(N, N) -> Nodo), ("retener", Nodo::Retener), ("pasatodo", Nodo::Pasatodo)] {
        let g = g.clone();
        t.set(
            nombre,
            lua.create_function(move |_, (a, b): (Arg, Arg)| {
                let (a, b) = (idx(&g, a)?, idx(&g, b)?);
                poner(&g, f(a, b))
            })?,
        )?;
    }
    fun!("db", |g, x: Arg| {
        let (x, v, d) = (idx(&g, x)?, idx(&g, Arg::Num(20.0))?, idx(&g, Arg::Num(10.0))?);
        let x = poner(&g, Nodo::Entre(x, v))?;
        poner(&g, Nodo::Pot(d, x.0))
    });
    fun!("retardo", |g, (x, s, r, max): (Arg, Arg, O, Option<f32>)| {
        let tope = match (&s, max) {
            (_, Some(m)) => m,
            (Arg::Num(v), None) => *v,
            _ => 1.0,
        };
        let tope = if tope.is_finite() { tope.clamp(0.001, MAX_RETARDO) } else { 1.0 };
        let n = Nodo::Retardo(idx(&g, x)?, idx(&g, s)?, idx(&g, o(r, 0.0))?, tope);
        poner(&g, n)
    });
    fun!("sala", |g, (x, tam, b): (Arg, O, O)| {
        let n = Nodo::Sala(idx(&g, x)?, idx(&g, o(tam, 0.5))?, idx(&g, o(b, 0.5))?);
        poner(&g, n)
    });
    fun!("cuerda", |g, (x, hz, c, b): (Arg, Arg, O, O)| {
        let n = Nodo::Cuerda(idx(&g, x)?, idx(&g, hz)?, idx(&g, o(c, 2.0))?, idx(&g, o(b, 0.5))?);
        poner(&g, n)
    });
    fun!("muestra", |g, (vel, canal): (O, Option<String>)| {
        let n = Nodo::Muestra(idx(&g, o(vel, 1.0))?, (canal.as_deref() == Some("der")) as u8);
        poner(&g, n)
    });
    for (nombre, f) in [("tanh", Nodo::Tanh as fn(N) -> Nodo), ("abs", Nodo::Abs), ("sen", Nodo::Sen)] {
        let g = g.clone();
        t.set(
            nombre,
            lua.create_function(move |_, x: Arg| {
                let x = idx(&g, x)?;
                poner(&g, f(x))
            })?,
        )?;
    }
    for (nombre, f) in [("minimo", Nodo::Min as fn(N, N) -> Nodo), ("maximo", Nodo::Max)] {
        let g = g.clone();
        t.set(
            nombre,
            lua.create_function(move |_, (a, b): (Arg, Arg)| {
                let (a, b) = (idx(&g, a)?, idx(&g, b)?);
                poner(&g, f(a, b))
            })?,
        )?;
    }
    fun!("mezclar", |g, (a, b, k): (Arg, Arg, Arg)| {
        let (a, b, k) = (idx(&g, a)?, idx(&g, b)?, idx(&g, k)?);
        let d = poner(&g, Nodo::Resta(b, a))?;
        let d = poner(&g, Nodo::Por(d.0, k))?;
        poner(&g, Nodo::Suma(a, d.0))
    });
    Ok(())
}

pub fn ficha_de(t: &Table) -> mlua::Result<Ficha> {
    let tipo = match t.get::<Option<String>>("tipo")?.as_deref() {
        None | Some("instrumento") => Tipo::Instrumento,
        Some("efecto") => Tipo::Efecto,
        Some("herramienta") => Tipo::Herramienta,
        Some("notas") => Tipo::Notas,
        Some(x) => return Err(mlua::Error::runtime(tf("tipo «{}» no existe: usa instrumento, efecto o herramienta", &[&x]))),
    };
    let nombre = t.get::<Option<Texto>>("nombre")?.ok_or_else(|| mlua::Error::runtime(tr("falta «nombre»")))?;
    let grupo = t.get::<Option<Texto>>("grupo")?.unwrap_or_else(|| Texto { es: "Otros".into(), en: None });
    let mut controles: Vec<Control> = vec![];
    if let Some(cs) = t.get::<Option<Table>>("controles")? {
        for c in cs.sequence_values::<Table>() {
            let c = c?;
            let (clave, nombre): (String, Texto) = (c.get(1)?, c.get(2)?);
            let (min, max, valor): (f32, f32, f32) = (c.get(3)?, c.get(4)?, c.get(5)?);
            if !(min < max && min.is_finite() && max.is_finite() && valor.is_finite()) {
                return Err(mlua::Error::runtime(tf("control «{}»: el mínimo debe ser menor que el máximo", &[&clave])));
            }
            if controles.iter().any(|x| x.clave == clave) {
                return Err(mlua::Error::runtime(tf("control «{}» repetido", &[&clave])));
            }
            controles.push(Control { clave, nombre, min, max, valor: valor.clamp(min, max) });
        }
    }
    let logo = t.get::<Option<String>>("logo")?.filter(|x| !x.trim().is_empty());
    let mut presets = vec![];
    if let Some(ps) = t.get::<Option<Table>>("presets")? {
        for p in ps.sequence_values::<Table>() {
            let p = p?;
            let n = p.get::<Option<Texto>>("nombre")?.ok_or_else(|| mlua::Error::runtime(tr("falta «nombre»")))?;
            let mut v = BTreeMap::new();
            for k in &controles {
                if let Some(x) = p.get::<Option<f32>>(k.clave.as_str())? {
                    v.insert(k.clave.clone(), x.clamp(k.min, k.max));
                }
            }
            presets.push((n, v));
        }
    }
    let api = t.get::<Option<u32>>("api")?.unwrap_or(1);
    if api > API {
        return Err(mlua::Error::runtime(tf("pide una versión más nueva del programa (api {})", &[&api])));
    }
    let vacio = || Texto { es: String::new(), en: None };
    let muestras = match t.get::<Value>("muestras")? {
        Value::Nil => Muestras::Archivo,
        Value::String(s) => match &*s.to_str()? {
            "archivo" => Muestras::Archivo,
            "pads" => Muestras::Pads,
            "cortes" => Muestras::Cortes,
            x => return Err(mlua::Error::runtime(tf("muestras «{}» no existe: usa archivo, pads, cortes o una lista", &[&x]))),
        },
        Value::Table(l) => Muestras::Propias(
            l.sequence_values::<Table>()
                .take(256)
                .map(|g| {
                    let g = g?;
                    let n = |k: &str, v: u8| -> mlua::Result<u8> { Ok(g.get::<Option<u8>>(k)?.unwrap_or(v).min(127)) };
                    let nota = n("nota", 60)?;
                    Ok(Grabacion {
                        archivo: g.get::<Option<String>>("archivo")?.ok_or_else(|| mlua::Error::runtime(tr("a una muestra le falta «archivo»")))?,
                        nota,
                        teclas: (n("desde", 0)?, n("hasta", 127)?),
                        fuerzas: (n("fuerza_min", 0)?, n("fuerza_max", 127)?),
                    })
                })
                .collect::<mlua::Result<_>>()?,
        ),
        v => return Err(mlua::Error::runtime(tf("muestras «{}» no existe: usa archivo, pads, cortes o una lista", &[&v.type_name()]))),
    };
    Ok(Ficha {
        nombre,
        tipo,
        grupo,
        controles,
        logo,
        presets,
        autor: t.get::<Option<String>>("autor")?.unwrap_or_default(),
        version: t.get::<Option<String>>("version")?.unwrap_or_default(),
        descripcion: t.get::<Option<Texto>>("descripcion")?.unwrap_or_else(vacio),
        muestras,
        huella: t.get::<Option<String>>("huella")?,
        oculto: t.get::<Option<bool>>("oculto")?.unwrap_or(false),
        sostiene: t.get::<Option<bool>>("sostiene")?.unwrap_or(false),
        vivo: t.get::<Option<Function>>("vivo")?.is_some(),
        medidas: t.get::<Option<Vec<String>>>("medidas")?.unwrap_or_default().into_iter().take(MAX_MEDIDAS).collect(),
    })
}

/// `incluir("otro")`: corre otro `.lua` de la carpeta del guion y da lo que devuelve (funciones, tablas…). Solo ve
/// los de su carpeta, ya leídos: el guion sigue sin poder abrir archivos.
pub fn poner_incluir(lua: &Lua, a: &Archivos) -> mlua::Result<()> {
    let mut luas = a.lua.clone();
    // Las piezas para armar ventanas de aparato, al alcance de todos.
    luas.entry("equipo".into()).or_insert_with(|| include_str!("../efectos/equipo.lua").into());
    let hondo = Rc::new(std::cell::Cell::new(0u8));
    let f = lua.create_function(move |lua, nombre: String| {
        let n = nombre.trim_end_matches(".lua").to_string();
        let fuente = luas.get(&n).ok_or_else(|| mlua::Error::runtime(tf("no hay «{}.lua» en la carpeta del guion", &[&n])))?;
        if hondo.get() >= 16 {
            return Err(mlua::Error::runtime(tr("«incluir» se llama a sí mismo sin fin")));
        }
        hondo.set(hondo.get() + 1);
        let r = lua.load(fuente.as_str()).set_name(format!("={n}")).set_mode(ChunkMode::Text).eval::<MultiValue>();
        hondo.set(hondo.get() - 1);
        r
    })?;
    lua.globals().set("incluir", f)
}

fn tabla(lua: &Lua, fuente: &str) -> mlua::Result<Table> {
    lua.load(fuente).set_name("=guion").set_mode(ChunkMode::Text).eval::<Table>()
}

/// El mensaje sin el rastro interno y con el renglón donde pasó.
pub fn texto(e: mlua::Error) -> String {
    let todo = e.to_string();
    let causa = match &e {
        mlua::Error::CallbackError { cause, .. } => cause.to_string(),
        _ => todo.clone(),
    };
    let s = causa.split("\nstack traceback").next().unwrap_or("").trim().trim_start_matches("runtime error: ").trim_start_matches("syntax error: ");
    if s.starts_with("guion:") {
        return s.replacen("guion:", tr("línea "), 1);
    }
    let linea: Option<String> = todo.split("guion:").nth(1).map(|r| r.chars().take_while(|c| c.is_ascii_digit()).collect());
    match linea.filter(|l| !l.is_empty()) {
        Some(l) => tf("línea {}: {}", &[&l, &s]),
        None => s.to_string(),
    }
}

/// Arma el sonido de un instrumento o efecto. De una herramienta solo da la ficha.
pub fn armar(fuente: &str, sr: f32) -> Result<(Ficha, Option<Receta>), String> {
    armar_con(fuente, sr, &Archivos::default())
}

/// Igual, con lo que trae su carpeta (otros `.lua` para `incluir`).
pub fn armar_con(fuente: &str, sr: f32, a: &Archivos) -> Result<(Ficha, Option<Receta>), String> {
    armar_(fuente, sr, a).map_err(texto)
}

fn armar_(fuente: &str, sr: f32, a: &Archivos) -> mlua::Result<(Ficha, Option<Receta>)> {
    let lua = lua_cerrado()?;
    poner_incluir(&lua, a)?;
    let g: Grafo = Rc::new(RefCell::new(vec![]));
    registrar(&lua, &g)?;
    let tablas = Rc::new(RefCell::new(vec![]));
    let medidas = Rc::new(RefCell::new(vec![]));
    registrar_propias(&lua, &g, &tablas, &medidas)?;
    let t = tabla(&lua, fuente)?;
    let f = ficha_de(&t)?;
    if f.tipo == Tipo::Herramienta {
        t.get::<Option<Function>>("notas")?.ok_or_else(|| mlua::Error::runtime(tr("falta la función «notas»")))?;
        return Ok((f, None));
    }
    let sonido = match t.get::<Option<Function>>("sonido")? {
        Some(f) => f,
        // Solo en vivo: lo que entra pasa tal cual (o nada, si es instrumento) y de ahí sigue `vivo`.
        None if f.vivo && f.tipo == Tipo::Efecto => lua.load("local e = ... return e.izq, e.der").into_function()?,
        None if f.vivo => lua.load("return 0").into_function()?,
        None => return Err(mlua::Error::runtime(tr("falta la función «sonido»"))),
    };
    let c = lua.create_table()?;
    for (i, k) in f.controles.iter().enumerate() {
        c.set(k.clave.as_str(), poner(&g, Nodo::Control(i as u16))?)?;
    }
    let n = lua.create_table()?;
    if matches!(f.tipo, Tipo::Efecto | Tipo::Notas) {
        n.set("izq", poner(&g, Nodo::Entrada(0))?)?;
        n.set("der", poner(&g, Nodo::Entrada(1))?)?;
        n.set("otra_izq", poner(&g, Nodo::Entrada(2))?)?;
        n.set("otra_der", poner(&g, Nodo::Entrada(3))?)?;
    } else {
        n.set("hz", poner(&g, Nodo::Hz)?)?;
        n.set("fuerza", poner(&g, Nodo::Fuerza)?)?;
        n.set("azar", poner(&g, Nodo::Azar)?)?;
    }
    let sale = sonido.call::<MultiValue>((n, c))?;
    let mut it = sale.into_iter();
    let izq = match it.next() {
        None | Some(Value::Nil) => return Err(mlua::Error::runtime(tr("«sonido» no devolvió nada: termina con return"))),
        Some(v) => idx(&g, Arg::from_lua(v, &lua)?)?,
    };
    let der = match it.next() {
        None | Some(Value::Nil) => izq,
        Some(v) => idx(&g, Arg::from_lua(v, &lua)?)?,
    };
    let voces = if f.tipo == Tipo::Efecto { 1 } else { t.get::<Option<usize>>("voces")?.unwrap_or(16).clamp(1, 32) };
    let nodos = g.borrow();
    let mut r = Receta::nueva(&nodos, [izq, der], f.controles.clone(), voces, sr);
    r.tablas = tablas.take();
    r.nombrar_medidas(&medidas.borrow());
    r.vivo = f.vivo;
    r.sostiene = f.sostiene;
    r.medidas_vivo = f.medidas.clone();
    if r.memoria * voces > MAX_MEMORIA {
        return Err(mlua::Error::runtime(tr("el sonido pide demasiada memoria: baja las voces o los retardos")));
    }
    Ok((f, Some(r)))
}

/// Corre una herramienta sobre las notas de un canal. Los tiempos van en pasos (semicorcheas).
pub fn herramienta(fuente: &str, notas: &[Nota], largo: u32, valores: &BTreeMap<String, f32>) -> Result<Vec<Nota>, String> {
    herramienta_con(fuente, &Archivos::default(), notas, largo, valores)
}

/// Igual, con lo que trae su carpeta.
pub fn herramienta_con(fuente: &str, a: &Archivos, notas: &[Nota], largo: u32, valores: &BTreeMap<String, f32>) -> Result<Vec<Nota>, String> {
    let f = || -> mlua::Result<Vec<Nota>> {
        let lua = lua_cerrado()?;
        poner_incluir(&lua, a)?;
        let t = tabla(&lua, fuente)?;
        let fi = ficha_de(&t)?;
        let c = lua.create_table()?;
        for k in &fi.controles {
            c.set(k.clave.as_str(), valores.get(&k.clave).copied().unwrap_or(k.valor))?;
        }
        c.set("largo", largo as f32 / PASO as f32)?;
        let l = lua.create_table()?;
        for n in notas {
            let x = lua.create_table()?;
            x.set("paso", n.tick as f32 / PASO as f32)?;
            x.set("largo", n.dur as f32 / PASO as f32)?;
            x.set("tono", n.tono)?;
            x.set("fuerza", n.fuerza)?;
            l.push(x)?;
        }
        let sale = t.get::<Function>("notas")?.call::<Table>((l, c))?;
        let mut v = vec![];
        for x in sale.sequence_values::<Table>() {
            let x = x?;
            let (paso, dur, tono, fuerza): (f32, f32, f32, f32) = (x.get("paso")?, x.get("largo")?, x.get("tono")?, x.get("fuerza")?);
            if ![paso, dur, tono, fuerza].iter().all(|v| v.is_finite()) || v.len() >= 20_000 {
                return Err(mlua::Error::runtime(tr("la herramienta devolvió notas inválidas")));
            }
            v.push(Nota {
                tick: (paso * PASO as f32).round().clamp(0.0, 1e7) as u32,
                dur: (dur * PASO as f32).round().clamp(1.0, 1e6) as u32,
                tono: tono.round().clamp(0.0, 127.0) as u8,
                fuerza: fuerza.clamp(0.01, 1.0),
            });
        }
        v.sort_by_key(|n| (n.tick, n.tono));
        Ok(v)
    };
    f().map_err(texto)
}
