// Beethoven Craft. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! El sonido de los efectos y del sintetizador: filtros, dinámica, espacio, modulación, distorsión y osciladores.
//! Escrito para Beethoven Craft a partir de las técnicas conocidas de audio digital (las fórmulas de filtros son de R.
//! Bristow-Johnson y de A. Simper; los cruces, de Linkwitz y Riley; la reverberación, una red de retardos como la de J.-M. Jot).
use crate::modelo::{SR, db};
use std::collections::VecDeque;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

const S: f32 = SR as f32;

/// Cuánto se acerca por muestra algo que llega a su meta en `ms`.
fn paso_de(ms: f32) -> f32 {
    1.0 - (-1.0 / (ms.max(0.02) * 0.001 * S)).exp()
}

fn a_db(x: f32) -> f32 {
    20.0 * x.max(1e-9).log10()
}

/// Un azar barato y repetible.
#[derive(Clone)]
pub struct Azar(u32);

impl Azar {
    pub fn nuevo(s: u32) -> Azar {
        Azar(s | 1)
    }
    /// De -1 a 1.
    pub fn dar(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

// ---- Bandas de ecualizador

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Banda {
    Apagada,
    PasaGraves,
    PasaBanda,
    PasaAgudos,
    QuitaBanda,
    MesetaGrave,
    Campana,
    MesetaAguda,
}

/// Filtro de segundo orden, estéreo, en doble precisión.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bq {
    b: [f64; 3],
    a: [f64; 2],
    z: [[f64; 2]; 2],
}

impl Bq {
    pub fn poner(&mut self, tipo: Banda, hz: f32, db: f32, q: f32) {
        let w = std::f64::consts::TAU * (hz as f64).clamp(10.0, SR as f64 * 0.49) / SR as f64;
        let (s, c) = w.sin_cos();
        let al = s / (2.0 * q.max(0.05) as f64);
        let g = 10f64.powf(db as f64 / 40.0);
        let r = 2.0 * g.sqrt() * al;
        let (b, a) = match tipo {
            Banda::Apagada => ([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
            Banda::PasaGraves => ([(1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0], [1.0 + al, -2.0 * c, 1.0 - al]),
            Banda::PasaAgudos => ([(1.0 + c) / 2.0, -1.0 - c, (1.0 + c) / 2.0], [1.0 + al, -2.0 * c, 1.0 - al]),
            Banda::PasaBanda => ([al, 0.0, -al], [1.0 + al, -2.0 * c, 1.0 - al]),
            Banda::QuitaBanda => ([1.0, -2.0 * c, 1.0], [1.0 + al, -2.0 * c, 1.0 - al]),
            Banda::Campana => ([1.0 + al * g, -2.0 * c, 1.0 - al * g], [1.0 + al / g, -2.0 * c, 1.0 - al / g]),
            Banda::MesetaGrave => (
                [g * ((g + 1.0) - (g - 1.0) * c + r), 2.0 * g * ((g - 1.0) - (g + 1.0) * c), g * ((g + 1.0) - (g - 1.0) * c - r)],
                [(g + 1.0) + (g - 1.0) * c + r, -2.0 * ((g - 1.0) + (g + 1.0) * c), (g + 1.0) + (g - 1.0) * c - r],
            ),
            Banda::MesetaAguda => (
                [g * ((g + 1.0) + (g - 1.0) * c + r), -2.0 * g * ((g - 1.0) + (g + 1.0) * c), g * ((g + 1.0) + (g - 1.0) * c - r)],
                [(g + 1.0) - (g - 1.0) * c + r, 2.0 * ((g - 1.0) - (g + 1.0) * c), (g + 1.0) - (g - 1.0) * c - r],
            ),
        };
        self.b = b.map(|x| x / a[0]);
        self.a = [a[1] / a[0], a[2] / a[0]];
    }
    #[inline]
    pub fn paso(&mut self, k: usize, x: f32) -> f32 {
        let (x, z) = (x as f64, &mut self.z[k]);
        let y = self.b[0] * x + z[0];
        z[0] = self.b[1] * x - self.a[0] * y + z[1];
        z[1] = self.b[2] * x - self.a[1] * y;
        y as f32
    }
    pub fn limpiar(&mut self) {
        self.z = Default::default();
    }
    /// Cuánto sube o baja esa frecuencia, en dB.
    pub fn db_en(&self, hz: f64) -> f64 {
        let w = std::f64::consts::TAU * hz / SR as f64;
        let m = |p: [f64; 3]| {
            let re = p[0] + p[1] * w.cos() + p[2] * (2.0 * w).cos();
            let im = p[1] * w.sin() + p[2] * (2.0 * w).sin();
            re * re + im * im
        };
        10.0 * (m(self.b).max(1e-30) / m([1.0, self.a[0], self.a[1]]).max(1e-30)).log10()
    }
}

/// Varias bandas en fila y una ganancia.
#[derive(Clone, Debug)]
pub struct Bandas {
    v: Vec<Bq>,
    activa: Vec<bool>,
    salida: f32,
}

impl Bandas {
    pub fn nuevas(n: usize) -> Bandas {
        Bandas { v: vec![Bq::default(); n], activa: vec![false; n], salida: 1.0 }
    }
    pub fn poner(&mut self, k: usize, tipo: Banda, hz: f32, db: f32, q: f32) {
        let con_nivel = matches!(tipo, Banda::MesetaGrave | Banda::Campana | Banda::MesetaAguda);
        let activa = tipo != Banda::Apagada && !(con_nivel && db.abs() < 0.01);
        if !activa {
            self.v[k].limpiar();
        }
        self.activa[k] = activa;
        self.v[k].poner(tipo, hz, db, q);
    }
    pub fn salida(&mut self, db_: f32) {
        self.salida = db(db_);
    }
    pub fn procesar(&mut self, buf: &mut [f32]) {
        for (q, si) in self.v.iter_mut().zip(&self.activa) {
            if *si {
                for f in buf.chunks_exact_mut(2) {
                    f[0] = q.paso(0, f[0]);
                    f[1] = q.paso(1, f[1]);
                }
            }
        }
        buf.iter_mut().for_each(|x| *x *= self.salida);
    }
    pub fn limpiar(&mut self) {
        self.v.iter_mut().for_each(Bq::limpiar);
    }
    pub fn curva(&self, hz: impl Iterator<Item = f64>) -> Vec<f64> {
        let g = a_db(self.salida) as f64;
        hz.map(|h| g + self.v.iter().zip(&self.activa).filter(|x| *x.1).map(|x| x.0.db_en(h)).sum::<f64>()).collect()
    }
}

// ---- Dinámica

/// Compresor con rodilla suave: mide la fuerza promedio y baja lo que pasa del umbral.
pub struct Compresor {
    pub umbral: f32,
    pub razon: f32,
    pub ataque: f32,
    pub suelta: f32,
    pub ganancia: f32,
    media: f32,
    baja: f32,
}

impl Compresor {
    pub fn nuevo() -> Compresor {
        Compresor { umbral: 0.0, razon: 1.0, ataque: 8.0, suelta: 120.0, ganancia: 1.0, media: 0.0, baja: 0.0 }
    }
    pub fn procesar(&mut self, buf: &mut [f32]) {
        const RODILLA: f32 = 4.0;
        let (km, ka, ks) = (paso_de(5.0), paso_de(self.ataque), paso_de(self.suelta));
        let k = 1.0 - 1.0 / self.razon.max(1.0);
        for f in buf.chunks_exact_mut(2) {
            let e = f[0].mul_add(f[0], f[1] * f[1]) * 0.5;
            self.media += (e - self.media) * km;
            let pasa = a_db(self.media.sqrt()) - self.umbral;
            let quiere = if pasa <= -RODILLA {
                0.0
            } else if pasa < RODILLA {
                k * (pasa + RODILLA).powi(2) / (4.0 * RODILLA)
            } else {
                k * pasa
            };
            self.baja += (quiere - self.baja) * if quiere > self.baja { ka } else { ks };
            let g = db(-self.baja) * self.ganancia;
            f[0] *= g;
            f[1] *= g;
        }
        if self.media < 1e-20 {
            self.media = 0.0;
        }
    }
    pub fn limpiar(&mut self) {
        (self.media, self.baja) = (0.0, 0.0);
    }
}

/// Compuerta: cierra cuando baja del umbral, con espera para no cortar las colas.
pub struct Compuerta {
    pub umbral: f32,
    pub cerrada: f32,
    pub suelta: f32,
    pico: f32,
    g: f32,
    espera: u32,
    abierta: bool,
}

impl Compuerta {
    pub fn nueva() -> Compuerta {
        Compuerta { umbral: 0.01, cerrada: 0.001, suelta: 80.0, pico: 0.0, g: 0.0, espera: 0, abierta: false }
    }
    pub fn procesar(&mut self, buf: &mut [f32]) {
        let (kp, ka, ks) = (paso_de(12.0), paso_de(0.6), paso_de(self.suelta));
        // Se cierra un poco más abajo de donde se abre: así no tiembla.
        let cierra = self.umbral * db(-3.0);
        for f in buf.chunks_exact_mut(2) {
            let x = f[0].abs().max(f[1].abs());
            self.pico = if x > self.pico { x } else { self.pico - self.pico * kp };
            if self.pico > self.umbral {
                self.abierta = true;
                self.espera = (S * 0.02) as u32;
            } else if self.pico < cierra {
                self.espera = self.espera.saturating_sub(1);
                self.abierta &= self.espera > 0;
            }
            let quiere = if self.abierta { 1.0 } else { self.cerrada };
            self.g += (quiere - self.g) * if quiere > self.g { ka } else { ks };
            f[0] *= self.g;
            f[1] *= self.g;
        }
    }
    pub fn limpiar(&mut self) {
        (self.pico, self.g, self.espera, self.abierta) = (0.0, self.cerrada, 0, false);
    }
}

/// Limitador que ve 2 ms adelante: el techo no se pasa y el sonido no se quiebra.
pub struct Limitador {
    pub entrada: f32,
    pub techo: f32,
    pub suelta: f32,
    cola: Vec<[f32; 2]>,
    i: usize,
    /// Los mínimos de la ventana, en orden: (cuándo, cuánto).
    min: VecDeque<(u64, f32)>,
    n: u64,
    r: f32,
    prom: Vec<f32>,
    suma: f64,
}

impl Limitador {
    const L: usize = 96;
    pub fn nuevo() -> Limitador {
        Limitador {
            entrada: 1.0,
            techo: 1.0,
            suelta: 60.0,
            cola: vec![[0.0; 2]; Self::L],
            i: 0,
            min: VecDeque::with_capacity(Self::L + 2),
            n: 0,
            r: 1.0,
            prom: vec![1.0; Self::L],
            suma: Self::L as f64,
        }
    }
    pub const fn latencia() -> usize {
        Self::L - 1
    }
    pub fn procesar(&mut self, buf: &mut [f32]) {
        let ks = paso_de(self.suelta);
        for f in buf.chunks_exact_mut(2) {
            let x = [f[0] * self.entrada, f[1] * self.entrada];
            let pico = x[0].abs().max(x[1].abs());
            let g = if pico > self.techo { self.techo / pico } else { 1.0 };
            while self.min.back().is_some_and(|m| m.1 >= g) {
                self.min.pop_back();
            }
            self.min.push_back((self.n, g));
            while self.min.front().is_some_and(|m| m.0 + Self::L as u64 <= self.n) {
                self.min.pop_front();
            }
            self.n += 1;
            let m = self.min.front().map_or(1.0, |m| m.1);
            self.r = if m < self.r { m } else { self.r + (m - self.r) * ks };
            // El promedio hace la rampa: llega a su lugar justo cuando sale el pico.
            self.suma += (self.r - self.prom[self.i]) as f64;
            self.prom[self.i] = self.r;
            self.cola[self.i] = x;
            self.i = (self.i + 1) % Self::L;
            let y = self.cola[self.i];
            let g = (self.suma / Self::L as f64) as f32;
            f[0] = (y[0] * g).clamp(-self.techo, self.techo);
            f[1] = (y[1] * g).clamp(-self.techo, self.techo);
        }
    }
    pub fn limpiar(&mut self) {
        self.cola.fill([0.0; 2]);
        self.prom.fill(1.0);
        (self.i, self.n, self.r, self.suma) = (0, 0, 1.0, Self::L as f64);
        self.min.clear();
    }
}

/// Cruce de Linkwitz-Riley: los dos lados sumados quedan parejos.
#[derive(Clone, Copy, Default)]
struct Cruce {
    bajo: [Bq; 2],
    alto: [Bq; 2],
}

impl Cruce {
    fn poner(&mut self, hz: f32) {
        let q = std::f32::consts::FRAC_1_SQRT_2;
        self.bajo.iter_mut().for_each(|b| b.poner(Banda::PasaGraves, hz, 0.0, q));
        self.alto.iter_mut().for_each(|b| b.poner(Banda::PasaAgudos, hz, 0.0, q));
    }
    #[inline]
    fn partir(&mut self, k: usize, x: f32) -> (f32, f32) {
        let b = self.bajo[0].paso(k, x);
        let a = self.alto[0].paso(k, x);
        (self.bajo[1].paso(k, b), self.alto[1].paso(k, a))
    }
    fn limpiar(&mut self) {
        self.bajo.iter_mut().chain(&mut self.alto).for_each(Bq::limpiar);
    }
}

/// Tres bandas comprimidas por separado y un limitador al final.
pub struct Multibanda {
    abajo: Cruce,
    arriba: Cruce,
    /// La banda grave pasa por el cruce de arriba para llegar a tiempo con las demás.
    empareja: Cruce,
    bandas: [Compresor; 3],
    trozo: [Vec<f32>; 3],
    fin: Limitador,
    salida: f32,
}

impl Multibanda {
    pub fn nueva() -> Multibanda {
        Multibanda {
            abajo: Cruce::default(),
            arriba: Cruce::default(),
            empareja: Cruce::default(),
            bandas: std::array::from_fn(|_| Compresor::nuevo()),
            trozo: Default::default(),
            fin: Limitador::nuevo(),
            salida: 1.0,
        }
    }
    pub fn ajustar(&mut self, cruces: [f32; 2], empuje: [f32; 4], suelta: f32, salida: f32) {
        self.abajo.poner(cruces[0]);
        self.arriba.poner(cruces[1]);
        self.empareja.poner(cruces[1]);
        for (c, e) in self.bandas.iter_mut().zip(empuje) {
            // Empujar sube lo quedo; lo fuerte se topa con el compresor.
            (c.umbral, c.razon, c.ataque, c.suelta, c.ganancia) = (-9.0 - e, 2.5, 6.0, suelta, db(e));
        }
        (self.fin.entrada, self.fin.techo, self.fin.suelta) = (db(empuje[3]), 1.0, suelta.min(200.0));
        self.salida = db(salida);
    }
    pub fn procesar(&mut self, buf: &mut [f32]) {
        for t in &mut self.trozo {
            t.clear();
            t.resize(buf.len(), 0.0);
        }
        for (i, x) in buf.iter().enumerate() {
            let k = i & 1;
            let (g, resto) = self.abajo.partir(k, *x);
            let (m, a) = self.arriba.partir(k, resto);
            let (gb, ga) = self.empareja.partir(k, g);
            (self.trozo[0][i], self.trozo[1][i], self.trozo[2][i]) = (gb + ga, m, a);
        }
        for (c, t) in self.bandas.iter_mut().zip(&mut self.trozo) {
            c.procesar(t);
        }
        for (i, x) in buf.iter_mut().enumerate() {
            *x = self.trozo[0][i] + self.trozo[1][i] + self.trozo[2][i];
        }
        self.fin.procesar(buf);
        buf.iter_mut().for_each(|x| *x *= self.salida);
    }
    pub fn limpiar(&mut self) {
        for c in [&mut self.abajo, &mut self.arriba, &mut self.empareja] {
            c.limpiar();
        }
        self.bandas.iter_mut().for_each(Compresor::limpiar);
        self.fin.limpiar();
    }
}

/// Separa el golpe de la cola comparando un seguidor rápido con uno lento.
#[derive(Default)]
pub struct Transitorios {
    pub golpe: f32,
    pub cola: f32,
    pub salida: f32,
    rapido: f32,
    lento: f32,
    corta: f32,
    larga: f32,
    g: f32,
}

impl Transitorios {
    pub fn procesar(&mut self, buf: &mut [f32]) {
        let sigue = |e: &mut f32, x: f32, sube: f32, baja: f32| *e += (x - *e) * if x > *e { sube } else { baja };
        let (k1, k25, k20, k250, k40, kg) = (paso_de(0.6), paso_de(25.0), paso_de(20.0), paso_de(250.0), paso_de(40.0), paso_de(1.0));
        for f in buf.chunks_exact_mut(2) {
            let x = f[0].abs().max(f[1].abs());
            sigue(&mut self.rapido, x, k1, k40);
            sigue(&mut self.lento, x, k25, k40);
            sigue(&mut self.corta, x, k1, k20);
            sigue(&mut self.larga, x, k1, k250);
            let piso = 1e-5;
            let golpe = a_db((self.rapido + piso) / (self.lento + piso)).clamp(0.0, 12.0);
            let cola = a_db((self.larga + piso) / (self.corta + piso)).clamp(0.0, 12.0);
            let quiere = (golpe * self.golpe + cola * self.cola) * 0.45;
            self.g += (quiere - self.g) * kg;
            let g = db(self.g) * self.salida;
            f[0] *= g;
            f[1] *= g;
        }
    }
    pub fn limpiar(&mut self) {
        (self.rapido, self.lento, self.corta, self.larga, self.g) = (0.0, 0.0, 0.0, 0.0, 0.0);
    }
}

// ---- Retardos

/// Línea de retardo con lectura entre muestras.
#[derive(Clone)]
struct Linea {
    v: Vec<f32>,
    i: usize,
}

impl Linea {
    fn nueva(largo: usize) -> Linea {
        Linea { v: vec![0.0; largo.next_power_of_two()], i: 0 }
    }
    #[inline]
    fn meter(&mut self, x: f32) {
        self.v[self.i] = if x.abs() < 1e-18 { 0.0 } else { x };
        self.i = (self.i + 1) & (self.v.len() - 1);
    }
    /// Hace `d` muestras (la última metida es 1).
    #[inline]
    fn leer(&self, d: f32) -> f32 {
        let m = self.v.len() - 1;
        let d = d.clamp(1.0, m as f32 - 1.0);
        let (a, t) = (d as usize, d.fract());
        let x = self.v[(self.i + m + 1 - a) & m];
        let y = self.v[(self.i + m - a) & m];
        x + (y - x) * t
    }
    fn limpiar(&mut self) {
        self.v.fill(0.0);
    }
}

/// Filtro de un polo.
#[derive(Clone, Copy, Default)]
struct Polo(f32);

impl Polo {
    fn k(hz: f32) -> f32 {
        1.0 - (-TAU * hz.min(S * 0.45) / S).exp()
    }
    #[inline]
    fn bajo(&mut self, x: f32, k: f32) -> f32 {
        self.0 += (x - self.0) * k;
        self.0
    }
}

/// Repeticiones, con rebote entre lados si se quiere.
pub struct Eco {
    pub ms: f32,
    pub repite: f32,
    pub mezcla: f32,
    pub hz: f32,
    pub rebote: bool,
    l: [Linea; 2],
    tono: [Polo; 2],
    d: f32,
}

impl Eco {
    pub fn nuevo() -> Eco {
        let largo = (S * 1.1) as usize;
        Eco { ms: 350.0, repite: 0.3, mezcla: 0.3, hz: 5000.0, rebote: false, l: [Linea::nueva(largo), Linea::nueva(largo)], tono: Default::default(), d: 0.0 }
    }
    pub fn procesar(&mut self, buf: &mut [f32]) {
        let (meta, kt, kd) = (self.ms * 0.001 * S, Polo::k(self.hz), paso_de(60.0));
        // Hasta arriba, sin filtro.
        let abierto = self.hz >= 19_500.0;
        if self.d == 0.0 {
            self.d = meta;
        }
        for f in buf.chunks_exact_mut(2) {
            self.d += (meta - self.d) * kd;
            let mut y: [f32; 2] = std::array::from_fn(|k| self.l[k].leer(self.d));
            if !abierto {
                y = std::array::from_fn(|k| self.tono[k].bajo(y[k], kt));
            }
            let x = if self.rebote { [(f[0] + f[1]) * 0.5 + y[1] * self.repite, y[0] * if self.repite > 0.0 { 1.0 } else { 0.0 }] } else { [f[0] + y[0] * self.repite, f[1] + y[1] * self.repite] };
            self.l[0].meter(x[0]);
            self.l[1].meter(x[1]);
            f[0] += y[0] * self.mezcla;
            f[1] += y[1] * self.mezcla;
        }
    }
    pub fn limpiar(&mut self) {
        self.l.iter_mut().for_each(Linea::limpiar);
        self.tono = Default::default();
        self.d = 0.0;
    }
}

/// Reverberación: primeros rebotes, difusión y una red de ocho retardos.
pub struct Sala {
    mezcla: f32,
    previo: f32,
    largos: [f32; 8],
    vueltas: [f32; 8],
    brillo: f32,
    graves: [Bq; 1],
    sin_graves: bool,
    entra: [Linea; 2],
    difusa: [[Linea; 4]; 2],
    red: Vec<Linea>,
    tono: [Polo; 8],
    fase: f32,
}

impl Sala {
    const MS: [f32; 8] = [31.3, 37.9, 41.7, 47.1, 53.3, 59.9, 67.3, 73.7];
    const DIF: [[f32; 4]; 2] = [[4.77, 3.59, 12.73, 9.31], [5.03, 3.83, 12.31, 9.77]];
    pub fn nueva() -> Sala {
        let l = |ms: f32| Linea::nueva((ms * 0.001 * S) as usize + 16);
        Sala {
            mezcla: 0.45,
            previo: 0.0,
            largos: Self::MS,
            vueltas: [0.0; 8],
            brillo: 1.0,
            graves: Default::default(),
            sin_graves: false,
            entra: [l(330.0), l(330.0)],
            difusa: Self::DIF.map(|d| d.map(l)),
            red: Self::MS.iter().map(|m| l(m * 1.5)).collect(),
            tono: Default::default(),
            fase: 0.0,
        }
    }
    pub fn ajustar(&mut self, mezcla: f32, segundos: f32, tam: f32, previo_ms: f32, corta: f32, brillo: f32) {
        self.mezcla = mezcla;
        self.previo = previo_ms * 0.001 * S;
        for k in 0..8 {
            self.largos[k] = Self::MS[k] * (0.4 + 0.9 * tam) * 0.001 * S;
            self.vueltas[k] = 10f32.powf(-3.0 * self.largos[k] / S / segundos.max(0.05));
        }
        self.brillo = Polo::k(brillo);
        self.sin_graves = corta > 20.5;
        self.graves[0].poner(Banda::PasaAgudos, corta, 0.0, 0.7);
    }
    pub fn procesar(&mut self, buf: &mut [f32]) {
        // Los primeros rebotes: cuándo (ms) y cuánto, distintos en cada lado.
        const REB: [[(f32, f32); 5]; 2] = [[(7.1, 0.42), (13.7, 0.33), (21.9, -0.27), (34.3, 0.2), (48.1, -0.15)], [(8.9, 0.4), (15.3, -0.32), (24.7, 0.26), (31.1, -0.2), (51.7, 0.15)]];
        let escala = self.largos[0] / (Self::MS[0] * 0.001 * S);
        // Las colas largas guardan más energía: se compensa para que la mezcla signifique lo mismo.
        let energia = (1.0 - self.vueltas.iter().map(|v| v * v).sum::<f32>() / 8.0).max(0.02).sqrt();
        for f in buf.chunks_exact_mut(2) {
            self.entra[0].meter(f[0]);
            self.entra[1].meter(f[1]);
            let mut y = [0f32; 2];
            let mut x = [0f32; 2];
            for k in 0..2 {
                for (ms, g) in REB[k] {
                    y[k] += self.entra[k].leer(self.previo + ms * escala * 0.001 * S) * g;
                }
                let mut z = self.entra[k].leer(self.previo + 1.0);
                for (j, l) in self.difusa[k].iter_mut().enumerate() {
                    let g = if j < 2 { 0.7 } else { 0.6 };
                    let d = l.leer(Self::DIF[k][j] * 0.001 * S);
                    let e = z - d * g;
                    l.meter(e);
                    z = d + e * g;
                }
                x[k] = z;
            }
            self.fase = (self.fase + 0.7 / S).fract();
            let mut v = [0f32; 8];
            for (k, v) in v.iter_mut().enumerate() {
                // Cuatro se mueven un poquito: sin eso la cola suena a metal.
                let mueve = if k % 2 == 0 { ((self.fase + k as f32 / 8.0) * TAU).sin() * 5.0 + 5.0 } else { 0.0 };
                let s = self.red[k].leer(self.largos[k] + mueve);
                *v = self.tono[k].bajo(s, self.brillo) * self.vueltas[k];
            }
            let comun = v.iter().sum::<f32>() * 0.25;
            for k in 0..8 {
                self.red[k].meter(v[k] - comun + x[k & 1] * if k & 2 == 0 { 0.5 } else { -0.5 });
            }
            for k in 0..2 {
                let cola = (v[k] + v[k + 2] - v[k + 4] - v[k + 6]) * 0.75 * energia;
                let mut w = y[k] * 0.5 + cola;
                if self.sin_graves {
                    w = self.graves[0].paso(k, w);
                }
                f[k] += w * self.mezcla;
            }
        }
    }
    pub fn limpiar(&mut self) {
        self.entra.iter_mut().chain(self.difusa.iter_mut().flatten()).chain(&mut self.red).for_each(Linea::limpiar);
        self.tono = Default::default();
        self.graves[0].limpiar();
    }
}

// ---- Modulación

/// Tres copias del sonido que se atrasan y adelantan; da solo las copias.
pub struct Coro {
    pub hz: f32,
    pub hondo: f32,
    pub ms: f32,
    pub abierto: f32,
    l: [Linea; 2],
    fase: [f32; 3],
}

impl Coro {
    pub fn nuevo() -> Coro {
        let n = (S * 0.05) as usize;
        Coro { hz: 0.6, hondo: 0.5, ms: 12.0, abierto: 0.6, l: [Linea::nueva(n), Linea::nueva(n)], fase: [0.0, 0.33, 0.66] }
    }
    /// `buf` entra seco; sale `seco * a + copias * b`.
    pub fn procesar(&mut self, buf: &mut [f32], a: f32, b: f32) {
        let hondo = self.hondo * 5.0;
        for f in buf.chunks_exact_mut(2) {
            self.l[0].meter(f[0]);
            self.l[1].meter(f[1]);
            let mut y = [0f32; 2];
            for (v, r) in [1.0, 0.73, 1.31].into_iter().enumerate() {
                self.fase[v] = (self.fase[v] + self.hz * r / S).fract();
                for (k, y) in y.iter_mut().enumerate() {
                    let o = ((self.fase[v] + k as f32 * self.abierto * 0.25) * TAU).sin();
                    *y += self.l[k].leer((self.ms + hondo * (1.0 + o) * 0.5 + 0.1) * 0.001 * S);
                }
            }
            f[0] = f[0] * a + y[0] * 0.58 * b;
            f[1] = f[1] * a + y[1] * 0.58 * b;
        }
    }
    pub fn limpiar(&mut self) {
        self.l.iter_mut().for_each(Linea::limpiar);
        self.fase = [0.0, 0.33, 0.66];
    }
}

/// Flanger: un retardo cortito que barre, con realimentación.
pub struct Flanger {
    pub hz: f32,
    pub hondo: f32,
    pub ms: f32,
    pub repite: f32,
    l: [Linea; 2],
    fase: f32,
}

impl Flanger {
    pub fn nuevo() -> Flanger {
        let n = (S * 0.05) as usize;
        Flanger { hz: 0.3, hondo: 4.0, ms: 0.5, repite: 0.4, l: [Linea::nueva(n), Linea::nueva(n)], fase: 0.0 }
    }
    pub fn procesar(&mut self, buf: &mut [f32], a: f32, b: f32) {
        for f in buf.chunks_exact_mut(2) {
            self.fase = (self.fase + self.hz / S).fract();
            for k in 0..2 {
                // Un lado va un cuarto de vuelta después.
                let o = 0.5 - 0.5 * ((self.fase + k as f32 * 0.25) * TAU).cos();
                let y = self.l[k].leer((self.ms + self.hondo * o + 0.05) * 0.001 * S + 1.0);
                self.l[k].meter((f[k] + y * self.repite).clamp(-8.0, 8.0));
                f[k] = f[k] * a + y * b;
            }
        }
    }
    pub fn limpiar(&mut self) {
        self.l.iter_mut().for_each(Linea::limpiar);
        self.fase = 0.0;
    }
}

/// Fáser: seis pasatodos que barren y se restan del sonido.
#[derive(Default)]
pub struct Faser {
    pub hz: f32,
    pub hondo: f32,
    pub repite: f32,
    pub mezcla: f32,
    z: [[f32; 6]; 2],
    ult: [f32; 2],
    fase: f32,
}

impl Faser {
    pub fn procesar(&mut self, buf: &mut [f32]) {
        for f in buf.chunks_exact_mut(2) {
            self.fase = (self.fase + self.hz / S).fract();
            for k in 0..2 {
                let o = 0.5 - 0.5 * ((self.fase + k as f32 * 0.2) * TAU).cos();
                let hz = 250.0 * 2f32.powf(o * self.hondo * 4.5);
                let t = (PI * hz / S).tan();
                let c = (t - 1.0) / (t + 1.0);
                let mut y = f[k] + self.ult[k] * self.repite * 0.9;
                for z in &mut self.z[k] {
                    let s = c * y + *z;
                    *z = y - c * s;
                    y = s;
                }
                self.ult[k] = y.clamp(-8.0, 8.0);
                f[k] = f[k] * (1.0 - self.mezcla) + y * self.mezcla;
            }
        }
    }
    pub fn limpiar(&mut self) {
        (self.z, self.ult, self.fase) = Default::default();
    }
}

// ---- Filtro

/// Filtro de estado variable que se puede mover sin chasquidos (A. Simper).
#[derive(Clone, Copy, Default)]
pub struct Variable {
    g: f32,
    k: f32,
    z: [[f32; 2]; 2],
}

impl Variable {
    /// `filo`: de 0 (suave) a 1 (casi silba).
    pub fn poner(&mut self, hz: f32, filo: f32) {
        self.g = (PI * hz.clamp(15.0, S * 0.45) / S).tan();
        self.k = 2.0 - 1.93 * filo.clamp(0.0, 1.0);
    }
    /// Pasa graves, banda y agudos.
    #[inline]
    pub fn paso(&mut self, c: usize, x: f32) -> [f32; 3] {
        let z = &mut self.z[c];
        let a = 1.0 / (1.0 + self.g * (self.g + self.k));
        let v3 = x - z[1];
        let v1 = a * z[0] + self.g * a * v3;
        let v2 = z[1] + self.g * v1;
        z[0] = 2.0 * v1 - z[0];
        z[1] = 2.0 * v2 - z[1];
        [v2, v1, x - self.k * v1 - v2]
    }
    pub fn limpiar(&mut self) {
        self.z = Default::default();
    }
}

// ---- Distorsión

/// Lo quedo pasa igual; lo que pasa del umbral se redondea hacia 1.
#[inline]
pub fn redondear(x: f32, umbral: f32) -> f32 {
    let m = x.abs();
    if m <= umbral {
        return x;
    }
    let falta = (1.0 - umbral).max(0.01);
    (umbral + falta * ((m - umbral) / falta).tanh()).copysign(x)
}

/// Distorsión tibia (como un bulbo) o rota (recorte con doblez).
#[inline]
pub fn distorsion(x: f32, empuje: f32, rota: bool) -> f32 {
    let y = x * empuje;
    if rota {
        // Lo que se pasa se regresa una vez y luego se recorta.
        let d = if y.abs() > 1.0 { (2.0 - y.abs()).max(0.25).copysign(y) } else { y };
        d.clamp(-1.0, 1.0)
    } else {
        // Un poco más de un lado que del otro: agrega armónicos pares.
        let b = 0.12;
        (y + b).tanh() - b.tanh()
    }
}

// ---- Estéreo

/// Gira la fase del lado derecho respecto al izquierdo, sin cambiar el tono.
#[derive(Default)]
struct Giro {
    z: [[[f32; 2]; 8]; 2],
}

impl Giro {
    /// Dos caminos de pasatodos que quedan a un cuarto de vuelta (coeficientes de la pareja clásica de O. Niemitalo).
    const A: [f32; 4] = [0.692_387_8, 0.936_065_4, 0.988_229_5, 0.998_748_9];
    const B: [f32; 4] = [0.402_192_1, 0.856_171_1, 0.972_290_9, 0.995_288_5];
    fn paso(&mut self, c: usize, x: f32) -> (f32, f32) {
        let mut camino = |v: &[f32; 4], des: usize| {
            let mut y = x;
            for (j, a) in v.iter().enumerate() {
                let z = &mut self.z[c][des + j];
                // y[n] = a² (x[n] + y[n-2]) - x[n-2], con el estado junto.
                let s = a * a * y + z[0];
                z[0] = z[1];
                z[1] = a * a * s - y;
                y = s;
            }
            y
        };
        (camino(&Self::A, 0), camino(&Self::B, 4))
    }
}

/// Mezcla, retrasa y gira cada lado.
pub struct Moldeador {
    /// Derecha a izquierda, izquierda, derecha, izquierda a derecha; luego retraso y fase (-1 a 1).
    pub p: [f32; 6],
    l: [Linea; 2],
    giro: Giro,
    ant: [f32; 2],
}

impl Moldeador {
    pub fn nuevo() -> Moldeador {
        let n = (S * 0.03) as usize;
        Moldeador { p: [0.0, 1.0, 1.0, 0.0, 0.0, 0.0], l: [Linea::nueva(n), Linea::nueva(n)], giro: Giro::default(), ant: [0.0; 2] }
    }
    pub fn procesar(&mut self, buf: &mut [f32]) {
        let [di, i, d, id, retraso, fase] = self.p;
        // Con retraso negativo se atrasa la izquierda; con positivo, la derecha. Hasta 20 ms.
        let atraso = [(-retraso).max(0.0), retraso.max(0.0)].map(|r| r * 0.02 * S);
        let (s, c) = (fase * FRAC_PI_2 * 2.0).sin_cos();
        for f in buf.chunks_exact_mut(2) {
            let mut y = [f[0] * i + f[1] * di, f[1] * d + f[0] * id];
            for k in 0..2 {
                self.l[k].meter(y[k]);
                if atraso[k] >= 0.5 {
                    y[k] = self.l[k].leer(atraso[k] + 1.0);
                }
            }
            if fase != 0.0 {
                let (a, _) = self.giro.paso(0, y[0]);
                let (b, q) = self.giro.paso(1, y[1]);
                // El segundo camino va una muestra después.
                let q_ant = std::mem::replace(&mut self.ant[1], q);
                y = [a, b * c + q_ant * s];
            }
            (f[0], f[1]) = (y[0], y[1]);
        }
    }
    pub fn limpiar(&mut self) {
        self.l.iter_mut().for_each(Linea::limpiar);
        (self.giro, self.ant) = (Giro::default(), [0.0; 2]);
    }
}

// ---- Vocoder

/// La fuerza de cada banda de la voz mueve la misma banda de otro sonido.
pub struct Vocoder {
    voz: Vec<Bq>,
    otro: Vec<Bq>,
    env: Vec<f32>,
    aire: Bq,
    ka: f32,
    ks: f32,
    ruido: f32,
    paso_voz: f32,
    azar: Azar,
}

impl Vocoder {
    pub fn nuevo() -> Vocoder {
        Vocoder { voz: vec![], otro: vec![], env: vec![], aire: Bq::default(), ka: 0.1, ks: 0.01, ruido: 0.0, paso_voz: 0.0, azar: Azar::nuevo(0x5bd1) }
    }
    pub fn ajustar(&mut self, bandas: usize, ataque: f32, suelta: f32, aire: f32) {
        let n = bandas.clamp(4, 64);
        (self.ka, self.ks) = (paso_de(ataque), paso_de(suelta));
        (self.ruido, self.paso_voz) = (aire * 0.35, aire * 0.5);
        self.aire.poner(Banda::PasaAgudos, 5500.0, 0.0, 0.7);
        if self.voz.len() != n {
            let razon = (11_000f32 / 90.0).powf(1.0 / (n - 1) as f32);
            let q = 1.0 / (razon.sqrt() - 1.0 / razon.sqrt());
            self.voz = (0..n)
                .map(|k| {
                    let mut b = Bq::default();
                    b.poner(Banda::PasaBanda, 90.0 * razon.powi(k as i32), 0.0, q);
                    b
                })
                .collect();
            self.otro = self.voz.clone();
            self.env = vec![0.0; n];
        }
    }
    /// `buf`: la voz; `otro`: lo que va a hablar. Devuelve el resultado en `otro`.
    pub fn procesar(&mut self, buf: &[f32], otro: &mut [f32]) {
        let n = self.voz.len() as f32;
        // Con más bandas, cada una lleva menos: se compensa.
        let g = 0.75 * (n / 8.0).sqrt();
        for (f, o) in buf.chunks_exact(2).zip(otro.chunks_exact_mut(2)) {
            let m = (f[0] + f[1]) * 0.5;
            let r = self.azar.dar() * self.ruido;
            let x = [o[0] + r, o[1] + r];
            let mut y = [0f32; 2];
            for ((v, c), e) in self.voz.iter_mut().zip(&mut self.otro).zip(&mut self.env) {
                let a = v.paso(0, m).abs();
                *e += (a - *e) * if a > *e { self.ka } else { self.ks };
                y[0] += c.paso(0, x[0]) * *e;
                y[1] += c.paso(1, x[1]) * *e;
            }
            for k in 0..2 {
                o[k] = y[k] * g + self.aire.paso(k, f[k]) * self.paso_voz;
            }
        }
    }
    pub fn limpiar(&mut self) {
        self.voz.iter_mut().chain(&mut self.otro).for_each(Bq::limpiar);
        self.env.fill(0.0);
        self.aire.limpiar();
    }
}

// ---- Sintetizador

/// Cómo suena: tres osciladores, su mezcla, envolvente y filtro.
#[derive(Clone, Debug, PartialEq)]
pub struct Timbre {
    /// Senoidal, triangular, cuadrada, sierra, sierra suave, ruido.
    pub onda: [u8; 3],
    pub semitonos: [f32; 3],
    /// Cuánto del segundo, y luego cuánto del tercero.
    pub mezcla: [f32; 2],
    /// De 0 a 1.
    pub abierto: f32,
    /// Ataque, caída, sostén (0 a 1) y salida; los tiempos en segundos.
    pub adsr: [f32; 4],
    /// Sin valor, sin filtro.
    pub corte: Option<f32>,
    pub filo: f32,
}

impl Default for Timbre {
    fn default() -> Self {
        Timbre { onda: [3; 3], semitonos: [0.0; 3], mezcla: [0.0; 2], abierto: 0.0, adsr: [0.005, 0.3, 0.7, 0.2], corte: None, filo: 0.0 }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Etapa {
    Sube,
    Cae,
    Suelta,
}

struct Voz {
    tecla: u8,
    fuerza: f32,
    fase: [[f32; 2]; 3],
    tri: [[f32; 2]; 3],
    etapa: Etapa,
    env: f32,
    filtro: Variable,
    edad: u64,
}

pub struct Sinte {
    voces: Vec<Voz>,
    azar: Azar,
    n: u64,
}

/// Suaviza el salto de una onda (PolyBLEP), para que no chille en los agudos.
#[inline]
fn blep(t: f32, d: f32) -> f32 {
    if t < d {
        let x = t / d;
        2.0 * x - x * x - 1.0
    } else if t > 1.0 - d {
        let x = (t - 1.0) / d;
        x * x + 2.0 * x + 1.0
    } else {
        0.0
    }
}

impl Sinte {
    pub const VOCES: usize = 32;
    pub fn nuevo() -> Sinte {
        Sinte { voces: Vec::with_capacity(Self::VOCES), azar: Azar::nuevo(0x2f6b), n: 0 }
    }
    pub fn callar(&mut self) {
        self.voces.clear();
    }
    pub fn tocar(&mut self, tecla: u8, fuerza: f32) {
        if self.voces.len() >= Self::VOCES
            && let Some(i) = (0..self.voces.len()).min_by_key(|i| self.voces[*i].edad)
        {
            self.voces.swap_remove(i);
        }
        self.n += 1;
        let mut f = || self.azar.dar().abs() * 0.2;
        let fase = std::array::from_fn(|_| [f(), f()]);
        self.voces.push(Voz { tecla, fuerza, fase, tri: [[0.0; 2]; 3], etapa: Etapa::Sube, env: 0.0, filtro: Variable::default(), edad: self.n });
    }
    pub fn soltar(&mut self, tecla: u8) {
        self.voces.iter_mut().filter(|v| v.tecla == tecla).for_each(|v| v.etapa = Etapa::Suelta);
    }
    /// Suma su sonido en `out` (estéreo intercalado).
    pub fn sonar(&mut self, out: &mut [f32], t: &Timbre, nivel: f32) {
        let [ataque, caida, sosten, salida] = t.adsr;
        let sube = 1.0 / (ataque.max(0.0005) * S);
        // Las caídas son curvas: en su tiempo bajan a un veinticincoavo.
        let curva = |s: f32| 1.0 - (-3.2 / (s.max(0.002) * S)).exp();
        let (kc, ks) = (curva(caida), curva(salida));
        let pesos = [(1.0 - t.mezcla[0]) * (1.0 - t.mezcla[1]), t.mezcla[0] * (1.0 - t.mezcla[1]), t.mezcla[1]];
        // Cada lado, apenas desafinado.
        let des = 2f32.powf(t.abierto * 7.0 / 1200.0);
        // Los dos primeros se abren a los lados.
        let lado = t.abierto * 0.5;
        let pan = [[1.0 + lado * 0.5, 1.0 - lado * 0.5], [1.0 - lado * 0.5, 1.0 + lado * 0.5], [1.0, 1.0]];
        let azar = &mut self.azar;
        self.voces.retain_mut(|v| {
            let hz = 440.0 * 2f32.powf((v.tecla as f32 - 69.0) / 12.0);
            let d: [[f32; 2]; 3] = std::array::from_fn(|o| {
                let h = hz * 2f32.powf(t.semitonos[o] / 12.0) / S;
                [(h * des).min(0.45), (h / des).min(0.45)]
            });
            if let Some(c) = t.corte {
                v.filtro.poner(c, t.filo);
            }
            for f in out.chunks_exact_mut(2) {
                match v.etapa {
                    Etapa::Sube => {
                        v.env += sube;
                        if v.env >= 1.0 {
                            (v.env, v.etapa) = (1.0, Etapa::Cae);
                        }
                    }
                    Etapa::Cae => v.env += (sosten - v.env) * kc,
                    Etapa::Suelta => v.env -= v.env * ks,
                }
                if v.env < 1e-5 && v.etapa != Etapa::Sube {
                    return false;
                }
                let g = v.env * v.fuerza * nivel;
                for k in 0..2 {
                    let mut x = 0.0;
                    for o in 0..3 {
                        if pesos[o] <= 0.0 {
                            continue;
                        }
                        let (p, d) = (v.fase[o][k], d[o][k]);
                        let cuadrada = || (if p < 0.5 { 1.0 } else { -1.0 }) + blep(p, d) - blep((p + 0.5).fract(), d);
                        let sierra = || 2.0 * p - 1.0 - blep(p, d);
                        let y = match t.onda[o] {
                            0 => (p * TAU).sin(),
                            1 => {
                                // La cuadrada acumulada es una triangular sin picos falsos.
                                let z = &mut v.tri[o][k];
                                *z += 4.0 * d * cuadrada() - *z * d * 0.02;
                                z.clamp(-1.2, 1.2)
                            }
                            2 => cuadrada() * 0.8,
                            3 => sierra(),
                            4 => sierra() * 0.35 + (p * TAU).sin() * -0.6,
                            _ => azar.dar(),
                        };
                        x += y * pesos[o] * pan[o][k];
                        v.fase[o][k] = (p + d).fract();
                    }
                    if t.corte.is_some() {
                        x = v.filtro.paso(k, x)[0];
                    }
                    f[k] += x * g;
                }
            }
            true
        });
    }
}
