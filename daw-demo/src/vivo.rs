// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Lua mientras suena. Un guion con `vivo = function(v) … end` corre en el hilo de audio, muestra por muestra o por
//! bloques. Para que no corte el sonido: su memoria sale de una reserva propia hecha de antemano, cada bloque tiene tope
//! de pasos, y si falla se apaga solo y el sonido pasa igual.
use crate::biblioteca::Medidas;
use crate::idioma::{t as tr, tf};
use crate::lua::{lua_cerrado, poner_incluir, texto};
use crate::paquete::Archivos;
use crate::plugin::Evento;
use crate::receta::{BLOQUE, Buf};
use crossbeam_channel::{Receiver, Sender, bounded};
use mlua::{ChunkMode, Function, HookTriggers, Lua, Table, Value, VmState, ffi};
use std::cell::{Cell, UnsafeCell};
use std::collections::BTreeMap;
use std::ffi::{CString, c_int, c_void};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering::*};

/// La reserva de cada guion. Solo se usa de verdad lo que toca.
const RESERVA: usize = 48 << 20;
/// Pasos de Lua por bloque. Pasarse es cortar el sonido: se apaga.
const PASOS: u32 = 900_000;
const MAX_FFT: usize = 4096;
/// Notas que puede mandar un guion en cada bloque.
const NOTAS: usize = 64;

/// Lo que se sabe del momento, para el guion.
#[derive(Clone, Copy, Default)]
pub struct Momento {
    pub bpm: f32,
    /// En negras.
    pub pos: f64,
    pub tocando: bool,
}

/// Una nota que manda un guion.
#[derive(Clone, Copy, Debug)]
pub struct Emitida {
    pub tono: u8,
    pub fuerza: f32,
    pub segundos: f32,
}

/// Reparte la reserva en tamaños de potencia de dos. Nunca pide memoria al sistema.
struct Reserva {
    base: *mut u8,
    usado: usize,
    libres: [*mut u8; 32],
    antes: ffi::lua_Alloc,
    antes_ud: *mut c_void,
}

fn forma() -> std::alloc::Layout {
    std::alloc::Layout::from_size_align(RESERVA, 64).expect("forma de la reserva")
}

impl Reserva {
    fn clase(n: usize) -> usize {
        n.max(32).next_power_of_two().trailing_zeros() as usize
    }

    fn mia(&self, p: *mut u8) -> bool {
        (self.base as usize..self.base as usize + RESERVA).contains(&(p as usize))
    }

    unsafe fn pedir(&mut self, n: usize) -> *mut u8 {
        let k = Self::clase(n);
        let l = self.libres[k];
        if !l.is_null() {
            self.libres[k] = unsafe { *(l as *mut *mut u8) };
            return l;
        }
        if self.usado + (1 << k) > RESERVA {
            return std::ptr::null_mut();
        }
        let p = unsafe { self.base.add(self.usado) };
        self.usado += 1 << k;
        p
    }

    unsafe fn soltar(&mut self, p: *mut u8, n: usize) {
        let k = Self::clase(n);
        unsafe { *(p as *mut *mut u8) = self.libres[k] };
        self.libres[k] = p;
    }
}

unsafe extern "C" fn repartir(ud: *mut c_void, p: *mut c_void, viejo: usize, nuevo: usize) -> *mut c_void {
    unsafe {
        let r = &mut *(ud as *mut Reserva);
        let p = p as *mut u8;
        if nuevo == 0 {
            if !p.is_null() {
                if r.mia(p) { r.soltar(p, viejo) } else { _ = (r.antes)(r.antes_ud, p as _, viejo, 0) }
            }
            return std::ptr::null_mut();
        }
        if !p.is_null() && r.mia(p) && Reserva::clase(viejo) == Reserva::clase(nuevo) {
            return p as _;
        }
        let q = r.pedir(nuevo);
        if q.is_null() || p.is_null() {
            return q as _;
        }
        std::ptr::copy_nonoverlapping(p, q, viejo.min(nuevo));
        if r.mia(p) {
            r.soltar(p, viejo)
        } else {
            _ = (r.antes)(r.antes_ud, p as _, viejo, 0)
        }
        q as _
    }
}

/// `fft(re, im)` o `fft(re, im, true)` para regresar.
unsafe extern "C-unwind" fn fft(l: *mut ffi::lua_State) -> c_int {
    unsafe {
        ffi::luaL_checktype(l, 1, ffi::LUA_TTABLE);
        ffi::luaL_checktype(l, 2, ffi::LUA_TTABLE);
        let atras = ffi::lua_toboolean(l, 3) != 0;
        let n = ffi::lua_rawlen(l, 1) as usize;
        if !n.is_power_of_two() || !(2..=MAX_FFT).contains(&n) || (ffi::lua_rawlen(l, 2) as usize) < n {
            return ffi::luaL_error(l, c"fft: 2 a 4096, potencia de dos".as_ptr());
        }
        let (mut re, mut im) = ([0.0f32; MAX_FFT], [0.0f32; MAX_FFT]);
        for i in 0..n {
            ffi::lua_rawgeti(l, 1, i as i64 + 1);
            ffi::lua_rawgeti(l, 2, i as i64 + 1);
            re[i] = ffi::lua_tonumber(l, -2) as f32;
            im[i] = ffi::lua_tonumber(l, -1) as f32;
            ffi::lua_pop(l, 2);
        }
        let mut j = 0;
        for i in 1..n {
            let mut b = n >> 1;
            while j & b != 0 {
                j ^= b;
                b >>= 1;
            }
            j |= b;
            if i < j {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut m = 2;
        while m <= n {
            let ang = if atras { 1.0 } else { -1.0 } * std::f64::consts::TAU / m as f64;
            let (ps, pc) = ang.sin_cos();
            let (mut c, mut s) = (1.0f64, 0.0f64);
            for k in 0..m / 2 {
                let (cf, sf) = (c as f32, s as f32);
                let mut i = k;
                while i < n {
                    let q = i + m / 2;
                    let (x, y) = (re[q] * cf - im[q] * sf, re[q] * sf + im[q] * cf);
                    (re[q], im[q]) = (re[i] - x, im[i] - y);
                    re[i] += x;
                    im[i] += y;
                    i += m;
                }
                (c, s) = (c * pc - s * ps, s * pc + c * ps);
            }
            m <<= 1;
        }
        let g = if atras { 1.0 / n as f32 } else { 1.0 };
        for i in 0..n {
            ffi::lua_pushnumber(l, (re[i] * g) as f64);
            ffi::lua_rawseti(l, 1, i as i64 + 1);
            ffi::lua_pushnumber(l, (im[i] * g) as f64);
            ffi::lua_rawseti(l, 2, i as i64 + 1);
        }
        0
    }
}

/// Lo que las funciones de C comparten con el guion. Sin pedir memoria.
struct Puente {
    emitidas: Vec<Emitida>,
    graba: bool,
    nombres: Vec<Vec<u8>>,
    medidas: Option<Arc<Medidas>>,
    base: usize,
}

struct Suelta(*mut Puente);

impl Drop for Suelta {
    fn drop(&mut self) {
        drop(unsafe { Box::from_raw(self.0) });
    }
}

unsafe fn puente<'a>(l: *mut ffi::lua_State) -> &'a mut Puente {
    unsafe { &mut *(ffi::lua_touserdata(l, ffi::lua_upvalueindex(1)) as *mut Puente) }
}

/// `toca(tono, fuerza, segundos)`
unsafe extern "C-unwind" fn toca(l: *mut ffi::lua_State) -> c_int {
    unsafe {
        let (tono, fuerza, seg) = (ffi::luaL_checknumber(l, 1), ffi::luaL_checknumber(l, 2), ffi::luaL_checknumber(l, 3));
        let p = puente(l);
        if p.emitidas.len() < NOTAS && tono.is_finite() && fuerza > 0.0 && seg.is_finite() {
            p.emitidas.push(Emitida { tono: tono.clamp(0.0, 127.0) as u8, fuerza: fuerza.min(1.0) as f32, segundos: seg.clamp(0.005, 60.0) as f32 });
        }
        0
    }
}

/// `mide("nombre", valor)`
unsafe extern "C-unwind" fn mide(l: *mut ffi::lua_State) -> c_int {
    unsafe {
        let v = ffi::luaL_checknumber(l, 2) as f32;
        let mut n = 0;
        let s = ffi::luaL_checklstring(l, 1, &mut n);
        let s = std::slice::from_raw_parts(s as *const u8, n);
        let p = puente(l);
        let k = p.nombres.iter().position(|x| x[..] == *s).map(|k| k + p.base).filter(|k| *k < crate::lua::MAX_MEDIDAS);
        if let (Some(k), Some(a)) = (k, &p.medidas) {
            a[k].store(v.to_bits(), Relaxed);
        }
        0
    }
}

/// `graba(true)`
unsafe extern "C-unwind" fn graba(l: *mut ffi::lua_State) -> c_int {
    unsafe {
        puente(l).graba = ffi::lua_toboolean(l, 1) != 0;
        0
    }
}

struct Dentro {
    bloque: Option<Function>,
    nota: Option<Function>,
    suelta: Option<Function>,
    rueda: Option<Function>,
    mando: Option<Function>,
    tablas: [Table; 4],
    c: Table,
    m: Table,
    claves: Vec<CString>,
    /// Lo último que se le puso a cada control.
    puestos: Vec<f32>,
    cuenta: Rc<Cell<u32>>,
    puente: Suelta,
    grabado: Sender<([[f32; 2]; BLOQUE], usize)>,
    fallas: Sender<String>,
    muerto: bool,
    lua: Lua,
    // La reserva se suelta después que Lua.
    _reserva: Guarda,
}

struct Guarda(*mut Reserva);

impl Drop for Guarda {
    fn drop(&mut self) {
        unsafe {
            let r = Box::from_raw(self.0);
            std::alloc::dealloc(r.base, forma());
        }
    }
}

pub struct Vivo {
    celda: UnsafeCell<Dentro>,
    ocupado: AtomicBool,
    /// Muestras que se tarda.
    pub latencia: u32,
    fallas: Receiver<String>,
    grabado: Receiver<([[f32; 2]; BLOQUE], usize)>,
}

// Se arma y se suelta en la ventana; suena en el audio. `ocupado` cuida que nunca lo toquen dos hilos a la vez.
unsafe impl Send for Vivo {}
unsafe impl Sync for Vivo {}

const M_POS: &std::ffi::CStr = c"pos";
const M_BPM: &std::ffi::CStr = c"bpm";
const M_TOCANDO: &std::ffi::CStr = c"tocando";

impl Vivo {
    /// En el hilo de la ventana.
    pub fn nuevo(
        fuente: &str,
        a: &Archivos,
        sr: f32,
        claves: &[String],
        datos: &BTreeMap<String, String>,
        medidas: (&[String], Option<Arc<Medidas>>, usize),
    ) -> Result<Vivo, String> {
        Self::armar(fuente, a, sr, claves, datos, medidas).map_err(texto)
    }

    fn armar(
        fuente: &str,
        a: &Archivos,
        sr: f32,
        claves: &[String],
        datos: &BTreeMap<String, String>,
        (nombres, arcos, base): (&[String], Option<Arc<Medidas>>, usize),
    ) -> mlua::Result<Vivo> {
        // Antes que Lua: si algo falla aquí abajo, se suelta después que él.
        let reserva = unsafe {
            let b = std::alloc::alloc_zeroed(forma());
            if b.is_null() {
                return Err(mlua::Error::runtime("memoria"));
            }
            Guarda(Box::into_raw(Box::new(Reserva {
                base: b,
                usado: 0,
                libres: [std::ptr::null_mut(); 32],
                antes: repartir,
                antes_ud: std::ptr::null_mut(),
            })))
        };
        let lua = lua_cerrado()?;
        unsafe {
            let r = reserva.0;
            lua.exec_raw::<()>((), |l| {
                let mut ud = std::ptr::null_mut();
                (*r).antes = ffi::lua_getallocf(l, &mut ud);
                (*r).antes_ud = ud;
                ffi::lua_setallocf(l, repartir, r as _);
            })?;
        }
        poner_incluir(&lua, a)?;
        let g = lua.globals();
        g.set("fft", unsafe { lua.create_c_function(fft)? })?;
        lua.load(include_str!("vivo.lua")).set_name("=vivo").set_mode(ChunkMode::Text).exec()?;
        let puente = Box::into_raw(Box::new(Puente {
            emitidas: Vec::with_capacity(NOTAS),
            graba: false,
            nombres: nombres.iter().map(|n| n.as_bytes().to_vec()).collect(),
            medidas: arcos,
            base,
        }));
        let puente_ = Suelta(puente);
        unsafe {
            lua.exec_raw::<()>((), |l| {
                for (n, f) in [(c"toca", toca as ffi::lua_CFunction), (c"mide", mide), (c"graba", graba)] {
                    ffi::lua_pushlightuserdata(l, puente as *mut c_void);
                    ffi::lua_pushcclosure(l, f, 1);
                    ffi::lua_setglobal(l, n.as_ptr());
                }
            })?;
        }

        let guion: Table = lua.load(fuente).set_name("=guion").set_mode(ChunkMode::Text).eval()?;
        let v = lua.create_table()?;
        v.set("sr", sr)?;
        v.set("bloque", BLOQUE)?;
        v.set("datos", lua.create_table_from(datos.iter().map(|(k, x)| (k.as_str(), x.as_str())))?)?;
        let f: Function = guion.get::<Option<Function>>("vivo")?.ok_or_else(|| mlua::Error::runtime(tr("falta la función «vivo»")))?;
        let (c, m) = (lua.create_table()?, lua.create_table()?);
        claves.iter().try_for_each(|k| c.set(k.as_str(), 0.0))?;
        for (k, x) in [("pos", 0.0), ("bpm", 120.0)] {
            m.set(k, x)?;
        }
        m.set("tocando", false)?;
        let mut tablas = vec![];
        for _ in 0..4 {
            let x = lua.create_table_with_capacity(BLOQUE, 0)?;
            (1..=BLOQUE).try_for_each(|i| x.set(i, 0.0))?;
            tablas.push(x);
        }
        m.set("otra_izq", &tablas[2])?;
        m.set("otra_der", &tablas[3])?;
        let tablas: [Table; 4] = tablas.try_into().expect("son cuatro");
        let por_muestra: Function = g.get("__por_muestra")?;
        let (mut nota, mut suelta, mut rueda, mut mando, mut latencia) = (None, None, None, None, 0u32);
        let bloque = match f.call::<Value>(v)? {
            Value::Function(x) => Some(por_muestra.call::<Function>(x)?),
            Value::Table(x) => {
                (nota, suelta) = (x.get("nota")?, x.get("suelta")?);
                (rueda, mando) = (x.get("rueda")?, x.get("mando")?);
                latencia = x.get::<Option<u32>>("latencia")?.unwrap_or(0).min(sr as u32);
                match x.get::<Option<Function>>("bloque")? {
                    Some(b) => Some(b),
                    None => x.get::<Option<Function>>("muestra")?.map(|f| por_muestra.call(f)).transpose()?,
                }
            }
            _ => return Err(mlua::Error::runtime(tr("«vivo» debe devolver una función o una tabla"))),
        };
        lua.gc_collect()?;
        // Desde aquí, tope de pasos por bloque.
        let cuenta = Rc::new(Cell::new(0u32));
        let cc = cuenta.clone();
        lua.set_hook(HookTriggers::new().every_nth_instruction(10_000), move |_, _| {
            cc.set(cc.get() + 10_000);
            if cc.get() > PASOS { Err(mlua::Error::runtime(tr("tardó demasiado para sonar en vivo"))) } else { Ok(VmState::Continue) }
        })?;
        let (manda_f, fallas) = bounded(4);
        let (manda_g, grabado) = bounded(1 << 14);
        let d = Dentro {
            bloque,
            nota,
            suelta,
            rueda,
            mando,
            tablas,
            c,
            m,
            claves: claves.iter().filter_map(|k| CString::new(k.as_str()).ok()).collect(),
            puestos: vec![f32::NAN; claves.len()],
            cuenta,
            puente: puente_,
            grabado: manda_g,
            fallas: manda_f,
            muerto: false,
            lua,
            _reserva: reserva,
        };
        Ok(Vivo { celda: UnsafeCell::new(d), ocupado: AtomicBool::new(false), latencia, fallas, grabado })
    }

    /// Lo que falló, si falló: el guion ya no suena.
    pub fn falla(&self) -> Option<String> {
        self.fallas.try_recv().ok()
    }

    /// Lo grabado desde la última vez.
    pub fn grabado(&self) -> Vec<[f32; 2]> {
        self.grabado.try_iter().flat_map(|(x, n)| x[..n].to_vec()).collect()
    }

    /// Hilo de audio: cambia `x` en su lugar. `ev` es lo que pasa al empezar el bloque. Las notas que mande quedan en `notas`.
    pub fn procesar(&self, ev: &[Evento], ctl: &[f32], t: &Momento, x: &mut [Buf; 2], otra: &[Buf; 2], n: usize, notas: &mut Vec<Emitida>) {
        if self.ocupado.swap(true, Acquire) {
            return;
        }
        let d = unsafe { &mut *self.celda.get() };
        if !d.muerto {
            let antes = *x;
            let r = d.correr(ev, ctl, t, x, otra, n);
            let bien = x.iter().all(|b| b[..n].iter().all(|v| v.is_finite() && v.abs() < 100.0));
            if r.is_err() || !bien {
                *x = antes;
                d.muerto = true;
                let e = r.err().map_or_else(|| tr("su sonido se vuelve infinito").to_string(), texto);
                let _ = d.fallas.try_send(e);
            }
            let puente = unsafe { &mut *d.puente.0 };
            if puente.graba {
                let mut q = [[0.0; 2]; BLOQUE];
                (0..n).for_each(|i| q[i] = [x[0][i], x[1][i]]);
                let _ = d.grabado.try_send((q, n));
            }
            let caben = notas.capacity() - notas.len();
            notas.extend(puente.emitidas.drain(..).take(caben));
        }
        self.ocupado.store(false, Release);
    }
}

impl Dentro {
    fn correr(&mut self, ev: &[Evento], ctl: &[f32], t: &Momento, x: &mut [Buf; 2], otra: &[Buf; 2], n: usize) -> mlua::Result<()> {
        self.cuenta.set(0);
        let (claves, puestos) = (&self.claves, &mut self.puestos);
        unsafe {
            self.lua.exec_raw::<()>((&self.c, &self.m), |l| {
                for ((k, v), p) in claves.iter().zip(ctl).zip(puestos.iter_mut()) {
                    if *p != *v {
                        *p = *v;
                        ffi::lua_pushnumber(l, *v as f64);
                        ffi::lua_setfield(l, 1, k.as_ptr());
                    }
                }
                ffi::lua_pushnumber(l, t.pos);
                ffi::lua_setfield(l, 2, M_POS.as_ptr());
                ffi::lua_pushnumber(l, t.bpm as f64);
                ffi::lua_setfield(l, 2, M_BPM.as_ptr());
                ffi::lua_pushboolean(l, t.tocando as c_int);
                ffi::lua_setfield(l, 2, M_TOCANDO.as_ptr());
                ffi::lua_settop(l, 0);
            })?;
        }
        for e in ev {
            match (e, &self.nota, &self.suelta, &self.rueda, &self.mando) {
                (Evento::Nota { tono, fuerza }, Some(f), ..) => f.call::<()>((*tono, *fuerza, &self.c))?,
                (Evento::Suelta { tono }, _, Some(f), ..) => f.call::<()>(*tono)?,
                (Evento::Rueda(v), _, _, Some(f), _) => f.call::<()>(*v)?,
                (Evento::Mando { n, v }, .., Some(f)) => f.call::<()>((*n, *v))?,
                _ => {}
            }
        }
        let Some(f) = &self.bloque else { return Ok(()) };
        unsafe {
            self.lua.exec_raw::<()>((&self.tablas[0], &self.tablas[1], &self.tablas[2], &self.tablas[3]), |l| {
                for (k, b) in [&x[0], &x[1], &otra[0], &otra[1]].into_iter().enumerate() {
                    for (i, v) in b[..n].iter().enumerate() {
                        ffi::lua_pushnumber(l, *v as f64);
                        ffi::lua_rawseti(l, k as c_int + 1, i as i64 + 1);
                    }
                }
                ffi::lua_settop(l, 0);
            })?;
        }
        f.call::<()>((&self.tablas[0], &self.tablas[1], n, &self.c, &self.m))?;
        unsafe {
            self.lua.exec_raw::<()>((&self.tablas[0], &self.tablas[1]), |l| {
                for (k, b) in x.iter_mut().enumerate() {
                    for (i, v) in b[..n].iter_mut().enumerate() {
                        ffi::lua_rawgeti(l, k as c_int + 1, i as i64 + 1);
                        *v = ffi::lua_tonumber(l, -1) as f32;
                        ffi::lua_pop(l, 1);
                    }
                }
                ffi::lua_settop(l, 0);
            })?;
        }
        Ok(())
    }
}

/// Un guion en vivo, ya con su error dicho de forma clara.
pub fn mensaje(nombre: &str, e: &str) -> String {
    tf("«{}» se apagó: {}", &[&nombre, &e])
}
