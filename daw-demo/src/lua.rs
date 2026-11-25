// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Lua: arma los sonidos y corre las herramientas de notas. Nunca corre en el hilo de audio.
use crate::idioma::{Texto, t as tr, tf};
use crate::modelo::{Nota, PASO};
use crate::paquete::Archivos;
use crate::receta::{Banda, Control, Filtro, MAX_RETARDO, N, Nodo, Onda, Receta};
use mlua::{
    ChunkMode, FromLua, Function, HookTriggers, Lua, LuaOptions, MetaMethod, MultiValue, StdLib, Table, UserData, UserDataMethods, Value, VmState,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

const MAX_NODOS: usize = 2000;
/// Números que pueden guardar todas las voces juntas (64 MB).
const MAX_MEMORIA: usize = 16 << 20;
const MAX_PASOS: u32 = 20_000_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tipo {
    Instrumento,
    Efecto,
    Herramienta,
}

#[derive(Clone, Debug)]
pub struct Ficha {
    pub nombre: Texto,
    pub tipo: Tipo,
    pub grupo: Texto,
    pub controles: Vec<Control>,
    /// Imagen de su carpeta que hace de logo («logo.png»).
    pub logo: Option<String>,
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
    Ok(Ficha { nombre, tipo, grupo, controles, logo })
}

/// `incluir("otro")`: corre otro `.lua` de la carpeta del guion y da lo que devuelve (funciones, tablas…). Solo ve
/// los de su carpeta, ya leídos: el guion sigue sin poder abrir archivos.
pub fn poner_incluir(lua: &Lua, a: &Archivos) -> mlua::Result<()> {
    let luas = a.lua.clone();
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
    let t = tabla(&lua, fuente)?;
    let f = ficha_de(&t)?;
    if f.tipo == Tipo::Herramienta {
        t.get::<Option<Function>>("notas")?.ok_or_else(|| mlua::Error::runtime(tr("falta la función «notas»")))?;
        return Ok((f, None));
    }
    let sonido = t.get::<Option<Function>>("sonido")?.ok_or_else(|| mlua::Error::runtime(tr("falta la función «sonido»")))?;
    let c = lua.create_table()?;
    for (i, k) in f.controles.iter().enumerate() {
        c.set(k.clave.as_str(), poner(&g, Nodo::Control(i as u16))?)?;
    }
    let n = lua.create_table()?;
    if f.tipo == Tipo::Efecto {
        n.set("izq", poner(&g, Nodo::Entrada(0))?)?;
        n.set("der", poner(&g, Nodo::Entrada(1))?)?;
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
    let r = Receta::nueva(&nodos, [izq, der], f.controles.clone(), voces, sr);
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
