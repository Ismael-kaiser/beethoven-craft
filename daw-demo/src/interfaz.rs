// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Interfaces hechas en Lua. El guion trae `interfaz = { ancho, alto, dibujar = function(d, c, m) … end }`; `dibujar`
//! corre en cada cuadro de la ventana (nunca en el audio) y solo anota qué pintar: lo demás lo hace el programa.
use crate::idioma::t as tr;
use crate::lua::{ficha_de, lua_cerrado, poner_incluir, presupuesto, texto};
use crate::paquete::Archivos;
use mlua::{Function, Lua, RegistryKey, Table, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

/// Tope de piezas por cuadro.
const MAX_PIEZAS: usize = 4000;
const MAX_PASOS: u32 = 3_000_000;

pub type Color = [u8; 4];

#[derive(Clone, Debug, PartialEq)]
pub enum Pieza {
    Caja {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        a: Color,
        b: Color,
    },
    Borde {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        c: Color,
        g: f32,
    },
    Circulo {
        x: f32,
        y: f32,
        r: f32,
        c: Color,
    },
    Aro {
        x: f32,
        y: f32,
        r: f32,
        c: Color,
        g: f32,
    },
    Linea {
        v: Vec<[f32; 2]>,
        c: Color,
        g: f32,
    },
    Poligono {
        v: Vec<[f32; 2]>,
        c: Color,
    },
    Texto {
        x: f32,
        y: f32,
        s: String,
        n: f32,
        c: Color,
        donde: u8,
    },
    Metal {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
    },
    Hueco {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
    },
    Vidrio {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        c: Color,
    },
    Tornillo {
        x: f32,
        y: f32,
        r: f32,
    },
    Perilla {
        x: f32,
        y: f32,
        r: f32,
        clave: String,
        c: Color,
    },
    Tirador {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        clave: String,
        c: Color,
    },
    Boton {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        s: String,
        clave: String,
        c: Color,
    },
    /// Una imagen PNG de la carpeta del guion, estirada al rectángulo, con su opacidad (0 a 1).
    Imagen {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        archivo: String,
        a: f32,
    },
    /// Se arrastra en las dos direcciones: cada una mueve un control.
    Pad {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        cx: String,
        cy: String,
    },
}

/// Lo que el guion sabe del momento.
#[derive(Clone, Copy, Default)]
pub struct Momento {
    pub tiempo: f64,
    pub nivel: f32,
    pub tocando: bool,
    pub raton: Option<[f32; 2]>,
}

pub struct Interfaz {
    lua: Lua,
    dibujar: RegistryKey,
    piezas: Rc<RefCell<Vec<Pieza>>>,
    pub ancho: f32,
    pub alto: f32,
    pub claves: Vec<String>,
    /// Si el guion falló, ya no se vuelve a correr.
    pub error: Option<String>,
    /// Lo que trae su carpeta: de aquí salen las imágenes.
    pub archivos: Arc<Archivos>,
}

fn color(s: Option<String>) -> mlua::Result<Color> {
    let Some(s) = s else { return Ok([255, 255, 255, 255]) };
    let h = s.trim_start_matches('#');
    let n = (h.is_ascii() && (h.len() == 6 || h.len() == 8)).then(|| u32::from_str_radix(h, 16).ok()).flatten();
    let n = n.ok_or_else(|| mlua::Error::runtime(crate::idioma::tf("«{}» no es un color: usa #rrggbb", &[&s])))?;
    Ok(if h.len() == 6 { [(n >> 16) as u8, (n >> 8) as u8, n as u8, 255] } else { [(n >> 24) as u8, (n >> 16) as u8, (n >> 8) as u8, n as u8] })
}

fn puntos(t: Table) -> mlua::Result<Vec<[f32; 2]>> {
    let v: Vec<f32> = t.sequence_values::<f32>().collect::<mlua::Result<_>>()?;
    if v.iter().any(|x| !x.is_finite()) || v.len() > 4000 {
        return Err(mlua::Error::runtime(tr("puntos inválidos")));
    }
    Ok(v.chunks_exact(2).map(|c| [c[0], c[1]]).collect())
}

/// Nada si el guion no trae interfaz.
pub fn cargar(fuente: &str) -> Result<Option<Interfaz>, String> {
    cargar_con(fuente, Arc::new(Archivos::default()))
}

/// Igual, con lo que trae su carpeta (imágenes y otros `.lua`).
pub fn cargar_con(fuente: &str, archivos: Arc<Archivos>) -> Result<Option<Interfaz>, String> {
    cargar_(fuente, archivos).map_err(texto)
}

fn cargar_(fuente: &str, archivos: Arc<Archivos>) -> mlua::Result<Option<Interfaz>> {
    let lua = lua_cerrado()?;
    poner_incluir(&lua, &archivos)?;
    let t: Table = lua.load(fuente).set_name("=guion").set_mode(mlua::ChunkMode::Text).eval()?;
    let Some(i) = t.get::<Option<Table>>("interfaz")? else { return Ok(None) };
    let claves: Vec<String> = ficha_de(&t)?.controles.into_iter().map(|c| c.clave).collect();
    let dibujar: Function =
        i.get::<Option<Function>>("dibujar")?.ok_or_else(|| mlua::Error::runtime(tr("a la interfaz le falta la función «dibujar»")))?;
    let lado = |k: &str, v: f32| -> mlua::Result<f32> { Ok(i.get::<Option<f32>>(k)?.filter(|x| x.is_finite()).unwrap_or(v).clamp(120.0, 1400.0)) };
    let (ancho, alto) = (lado("ancho", 420.0)?, lado("alto", 260.0)?);
    let dibujar = lua.create_registry_value(dibujar)?;
    let piezas = Rc::new(RefCell::new(vec![]));
    let d = lua.create_table()?;
    macro_rules! pieza {
        ($n:literal, ($($a:pat_param),*) : $t:ty => $e:expr) => {{
            let p = piezas.clone();
            let validas = claves.clone();
            let _ = &validas;
            d.set($n, lua.create_function(move |_, ($($a),*): $t| {
                let hay = |k: &String| -> mlua::Result<()> {
                    if validas.contains(k) { Ok(()) } else { Err(mlua::Error::runtime(crate::idioma::tf("no hay un control «{}»", &[k]))) }
                };
                let _ = &hay;
                let mut p = p.borrow_mut();
                if p.len() >= MAX_PIEZAS {
                    return Err(mlua::Error::runtime(tr("la interfaz dibuja demasiado")));
                }
                let f = || -> mlua::Result<Pieza> { $e(&hay) };
                p.push(f()?);
                Ok(())
            })?)?;
        }};
    }
    type H<'a> = &'a dyn Fn(&String) -> mlua::Result<()>;
    type S = Option<String>;
    pieza!("caja", (x, y, w, h, a, r) : (f32, f32, f32, f32, S, Option<f32>) => |_: H| { let a = color(a.clone())?; Ok(Pieza::Caja { x, y, w, h, r: r.unwrap_or(0.0), a, b: a }) });
    pieza!("degradado", (x, y, w, h, a, b, r) : (f32, f32, f32, f32, S, S, Option<f32>) => |_: H| Ok(Pieza::Caja { x, y, w, h, r: r.unwrap_or(0.0), a: color(a.clone())?, b: color(b.clone())? }));
    pieza!("borde", (x, y, w, h, c, g, r) : (f32, f32, f32, f32, S, Option<f32>, Option<f32>) => |_: H| Ok(Pieza::Borde { x, y, w, h, r: r.unwrap_or(0.0), c: color(c.clone())?, g: g.unwrap_or(1.0) }));
    pieza!("circulo", (x, y, r, c) : (f32, f32, f32, S) => |_: H| Ok(Pieza::Circulo { x, y, r, c: color(c.clone())? }));
    pieza!("aro", (x, y, r, c, g) : (f32, f32, f32, S, Option<f32>) => |_: H| Ok(Pieza::Aro { x, y, r, c: color(c.clone())?, g: g.unwrap_or(1.0) }));
    pieza!("linea", (v, c, g) : (Table, S, Option<f32>) => |_: H| Ok(Pieza::Linea { v: puntos(v.clone())?, c: color(c.clone())?, g: g.unwrap_or(1.0) }));
    pieza!("poligono", (v, c) : (Table, S) => |_: H| Ok(Pieza::Poligono { v: puntos(v.clone())?, c: color(c.clone())? }));
    pieza!("texto", (x, y, s, n, c, donde) : (f32, f32, String, Option<f32>, S, S) => |_: H| {
        let donde = match donde.as_deref() { Some("centro") => 1, Some("derecha") => 2, _ => 0 };
        Ok(Pieza::Texto { x, y, s: s.chars().take(300).collect(), n: n.unwrap_or(14.0).clamp(6.0, 120.0), c: color(c.clone())?, donde })
    });
    pieza!("metal", (x, y, w, h, r) : (f32, f32, f32, f32, Option<f32>) => |_: H| Ok(Pieza::Metal { x, y, w, h, r: r.unwrap_or(6.0) }));
    pieza!("hueco", (x, y, w, h, r) : (f32, f32, f32, f32, Option<f32>) => |_: H| Ok(Pieza::Hueco { x, y, w, h, r: r.unwrap_or(4.0) }));
    pieza!("vidrio", (x, y, w, h, c, r) : (f32, f32, f32, f32, S, Option<f32>) => |_: H| Ok(Pieza::Vidrio { x, y, w, h, r: r.unwrap_or(4.0), c: color(c.clone())? }));
    pieza!("tornillo", (x, y, r) : (f32, f32, Option<f32>) => |_: H| Ok(Pieza::Tornillo { x, y, r: r.unwrap_or(4.0) }));
    pieza!("perilla", (x, y, r, clave, c) : (f32, f32, f32, String, S) => |hay: H| { hay(&clave)?; Ok(Pieza::Perilla { x, y, r: r.clamp(8.0, 200.0), clave: clave.clone(), c: color(Some(c.clone().unwrap_or("#ffd95e".into())))? }) });
    pieza!("tirador", (x, y, w, h, clave, c) : (f32, f32, f32, f32, String, S) => |hay: H| { hay(&clave)?; Ok(Pieza::Tirador { x, y, w, h, clave: clave.clone(), c: color(Some(c.clone().unwrap_or("#ffd95e".into())))? }) });
    pieza!("boton", (x, y, w, h, s, clave, c) : (f32, f32, f32, f32, String, String, S) => |hay: H| { hay(&clave)?; Ok(Pieza::Boton { x, y, w, h, s: s.clone(), clave: clave.clone(), c: color(Some(c.clone().unwrap_or("#ffd95e".into())))? }) });
    pieza!("pad", (x, y, w, h, cx, cy) : (f32, f32, f32, f32, String, String) => |hay: H| { hay(&cx)?; hay(&cy)?; Ok(Pieza::Pad { x, y, w, h, cx: cx.clone(), cy: cy.clone() }) });
    {
        // Solo las imágenes de su carpeta.
        let nombres: Vec<String> = archivos.imagenes.keys().cloned().collect();
        pieza!("imagen", (x, y, w, h, archivo, a) : (f32, f32, f32, f32, String, Option<f32>) => |_: H| {
            if !nombres.contains(&archivo) {
                return Err(mlua::Error::runtime(crate::idioma::tf("no hay una imagen «{}» en la carpeta del guion", &[&archivo])));
            }
            Ok(Pieza::Imagen { x, y, w, h, archivo: archivo.clone(), a: a.unwrap_or(1.0).clamp(0.0, 1.0) })
        });
    }
    lua.globals().set("__d", d)?;
    Ok(Some(Interfaz { lua, dibujar, piezas, ancho, alto, claves, error: None, archivos }))
}

impl Interfaz {
    /// Lo que hay que pintar en este cuadro.
    pub fn cuadro(&mut self, valores: &BTreeMap<String, f32>, m: &Momento) -> Vec<Pieza> {
        if self.error.is_some() {
            return vec![];
        }
        self.piezas.borrow_mut().clear();
        let f = || -> mlua::Result<()> {
            presupuesto(&self.lua, MAX_PASOS)?;
            let d: Table = self.lua.globals().get("__d")?;
            let c = self.lua.create_table()?;
            for (k, v) in valores {
                c.set(k.as_str(), *v)?;
            }
            let q = self.lua.create_table()?;
            q.set("tiempo", m.tiempo)?;
            q.set("nivel", m.nivel)?;
            q.set("tocando", m.tocando)?;
            q.set("idioma", crate::idioma::actual().codigo())?;
            if let Some([x, y]) = m.raton {
                q.set("x", x)?;
                q.set("y", y)?;
            }
            self.lua.registry_value::<Function>(&self.dibujar)?.call::<Value>((d, c, q))?;
            Ok(())
        };
        if let Err(e) = f() {
            self.error = Some(texto(e));
        }
        std::mem::take(&mut *self.piezas.borrow_mut())
    }
}
