// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Las recetas de sonido: un grafo plano que Lua arma una sola vez y que el hilo de audio toca sin volver a Lua.
//! Se procesa por bloques y no pide memoria.
use crate::muestrario::{CAPAS, Muestra};
use std::f32::consts::TAU;

pub const BLOQUE: usize = 64;
pub type Buf = [f32; BLOQUE];
/// Tope de retardo por nodo, en segundos.
pub const MAX_RETARDO: f32 = 2.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Banda {
    Campana,
    Graves,
    Agudos,
    Muesca,
}

/// Los números de una banda de ecualizador (fórmulas públicas de R. Bristow-Johnson): b0, b1, b2, a1, a2.
pub fn banda(tipo: Banda, hz: f32, db: f32, q: f32, sr: f32) -> [f32; 5] {
    let w = TAU * hz.clamp(10.0, sr * 0.45) / sr;
    let (s, c) = w.sin_cos();
    let a = 10f32.powf(db.clamp(-48.0, 48.0) / 40.0);
    let al = s / (2.0 * q.clamp(0.1, 30.0));
    let r = 2.0 * a.sqrt() * al;
    let (b0, b1, b2, a0, a1, a2) = match tipo {
        Banda::Campana => (1.0 + al * a, -2.0 * c, 1.0 - al * a, 1.0 + al / a, -2.0 * c, 1.0 - al / a),
        Banda::Muesca => (1.0, -2.0 * c, 1.0, 1.0 + al, -2.0 * c, 1.0 - al),
        Banda::Graves => (
            a * ((a + 1.0) - (a - 1.0) * c + r),
            2.0 * a * ((a - 1.0) - (a + 1.0) * c),
            a * ((a + 1.0) - (a - 1.0) * c - r),
            (a + 1.0) + (a - 1.0) * c + r,
            -2.0 * ((a - 1.0) + (a + 1.0) * c),
            (a + 1.0) + (a - 1.0) * c - r,
        ),
        Banda::Agudos => (
            a * ((a + 1.0) + (a - 1.0) * c + r),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * c),
            a * ((a + 1.0) + (a - 1.0) * c - r),
            (a + 1.0) - (a - 1.0) * c + r,
            2.0 * ((a - 1.0) - (a + 1.0) * c),
            (a + 1.0) - (a - 1.0) * c - r,
        ),
    };
    [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0]
}

/// Cuánto sube o baja esa banda una frecuencia, en decibeles.
pub fn respuesta(k: &[f32; 5], hz: f32, sr: f32) -> f32 {
    let (s, c) = (TAU * hz / sr).sin_cos();
    let (s2, c2) = (2.0 * TAU * hz / sr).sin_cos();
    let n = (k[0] + k[1] * c + k[2] * c2).powi(2) + (k[1] * s + k[2] * s2).powi(2);
    let d = (1.0 + k[3] * c + k[4] * c2).powi(2) + (k[3] * s + k[4] * s2).powi(2);
    10.0 * (n / d.max(1e-12)).max(1e-12).log10()
}

/// Índice de otro nodo (siempre anterior).
pub type N = u16;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Onda {
    Seno,
    Sierra,
    Pulso,
    Triangulo,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Filtro {
    Bajo,
    Alto,
    Banda,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Nodo {
    Const(f32),
    Control(u16),
    Hz,
    Fuerza,
    Azar,
    /// 0 izquierda, 1 derecha.
    Entrada(u8),
    Suma(N, N),
    Resta(N, N),
    Por(N, N),
    Entre(N, N),
    Min(N, N),
    Max(N, N),
    Pot(N, N),
    Tanh(N),
    Abs(N),
    Sen(N),
    /// onda, hz, fase sumada (en vueltas), ancho del pulso.
    Osc(Onda, N, N, N),
    Ruido,
    /// ataque, caída, sostén, soltar (segundos, segundos, 0-1, segundos).
    Adsr(N, N, N, N),
    /// Cae sola en esos segundos, sin importar la tecla.
    Golpe(N),
    Svf(Filtro, N, N, N),
    /// banda, entrada, hz, decibeles, ancho.
    Eq(Banda, N, N, N, N),
    /// Sigue el nivel: entrada, ataque y soltar en segundos.
    Seguidor(N, N, N),
    /// Redondea a tantos escalones por lado.
    Escalon(N, N),
    /// Guarda el valor y lo renueva tantas veces por segundo.
    Retener(N, N),
    /// Pasatodo de primer orden: solo mueve la fase.
    Pasatodo(N, N),
    /// entrada, segundos, realimentación, tope del retardo en segundos.
    Retardo(N, N, N, f32),
    /// entrada, tamaño 0-1, brillo 0-1.
    Sala(N, N, N),
    /// excitación, hz, caída en segundos, brillo 0-1.
    Cuerda(N, N, N, N),
    /// velocidad, canal (0/1).
    Muestra(N, u8),
    Raiz(N),
    Log(N),
    Exp(N),
    Piso(N),
    /// 1 si el primero es mayor; si no, 0.
    Mayor(N, N),
    Resto(N, N),
    /// El tempo de la canción.
    Bpm,
    /// Negras desde el principio de la canción; parada, sigue contando sola.
    Posicion,
    /// Una curva hecha en Lua: entrada, cuál tabla y si se suaviza para que no raspe.
    Curva(N, u16, bool),
    /// Una onda hecha en Lua: hz, fase sumada y cuál tabla.
    Propia(N, N, u16),
    /// La salida de un lazo: segundos atrás y tope.
    Lazo(N, f32),
    /// Pasa igual y deja ver su valor en la interfaz: entrada y cuál medida.
    Mide(N, u16),
    /// Convolución con la huella puesta en el efecto: entrada y lado (0/1).
    Convolucion(N, u8),
    /// Lo que entra al lazo (el segundo) para el `Lazo` que es el primero. Su salida es la de ese lazo.
    Cierra(N, N),
}

/// Puntos de una onda o de una curva hecha en Lua.
pub const PUNTOS: usize = 2048;
/// Hasta dónde llega la entrada de una curva.
pub const ALCANCE: f32 = 4.0;

/// Una tabla que el guion llenó una sola vez, al armarse.
#[derive(Debug)]
pub struct Tabla {
    /// Una curva trae una sola; una onda, una por octava, cada una con la mitad de armónicos.
    pub v: Vec<Vec<f32>>,
    /// El área bajo la curva, para suavizarla.
    area: Vec<f64>,
}

impl Tabla {
    /// De `2 * PUNTOS + 1` valores repartidos entre -ALCANCE y ALCANCE.
    pub fn curva(v: Vec<f32>) -> Tabla {
        let h = 2.0 * ALCANCE as f64 / (v.len() - 1) as f64;
        let mut area = vec![0.0f64; v.len()];
        for i in 1..v.len() {
            area[i] = area[i - 1] + (v[i - 1] as f64 + v[i] as f64) * 0.5 * h;
        }
        Tabla { v: vec![v], area }
    }

    /// De una vuelta de `PUNTOS` valores. Las versiones sin armónicos altos evitan que las notas agudas raspen.
    pub fn onda(v: &[f32]) -> Tabla {
        let n = v.len();
        let seno: Vec<f64> = (0..n).map(|i| (std::f64::consts::TAU * i as f64 / n as f64).sin()).collect();
        let cos = |i: usize| seno[(i + n / 4) % n];
        let partes: Vec<(f64, f64)> = (0..n / 2)
            .map(|k| {
                let (a, b) =
                    v.iter().enumerate().fold((0.0, 0.0), |(a, b), (i, x)| (a + *x as f64 * cos(i * k % n), b + *x as f64 * seno[i * k % n]));
                let g = if k == 0 { 1.0 } else { 2.0 } / n as f64;
                (a * g, b * g)
            })
            .collect();
        let mut niveles = vec![v.to_vec()];
        let mut tope = n / 4;
        while tope >= 1 {
            let nivel = (0..n)
                .map(|i| partes[..=tope].iter().enumerate().map(|(k, (a, b))| a * cos(i * k % n) + b * seno[i * k % n]).sum::<f64>() as f32)
                .collect();
            niveles.push(nivel);
            tope /= 2;
        }
        Tabla { v: niveles, area: vec![] }
    }

    #[inline(always)]
    fn en(&self, x: f32) -> (usize, f32) {
        let q = (x.clamp(-ALCANCE, ALCANCE) + ALCANCE) / (2.0 * ALCANCE) * (self.v[0].len() - 1) as f32;
        let i = (q as usize).min(self.v[0].len() - 2);
        (i, q - i as f32)
    }

    #[inline(always)]
    fn leer(&self, x: f32) -> f32 {
        let (i, t) = self.en(x);
        let v = &self.v[0];
        v[i] + (v[i + 1] - v[i]) * t
    }

    #[inline(always)]
    fn area(&self, x: f32) -> f64 {
        let (i, t) = self.en(x);
        let v = &self.v[0];
        let h = 2.0 * ALCANCE as f64 / (v.len() - 1) as f64;
        let t = t as f64;
        self.area[i] + h * t * (v[i] as f64 + (v[i + 1] - v[i]) as f64 * t * 0.5)
    }
}

/// Retardos de la sala, en segundos a tamaño 1. Primos entre sí en muestras.
const SALA: [f32; 8] = [0.0297, 0.0371, 0.0411, 0.0437, 0.0533, 0.0617, 0.0689, 0.0797];
const SALA_MAX: f32 = 0.085;

impl Nodo {
    fn entradas(&self) -> [Option<N>; 4] {
        use Nodo::*;
        match *self {
            Suma(a, b)
            | Resta(a, b)
            | Por(a, b)
            | Entre(a, b)
            | Min(a, b)
            | Max(a, b)
            | Pot(a, b)
            | Escalon(a, b)
            | Retener(a, b)
            | Pasatodo(a, b)
            | Mayor(a, b)
            | Resto(a, b)
            | Propia(a, b, _)
            | Cierra(a, b) => [Some(a), Some(b), None, None],
            Tanh(a)
            | Abs(a)
            | Sen(a)
            | Golpe(a)
            | Muestra(a, _)
            | Raiz(a)
            | Log(a)
            | Exp(a)
            | Piso(a)
            | Curva(a, ..)
            | Lazo(a, _)
            | Mide(a, _)
            | Convolucion(a, _) => [Some(a), None, None, None],
            Osc(_, a, b, c) | Svf(_, a, b, c) | Retardo(a, b, c, _) | Sala(a, b, c) | Seguidor(a, b, c) => [Some(a), Some(b), Some(c), None],
            Adsr(a, b, c, d) | Cuerda(a, b, c, d) | Eq(_, a, b, c, d) => [Some(a), Some(b), Some(c), Some(d)],
            _ => [None; 4],
        }
    }

    fn con_entradas(self, f: impl Fn(N) -> N) -> Nodo {
        use Nodo::*;
        match self {
            Suma(a, b) => Suma(f(a), f(b)),
            Resta(a, b) => Resta(f(a), f(b)),
            Por(a, b) => Por(f(a), f(b)),
            Entre(a, b) => Entre(f(a), f(b)),
            Min(a, b) => Min(f(a), f(b)),
            Max(a, b) => Max(f(a), f(b)),
            Pot(a, b) => Pot(f(a), f(b)),
            Tanh(a) => Tanh(f(a)),
            Abs(a) => Abs(f(a)),
            Sen(a) => Sen(f(a)),
            Golpe(a) => Golpe(f(a)),
            Muestra(a, c) => Muestra(f(a), c),
            Raiz(a) => Raiz(f(a)),
            Log(a) => Log(f(a)),
            Exp(a) => Exp(f(a)),
            Piso(a) => Piso(f(a)),
            Mayor(a, b) => Mayor(f(a), f(b)),
            Resto(a, b) => Resto(f(a), f(b)),
            Curva(a, t, s) => Curva(f(a), t, s),
            Propia(a, b, t) => Propia(f(a), f(b), t),
            Lazo(a, m) => Lazo(f(a), m),
            Mide(a, k) => Mide(f(a), k),
            Convolucion(a, l) => Convolucion(f(a), l),
            Cierra(a, b) => Cierra(f(a), f(b)),
            Osc(o, a, b, c) => Osc(o, f(a), f(b), f(c)),
            Svf(t, a, b, c) => Svf(t, f(a), f(b), f(c)),
            Retardo(a, b, c, m) => Retardo(f(a), f(b), f(c), m),
            Sala(a, b, c) => Sala(f(a), f(b), f(c)),
            Adsr(a, b, c, d) => Adsr(f(a), f(b), f(c), f(d)),
            Cuerda(a, b, c, d) => Cuerda(f(a), f(b), f(c), f(d)),
            Eq(t, a, b, c, d) => Eq(t, f(a), f(b), f(c), f(d)),
            Seguidor(a, b, c) => Seguidor(f(a), f(b), f(c)),
            Escalon(a, b) => Escalon(f(a), f(b)),
            Retener(a, b) => Retener(f(a), f(b)),
            Pasatodo(a, b) => Pasatodo(f(a), f(b)),
            x => x,
        }
    }

    /// Cuántos números guarda cada voz para este nodo.
    fn memoria(&self, sr: f32) -> usize {
        match *self {
            Nodo::Osc(..) | Nodo::Ruido => 1,
            Nodo::Adsr(..) => 3,
            Nodo::Golpe(_) | Nodo::Seguidor(..) | Nodo::Pasatodo(..) => 1,
            Nodo::Eq(..) | Nodo::Retener(..) => 2,
            Nodo::Svf(..) => 2,
            Nodo::Retardo(_, _, _, m) => 1 + (m * sr) as usize + 4,
            Nodo::Sala(..) => 1 + 8 + 8 * ((SALA_MAX * sr) as usize + 4),
            Nodo::Cuerda(..) => 2 + (sr / 20.0) as usize + 4,
            Nodo::Muestra(..) => 2 * CAPAS,
            Nodo::Curva(_, _, true) | Nodo::Propia(..) | Nodo::Mide(..) => 1,
            Nodo::Convolucion(..) => crate::impulso::Impulso::memoria(sr),
            Nodo::Lazo(_, m) => 1 + (m * sr) as usize + BLOQUE + 4,
            _ => 0,
        }
    }

    /// Da lo mismo para todas las voces del canal.
    fn comun(&self) -> bool {
        use Nodo::*;
        matches!(
            self,
            Const(_)
                | Control(_)
                | Suma(..)
                | Resta(..)
                | Por(..)
                | Entre(..)
                | Min(..)
                | Max(..)
                | Pot(..)
                | Tanh(_)
                | Abs(_)
                | Sen(_)
                | Raiz(_)
                | Log(_)
                | Exp(_)
                | Piso(_)
                | Mayor(..)
                | Resto(..)
                | Curva(_, _, false)
        )
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Control {
    pub clave: String,
    pub nombre: crate::idioma::Texto,
    pub min: f32,
    pub max: f32,
    pub valor: f32,
}

/// Un sonido listo para tocar.
#[derive(Debug)]
pub struct Receta {
    pub controles: Vec<Control>,
    nodos: Vec<(Nodo, u32)>,
    /// Los primeros nodos son los comunes a todas las voces.
    comunes: usize,
    sale: [N; 2],
    pub memoria: usize,
    pub voces: usize,
    /// Segundos que puede callarse y volver a sonar (por sus retardos).
    pub cola: f32,
    sr: f32,
    /// Las ondas y curvas que hizo el guion.
    pub tablas: Vec<Tabla>,
    /// Lo que el guion deja ver: nombre y en qué lugar de la memoria está.
    pub medidas: Vec<(String, usize)>,
    /// Corre Lua en vivo, y lo que mide desde ahí.
    pub vivo: bool,
    pub sostiene: bool,
    pub medidas_vivo: Vec<String>,
}

impl Receta {
    /// Quita lo que no llega a la salida, junta lo repetido y adelanta lo común.
    pub fn nueva(nodos: &[Nodo], sale: [N; 2], controles: Vec<Control>, voces: usize, sr: f32) -> Receta {
        // Lo repetido: mismo nodo con las mismas entradas (menos el ruido, que cada uno es distinto).
        let mut igual: Vec<N> = Vec::with_capacity(nodos.len());
        let mut unicos: Vec<Nodo> = vec![];
        for n in nodos {
            let n = n.con_entradas(|i| igual[i as usize]);
            let ya = if n == Nodo::Ruido || n == Nodo::Azar || matches!(n, Nodo::Lazo(..)) { None } else { unicos.iter().position(|u| *u == n) };
            igual.push(ya.unwrap_or_else(|| {
                unicos.push(n);
                unicos.len() - 1
            }) as N);
        }
        let sale = sale.map(|s| igual[s as usize]);
        // Lo que sí suena.
        let mut vivo = vec![false; unicos.len()];
        sale.iter().for_each(|s| vivo[*s as usize] = true);
        for i in (0..unicos.len()).rev() {
            if vivo[i] {
                unicos[i].entradas().into_iter().flatten().for_each(|e| vivo[e as usize] = true);
            }
        }
        let mut comun = vec![false; unicos.len()];
        for i in 0..unicos.len() {
            comun[i] = unicos[i].comun() && unicos[i].entradas().into_iter().flatten().all(|e| comun[e as usize]);
        }
        let orden: Vec<usize> =
            (0..unicos.len()).filter(|i| vivo[*i] && comun[*i]).chain((0..unicos.len()).filter(|i| vivo[*i] && !comun[*i])).collect();
        let mut nuevo = vec![0 as N; unicos.len()];
        orden.iter().enumerate().for_each(|(k, i)| nuevo[*i] = k as N);
        let mut memoria = 0;
        let nodos: Vec<(Nodo, u32)> = orden
            .iter()
            .map(|i| {
                let n = unicos[*i].con_entradas(|e| nuevo[e as usize]);
                let off = memoria as u32;
                memoria += n.memoria(sr);
                (n, off)
            })
            .collect();
        let comunes = orden.iter().filter(|i| comun[**i]).count();
        let cola = nodos.iter().map(|(n, _)| if let Nodo::Retardo(_, _, _, m) | Nodo::Lazo(_, m) = n { *m } else { 0.0 }).sum();
        Receta {
            controles,
            nodos,
            comunes,
            sale: sale.map(|s| nuevo[s as usize]),
            memoria,
            voces,
            cola,
            sr,
            tablas: vec![],
            medidas: vec![],
            vivo: false,
            sostiene: false,
            medidas_vivo: vec![],
        }
    }

    /// Les pone nombre a los `Mide`.
    pub fn nombrar_medidas(&mut self, nombres: &[String]) {
        self.medidas = self
            .nodos
            .iter()
            .filter_map(|(n, off)| if let Nodo::Mide(_, k) = n { Some((nombres.get(*k as usize)?.clone(), *off as usize)) } else { None })
            .collect();
    }

    /// Todo lo que deja ver, en orden.
    pub fn nombres_medidas(&self) -> impl Iterator<Item = &str> {
        self.medidas.iter().map(|m| m.0.as_str()).chain(self.medidas_vivo.iter().map(String::as_str)).take(crate::lua::MAX_MEDIDAS)
    }

    /// Si necesita la huella de un lugar.
    pub fn con_huella(&self) -> bool {
        self.nodos.iter().any(|(n, _)| matches!(n, Nodo::Convolucion(..)))
    }

    /// Si toca una grabación.
    pub fn usa_muestra(&self) -> bool {
        self.nodos.iter().any(|(n, _)| matches!(n, Nodo::Muestra(..)))
    }

    /// Si oye otra pista además de la suya.
    pub fn con_lateral(&self) -> bool {
        self.nodos.iter().any(|(n, _)| matches!(n, Nodo::Entrada(2 | 3)))
    }

    pub fn sr(&self) -> f32 {
        self.sr
    }

    pub fn cuantos_nodos(&self) -> usize {
        self.nodos.len()
    }

    pub fn cuantos_comunes(&self) -> usize {
        self.comunes
    }
}

/// Lo que una voz sabe de su nota.
#[derive(Clone, Copy, Default)]
pub struct DatosVoz {
    pub hz: f32,
    pub fuerza: f32,
    pub azar: f32,
    pub puerta: bool,
    /// Lo que le suma su pad: volumen, velocidad y lado.
    pub pad: [f32; 3],
    /// Desde qué parte de la grabación empieza (0 a 1).
    pub desde: f32,
    /// El tempo; en cero cuenta como 120.
    pub bpm: f32,
    /// En qué negra empieza este bloque.
    pub negra: f32,
    /// Las grabaciones que le tocan.
    pub zonas: [u16; CAPAS],
    pub cuantas: u8,
}

pub fn comunes(r: &Receta, ctl: &[f32], b: &mut [Buf], n: usize) {
    let vacia = DatosVoz::default();
    for i in 0..r.comunes {
        tocar_nodo(r, i, ctl, &vacia, None, None, &mut [], b, n);
    }
}

/// Toca `n` cuadros de una voz. `b[0..comunes]` ya está lleno.
pub fn voz(r: &Receta, d: &DatosVoz, ctl: &[f32], m: Option<&Muestra>, entra: Option<&[Buf]>, mem: &mut [f32], b: &mut [Buf], n: usize) -> [N; 2] {
    for i in r.comunes..r.nodos.len() {
        tocar_nodo(r, i, ctl, d, m, entra, mem, b, n);
    }
    r.sale
}

/// Corrección del salto de las ondas para que no raspen.
#[inline(always)]
fn pulir(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

/// Seno de `x` vueltas. Error menor a 0.0002.
#[inline(always)]
pub fn seno(x: f32) -> f32 {
    let x = x - x.floor();
    let x = if x < 0.25 {
        x
    } else if x < 0.75 {
        0.5 - x
    } else {
        x - 1.0
    };
    let y = x * TAU;
    let z = y * y;
    y * (1.0 + z * (-1.0 / 6.0 + z * (1.0 / 120.0 - z * (1.0 / 5040.0))))
}

#[inline(always)]
fn ruido(s: &mut f32) -> f32 {
    let mut x = s.to_bits();
    if x == 0 {
        x = 0x9E37_79B9;
    }
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *s = f32::from_bits(x);
    x as f32 / u32::MAX as f32 * 2.0 - 1.0
}

/// Lee `d` muestras atrás con una curva de cuatro puntos; con muy poco atraso, entre dos.
#[inline(always)]
fn leer4(l: &[f32], pos: usize, d: f32) -> f32 {
    let largo = l.len();
    let e = d as usize;
    let f = d - e as f32;
    if e < 2 || e + 3 > largo || f == 0.0 {
        return leer(l, pos, d);
    }
    let en = |k: usize| l[(pos + largo - k) % largo];
    let (p, b, c, q) = (en(e - 1), en(e), en(e + 1), en(e + 2));
    b + 0.5 * f * (c - p + f * (2.0 * p - 5.0 * b + 4.0 * c - q + f * (3.0 * (b - c) + q - p)))
}

/// Lee `d` muestras atrás, entre dos muestras.
#[inline(always)]
fn leer(l: &[f32], pos: usize, d: f32) -> f32 {
    let largo = l.len();
    let e = d as usize;
    let f = d - e as f32;
    let i = pos + largo - e;
    let a = l[if i >= largo { i - largo } else { i }];
    let j = i - 1;
    let c = l[if j >= largo { j - largo } else { j }];
    a + (c - a) * f
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn tocar_nodo(r: &Receta, i: usize, ctl: &[f32], d: &DatosVoz, m: Option<&Muestra>, entra: Option<&[Buf]>, mem: &mut [f32], b: &mut [Buf], n: usize) {
    use Nodo::*;
    let (nodo, off) = r.nodos[i];
    let off = off as usize;
    let sr = r.sr;
    let (antes, aqui) = b.split_at_mut(i);
    let o = &mut aqui[0][..n];
    let e = |k: N| &antes[k as usize][..n];
    macro_rules! dos {
        ($a:expr, $b:expr, |$x:ident, $y:ident| $f:expr) => {{
            let (a, c) = (e($a), e($b));
            for ((o, $x), $y) in o.iter_mut().zip(a.iter().copied()).zip(c.iter().copied()) {
                *o = $f;
            }
        }};
    }
    macro_rules! uno {
        ($a:expr, |$x:ident| $f:expr) => {
            for (o, $x) in o.iter_mut().zip(e($a).iter().copied()) {
                *o = $f;
            }
        };
    }
    match nodo {
        Const(v) => o.fill(v),
        Control(k) => o.fill(ctl[k as usize]),
        Hz => o.fill(d.hz),
        Fuerza => o.fill(d.fuerza),
        Azar => o.fill(d.azar),
        Entrada(l) => match entra {
            // 2 y 3 son la pista que se elija como entrada lateral.
            Some(x) if x.len() > l as usize => o.copy_from_slice(&x[l as usize][..n]),
            _ => o.fill(0.0),
        },
        Suma(a, c) => dos!(a, c, |x, y| x + y),
        Resta(a, c) => dos!(a, c, |x, y| x - y),
        Por(a, c) => dos!(a, c, |x, y| x * y),
        Entre(a, c) => dos!(a, c, |x, y| if y.abs() < 1e-9 { 0.0 } else { x / y }),
        Min(a, c) => dos!(a, c, |x, y| x.min(y)),
        Max(a, c) => dos!(a, c, |x, y| x.max(y)),
        Pot(a, c) => dos!(a, c, |x, y| {
            let v = x.powf(y);
            if v.is_finite() { v } else { 0.0 }
        }),
        Tanh(a) => uno!(a, |x| {
            // Aproximación racional: suave y mucho más barata que `tanh`.
            let x = x.clamp(-3.0, 3.0);
            x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
        }),
        Abs(a) => uno!(a, |x| x.abs()),
        Sen(a) => uno!(a, |x| seno(x)),
        Osc(onda, hz, fase, ancho) => {
            let (hz, fase, ancho) = (e(hz), e(fase), e(ancho));
            let mut p = mem[off];
            for k in 0..n {
                let dt = (hz[k] / sr).clamp(0.0, 0.49);
                let t = p + fase[k];
                let t = t - t.floor();
                o[k] = match onda {
                    Onda::Seno => seno(t),
                    Onda::Sierra => 2.0 * t - 1.0 - pulir(t, dt),
                    Onda::Pulso => {
                        let w = ancho[k].clamp(0.05, 0.95);
                        let u = t - w;
                        let u = if u < 0.0 { u + 1.0 } else { u };
                        (if t < w { 1.0 } else { -1.0 }) + pulir(t, dt) - pulir(u, dt)
                    }
                    Onda::Triangulo => 1.0 - 4.0 * (t - 0.5).abs(),
                };
                p += dt;
                if p >= 1.0 {
                    p -= 1.0;
                }
            }
            mem[off] = p;
        }
        Ruido => {
            let s = &mut mem[off];
            if *s == 0.0 {
                // Cada nota empieza con otra semilla.
                *s = f32::from_bits(((d.azar * 1e6) as u32).wrapping_mul(2_654_435_761) ^ 0x5bd1_e995 ^ (off as u32) << 8);
            }
            o.iter_mut().for_each(|o| *o = ruido(s));
        }
        Adsr(a, dc, s, rl) => {
            // [etapa, nivel, paso al soltar]; etapa 0 ataque, 1 caída, 2 soltando.
            let (a, dc, s, rl) = (e(a)[0].max(0.0005), e(dc)[0].max(0.001), e(s)[0].clamp(0.0, 1.0), e(rl)[0].max(0.002));
            let [etapa, nivel, baja] = &mut mem[off..off + 3] else { unreachable!() };
            if !d.puerta && *etapa < 2.0 {
                *etapa = 2.0;
                *baja = *nivel / (rl * sr);
            }
            let sube = 1.0 / (a * sr);
            // Caída exponencial: llega al 1 % de lo que falta en `dc` segundos.
            let cae = 1.0 - (-4.6 / (dc * sr)).exp();
            for o in o.iter_mut() {
                if *etapa == 0.0 {
                    *nivel += sube;
                    if *nivel >= 1.0 {
                        *nivel = 1.0;
                        *etapa = 1.0;
                    }
                } else if *etapa == 1.0 {
                    *nivel += (s - *nivel) * cae;
                } else {
                    *nivel = (*nivel - *baja).max(0.0);
                }
                *o = *nivel;
            }
        }
        Golpe(s) => {
            // Guarda lo ya caído, así empieza en 0 y la memoria limpia sirve.
            let k = (-6.9 / (e(s)[0].max(0.001) * sr)).exp();
            let mut v = 1.0 - mem[off];
            for o in o.iter_mut() {
                *o = v;
                v *= k;
            }
            mem[off] = 1.0 - v;
        }
        Svf(tipo, x, hz, res) => {
            let (x, hz, res) = (e(x), e(hz), e(res));
            let [s1, s2] = &mut mem[off..off + 2] else { unreachable!() };
            // Los coeficientes se calculan cada 16 cuadros: no se oye y ahorra casi toda la tangente.
            let mut k = 0;
            while k < n {
                let fin = (k + 16).min(n);
                let g = (std::f32::consts::PI * (hz[k].clamp(15.0, sr * 0.45) / sr)).tan();
                let q = 2.0 - 1.95 * res[k].clamp(0.0, 1.0);
                let a1 = 1.0 / (1.0 + g * (g + q));
                let (a2, a3) = (g * a1, g * g * a1);
                for j in k..fin {
                    let v3 = x[j] - *s2;
                    let v1 = a1 * *s1 + a2 * v3;
                    let v2 = *s2 + a2 * *s1 + a3 * v3;
                    *s1 = 2.0 * v1 - *s1;
                    *s2 = 2.0 * v2 - *s2;
                    o[j] = match tipo {
                        Filtro::Bajo => v2,
                        Filtro::Banda => v1,
                        Filtro::Alto => x[j] - q * v1 - v2,
                    };
                }
                k = fin;
            }
            if !s1.is_finite() || !s2.is_finite() {
                (*s1, *s2) = (0.0, 0.0);
            }
        }
        Eq(tipo, x, hz, db, q) => {
            let x = e(x);
            let k = banda(tipo, e(hz)[0], e(db)[0], e(q)[0], sr);
            let [s1, s2] = &mut mem[off..off + 2] else { unreachable!() };
            for (o, x) in o.iter_mut().zip(x) {
                let y = k[0] * x + *s1;
                *s1 = k[1] * x - k[3] * y + *s2;
                *s2 = k[2] * x - k[4] * y;
                *o = y;
            }
            if !s1.is_finite() || !s2.is_finite() {
                (*s1, *s2) = (0.0, 0.0);
            }
        }
        Seguidor(x, a, s) => {
            let x = e(x);
            let paso = |t: f32| 1.0 - (-1.0 / (t.max(0.00005) * sr)).exp();
            let (ka, ks) = (paso(e(a)[0]), paso(e(s)[0]));
            let mut v = mem[off];
            for (o, x) in o.iter_mut().zip(x) {
                v += (x - v) * if *x > v { ka } else { ks };
                *o = v;
            }
            mem[off] = if v.is_finite() { v } else { 0.0 };
        }
        Escalon(x, n) => dos!(x, n, |v, n| {
            let n = n.max(1.0);
            (v * n).round() / n
        }),
        Retener(x, hz) => {
            let (x, hz) = (e(x), e(hz));
            let [fase, v] = &mut mem[off..off + 2] else { unreachable!() };
            for k in 0..n {
                *fase -= hz[k] / sr;
                if *fase <= 0.0 {
                    *fase = (*fase + 1.0).max(0.0);
                    *v = x[k];
                }
                o[k] = *v;
            }
        }
        Pasatodo(x, hz) => {
            let (x, hz) = (e(x), e(hz));
            let mut z = mem[off];
            for k in 0..n {
                let g = (std::f32::consts::PI * hz[k].clamp(10.0, sr * 0.45) / sr).tan();
                let c = (g - 1.0) / (g + 1.0);
                let y = c * x[k] + z;
                z = x[k] - c * y;
                o[k] = y;
            }
            mem[off] = if z.is_finite() { z } else { 0.0 };
        }
        Retardo(x, seg, real, max) => {
            let (x, seg, real) = (e(x), e(seg), e(real));
            let largo = (max * sr) as usize + 4;
            let (pos, l) = mem[off..off + 1 + largo].split_first_mut().unwrap();
            let mut p = *pos as usize;
            for k in 0..n {
                let dd = (seg[k] * sr).clamp(1.0, (largo - 3) as f32);
                let y = leer4(l, p, dd);
                l[p] = x[k] + y * real[k].clamp(-0.98, 0.98);
                o[k] = y;
                p += 1;
                if p == largo {
                    p = 0;
                }
            }
            *pos = p as f32;
        }
        Sala(x, tam, brillo) => {
            let x = e(x);
            let (tam, brillo) = (e(tam)[0].clamp(0.0, 1.0), e(brillo)[0].clamp(0.0, 1.0));
            let largo = (SALA_MAX * sr) as usize + 4;
            let (cab, lineas) = mem[off..off + 9 + 8 * largo].split_at_mut(9);
            let (pos, amort) = cab.split_first_mut().unwrap();
            let mut p = *pos as usize;
            let esc = 0.35 + 0.65 * tam;
            let dd: [usize; 8] = std::array::from_fn(|j| ((SALA[j] * esc * sr) as usize).clamp(1, largo - 2));
            // Segundos que tarda en apagarse.
            let cola = 0.3 + tam * tam * 6.0;
            let g: [f32; 8] = std::array::from_fn(|j| 10f32.powf(-3.0 * dd[j] as f32 / sr / cola));
            let kb = 0.15 + 0.85 * brillo;
            for k in 0..n {
                let mut y = [0.0f32; 8];
                for j in 0..8 {
                    let i = p + largo - dd[j];
                    let v = lineas[j * largo + if i >= largo { i - largo } else { i }];
                    amort[j] += (v - amort[j]) * kb;
                    y[j] = amort[j] * g[j];
                }
                // Matriz de Householder: mezcla las ocho líneas sin ganar energía.
                let s = y.iter().sum::<f32>() * 0.25;
                for j in 0..8 {
                    lineas[j * largo + p] = x[k] + y[j] - s;
                }
                o[k] = (y[0] - y[1] + y[2] - y[3] + y[4] - y[5] + y[6] - y[7]) * 0.35;
                p += 1;
                if p == largo {
                    p = 0;
                }
            }
            *pos = p as f32;
        }
        Cuerda(x, hz, caida, brillo) => {
            let (x, hz) = (e(x), e(hz));
            let (caida, brillo) = (e(caida)[0].max(0.02), e(brillo)[0].clamp(0.0, 1.0));
            let largo = (sr / 20.0) as usize + 4;
            let (cab, l) = mem[off..off + 2 + largo].split_at_mut(2);
            let mut p = cab[0] as usize;
            let kb = 0.2 + 0.8 * brillo;
            for k in 0..n {
                let f = hz[k].clamp(21.0, sr * 0.2);
                // El filtro atrasa: se descuenta para que afine.
                let dd = (sr / f - (1.0 - kb) / kb).clamp(2.0, (largo - 3) as f32);
                let g = 10f32.powf(-3.0 / (f * caida));
                let v = leer(l, p, dd);
                cab[1] += (v - cab[1]) * kb;
                l[p] = x[k] + cab[1] * g;
                o[k] = l[p];
                p += 1;
                if p == largo {
                    p = 0;
                }
            }
            cab[0] = p as f32;
        }
        Raiz(a) => uno!(a, |x| x.max(0.0).sqrt()),
        Log(a) => uno!(a, |x| x.max(1e-9).ln()),
        Exp(a) => uno!(a, |x| x.min(40.0).exp()),
        Piso(a) => uno!(a, |x| x.floor()),
        Mayor(a, c) => dos!(a, c, |x, y| if x > y { 1.0 } else { 0.0 }),
        Resto(a, c) => dos!(a, c, |x, y| if y.abs() < 1e-9 { 0.0 } else { x - y * (x / y).floor() }),
        Bpm => o.fill(if d.bpm > 0.0 { d.bpm } else { 120.0 }),
        Posicion => {
            let paso = if d.bpm > 0.0 { d.bpm } else { 120.0 } / 60.0 / sr;
            o.iter_mut().enumerate().for_each(|(k, o)| *o = d.negra + k as f32 * paso);
        }
        Curva(x, k, suave) => {
            let t = &r.tablas[k as usize];
            if suave {
                // El promedio de la curva entre una muestra y la anterior: raspa mucho menos al distorsionar.
                let mut antes = mem[off];
                for (o, x) in o.iter_mut().zip(e(x).iter().map(|x| x.clamp(-ALCANCE, ALCANCE))) {
                    let paso = x - antes;
                    *o = if paso.abs() < 1e-3 { t.leer((x + antes) * 0.5) } else { ((t.area(x) - t.area(antes)) / paso as f64) as f32 };
                    antes = x;
                }
                mem[off] = if antes.is_finite() { antes } else { 0.0 };
            } else {
                uno!(x, |x| t.leer(x));
            }
        }
        Propia(hz, fase, k) => {
            let (hz, fase) = (e(hz), e(fase));
            let t = &r.tablas[k as usize];
            let mut p = mem[off];
            for k in 0..n {
                let dt = (hz[k] / sr).clamp(0.0, 0.49);
                // La versión cuyos armónicos caben bajo la mitad de la frecuencia de muestreo.
                let mut nivel = 0;
                while nivel + 1 < t.v.len() && (PUNTOS >> (nivel + 1)) as f32 * dt > 0.5 {
                    nivel += 1;
                }
                let u = p + fase[k];
                let q = (u - u.floor()) * PUNTOS as f32;
                let (i, f) = (q as usize % PUNTOS, q - q.floor());
                let v = &t.v[nivel];
                o[k] = v[i] + (v[(i + 1) % PUNTOS] - v[i]) * f;
                p += dt;
                if p >= 1.0 {
                    p -= 1.0;
                }
            }
            mem[off] = p;
        }
        Lazo(seg, max) => {
            // Lee atrás al menos un bloque entero: así todo lo que lee ya está escrito.
            let seg = e(seg);
            let largo = (max * sr) as usize + BLOQUE + 4;
            let (pos, l) = mem[off..off + 1 + largo].split_first_mut().unwrap();
            let p = *pos as usize;
            for k in 0..n {
                let d = (seg[k] * sr).clamp(BLOQUE as f32, (largo - 3) as f32);
                o[k] = leer(l, (p + k) % largo, d);
            }
        }
        Mide(x, _) => {
            o.copy_from_slice(e(x));
            mem[off] = o[n - 1];
        }
        Convolucion(x, lado) => match m.and_then(|m| m.impulso.as_ref()) {
            Some(h) => h.tocar(lado as usize, e(x), o, &mut mem[off..off + crate::impulso::Impulso::memoria(sr)], sr),
            None => o.fill(0.0),
        },
        Cierra(lazo, x) => {
            let (Lazo(_, max), donde) = r.nodos[lazo as usize] else { return o.fill(0.0) };
            let largo = (max * sr) as usize + BLOQUE + 4;
            let (pos, l) = mem[donde as usize..donde as usize + 1 + largo].split_first_mut().unwrap();
            let mut p = *pos as usize;
            for v in e(x) {
                // Con tope: un lazo que suma más de lo que resta no revienta.
                l[p] = if v.is_finite() { v.clamp(-8.0, 8.0) } else { 0.0 };
                p = (p + 1) % largo;
            }
            *pos = p as f32;
            o.copy_from_slice(e(lazo));
        }
        Muestra(vel, canal) => {
            let vel = e(vel);
            o.fill(0.0);
            let Some(m) = m else { return };
            for c in 0..d.cuantas as usize {
                let z = &m.zonas[d.zonas[c] as usize];
                let lado = if canal == 0 { 1.0 - d.pad[2].max(0.0) } else { 1.0 + d.pad[2].min(0.0) };
                let gan = z.gan[canal as usize] * (1.0 + d.pad[0]) * lado;
                if gan == 0.0 {
                    continue;
                }
                // Entero y fracción por separado: un solo f32 pierde precisión en muestras largas.
                let pm = &mut mem[off + 2 * c..off + 2 * c + 2];
                let mut p = pm[0] as f64 + pm[1] as f64;
                // `velocidad` 1 es la nota 60; cada grabación tiene su raíz.
                let paso = (2f32.powf((60.0 - z.raiz) / 12.0) * z.sr / sr * (1.0 + d.pad[1])) as f64;
                let ini = z.ini + ((z.fin - z.ini) as f32 * d.desde) as usize;
                let x = &m.datos[ini..z.fin];
                for k in 0..n {
                    let j = p as usize;
                    if j + 1 >= x.len() {
                        break;
                    }
                    let f = (p - j as f64) as f32;
                    o[k] += (x[j] + (x[j + 1] - x[j]) * f) * gan;
                    p += paso * vel[k].max(0.0) as f64;
                    if let Some((a, b)) = z.bucle.filter(|b| b.0 >= ini) {
                        let (a, b) = ((a - ini) as f64, (b - ini) as f64);
                        if p >= b {
                            p = a + (p - b) % (b - a);
                        }
                    }
                }
                pm[0] = p.floor() as f32;
                pm[1] = p.fract() as f32;
            }
        }
    }
}

/// Apaga los números diminutos: hacen lentísimo al procesador al apagarse un sonido.
pub fn sin_denormales() {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let mut f: u64;
        std::arch::asm!("mrs {}, fpcr", out(reg) f);
        f |= 1 << 24;
        std::arch::asm!("msr fpcr, {}", in(reg) f);
    }
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let mut f: u32 = 0;
        std::arch::asm!("stmxcsr [{}]", in(reg) &mut f);
        f |= 0x8040;
        std::arch::asm!("ldmxcsr [{}]", in(reg) &f);
    }
}
