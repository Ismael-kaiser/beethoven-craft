// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
use super::{ControlP, EVENTOS, Evento, Oferta, Puertos, Tiempo, binario};
use crate::idioma::t;
use crate::receta::{BLOQUE, Buf};
use clap_sys::audio_buffer::clap_audio_buffer;
use clap_sys::entry::clap_plugin_entry;
use clap_sys::events::*;
use clap_sys::ext::audio_ports::{CLAP_EXT_AUDIO_PORTS, clap_audio_port_info, clap_plugin_audio_ports};
use clap_sys::ext::gui::*;
use clap_sys::ext::params::*;
use clap_sys::ext::state::{CLAP_EXT_STATE, clap_host_state, clap_plugin_state};
use clap_sys::factory::plugin_factory::{CLAP_PLUGIN_FACTORY_ID, clap_plugin_factory};
use clap_sys::host::clap_host;
use clap_sys::plugin::clap_plugin;
use clap_sys::process::clap_process;
use clap_sys::stream::{clap_istream, clap_ostream};
use clap_sys::version::CLAP_VERSION;
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::Path;
use std::ptr::{null, null_mut};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

/// Las bibliotecas abiertas se quedan abiertas: cerrarlas suele tronar.
static ABIERTAS: Mutex<Vec<(std::path::PathBuf, usize)>> = Mutex::new(vec![]);

fn fabrica(ruta: &Path) -> Result<*const clap_plugin_factory, String> {
    let mut v = ABIERTAS.lock().unwrap();
    if let Some((_, f)) = v.iter().find(|x| x.0 == ruta) {
        return Ok(*f as *const _);
    }
    unsafe {
        let lib = libloading::Library::new(binario(ruta)?).map_err(|e| e.to_string())?;
        let e: *const clap_plugin_entry = *lib.get::<*const clap_plugin_entry>(b"clap_entry\0").map_err(|e| e.to_string())?;
        let e = e.as_ref().ok_or(t("sin entrada"))?;
        let c = CString::new(ruta.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
        if !e.init.is_some_and(|f| f(c.as_ptr())) {
            return Err(t("El plugin no arrancó").into());
        }
        let f = e.get_factory.map_or(null(), |f| f(CLAP_PLUGIN_FACTORY_ID.as_ptr())) as *const clap_plugin_factory;
        if f.is_null() {
            return Err(t("El archivo no trae plugins").into());
        }
        std::mem::forget(lib);
        v.push((ruta.into(), f as usize));
        Ok(f)
    }
}

unsafe fn txt(p: *const c_char) -> String {
    if p.is_null() { String::new() } else { unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned() }
}

pub fn ofertas(ruta: &Path) -> Result<Vec<Oferta>, String> {
    let f = fabrica(ruta)?;
    let mut v = vec![];
    unsafe {
        let n = (*f).get_plugin_count.map_or(0, |g| g(f));
        for i in 0..n {
            let Some(d) = (*f).get_plugin_descriptor.and_then(|g| g(f, i).as_ref()) else { continue };
            let mut instrumento = false;
            let mut p = d.features;
            while !p.is_null() && !(*p).is_null() {
                instrumento |= txt(*p) == "instrument";
                p = p.add(1);
            }
            v.push(Oferta { ruta: ruta.to_string_lossy().into(), id: txt(d.id), nombre: txt(d.name), instrumento });
        }
    }
    Ok(v)
}

/// Lo que el plugin le pide al programa.
#[derive(Default)]
struct Pedidos {
    llamar: AtomicBool,
    /// ancho << 32 | alto; 0 es nada.
    tam: AtomicU64,
}

unsafe fn pedidos<'a>(h: *const clap_host) -> &'a Pedidos {
    unsafe { &*((*h).host_data as *const Pedidos) }
}

unsafe extern "C" fn extension(_: *const clap_host, id: *const c_char) -> *const c_void {
    static GUI: clap_host_gui = clap_host_gui {
        resize_hints_changed: Some(nada),
        request_resize: Some(pide_tam),
        request_show: Some(no),
        request_hide: Some(no),
        closed: Some(cerro),
    };
    static PAR: clap_host_params = clap_host_params { rescan: Some(releer), clear: Some(borrar), request_flush: Some(nada) };
    static EST: clap_host_state = clap_host_state { mark_dirty: Some(nada) };
    let id = unsafe { CStr::from_ptr(id) };
    if id == CLAP_EXT_GUI {
        &GUI as *const _ as *const c_void
    } else if id == CLAP_EXT_PARAMS {
        &PAR as *const _ as *const c_void
    } else if id == CLAP_EXT_STATE {
        &EST as *const _ as *const c_void
    } else {
        null()
    }
}
unsafe extern "C" fn nada(_: *const clap_host) {}
unsafe extern "C" fn no(_: *const clap_host) -> bool {
    false
}
unsafe extern "C" fn cerro(_: *const clap_host, _: bool) {}
unsafe extern "C" fn releer(_: *const clap_host, _: u32) {}
unsafe extern "C" fn borrar(_: *const clap_host, _: u32, _: u32) {}
unsafe extern "C" fn llamar(h: *const clap_host) {
    unsafe { pedidos(h) }.llamar.store(true, Relaxed);
}
unsafe extern "C" fn pide_tam(h: *const clap_host, w: u32, a: u32) -> bool {
    unsafe { pedidos(h) }.tam.store((w as u64) << 32 | a as u64, Relaxed);
    true
}

#[repr(C)]
union Ev {
    cab: clap_event_header,
    nota: clap_event_note,
    control: clap_event_param_value,
}

struct Lista {
    v: Vec<Ev>,
}

unsafe extern "C" fn cuantos(l: *const clap_input_events) -> u32 {
    unsafe { (*((*l).ctx as *const Lista)).v.len() as u32 }
}
unsafe extern "C" fn uno(l: *const clap_input_events, i: u32) -> *const clap_event_header {
    unsafe { (&(*((*l).ctx as *const Lista)).v).get(i as usize).map_or(null(), |e| &e.cab) }
}
unsafe extern "C" fn recibe(_: *const clap_output_events, _: *const clap_event_header) -> bool {
    true
}

pub struct Clap {
    p: *const clap_plugin,
    _host: Box<clap_host>,
    pedidos: Box<Pedidos>,
    params: *const clap_plugin_params,
    estado: *const clap_plugin_state,
    gui: *const clap_plugin_gui,
    info: Vec<clap_param_info>,
    entra: Puertos,
    sale: Puertos,
    lista: Box<Lista>,
    cookies: Vec<(u32, *mut c_void)>,
    procesando: bool,
    activo: bool,
    cuadros: i64,
    abierta: bool,
}

impl Clap {
    pub fn cargar(ruta: &Path, id: &str, sr: f64) -> Result<Box<Clap>, String> {
        let f = fabrica(ruta)?;
        let pedidos = Box::new(Pedidos::default());
        let host = Box::new(clap_host {
            clap_version: CLAP_VERSION,
            host_data: &*pedidos as *const Pedidos as *mut c_void,
            name: c"Beethoven DAW Demo".as_ptr(),
            vendor: c"mixter-ha99".as_ptr(),
            url: c"".as_ptr(),
            version: c"0.1.0".as_ptr(),
            get_extension: Some(extension),
            request_restart: Some(nada),
            request_process: Some(nada),
            request_callback: Some(llamar),
        });
        let cid = CString::new(id).map_err(|e| e.to_string())?;
        unsafe {
            let p = (*f).create_plugin.map_or(null(), |c| c(f, &*host, cid.as_ptr()));
            if p.is_null() {
                return Err(t("El plugin no se pudo crear").into());
            }
            let mut c = Box::new(Clap {
                p,
                _host: host,
                pedidos,
                params: null(),
                estado: null(),
                gui: null(),
                info: vec![],
                entra: Puertos::nuevos(&[]),
                sale: Puertos::nuevos(&[]),
                lista: Box::new(Lista { v: Vec::with_capacity(EVENTOS) }),
                cookies: vec![],
                procesando: false,
                activo: false,
                cuadros: 0,
                abierta: false,
            });
            if !(*p).init.is_some_and(|i| i(p)) {
                return Err(t("El plugin no arrancó").into());
            }
            let ext = |id: &CStr| (*p).get_extension.map_or(null(), |g| g(p, id.as_ptr()));
            c.params = ext(CLAP_EXT_PARAMS) as *const _;
            c.estado = ext(CLAP_EXT_STATE) as *const _;
            c.gui = ext(CLAP_EXT_GUI) as *const _;
            let puertos = ext(CLAP_EXT_AUDIO_PORTS) as *const clap_plugin_audio_ports;
            let canales = |entrada: bool| -> Vec<usize> {
                let Some(a) = puertos.as_ref() else { return if entrada { vec![] } else { vec![2] } };
                let n = a.count.map_or(0, |c| c(p, entrada));
                (0..n)
                    .map(|i| {
                        let mut x: clap_audio_port_info = std::mem::zeroed();
                        if a.get.is_some_and(|g| g(p, i, entrada, &mut x)) { x.channel_count as usize } else { 0 }
                    })
                    .collect()
            };
            c.entra = Puertos::nuevos(&canales(true));
            c.sale = Puertos::nuevos(&canales(false));
            if let Some(a) = c.params.as_ref() {
                for i in 0..a.count.map_or(0, |f| f(p)) {
                    let mut x: clap_param_info = std::mem::zeroed();
                    if a.get_info.is_some_and(|g| g(p, i, &mut x)) {
                        c.cookies.push((x.id, x.cookie));
                        c.info.push(x);
                    }
                }
            }
            if !(*p).activate.is_some_and(|a| a(p, sr, 1, BLOQUE as u32)) {
                return Err(t("El plugin no se pudo activar").into());
            }
            c.activo = true;
            Ok(c)
        }
    }

    pub fn nombre(&self) -> String {
        unsafe { (*self.p).desc.as_ref().map_or(String::new(), |d| txt(d.name)) }
    }

    pub fn controles(&self) -> Vec<ControlP> {
        self.info
            .iter()
            .filter(|i| i.flags & (CLAP_PARAM_IS_HIDDEN | CLAP_PARAM_IS_READONLY) == 0 && i.max_value > i.min_value)
            .map(|i| ControlP {
                id: i.id,
                nombre: unsafe { txt(i.name.as_ptr()) },
                min: i.min_value,
                max: i.max_value,
                entero: i.flags & CLAP_PARAM_IS_STEPPED != 0,
            })
            .collect()
    }

    pub fn valor(&self, id: u32) -> f64 {
        let mut v = 0.0;
        unsafe {
            if let Some(g) = self.params.as_ref().and_then(|a| a.get_value) {
                g(self.p, id, &mut v);
            }
        }
        v
    }

    pub fn texto(&self, id: u32, valor: f64) -> String {
        let mut b = [0 as c_char; 64];
        unsafe {
            match self.params.as_ref().and_then(|a| a.value_to_text) {
                Some(f) if f(self.p, id, valor, b.as_mut_ptr(), 63) => txt(b.as_ptr()),
                _ => format!("{valor:.2}"),
            }
        }
    }

    pub fn estado(&self) -> Vec<u8> {
        unsafe extern "C" fn escribe(s: *const clap_ostream, b: *const c_void, n: u64) -> i64 {
            unsafe {
                let v = &mut *((*s).ctx as *mut Vec<u8>);
                v.extend_from_slice(std::slice::from_raw_parts(b as *const u8, n as usize));
            }
            n as i64
        }
        let mut v: Vec<u8> = vec![];
        let s = clap_ostream { ctx: &mut v as *mut _ as *mut c_void, write: Some(escribe) };
        unsafe {
            if let Some(f) = self.estado.as_ref().and_then(|a| a.save) {
                f(self.p, &s);
            }
        }
        v
    }

    pub fn poner_estado(&mut self, b: &[u8]) {
        unsafe extern "C" fn lee(s: *const clap_istream, b: *mut c_void, n: u64) -> i64 {
            unsafe {
                let r = &mut *((*s).ctx as *mut &[u8]);
                let n = r.len().min(n as usize);
                std::ptr::copy_nonoverlapping(r.as_ptr(), b as *mut u8, n);
                *r = &r[n..];
                n as i64
            }
        }
        let mut r = b;
        let s = clap_istream { ctx: &mut r as *mut _ as *mut c_void, read: Some(lee) };
        unsafe {
            if let Some(f) = self.estado.as_ref().and_then(|a| a.load) {
                f(self.p, &s);
            }
        }
    }

    pub fn atender(&self) {
        if self.pedidos.llamar.swap(false, Relaxed) {
            unsafe {
                if let Some(f) = (*self.p).on_main_thread {
                    f(self.p);
                }
            }
        }
    }

    pub fn procesar(&mut self, ev: &[Evento], t: &Tiempo, entra: &[Buf; 2], sale: &mut [Buf; 2], n: usize) {
        unsafe {
            if !self.procesando {
                // Si se niega, no se vuelve a intentar en cada bloque.
                self.procesando = true;
                if !(*self.p).start_processing.is_none_or(|f| f(self.p)) {
                    self.activo = false;
                }
            }
            if !self.activo {
                return sale.iter_mut().for_each(|c| c[..n].fill(0.0));
            }
            self.entra.llenar(entra, n);
            self.lista.v.clear();
            let cab = |tipo, tam: usize| clap_event_header { size: tam as u32, time: 0, space_id: CLAP_CORE_EVENT_SPACE_ID, type_: tipo, flags: 0 };
            let nota = |tipo, tono: u8, fuerza: f64| Ev {
                nota: clap_event_note {
                    header: cab(tipo, size_of::<clap_event_note>()),
                    note_id: -1,
                    port_index: 0,
                    channel: 0,
                    key: tono as i16,
                    velocity: fuerza,
                },
            };
            for e in ev {
                self.lista.v.push(match *e {
                    Evento::Nota { tono, fuerza } => nota(CLAP_EVENT_NOTE_ON, tono, fuerza as f64),
                    Evento::Suelta { tono } => nota(CLAP_EVENT_NOTE_OFF, tono, 0.0),
                    Evento::Control { id, valor } => Ev {
                        control: clap_event_param_value {
                            header: cab(CLAP_EVENT_PARAM_VALUE, size_of::<clap_event_param_value>()),
                            param_id: id,
                            cookie: self.cookies.iter().find(|c| c.0 == id).map_or(null_mut(), |c| c.1),
                            note_id: -1,
                            port_index: -1,
                            channel: -1,
                            key: -1,
                            value: valor,
                        },
                    },
                });
            }
            let en = clap_input_events { ctx: &mut *self.lista as *mut Lista as *mut c_void, size: Some(cuantos), get: Some(uno) };
            let fuera = clap_output_events { ctx: null_mut(), try_push: Some(recibe) };
            let mut tr: clap_event_transport = std::mem::zeroed();
            tr.header = cab(CLAP_EVENT_TRANSPORT, size_of::<clap_event_transport>());
            tr.flags = CLAP_TRANSPORT_HAS_TEMPO | CLAP_TRANSPORT_HAS_BEATS_TIMELINE | CLAP_TRANSPORT_HAS_TIME_SIGNATURE;
            if t.tocando {
                tr.flags |= CLAP_TRANSPORT_IS_PLAYING;
            }
            tr.tempo = t.bpm;
            tr.song_pos_beats = (t.pos * (1u64 << 31) as f64) as i64;
            tr.bar_start = ((t.pos / 4.0).floor() * 4.0 * (1u64 << 31) as f64) as i64;
            tr.bar_number = (t.pos / 4.0) as i32;
            (tr.tsig_num, tr.tsig_denom) = (4, 4);
            const P: usize = 16;
            let lleno = |p: &mut Puertos| -> ([clap_audio_buffer; P], usize) {
                let mut b: [clap_audio_buffer; P] = std::mem::zeroed();
                let n = p.punteros.len().min(P);
                for (b, p) in b.iter_mut().zip(&mut p.punteros) {
                    b.data32 = p.as_mut_ptr();
                    b.channel_count = p.len() as u32;
                }
                (b, n)
            };
            let (a, na) = lleno(&mut self.entra);
            let (mut b, nb) = lleno(&mut self.sale);
            let pr = clap_process {
                steady_time: self.cuadros,
                frames_count: n as u32,
                transport: &tr,
                audio_inputs: a.as_ptr(),
                audio_outputs: b.as_mut_ptr(),
                audio_inputs_count: na as u32,
                audio_outputs_count: nb as u32,
                in_events: &en,
                out_events: &fuera,
            };
            let bien = (*self.p).process.is_some_and(|f| f(self.p, &pr) != 0);
            self.cuadros += n as i64;
            if bien {
                self.sale.vaciar(sale, n);
            } else {
                sale.iter_mut().for_each(|c| c[..n].fill(0.0));
            }
        }
    }

    pub fn pegar(&mut self, vista: *mut c_void) -> Option<(u32, u32)> {
        let api = if cfg!(windows) { CLAP_WINDOW_API_WIN32 } else { CLAP_WINDOW_API_COCOA };
        unsafe {
            let g = self.gui.as_ref()?;
            if self.abierta || !g.is_api_supported.is_some_and(|f| f(self.p, api.as_ptr(), false)) || !g.create?(self.p, api.as_ptr(), false) {
                return None;
            }
            self.abierta = true;
            let w = clap_window { api: api.as_ptr(), specific: clap_window_handle { cocoa: vista } };
            let (mut x, mut y) = (600, 400);
            g.get_size?(self.p, &mut x, &mut y);
            if !g.set_parent?(self.p, &w) {
                self.despegar();
                return None;
            }
            g.show?(self.p);
            Some((x, y))
        }
    }

    pub fn despegar(&mut self) {
        if std::mem::take(&mut self.abierta) {
            unsafe {
                if let Some(d) = self.gui.as_ref().and_then(|g| g.destroy) {
                    d(self.p);
                }
            }
        }
    }

    pub fn tamano_pedido(&self) -> Option<(u32, u32)> {
        let x = self.pedidos.tam.swap(0, Relaxed);
        let t = (x != 0).then_some(((x >> 32) as u32, x as u32))?;
        unsafe {
            if let Some(f) = self.gui.as_ref().and_then(|g| g.set_size) {
                f(self.p, t.0, t.1);
            }
        }
        Some(t)
    }
}

impl Drop for Clap {
    fn drop(&mut self) {
        self.despegar();
        unsafe {
            let p = &*self.p;
            if self.procesando && self.activo {
                p.stop_processing.inspect(|f| f(p));
            }
            if self.activo {
                p.deactivate.inspect(|f| f(p));
            }
            p.destroy.inspect(|f| f(p));
        }
    }
}
