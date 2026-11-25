// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! El lado CLAP.
use crate::{Cual, escrito, seguro, todos};
use beethoven_daw_demo::instancia::{En, Instancia};
use beethoven_daw_demo::plugin::{Evento, Tiempo};
use clap_sys::audio_buffer::clap_audio_buffer;
use clap_sys::entry::clap_plugin_entry;
use clap_sys::events::*;
use clap_sys::ext::audio_ports::*;
use clap_sys::ext::gui::*;
use clap_sys::ext::latency::*;
use clap_sys::ext::note_ports::*;
use clap_sys::ext::params::*;
use clap_sys::ext::state::*;
#[cfg(target_os = "linux")]
use clap_sys::ext::timer_support::*;
use clap_sys::factory::plugin_factory::*;
use clap_sys::host::clap_host;
use clap_sys::id::clap_id;
use clap_sys::plugin::*;
use clap_sys::plugin_features::*;
use clap_sys::process::*;
use clap_sys::stream::*;
use clap_sys::version::CLAP_VERSION;
use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr::{null, null_mut};
use std::sync::OnceLock;

struct Descriptor(clap_plugin_descriptor, #[allow(dead_code)] Box<[*const c_char; 4]>);
unsafe impl Send for Descriptor {}
unsafe impl Sync for Descriptor {}

fn descriptores() -> &'static [Descriptor] {
    static D: OnceLock<Vec<Descriptor>> = OnceLock::new();
    D.get_or_init(|| {
        todos()
            .iter()
            .map(|c| {
                let tipo = if c.instrumento() { CLAP_PLUGIN_FEATURE_INSTRUMENT } else { CLAP_PLUGIN_FEATURE_AUDIO_EFFECT };
                let f = Box::new([tipo.as_ptr(), CLAP_PLUGIN_FEATURE_STEREO.as_ptr(), null(), null()]);
                let d = clap_plugin_descriptor {
                    clap_version: CLAP_VERSION,
                    id: c.c_id.as_ptr(),
                    name: c.c_nombre.as_ptr(),
                    vendor: c"Beethoven".as_ptr(),
                    url: c"".as_ptr(),
                    manual_url: c"".as_ptr(),
                    support_url: c"".as_ptr(),
                    version: c"1.0.0".as_ptr(),
                    description: c"".as_ptr(),
                    features: f.as_ptr(),
                };
                Descriptor(d, f)
            })
            .collect()
    })
}

struct Uno {
    plugin: clap_plugin,
    #[allow(dead_code)]
    host: *const clap_host,
    cual: &'static Cual,
    n: Option<Box<Instancia>>,
    eventos: Vec<En>,
    movidos: u64,
    silencio: Vec<f32>,
    /// El reloj que presta el otro programa, en Linux.
    #[cfg(target_os = "linux")]
    reloj: Option<clap_id>,
    ventana: Option<crate::ventana::Ventana>,
}

unsafe fn uno<'a>(p: *const clap_plugin) -> &'a mut Uno {
    unsafe { &mut *((*p).plugin_data as *mut Uno) }
}

unsafe extern "C" fn iniciar(p: *const clap_plugin) -> bool {
    seguro(false, || unsafe {
        let u = uno(p);
        u.n = Instancia::nueva(&u.cual.clave, 48000.0, Default::default()).map(Box::new);
        u.n.is_some()
    })
}

unsafe extern "C" fn destruir(p: *const clap_plugin) {
    seguro((), || unsafe { drop(Box::from_raw((*p).plugin_data as *mut Uno)) })
}

unsafe extern "C" fn activar(p: *const clap_plugin, sr: f64, _: u32, max: u32) -> bool {
    seguro(false, || unsafe {
        let u = uno(p);
        u.silencio = vec![0.0; max as usize];
        // Otra frecuencia: se arma de nuevo con lo mismo.
        if let Some(n) = u.n.as_ref().filter(|n| n.sr != sr as f32) {
            let g = n.guardado();
            u.ventana = None;
            match Instancia::nueva(&u.cual.clave, sr as f32, g) {
                Some(x) => u.n = Some(Box::new(x)),
                None => return false,
            }
        }
        true
    })
}

unsafe extern "C" fn nada(_: *const clap_plugin) {}
unsafe extern "C" fn si(_: *const clap_plugin) -> bool {
    true
}

unsafe fn evento(n: &Instancia, v: &mut Vec<En>, e: *const clap_event_header) {
    unsafe {
        let h = &*e;
        if h.space_id != CLAP_CORE_EVENT_SPACE_ID {
            return;
        }
        let en = h.time as usize;
        let x = match h.type_ {
            CLAP_EVENT_NOTE_ON => {
                let q = &*(e as *const clap_event_note);
                Evento::Nota { tono: q.key.clamp(0, 127) as u8, fuerza: q.velocity as f32 }
            }
            CLAP_EVENT_NOTE_OFF | CLAP_EVENT_NOTE_CHOKE => Evento::Suelta { tono: (*(e as *const clap_event_note)).key.clamp(0, 127) as u8 },
            CLAP_EVENT_PARAM_VALUE => {
                let q = &*(e as *const clap_event_param_value);
                n.poner(q.param_id as usize, q.value as f32);
                return;
            }
            CLAP_EVENT_MIDI => {
                let d = (*(e as *const clap_event_midi)).data;
                match (d[0] >> 4, d[2]) {
                    (9, 1..) => Evento::Nota { tono: d[1] & 127, fuerza: d[2] as f32 / 127.0 },
                    (8 | 9, _) => Evento::Suelta { tono: d[1] & 127 },
                    (14, _) => Evento::Rueda(((d[2] as i32) << 7 | d[1] as i32) as f32 / 8192.0 - 1.0),
                    (11, _) => Evento::Mando { n: d[1], v: d[2] as f32 / 127.0 },
                    _ => return,
                }
            }
            _ => return,
        };
        if v.len() < v.capacity() {
            v.push((en, x));
        }
    }
}

unsafe fn canales<'a>(b: *const clap_audio_buffer, k: usize, cuantos: u32, n: usize) -> [&'a [f32]; 2] {
    unsafe {
        if b.is_null() || k >= cuantos as usize {
            return [&[], &[]];
        }
        let b = &*b.add(k);
        if b.data32.is_null() || b.channel_count == 0 {
            return [&[], &[]];
        }
        let c = |i: u32| std::slice::from_raw_parts(*b.data32.add(i.min(b.channel_count - 1) as usize), n);
        [c(0), c(1)]
    }
}

unsafe extern "C" fn procesar(p: *const clap_plugin, d: *const clap_process) -> clap_process_status {
    unsafe {
        let u = uno(p);
        let d = &*d;
        let Some(n) = u.n.as_deref().map(|n| &*(n as *const Instancia)) else { return CLAP_PROCESS_ERROR };
        let largo = d.frames_count as usize;
        u.eventos.clear();
        if let Some(l) = d.in_events.as_ref() {
            let cuantos = l.size.map_or(0, |f| f(l));
            for i in 0..cuantos {
                if let Some(e) = l.get.map(|f| f(l, i)).filter(|e| !e.is_null()) {
                    evento(n, &mut u.eventos, e);
                }
            }
        }
        avisar(u, d.out_events);
        if d.audio_outputs_count == 0 || largo == 0 {
            return CLAP_PROCESS_CONTINUE;
        }
        let o = &*d.audio_outputs;
        if o.channel_count < 2 || o.data32.is_null() {
            return CLAP_PROCESS_ERROR;
        }
        let mut t = Tiempo { bpm: 120.0, pos: 0.0, tocando: false, sr: n.sr as f64 };
        if let Some(x) = d.transport.as_ref() {
            if x.flags & CLAP_TRANSPORT_HAS_TEMPO != 0 {
                t.bpm = x.tempo;
            }
            if x.flags & CLAP_TRANSPORT_HAS_BEATS_TIMELINE != 0 {
                t.pos = x.song_pos_beats as f64 / (1u64 << 31) as f64;
            }
            t.tocando = x.flags & CLAP_TRANSPORT_IS_PLAYING != 0;
        }
        let entra = canales(d.audio_inputs, 0, d.audio_inputs_count, largo);
        let lado = canales(d.audio_inputs, 1, d.audio_inputs_count, largo);
        let sale = [std::slice::from_raw_parts_mut(*o.data32, largo), std::slice::from_raw_parts_mut(*o.data32.add(1), largo)];
        n.procesar(&u.eventos, entra, lado, sale, &t);
        CLAP_PROCESS_CONTINUE
    }
}

/// Lo que se movió en la ventana, para que el otro programa lo anote.
unsafe fn avisar(u: &mut Uno, l: *const clap_output_events) {
    unsafe {
        let Some(n) = &u.n else { return };
        let m = n.movidos.load(std::sync::atomic::Ordering::Relaxed);
        if m == u.movidos {
            return;
        }
        u.movidos = m;
        let Some(l) = l.as_ref() else { return };
        for k in 0..n.receta.controles.len() {
            let e = clap_event_param_value {
                header: clap_event_header {
                    size: size_of::<clap_event_param_value>() as u32,
                    time: 0,
                    space_id: CLAP_CORE_EVENT_SPACE_ID,
                    type_: CLAP_EVENT_PARAM_VALUE,
                    flags: 0,
                },
                param_id: k as clap_id,
                cookie: null_mut(),
                note_id: -1,
                port_index: -1,
                channel: -1,
                key: -1,
                value: n.valor(k) as f64,
            };
            if let Some(f) = l.try_push {
                f(l, &e.header);
            }
        }
    }
}

unsafe extern "C" fn principal(p: *const clap_plugin) {
    seguro((), || unsafe {
        if let Some(n) = &uno(p).n {
            n.poner_al_dia();
        }
    })
}

// ---- Puertos ----
unsafe extern "C" fn puertos(p: *const clap_plugin, entrada: bool) -> u32 {
    unsafe {
        match (uno(p).cual.instrumento(), entrada) {
            (true, true) => 0,
            (false, true) => 2,
            _ => 1,
        }
    }
}

fn texto(de: &str, a: &mut [c_char]) {
    let n = de.len().min(a.len() - 1);
    let mut n = n;
    while !de.is_char_boundary(n) {
        n -= 1;
    }
    for (x, b) in a.iter_mut().zip(&de.as_bytes()[..n]) {
        *x = *b as c_char;
    }
    a[n] = 0;
}

unsafe extern "C" fn puerto(p: *const clap_plugin, i: u32, entrada: bool, o: *mut clap_audio_port_info) -> bool {
    unsafe {
        if i >= puertos(p, entrada) {
            return false;
        }
        let o = &mut *o;
        o.id = i;
        texto(if i == 1 { "Sidechain" } else { "Main" }, &mut o.name);
        o.flags = if i == 0 { CLAP_AUDIO_PORT_IS_MAIN } else { 0 };
        o.channel_count = 2;
        o.port_type = CLAP_PORT_STEREO.as_ptr();
        o.in_place_pair = clap_sys::id::CLAP_INVALID_ID;
        true
    }
}

static PUERTOS: clap_plugin_audio_ports = clap_plugin_audio_ports { count: Some(puertos), get: Some(puerto) };

unsafe extern "C" fn notas(p: *const clap_plugin, entrada: bool) -> u32 {
    unsafe { (entrada && uno(p).cual.instrumento()) as u32 }
}
unsafe extern "C" fn nota(p: *const clap_plugin, i: u32, entrada: bool, o: *mut clap_note_port_info) -> bool {
    unsafe {
        if i >= notas(p, entrada) {
            return false;
        }
        let o = &mut *o;
        o.id = 0;
        o.supported_dialects = CLAP_NOTE_DIALECT_CLAP | CLAP_NOTE_DIALECT_MIDI;
        o.preferred_dialect = CLAP_NOTE_DIALECT_MIDI;
        texto("Notes", &mut o.name);
        true
    }
}
static NOTAS: clap_plugin_note_ports = clap_plugin_note_ports { count: Some(notas), get: Some(nota) };

// ---- Controles ----
unsafe extern "C" fn p_cuantos(p: *const clap_plugin) -> u32 {
    unsafe { uno(p).n.as_ref().map_or(0, |n| n.receta.controles.len() as u32) }
}
unsafe extern "C" fn p_info(p: *const clap_plugin, i: u32, o: *mut clap_param_info) -> bool {
    unsafe {
        let Some(c) = uno(p).n.as_ref().and_then(|n| n.receta.controles.get(i as usize)) else { return false };
        let o = &mut *o;
        o.id = i;
        o.flags = CLAP_PARAM_IS_AUTOMATABLE;
        o.cookie = null_mut();
        texto(c.nombre.ver(), &mut o.name);
        texto("", &mut o.module);
        (o.min_value, o.max_value, o.default_value) = (c.min as f64, c.max as f64, c.valor as f64);
        true
    }
}
unsafe extern "C" fn p_valor(p: *const clap_plugin, id: clap_id, v: *mut f64) -> bool {
    unsafe {
        let Some(n) = uno(p).n.as_ref().filter(|n| (id as usize) < n.receta.controles.len()) else { return false };
        *v = n.valor(id as usize) as f64;
        true
    }
}
unsafe extern "C" fn p_texto(_: *const clap_plugin, _: clap_id, v: f64, o: *mut c_char, n: u32) -> bool {
    unsafe {
        if n < 2 {
            return false;
        }
        texto(&escrito(v as f32), std::slice::from_raw_parts_mut(o, n as usize));
        true
    }
}
unsafe extern "C" fn p_leer(_: *const clap_plugin, _: clap_id, s: *const c_char, o: *mut f64) -> bool {
    unsafe {
        let Some(v) = CStr::from_ptr(s).to_str().ok().and_then(|s| s.trim().parse::<f64>().ok()) else { return false };
        *o = v;
        true
    }
}
unsafe extern "C" fn p_pasar(p: *const clap_plugin, e: *const clap_input_events, s: *const clap_output_events) {
    unsafe {
        let u = uno(p);
        if let (Some(l), Some(n)) = (e.as_ref(), &u.n) {
            let mut v: Vec<En> = vec![];
            for i in 0..l.size.map_or(0, |f| f(l)) {
                if let Some(x) = l.get.map(|f| f(l, i)).filter(|x| !x.is_null()) {
                    evento(n, &mut v, x);
                }
            }
        }
        avisar(u, s);
    }
}
static CONTROLES: clap_plugin_params = clap_plugin_params {
    count: Some(p_cuantos),
    get_info: Some(p_info),
    get_value: Some(p_valor),
    value_to_text: Some(p_texto),
    text_to_value: Some(p_leer),
    flush: Some(p_pasar),
};

// ---- Estado ----
unsafe extern "C" fn e_guardar(p: *const clap_plugin, s: *const clap_ostream) -> bool {
    seguro(false, || unsafe {
        let Some(n) = &uno(p).n else { return false };
        let b = n.a_texto();
        let mut i = 0;
        while i < b.len() {
            let k = (*s).write.map_or(-1, |w| w(s, b[i..].as_ptr() as *const c_void, (b.len() - i) as u64));
            if k <= 0 {
                return false;
            }
            i += k as usize;
        }
        true
    })
}
unsafe extern "C" fn e_abrir(p: *const clap_plugin, s: *const clap_istream) -> bool {
    seguro(false, || unsafe {
        let Some(n) = &uno(p).n else { return false };
        let (mut b, mut t) = (vec![], [0u8; 4096]);
        loop {
            let k = (*s).read.map_or(-1, |r| r(s, t.as_mut_ptr() as *mut c_void, t.len() as u64));
            if k < 0 || b.len() > 16 << 20 {
                return false;
            }
            if k == 0 {
                break;
            }
            b.extend_from_slice(&t[..k as usize]);
        }
        n.de_texto(&b)
    })
}
static ESTADO: clap_plugin_state = clap_plugin_state { save: Some(e_guardar), load: Some(e_abrir) };

unsafe extern "C" fn tarda(p: *const clap_plugin) -> u32 {
    unsafe { uno(p).n.as_ref().map_or(0, |n| n.latencia) }
}
static LATENCIA: clap_plugin_latency = clap_plugin_latency { get: Some(tarda) };

// ---- Ventana ----
mod gui {
    use super::*;
    const API: &CStr = if cfg!(target_os = "macos") {
        CLAP_WINDOW_API_COCOA
    } else if cfg!(target_os = "windows") {
        CLAP_WINDOW_API_WIN32
    } else {
        CLAP_WINDOW_API_X11
    };
    unsafe extern "C" fn sirve(_: *const clap_plugin, api: *const c_char, flota: bool) -> bool {
        unsafe { !flota && CStr::from_ptr(api) == API }
    }
    unsafe extern "C" fn cual(_: *const clap_plugin, api: *mut *const c_char, flota: *mut bool) -> bool {
        unsafe {
            *api = API.as_ptr();
            *flota = false;
        }
        true
    }
    unsafe extern "C" fn crear(p: *const clap_plugin, api: *const c_char, flota: bool) -> bool {
        seguro(false, || unsafe {
            if !sirve(p, api, flota) {
                return false;
            }
            let u = uno(p);
            let Some(n) = &u.n else { return false };
            // La instancia vive en su caja mientras viva el plugin; la ventana se suelta antes.
            let n: &'static Instancia = &*(&**n as *const Instancia);
            u.ventana = crate::ventana::Ventana::nueva(n);
            u.ventana.is_some()
        })
    }
    unsafe extern "C" fn soltar(p: *const clap_plugin) {
        seguro((), || unsafe {
            let u = uno(p);
            #[cfg(target_os = "linux")]
            if let Some(id) = u.reloj.take() {
                let t = (*u.host).get_extension.map_or(null(), |f| f(u.host, CLAP_EXT_TIMER_SUPPORT.as_ptr())) as *const clap_host_timer_support;
                if let Some(f) = t.as_ref().and_then(|t| t.unregister_timer) {
                    f(u.host, id);
                }
            }
            u.ventana = None;
        })
    }
    unsafe extern "C" fn escala(_: *const clap_plugin, _: f64) -> bool {
        false
    }
    unsafe extern "C" fn tam(p: *const clap_plugin, w: *mut u32, h: *mut u32) -> bool {
        unsafe {
            let Some(v) = &uno(p).ventana else { return false };
            (*w, *h) = v.tam();
            true
        }
    }
    unsafe extern "C" fn fija(_: *const clap_plugin) -> bool {
        false
    }
    unsafe extern "C" fn pistas(_: *const clap_plugin, _: *mut clap_gui_resize_hints) -> bool {
        false
    }
    unsafe extern "C" fn ajusta(p: *const clap_plugin, w: *mut u32, h: *mut u32) -> bool {
        unsafe { tam(p, w, h) }
    }
    unsafe extern "C" fn poner(_: *const clap_plugin, _: u32, _: u32) -> bool {
        true
    }
    unsafe extern "C" fn padre(p: *const clap_plugin, w: *const clap_window) -> bool {
        seguro(false, || unsafe {
            let u = uno(p);
            let Some(v) = &mut u.ventana else { return false };
            #[cfg(target_os = "macos")]
            let padre = (*w).specific.cocoa;
            #[cfg(target_os = "windows")]
            let padre = (*w).specific.win32;
            #[cfg(target_os = "linux")]
            let padre = (*w).specific.x11 as usize as *mut c_void;
            if !v.pegar(padre) {
                return false;
            }
            #[cfg(target_os = "linux")]
            {
                // Sin un reloj del otro programa no hay quien dibuje.
                let t = (*u.host).get_extension.map_or(null(), |f| f(u.host, CLAP_EXT_TIMER_SUPPORT.as_ptr())) as *const clap_host_timer_support;
                let mut id = 0;
                if t.as_ref().and_then(|t| t.register_timer).is_some_and(|f| f(u.host, 16, &mut id)) {
                    u.reloj = Some(id);
                }
            }
            true
        })
    }
    unsafe extern "C" fn transitoria(_: *const clap_plugin, _: *const clap_window) -> bool {
        false
    }
    unsafe extern "C" fn titulo(_: *const clap_plugin, _: *const c_char) {}
    unsafe extern "C" fn ver(_: *const clap_plugin) -> bool {
        true
    }
    pub static GUI: clap_plugin_gui = clap_plugin_gui {
        is_api_supported: Some(sirve),
        get_preferred_api: Some(cual),
        create: Some(crear),
        destroy: Some(soltar),
        set_scale: Some(escala),
        get_size: Some(tam),
        can_resize: Some(fija),
        get_resize_hints: Some(pistas),
        adjust_size: Some(ajusta),
        set_size: Some(poner),
        set_parent: Some(padre),
        set_transient: Some(transitoria),
        suggest_title: Some(titulo),
        show: Some(ver),
        hide: Some(ver),
    };
}

#[cfg(target_os = "linux")]
unsafe extern "C" fn al_reloj(p: *const clap_plugin, _: clap_id) {
    seguro((), || unsafe {
        if let Some(v) = &mut uno(p).ventana {
            v.latido();
        }
    })
}
#[cfg(target_os = "linux")]
static RELOJ: clap_plugin_timer_support = clap_plugin_timer_support { on_timer: Some(al_reloj) };

unsafe extern "C" fn extension(_: *const clap_plugin, id: *const c_char) -> *const c_void {
    unsafe {
        let id = CStr::from_ptr(id);
        if id == CLAP_EXT_AUDIO_PORTS {
            return &PUERTOS as *const _ as _;
        }
        if id == CLAP_EXT_NOTE_PORTS {
            return &NOTAS as *const _ as _;
        }
        if id == CLAP_EXT_PARAMS {
            return &CONTROLES as *const _ as _;
        }
        if id == CLAP_EXT_STATE {
            return &ESTADO as *const _ as _;
        }
        if id == CLAP_EXT_LATENCY {
            return &LATENCIA as *const _ as _;
        }
        #[cfg(target_os = "linux")]
        if id == CLAP_EXT_TIMER_SUPPORT {
            return &RELOJ as *const _ as _;
        }
        if id == CLAP_EXT_GUI {
            return &gui::GUI as *const _ as _;
        }
        null()
    }
}

// ---- Fábrica ----
unsafe extern "C" fn cuantos(_: *const clap_plugin_factory) -> u32 {
    seguro(0, || descriptores().len() as u32)
}
unsafe extern "C" fn descriptor(_: *const clap_plugin_factory, i: u32) -> *const clap_plugin_descriptor {
    seguro(null(), || descriptores().get(i as usize).map_or(null(), |d| &d.0))
}
unsafe extern "C" fn crear(_: *const clap_plugin_factory, host: *const clap_host, id: *const c_char) -> *const clap_plugin {
    seguro(null(), || unsafe {
        let id: CString = CStr::from_ptr(id).into();
        let Some(k) = todos().iter().position(|c| c.c_id == id) else { return null() };
        let u = Box::into_raw(Box::new(Uno {
            plugin: clap_plugin {
                desc: &descriptores()[k].0,
                plugin_data: null_mut(),
                init: Some(iniciar),
                destroy: Some(destruir),
                activate: Some(activar),
                deactivate: Some(nada),
                start_processing: Some(si),
                stop_processing: Some(nada),
                reset: Some(nada),
                process: Some(procesar),
                get_extension: Some(extension),
                on_main_thread: Some(principal),
            },
            host,
            cual: &todos()[k],
            n: None,
            eventos: Vec::with_capacity(1024),
            movidos: 0,
            silencio: vec![],
            #[cfg(target_os = "linux")]
            reloj: None,
            ventana: None,
        }));
        (*u).plugin.plugin_data = u as *mut c_void;
        &(*u).plugin
    })
}
static FABRICA: clap_plugin_factory =
    clap_plugin_factory { get_plugin_count: Some(cuantos), get_plugin_descriptor: Some(descriptor), create_plugin: Some(crear) };

unsafe extern "C" fn arranca(_: *const c_char) -> bool {
    true
}
unsafe extern "C" fn termina() {}
unsafe extern "C" fn fabrica(id: *const c_char) -> *const c_void {
    unsafe { if CStr::from_ptr(id) == CLAP_PLUGIN_FACTORY_ID { &FABRICA as *const _ as _ } else { null() } }
}

#[unsafe(no_mangle)]
pub static clap_entry: clap_plugin_entry =
    clap_plugin_entry { clap_version: CLAP_VERSION, init: Some(arranca), deinit: Some(termina), get_factory: Some(fabrica) };
