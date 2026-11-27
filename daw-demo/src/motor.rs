// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! El motor: toca una `Escena` ya lista. Aquí no se pide ni se suelta memoria ni se espera a nadie.
use crate::biblioteca::Biblioteca;
use crate::lua::Muestras;
use crate::modelo::*;
use crate::muestrario::Muestra;
use crate::ojo::Ojo;
use crate::plugin::{EVENTOS, Evento as EvPlugin, Plugin, Tiempo};
use crate::receta::{self, BLOQUE, Buf, DatosVoz, Receta};
use crate::vivo::{Emitida, Momento as EnVivo, Vivo};
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

/// Un control que mueve otro efecto con lo que mide como «control».
struct Vinculo {
    ctl: usize,
    de: Arc<crate::biblioteca::Medidas>,
    k: usize,
    min: f32,
    max: f32,
}

impl Vinculo {
    fn mover(&self, meta: &mut [f32]) {
        let v = f32::from_bits(self.de[self.k].load(Relaxed));
        let v = if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
        // Como las perillas: las frecuencias van en octavas.
        meta[self.ctl] = if self.min > 0.0 && self.max / self.min > 20.0 {
            self.min * (self.max / self.min).powf(v)
        } else {
            self.min + (self.max - self.min) * v
        };
    }
}

/// Los vínculos puestos, ya con su lugar. Los que apuntan a algo que ya no está no cuentan.
fn vinculos(p: &Proyecto, bib: &mut Biblioteca, r: &Receta, v: &std::collections::BTreeMap<String, u64>) -> Vec<Vinculo> {
    let mut sale = vec![];
    for (clave, id) in v {
        let Some(ctl) = r.controles.iter().position(|c| c.clave == *clave) else { continue };
        let Some((i, k)) = p.controlador(*id) else { continue };
        let Some(g) = bib.ver(&p.mezcla[i].efectos[k].efecto).and_then(|g| g.receta.clone()) else { continue };
        let Some(m) = g.nombres_medidas().position(|m| m == "control") else { continue };
        let de = bib.medidas.entry((i, k)).or_default().clone();
        sale.push(Vinculo { ctl, de, k: m, min: r.controles[ctl].min, max: r.controles[ctl].max });
    }
    sale
}

struct CanalE {
    ojo: Arc<Ojo>,
    rueda: f32,
    /// Lua en vivo: el instrumento mismo, o lo que cambia sus notas.
    vivo: Option<Arc<Vivo>>,
    notas: Option<(Arc<Vivo>, Vec<f32>)>,
    /// Lo que llegó para cambiar, y lo que falta por soltar: tono y cuadros.
    llegan: Vec<EvPlugin>,
    sueltan: Vec<(u8, f64)>,
    dio: Vec<Emitida>,
    vinculos: Vec<Vinculo>,
    /// Volumen, velocidad y lado de cada pad, y su grupo de corte.
    pads: Vec<([f32; 3], u8)>,
    desde: f32,
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
    ojo: Option<Arc<Ojo>>,
    vivo: Option<Arc<Vivo>>,
    latencia: u32,
    vinculos: Vec<Vinculo>,
    /// La otra pista que oye.
    lateral: usize,
    receta: Option<Arc<Receta>>,
    plugin: Option<Arc<Plugin>>,
    ctl: Vec<f32>,
    meta: Vec<f32>,
    mem: Vec<f32>,
    /// La huella del lugar, si hace convolución.
    huella: Option<Arc<Muestra>>,
    medidas: Option<Arc<crate::biblioteca::Medidas>>,
    /// Cuánto efecto: lo que suena y a dónde va.
    mezcla: f32,
    mezcla_meta: f32,
}

struct PistaE {
    /// Cuadros seguidos sin que le entre nada, y qué tan fuerte sonó su cola en este rato y en el anterior.
    quieta: u32,
    rato: u32,
    cola: [f32; 2],
    /// Su cola no baja: se calla hasta que entre algo.
    sin_fin: bool,
    tapa: f32,
    /// Lo que espera para salir parejo con las más tardadas, y lo que espera además para llegar a la general junto con los
    /// envíos.
    espera: Espera,
    junta: Espera,
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
    /// Trae un controlador: no descansa.
    controla: bool,
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

fn hz_de(tono: u8) -> f32 {
    440.0 * 2f32.powf((tono as f32 - 69.0) / 12.0)
}

/// Espera para emparejar las pistas cuyos efectos se tardan.
#[derive(Default)]
struct Espera {
    v: Vec<[f32; 2]>,
    p: usize,
}

impl Espera {
    fn pasar(&mut self, m: &mut [Buf; 2], n: usize) {
        if self.v.is_empty() {
            return;
        }
        for j in 0..n {
            let x = std::mem::replace(&mut self.v[self.p], [m[0][j], m[1][j]]);
            (m[0][j], m[1][j]) = (x[0], x[1]);
            self.p = (self.p + 1) % self.v.len();
        }
    }
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
            // Un plugin no usa receta, pero el canal necesita una: sin esto se descartaba y no sonaba.
            let clave = if banco.is_some() || c.plugin.is_some() { "muestra" } else { c.instrumento.as_str() };
            let Some(receta) = bib.ver(clave).and_then(|g| g.receta.clone()) else { continue };
            let plugin = c.plugin.as_ref().and_then(|x| bib.rack.dar(x, sr).ok());
            let vivo = if plugin.is_none() && banco.is_none() { bib.vivo(&format!("canal {}", c.id), clave, &c.datos, None) } else { None };
            let notas = c.notas.as_ref().and_then(|k| {
                let v = bib.vivo(&format!("notas {}", c.id), k, &c.datos, None)?;
                let r = bib.ver(k)?.receta.clone()?;
                Some((v, valores(&r, &c.notas_controles)))
            });
            let voces = if plugin.is_some() || vivo.is_some() { EVENTOS / 2 } else { receta.voces };
            let suyas = if c.plugin.is_none() && banco.is_none() { bib.ver(clave).map(|g| g.ficha.muestras.clone()) } else { None };
            let pads = suyas == Some(Muestras::Pads);
            let muestra = match banco {
                None if pads => bib.pads(&c.pads),
                None if suyas == Some(Muestras::Cortes) => c.muestra.as_ref().and_then(|r| bib.cortes(r, &c.controles)),
                None if c.muestra.is_none() && matches!(suyas, Some(Muestras::Propias(_))) => bib.propias(clave),
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
            let lee = |k: &str| meta.get(receta.controles.iter().position(|x| x.clave == k).unwrap_or(usize::MAX)).copied().unwrap_or(0.0);
            let ajustes = if pads {
                (1..=PADS)
                    .map(|i| {
                        (
                            [lee(&format!("v{i}")) - 1.0, 2f32.powf(lee(&format!("t{i}")) / 12.0) - 1.0, lee(&format!("l{i}"))],
                            lee(&format!("g{i}")).round() as u8,
                        )
                    })
                    .collect()
            } else {
                vec![]
            };
            let desde = lee("inicio").clamp(0.0, 0.99);
            let vinculos = vinculos(p, bib, &receta, &c.vinculos);
            canales.push(CanalE {
                ojo: bib.ojo(&format!("canal {}", c.id)),
                rueda: 1.0,
                vinculos,
                pads: ajustes,
                desde,
                id: c.id,
                muestra,
                fija: banco.and_then(|x| x.nota),
                ctl: meta.clone(),
                meta,
                gan,
                gan_meta: gan,
                pista: c.pista.min(p.mezcla.len() - 1),
                voces: vec![Voz::default(); voces],
                mem: vec![0.0; if plugin.is_some() || vivo.is_some() { 0 } else { receta.memoria * voces }],
                vivo,
                notas,
                llegan: Vec::with_capacity(EVENTOS),
                sueltan: Vec::with_capacity(EVENTOS),
                dio: Vec::with_capacity(EVENTOS),
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
            for (k, e) in m.efectos.iter().enumerate().filter(|(_, e)| e.activo) {
                if let Some(x) = &e.plugin {
                    if let Ok(plugin) = bib.rack.dar(x, sr) {
                        let latencia = plugin.latencia();
                        efectos.push(EfectoE {
                            ojo: None,
                            receta: None,
                            plugin: Some(plugin),
                            ctl: vec![],
                            meta: vec![],
                            mem: vec![],
                            huella: None,
                            medidas: None,
                            vinculos: vec![],
                            lateral: 0,
                            vivo: None,
                            latencia,
                            mezcla: e.mezcla,
                            mezcla_meta: e.mezcla,
                        });
                    }
                    continue;
                }
                let Some(receta) = bib.ver(&e.efecto).and_then(|g| g.receta.clone()) else { continue };
                let meta = valores(&receta, &e.controles);
                let huella = if receta.con_huella() { bib.huella(e) } else { None };
                let medidas = (receta.nombres_medidas().next().is_some()).then(|| bib.medidas.entry((i, k)).or_default().clone());
                let vivo = bib.vivo(&format!("efecto {i} {k}"), &e.efecto, &e.datos, medidas.clone());
                let latencia = vivo.as_ref().map_or(0, |v| v.latencia);
                let vinculos = vinculos(p, bib, &receta, &e.vinculos);
                let pista = receta.controles.iter().position(|c| c.clave == "pista").map_or(0.0, |k| meta[k]);
                let lateral = if receta.con_lateral() { (pista.round().max(0.0) as usize).min(p.mezcla.len() - 1) } else { 0 };
                efectos.push(EfectoE {
                    ojo: Some(bib.ojo(&format!("efecto {i} {k}"))),
                    vivo,
                    latencia,
                    vinculos,
                    lateral,
                    huella,
                    medidas,
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
                quieta: 0,
                rato: 0,
                cola: [0.0; 2],
                sin_fin: false,
                tapa: 1.0,
                espera: Espera::default(),
                junta: Espera::default(),
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
                controla: m.efectos.iter().any(|e| e.activo && e.id != 0),
            });
        }
        // Emparejar: cada pista espera lo que le falta para la más tardada.
        let tarda = |p: &PistaE| (p.efectos.iter().map(|e| e.latencia as usize).sum::<usize>()).min(sr as usize);
        let tope = pistas[1..PRIMER_ENVIO].iter().map(tarda).max().unwrap_or(0);
        let tope_envios = pistas[PRIMER_ENVIO..].iter().map(tarda).max().unwrap_or(0);
        for (i, p) in pistas.iter_mut().enumerate().skip(1) {
            let t = tarda(p);
            let (a, b) = if i < PRIMER_ENVIO { (tope - t, tope_envios) } else { (0, tope_envios - t) };
            p.espera.v = vec![[0.0; 2]; a];
            p.junta.v = vec![[0.0; 2]; b];
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
                std::mem::swap(&mut c.llegan, &mut o.llegan);
                std::mem::swap(&mut c.sueltan, &mut o.sueltan);
                std::mem::swap(&mut c.mem, &mut o.mem);
                std::mem::swap(&mut c.ctl, &mut o.ctl);
                c.gan = o.gan;
                c.rueda = o.rueda;
            }
        }
        for (p, o) in self.pistas.iter_mut().zip(&mut v.pistas) {
            p.gan = o.gan;
            p.eq_mem = o.eq_mem;
            for (a, b) in [(&mut p.espera, &mut o.espera), (&mut p.junta, &mut o.junta)] {
                if a.v.len() == b.v.len() {
                    std::mem::swap(a, b);
                }
            }
            p.callada = o.callada;
            (p.quieta, p.rato, p.cola, p.sin_fin, p.tapa) = (o.quieta, o.rato, o.cola, o.sin_fin, o.tapa);
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

/// Pads de la batería y la tecla del primero.
pub const PADS: usize = 16;
pub const PRIMER_PAD: u8 = 36;
/// Cuántas ranuras de una pista pueden oír otra.
const LATERALES: usize = 16;

pub enum Orden {
    Escena(Box<Escena>),
    Tocar(bool),
    Ir(f64),
    Nota {
        canal: u32,
        tono: u8,
        fuerza: f32,
    },
    Soltar {
        canal: u32,
        tono: u8,
    },
    Silencio,
    Metronomo(bool),
    /// La rueda o un mando MIDI, para un canal.
    Mando {
        canal: u32,
        evento: EvPlugin,
    },
    /// Oír un trozo de audio suelto (el del editor), o callarlo.
    Oir(Option<Escucha>),
}

/// Lo que suena desde el editor de audio, aparte de la canción.
pub struct Escucha {
    pub datos: Arc<Vec<[f32; 2]>>,
    pub pos: f64,
    pub fin: f64,
    /// Cuadros del audio por cada cuadro de salida.
    pub paso: f64,
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
    /// Por qué cuadro va lo que se oye desde el editor; `u32::MAX` si no suena.
    pub oyendo: AtomicU32,
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
            oyendo: AtomicU32::new(u32::MAX),
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
    oidos: Receiver<Escucha>,
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
        while self.oidos.try_recv().is_ok() {}
        self.ordenes.try_send(o).is_ok()
    }
}

/// Cuando el motor vive dentro de un plugin: lo que entra y el tiempo los pone otro programa.
pub struct Externo {
    pub tiempo: Tiempo,
    /// A la pista 1, y de lateral a la 2.
    pub entra: [Buf; 2],
    pub lateral: [Buf; 2],
}

/// Qué pasa con las colas al parar.
#[derive(Clone, Copy, PartialEq)]
enum Corte {
    /// Suena normal.
    No,
    /// Se paró: todo baja en un momento.
    Baja(f32),
    /// Ya bajó: los efectos no suenan hasta que haya algo que tocar.
    Callado,
}

pub struct Motor {
    corte: Corte,
    pub externo: Option<Box<Externo>>,
    escena: Box<Escena>,
    ordenes: Receiver<Orden>,
    viejas: Sender<Box<Escena>>,
    escucha: Option<Escucha>,
    oidos: Sender<Escucha>,
    pub estado: Arc<Estado>,
    tocando: bool,
    pos: f64,
    cursor: usize,
    pub bucle: bool,
    /// Llegó al final sin bucle.
    pub acabo: bool,
    metronomo: bool,
    /// Negras que van contadas mientras la canción está parada.
    libre: f64,
    azar: u32,
    reloj: u64,
    limite: f32,
}

impl Motor {
    pub fn nuevo(e: Escena) -> (Motor, Mando) {
        let (ordenes, recibe) = bounded(256);
        let (viejas, sueltas) = bounded(64);
        let (oidos, oidas) = bounded(64);
        let estado = Arc::new(Estado::default());
        let m = Motor {
            corte: Corte::No,
            externo: None,
            escena: Box::new(e),
            ordenes: recibe,
            viejas,
            escucha: None,
            oidos,
            estado: estado.clone(),
            tocando: false,
            pos: 0.0,
            cursor: 0,
            bucle: true,
            acabo: false,
            metronomo: false,
            libre: 0.0,
            azar: 1,
            reloj: 0,
            limite: 1.0,
        };
        (m, Mando { ordenes, viejas: sueltas, oidos: oidas, estado })
    }

    fn buscar_cursor(&mut self) {
        let pos = self.pos;
        self.cursor = self.escena.eventos.partition_point(|e| e.tick < pos);
    }

    fn atender(&mut self) {
        while !self.viejas.is_full() && !self.oidos.is_full() {
            let Ok(o) = self.ordenes.try_recv() else { break };
            self.hacer(o);
        }
    }

    /// Hace una orden ya. Solo en el hilo de audio.
    pub fn hacer(&mut self, o: Orden) {
        {
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
                    // Al parar las colas bajan rápido; al tocar, todo vuelve.
                    self.corte = match (t, self.tocando) {
                        (false, true) => Corte::Baja(1.0),
                        (false, false) => self.corte,
                        _ => Corte::No,
                    };
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
                        if c.notas.is_some() && c.llegan.len() < EVENTOS {
                            c.llegan.push(EvPlugin::Suelta { tono });
                        }
                        let tono = c.fija.unwrap_or(tono);
                        let libres = c.pads.is_empty();
                        c.voces.iter_mut().filter(|v| libres && v.a_mano && v.tono == tono).for_each(|v| v.d.puerta = false);
                    }
                }
                Orden::Mando { canal, evento } => {
                    for c in self.escena.canales.iter_mut().filter(|c| c.id == canal) {
                        if (c.plugin.is_some() || c.vivo.is_some()) && c.avisos.len() < EVENTOS {
                            c.avisos.push(evento);
                        }
                        if c.notas.is_some() && c.llegan.len() < EVENTOS {
                            c.llegan.push(evento);
                        }
                        if let EvPlugin::Rueda(v) = evento {
                            // Dos semitonos por lado, como siempre.
                            c.rueda = 2f32.powf(v.clamp(-1.0, 1.0) / 6.0);
                            c.voces.iter_mut().for_each(|x| x.d.hz = hz_de(x.tono) * c.rueda);
                        }
                    }
                }
                Orden::Metronomo(si) => self.metronomo = si,
                Orden::Oir(x) => {
                    // El anterior se suelta en el hilo de la ventana.
                    if let Some(v) = std::mem::replace(&mut self.escucha, x) {
                        let _ = self.oidos.try_send(v);
                    }
                }
                Orden::Silencio => {
                    for c in &mut self.escena.canales {
                        c.voces.iter_mut().for_each(|v| if c.plugin.is_some() || c.vivo.is_some() { v.d.puerta = false } else { v.viva = false });
                    }
                    for e in self.escena.pistas.iter_mut().flat_map(|p| &mut p.efectos) {
                        e.mem.fill(0.0);
                    }
                }
            }
        }
    }

    /// Mueve un control del único efecto (o del único instrumento). Para cuando el motor vive dentro de un plugin.
    pub fn control(&mut self, k: usize, v: f32) {
        let e = self.escena.pistas.get_mut(1).and_then(|p| p.efectos.first_mut()).map(|e| &mut e.meta);
        let m = e.or_else(|| self.escena.canales.first_mut().map(|c| &mut c.meta));
        if let Some(x) = m.and_then(|m| m.get_mut(k)) {
            *x = v;
        }
    }

    fn soltar_todo(&mut self) {
        for c in &mut self.escena.canales {
            // Lo que su guion de notas tenía apretado también se suelta.
            while c.llegan.len() < EVENTOS
                && let Some((tono, _)) = c.sueltan.pop()
            {
                c.llegan.push(EvPlugin::Suelta { tono });
            }
            c.voces.iter_mut().for_each(|v| v.d.puerta = false);
        }
    }

    /// Una nota para ese canal: directo, o por su guion de notas.
    fn sonar(&mut self, i: usize, tono: u8, fuerza: f32, dur: Option<f64>) {
        let c = &mut self.escena.canales[i];
        if c.notas.is_none() {
            return self.sonar_directo(i, tono, fuerza, dur);
        }
        if c.llegan.len() < EVENTOS && c.sueltan.len() < EVENTOS {
            c.llegan.push(EvPlugin::Nota { tono, fuerza });
            if let Some(d) = dur {
                c.sueltan.push((tono, d));
            }
        }
    }

    /// Corre los guiones de notas y suena lo que manden.
    fn notas_en_vivo(&mut self, n: usize, t: &EnVivo) {
        for i in 0..self.escena.canales.len() {
            let sr = self.escena.sr;
            let c = &mut self.escena.canales[i];
            let Some((v, ctl)) = c.notas.take() else { continue };
            c.sueltan.iter_mut().for_each(|s| s.1 -= n as f64);
            while let Some(k) = c.sueltan.iter().position(|s| s.1 <= 0.0) {
                let tono = c.sueltan.swap_remove(k).0;
                if c.llegan.len() < EVENTOS {
                    c.llegan.push(EvPlugin::Suelta { tono });
                }
            }
            let mut dio = std::mem::take(&mut c.dio);
            let mut nada = [[0.0; BLOQUE]; 2];
            v.procesar(&c.llegan, &ctl, t, &mut nada, &[[0.0; BLOQUE]; 2], n, &mut dio);
            c.llegan.clear();
            c.notas = Some((v, ctl));
            for e in dio.drain(..) {
                self.sonar_directo(i, e.tono, e.fuerza, Some((e.segundos * sr) as f64));
            }
            self.escena.canales[i].dio = dio;
        }
    }

    fn sonar_directo(&mut self, i: usize, tono: u8, fuerza: f32, dur: Option<f64>) {
        self.corte = Corte::No;
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
        if c.plugin.is_some() || c.vivo.is_some() {
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
        let golpe = tono.checked_sub(PRIMER_PAD).map(|k| k as usize).filter(|k| *k < c.pads.len());
        if let Some(k) = golpe.filter(|k| c.pads[*k].1 > 0) {
            // Los del mismo grupo se callan, como un platillo al cerrarse.
            let g = c.pads[k].1;
            for v in c.voces.iter_mut().filter(|v| v.viva) {
                if v.tono.checked_sub(PRIMER_PAD).and_then(|q| c.pads.get(q as usize)).is_some_and(|q| q.1 == g) {
                    v.d.puerta = false;
                }
            }
        }
        let pad = golpe.map_or([0.0; 3], |k| c.pads[k].0);
        // Los pads suenan completos aunque la nota sea corta.
        let dur = if c.pads.is_empty() { dur } else { dur.map(|_| f64::MAX) };
        let (zonas, cuantas) = c.muestra.as_ref().map_or(Default::default(), |x| x.elegir(tono, fuerza));
        c.voces[k] = Voz {
            viva: true,
            d: DatosVoz {
                bpm: self.escena.bpm,
                negra: (self.pos / PPQ as f64) as f32,
                hz: hz_de(tono) * c.rueda,
                fuerza,
                azar: self.azar as f32 / u32::MAX as f32,
                puerta: true,
                pad,
                desde: c.desde,
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
        let mut resto = &mut sale[..];
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
            if let Some(x) = &self.externo {
                self.escena.bpm = if x.tiempo.bpm > 1.0 { x.tiempo.bpm as f32 } else { 120.0 };
            }
            let t = if let Some(x) = &self.externo {
                x.tiempo
            } else {
                Tiempo { bpm: self.escena.bpm as f64, pos: self.pos / PPQ as f64, tocando: self.tocando && !self.acabo, sr: self.escena.sr as f64 }
            };
            let en = EnVivo { bpm: self.escena.bpm, pos: if t.tocando { t.pos } else { self.libre }, tocando: t.tocando };
            self.notas_en_vivo(n, &en);
            self.bloque(ahora, &t);
            if self.tocando && !self.acabo {
                self.pos += n as f64 * paso;
            }
            resto = luego;
        }
        self.oir_suelto(sale);
        self.estado.tocando.store(self.tocando && !self.acabo, Relaxed);
        self.estado.tick.store((self.pos as f32).to_bits(), Relaxed);
        self.estado.hora.store(self.estado.base.elapsed().as_micros() as u64, Relaxed);
    }

    fn oir_suelto(&mut self, sale: &mut [[f32; 2]]) {
        let mut va = u32::MAX;
        if let Some(e) = &mut self.escucha {
            let fin = e.fin.min(e.datos.len() as f64);
            for s in sale.iter_mut() {
                if e.pos >= fin {
                    break;
                }
                let (i, f) = (e.pos as usize, e.pos.fract() as f32);
                let (a, b) = (e.datos[i], e.datos.get(i + 1).copied().unwrap_or([0.0; 2]));
                *s = [0, 1].map(|l| (s[l] + a[l] + (b[l] - a[l]) * f).clamp(-1.0, 1.0));
                e.pos += e.paso;
            }
            if e.pos < fin {
                va = e.pos as u32;
            }
        }
        self.estado.oyendo.store(va, Relaxed);
    }

    fn bloque(&mut self, sale: &mut [[f32; 2]], t: &Tiempo) {
        let n = sale.len();
        let Escena { canales, audios, pistas, bufs, sr, bpm, .. } = &mut *self.escena;
        let negra = if t.tocando { t.pos } else { self.libre } as f32;
        if !t.tocando {
            self.libre += n as f64 * *bpm as f64 / 60.0 / *sr as f64;
        }
        let en = EnVivo { bpm: *bpm, pos: negra as f64, tocando: t.tocando };
        // Suaviza en unos 5 ms.
        let k = 1.0 - (-(n as f32) / (0.005 * *sr)).exp();
        for p in pistas.iter_mut() {
            p.mezcla[0][..n].fill(0.0);
            p.mezcla[1][..n].fill(0.0);
        }
        if let Some(x) = &self.externo {
            for (k, b) in [(1, &x.entra), (2, &x.lateral)] {
                pistas[k].mezcla = *b;
                pistas[k].callada = 0;
            }
        }
        let mut vivas = 0;
        let mudo = [[0.0; BLOQUE]; 2];
        for c in canales.iter_mut() {
            let antes = c.gan;
            for l in 0..2 {
                c.gan[l] += (c.gan_meta[l] - c.gan[l]) * k;
            }
            for v in &c.vinculos {
                v.mover(&mut c.meta);
            }
            for (v, m) in c.ctl.iter_mut().zip(&c.meta) {
                *v += (*m - *v) * k;
            }
            if c.plugin.is_some() || c.vivo.is_some() {
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
                match (&c.plugin, &c.vivo) {
                    (Some(p), _) => p.procesar(&c.avisos, t, &mudo, &mut b, n),
                    (_, Some(v)) => v.procesar(&c.avisos, &c.ctl, &en, &mut b, &mudo, n, &mut c.dio),
                    _ => {}
                }
                c.avisos.clear();
                let pico = b.iter().flat_map(|x| &x[..n]).fold(0f32, |m, x| m.max(x.abs()));
                if b.iter().flat_map(|x| &x[..n]).any(|x| !x.is_finite()) {
                    continue;
                }
                c.callada = if pico < 1e-5 { c.callada.saturating_add(n as u32) } else { 0 };
                if c.ojo.mirando() {
                    c.ojo.anotar(&mudo, &b, n);
                }
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
            let antes_de = c.ojo.mirando().then_some(destino.mezcla);
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
            if let Some(q) = antes_de {
                let mut solo = destino.mezcla;
                for l in 0..2 {
                    solo[l][..n].iter_mut().zip(&q[l][..n]).for_each(|(s, q)| *s -= q);
                }
                c.ojo.anotar(&mudo, &solo, n);
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
        let mut nada: Vec<Emitida> = Vec::new();
        let mut laterales = [[[0.0; BLOQUE]; 2]; LATERALES];
        for i in (1..pistas.len()).chain([0]) {
            // Lo que oyen de otras pistas sus efectos, antes de tomar esta.
            laterales.iter_mut().for_each(|x| *x = [[0.0; BLOQUE]; 2]);
            for (k, e) in pistas[i].efectos.iter().enumerate().take(LATERALES).filter(|(_, e)| e.lateral > 0) {
                laterales[k] = pistas[e.lateral].mezcla;
            }
            let (general, otras) = pistas.split_at_mut(1);
            let p = if i == 0 { &mut general[0] } else { &mut otras[i - 1] };
            let antes = p.gan;
            for l in 0..2 {
                p.gan[l] += (p.gan_meta[l] - p.gan[l]) * k;
            }
            // Diez segundos callada: ni sus efectos tienen cola.
            if i > 0 && !p.controla && p.callada as f32 > 10.0 * *sr {
                continue;
            }
            p.callada = p.callada.saturating_add(n as u32);
            // Lo que le entra, antes de sus efectos.
            let entra_algo = p.mezcla.iter().any(|b| b[..n].iter().any(|x| x.abs() > 1e-5));
            if entra_algo {
                (p.quieta, p.rato, p.cola, p.sin_fin) = (0, 0, [0.0; 2], false);
            } else {
                p.quieta = p.quieta.saturating_add(n as u32);
            }
            let callado = self.corte == Corte::Callado;
            for (ranura, e) in p.efectos.iter_mut().enumerate().filter(|(_, e)| !callado || e.plugin.is_some()) {
                for v in &e.vinculos {
                    v.mover(&mut e.meta);
                }
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
                let otra = laterales.get(ranura).copied().unwrap_or([[0.0; BLOQUE]; 2]);
                let entra = [p.mezcla[0], p.mezcla[1], otra[0], otra[1]];
                receta::comunes(r, &e.ctl, bufs, n);
                let [a, b] = receta::voz(
                    r,
                    &DatosVoz { bpm: *bpm, negra, ..Default::default() },
                    &e.ctl,
                    e.huella.as_deref(),
                    Some(&entra),
                    &mut e.mem,
                    bufs,
                    n,
                );
                if let Some(v) = &e.vivo {
                    let mut x = [bufs[a as usize], bufs[b as usize]];
                    v.procesar(&[], &e.ctl, &en, &mut x, &otra, n, &mut nada);
                    (bufs[a as usize], bufs[b as usize]) = (x[0], x[1]);
                }
                if let Some(x) = &e.medidas {
                    r.medidas.iter().zip(x.iter()).for_each(|((_, off), x)| x.store(e.mem[*off].to_bits(), Relaxed));
                }
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
                if let Some(o) = e.ojo.as_ref().filter(|o| o.mirando()) {
                    o.anotar(&seco, &p.mezcla, n);
                }
            }
            // Una cola que no baja no es una cola: un silbido, un ruido. Se calla sola hasta que entre algo.
            if p.quieta as f32 > *sr && !p.efectos.is_empty() && !p.efectos.iter().any(|e| e.receta.as_ref().is_some_and(|r| r.sostiene)) {
                p.cola[1] = p.mezcla.iter().flat_map(|b| &b[..n]).fold(p.cola[1], |m, x| m.max(x.abs()));
                p.rato += n as u32;
                if p.rato as f32 > *sr * 1.5 {
                    p.sin_fin |= p.cola[1] > 1e-4 && p.cola[0] > 0.0 && p.cola[1] > p.cola[0] * 0.97;
                    (p.cola, p.rato) = ([p.cola[1], 0.0], 0);
                }
            }
            let tapa = if p.sin_fin { 0.0 } else { 1.0 };
            if p.tapa != tapa || tapa == 0.0 {
                let paso = (tapa - p.tapa).clamp(-1.0, 1.0) * k.min(0.2);
                for j in 0..n {
                    let g = (p.tapa + paso * j as f32 / n as f32).clamp(0.0, 1.0);
                    p.mezcla[0][j] *= g;
                    p.mezcla[1][j] *= g;
                }
                p.tapa += paso;
                if (p.tapa - tapa).abs() < 1e-3 {
                    p.tapa = tapa;
                }
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
            p.espera.pasar(&mut p.mezcla, n);
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
                let q = &mut otras[i - 1];
                q.junta.pasar(&mut q.mezcla, n);
                for l in 0..2 {
                    for (d, s) in general[0].mezcla[l][..n].iter_mut().zip(&otras[i - 1].mezcla[l][..n]) {
                        *d += s;
                    }
                }
            }
        }
        // Al parar: todo baja en unos tres décimos de segundo y los efectos olvidan lo que traían.
        if let Corte::Baja(g) = self.corte {
            let paso = 1.0 / (0.3 * *sr);
            for j in 0..n {
                let h = (g - paso * j as f32).max(0.0);
                pistas[0].mezcla[0][j] *= h;
                pistas[0].mezcla[1][j] *= h;
            }
            let g = g - paso * n as f32;
            self.corte = if g > 0.0 { Corte::Baja(g) } else { Corte::Callado };
            if g <= 0.0 {
                for e in pistas.iter_mut().flat_map(|p| &mut p.efectos) {
                    e.mem.fill(0.0);
                }
            }
        } else if self.corte == Corte::Callado {
            pistas[0].mezcla.iter_mut().for_each(|b| b[..n].fill(0.0));
        }
        // Limitador: baja de golpe y se recupera en ~100 ms.
        let vuelve = 1.0 - (-1.0 / (0.1 * *sr)).exp();
        let g = &pistas[0].mezcla;
        for (j, s) in sale.iter_mut().enumerate() {
            let (a, b) = (g[0][j], g[1][j]);
            if self.externo.is_some() {
                *s = [a, b];
                continue;
            }
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
