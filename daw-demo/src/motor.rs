// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! El motor: toca una `Escena` ya lista. Aquí no se pide ni se suelta memoria ni se espera a nadie.
use crate::biblioteca::Biblioteca;
use crate::modelo::*;
use crate::muestrario::Muestra;
use crate::plugin::{EVENTOS, Evento as EvPlugin, Plugin, Tiempo};
use crate::receta::{self, BLOQUE, Buf, DatosVoz, Receta};
use crossbeam_channel::{Receiver, Sender, bounded};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Modo {
    Patron(u32),
    Cancion,
}

#[derive(Clone, Copy)]
struct Evento {
    tick: f64,
    canal: usize,
    tono: u8,
    fuerza: f32,
    dur: f64,
}

#[derive(Clone, Copy, Default)]
struct Voz {
    viva: bool,
    d: DatosVoz,
    tono: u8,
    /// Nota tocada a mano: dura hasta soltarla.
    a_mano: bool,
    restan: f64,
    edad: u64,
    callada: u32,
    suelta: u64,
}

struct CanalE {
    id: u32,
    receta: Arc<Receta>,
    plugin: Option<Arc<Plugin>>,
    /// Lo que se le manda al plugin al empezar el siguiente bloque.
    avisos: Vec<EvPlugin>,
    callada: u32,
    muestra: Option<Arc<Muestra>>,
    /// Una pieza de batería del banco toca siempre su tecla.
    fija: Option<u8>,
    /// Valor que suena y valor al que va: así no truena al mover una perilla.
    ctl: Vec<f32>,
    meta: Vec<f32>,
    gan: [f32; 2],
    gan_meta: [f32; 2],
    pista: usize,
    voces: Vec<Voz>,
    mem: Vec<f32>,
}

struct EfectoE {
    receta: Option<Arc<Receta>>,
    plugin: Option<Arc<Plugin>>,
    ctl: Vec<f32>,
    meta: Vec<f32>,
    mem: Vec<f32>,
    /// Cuánto efecto: lo que suena y a dónde va.
    mezcla: f32,
    mezcla_meta: f32,
}

struct PistaE {
    /// Ecualizador de la pista; nada si está plano.
    eq: Option<[[f32; 5]; 3]>,
    eq_mem: [[[f32; 2]; 3]; 2],
    ancho: f32,
    fase: f32,
    gan: [f32; 2],
    gan_meta: [f32; 2],
    envios: [f32; ENVIOS],
    efectos: Vec<EfectoE>,
    mezcla: [Buf; 2],
    /// Cuadros seguidos en silencio: si son muchos, no se procesa.
    callada: u32,
}

/// Un audio de la playlist. Su lugar sale de la posición, así que saltar no requiere nada.
struct AudioE {
    ini: f64,
    fin: f64,
    inicio: f32,
    velocidad: f32,
    /// Segundos de cada fundido.
    entra: f32,
    sale: f32,
    gan: [f32; 2],
    pista: usize,
    m: Arc<Muestra>,
}

pub struct Escena {
    canales: Vec<CanalE>,
    audios: Vec<AudioE>,
    pistas: Vec<PistaE>,
    eventos: Vec<Evento>,
    pub largo: f64,
    bpm: f32,
    bufs: Vec<Buf>,
    sr: f32,
}

fn balance(v: f32, pan: f32) -> [f32; 2] {
    let pan = pan.clamp(-1.0, 1.0);
    [v * (1.0 - pan.max(0.0)), v * (1.0 + pan.min(0.0))]
}

fn valores(r: &Receta, puestos: &std::collections::BTreeMap<String, f32>) -> Vec<f32> {
    r.controles.iter().map(|c| puestos.get(&c.clave).copied().unwrap_or(c.valor).clamp(c.min, c.max)).collect()
}

impl Escena {
    /// Se arma fuera del hilo de audio, con toda la memoria que hará falta.
    pub fn nueva(p: &Proyecto, bib: &mut Biblioteca, modo: Modo) -> Escena {
        let sr = bib.sr;
        let hay_solo = p.canales.iter().any(|c| c.solo);
        let mut canales = vec![];
        let mut lugar = std::collections::HashMap::new();
        for c in &p.canales {
            // Los de fábrica son grabaciones del banco y suenan con el muestreador. Un instrumento que ya no está se
            // oye con el piano en vez de callarse.
            let banco = match (&c.muestra, &c.plugin) {
                (Some(_), _) | (_, Some(_)) => None,
                _ => bib.del_banco(&c.instrumento).or_else(|| bib.ver(&c.instrumento).is_none().then(|| crate::banco::de("piano")).flatten()),
            };
            let clave = if banco.is_some() { "muestra" } else { c.instrumento.as_str() };
            let Some(receta) = bib.ver(clave).and_then(|g| g.receta.clone()) else { continue };
            let plugin = c.plugin.as_ref().and_then(|x| bib.rack.dar(x, sr).ok());
            let voces = if plugin.is_some() { EVENTOS / 2 } else { receta.voces };
            let muestra = match banco {
                Some(x) => bib.banco(x),
                None => c.muestra.as_ref().and_then(|r| bib.muestra(r, c.preset)),
            };
            let suena = !c.mudo && (!hay_solo || c.solo);
            // Si nadie movió «Soltar», dura lo que pide el SoundFont.
            let mut puestos = c.controles.clone();
            if let Some(s) = muestra.as_ref().and_then(|m| m.soltar) {
                puestos.entry("baja".into()).or_insert(s);
            }
            let meta = valores(&receta, &puestos);
            let gan = balance(if suena { c.volumen } else { 0.0 }, c.pan);
            lugar.insert(c.id, canales.len());
            canales.push(CanalE {
                id: c.id,
                muestra,
                fija: banco.and_then(|x| x.nota),
                ctl: meta.clone(),
                meta,
                gan,
                gan_meta: gan,
                pista: c.pista.min(p.mezcla.len() - 1),
                voces: vec![Voz::default(); voces],
                mem: vec![0.0; if plugin.is_some() { 0 } else { receta.memoria * voces }],
                avisos: Vec::with_capacity(EVENTOS),
                callada: 0,
                plugin,
                receta,
            });
        }
        let mut pistas: Vec<PistaE> = vec![];
        let solo = p.mezcla[..PRIMER_ENVIO].iter().any(|m| m.solo);
        for (i, m) in p.mezcla.iter().enumerate() {
            let mut efectos = vec![];
            for e in m.efectos.iter().filter(|e| e.activo) {
                if let Some(x) = &e.plugin {
                    if let Ok(plugin) = bib.rack.dar(x, sr) {
                        efectos.push(EfectoE {
                            receta: None,
                            plugin: Some(plugin),
                            ctl: vec![],
                            meta: vec![],
                            mem: vec![],
                            mezcla: e.mezcla,
                            mezcla_meta: e.mezcla,
                        });
                    }
                    continue;
                }
                let Some(receta) = bib.ver(&e.efecto).and_then(|g| g.receta.clone()) else { continue };
                let meta = valores(&receta, &e.controles);
                efectos.push(EfectoE {
                    ctl: meta.clone(),
                    meta,
                    mem: vec![0.0; receta.memoria],
                    receta: Some(receta),
                    plugin: None,
                    mezcla: e.mezcla,
                    mezcla_meta: e.mezcla,
                });
            }
            let calla = m.mudo || solo && !m.solo && (1..PRIMER_ENVIO).contains(&i);
            let gan = balance(if calla { 0.0 } else { m.volumen }, m.pan);
            let eq = m.eq.iter().any(|x| *x != 0.0).then(|| eq_de_pista(&m.eq, sr));
            pistas.push(PistaE {
                eq,
                eq_mem: Default::default(),
                ancho: m.ancho,
                fase: if m.fase { -1.0 } else { 1.0 },
                gan,
                gan_meta: gan,
                envios: m.envios,
                efectos,
                mezcla: [[0.0; BLOQUE]; 2],
                callada: u32::MAX,
            });
        }
        let mut eventos = vec![];
        let mut audios = vec![];
        let mut poner = |pat: &Patron, inicio: u32, desfase: u32, dur: u32| {
            for (id, t, n, d) in pat.notas_en(desfase, dur) {
                let Some(&canal) = lugar.get(&id) else { continue };
                let swing = if n.tick % (PASO * 2) == PASO { p.swing * PASO as f32 * 0.5 } else { 0.0 };
                eventos.push(Evento { tick: (inicio + t) as f64 + swing as f64, canal, tono: n.tono, fuerza: n.fuerza, dur: d as f64 });
            }
        };
        let largo = match modo {
            Modo::Patron(id) => p.patron(id).map_or(COMPAS, |x| {
                poner(x, 0, 0, x.largo());
                x.largo()
            }),
            Modo::Cancion => {
                for c in p.clips.iter().filter(|c| p.pista_suena(c.pista)) {
                    if let Some(a) = &c.audio {
                        if let Some(m) = bib.muestra(&a.ruta, 0) {
                            audios.push(AudioE {
                                ini: c.tick as f64,
                                fin: (c.tick + c.dur) as f64,
                                inicio: a.inicio,
                                velocidad: a.velocidad,
                                entra: a.entra.max(0.003),
                                sale: a.sale.max(0.003),
                                gan: balance(a.volumen, a.pan),
                                pista: a.mezcla.min(p.mezcla.len() - 1),
                                m,
                            });
                        }
                    } else if let Some(x) = p.patron(c.patron) {
                        poner(x, c.tick, c.desfase, c.dur);
                    }
                }
                p.largo()
            }
        };
        eventos.sort_by(|a, b| a.tick.total_cmp(&b.tick));
        let nodos = canales
            .iter()
            .map(|c| c.receta.cuantos_nodos())
            .chain(pistas.iter().flat_map(|p| p.efectos.iter().filter_map(|e| Some(e.receta.as_ref()?.cuantos_nodos()))))
            .max()
            .unwrap_or(0);
        Escena { canales, audios, pistas, eventos, largo: largo as f64, bpm: p.bpm, bufs: vec![[0.0; BLOQUE]; nodos], sr }
    }

    /// Se queda con lo que venía sonando en la escena anterior.
    fn heredar(&mut self, v: &mut Escena) {
        for c in &mut self.canales {
            let mismo = |a: &Option<Arc<Plugin>>, b: &Option<Arc<Plugin>>| match (a, b) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            };
            if let Some(o) = v.canales.iter_mut().find(|o| o.id == c.id && Arc::ptr_eq(&o.receta, &c.receta) && mismo(&o.plugin, &c.plugin)) {
                std::mem::swap(&mut c.avisos, &mut o.avisos);
                c.callada = o.callada;
                std::mem::swap(&mut c.voces, &mut o.voces);
                std::mem::swap(&mut c.mem, &mut o.mem);
                std::mem::swap(&mut c.ctl, &mut o.ctl);
                c.gan = o.gan;
            }
        }
        for (p, o) in self.pistas.iter_mut().zip(&mut v.pistas) {
            p.gan = o.gan;
            p.eq_mem = o.eq_mem;
            p.callada = o.callada;
            for (i, e) in p.efectos.iter_mut().enumerate() {
                if let Some(x) = o.efectos.get_mut(i).filter(|x| matches!((&x.receta, &e.receta), (Some(a), Some(b)) if Arc::ptr_eq(a, b))) {
                    std::mem::swap(&mut e.mem, &mut x.mem);
                    std::mem::swap(&mut e.ctl, &mut x.ctl);
                    e.mezcla = x.mezcla;
                }
            }
        }
    }
}

pub enum Orden {
    Escena(Box<Escena>),
    Tocar(bool),
    Ir(f64),
    Nota { canal: u32, tono: u8, fuerza: f32 },
    Soltar { canal: u32, tono: u8 },
    Silencio,
    Metronomo(bool),
}

/// Muestras que guarda el analizador.
pub const ONDA: usize = 2048;

/// Las tres bandas de cada pista: graves, medios y agudos.
pub fn eq_de_pista(db: &[f32; 3], sr: f32) -> [[f32; 5]; 3] {
    use crate::receta::{Banda::*, banda};
    [banda(Graves, 120.0, db[0], 0.707, sr), banda(Campana, 1200.0, db[1], 0.6, sr), banda(Agudos, 6000.0, db[2], 0.707, sr)]
}

/// Lo que la ventana lee del motor.
pub struct Estado {
    pub tocando: AtomicBool,
    tick: AtomicU32,
    /// Cuándo se anotó, en microsegundos desde `base`.
    hora: std::sync::atomic::AtomicU64,
    base: std::time::Instant,
    /// Cuántas veces se ha saltado de lugar.
    pub saltos: AtomicU32,
    picos: [AtomicU32; PISTAS_MEZCLA * 2],
    pub voces: AtomicU32,
    /// La pista que se está mirando en el analizador.
    pub escucha: AtomicU32,
    onda: Vec<AtomicU32>,
    onda_pos: AtomicU32,
    /// Milésimas del tiempo disponible que usa el audio.
    pub carga: AtomicU32,
}

impl Default for Estado {
    fn default() -> Estado {
        Estado {
            tocando: Default::default(),
            tick: Default::default(),
            hora: Default::default(),
            base: std::time::Instant::now(),
            saltos: Default::default(),
            picos: std::array::from_fn(|_| AtomicU32::new(0)),
            voces: Default::default(),
            escucha: AtomicU32::new(u32::MAX),
            onda: (0..ONDA).map(|_| AtomicU32::new(0)).collect(),
            onda_pos: Default::default(),
            carga: Default::default(),
        }
    }
}

impl Estado {
    pub fn tick(&self) -> f32 {
        f32::from_bits(self.tick.load(Relaxed))
    }

    fn oir(&self, m: &[Buf; 2], n: usize) {
        let p = self.onda_pos.load(Relaxed) as usize;
        for j in 0..n {
            self.onda[(p + j) % ONDA].store(((m[0][j] + m[1][j]) * 0.5).to_bits(), Relaxed);
        }
        self.onda_pos.store(((p + n) % ONDA) as u32, Relaxed);
    }

    /// Lo último que sonó en la pista escuchada, de lo más viejo a lo más nuevo.
    pub fn onda(&self) -> Vec<f32> {
        let p = self.onda_pos.load(Relaxed) as usize;
        (0..ONDA).map(|j| f32::from_bits(self.onda[(p + j) % ONDA].load(Relaxed))).collect()
    }

    /// Hace cuántos segundos se anotó la posición.
    pub fn hace(&self) -> f64 {
        (self.base.elapsed().as_micros() as u64).saturating_sub(self.hora.load(Relaxed)) as f64 / 1e6
    }

    /// Lee el pico y lo deja en cero.
    pub fn pico(&self, pista: usize) -> [f32; 2] {
        [0, 1].map(|l| f32::from_bits(self.picos[pista * 2 + l].swap(0, Relaxed)))
    }
}

/// El lado de la ventana.
pub struct Mando {
    ordenes: Sender<Orden>,
    viejas: Receiver<Box<Escena>>,
    pub estado: Arc<Estado>,
}

impl Mando {
    /// Para los teclados MIDI, que mandan sus notas desde su propio hilo.
    pub fn ordenes(&self) -> Sender<Orden> {
        self.ordenes.clone()
    }

    pub fn mandar(&self, o: Orden) -> bool {
        // Las escenas viejas se sueltan aquí, no en el hilo de audio.
        while self.viejas.try_recv().is_ok() {}
        self.ordenes.try_send(o).is_ok()
    }
}

pub struct Motor {
    escena: Box<Escena>,
    ordenes: Receiver<Orden>,
    viejas: Sender<Box<Escena>>,
    pub estado: Arc<Estado>,
    tocando: bool,
    pos: f64,
    cursor: usize,
    pub bucle: bool,
    /// Llegó al final sin bucle.
    pub acabo: bool,
    metronomo: bool,
    azar: u32,
    reloj: u64,
    limite: f32,
}

impl Motor {
    pub fn nuevo(e: Escena) -> (Motor, Mando) {
        let (ordenes, recibe) = bounded(256);
        let (viejas, sueltas) = bounded(64);
        let estado = Arc::new(Estado::default());
        let m = Motor {
            escena: Box::new(e),
            ordenes: recibe,
            viejas,
            estado: estado.clone(),
            tocando: false,
            pos: 0.0,
            cursor: 0,
            bucle: true,
            acabo: false,
            metronomo: false,
            azar: 1,
            reloj: 0,
            limite: 1.0,
        };
        (m, Mando { ordenes, viejas: sueltas, estado })
    }

    fn buscar_cursor(&mut self) {
        let pos = self.pos;
        self.cursor = self.escena.eventos.partition_point(|e| e.tick < pos);
    }

    fn atender(&mut self) {
        while !self.viejas.is_full() {
            let Ok(o) = self.ordenes.try_recv() else { break };
            match o {
                Orden::Escena(mut e) => {
                    e.heredar(&mut self.escena);
                    let vieja = std::mem::replace(&mut self.escena, e);
                    let _ = self.viejas.try_send(vieja);
                    if self.pos >= self.escena.largo {
                        self.pos = 0.0;
                    }
                    self.buscar_cursor();
                }
                Orden::Tocar(t) => {
                    if !t {
                        self.soltar_todo();
                    }
                    self.tocando = t;
                    self.acabo = false;
                }
                Orden::Ir(t) => {
                    self.soltar_todo();
                    self.pos = t.clamp(0.0, (self.escena.largo - 1.0).max(0.0));
                    self.estado.saltos.fetch_add(1, Relaxed);
                    self.acabo = false;
                    self.buscar_cursor();
                }
                Orden::Nota { canal, tono, fuerza } => {
                    if let Some(i) = self.escena.canales.iter().position(|c| c.id == canal) {
                        self.sonar(i, tono, fuerza, None);
                    }
                }
                Orden::Soltar { canal, tono } => {
                    for c in self.escena.canales.iter_mut().filter(|c| c.id == canal) {
                        let tono = c.fija.unwrap_or(tono);
                        c.voces.iter_mut().filter(|v| v.a_mano && v.tono == tono).for_each(|v| v.d.puerta = false);
                    }
                }
                Orden::Metronomo(si) => self.metronomo = si,
                Orden::Silencio => {
                    for c in &mut self.escena.canales {
                        c.voces.iter_mut().for_each(|v| if c.plugin.is_some() { v.d.puerta = false } else { v.viva = false });
                    }
                    for e in self.escena.pistas.iter_mut().flat_map(|p| &mut p.efectos) {
                        e.mem.fill(0.0);
                    }
                }
            }
        }
    }

    fn soltar_todo(&mut self) {
        for c in &mut self.escena.canales {
            c.voces.iter_mut().for_each(|v| v.d.puerta = false);
        }
    }

    fn sonar(&mut self, i: usize, tono: u8, fuerza: f32, dur: Option<f64>) {
        let c = &mut self.escena.canales[i];
        let tono = c.fija.unwrap_or(tono);
        self.reloj += 1;
        // Una voz libre; si no hay, la que ya se soltó hace más; si no, la más vieja.
        let Some(k) = (0..c.voces.len()).min_by_key(|k| {
            let v = &c.voces[*k];
            (v.viva, v.viva && v.d.puerta, if v.viva { v.edad } else { 0 })
        }) else {
            return;
        };
        // xorshift
        self.azar ^= self.azar << 13;
        self.azar ^= self.azar >> 17;
        self.azar ^= self.azar << 5;
        if c.plugin.is_some() {
            // Cada voz aquí solo recuerda qué tecla hay que soltar.
            if c.avisos.len() + 2 > EVENTOS {
                return;
            }
            if c.voces[k].viva && c.voces[k].d.puerta {
                c.avisos.push(EvPlugin::Suelta { tono: c.voces[k].tono });
            }
            c.avisos.push(EvPlugin::Nota { tono, fuerza });
        } else {
            let m = c.receta.memoria;
            c.mem[k * m..(k + 1) * m].fill(0.0);
        }
        let (zonas, cuantas) = c.muestra.as_ref().map_or(Default::default(), |x| x.elegir(tono, fuerza));
        c.voces[k] = Voz {
            viva: true,
            d: DatosVoz {
                hz: 440.0 * 2f32.powf((tono as f32 - 69.0) / 12.0),
                fuerza,
                azar: self.azar as f32 / u32::MAX as f32,
                puerta: true,
                zonas,
                cuantas,
            },
            tono,
            a_mano: dur.is_none(),
            restan: dur.unwrap_or(f64::MAX),
            edad: self.reloj,
            callada: 0,
            suelta: 0,
        };
    }

    /// Llena `sale`. Es lo único que corre en el hilo de audio.
    pub fn tocar(&mut self, sale: &mut [[f32; 2]]) {
        self.atender();
        let mut resto = sale;
        while !resto.is_empty() {
            let mut n = resto.len().min(BLOQUE);
            let paso = self.escena.bpm as f64 * PPQ as f64 / 60.0 / self.escena.sr as f64;
            if self.tocando && !self.acabo {
                if self.pos >= self.escena.largo {
                    if self.bucle {
                        self.pos -= self.escena.largo;
                        self.cursor = 0;
                    } else {
                        self.acabo = true;
                    }
                }
                if !self.acabo {
                    while let Some(e) = self.escena.eventos.get(self.cursor).copied().filter(|e| e.tick <= self.pos) {
                        self.sonar(e.canal, e.tono, e.fuerza, Some(e.dur / paso));
                        self.cursor += 1;
                    }
                    // El bloque termina justo donde entra la siguiente nota o acaba la vuelta.
                    let sig = self.escena.eventos.get(self.cursor).map_or(f64::MAX, |e| e.tick).min(self.escena.largo);
                    n = n.min((((sig - self.pos) / paso).ceil() as usize).max(1));
                }
            }
            let (ahora, luego) = resto.split_at_mut(n);
            let t =
                Tiempo { bpm: self.escena.bpm as f64, pos: self.pos / PPQ as f64, tocando: self.tocando && !self.acabo, sr: self.escena.sr as f64 };
            self.bloque(ahora, &t);
            if self.tocando && !self.acabo {
                self.pos += n as f64 * paso;
            }
            resto = luego;
        }
        self.estado.tocando.store(self.tocando && !self.acabo, Relaxed);
        self.estado.tick.store((self.pos as f32).to_bits(), Relaxed);
        self.estado.hora.store(self.estado.base.elapsed().as_micros() as u64, Relaxed);
    }

    fn bloque(&mut self, sale: &mut [[f32; 2]], t: &Tiempo) {
        let n = sale.len();
        let Escena { canales, audios, pistas, bufs, sr, bpm, .. } = &mut *self.escena;
        // Suaviza en unos 5 ms.
        let k = 1.0 - (-(n as f32) / (0.005 * *sr)).exp();
        for p in pistas.iter_mut() {
            p.mezcla[0][..n].fill(0.0);
            p.mezcla[1][..n].fill(0.0);
        }
        let mut vivas = 0;
        let mudo = [[0.0; BLOQUE]; 2];
        for c in canales.iter_mut() {
            let antes = c.gan;
            for l in 0..2 {
                c.gan[l] += (c.gan_meta[l] - c.gan[l]) * k;
            }
            for (v, m) in c.ctl.iter_mut().zip(&c.meta) {
                *v += (*m - *v) * k;
            }
            if let Some(p) = &c.plugin {
                for v in c.voces.iter_mut().filter(|v| v.viva) {
                    if !v.a_mano {
                        v.restan -= n as f64;
                    }
                    if (v.restan <= 0.0 || !v.d.puerta) && c.avisos.len() < EVENTOS {
                        c.avisos.push(EvPlugin::Suelta { tono: v.tono });
                        v.viva = false;
                    }
                    vivas += 1;
                }
                if !c.avisos.is_empty() || c.voces.iter().any(|v| v.viva) {
                    c.callada = 0;
                }
                // Diez segundos sin sonar: el plugin descansa.
                if c.callada as f32 > 10.0 * *sr {
                    continue;
                }
                let mut b = [[0.0; BLOQUE]; 2];
                p.procesar(&c.avisos, t, &mudo, &mut b, n);
                c.avisos.clear();
                let pico = b.iter().flat_map(|x| &x[..n]).fold(0f32, |m, x| m.max(x.abs()));
                if b.iter().flat_map(|x| &x[..n]).any(|x| !x.is_finite()) {
                    continue;
                }
                c.callada = if pico < 1e-5 { c.callada.saturating_add(n as u32) } else { 0 };
                let destino = &mut pistas[c.pista];
                if pico > 0.0 {
                    destino.callada = 0;
                }
                for l in 0..2 {
                    let paso = (c.gan[l] - antes[l]) / n as f32;
                    for (j, (d, s)) in destino.mezcla[l][..n].iter_mut().zip(&b[l][..n]).enumerate() {
                        *d += s * (antes[l] + paso * j as f32);
                    }
                }
                continue;
            }
            if c.voces.iter().all(|v| !v.viva) {
                continue;
            }
            let r = &*c.receta;
            receta::comunes(r, &c.ctl, bufs, n);
            let destino = &mut pistas[c.pista];
            destino.callada = 0;
            let cola = (r.cola * *sr) as u32;
            for (i, v) in c.voces.iter_mut().enumerate().filter(|(_, v)| v.viva) {
                vivas += 1;
                if !v.a_mano {
                    v.restan -= n as f64;
                    if v.restan <= 0.0 {
                        v.d.puerta = false;
                    }
                }
                let mem = &mut c.mem[i * r.memoria..(i + 1) * r.memoria];
                let [a, b] = receta::voz(r, &v.d, &c.ctl, c.muestra.as_deref(), None, mem, bufs, n);
                let mut pico = 0f32;
                let [di, dd] = &mut destino.mezcla;
                // El volumen va en rampa dentro del bloque.
                let paso = [0, 1].map(|l| (c.gan[l] - antes[l]) / n as f32);
                for (l, (d, s)) in [(di, a), (dd, b)].into_iter().enumerate() {
                    let s = &bufs[s as usize][..n];
                    for (j, (d, s)) in d[..n].iter_mut().zip(s).enumerate() {
                        pico = pico.max(s.abs());
                        *d += s * (antes[l] + paso[l] * j as f32);
                    }
                }
                if !pico.is_finite() {
                    v.viva = false;
                    destino.mezcla.iter_mut().for_each(|d| d.iter_mut().filter(|x| !x.is_finite()).for_each(|x| *x = 0.0));
                    continue;
                }
                v.callada = if pico < 1e-5 { v.callada + n as u32 } else { 0 };
                if !v.d.puerta {
                    v.suelta += n as u64;
                }
                let aguanta = if v.d.puerta { 4096 } else { 256 } + cola;
                // Veinte segundos después de soltarla se apaga aunque el guion no la haya cerrado.
                if v.callada > aguanta || v.suelta > 20 * *sr as u64 {
                    v.viva = false;
                }
            }
        }
        if t.tocando {
            let (pos, paso) = (self.pos, *bpm as f64 * PPQ as f64 / 60.0 / *sr as f64);
            let seg = 60.0 / *bpm as f64 / PPQ as f64;
            for a in audios.iter().filter(|a| a.ini < pos + n as f64 * paso && a.fin > pos) {
                let destino = &mut pistas[a.pista];
                destino.callada = 0;
                for z in &a.m.zonas {
                    let x = &a.m.datos[z.ini..z.fin];
                    for j in 0..n {
                        let k = pos + j as f64 * paso;
                        if k < a.ini || k >= a.fin {
                            continue;
                        }
                        let s = (k - a.ini) * seg;
                        // Los fundidos; al menos 3 ms en cada orilla para que el corte no truene.
                        let f = (a.fin - k) * seg;
                        let orilla = ((s / a.entra as f64).min(1.0) * (f / a.sale as f64).min(1.0)) as f32;
                        let q = (s * a.velocidad as f64 + a.inicio as f64) * z.sr as f64;
                        let i = q as usize;
                        if i + 1 < x.len() {
                            let v = (x[i] + (x[i + 1] - x[i]) * (q - i as f64) as f32) * orilla;
                            destino.mezcla[0][j] += v * z.gan[0] * a.gan[0];
                            destino.mezcla[1][j] += v * z.gan[1] * a.gan[1];
                        }
                    }
                }
            }
        }
        // Primero las pistas, luego los envíos y al final la general.
        for i in (1..pistas.len()).chain([0]) {
            let (general, otras) = pistas.split_at_mut(1);
            let p = if i == 0 { &mut general[0] } else { &mut otras[i - 1] };
            let antes = p.gan;
            for l in 0..2 {
                p.gan[l] += (p.gan_meta[l] - p.gan[l]) * k;
            }
            // Diez segundos callada: ni sus efectos tienen cola.
            if i > 0 && p.callada as f32 > 10.0 * *sr {
                continue;
            }
            p.callada = p.callada.saturating_add(n as u32);
            for e in &mut p.efectos {
                let seco = p.mezcla;
                e.mezcla += (e.mezcla_meta - e.mezcla) * k;
                let mojado = e.mezcla;
                let junta = |m: &mut [Buf; 2]| {
                    if mojado < 0.999 {
                        for l in 0..2 {
                            m[l][..n].iter_mut().zip(&seco[l][..n]).for_each(|(m, s)| *m = s + (*m - s) * mojado);
                        }
                    }
                };
                for (v, m) in e.ctl.iter_mut().zip(&e.meta) {
                    *v += (*m - *v) * k;
                }
                if let Some(x) = &e.plugin {
                    let entra = p.mezcla;
                    x.procesar(&[], t, &entra, &mut p.mezcla, n);
                    if p.mezcla.iter().flat_map(|c| &c[..n]).any(|v| !v.is_finite()) {
                        p.mezcla = entra;
                    }
                    junta(&mut p.mezcla);
                    continue;
                }
                let Some(r) = e.receta.as_deref() else { continue };
                receta::comunes(r, &e.ctl, bufs, n);
                let [a, b] = receta::voz(r, &DatosVoz::default(), &e.ctl, None, Some(&p.mezcla), &mut e.mem, bufs, n);
                let mut bien = true;
                for (l, s) in [a, b].into_iter().enumerate() {
                    p.mezcla[l][..n].copy_from_slice(&bufs[s as usize][..n]);
                    bien &= p.mezcla[l][..n].iter().all(|x| x.is_finite());
                }
                if !bien {
                    e.mem.fill(0.0);
                    p.mezcla.iter_mut().for_each(|m| m.fill(0.0));
                }
                junta(&mut p.mezcla);
            }
            if let Some(q) = &p.eq {
                for l in 0..2 {
                    for (c, z) in q.iter().zip(&mut p.eq_mem[l]) {
                        for x in &mut p.mezcla[l][..n] {
                            let y = c[0] * *x + z[0];
                            z[0] = c[1] * *x - c[3] * y + z[1];
                            z[1] = c[2] * *x - c[4] * y;
                            *x = y;
                        }
                    }
                }
            }
            if p.ancho != 1.0 || p.fase < 0.0 {
                for j in 0..n {
                    let (a, b) = (p.mezcla[0][j], p.mezcla[1][j]);
                    let (m, s) = ((a + b) * 0.5 * p.fase, (a - b) * 0.5 * p.ancho * p.fase);
                    (p.mezcla[0][j], p.mezcla[1][j]) = (m + s, m - s);
                }
            }
            if self.estado.escucha.load(Relaxed) == i as u32 {
                self.estado.oir(&p.mezcla, n);
            }
            for l in 0..2 {
                let paso = (p.gan[l] - antes[l]) / n as f32;
                let mut pico = 0f32;
                for (j, x) in p.mezcla[l][..n].iter_mut().enumerate() {
                    *x *= antes[l] + paso * j as f32;
                    pico = pico.max(x.abs());
                }
                let a = &self.estado.picos[i * 2 + l];
                if pico > f32::from_bits(a.load(Relaxed)) {
                    a.store(pico.to_bits(), Relaxed);
                }
            }
            if (1..PRIMER_ENVIO).contains(&i) {
                let (x, envios) = (otras[i - 1].mezcla, otras[i - 1].envios);
                for (k, e) in envios.into_iter().enumerate().filter(|e| e.1 > 0.0) {
                    let d = &mut otras[PRIMER_ENVIO - 1 + k];
                    d.callada = 0;
                    for l in 0..2 {
                        d.mezcla[l][..n].iter_mut().zip(&x[l][..n]).for_each(|(d, s)| *d += s * e);
                    }
                }
            }
            if i > 0 {
                for l in 0..2 {
                    for (d, s) in general[0].mezcla[l][..n].iter_mut().zip(&otras[i - 1].mezcla[l][..n]) {
                        *d += s;
                    }
                }
            }
        }
        // Limitador: baja de golpe y se recupera en ~100 ms.
        let vuelve = 1.0 - (-1.0 / (0.1 * *sr)).exp();
        let g = &pistas[0].mezcla;
        for (j, s) in sale.iter_mut().enumerate() {
            let (a, b) = (g[0][j], g[1][j]);
            let pico = a.abs().max(b.abs());
            self.limite += (1.0 - self.limite) * vuelve;
            if pico * self.limite > 0.98 {
                self.limite = 0.98 / pico;
            }
            *s = [a * self.limite, b * self.limite];
        }
        if self.metronomo && t.tocando {
            // Un clic por negra, más agudo en el primer tiempo.
            let paso = *bpm as f64 * PPQ as f64 / 60.0 / *sr as f64;
            let seg = 60.0 / *bpm as f64;
            for (j, s) in sale.iter_mut().enumerate() {
                let negras = (self.pos + j as f64 * paso) / PPQ as f64;
                let x = (negras.fract() * seg) as f32;
                if x < 0.04 {
                    let hz = if (negras as u64).is_multiple_of(4) { 1760.0 } else { 1175.0 };
                    let v = receta::seno(hz * x) * (-x * 120.0).exp() * 0.35;
                    *s = s.map(|y| (y + v).clamp(-1.0, 1.0));
                }
            }
        }
        self.estado.voces.store(vivas, Relaxed);
    }
}

/// Toda la canción (o un patrón) a un arreglo, con la cola de los efectos.
pub fn a_audio(p: &Proyecto, bib: &mut Biblioteca, modo: Modo) -> Vec<[f32; 2]> {
    let sr = bib.sr;
    let (mut m, mando) = Motor::nuevo(Escena::nueva(p, bib, modo));
    m.bucle = false;
    mando.mandar(Orden::Tocar(true));
    let mut v = vec![];
    let mut b = [[0.0f32; 2]; 512];
    let (mut callado, mut cola) = (0, 0);
    loop {
        m.tocar(&mut b);
        v.extend_from_slice(&b);
        if m.acabo {
            cola += b.len();
            callado = if b.iter().flatten().all(|x| x.abs() < 1e-4) { callado + b.len() } else { 0 };
            if callado as f32 > sr * 0.5 || cola as f32 > sr * 12.0 {
                return v;
            }
        }
    }
}
