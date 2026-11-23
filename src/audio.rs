//! Salida de audio, reloj de reproducción y grabación (Core Audio, por medio de cpal).
use crate::medios::Wav;
use crate::modelo::SR;
use crate::motor::{BLOQUE, Motor, Vivo};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Qué posición se escucha y cuándo: `pos` (en segundos) suena en `instante`.
#[derive(Clone, Copy)]
pub struct Reloj {
    pub pos: f64,
    pub instante: Instant,
    pub tocando: bool,
    pub desde: f64,
}

impl Reloj {
    pub fn nuevo() -> Self {
        Reloj { pos: 0.0, instante: Instant::now(), tocando: false, desde: 0.0 }
    }
    /// Segundo que se oye en `t`, sin topes.
    pub fn en(&self, t: Instant) -> f64 {
        if t >= self.instante {
            self.pos + (t - self.instante).as_secs_f64()
        } else {
            self.pos - (self.instante - t).as_secs_f64()
        }
    }
    /// Segundo que se oye ahora.
    pub fn ahora(&self) -> f64 {
        if self.tocando { self.en(Instant::now()).max(self.desde) } else { self.pos }
    }
}

/// Pasa la mezcla de 48 kHz a la frecuencia del dispositivo sin cambiarla (otras apps lo están usando).
struct Adaptador {
    bloque: [f32; 2 * BLOQUE],
    i: usize,
    hist: [[f32; 2]; 4],
    fase: f64,
    paso: f64,
}

impl Adaptador {
    fn cuadro(&mut self, m: &mut Motor) -> [f32; 2] {
        if self.i >= BLOQUE {
            m.bloque(&mut self.bloque);
            self.i = 0;
        }
        let f = [self.bloque[2 * self.i], self.bloque[2 * self.i + 1]];
        self.i += 1;
        f
    }
    fn siguiente(&mut self, m: &mut Motor) -> [f32; 2] {
        if self.paso == 1.0 {
            return self.cuadro(m);
        }
        self.fase += self.paso;
        while self.fase >= 1.0 {
            self.fase -= 1.0;
            let f = self.cuadro(m);
            self.hist.rotate_left(1);
            self.hist[3] = f;
        }
        let t = self.fase as f32;
        let [a, b, c, d] = self.hist;
        std::array::from_fn(|k| {
            let (a, b, c, d) = (a[k], b[k], c[k], d[k]);
            b + 0.5 * t * (c - a + t * (2.0 * a - 5.0 * b + 4.0 * c - d + t * (3.0 * (b - c) + d - a)))
        })
    }
}

pub struct Salida {
    _s: cpal::Stream,
    pub rota: Arc<AtomicBool>,
    pub nombre: String,
}

pub fn abrir_salida(motor: Arc<Mutex<Motor>>, reloj: Arc<Mutex<Reloj>>, muda: bool) -> Result<Salida, String> {
    let d = cpal::default_host().default_output_device().ok_or("No hay salida de audio")?;
    let nombre = d.description().map(|x| x.name().to_string()).unwrap_or_default();
    let conf = d.default_output_config().map_err(|e| e.to_string())?;
    let canales = conf.channels() as usize;
    let mut ad = Adaptador {
        bloque: [0.0; 2 * BLOQUE],
        i: BLOQUE,
        hist: [[0.0; 2]; 4],
        fase: 0.0,
        paso: SR as f64 / conf.sample_rate() as f64,
    };
    let rota = Arc::new(AtomicBool::new(false));
    let r2 = rota.clone();
    let mut tocaba = false;
    let s = d
        .build_output_stream(
            bloque_corto(&conf, BLOQUE as u32),
            move |out: &mut [f32], info: &cpal::OutputCallbackInfo| {
                let ts = info.timestamp();
                let retardo = ts.playback.saturating_duration_since(ts.callback);
                let mut m = motor.lock().unwrap();
                if m.tocando && !tocaba {
                    ad.i = BLOQUE;
                    ad.hist = [[0.0; 2]; 4];
                }
                // Lo que ya se mezcló pero sigue en el bloque todavía no suena.
                let pendiente = (BLOQUE - ad.i) as i64;
                let pos = (m.pos - pendiente) as f64 / SR as f64;
                {
                    let mut r = reloj.lock().unwrap();
                    if m.tocando {
                        if !tocaba {
                            r.desde = pos;
                        }
                        (r.pos, r.instante) = (pos, Instant::now() + retardo);
                    } else if tocaba {
                        r.pos = m.pos as f64 / SR as f64;
                    }
                    r.tocando = m.tocando;
                }
                tocaba = m.tocando;
                for f in out.chunks_exact_mut(canales) {
                    let x = ad.siguiente(&mut m).map(|x| if muda { 0.0 } else { x.clamp(-1.0, 1.0) });
                    f.fill(0.0);
                    if canales == 1 {
                        f[0] = 0.5 * (x[0] + x[1]);
                    } else {
                        f[..2].copy_from_slice(&x);
                    }
                }
            },
            move |_| r2.store(true, Relaxed),
            None,
        )
        .map_err(|e| crate::idioma::tf("No pude abrir la salida de audio: {}", &[&e]))?;
    s.play().map_err(|e| e.to_string())?;
    Ok(Salida { _s: s, rota, nombre })
}

/// Reproduce sin dispositivo, para pruebas: el reloj corre a tiempo real y no sale sonido.
pub fn salida_simulada(motor: Arc<Mutex<Motor>>, reloj: Arc<Mutex<Reloj>>, parar: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut b = [0f32; 2 * BLOQUE];
        let mut sig = Instant::now();
        let mut tocaba = false;
        while !parar.load(Relaxed) {
            let mut m = motor.lock().unwrap();
            let mut r = reloj.lock().unwrap();
            let pos = m.pos as f64 / SR as f64;
            if m.tocando {
                if !tocaba {
                    r.desde = pos;
                }
                (r.pos, r.instante) = (pos, sig);
            } else if tocaba {
                r.pos = pos;
            }
            r.tocando = m.tocando;
            tocaba = m.tocando;
            m.bloque(&mut b);
            drop((m, r));
            sig += Duration::from_secs_f64(BLOQUE as f64 / SR as f64);
            std::thread::sleep(sig.saturating_duration_since(Instant::now()));
        }
    });
}

/// Sin esto macOS entrega bloques de 512 cuadros o más y uno se oye a sí mismo con retraso.
fn bloque_corto(c: &cpal::SupportedStreamConfig, n: u32) -> cpal::StreamConfig {
    let mut s = c.config();
    if let cpal::SupportedBufferSize::Range { min, max } = c.buffer_size() {
        if (*min..=*max).contains(&n) {
            s.buffer_size = cpal::BufferSize::Fixed(n);
        }
    }
    s
}

fn microfono(nombre: Option<&str>) -> Result<cpal::Device, String> {
    let h = cpal::default_host();
    match nombre {
        Some(n) => h
            .input_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.description().is_ok_and(|x| x.name() == n))
            .ok_or(crate::idioma::tf("No encuentro el micrófono «{}»", &[&n])),
        None => h.default_input_device().ok_or(crate::idioma::t("No hay micrófono").into()),
    }
}

/// «Escucharme»: pasa el micrófono al motor.
pub struct Monitor {
    _s: cpal::Stream,
    pub vivo: Vivo,
}

/// Pone en la cola `x` (a `sr` Hz) convertido a 48 kHz y descarta lo viejo si se acumula retraso.
pub struct Puente {
    fase: f64,
    paso: f64,
    antes: [f32; 2],
    vivo: Vivo,
}

impl Puente {
    /// Más de esto (16 ms) en la cola es retraso que no hace falta.
    const TOPE: usize = 768;
    pub fn nuevo(sr: u32, vivo: Vivo) -> Self {
        Puente { fase: 0.0, paso: sr as f64 / SR as f64, antes: [0.0; 2], vivo }
    }
    pub fn meter(&mut self, x: impl Iterator<Item = [f32; 2]>) {
        let mut q = self.vivo.lock().unwrap();
        for f in x {
            while self.fase < 1.0 {
                let t = self.fase as f32;
                q.push_back(std::array::from_fn(|k| self.antes[k] + (f[k] - self.antes[k]) * t));
                self.fase += self.paso;
            }
            self.fase -= 1.0;
            self.antes = f;
        }
        if q.len() > Self::TOPE {
            let sobra = q.len() - Self::TOPE / 2;
            q.drain(..sobra);
        }
    }
}

impl Monitor {
    pub fn iniciar(nombre: Option<&str>) -> Result<Monitor, String> {
        let d = microfono(nombre)?;
        let conf = d.default_input_config().map_err(|e| crate::idioma::tf("No pude abrir el micrófono: {}", &[&e]))?;
        let canales = conf.channels() as usize;
        let vivo = Vivo::default();
        let mut p = Puente::nuevo(conf.sample_rate(), vivo.clone());
        let s = d
            .build_input_stream(
                bloque_corto(&conf, 128),
                move |x: &[f32], _: &cpal::InputCallbackInfo| {
                    p.meter(x.chunks_exact(canales).map(|f| if canales == 1 { [f[0]; 2] } else { [f[0], f[1]] }))
                },
                |_| {},
                None,
            )
            .map_err(|e| crate::idioma::tf("No pude abrir el micrófono: {}", &[&e]))?;
        s.play().map_err(|e| e.to_string())?;
        Ok(Monitor { _s: s, vivo })
    }
}

pub fn entradas() -> Vec<String> {
    let Ok(l) = cpal::default_host().input_devices() else { return vec![] };
    l.filter_map(|d| Some(d.description().ok()?.name().to_string())).collect()
}

pub struct Grabacion {
    s: Option<cpal::Stream>,
    hilo: Option<std::thread::JoinHandle<std::io::Result<()>>>,
    primera: Arc<Mutex<Option<Instant>>>,
    nivel: Arc<AtomicU32>,
    pub ruta: PathBuf,
}

impl Grabacion {
    /// Abre el micrófono `nombre` (o el predeterminado) y empieza a escribir a `ruta`.
    pub fn iniciar(nombre: Option<&str>, ruta: &Path) -> Result<Grabacion, String> {
        let d = microfono(nombre)?;
        let conf = d.default_input_config().map_err(|e| crate::idioma::tf("No pude abrir el micrófono: {}", &[&e]))?;
        let canales = conf.channels() as usize;
        let usar = canales.min(2);
        std::fs::create_dir_all(ruta.parent().unwrap()).map_err(|e| e.to_string())?;
        let mut wav = Wav::crear(ruta, usar as u16, conf.sample_rate()).map_err(|e| e.to_string())?;
        let (tx, rx) = crossbeam_channel::unbounded::<Vec<f32>>();
        let hilo = std::thread::spawn(move || {
            for v in rx {
                wav.escribir(&v)?;
            }
            wav.cerrar()
        });
        let primera = Arc::new(Mutex::new(None));
        let nivel = Arc::new(AtomicU32::new(0));
        let (p2, n2) = (primera.clone(), nivel.clone());
        let s = d
            .build_input_stream(
                conf.config(),
                move |x: &[f32], info: &cpal::InputCallbackInfo| {
                    let ts = info.timestamp();
                    let atras = ts.callback.saturating_duration_since(ts.capture);
                    let mut p = p2.lock().unwrap();
                    if p.is_none() {
                        *p = Instant::now().checked_sub(atras);
                    }
                    drop(p);
                    let v: Vec<f32> =
                        x.chunks_exact(canales).flat_map(|f| f[..usar].iter().copied()).collect();
                    let pico = v.iter().fold(0f32, |m, x| m.max(x.abs()));
                    n2.fetch_max((pico * 1e6) as u32, Relaxed);
                    let _ = tx.send(v);
                },
                |_| {},
                None,
            )
            .map_err(|e| crate::idioma::tf("No pude abrir el micrófono: {}", &[&e]))?;
        s.play().map_err(|e| e.to_string())?;
        Ok(Grabacion { s: Some(s), hilo: Some(hilo), primera, nivel, ruta: ruta.into() })
    }

    /// Momento en que el micrófono captó la primera muestra.
    pub fn primera(&self) -> Option<Instant> {
        *self.primera.lock().unwrap()
    }

    pub fn nivel(&self) -> f32 {
        self.nivel.swap(0, Relaxed) as f32 / 1e6
    }

    /// Cierra el archivo.
    pub fn terminar(&mut self) -> Result<(), String> {
        self.s = None;
        match self.hilo.take().map(|h| h.join()) {
            Some(Ok(Ok(()))) | None => Ok(()),
            Some(Ok(Err(e))) => Err(crate::idioma::tf("No pude guardar la grabación: {}", &[&e])),
            Some(Err(_)) => Err(crate::idioma::t("No pude guardar la grabación").into()),
        }
    }
}
