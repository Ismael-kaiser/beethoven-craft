// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
use super::{ControlP, EVENTOS, Evento, Oferta, Puertos, Tiempo, binario};
use crate::idioma::t;
use crate::receta::{BLOQUE, Buf};
use crossbeam_channel::Sender;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::ffi::{c_char, c_void};
use std::path::Path;
use std::ptr::null_mut;
use std::sync::Mutex;
use vst3::Steinberg::Vst::{Event as EventV, *};
use vst3::Steinberg::*;
use vst3::{Class, ComPtr, ComWrapper, Interface};

static ABIERTAS: Mutex<Vec<(std::path::PathBuf, usize)>> = Mutex::new(vec![]);

#[cfg(target_os = "macos")]
fn paquete(ruta: &Path) -> *mut c_void {
    use std::os::unix::ffi::OsStrExt;
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFURLCreateFromFileSystemRepresentation(a: *const c_void, b: *const u8, n: isize, dir: u8) -> *const c_void;
        fn CFBundleCreate(a: *const c_void, url: *const c_void) -> *mut c_void;
        fn CFRelease(a: *const c_void);
    }
    let b = ruta.as_os_str().as_bytes();
    unsafe {
        let u = CFURLCreateFromFileSystemRepresentation(std::ptr::null(), b.as_ptr(), b.len() as isize, 1);
        if u.is_null() {
            return null_mut();
        }
        let p = CFBundleCreate(std::ptr::null(), u);
        CFRelease(u);
        p
    }
}

fn fabrica(ruta: &Path) -> Result<ComPtr<IPluginFactory>, String> {
    let mut v = ABIERTAS.lock().unwrap();
    let e = |e: libloading::Error| e.to_string();
    unsafe {
        let p = match v.iter().find(|x| x.0 == ruta) {
            Some((_, f)) => *f as *mut IPluginFactory,
            None => {
                let lib = libloading::Library::new(binario(ruta)?).map_err(e)?;
                #[cfg(target_os = "macos")]
                {
                    let entra = lib.get::<unsafe extern "C" fn(*mut c_void) -> bool>(b"bundleEntry\0").map_err(e)?;
                    if !entra(paquete(ruta)) {
                        return Err(t("El plugin no arrancó").into());
                    }
                }
                #[cfg(target_os = "linux")]
                if let Ok(entra) = lib.get::<unsafe extern "C" fn(*mut c_void) -> bool>(b"ModuleEntry\0") {
                    entra(null_mut());
                }
                #[cfg(windows)]
                if let Ok(entra) = lib.get::<unsafe extern "system" fn() -> bool>(b"InitDll\0") {
                    entra();
                }
                let f = lib.get::<unsafe extern "system" fn() -> *mut IPluginFactory>(b"GetPluginFactory\0").map_err(e)?();
                std::mem::forget(lib);
                if f.is_null() {
                    return Err(t("El archivo no trae plugins").into());
                }
                v.push((ruta.into(), f as usize));
                f
            }
        };
        // La referencia del módulo se queda viva con él.
        Ok(vst3::ComRef::from_raw(p).ok_or(t("sin fábrica"))?.to_com_ptr())
    }
}

fn txt(c: &[c_char]) -> String {
    String::from_utf8_lossy(&c.iter().take_while(|x| **x != 0).map(|x| *x as u8).collect::<Vec<_>>()).into_owned()
}

fn txt16(c: &[TChar]) -> String {
    String::from_utf16_lossy(&c.iter().take_while(|x| **x != 0).map(|x| *x as u16).collect::<Vec<_>>())
}

fn clases(f: &ComPtr<IPluginFactory>) -> Vec<(TUID, String, bool)> {
    let mut v = vec![];
    unsafe {
        let f2 = f.cast::<IPluginFactory2>();
        for i in 0..f.countClasses() {
            let mut c: PClassInfo = std::mem::zeroed();
            if f.getClassInfo(i, &mut c) != kResultOk || txt(&c.category) != "Audio Module Class" {
                continue;
            }
            let mut c2: PClassInfo2 = std::mem::zeroed();
            let ins = f2.as_ref().is_some_and(|f| f.getClassInfo2(i, &mut c2) == kResultOk && txt(&c2.subCategories).contains("Instrument"));
            v.push((c.cid, txt(&c.name), ins));
        }
    }
    v
}

fn id(t: &TUID) -> String {
    t.iter().map(|b| format!("{:02X}", *b as u8)).collect()
}

pub fn ofertas(ruta: &Path) -> Result<Vec<Oferta>, String> {
    Ok(clases(&fabrica(ruta)?)
        .into_iter()
        .map(|(t, nombre, instrumento)| Oferta { ruta: ruta.to_string_lossy().into(), id: id(&t), nombre, instrumento })
        .collect())
}

// ---- Lo que el programa le presta al plugin ----

struct Anfitrion;
impl Class for Anfitrion {
    type Interfaces = (IHostApplication,);
}
impl IHostApplicationTrait for Anfitrion {
    unsafe fn getName(&self, name: *mut String128) -> tresult {
        let n = unsafe { &mut *name };
        n.fill(0);
        for (d, s) in n.iter_mut().zip("Beethoven DAW Demo".encode_utf16()) {
            *d = s as TChar;
        }
        kResultOk
    }
    unsafe fn createInstance(&self, _: *mut TUID, _: *mut TUID, obj: *mut *mut c_void) -> tresult {
        unsafe { *obj = null_mut() };
        kResultFalse
    }
}

struct Manejador(Sender<(u32, f64)>);
impl Class for Manejador {
    type Interfaces = (IComponentHandler,);
}
impl IComponentHandlerTrait for Manejador {
    unsafe fn beginEdit(&self, _: ParamID) -> tresult {
        kResultOk
    }
    unsafe fn performEdit(&self, id: ParamID, v: ParamValue) -> tresult {
        let _ = self.0.try_send((id, v));
        kResultOk
    }
    unsafe fn endEdit(&self, _: ParamID) -> tresult {
        kResultOk
    }
    unsafe fn restartComponent(&self, _: int32) -> tresult {
        kResultOk
    }
}

#[derive(Default)]
struct Flujo(RefCell<(Vec<u8>, usize)>);
impl Class for Flujo {
    type Interfaces = (IBStream,);
}
impl IBStreamTrait for Flujo {
    unsafe fn read(&self, b: *mut c_void, n: int32, leidos: *mut int32) -> tresult {
        let mut f = self.0.borrow_mut();
        let (pos, n) = (f.1, (n.max(0) as usize).min(f.0.len().saturating_sub(f.1)));
        unsafe {
            std::ptr::copy_nonoverlapping(f.0.as_ptr().add(pos), b as *mut u8, n);
            if !leidos.is_null() {
                *leidos = n as i32;
            }
        }
        f.1 += n;
        kResultOk
    }
    unsafe fn write(&self, b: *mut c_void, n: int32, escritos: *mut int32) -> tresult {
        let mut f = self.0.borrow_mut();
        let n = n.max(0) as usize;
        let pos = f.1;
        if f.0.len() < pos + n {
            f.0.resize(pos + n, 0);
        }
        unsafe {
            f.0[pos..pos + n].copy_from_slice(std::slice::from_raw_parts(b as *const u8, n));
            if !escritos.is_null() {
                *escritos = n as i32;
            }
        }
        f.1 += n;
        kResultOk
    }
    unsafe fn seek(&self, pos: int64, modo: int32, res: *mut int64) -> tresult {
        let mut f = self.0.borrow_mut();
        let base = match modo {
            0 => 0,
            1 => f.1 as i64,
            _ => f.0.len() as i64,
        };
        // Más allá del final solo se llega escribiendo.
        f.1 = (base + pos).clamp(0, f.0.len() as i64) as usize;
        unsafe {
            if !res.is_null() {
                *res = f.1 as i64;
            }
        }
        kResultOk
    }
    unsafe fn tell(&self, pos: *mut int64) -> tresult {
        unsafe { *pos = self.0.borrow().1 as i64 };
        kResultOk
    }
}

struct Eventos {
    n: Cell<usize>,
    v: UnsafeCell<[EventV; EVENTOS]>,
}
impl Class for Eventos {
    type Interfaces = (IEventList,);
}
impl Eventos {
    fn nueva() -> ComWrapper<Eventos> {
        ComWrapper::new(Eventos { n: Cell::new(0), v: UnsafeCell::new(unsafe { std::mem::zeroed() }) })
    }
    fn poner(&self, e: EventV) {
        let n = self.n.get();
        if n < EVENTOS {
            unsafe { (*self.v.get())[n] = e };
            self.n.set(n + 1);
        }
    }
}
impl IEventListTrait for Eventos {
    unsafe fn getEventCount(&self) -> int32 {
        self.n.get() as i32
    }
    unsafe fn getEvent(&self, i: int32, e: *mut EventV) -> tresult {
        if i < 0 || i as usize >= self.n.get() {
            return kInvalidArgument;
        }
        unsafe { *e = (*self.v.get())[i as usize] };
        kResultOk
    }
    unsafe fn addEvent(&self, e: *mut EventV) -> tresult {
        self.poner(unsafe { *e });
        kResultOk
    }
}

#[derive(Default)]
struct Cola {
    id: Cell<u32>,
    valor: Cell<f64>,
}
impl Class for Cola {
    type Interfaces = (IParamValueQueue,);
}
impl IParamValueQueueTrait for Cola {
    unsafe fn getParameterId(&self) -> ParamID {
        self.id.get()
    }
    unsafe fn getPointCount(&self) -> int32 {
        1
    }
    unsafe fn getPoint(&self, i: int32, cuando: *mut int32, v: *mut ParamValue) -> tresult {
        if i != 0 {
            return kInvalidArgument;
        }
        unsafe {
            *cuando = 0;
            *v = self.valor.get();
        }
        kResultOk
    }
    unsafe fn addPoint(&self, _: int32, v: ParamValue, i: *mut int32) -> tresult {
        self.valor.set(v);
        unsafe {
            if !i.is_null() {
                *i = 0;
            }
        }
        kResultOk
    }
}

/// Cambios de controles, con todas sus colas hechas de antemano.
struct Cambios {
    n: Cell<usize>,
    colas: Vec<(ComWrapper<Cola>, ComPtr<IParamValueQueue>)>,
}
impl Class for Cambios {
    type Interfaces = (IParameterChanges,);
}
impl Cambios {
    fn nuevos() -> ComWrapper<Cambios> {
        let colas = (0..EVENTOS)
            .map(|_| {
                let c = ComWrapper::new(Cola::default());
                let p = c.to_com_ptr().unwrap();
                (c, p)
            })
            .collect();
        ComWrapper::new(Cambios { n: Cell::new(0), colas })
    }
    fn poner(&self, id: u32, valor: f64) -> Option<usize> {
        let n = self.n.get();
        let i = (0..n).find(|i| self.colas[*i].0.id.get() == id).unwrap_or(n);
        if i >= EVENTOS {
            return None;
        }
        self.colas[i].0.id.set(id);
        self.colas[i].0.valor.set(valor);
        self.n.set(n.max(i + 1));
        Some(i)
    }
}
impl IParameterChangesTrait for Cambios {
    unsafe fn getParameterCount(&self) -> int32 {
        self.n.get() as i32
    }
    unsafe fn getParameterData(&self, i: int32) -> *mut IParamValueQueue {
        if i < 0 || i as usize >= self.n.get() { null_mut() } else { self.colas[i as usize].1.as_ptr() }
    }
    unsafe fn addParameterData(&self, id: *const ParamID, i: *mut int32) -> *mut IParamValueQueue {
        match self.poner(unsafe { *id }, 0.0) {
            Some(k) => {
                unsafe {
                    if !i.is_null() {
                        *i = k as i32;
                    }
                }
                self.colas[k].1.as_ptr()
            }
            None => null_mut(),
        }
    }
}

struct Marco {
    ventana: Cell<*mut c_void>,
}
impl Class for Marco {
    type Interfaces = (IPlugFrame,);
}
impl IPlugFrameTrait for Marco {
    unsafe fn resizeView(&self, vista: *mut IPlugView, r: *mut ViewRect) -> tresult {
        unsafe {
            let t = *r;
            super::ventana::medir(self.ventana.get(), (t.right - t.left).max(1) as u32, (t.bottom - t.top).max(1) as u32);
            if let Some(v) = vst3::ComRef::from_raw(vista) {
                v.onSize(r);
            }
        }
        kResultOk
    }
}

pub struct Vst3 {
    pub nombre: String,
    comp: ComPtr<IComponent>,
    proc_: ComPtr<IAudioProcessor>,
    ctl: Option<ComPtr<IEditController>>,
    ctl_aparte: bool,
    _anfitrion: ComPtr<FUnknown>,
    _manejador: ComPtr<IComponentHandler>,
    marco: ComWrapper<Marco>,
    marco_p: ComPtr<IPlugFrame>,
    vista: Option<ComPtr<IPlugView>>,
    eventos: ComWrapper<Eventos>,
    eventos_p: ComPtr<IEventList>,
    fuera: ComWrapper<Eventos>,
    fuera_p: ComPtr<IEventList>,
    cambios: ComWrapper<Cambios>,
    cambios_p: ComPtr<IParameterChanges>,
    salen: ComWrapper<Cambios>,
    salen_p: ComPtr<IParameterChanges>,
    entra: Puertos,
    sale: Puertos,
    procesando: bool,
    cuadros: i64,
}

impl Vst3 {
    pub fn cargar(ruta: &Path, cual: &str, sr: f64, cambios: Sender<(u32, f64)>) -> Result<Box<Vst3>, String> {
        let f = fabrica(ruta)?;
        let (cid, nombre, _) = clases(&f).into_iter().find(|c| id(&c.0) == cual).ok_or(t("Ese plugin ya no está en el archivo"))?;
        let anfitrion: ComPtr<FUnknown> = ComWrapper::new(Anfitrion).to_com_ptr::<IHostApplication>().unwrap().upcast();
        let manejador: ComPtr<IComponentHandler> = ComWrapper::new(Manejador(cambios)).to_com_ptr().unwrap();
        unsafe fn crear<I: Interface>(f: &ComPtr<IPluginFactory>, cid: &TUID) -> Option<ComPtr<I>> {
            let mut o = null_mut();
            unsafe {
                f.createInstance(cid.as_ptr(), I::IID.as_ptr() as *const c_char, &mut o);
                ComPtr::from_raw(o as *mut I)
            }
        }
        unsafe {
            let comp: ComPtr<IComponent> = crear(&f, &cid).ok_or(t("El plugin no se pudo crear"))?;
            if comp.initialize(anfitrion.as_ptr()) != kResultOk {
                return Err(t("El plugin no arrancó").into());
            }
            let proc_ = comp.cast::<IAudioProcessor>().ok_or(t("El plugin no procesa audio"))?;
            let mut ctl = comp.cast::<IEditController>();
            let mut ctl_aparte = false;
            if ctl.is_none() {
                let mut c: TUID = std::mem::zeroed();
                if comp.getControllerClassId(&mut c) == kResultOk {
                    ctl = crear::<IEditController>(&f, &c).filter(|x| x.initialize(anfitrion.as_ptr()) == kResultOk);
                    ctl_aparte = ctl.is_some();
                }
            }
            if let Some(c) = &ctl {
                c.setComponentHandler(manejador.as_ptr());
                if ctl_aparte && let (Some(a), Some(b)) = (comp.cast::<IConnectionPoint>(), c.cast::<IConnectionPoint>()) {
                    a.connect(b.as_ptr());
                    b.connect(a.as_ptr());
                }
                // El controlador arranca igual que el sonido.
                let fl = ComWrapper::new(Flujo::default());
                let p = fl.to_com_ptr::<IBStream>().unwrap();
                if comp.getState(p.as_ptr()) == kResultOk {
                    fl.0.borrow_mut().1 = 0;
                    c.setComponentState(p.as_ptr());
                }
            }
            let (au, ev) = (MediaTypes_::kAudio as i32, MediaTypes_::kEvent as i32);
            let (en, sa) = (BusDirections_::kInput as i32, BusDirections_::kOutput as i32);
            let mut ins = vec![SpeakerArr::kStereo; comp.getBusCount(au, en).max(0) as usize];
            let mut outs = vec![SpeakerArr::kStereo; comp.getBusCount(au, sa).max(0) as usize];
            proc_.setBusArrangements(ins.as_mut_ptr(), ins.len() as i32, outs.as_mut_ptr(), outs.len() as i32);
            let canales = |dir: i32| -> Vec<usize> {
                (0..comp.getBusCount(au, dir))
                    .map(|i| {
                        let mut b: BusInfo = std::mem::zeroed();
                        comp.getBusInfo(au, dir, i, &mut b);
                        // Solo suenan los principales, pero todos reciben su memoria.
                        comp.activateBus(au, dir, i, (i == 0) as TBool);
                        b.channelCount.clamp(0, 64) as usize
                    })
                    .collect()
            };
            let (entra, sale) = (Puertos::nuevos(&canales(en)), Puertos::nuevos(&canales(sa)));
            for dir in [en, sa] {
                for i in 0..comp.getBusCount(ev, dir) {
                    comp.activateBus(ev, dir, i, 1);
                }
            }
            let mut s = ProcessSetup {
                processMode: ProcessModes_::kRealtime as i32,
                symbolicSampleSize: SymbolicSampleSizes_::kSample32 as i32,
                maxSamplesPerBlock: BLOQUE as i32,
                sampleRate: sr,
            };
            proc_.setupProcessing(&mut s);
            if comp.setActive(1) != kResultOk {
                return Err(t("El plugin no se pudo activar").into());
            }
            let (eventos, fuera, cambios, salen) = (Eventos::nueva(), Eventos::nueva(), Cambios::nuevos(), Cambios::nuevos());
            let marco = ComWrapper::new(Marco { ventana: Cell::new(null_mut()) });
            Ok(Box::new(Vst3 {
                nombre,
                comp,
                proc_,
                ctl,
                ctl_aparte,
                _anfitrion: anfitrion,
                _manejador: manejador,
                marco_p: marco.to_com_ptr().unwrap(),
                marco,
                vista: None,
                eventos_p: eventos.to_com_ptr().unwrap(),
                eventos,
                fuera_p: fuera.to_com_ptr().unwrap(),
                fuera,
                cambios_p: cambios.to_com_ptr().unwrap(),
                cambios,
                salen_p: salen.to_com_ptr().unwrap(),
                salen,
                entra,
                sale,
                procesando: false,
                cuadros: 0,
            }))
        }
    }

    pub fn controles(&self) -> Vec<ControlP> {
        let Some(c) = &self.ctl else { return vec![] };
        let mut v = vec![];
        unsafe {
            for i in 0..c.getParameterCount() {
                let mut p: ParameterInfo = std::mem::zeroed();
                // 2: solo lectura, 16: escondido.
                if c.getParameterInfo(i, &mut p) == kResultOk && p.flags & (2 | 16) == 0 {
                    v.push(ControlP { id: p.id, nombre: txt16(&p.title), min: 0.0, max: 1.0, entero: p.stepCount > 0 });
                }
            }
        }
        v
    }

    pub fn valor(&self, id: u32) -> f64 {
        self.ctl.as_ref().map_or(0.0, |c| unsafe { c.getParamNormalized(id) })
    }

    pub fn mover(&self, id: u32, v: f64) {
        if let Some(c) = &self.ctl {
            unsafe { c.setParamNormalized(id, v) };
        }
    }

    pub fn texto(&self, id: u32, v: f64) -> String {
        let mut s: String128 = [0; 128];
        match &self.ctl {
            Some(c) if unsafe { c.getParamStringByValue(id, v, &mut s) } == kResultOk => txt16(&s),
            _ => format!("{v:.2}"),
        }
    }

    pub fn estado(&self) -> Vec<u8> {
        let toma = |f: &dyn Fn(*mut IBStream) -> tresult| -> Vec<u8> {
            let w = ComWrapper::new(Flujo::default());
            let p = w.to_com_ptr::<IBStream>().unwrap();
            if f(p.as_ptr()) == kResultOk { w.0.borrow().0.clone() } else { vec![] }
        };
        let a = toma(&|p| unsafe { self.comp.getState(p) });
        let b = match &self.ctl {
            Some(c) if self.ctl_aparte => toma(&|p| unsafe { c.getState(p) }),
            _ => vec![],
        };
        let mut v = (a.len() as u32).to_le_bytes().to_vec();
        v.extend(a);
        v.extend(b);
        v
    }

    pub fn poner_estado(&mut self, b: &[u8]) {
        let Some(n) = b.get(..4).map(|x| u32::from_le_bytes(x.try_into().unwrap()) as usize) else { return };
        let (Some(a), Some(c)) = (b.get(4..4 + n), b.get(4 + n..)) else { return };
        let flujo = |x: &[u8]| ComWrapper::new(Flujo(RefCell::new((x.to_vec(), 0)))).to_com_ptr::<IBStream>().unwrap();
        unsafe {
            if !a.is_empty() {
                self.comp.setState(flujo(a).as_ptr());
                if let Some(k) = &self.ctl {
                    k.setComponentState(flujo(a).as_ptr());
                }
            }
            if let Some(k) = self.ctl.as_ref().filter(|_| !c.is_empty()) {
                k.setState(flujo(c).as_ptr());
            }
        }
    }

    pub fn latencia(&self) -> u32 {
        unsafe { self.proc_.getLatencySamples() }
    }

    pub fn procesar(&mut self, ev: &[Evento], t: &Tiempo, entra: &[Buf; 2], sale: &mut [Buf; 2], n: usize) {
        unsafe {
            if !self.procesando {
                self.procesando = true;
                self.proc_.setProcessing(1);
            }
            self.entra.llenar(entra, n);
            self.eventos.n.set(0);
            self.fuera.n.set(0);
            self.cambios.n.set(0);
            self.salen.n.set(0);
            for e in ev {
                let mut x: EventV = std::mem::zeroed();
                x.ppqPosition = t.pos;
                match *e {
                    Evento::Nota { tono, fuerza } => {
                        x.r#type = Event_::EventTypes_::kNoteOnEvent as u16;
                        x.__field0.noteOn = NoteOnEvent { channel: 0, pitch: tono as i16, tuning: 0.0, velocity: fuerza, length: 0, noteId: -1 };
                        self.eventos.poner(x);
                    }
                    Evento::Suelta { tono } => {
                        x.r#type = Event_::EventTypes_::kNoteOffEvent as u16;
                        x.__field0.noteOff = NoteOffEvent { channel: 0, pitch: tono as i16, velocity: 0.0, noteId: -1, tuning: 0.0 };
                        self.eventos.poner(x);
                    }
                    Evento::Control { id, valor } => {
                        self.cambios.poner(id, valor);
                    }
                    // En VST3 la rueda y los mandos llegan como parámetros: aún no.
                    Evento::Rueda(_) | Evento::Mando { .. } => {}
                }
            }
            let mut c: ProcessContext = std::mem::zeroed();
            // tocando, posición, compás, tempo y métrica válidos
            c.state = (t.tocando as u32) << 1 | 1 << 9 | 1 << 11 | 1 << 10 | 1 << 13;
            c.sampleRate = t.sr;
            c.projectTimeSamples = (t.pos * 60.0 / t.bpm * t.sr) as i64;
            c.continousTimeSamples = self.cuadros;
            c.projectTimeMusic = t.pos;
            c.barPositionMusic = (t.pos / 4.0).floor() * 4.0;
            c.tempo = t.bpm;
            (c.timeSigNumerator, c.timeSigDenominator) = (4, 4);
            const P: usize = 16;
            let lleno = |p: &mut Puertos| -> ([AudioBusBuffers; P], usize) {
                let mut b: [AudioBusBuffers; P] = std::mem::zeroed();
                let n = p.punteros.len().min(P);
                for (b, p) in b.iter_mut().zip(&mut p.punteros) {
                    b.numChannels = p.len() as i32;
                    b.__field0.channelBuffers32 = p.as_mut_ptr();
                }
                (b, n)
            };
            let (mut a, na) = lleno(&mut self.entra);
            let (mut b, nb) = lleno(&mut self.sale);
            let mut d = ProcessData {
                processMode: ProcessModes_::kRealtime as i32,
                symbolicSampleSize: SymbolicSampleSizes_::kSample32 as i32,
                numSamples: n as i32,
                numInputs: na as i32,
                numOutputs: nb as i32,
                inputs: a.as_mut_ptr(),
                outputs: b.as_mut_ptr(),
                inputParameterChanges: self.cambios_p.as_ptr(),
                outputParameterChanges: self.salen_p.as_ptr(),
                inputEvents: self.eventos_p.as_ptr(),
                outputEvents: self.fuera_p.as_ptr(),
                processContext: &mut c,
            };
            let r = self.proc_.process(&mut d);
            self.cuadros += n as i64;
            if r == kResultOk {
                self.sale.vaciar(sale, n);
            } else {
                sale.iter_mut().for_each(|c| c[..n].fill(0.0));
            }
        }
    }

    pub fn pegar(&mut self, donde: *mut c_void, ventana: *mut c_void) -> Option<(u32, u32)> {
        if self.vista.is_some() {
            return None;
        }
        let tipo = if cfg!(windows) { kPlatformTypeHWND } else { kPlatformTypeNSView };
        unsafe {
            let v = ComPtr::from_raw(self.ctl.as_ref()?.createView(c"editor".as_ptr()))?;
            if v.isPlatformTypeSupported(tipo) != kResultOk {
                return None;
            }
            self.marco.ventana.set(ventana);
            v.setFrame(self.marco_p.as_ptr());
            let mut r = ViewRect { left: 0, top: 0, right: 600, bottom: 400 };
            v.getSize(&mut r);
            if v.attached(donde, tipo) != kResultOk {
                v.setFrame(null_mut());
                return None;
            }
            self.vista = Some(v);
            Some(((r.right - r.left).max(1) as u32, (r.bottom - r.top).max(1) as u32))
        }
    }

    pub fn despegar(&mut self) {
        if let Some(v) = self.vista.take() {
            unsafe {
                v.removed();
                v.setFrame(null_mut());
            }
            self.marco.ventana.set(null_mut());
        }
    }
}

impl Drop for Vst3 {
    fn drop(&mut self) {
        self.despegar();
        unsafe {
            if self.procesando {
                self.proc_.setProcessing(0);
            }
            self.comp.setActive(0);
            if let Some(c) = &self.ctl {
                c.setComponentHandler(null_mut());
                if self.ctl_aparte {
                    if let (Some(a), Some(b)) = (self.comp.cast::<IConnectionPoint>(), c.cast::<IConnectionPoint>()) {
                        a.disconnect(b.as_ptr());
                        b.disconnect(a.as_ptr());
                    }
                    c.terminate();
                }
            }
            self.comp.terminate();
        }
    }
}
