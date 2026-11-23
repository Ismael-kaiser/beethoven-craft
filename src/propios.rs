//! Efectos de audio hechos para Beethoven Craft.
use crate::efectos::Procesador;
use crate::modelo::{SR, db};
use std::f32::consts::{PI, TAU};

const S: f32 = SR as f32;

/// Filtro de segundo orden (fórmulas de R. Bristow-Johnson).
#[derive(Clone, Copy, Default)]
pub struct Bi {
    b: [f32; 3],
    a: [f32; 2],
    z: [[f32; 2]; 2],
}

impl Bi {
    fn poner(&mut self, b: [f32; 3], a: [f32; 3]) {
        self.b = b.map(|x| x / a[0]);
        self.a = [a[1] / a[0], a[2] / a[0]];
    }
    pub fn pasa_altos(&mut self, hz: f32, q: f32) {
        let w = TAU * hz.clamp(10.0, S * 0.45) / S;
        let (s, c) = w.sin_cos();
        let al = s / (2.0 * q);
        self.poner([(1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0], [1.0 + al, -2.0 * c, 1.0 - al]);
    }
    pub fn pasa_bajos(&mut self, hz: f32, q: f32) {
        let w = TAU * hz.clamp(10.0, S * 0.45) / S;
        let (s, c) = w.sin_cos();
        let al = s / (2.0 * q);
        self.poner([(1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0], [1.0 + al, -2.0 * c, 1.0 - al]);
    }
    pub fn pasa_banda(&mut self, hz: f32, q: f32) {
        let w = TAU * hz.clamp(10.0, S * 0.45) / S;
        let (s, c) = w.sin_cos();
        let al = s / (2.0 * q);
        self.poner([al, 0.0, -al], [1.0 + al, -2.0 * c, 1.0 - al]);
    }
    #[inline]
    pub fn paso(&mut self, k: usize, x: f32) -> f32 {
        let z = &mut self.z[k];
        let y = self.b[0] * x + z[0];
        z[0] = self.b[1] * x - self.a[0] * y + z[1];
        z[1] = self.b[2] * x - self.a[1] * y;
        if y.abs() < 1e-20 { 0.0 } else { y }
    }
    fn limpiar(&mut self) {
        self.z = [[0.0; 2]; 2];
    }
}

/// Cambia el tono leyendo una línea de retardo con dos cabezas que se turnan.
struct Tono {
    v: [Vec<f32>; 2],
    i: usize,
    fase: f32,
    vent: f32,
}

impl Tono {
    const N: usize = 8192;
    fn nuevo(vent_ms: f32) -> Self {
        Tono { v: [vec![0.0; Self::N], vec![0.0; Self::N]], i: 0, fase: 0.0, vent: vent_ms / 1000.0 * S }
    }
    fn leer(&self, k: usize, d: f32) -> f32 {
        let p = self.i as f32 + Self::N as f32 - d;
        let a = p.floor();
        let t = p - a;
        let m = Self::N - 1;
        let j = a as usize;
        self.v[k][j & m] * (1.0 - t) + self.v[k][(j + 1) & m] * t
    }
    fn paso(&mut self, x: [f32; 2], razon: f32) -> [f32; 2] {
        for k in 0..2 {
            self.v[k][self.i] = x[k];
        }
        self.fase += (1.0 - razon) / self.vent;
        self.fase -= self.fase.floor();
        let mut y = [0.0; 2];
        for h in 0..2 {
            let f = (self.fase + h as f32 * 0.5).fract();
            // Seno al cuadrado: las dos cabezas siempre suman uno.
            let g = (PI * f).sin().powi(2);
            for (k, y) in y.iter_mut().enumerate() {
                *y += self.leer(k, 2.0 + f * self.vent) * g;
            }
        }
        self.i = (self.i + 1) & (Self::N - 1);
        y
    }
    fn limpiar(&mut self) {
        self.v.iter_mut().for_each(|v| v.fill(0.0));
        self.fase = 0.0;
    }
}

/// Detecta el tono de una voz (YIN, A. de Cheveigné y H. Kawahara).
pub struct Detector {
    v: Vec<f32>,
    lleno: usize,
    i: usize,
    pub hz: Option<f32>,
}

impl Detector {
    /// Se analiza a la mitad de la frecuencia: basta para la voz y cuesta la cuarta parte.
    const N: usize = 1024;
    const SALTO: usize = 256;
    pub fn nuevo() -> Self {
        Detector { v: vec![0.0; Self::N], lleno: 0, i: 0, hz: None }
    }
    pub fn meter(&mut self, x: f32) {
        self.v[self.i] = x;
        self.i = (self.i + 1) % Self::N;
        self.lleno += 1;
        if self.lleno >= Self::SALTO {
            self.lleno = 0;
            self.hz = self.analizar();
        }
    }
    fn analizar(&self) -> Option<f32> {
        let sr = S / 2.0;
        let m = |k: usize| self.v[(self.i + k) % Self::N];
        let x: Vec<f32> = (0..Self::N).map(m).collect();
        if x.iter().map(|v| v * v).sum::<f32>() / (Self::N as f32) < 1e-6 {
            return None;
        }
        let max = (sr / 70.0) as usize;
        let min = (sr / 1000.0) as usize;
        let w = Self::N - max - 1;
        let mut d = vec![0f32; max + 1];
        for t in 1..=max {
            d[t] = (0..w).map(|j| (x[j] - x[j + t]).powi(2)).sum();
        }
        let mut suma = 0.0;
        for t in 1..=max {
            suma += d[t];
            d[t] = if suma > 0.0 { d[t] * t as f32 / suma } else { 1.0 };
        }
        let mut t = min;
        while t < max {
            if d[t] < 0.15 {
                while t + 1 < max && d[t + 1] < d[t] {
                    t += 1;
                }
                let (a, b, c) = (d[t - 1], d[t], d[t + 1]);
                let q = a - 2.0 * b + c;
                let aj = if q.abs() > 1e-9 { 0.5 * (a - c) / q } else { 0.0 };
                return Some(sr / (t as f32 + aj));
            }
            t += 1;
        }
        None
    }
}

pub const ESCALAS: [(&str, &[i32]); 5] = [
    ("Cromática", &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
    ("Mayor", &[0, 2, 4, 5, 7, 9, 11]),
    ("Menor", &[0, 2, 3, 5, 7, 8, 10]),
    ("Pentatónica mayor", &[0, 2, 4, 7, 9]),
    ("Pentatónica menor", &[0, 3, 5, 7, 10]),
];

/// Nota de la escala más cercana, en número MIDI.
pub fn nota_cercana(midi: f32, raiz: i32, escala: &[i32]) -> f32 {
    let c = midi.round() as i32;
    (c - 6..=c + 6)
        .filter(|n| escala.contains(&(n - raiz).rem_euclid(12)))
        .min_by(|a, b| (*a as f32 - midi).abs().total_cmp(&(*b as f32 - midi).abs()))
        .unwrap_or(c) as f32
}

/// Afinador de voz.
pub struct Afinador {
    d: Detector,
    t: Tono,
    razon: f32,
    par: bool,
    ant: f32,
    p: Vec<f32>,
}

impl Afinador {
    pub fn nuevo() -> Self {
        Afinador { d: Detector::nuevo(), t: Tono::nuevo(38.0), razon: 1.0, par: false, ant: 0.0, p: vec![0.0; 5] }
    }
}

impl Procesador for Afinador {
    fn procesar(&mut self, b: &mut [f32]) {
        // raíz, escala, rapidez (ms), cantidad, transporte
        let esc = ESCALAS[(self.p[1].round() as usize).min(ESCALAS.len() - 1)].1;
        let k = 1.0 - (-1.0 / (self.p[2].max(0.5) / 1000.0 * S)).exp();
        let mez = self.p[3] / 100.0;
        for f in b.chunks_exact_mut(2) {
            let m = 0.5 * (f[0] + f[1]);
            if self.par {
                self.d.meter(0.5 * (m + self.ant));
            }
            self.par = !self.par;
            self.ant = m;
            let meta = match self.d.hz {
                Some(hz) => {
                    let n = 69.0 + 12.0 * (hz / 440.0).log2();
                    let q = nota_cercana(n, self.p[0].round() as i32, esc);
                    2f32.powf(((q - n) * mez + self.p[4].round()) / 12.0)
                }
                None => 2f32.powf(self.p[4].round() / 12.0),
            };
            self.razon += (meta - self.razon) * k;
            let y = self.t.paso([f[0], f[1]], self.razon);
            f.copy_from_slice(&y);
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.p = p.to_vec();
    }
    fn limpiar(&mut self) {
        self.t.limpiar();
        self.razon = 1.0;
    }
    fn latencia(&self) -> usize {
        (self.t.vent / 2.0) as usize
    }
}

/// Cambio de tono en semitonos.
pub struct CambioTono {
    t: Tono,
    razon: f32,
    mez: f32,
}

impl CambioTono {
    pub fn nuevo() -> Self {
        CambioTono { t: Tono::nuevo(60.0), razon: 1.0, mez: 1.0 }
    }
}

impl Procesador for CambioTono {
    fn procesar(&mut self, b: &mut [f32]) {
        for f in b.chunks_exact_mut(2) {
            let y = self.t.paso([f[0], f[1]], self.razon);
            for k in 0..2 {
                f[k] = f[k] * (1.0 - self.mez) + y[k] * self.mez;
            }
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.razon = 2f32.powf((p[0].round() + p[1] / 100.0) / 12.0);
        self.mez = p[2] / 100.0;
    }
    fn limpiar(&mut self) {
        self.t.limpiar();
    }
    fn latencia(&self) -> usize {
        // Solo si no hay señal seca con la que deba coincidir.
        if self.mez > 0.99 { (self.t.vent / 2.0) as usize } else { 0 }
    }
}

/// Quita el silbido de las eses.
#[derive(Default)]
pub struct Seseo {
    alto: Bi,
    bajo: Bi,
    det: Bi,
    env: f32,
    umbral: f32,
    max: f32,
}

impl Procesador for Seseo {
    fn procesar(&mut self, b: &mut [f32]) {
        let (sube, baja) = (1.0 - (-1.0 / (0.001 * S)).exp(), 1.0 - (-1.0 / (0.06 * S)).exp());
        for f in b.chunks_exact_mut(2) {
            let d = self.det.paso(0, 0.5 * (f[0] + f[1])).abs();
            self.env += (d - self.env) * if d > self.env { sube } else { baja };
            let sobra = (20.0 * (self.env + 1e-9).log10() - self.umbral).max(0.0);
            let g = db(-(sobra * 0.75).min(self.max));
            for k in 0..2 {
                // Cruce Linkwitz-Riley de segundo orden: al restar, las dos bandas suman parejo.
                let (h, l) = (self.alto.paso(k, f[k]), self.bajo.paso(k, f[k]));
                f[k] = l - h * g;
            }
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.alto.pasa_altos(p[0], 0.5);
        self.bajo.pasa_bajos(p[0], 0.5);
        self.det.pasa_altos(p[0], 0.707);
        self.umbral = p[1];
        self.max = p[2];
    }
    fn limpiar(&mut self) {
        self.alto.limpiar();
        self.bajo.limpiar();
        self.det.limpiar();
        self.env = 0.0;
    }
}

/// Trémolo y autopaneo: el mismo oscilador, en volumen o de lado a lado.
pub struct Onda {
    pub paneo: bool,
    f: f32,
    hz: f32,
    prof: f32,
    cuadrada: f32,
}

impl Onda {
    pub fn nueva(paneo: bool) -> Self {
        Onda { paneo, f: 0.0, hz: 4.0, prof: 0.5, cuadrada: 0.0 }
    }
}

impl Procesador for Onda {
    fn procesar(&mut self, b: &mut [f32]) {
        for f in b.chunks_exact_mut(2) {
            self.f = (self.f + self.hz / S).fract();
            let s = (TAU * self.f).sin();
            // De senoidal a casi cuadrada.
            let o = (s * (1.0 + self.cuadrada * 12.0)).tanh() / (1.0 + self.cuadrada * 12.0).tanh();
            if self.paneo {
                let p = o * self.prof;
                f[0] *= (1.0 - p).min(1.0);
                f[1] *= (1.0 + p).min(1.0);
            } else {
                let g = 1.0 - self.prof * (0.5 + 0.5 * o);
                f[0] *= g;
                f[1] *= g;
            }
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.hz = p[0];
        self.prof = p[1] / 100.0;
        self.cuadrada = p[2] / 100.0;
    }
    fn limpiar(&mut self) {
        self.f = 0.0;
    }
}

/// Voz de robot: modulación en anillo.
#[derive(Default)]
pub struct Anillo {
    f: f32,
    hz: f32,
    mez: f32,
}

impl Procesador for Anillo {
    fn procesar(&mut self, b: &mut [f32]) {
        for f in b.chunks_exact_mut(2) {
            self.f = (self.f + self.hz / S).fract();
            let o = (TAU * self.f).sin();
            f.iter_mut().for_each(|x| *x = *x * (1.0 - self.mez) + *x * o * self.mez * 1.4);
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.hz = p[0];
        self.mez = p[1] / 100.0;
    }
    fn limpiar(&mut self) {
        self.f = 0.0;
    }
}

/// Menos bits y menos muestras.
#[derive(Default)]
pub struct Bits {
    niveles: f32,
    paso: f32,
    fase: f32,
    u: [f32; 2],
    mez: f32,
}

impl Procesador for Bits {
    fn procesar(&mut self, b: &mut [f32]) {
        for f in b.chunks_exact_mut(2) {
            self.fase += self.paso;
            if self.fase >= 1.0 {
                self.fase -= 1.0;
                for k in 0..2 {
                    self.u[k] = (f[k] * self.niveles).round() / self.niveles;
                }
            }
            for k in 0..2 {
                f[k] = f[k] * (1.0 - self.mez) + self.u[k] * self.mez;
            }
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.niveles = 2f32.powf(p[0] - 1.0);
        self.paso = (p[1] / S).min(1.0);
        self.mez = p[2] / 100.0;
    }
    fn limpiar(&mut self) {
        self.fase = 1.0;
    }
}

/// Wah: filtro que sigue la fuerza o un vaivén.
#[derive(Default)]
pub struct Wah {
    f: Bi,
    env: f32,
    fase: f32,
    n: u32,
    p: [f32; 5],
}

impl Procesador for Wah {
    fn procesar(&mut self, b: &mut [f32]) {
        let mez = self.p[4] / 100.0;
        for f in b.chunks_exact_mut(2) {
            let m = (0.5 * (f[0] + f[1])).abs();
            self.env += (m - self.env) * if m > self.env { 0.004 } else { 0.0004 };
            self.fase = (self.fase + self.p[1] / S).fract();
            if self.n % 16 == 0 {
                let vaiven = 0.5 + 0.5 * (TAU * self.fase).sin();
                let k = (self.env * self.p[2] / 8.0).min(1.0) + vaiven * self.p[3] / 100.0;
                self.f.pasa_banda(self.p[0] * 12f32.powf(k.min(1.2)), 3.5);
            }
            self.n = self.n.wrapping_add(1);
            for k in 0..2 {
                let y = self.f.paso(k, f[k]) * 2.2;
                f[k] = f[k] * (1.0 - mez) + y * mez;
            }
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.p.copy_from_slice(&p[..5]);
    }
    fn limpiar(&mut self) {
        self.f.limpiar();
        self.env = 0.0;
    }
}

/// Dobla la voz: dos copias un poco desafinadas y retrasadas, a los lados.
pub struct Doblador {
    t: [Tono; 2],
    r: [f32; 2],
    v: [Vec<f32>; 2],
    i: usize,
    ret: usize,
    mez: f32,
}

impl Doblador {
    pub fn nuevo() -> Self {
        Doblador { t: [Tono::nuevo(45.0), Tono::nuevo(52.0)], r: [1.0; 2], v: [vec![0.0; 4096], vec![0.0; 4096]], i: 0, ret: 700, mez: 0.5 }
    }
}

impl Procesador for Doblador {
    fn procesar(&mut self, b: &mut [f32]) {
        for f in b.chunks_exact_mut(2) {
            let m = 0.5 * (f[0] + f[1]);
            for k in 0..2 {
                let y = self.t[k].paso([m, 0.0], self.r[k])[0];
                let j = (self.i + 4096 - self.ret - k * 190) & 4095;
                f[k] += self.v[k][j] * self.mez;
                self.v[k][self.i] = y;
            }
            self.i = (self.i + 1) & 4095;
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.r = [2f32.powf(p[0] / 1200.0), 2f32.powf(-p[0] / 1200.0)];
        self.ret = ((p[1] / 1000.0 * S) as usize).clamp(1, 3800);
        self.mez = p[2] / 100.0;
    }
    fn limpiar(&mut self) {
        self.t.iter_mut().for_each(Tono::limpiar);
        self.v.iter_mut().for_each(|v| v.fill(0.0));
    }
}

/// Brillo: armónicos nuevos en los agudos.
#[derive(Default)]
pub struct Excitador {
    h: Bi,
    h2: Bi,
    emp: f32,
    mez: f32,
}

impl Procesador for Excitador {
    fn procesar(&mut self, b: &mut [f32]) {
        for f in b.chunks_exact_mut(2) {
            for k in 0..2 {
                let a = self.h.paso(k, f[k]);
                let y = (a * self.emp).tanh();
                f[k] += self.h2.paso(k, y) * self.mez;
            }
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.h.pasa_altos(p[0], 0.707);
        self.h2.pasa_altos(p[0], 0.707);
        self.emp = 1.0 + p[1] / 8.0;
        self.mez = p[2] / 100.0 * 0.6;
    }
    fn limpiar(&mut self) {
        self.h.limpiar();
        self.h2.limpiar();
    }
}

/// Karaoke: quita lo que está al centro (casi siempre la voz) y conserva los graves.
#[derive(Default)]
pub struct Karaoke {
    bajo: Bi,
    alto: Bi,
    cant: f32,
}

impl Procesador for Karaoke {
    fn procesar(&mut self, b: &mut [f32]) {
        for f in b.chunks_exact_mut(2) {
            let m = 0.5 * (f[0] + f[1]);
            // Banda de la voz dentro del centro.
            let medio = self.alto.paso(0, m);
            let voz = self.bajo.paso(0, medio);
            f[0] -= voz * self.cant;
            f[1] -= voz * self.cant;
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.alto.pasa_altos(p[0], 0.707);
        self.bajo.pasa_bajos(p[1], 0.707);
        self.cant = p[2] / 100.0;
    }
    fn limpiar(&mut self) {
        self.bajo.limpiar();
        self.alto.limpiar();
    }
}

/// Coros dentro de la escala: dos voces a tantos grados de la tuya.
pub struct Armonias {
    d: Detector,
    t: [Tono; 2],
    razon: [f32; 2],
    par: bool,
    ant: f32,
    p: Vec<f32>,
}

impl Armonias {
    pub fn nuevo() -> Self {
        Armonias { d: Detector::nuevo(), t: [Tono::nuevo(42.0), Tono::nuevo(47.0)], razon: [1.0; 2], par: false, ant: 0.0, p: vec![0.0; 7] }
    }
}

/// Semitonos que hay que subir desde `midi` para llegar `grados` arriba en la escala.
pub fn intervalo(midi: f32, raiz: i32, escala: &[i32], grados: i32) -> f32 {
    let n = nota_cercana(midi, raiz, escala) as i32;
    let dentro = |x: i32| escala.contains(&(x - raiz).rem_euclid(12));
    let (mut x, mut g) = (n, grados.abs());
    while g > 0 {
        x += grados.signum();
        if dentro(x) {
            g -= 1;
        }
    }
    (x - n) as f32
}

impl Procesador for Armonias {
    fn procesar(&mut self, b: &mut [f32]) {
        let esc = ESCALAS[(self.p[1].round() as usize).min(ESCALAS.len() - 1)].1;
        let raiz = self.p[0].round() as i32;
        let k = 1.0 - (-1.0 / (0.02 * S)).exp();
        let abrir = self.p[6] / 100.0;
        for f in b.chunks_exact_mut(2) {
            let m = 0.5 * (f[0] + f[1]);
            if self.par {
                self.d.meter(0.5 * (m + self.ant));
            }
            self.par = !self.par;
            self.ant = m;
            for v in 0..2 {
                // Sin nota clara se queda con la última.
                if let Some(hz) = self.d.hz {
                    let n = 69.0 + 12.0 * (hz / 440.0).log2();
                    let meta = 2f32.powf(intervalo(n, raiz, esc, self.p[2 + v].round() as i32) / 12.0);
                    self.razon[v] += (meta - self.razon[v]) * k;
                }
                let y = self.t[v].paso([m, m], self.razon[v]);
                let g = self.p[4 + v] / 100.0;
                let (l, r) = if v == 0 { (1.0, 1.0 - abrir) } else { (1.0 - abrir, 1.0) };
                f[0] += y[0] * g * l;
                f[1] += y[1] * g * r;
            }
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.p = p.to_vec();
    }
    fn limpiar(&mut self) {
        self.t.iter_mut().for_each(Tono::limpiar);
        self.razon = [1.0; 2];
    }
}

/// Retardo que sube y baja: cambia el tono un poquito.
pub struct Vibrato {
    t: Tono,
    f: f32,
    hz: f32,
    prof: f32,
}

impl Vibrato {
    pub fn nuevo() -> Self {
        Vibrato { t: Tono::nuevo(20.0), f: 0.0, hz: 5.0, prof: 0.0 }
    }
}

impl Procesador for Vibrato {
    fn procesar(&mut self, b: &mut [f32]) {
        // El retardo que da ese cambio de tono: derivada máxima = razón - 1.
        let amp = (2f32.powf(self.prof / 1200.0) - 1.0) * S / (TAU * self.hz);
        let amp = amp.min(3000.0);
        for f in b.chunks_exact_mut(2) {
            self.f = (self.f + self.hz / S).fract();
            for k in 0..2 {
                self.t.v[k][self.t.i] = f[k];
                f[k] = self.t.leer(k, 2.0 + amp * (1.0 + (TAU * self.f).sin()));
            }
            self.t.i = (self.t.i + 1) & (Tono::N - 1);
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.hz = p[0].max(0.05);
        self.prof = p[1];
    }
    fn limpiar(&mut self) {
        self.t.limpiar();
        self.f = 0.0;
    }
}

/// Banda angosta y rasposa.
#[derive(Default)]
pub struct Telefono {
    a: [Bi; 2],
    b: [Bi; 2],
    ras: f32,
    mez: f32,
}

impl Procesador for Telefono {
    fn procesar(&mut self, b: &mut [f32]) {
        for f in b.chunks_exact_mut(2) {
            for k in 0..2 {
                let mut y = self.a[0].paso(k, f[k]);
                y = self.a[1].paso(k, y);
                let e = 1.0 + 12.0 * self.ras;
                y = (y * e).tanh() / e.sqrt();
                y = self.b[0].paso(k, y);
                y = self.b[1].paso(k, y);
                f[k] += (y * 1.3 - f[k]) * self.mez;
            }
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.a.iter_mut().for_each(|x| x.pasa_altos(p[0], 0.8));
        self.b.iter_mut().for_each(|x| x.pasa_bajos(p[1], 0.8));
        self.ras = p[2] / 100.0;
        self.mez = p[3] / 100.0;
    }
    fn limpiar(&mut self) {
        self.a.iter_mut().chain(&mut self.b).for_each(Bi::limpiar);
    }
}

/// Disco viejo.
pub struct Vinilo {
    t: Tono,
    f: f32,
    s: u32,
    lp: Bi,
    chas: f32,
    p: Vec<f32>,
}

impl Vinilo {
    pub fn nuevo() -> Self {
        Vinilo { t: Tono::nuevo(20.0), f: 0.0, s: 0x2545_F491, lp: Bi::default(), chas: 0.0, p: vec![0.0; 4] }
    }
    fn azar(&mut self) -> f32 {
        self.s ^= self.s << 13;
        self.s ^= self.s >> 17;
        self.s ^= self.s << 5;
        self.s as f32 / u32::MAX as f32
    }
}

impl Procesador for Vinilo {
    fn procesar(&mut self, b: &mut [f32]) {
        let amp = self.p[0] / 100.0 * 220.0;
        let (sis, cha) = (self.p[1] / 100.0 * 0.012, self.p[2] / 100.0);
        for f in b.chunks_exact_mut(2) {
            // 33⅓ vueltas por minuto, más un temblor rápido.
            self.f = (self.f + 0.5556 / S).fract();
            let d = 2.0 + amp * (1.0 + (TAU * self.f).sin() + 0.06 * (TAU * self.f * 11.0).sin());
            if self.azar() < cha * 9.0 / S {
                self.chas = (0.08 + 0.3 * self.azar()) * if self.azar() > 0.5 { 1.0 } else { -1.0 };
            }
            let ruido = (self.azar() - 0.5) * sis + self.chas * cha;
            self.chas *= 0.86;
            for k in 0..2 {
                self.t.v[k][self.t.i] = f[k];
                let y = self.t.leer(k, d);
                f[k] = self.lp.paso(k, y) + ruido;
            }
            self.t.i = (self.t.i + 1) & (Tono::N - 1);
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.p = p.to_vec();
        self.lp.pasa_bajos(18000.0 * (2500.0f32 / 18000.0).powf(p[3] / 100.0), 0.707);
    }
    fn limpiar(&mut self) {
        self.t.limpiar();
        self.lp.limpiar();
        self.chas = 0.0;
    }
}

/// Compuerta con ritmo.
#[derive(Default)]
pub struct Tartamudeo {
    f: f32,
    g: f32,
    hz: f32,
    abierto: f32,
    k: f32,
    prof: f32,
}

impl Procesador for Tartamudeo {
    fn procesar(&mut self, b: &mut [f32]) {
        for f in b.chunks_exact_mut(2) {
            let meta = if self.f < self.abierto { 1.0 } else { 1.0 - self.prof };
            self.g += (meta - self.g) * self.k;
            f[0] *= self.g;
            f[1] *= self.g;
            self.f = (self.f + self.hz / S).fract();
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.hz = p[0] / 60.0 * p[1].round().max(1.0);
        self.abierto = p[2] / 100.0;
        self.k = 1.0 - (-1.0 / (p[3] / 1000.0 * S)).exp();
        self.prof = p[4] / 100.0;
    }
    fn limpiar(&mut self) {
        self.f = 0.0;
        self.g = 1.0;
    }
}

/// Una octava abajo (divisor de frecuencia) y armónicos de los graves.
#[derive(Default)]
pub struct SubGrave {
    ent: Bi,
    sal: Bi,
    arm: Bi,
    arriba: bool,
    signo: f32,
    sub: f32,
    har: f32,
}

impl Procesador for SubGrave {
    fn procesar(&mut self, b: &mut [f32]) {
        for f in b.chunks_exact_mut(2) {
            let g = self.ent.paso(0, 0.5 * (f[0] + f[1]));
            // Cambia de signo en cada cruce hacia arriba: mitad de frecuencia.
            let a = g > 0.0;
            if a && !self.arriba {
                self.signo = if self.signo > 0.0 { -1.0 } else { 1.0 };
            }
            self.arriba = a;
            let s = self.sal.paso(0, g.abs() * self.signo);
            let h = self.arm.paso(0, (g * 4.0).tanh() - g * 0.5);
            let y = s * self.sub * 1.6 + h * self.har * 0.5;
            f[0] += y;
            f[1] += y;
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        self.ent.pasa_bajos(p[0], 0.707);
        self.sal.pasa_bajos(p[0] * 0.8, 0.707);
        self.arm.pasa_altos(p[0], 0.707);
        self.sub = p[1] / 100.0;
        self.har = p[2] / 100.0;
    }
    fn limpiar(&mut self) {
        self.ent.limpiar();
        self.sal.limpiar();
        self.arm.limpiar();
        self.signo = 1.0;
        self.arriba = false;
    }
}
