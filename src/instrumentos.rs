// Beethoven Craft. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Instrumentos: un sintetizador de tres osciladores, una batería y muestras.
use crate::medios::Audio;
use crate::modelo::{Instrumento, SR};
use crate::sonido::{Sinte, Timbre};
use std::sync::{Arc, OnceLock};

/// Piezas de la batería: tecla, nombre.
pub const PIEZAS: [(u8, &str); 8] = [
    (36, "Bombo"),
    (38, "Tarola"),
    (39, "Palmada"),
    (42, "Contratiempo cerrado"),
    (46, "Contratiempo abierto"),
    (45, "Tom grave"),
    (50, "Tom agudo"),
    (49, "Platillo"),
];

pub struct ParamTrio {
    pub nombre: &'static str,
    pub min: f32,
    pub max: f32,
    pub def: f32,
    pub unidad: &'static str,
    pub log: bool,
}

const fn p(nombre: &'static str, min: f32, max: f32, def: f32, unidad: &'static str, log: bool) -> ParamTrio {
    ParamTrio { nombre, min, max, def, unidad, log }
}

pub const ONDAS: [&str; 6] = ["Senoidal", "Triangular", "Cuadrada", "Sierra", "Sierra suave", "Ruido"];

/// Los tres primeros son la onda de cada oscilador (índice en `ONDAS`).
pub const TRIO: [ParamTrio; 14] = [
    p("Onda 1", 0.0, 5.0, 3.0, "", false),
    p("Onda 2", 0.0, 5.0, 3.0, "", false),
    p("Onda 3", 0.0, 5.0, 2.0, "", false),
    p("Afinación 2", -24.0, 24.0, 0.1, "st", false),
    p("Afinación 3", -24.0, 24.0, -12.0, "st", false),
    p("Mezcla 2", 0.0, 100.0, 50.0, "%", false),
    p("Mezcla 3", 0.0, 100.0, 25.0, "%", false),
    p("Apertura", 0.0, 100.0, 30.0, "%", false),
    p("Ataque", 1.0, 3000.0, 4.0, "ms", true),
    p("Caída", 10.0, 5000.0, 600.0, "ms", true),
    p("Sostén", 0.0, 100.0, 60.0, "%", false),
    p("Salida", 10.0, 5000.0, 220.0, "ms", true),
    p("Corte", 0.0, 100.0, 70.0, "%", false),
    p("Resonancia", 0.0, 100.0, 10.0, "%", false),
];

pub const SONIDOS: [(&str, [f32; 14]); 32] = [
    ("Brillante", [3.0, 3.0, 2.0, 0.1, -12.0, 50.0, 25.0, 30.0, 4.0, 600.0, 60.0, 220.0, 70.0, 10.0]),
    ("Piano suave", [1.0, 0.0, 0.0, 12.0, 0.0, 25.0, 20.0, 20.0, 1.0, 1400.0, 0.0, 250.0, 80.0, 0.0]),
    ("Colchón", [3.0, 3.0, 0.0, 0.12, -12.0, 50.0, 30.0, 80.0, 500.0, 2000.0, 80.0, 1200.0, 50.0, 10.0]),
    ("Bajo", [3.0, 2.0, 0.0, 0.05, -12.0, 40.0, 60.0, 0.0, 2.0, 350.0, 40.0, 90.0, 42.0, 25.0]),
    ("Órgano", [0.0, 0.0, 0.0, 12.0, 19.0, 45.0, 30.0, 10.0, 8.0, 100.0, 100.0, 60.0, 100.0, 0.0]),
    ("Campana", [0.0, 0.0, 0.0, 19.0, 24.0, 40.0, 30.0, 40.0, 1.0, 2200.0, 0.0, 900.0, 100.0, 0.0]),
    ("Solista", [2.0, 3.0, 3.0, 0.08, 12.0, 50.0, 15.0, 20.0, 10.0, 500.0, 75.0, 150.0, 65.0, 30.0]),
    ("Piano eléctrico", [0.0, 0.0, 1.0, 0.04, 24.0, 45.0, 12.0, 30.0, 1.0, 1700.0, 8.0, 260.0, 72.0, 0.0]),
    ("Piano de juguete", [1.0, 0.0, 0.0, 24.0, 31.0, 30.0, 18.0, 10.0, 1.0, 500.0, 0.0, 180.0, 100.0, 0.0]),
    ("Clavecín", [3.0, 2.0, 3.0, 12.0, 0.04, 30.0, 35.0, 25.0, 1.0, 900.0, 0.0, 120.0, 92.0, 5.0]),
    ("Órgano de iglesia", [0.0, 1.0, 0.0, 12.0, 24.0, 55.0, 35.0, 40.0, 60.0, 200.0, 100.0, 500.0, 100.0, 0.0]),
    ("Acordeón", [2.0, 3.0, 2.0, 0.15, 12.0, 60.0, 30.0, 45.0, 40.0, 300.0, 90.0, 120.0, 72.0, 5.0]),
    ("Guitarra limpia", [1.0, 3.0, 0.0, 0.03, 12.0, 25.0, 20.0, 25.0, 1.0, 1100.0, 0.0, 160.0, 62.0, 8.0]),
    ("Guitarra apagada", [3.0, 1.0, 2.0, 0.02, -12.0, 30.0, 20.0, 10.0, 1.0, 180.0, 0.0, 70.0, 48.0, 12.0]),
    ("Arpa", [1.0, 0.0, 0.0, 12.0, 19.0, 30.0, 10.0, 60.0, 1.0, 1800.0, 0.0, 900.0, 78.0, 0.0]),
    ("Marimba", [0.0, 0.0, 0.0, 24.0, 0.0, 22.0, 0.0, 20.0, 1.0, 420.0, 0.0, 200.0, 100.0, 0.0]),
    ("Vibráfono", [0.0, 0.0, 0.0, 24.0, 0.05, 25.0, 50.0, 55.0, 2.0, 2600.0, 0.0, 1200.0, 100.0, 0.0]),
    ("Caja de música", [0.0, 0.0, 1.0, 24.0, 36.0, 45.0, 12.0, 35.0, 1.0, 900.0, 0.0, 700.0, 100.0, 0.0]),
    ("Cuerdas", [3.0, 3.0, 3.0, 0.1, -0.1, 60.0, 60.0, 90.0, 220.0, 800.0, 85.0, 500.0, 62.0, 0.0]),
    ("Cuerdas cortas", [3.0, 3.0, 3.0, 0.08, 12.0, 60.0, 25.0, 70.0, 12.0, 260.0, 10.0, 200.0, 66.0, 0.0]),
    ("Metales", [3.0, 3.0, 3.0, 0.06, -12.0, 60.0, 40.0, 50.0, 50.0, 350.0, 75.0, 160.0, 60.0, 14.0]),
    ("Flauta", [1.0, 0.0, 5.0, 12.0, 0.0, 12.0, 2.5, 10.0, 70.0, 300.0, 85.0, 140.0, 70.0, 0.0]),
    ("Silbido", [0.0, 0.0, 5.0, 12.0, 0.0, 5.0, 2.0, 10.0, 50.0, 200.0, 95.0, 120.0, 100.0, 0.0]),
    ("Coro", [1.0, 4.0, 0.0, 0.12, -0.12, 60.0, 60.0, 100.0, 300.0, 900.0, 85.0, 900.0, 45.0, 22.0]),
    ("Bajo profundo", [0.0, 1.0, 0.0, 0.0, -12.0, 30.0, 70.0, 0.0, 3.0, 600.0, 70.0, 120.0, 55.0, 0.0]),
    ("Bajo con cuerpo", [3.0, 3.0, 2.0, 0.08, -12.0, 60.0, 60.0, 20.0, 2.0, 500.0, 55.0, 100.0, 48.0, 35.0]),
    ("Bajo picado", [2.0, 3.0, 0.0, 0.02, -12.0, 35.0, 45.0, 0.0, 1.0, 160.0, 0.0, 60.0, 58.0, 30.0]),
    ("Punteo", [3.0, 2.0, 3.0, 12.0, 0.06, 30.0, 50.0, 45.0, 1.0, 220.0, 0.0, 180.0, 74.0, 20.0]),
    ("Super sierra", [3.0, 3.0, 3.0, 0.22, -0.22, 85.0, 85.0, 100.0, 4.0, 500.0, 80.0, 250.0, 88.0, 5.0]),
    ("Solista chillón", [3.0, 2.0, 3.0, 0.12, 12.0, 60.0, 35.0, 30.0, 3.0, 400.0, 85.0, 120.0, 82.0, 55.0]),
    ("Ocho bits", [2.0, 2.0, 2.0, 12.0, 0.0, 15.0, 0.0, 0.0, 1.0, 200.0, 80.0, 30.0, 100.0, 0.0]),
    ("Viento", [5.0, 5.0, 0.0, 0.0, 0.0, 40.0, 0.0, 100.0, 900.0, 2000.0, 70.0, 1800.0, 42.0, 45.0]),
];

/// Cómo se agrupan los sonidos en los menús.
pub const FAMILIAS: [(&str, &[&str]); 7] = [
    ("Pianos y teclados", &["Piano suave", "Piano eléctrico", "Piano de juguete", "Clavecín", "Órgano", "Órgano de iglesia", "Acordeón"]),
    ("Guitarras y arpa", &["Guitarra limpia", "Guitarra apagada", "Arpa", "Punteo"]),
    ("Campanas y percusión con tono", &["Campana", "Marimba", "Vibráfono", "Caja de música"]),
    ("Cuerdas, metales y voces", &["Cuerdas", "Cuerdas cortas", "Metales", "Coro"]),
    ("Vientos", &["Flauta", "Silbido", "Viento"]),
    ("Bajos", &["Bajo", "Bajo profundo", "Bajo con cuerpo", "Bajo picado"]),
    ("Sintetizadores", &["Brillante", "Colchón", "Solista", "Solista chillón", "Super sierra", "Ocho bits"]),
];

/// Nombre del sonido de un instrumento.
pub fn nombre_sonido(i: &Instrumento) -> String {
    match i {
        Instrumento::Trio(v) => SONIDOS.iter().find(|s| s.1[..] == v[..]).map_or("Sonido a mi gusto", |s| s.0).into(),
        Instrumento::Bateria => "Batería".into(),
        Instrumento::Muestra { ruta, .. } => ruta.file_stem().map_or("Muestra".into(), |s| s.to_string_lossy().into_owned()),
    }
}

fn timbre(v: &[f32]) -> Timbre {
    Timbre {
        onda: std::array::from_fn(|k| v[k].round().clamp(0.0, 5.0) as u8),
        semitonos: [0.0, v[3], v[4]],
        mezcla: [v[5] / 100.0, v[6] / 100.0],
        abierto: v[7] / 100.0,
        adsr: [v[8] / 1000.0, v[9] / 1000.0, v[10] / 100.0, v[11] / 1000.0],
        corte: (v[12] < 99.5).then(|| 40.0 * 400f32.powf(v[12] / 100.0)),
        filo: v[13] / 100.0,
    }
}

/// Sonidos de batería hechos al vuelo una vez.
fn bateria() -> &'static [Vec<f32>; 8] {
    static B: OnceLock<[Vec<f32>; 8]> = OnceLock::new();
    B.get_or_init(|| {
        let sr = SR as f32;
        let mut s = 0x1234_5678u32;
        let mut ruido = move || {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            s as f32 / u32::MAX as f32 * 2.0 - 1.0
        };
        let tau = std::f32::consts::TAU;
        // Tono que baja de `a` a `b` Hz en `t` segundos, con caída `d`.
        let tono = |dur: f32, a: f32, b: f32, t: f32, d: f32| -> Vec<f32> {
            let mut f = 0.0f32;
            (0..(dur * sr) as usize)
                .map(|i| {
                    let x = i as f32 / sr;
                    f += (b + (a - b) * (-x / t).exp()) / sr;
                    (f * tau).sin() * (-x / d).exp()
                })
                .collect()
        };
        // Ruido sin graves (diferencias), con caída `d`.
        let mut silbido = |dur: f32, d: f32, orden: usize| -> Vec<f32> {
            let mut ant = [0.0f32; 3];
            (0..(dur * sr) as usize)
                .map(|i| {
                    let mut y = ruido();
                    for a in ant.iter_mut().take(orden) {
                        let z = y - *a;
                        *a = y;
                        y = z * 0.6;
                    }
                    y * (-(i as f32 / sr) / d).exp()
                })
                .collect()
        };
        let mut bombo = tono(0.5, 190.0, 48.0, 0.035, 0.16);
        bombo.iter_mut().zip(silbido(0.01, 0.002, 1)).for_each(|(a, b)| *a = (*a * 1.5 + b * 0.4).tanh());
        let mut tarola = silbido(0.3, 0.075, 1);
        tarola.iter_mut().zip(tono(0.3, 260.0, 180.0, 0.02, 0.06)).for_each(|(a, b)| *a = *a * 0.8 + b * 0.6);
        let base = silbido(0.35, 0.09, 1);
        let palmada: Vec<f32> = base
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let t = i as f32 / sr;
                let e = if t < 0.03 { (-(t % 0.01) / 0.004).exp() } else { 1.0 };
                x * e
            })
            .collect();
        [
            bombo,
            tarola,
            palmada,
            silbido(0.08, 0.022, 2),
            silbido(0.5, 0.16, 2),
            tono(0.5, 150.0, 95.0, 0.05, 0.18),
            tono(0.4, 260.0, 170.0, 0.04, 0.14),
            silbido(1.6, 0.5, 2),
        ]
    })
}

struct VozMuestra {
    tecla: u8,
    pos: f64,
    paso: f64,
    g: f32,
    /// Cuadros que faltan del apagado, si ya se soltó.
    suelta: Option<f32>,
}

/// Un instrumento sonando.
pub struct InstrRT {
    tipo: Instrumento,
    sinte: Sinte,
    timbre: Timbre,
    muestra: Option<Arc<Audio>>,
    voces: Vec<VozMuestra>,
}

const APAGADO: f32 = 1200.0;

impl InstrRT {
    pub fn nuevo(tipo: &Instrumento, muestra: Option<Arc<Audio>>) -> Self {
        let _ = bateria();
        let mut i = InstrRT {
            tipo: tipo.clone(),
            sinte: Sinte::nuevo(),
            timbre: Timbre::default(),
            muestra,
            voces: Vec::with_capacity(64),
        };
        i.ajustar(tipo, None);
        i
    }

    pub fn ajustar(&mut self, tipo: &Instrumento, muestra: Option<Arc<Audio>>) {
        if let Instrumento::Trio(v) = tipo {
            if v.len() == TRIO.len() {
                self.timbre = timbre(v);
            }
        }
        if std::mem::discriminant(tipo) != std::mem::discriminant(&self.tipo) {
            self.callar();
        }
        if muestra.is_some() {
            self.muestra = muestra;
        }
        self.tipo = tipo.clone();
    }

    pub fn callar(&mut self) {
        self.sinte.callar();
        self.voces.clear();
    }

    pub fn nota(&mut self, tecla: u8, vel: u8, on: bool) {
        match &self.tipo {
            Instrumento::Trio(_) => {
                if on {
                    self.sinte.tocar(tecla, vel as f32 / 128.0);
                } else {
                    self.sinte.soltar(tecla);
                }
            }
            Instrumento::Bateria => {
                if on {
                    // El contratiempo cerrado calla al abierto.
                    if tecla == 42 {
                        self.voces.iter_mut().filter(|v| v.tecla == 46).for_each(|v| v.suelta = v.suelta.or(Some(APAGADO)));
                    }
                    self.voz(tecla, vel, 1.0);
                }
            }
            Instrumento::Muestra { raiz, .. } => {
                if on {
                    let paso = 2f64.powf((tecla as f64 - *raiz as f64) / 12.0);
                    self.voz(tecla, vel, paso);
                } else {
                    self.voces.iter_mut().filter(|v| v.tecla == tecla && v.suelta.is_none()).for_each(|v| v.suelta = Some(APAGADO));
                }
            }
        }
    }

    fn voz(&mut self, tecla: u8, vel: u8, paso: f64) {
        if self.voces.len() >= 64 {
            self.voces.remove(0);
        }
        let g = (vel as f32 / 127.0).powf(1.5);
        self.voces.push(VozMuestra { tecla, pos: 0.0, paso, g, suelta: None });
    }

    /// Suma el instrumento en `out` (estéreo intercalado).
    pub fn sonar(&mut self, out: &mut [f32]) {
        match &self.tipo {
            Instrumento::Trio(_) => self.sinte.sonar(out, &self.timbre, 0.35),
            _ => {
                let bat = matches!(self.tipo, Instrumento::Bateria);
                let muestra = self.muestra.clone();
                self.voces.retain_mut(|v| {
                    let mono = bat.then(|| PIEZAS.iter().position(|p| p.0 == v.tecla).map(|i| &bateria()[i])).flatten();
                    let largo = match (&mono, &muestra) {
                        (Some(m), _) => m.len(),
                        (None, Some(a)) if !bat => a.cuadros.len(),
                        _ => return false,
                    };
                    for f in out.chunks_exact_mut(2) {
                        let i = v.pos as usize;
                        if i + 1 >= largo {
                            return false;
                        }
                        let k = (v.pos - i as f64) as f32;
                        let x = match (&mono, &muestra) {
                            (Some(m), _) => [m[i] + (m[i + 1] - m[i]) * k; 2],
                            (None, Some(a)) => std::array::from_fn(|c| a.cuadros[i][c] + (a.cuadros[i + 1][c] - a.cuadros[i][c]) * k),
                            _ => [0.0; 2],
                        };
                        let mut g = v.g;
                        if let Some(s) = &mut v.suelta {
                            *s -= 1.0;
                            if *s <= 0.0 {
                                return false;
                            }
                            g *= *s / APAGADO;
                        }
                        f[0] += x[0] * g * 0.8;
                        f[1] += x[1] * g * 0.8;
                        v.pos += v.paso;
                    }
                    true
                });
            }
        }
    }
}
