// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Lo que se guarda: canales, patrones, clips y mezcla.
use crate::idioma::{t, tf};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Pulsos por negra. 96 es también lo normal en los .flp.
pub const PPQ: u32 = 96;
pub const PASO: u32 = PPQ / 4;
pub const COMPAS: u32 = PPQ * 4;
/// La general, 32 pistas y los envíos al final. Hasta la 0.12.4 eran 12 pistas (17 en total): `de_json` las acomoda.
pub const PISTAS_MEZCLA: usize = 37;
/// Cuántas había antes, para abrir los proyectos viejos.
const PISTAS_MEZCLA_VIEJAS: usize = 17;
/// Las últimas pistas de mezcla son de envío.
pub const ENVIOS: usize = 4;
pub const PRIMER_ENVIO: usize = PISTAS_MEZCLA - ENVIOS;
pub const PISTAS_ARREGLO: usize = 32;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Nota {
    pub tick: u32,
    pub dur: u32,
    pub tono: u8,
    /// 0 a 1.
    pub fuerza: f32,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Canal {
    pub id: u32,
    pub nombre: String,
    pub color: [u8; 3],
    /// Clave del instrumento en la biblioteca de Lua.
    pub instrumento: String,
    /// Controles movidos; los demás usan su valor de fábrica.
    #[serde(default)]
    pub controles: BTreeMap<String, f32>,
    /// WAV, SF2 o SFZ para los instrumentos que llaman a `muestra()`.
    #[serde(default)]
    pub muestra: Option<String>,
    /// Cuál instrumento del SF2.
    #[serde(default)]
    pub preset: usize,
    #[serde(default)]
    pub plugin: Option<PluginPuesto>,
    pub volumen: f32,
    pub pan: f32,
    #[serde(default)]
    pub mudo: bool,
    #[serde(default)]
    pub solo: bool,
    /// Pista de mezcla (0 es la general).
    #[serde(default)]
    pub pista: usize,
    /// Los audios de cada pad de la batería.
    #[serde(default)]
    pub pads: Vec<String>,
    /// El guion que cambia las notas antes de que suenen, y sus controles.
    #[serde(default)]
    pub notas: Option<String>,
    #[serde(default)]
    pub notas_controles: BTreeMap<String, f32>,
    #[serde(default)]
    pub datos: BTreeMap<String, String>,
    /// Qué controlador mueve cada control, por su `id`.
    #[serde(default)]
    pub vinculos: BTreeMap<String, u64>,
}

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Patron {
    pub id: u32,
    pub nombre: String,
    pub color: [u8; 3],
    /// Por id de canal.
    pub notas: BTreeMap<u32, Vec<Nota>>,
    /// Largo mínimo, en compases.
    pub compases: u32,
}

impl Patron {
    /// Largo en pulsos: compases enteros, y crece si hay notas más allá.
    pub fn largo(&self) -> u32 {
        let fin = self.notas.values().flatten().map(|n| n.tick + 1).max().unwrap_or(0);
        fin.div_ceil(COMPAS).max(self.compases).max(1) * COMPAS
    }

    /// Lo que suena al repetir el patrón durante `dur`, empezando en `desfase`: canal, pulso, nota y cuánto dura.
    pub fn notas_en(&self, desfase: u32, dur: u32) -> impl Iterator<Item = (u32, u32, &Nota, u32)> {
        let (largo, fin) = (self.largo(), desfase + dur);
        (desfase / largo * largo..fin).step_by(largo as usize).flat_map(move |base| {
            self.notas.iter().flat_map(move |(c, v)| v.iter().map(move |n| (*c, n))).filter_map(move |(c, n)| {
                let t = n.tick + base;
                (t >= desfase && t < fin).then(|| (c, t - desfase, n, n.dur.min(fin - t)))
            })
        })
    }
}

/// Un patrón o un audio puesto en la canción.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Clip {
    pub patron: u32,
    pub pista: usize,
    pub tick: u32,
    pub dur: u32,
    /// Pulsos del patrón que se saltan al inicio.
    #[serde(default)]
    pub desfase: u32,
    #[serde(default)]
    pub audio: Option<ClipAudio>,
}

impl Clip {
    /// Lo parte en ese pulso y devuelve el segundo pedazo. Nada si el corte cae fuera.
    pub fn cortar(&mut self, en: u32, bpm: f32) -> Option<Clip> {
        if en <= self.tick || en >= self.tick + self.dur {
            return None;
        }
        let mut b = self.clone();
        (b.tick, b.dur) = (en, self.tick + self.dur - en);
        match &mut b.audio {
            Some(a) => {
                a.inicio += (en - self.tick) as f32 * 60.0 / bpm / PPQ as f32 * a.velocidad;
                a.entra = 0.0;
            }
            None => b.desfase += en - self.tick,
        }
        if let Some(a) = &mut self.audio {
            a.sale = 0.0;
        }
        self.dur = en - self.tick;
        Some(b)
    }

    pub fn de_patron(patron: u32, pista: usize, tick: u32, dur: u32) -> Clip {
        Clip { patron, pista, tick, dur, desfase: 0, audio: None }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ClipAudio {
    pub ruta: String,
    pub nombre: String,
    /// Segundos del archivo que se saltan al inicio.
    #[serde(default)]
    pub inicio: f32,
    pub volumen: f32,
    /// Pista de mezcla.
    #[serde(default)]
    pub mezcla: usize,
    #[serde(default)]
    pub pan: f32,
    /// Fundidos de entrada y salida, en segundos.
    #[serde(default)]
    pub entra: f32,
    #[serde(default)]
    pub sale: f32,
    /// Más rápido suena más agudo.
    #[serde(default = "uno")]
    pub velocidad: f32,
    /// La curva de volumen: puntos de (segundo del archivo, ganancia de 0 a 2). Por ir en segundos del archivo no se
    /// mueve al recortar ni al partir el clip. Vacía es plana.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub envolvente: Vec<[f32; 2]>,
}

/// Cuánto suena la curva `e` en el segundo `s` del clip: recta entre puntos, igual antes del primero y después del último.
pub fn envolvente(e: &[[f32; 2]], s: f32) -> f32 {
    let i = e.partition_point(|p| p[0] <= s);
    match (i.checked_sub(1).and_then(|k| e.get(k)), e.get(i)) {
        (None, None) => 1.0,
        (Some(a), None) => a[1],
        (None, Some(b)) => b[1],
        (Some(a), Some(b)) => {
            let t = if b[0] > a[0] { (s - a[0]) / (b[0] - a[0]) } else { 1.0 };
            a[1] + (b[1] - a[1]) * t
        }
    }
}

impl Default for ClipAudio {
    fn default() -> ClipAudio {
        ClipAudio {
            ruta: String::new(),
            nombre: String::new(),
            inicio: 0.0,
            volumen: 1.0,
            mezcla: 0,
            pan: 0.0,
            entra: 0.0,
            sale: 0.0,
            velocidad: 1.0,
            envolvente: vec![],
        }
    }
}

fn uno() -> f32 {
    1.0
}

/// Un renglón de la playlist.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct PistaArreglo {
    #[serde(default)]
    pub nombre: String,
    #[serde(default)]
    pub mudo: bool,
    #[serde(default)]
    pub solo: bool,
    /// Color de su placa y de sus audios.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct EfectoPuesto {
    pub efecto: String,
    #[serde(default)]
    pub controles: BTreeMap<String, f32>,
    pub activo: bool,
    /// 0 es el sonido sin efecto, 1 solo el efecto.
    #[serde(default = "uno")]
    pub mezcla: f32,
    #[serde(default)]
    pub plugin: Option<PluginPuesto>,
    /// El audio que usa de huella un efecto de convolución.
    #[serde(default)]
    pub muestra: Option<String>,
    /// Para que otros lo encuentren aunque cambie de lugar; 0 si nadie lo necesita.
    #[serde(default)]
    pub id: u64,
    /// Qué controlador mueve cada control, por su `id`.
    #[serde(default)]
    pub vinculos: BTreeMap<String, u64>,
    /// Textos que guarda su guion.
    #[serde(default)]
    pub datos: BTreeMap<String, String>,
    /// Es copia de un efecto de la clase de su pista: cambiarlo cambia el de todas.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub de_clase: bool,
}

impl EfectoPuesto {
    pub fn nuevo(efecto: &str) -> EfectoPuesto {
        EfectoPuesto {
            efecto: efecto.into(),
            controles: BTreeMap::new(),
            activo: true,
            mezcla: 1.0,
            plugin: None,
            muestra: None,
            id: 0,
            vinculos: BTreeMap::new(),
            datos: BTreeMap::new(),
            de_clase: false,
        }
    }
}

/// Un plugin VST3 o CLAP del proyecto.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PluginPuesto {
    /// Único en el proyecto: une esto con la copia que está sonando.
    pub uid: u64,
    pub ruta: String,
    pub id: String,
    pub nombre: String,
    /// Su estado, en hexadecimal. Se actualiza al guardar.
    #[serde(default)]
    pub estado: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PistaMezcla {
    pub nombre: String,
    pub volumen: f32,
    pub pan: f32,
    #[serde(default)]
    pub mudo: bool,
    #[serde(default)]
    pub solo: bool,
    /// Cuánto manda a cada pista de envío.
    #[serde(default)]
    pub envios: [f32; ENVIOS],
    /// Graves, medios y agudos, en decibeles.
    #[serde(default)]
    pub eq: [f32; 3],
    /// 0 es mono, 1 normal y 2 el doble de abierto.
    #[serde(default = "uno")]
    pub ancho: f32,
    /// Invierte la polaridad.
    #[serde(default)]
    pub fase: bool,
    #[serde(default)]
    pub efectos: Vec<EfectoPuesto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
    /// Grupo desplegable del Mixer en que va (`Proyecto::grupos`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grupo: Option<u64>,
    /// Clase cuyos efectos comparte (`Proyecto::clases`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clase: Option<u64>,
}

/// Varias pistas del Mixer juntas, que se pliegan en una sola tira.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GrupoMezcla {
    pub id: u64,
    pub nombre: String,
    pub color: [u8; 3],
    #[serde(default)]
    pub plegado: bool,
}

/// Pistas que comparten los mismos efectos, como las clases de clips de Movie: cada una suena con su copia y cambiar
/// uno cambia el de todas.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ClaseMezcla {
    pub id: u64,
    pub nombre: String,
    pub color: [u8; 3],
    #[serde(default)]
    pub efectos: Vec<EfectoPuesto>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Proyecto {
    pub nombre: String,
    pub bpm: f32,
    /// Retraso de los pasos impares, 0 a 1.
    #[serde(default)]
    pub swing: f32,
    pub canales: Vec<Canal>,
    pub patrones: Vec<Patron>,
    pub clips: Vec<Clip>,
    #[serde(default)]
    pub pistas: Vec<PistaArreglo>,
    pub mezcla: Vec<PistaMezcla>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grupos: Vec<GrupoMezcla>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clases: Vec<ClaseMezcla>,
    /// Cuántas pistas (sin la general ni los envíos) se ven en el Mixer; «+ Pista» muestra otra.
    #[serde(default = "doce")]
    pub vistas: usize,
    /// El último número dado a un grupo o una clase.
    #[serde(default)]
    pub sig: u64,
}

fn doce() -> usize {
    12
}

pub const COLORES: [[u8; 3]; 8] =
    [[255, 138, 76], [94, 200, 255], [180, 136, 255], [110, 224, 140], [255, 204, 84], [255, 110, 150], [90, 220, 205], [160, 175, 255]];

impl Default for Proyecto {
    fn default() -> Self {
        let mezcla = (0..PISTAS_MEZCLA)
            .map(|i| PistaMezcla {
                nombre: match i {
                    0 => t("General").into(),
                    i if i >= PRIMER_ENVIO => tf("Envío {}", &[&(i - PRIMER_ENVIO + 1)]),
                    i => tf("Pista {}", &[&i]),
                },
                volumen: 0.8,
                pan: 0.0,
                mudo: false,
                solo: false,
                envios: [0.0; ENVIOS],
                eq: [0.0; 3],
                ancho: 1.0,
                fase: false,
                efectos: match i.checked_sub(PRIMER_ENVIO) {
                    // Listos para usarse: reverberación y eco, sin el sonido directo.
                    Some(k @ (0 | 1)) => {
                        let controles = [("seco".to_string(), 0.0), ("mezcla".to_string(), 1.0)].into();
                        vec![EfectoPuesto { controles, ..EfectoPuesto::nuevo(["reverb", "eco"][k]) }]
                    }
                    _ => vec![],
                },
                color: None,
                grupo: None,
                clase: None,
            })
            .collect();
        let patron = Patron { id: 1, nombre: tf("Patrón {}", &[&1]), color: COLORES[0], notas: BTreeMap::new(), compases: 1 };
        Proyecto {
            nombre: t("Sin título").into(),
            bpm: 120.0,
            swing: 0.0,
            canales: vec![],
            patrones: vec![patron],
            clips: vec![],
            pistas: vec![Default::default(); PISTAS_ARREGLO],
            mezcla,
            grupos: vec![],
            clases: vec![],
            vistas: doce(),
            sig: 0,
        }
    }
}

impl Proyecto {
    pub fn agregar_canal(&mut self, instrumento: &str, nombre: &str) -> u32 {
        let id = self.canales.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        self.canales.push(Canal {
            id,
            nombre: nombre.into(),
            color: COLORES[id as usize % COLORES.len()],
            instrumento: instrumento.into(),
            controles: BTreeMap::new(),
            muestra: None,
            preset: 0,
            plugin: None,
            volumen: 0.8,
            pan: 0.0,
            mudo: false,
            solo: false,
            pista: 0,
            pads: vec![],
            notas: None,
            notas_controles: BTreeMap::new(),
            datos: BTreeMap::new(),
            vinculos: BTreeMap::new(),
        });
        id
    }

    pub fn agregar_patron(&mut self) -> u32 {
        let id = self.patrones.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        self.patrones.push(Patron {
            id,
            nombre: tf("Patrón {}", &[&id]),
            color: COLORES[(id as usize - 1) % COLORES.len()],
            notas: BTreeMap::new(),
            compases: 1,
        });
        id
    }

    pub fn patron(&self, id: u32) -> Option<&Patron> {
        self.patrones.iter().find(|p| p.id == id)
    }

    /// ¿Se oye esa pista de la playlist?
    pub fn pista_suena(&self, i: usize) -> bool {
        let solo = self.pistas.iter().any(|x| x.solo);
        self.pistas.get(i).is_none_or(|x| !x.mudo && (!solo || x.solo))
    }

    /// Una pista de mezcla que nadie usa, ya con ese nombre. Si no queda ninguna, la general.
    pub fn pista_para(&mut self, nombre: &str) -> usize {
        let usada = |i: usize| {
            self.canales.iter().any(|c| c.pista == i)
                || self.clips.iter().any(|c| c.audio.as_ref().is_some_and(|a| a.mezcla == i))
                || !self.mezcla[i].efectos.is_empty()
        };
        let Some(i) = (1..PRIMER_ENVIO).find(|i| !usada(*i)) else { return 0 };
        self.mezcla[i].nombre = nombre.chars().take(24).collect();
        i
    }

    /// Dónde está el controlador con ese `id`, si sigue puesto y prendido.
    pub fn controlador(&self, id: u64) -> Option<(usize, usize)> {
        let de = |m: &PistaMezcla| m.efectos.iter().position(|e| e.id == id && e.activo);
        self.mezcla.iter().enumerate().find_map(|(i, m)| Some((i, de(m)?))).filter(|_| id != 0)
    }

    /// Pulsos que dura un audio de esos segundos.
    pub fn pulsos(&self, segundos: f32) -> u32 {
        ((segundos * self.bpm / 60.0 * PPQ as f32).round() as u32).max(1)
    }

    /// Largo de la canción en pulsos.
    pub fn largo(&self) -> u32 {
        self.clips.iter().map(|c| c.tick + c.dur).max().unwrap_or(0).max(COMPAS)
    }

    /// Abre el texto de un proyecto y deja en regla lo que el resto del programa da por hecho.
    pub fn de_json(t: &str) -> Result<Proyecto, String> {
        let mut p: Proyecto = serde_json::from_str(t).map_err(|e| tf("No es un proyecto válido: {}", &[&e]))?;
        let fabrica = Proyecto::default();
        if p.patrones.is_empty() {
            p.patrones = fabrica.patrones;
        }
        p.pistas.resize(PISTAS_ARREGLO, Default::default());
        if p.mezcla.len() == PISTAS_MEZCLA_VIEJAS {
            // De antes de la 0.12.5: los envíos pasan al final de las pistas nuevas y lo que iba a ellos los sigue.
            let viejo = PISTAS_MEZCLA_VIEJAS - ENVIOS;
            let envios = p.mezcla.split_off(viejo);
            p.mezcla.extend_from_slice(&fabrica.mezcla[viejo..PRIMER_ENVIO]);
            p.mezcla.extend(envios);
            let corre = |i: &mut usize| {
                if *i >= viejo {
                    *i += PRIMER_ENVIO - viejo;
                }
            };
            p.canales.iter_mut().for_each(|c| corre(&mut c.pista));
            p.clips.iter_mut().filter_map(|c| c.audio.as_mut()).for_each(|a| corre(&mut a.mezcla));
            for e in p.mezcla.iter_mut().flat_map(|m| &mut m.efectos) {
                if let Some(v) = e.controles.get_mut("pista") {
                    let mut i = v.max(0.0).round() as usize;
                    corre(&mut i);
                    *v = i as f32;
                }
            }
        }
        p.mezcla.truncate(PISTAS_MEZCLA);
        let n = p.mezcla.len();
        p.mezcla.extend_from_slice(&fabrica.mezcla[n..]);
        // Que se vea toda pista que ya se usa.
        let usada = (1..PRIMER_ENVIO)
            .filter(|i| {
                p.canales.iter().any(|c| c.pista == *i)
                    || p.clips.iter().any(|c| c.audio.as_ref().is_some_and(|a| a.mezcla == *i))
                    || !p.mezcla[*i].efectos.is_empty()
            })
            .max()
            .unwrap_or(0);
        p.vistas = p.vistas.max(usada).clamp(1, PRIMER_ENVIO - 1);
        for c in &mut p.clips {
            if let Some(a) = &mut c.audio {
                a.envolvente.retain(|x| x[0].is_finite() && x[1].is_finite());
                a.envolvente.iter_mut().for_each(|x| x[1] = x[1].clamp(0.0, 2.0));
                a.envolvente.sort_by(|x, y| x[0].total_cmp(&y[0]));
            }
        }
        p.bpm = if p.bpm.is_finite() { p.bpm.clamp(20.0, 400.0) } else { 120.0 };
        for c in &mut p.canales {
            c.pista = c.pista.min(PISTAS_MEZCLA - 1);
        }
        for c in &mut p.clips {
            c.pista = c.pista.min(PISTAS_ARREGLO - 1);
            c.dur = c.dur.max(1);
            if let Some(a) = &mut c.audio {
                a.velocidad = a.velocidad.clamp(0.1, 8.0);
                a.mezcla = a.mezcla.min(PISTAS_MEZCLA - 1);
            }
        }
        for n in p.patrones.iter_mut().flat_map(|p| p.notas.values_mut()).flatten() {
            n.tono = n.tono.min(127);
            n.dur = n.dur.max(1);
        }
        Ok(p)
    }

    /// La canción con la que abre la demo.
    pub fn demo() -> Proyecto {
        let mut p = Proyecto { nombre: "Demo".into(), bpm: 100.0, swing: 0.15, ..Default::default() };
        let bombo = p.agregar_canal("bombo", t("Bombo"));
        let caja = p.agregar_canal("caja", t("Caja"));
        let hat = p.agregar_canal("platillo", t("Platillo"));
        let bajo = p.agregar_canal("bajo", t("Bajo"));
        let piano = p.agregar_canal("piano", t("Piano"));
        let colchon = p.agregar_canal("colchon", t("Colchón"));
        for (i, c) in p.canales.iter_mut().enumerate() {
            c.pista = [1, 2, 2, 3, 4, 5][i];
            c.volumen = [0.9, 0.7, 0.3, 0.7, 0.5, 0.4][i];
        }
        for (i, n) in ["Bombo", "Batería", "Bajo", "Campana", "Colchón"].iter().enumerate() {
            p.mezcla[i + 1].nombre = t(n).to_string();
        }
        p.mezcla[2].envios[0] = 0.12;
        p.mezcla[4].envios = [0.25, 0.3, 0.0, 0.0];
        p.mezcla[5].envios[0] = 0.4;
        p.mezcla[0].efectos.push(EfectoPuesto::nuevo("limitador"));

        let n = |paso: u32, pasos: u32, tono: u8, f: f32| Nota { tick: paso * PASO, dur: pasos * PASO, tono, fuerza: f };
        let pat = &mut p.patrones[0];
        pat.nombre = t("Ritmo").into();
        pat.notas.insert(bombo, [0, 4, 8, 12].map(|x| n(x, 1, 60, 1.0)).to_vec());
        pat.notas.insert(caja, [3, 6, 11, 14].map(|x| n(x, 1, 60, 0.8)).to_vec());
        pat.notas.insert(hat, (0..16).map(|x| n(x, 1, 60, if x % 4 == 2 { 1.0 } else { 0.55 })).collect());
        let armonia = p.agregar_patron();
        let pat = p.patrones.last_mut().unwrap();
        pat.nombre = t("Armonía").into();
        pat.compases = 4;
        // Am, F, C, G.
        let acordes: [[u8; 3]; 4] = [[57, 60, 64], [53, 57, 60], [48, 55, 64], [55, 59, 62]];
        for (i, ac) in acordes.iter().enumerate() {
            let b = i as u32 * 16;
            pat.notas.entry(colchon).or_default().extend(ac.map(|t| n(b, 16, t + 12, 0.7)));
            let raiz = ac[0] - 12;
            pat.notas.entry(bajo).or_default().extend([0, 3, 6, 8, 11, 14].map(|x| n(b + x, 2, raiz, 0.9)));
        }
        let melodia = p.agregar_patron();
        let pat = p.patrones.last_mut().unwrap();
        pat.nombre = t("Melodía").into();
        pat.compases = 4;
        for (i, ac) in acordes.iter().enumerate() {
            let b = i as u32 * 16;
            let notas = [(0, ac[2]), (3, ac[1]), (6, ac[2]), (8, ac[0] + 12), (11, ac[2]), (14, ac[1])];
            pat.notas.entry(piano).or_default().extend(notas.map(|(x, t)| n(b + x, 2, t + 12, 0.75)));
        }
        let mut clip = |patron, pista, compas: u32, largo: u32| p.clips.push(Clip::de_patron(patron, pista, compas * COMPAS, largo * COMPAS));
        clip(armonia, 1, 0, 4);
        clip(melodia, 2, 0, 4);
        for c in 4..12 {
            clip(1, 0, c, 1);
        }
        clip(armonia, 1, 4, 8);
        clip(melodia, 2, 8, 4);
        p
    }
}

impl Proyecto {
    /// Un número nuevo para un grupo o una clase.
    pub fn id_nuevo(&mut self) -> u64 {
        self.sig = self.sig.max(self.grupos.iter().map(|g| g.id).chain(self.clases.iter().map(|c| c.id)).max().unwrap_or(0)) + 1;
        self.sig
    }

    /// Deja iguales los efectos de cada clase en todas sus pistas. Si en una pista cambió una copia (comparado con
    /// `antes`), ese cambio pasa a la clase y de ahí a las demás. Las copias van primero en la cadena de cada pista.
    pub fn igualar_clases(&mut self, antes: Option<&Proyecto>) {
        let copias = |m: &PistaMezcla| m.efectos.iter().filter(|e| e.de_clase).cloned().collect::<Vec<_>>();
        for k in 0..self.clases.len() {
            let id = self.clases[k].id;
            let mut lista = self.clases[k].efectos.clone();
            for (i, m) in self.mezcla.iter().enumerate().filter(|(_, m)| m.clase == Some(id)) {
                let ahora = copias(m);
                let era = antes.and_then(|a| a.mezcla.get(i)).filter(|a| a.clase == Some(id)).map(copias);
                if era.is_some_and(|e| e != ahora) {
                    lista = ahora;
                    break;
                }
            }
            // Los VST3 y CLAP tienen una sola copia sonando: no se comparten.
            lista.retain(|e| e.plugin.is_none());
            lista.iter_mut().for_each(|e| (e.de_clase, e.id) = (true, 0));
            self.clases[k].efectos = lista.clone();
            for m in self.mezcla.iter_mut().filter(|m| m.clase == Some(id)) {
                if copias(m) != lista {
                    m.efectos.retain(|e| !e.de_clase);
                    m.efectos.splice(0..0, lista.iter().cloned());
                }
            }
        }
        // Las que dejaron su clase se quedan solo con lo suyo.
        let clases: Vec<u64> = self.clases.iter().map(|c| c.id).collect();
        for m in &mut self.mezcla {
            if m.clase.is_some_and(|c| !clases.contains(&c)) {
                m.clase = None;
            }
            if m.clase.is_none() {
                m.efectos.retain(|e| !e.de_clase);
            }
        }
        let grupos: Vec<u64> = self.grupos.iter().map(|g| g.id).collect();
        for m in &mut self.mezcla {
            if m.grupo.is_some_and(|g| !grupos.contains(&g)) {
                m.grupo = None;
            }
        }
    }
}
