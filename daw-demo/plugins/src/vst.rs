// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! El lado VST3: un solo objeto hace de procesador y de controlador.
use crate::{Cual, a01, de01, escrito, seguro, todos};
use beethoven_daw_demo::instancia::{En, Instancia};
use beethoven_daw_demo::plugin::{Evento, Tiempo};
use std::cell::{RefCell, UnsafeCell};
use std::ffi::{c_char, c_void};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
#[cfg(target_os = "linux")]
use vst3::Steinberg::Linux::IRunLoopTrait;
use vst3::{Class, ComPtr, ComRef, ComWrapper, Steinberg::Vst::*, Steinberg::*};

fn corta(s: &str, d: &mut [c_char]) {
    let mut n = s.len().min(d.len() - 1);
    while !s.is_char_boundary(n) {
        n -= 1;
    }
    for (a, b) in d.iter_mut().zip(&s.as_bytes()[..n]) {
        *a = *b as c_char;
    }
    d[n] = 0;
}

fn larga(s: &str, d: &mut [TChar]) {
    let n = d.len();
    let mut k = 0;
    for (a, b) in d[..n - 1].iter_mut().zip(s.encode_utf16()) {
        *a = b as TChar;
        k += 1;
    }
    d[k] = 0;
}

fn id(c: &Cual) -> TUID {
    c.id.map(|b| b as c_char)
}

/// Se arma al saber la frecuencia; antes solo hace falta para decir sus controles.
struct Todo {
    cual: &'static Cual,
    n: UnsafeCell<Option<Arc<Instancia>>>,
    eventos: UnsafeCell<Vec<En>>,
    movidos: AtomicU64,
    reinicia: AtomicBool,
    manejador: RefCell<Option<ComPtr<IComponentHandler>>>,
}

impl Class for Todo {
    type Interfaces = (IComponent, IAudioProcessor, IEditController, IProcessContextRequirements);
}

impl Todo {
    fn n(&self) -> Option<&Arc<Instancia>> {
        // Solo cambia en `initialize` y `setupProcessing`, cuando no suena.
        unsafe { (*self.n.get()).as_ref() }
    }
}

impl IPluginBaseTrait for Todo {
    unsafe fn initialize(&self, _: *mut FUnknown) -> tresult {
        seguro(kResultFalse, || {
            let x = Instancia::nueva(&self.cual.clave, 48000.0, Default::default()).map(Arc::new);
            let bien = x.is_some();
            *self.n.get() = x;
            if bien { kResultOk } else { kResultFalse }
        })
    }

    unsafe fn terminate(&self) -> tresult {
        *self.manejador.borrow_mut() = None;
        seguro(kResultOk, || {
            *self.n.get() = None;
            kResultOk
        })
    }
}

impl IComponentTrait for Todo {
    unsafe fn getControllerClassId(&self, _: *mut TUID) -> tresult {
        // Es el mismo objeto.
        kNotImplemented
    }
    unsafe fn setIoMode(&self, _: IoMode) -> tresult {
        kResultOk
    }
    unsafe fn getBusCount(&self, m: MediaType, d: BusDirection) -> i32 {
        let entra = d as BusDirections == BusDirections_::kInput;
        match (m as MediaTypes == MediaTypes_::kAudio, entra, self.cual.instrumento()) {
            (true, true, false) => 2,
            (true, true, true) => 0,
            (true, false, _) => 1,
            (false, true, true) => 1,
            _ => 0,
        }
    }
    unsafe fn getBusInfo(&self, m: MediaType, d: BusDirection, i: i32, b: *mut BusInfo) -> tresult {
        if i < 0 || i >= self.getBusCount(m, d) {
            return kInvalidArgument;
        }
        let b = &mut *b;
        let audio = m as MediaTypes == MediaTypes_::kAudio;
        (b.mediaType, b.direction) = (m, d);
        b.channelCount = if audio { 2 } else { 1 };
        larga(
            if !audio {
                "Notes"
            } else if i == 1 {
                "Sidechain"
            } else {
                "Main"
            },
            &mut b.name,
        );
        b.busType = if i == 0 { BusTypes_::kMain } else { BusTypes_::kAux } as BusType;
        b.flags = if i == 0 { BusInfo_::BusFlags_::kDefaultActive as u32 } else { 0 };
        kResultOk
    }
    unsafe fn getRoutingInfo(&self, _: *mut RoutingInfo, _: *mut RoutingInfo) -> tresult {
        kNotImplemented
    }
    unsafe fn activateBus(&self, _: MediaType, _: BusDirection, _: i32, _: TBool) -> tresult {
        kResultOk
    }
    unsafe fn setActive(&self, _: TBool) -> tresult {
        kResultOk
    }
    unsafe fn setState(&self, s: *mut IBStream) -> tresult {
        seguro(kResultFalse, || {
            let (Some(s), Some(n)) = (ComRef::from_raw(s), self.n()) else { return kResultFalse };
            let (mut b, mut t) = (vec![], [0u8; 4096]);
            loop {
                let mut k = 0;
                if s.read(t.as_mut_ptr() as *mut c_void, t.len() as i32, &mut k) != kResultOk || k <= 0 || b.len() > 16 << 20 {
                    break;
                }
                b.extend_from_slice(&t[..k as usize]);
            }
            if n.de_texto(&b) { kResultOk } else { kResultFalse }
        })
    }
    unsafe fn getState(&self, s: *mut IBStream) -> tresult {
        seguro(kResultFalse, || {
            let (Some(s), Some(n)) = (ComRef::from_raw(s), self.n()) else { return kResultFalse };
            let b = n.a_texto();
            let mut k = 0;
            s.write(b.as_ptr() as *mut c_void, b.len() as i32, &mut k)
        })
    }
}

impl IAudioProcessorTrait for Todo {
    unsafe fn setBusArrangements(&self, e: *mut SpeakerArrangement, ne: i32, s: *mut SpeakerArrangement, ns: i32) -> tresult {
        let bien = |p: *mut SpeakerArrangement, n: i32| (0..n).all(|i| *p.add(i as usize) == SpeakerArr::kStereo);
        if ns == 1 && ne <= 2 && bien(e, ne) && bien(s, ns) { kResultTrue } else { kResultFalse }
    }
    unsafe fn getBusArrangement(&self, d: BusDirection, i: i32, a: *mut SpeakerArrangement) -> tresult {
        if i < 0 || i >= self.getBusCount(MediaTypes_::kAudio as MediaType, d) {
            return kInvalidArgument;
        }
        *a = SpeakerArr::kStereo;
        kResultOk
    }
    unsafe fn canProcessSampleSize(&self, s: i32) -> tresult {
        if s as SymbolicSampleSizes == SymbolicSampleSizes_::kSample32 { kResultOk } else { kResultFalse }
    }
    unsafe fn getLatencySamples(&self) -> u32 {
        self.n().map_or(0, |n| n.latencia)
    }
    unsafe fn setupProcessing(&self, s: *mut ProcessSetup) -> tresult {
        seguro(kResultFalse, || {
            let sr = (*s).sampleRate as f32;
            if let Some(n) = self.n().filter(|n| n.sr != sr) {
                match Instancia::nueva(&self.cual.clave, sr, n.guardado()) {
                    Some(x) => *self.n.get() = Some(Arc::new(x)),
                    None => return kResultFalse,
                }
            }
            kResultOk
        })
    }
    unsafe fn setProcessing(&self, _: TBool) -> tresult {
        kResultOk
    }
    unsafe fn process(&self, d: *mut ProcessData) -> tresult {
        let d = &*d;
        let Some(n) = self.n() else { return kResultFalse };
        if let Some(c) = ComRef::from_raw(d.inputParameterChanges) {
            for i in 0..c.getParameterCount() {
                let Some(q) = ComRef::from_raw(c.getParameterData(i)) else { continue };
                let (mut en, mut v) = (0, 0.0);
                let k = q.getParameterId() as usize;
                if q.getPointCount() > 0
                    && q.getPoint(q.getPointCount() - 1, &mut en, &mut v) == kResultOk
                    && let Some(x) = n.receta.controles.get(k)
                {
                    n.poner(k, de01(x, v));
                }
            }
        }
        let ev = &mut *self.eventos.get();
        ev.clear();
        if let Some(l) = ComRef::from_raw(d.inputEvents) {
            for i in 0..l.getEventCount() {
                let mut e: Event = std::mem::zeroed();
                if l.getEvent(i, &mut e) != kResultOk || ev.len() == ev.capacity() {
                    continue;
                }
                let en = e.sampleOffset.max(0) as usize;
                match e.r#type as u32 {
                    t if t == Event_::EventTypes_::kNoteOnEvent as u32 => {
                        let q = e.__field0.noteOn;
                        ev.push((en, Evento::Nota { tono: q.pitch.clamp(0, 127) as u8, fuerza: q.velocity }));
                    }
                    t if t == Event_::EventTypes_::kNoteOffEvent as u32 => {
                        ev.push((en, Evento::Suelta { tono: e.__field0.noteOff.pitch.clamp(0, 127) as u8 }));
                    }
                    _ => {}
                }
            }
        }
        ev.sort_unstable_by_key(|e| e.0);
        let largo = d.numSamples as usize;
        if d.numOutputs < 1 || largo == 0 {
            return kResultOk;
        }
        let o = &*d.outputs;
        if o.numChannels < 2 {
            return kResultOk;
        }
        let entra = |k: usize| -> [&[f32]; 2] {
            if d.inputs.is_null() || k >= d.numInputs as usize {
                return [&[], &[]];
            }
            let b = &*d.inputs.add(k);
            if b.numChannels < 1 || b.__field0.channelBuffers32.is_null() {
                return [&[], &[]];
            }
            [0, 1].map(|c: i32| {
                let p = *b.__field0.channelBuffers32.add(c.min(b.numChannels - 1) as usize);
                if p.is_null() { &[][..] } else { std::slice::from_raw_parts(p, largo) }
            })
        };
        let mut t = Tiempo { bpm: 120.0, pos: 0.0, tocando: false, sr: n.sr as f64 };
        if let Some(c) = d.processContext.as_ref() {
            if c.state & 1 << 10 != 0 && c.tempo > 1.0 {
                t.bpm = c.tempo;
            }
            if c.state & 1 << 9 != 0 {
                t.pos = c.projectTimeMusic;
            }
            t.tocando = c.state & 2 != 0;
        }
        let sale = [0, 1].map(|c| std::slice::from_raw_parts_mut(*o.__field0.channelBuffers32.add(c), largo));
        n.procesar(ev, entra(0), entra(1), sale, &t);
        kResultOk
    }
    unsafe fn getTailSamples(&self) -> u32 {
        (self.n().map_or(48000.0, |n| n.sr) * 4.0) as u32
    }
}

impl IProcessContextRequirementsTrait for Todo {
    unsafe fn getProcessContextRequirements(&self) -> u32 {
        // Tempo, posición en negras y si toca.
        1 << 2 | 1 << 5 | 1 << 10
    }
}

impl IEditControllerTrait for Todo {
    unsafe fn setComponentState(&self, _: *mut IBStream) -> tresult {
        kResultOk
    }
    unsafe fn setState(&self, _: *mut IBStream) -> tresult {
        kResultOk
    }
    unsafe fn getState(&self, _: *mut IBStream) -> tresult {
        kResultOk
    }
    unsafe fn getParameterCount(&self) -> i32 {
        self.n().map_or(0, |n| n.receta.controles.len() as i32)
    }
    unsafe fn getParameterInfo(&self, i: i32, o: *mut ParameterInfo) -> tresult {
        let Some(c) = self.n().and_then(|n| n.receta.controles.get(i as usize)) else { return kInvalidArgument };
        let o = &mut *o;
        o.id = i as u32;
        larga(c.nombre.ver(), &mut o.title);
        larga(c.nombre.ver(), &mut o.shortTitle);
        larga("", &mut o.units);
        o.stepCount = 0;
        o.defaultNormalizedValue = a01(c, c.valor);
        o.unitId = 0;
        o.flags = ParameterInfo_::ParameterFlags_::kCanAutomate as i32;
        kResultOk
    }
    unsafe fn getParamStringByValue(&self, k: u32, v: f64, s: *mut String128) -> tresult {
        let Some(c) = self.n().and_then(|n| n.receta.controles.get(k as usize)) else { return kInvalidArgument };
        larga(&escrito(de01(c, v)), &mut *s);
        kResultOk
    }
    unsafe fn getParamValueByString(&self, k: u32, s: *mut TChar, v: *mut f64) -> tresult {
        let Some(c) = self.n().and_then(|n| n.receta.controles.get(k as usize)) else { return kInvalidArgument };
        let mut n = 0;
        while n < 127 && *s.add(n) != 0 {
            n += 1;
        }
        let t = String::from_utf16_lossy(std::slice::from_raw_parts(s as *const u16, n));
        let Ok(x) = t.trim().parse::<f32>() else { return kResultFalse };
        *v = a01(c, x.clamp(c.min, c.max));
        kResultOk
    }
    unsafe fn normalizedParamToPlain(&self, k: u32, v: f64) -> f64 {
        self.n().and_then(|n| n.receta.controles.get(k as usize)).map_or(v, |c| de01(c, v) as f64)
    }
    unsafe fn plainParamToNormalized(&self, k: u32, v: f64) -> f64 {
        self.n().and_then(|n| n.receta.controles.get(k as usize)).map_or(v, |c| a01(c, v as f32))
    }
    unsafe fn getParamNormalized(&self, k: u32) -> f64 {
        self.n().and_then(|n| n.receta.controles.get(k as usize).map(|c| a01(c, n.valor(k as usize)))).unwrap_or(0.0)
    }
    unsafe fn setParamNormalized(&self, k: u32, v: f64) -> tresult {
        seguro(kResultFalse, || {
            let Some(n) = self.n() else { return kResultFalse };
            let Some(c) = n.receta.controles.get(k as usize) else { return kInvalidArgument };
            // Los otros programas devuelven aquí lo que la ventana ya movió: si es lo mismo, no se toca.
            if (a01(c, n.valor(k as usize)) - v).abs() > 1e-6 {
                n.poner(k as usize, de01(c, v));
                n.poner_al_dia();
            }
            kResultOk
        })
    }
    unsafe fn setComponentHandler(&self, h: *mut IComponentHandler) -> tresult {
        *self.manejador.borrow_mut() = ComRef::from_raw(h).map(|h| h.to_com_ptr());
        kResultOk
    }
    unsafe fn createView(&self, _n: *const c_char) -> *mut IPlugView {
        {
            let Some(n) = self.n() else { return std::ptr::null_mut() };
            let manda = self.manejador.borrow().clone();
            seguro(std::ptr::null_mut(), || {
                let v = Vista {
                    n: n.clone(),
                    v: Rc::new(RefCell::new(None)),
                    manda,
                    #[cfg(target_os = "linux")]
                    marco: RefCell::new(None),
                    #[cfg(target_os = "linux")]
                    reloj: RefCell::new(None),
                };
                ComWrapper::new(v).to_com_ptr::<IPlugView>().map_or(std::ptr::null_mut(), |p| p.into_raw())
            })
        }
    }
}

struct Vista {
    // La ventana se suelta antes que lo que muestra.
    v: Rc<RefCell<Option<crate::ventana::Ventana>>>,
    n: Arc<Instancia>,
    manda: Option<ComPtr<IComponentHandler>>,
    /// En Linux el otro programa presta su reloj.
    #[cfg(target_os = "linux")]
    marco: RefCell<Option<ComPtr<IPlugFrame>>>,
    #[cfg(target_os = "linux")]
    reloj: RefCell<Option<(ComPtr<Linux::IRunLoop>, ComPtr<Linux::ITimerHandler>)>>,
}

#[cfg(target_os = "linux")]
struct Reloj(Rc<RefCell<Option<crate::ventana::Ventana>>>);

#[cfg(target_os = "linux")]
impl Class for Reloj {
    type Interfaces = (Linux::ITimerHandler,);
}

#[cfg(target_os = "linux")]
impl Linux::ITimerHandlerTrait for Reloj {
    unsafe fn onTimer(&self) {
        seguro((), || {
            if let Ok(mut v) = self.0.try_borrow_mut()
                && let Some(v) = v.as_mut()
            {
                v.latido();
            }
        })
    }
}

/// Cómo se llama en VST3 la ventana de cada sistema.
const TIPO: &std::ffi::CStr = if cfg!(target_os = "macos") {
    c"NSView"
} else if cfg!(target_os = "windows") {
    c"HWND"
} else {
    c"X11EmbedWindowID"
};

impl Class for Vista {
    type Interfaces = (IPlugView,);
}

impl IPlugViewTrait for Vista {
    unsafe fn isPlatformTypeSupported(&self, t: FIDString) -> tresult {
        if std::ffi::CStr::from_ptr(t) == TIPO { kResultTrue } else { kResultFalse }
    }
    unsafe fn attached(&self, padre: *mut c_void, t: FIDString) -> tresult {
        if self.isPlatformTypeSupported(t) != kResultTrue {
            return kResultFalse;
        }
        seguro(kResultFalse, || {
            let n: &'static Instancia = &*Arc::as_ptr(&self.n);
            let Some(mut v) = crate::ventana::Ventana::nueva(n) else { return kResultFalse };
            // Lo que se mueve en la ventana se le cuenta al otro programa.
            if let Some(m) = self.manda.clone() {
                let mut vistos: Vec<f32> = (0..n.receta.controles.len()).map(|k| n.valor(k)).collect();
                let mut movidos = n.movidos.load(Relaxed);
                v.al_cuadro(Box::new(move || {
                    let ahora = n.movidos.load(Relaxed);
                    if ahora == movidos {
                        return;
                    }
                    movidos = ahora;
                    for (k, c) in n.receta.controles.iter().enumerate() {
                        let x = n.valor(k);
                        if x != vistos[k] {
                            vistos[k] = x;
                            m.beginEdit(k as u32);
                            m.performEdit(k as u32, a01(c, x));
                            m.endEdit(k as u32);
                        }
                    }
                }));
            }
            if !v.pegar(padre) {
                return kResultFalse;
            }
            *self.v.borrow_mut() = Some(v);
            #[cfg(target_os = "linux")]
            if let Some(l) = self.marco.borrow().as_ref().and_then(|m| m.cast::<Linux::IRunLoop>())
                && let Some(r) = ComWrapper::new(Reloj(self.v.clone())).to_com_ptr::<Linux::ITimerHandler>()
            {
                l.registerTimer(r.as_ptr(), 16);
                *self.reloj.borrow_mut() = Some((l, r));
            }
            kResultOk
        })
    }
    unsafe fn removed(&self) -> tresult {
        seguro(kResultOk, || {
            #[cfg(target_os = "linux")]
            if let Some((l, r)) = self.reloj.borrow_mut().take() {
                l.unregisterTimer(r.as_ptr());
            }
            *self.v.borrow_mut() = None;
            kResultOk
        })
    }
    unsafe fn onWheel(&self, _: f32) -> tresult {
        kResultFalse
    }
    unsafe fn onKeyDown(&self, _: char16, _: int16, _: int16) -> tresult {
        kResultFalse
    }
    unsafe fn onKeyUp(&self, _: char16, _: int16, _: int16) -> tresult {
        kResultFalse
    }
    unsafe fn getSize(&self, r: *mut ViewRect) -> tresult {
        // Con la ventana abierta, lo que mide de verdad; antes, lo que va a medir.
        let abierta = self.v.try_borrow().ok().and_then(|v| v.as_ref().map(|v| v.tam()));
        let n: &'static Instancia = &*Arc::as_ptr(&self.n);
        let (w, h) = abierta.unwrap_or_else(|| crate::ventana::medida(n));
        *r = ViewRect { left: 0, top: 0, right: w as i32, bottom: h as i32 };
        kResultOk
    }
    unsafe fn onSize(&self, _: *mut ViewRect) -> tresult {
        kResultOk
    }
    unsafe fn onFocus(&self, _: TBool) -> tresult {
        kResultOk
    }
    unsafe fn setFrame(&self, _f: *mut IPlugFrame) -> tresult {
        #[cfg(target_os = "linux")]
        {
            *self.marco.borrow_mut() = ComRef::from_raw(_f).map(|f| f.to_com_ptr());
        }
        kResultOk
    }
    unsafe fn canResize(&self) -> tresult {
        kResultFalse
    }
    unsafe fn checkSizeConstraint(&self, r: *mut ViewRect) -> tresult {
        self.getSize(r)
    }
}

struct Fabrica;

impl Class for Fabrica {
    type Interfaces = (IPluginFactory, IPluginFactory2);
}

impl IPluginFactoryTrait for Fabrica {
    unsafe fn getFactoryInfo(&self, i: *mut PFactoryInfo) -> tresult {
        let i = &mut *i;
        corta("Beethoven", &mut i.vendor);
        corta("", &mut i.url);
        corta("", &mut i.email);
        i.flags = PFactoryInfo_::FactoryFlags_::kUnicode as int32;
        kResultOk
    }
    unsafe fn countClasses(&self) -> i32 {
        seguro(0, || todos().len() as i32)
    }
    unsafe fn getClassInfo(&self, k: i32, i: *mut PClassInfo) -> tresult {
        seguro(kInvalidArgument, || {
            let Some(c) = todos().get(k as usize) else { return kInvalidArgument };
            let i = &mut *i;
            i.cid = id(c);
            i.cardinality = PClassInfo_::ClassCardinality_::kManyInstances as int32;
            corta("Audio Module Class", &mut i.category);
            corta(&c.nombre, &mut i.name);
            kResultOk
        })
    }
    unsafe fn createInstance(&self, cid: FIDString, iid: FIDString, o: *mut *mut c_void) -> tresult {
        seguro(kInvalidArgument, || {
            let cid = *(cid as *const TUID);
            let Some(c) = todos().iter().find(|c| id(c) == cid) else { return kInvalidArgument };
            let t = Todo {
                cual: c,
                n: UnsafeCell::new(None),
                eventos: UnsafeCell::new(Vec::with_capacity(1024)),
                movidos: AtomicU64::new(0),
                reinicia: AtomicBool::new(false),
                manejador: RefCell::new(None),
            };
            let _ = (&t.movidos, &t.reinicia);
            let Some(p) = ComWrapper::new(t).to_com_ptr::<FUnknown>() else { return kInvalidArgument };
            let p = p.as_ptr();
            ((*(*p).vtbl).queryInterface)(p, iid as *const TUID, o)
        })
    }
}

impl IPluginFactory2Trait for Fabrica {
    unsafe fn getClassInfo2(&self, k: i32, i: *mut PClassInfo2) -> tresult {
        seguro(kInvalidArgument, || {
            let Some(c) = todos().get(k as usize) else { return kInvalidArgument };
            let i = &mut *i;
            i.cid = id(c);
            i.cardinality = PClassInfo_::ClassCardinality_::kManyInstances as int32;
            corta("Audio Module Class", &mut i.category);
            corta(&c.nombre, &mut i.name);
            i.classFlags = 0;
            corta(if c.instrumento() { "Instrument|Synth" } else { "Fx" }, &mut i.subCategories);
            corta("Beethoven", &mut i.vendor);
            corta("1.0.0", &mut i.version);
            corta("VST 3.7", &mut i.sdkVersion);
            kResultOk
        })
    }
}

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
extern "system" fn InitDll() -> bool {
    true
}
#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
extern "system" fn ExitDll() -> bool {
    true
}
#[cfg(target_os = "macos")]
#[unsafe(no_mangle)]
extern "system" fn bundleEntry(_: *mut c_void) -> bool {
    true
}
#[cfg(target_os = "macos")]
#[unsafe(no_mangle)]
extern "system" fn bundleExit() -> bool {
    true
}
#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
extern "system" fn ModuleEntry(_: *mut c_void) -> bool {
    true
}
#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
extern "system" fn ModuleExit() -> bool {
    true
}
#[unsafe(no_mangle)]
extern "system" fn GetPluginFactory() -> *mut IPluginFactory {
    ComWrapper::new(Fabrica).to_com_ptr::<IPluginFactory>().map_or(std::ptr::null_mut(), |p| p.into_raw())
}
