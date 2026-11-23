//! Efectos de video hechos para Beethoven Craft (identificador `bc:…`).
//! Trabajan en color sRGB sin premultiplicar sobre la capa completa.
use crate::modelo::{FxVideo, Prop, Valor};
use effectcraft_raster::Image;
use rayon::prelude::*;

type P4 = [f32; 4];

pub enum Ctl {
    /// Nombre, mínimo, máximo, inicial, unidad.
    N(&'static str, &'static str, f64, f64, f64, &'static str),
    C(&'static str, &'static str, [f32; 4]),
    /// Lista de opciones.
    O(&'static str, &'static str, &'static [&'static str]),
    /// Rueda de color; su brillo es el control que sigue, `<id>_l`.
    R(&'static str, &'static str),
}
use Ctl::*;

pub struct Def {
    pub id: &'static str,
    pub nombre: &'static str,
    pub grupo: &'static str,
    pub ctl: &'static [Ctl],
}

const fn n(id: &'static str, nombre: &'static str, min: f64, max: f64, def: f64, u: &'static str) -> Ctl {
    N(id, nombre, min, max, def, u)
}
const BPM: Ctl = n("bpm", "Tempo", 40.0, 240.0, 120.0, "bpm");
const CADA: Ctl = n("cada", "Cada cuántos golpes", 1.0, 8.0, 1.0, "");
const DESF: Ctl = n("desf", "Mover el golpe", -1500.0, 1500.0, 0.0, "ms");
const MEZ: Ctl = n("mezcla", "Mezcla", 0.0, 100.0, 100.0, "%");
const VEL: Ctl = n("vel", "Velocidad", 0.0, 400.0, 100.0, "%");
const CANT: Ctl = n("cant", "Cantidad", 5.0, 400.0, 100.0, "%");
const TAM: Ctl = n("tam", "Tamaño", 10.0, 400.0, 100.0, "%");

const RI: &str = "Al ritmo";
const PA: &str = "Pantallas y cámaras";
const AD: &str = "Adornos";
const CO: &str = "Cómic y color";
const ES: &str = "Espejos y ondas";

pub const DEFS: [Def; 25] = [
    Def {
        id: "bc:color",
        nombre: "Mezcla de color",
        grupo: CO,
        ctl: &[
            R("sombras", "Sombras"),
            n("sombras_l", "Brillo de sombras", -100.0, 100.0, 0.0, ""),
            R("medios", "Medios"),
            n("medios_l", "Brillo de medios", -100.0, 100.0, 0.0, ""),
            R("luces", "Luces"),
            n("luces_l", "Brillo de luces", -100.0, 100.0, 0.0, ""),
            n("exp", "Exposición", -4.0, 4.0, 0.0, ""),
            n("con", "Contraste", -100.0, 100.0, 0.0, ""),
            n("sat", "Saturación", 0.0, 300.0, 100.0, "%"),
            n("vib", "Intensidad", -100.0, 100.0, 0.0, ""),
            n("temp", "Temperatura", -100.0, 100.0, 0.0, ""),
            n("tinte", "Tinte", -100.0, 100.0, 0.0, ""),
            n("giro", "Girar tonos", -180.0, 180.0, 0.0, "°"),
            n("deslavar", "Deslavado", 0.0, 100.0, 0.0, "%"),
            n("vin", "Viñeta", 0.0, 100.0, 0.0, "%"),
            n("grano", "Grano", 0.0, 100.0, 0.0, "%"),
            MEZ,
        ],
    },
    Def { id: "bc:ritmo_zoom", nombre: "Zoom al ritmo", grupo: RI, ctl: &[BPM, CADA, DESF, n("fuerza", "Fuerza", 0.0, 60.0, 12.0, "%"), n("caida", "Caída", 40.0, 800.0, 180.0, "ms")] },
    Def { id: "bc:ritmo_destello", nombre: "Destello al ritmo", grupo: RI, ctl: &[BPM, CADA, DESF, n("fuerza", "Fuerza", 0.0, 100.0, 70.0, "%"), n("caida", "Caída", 40.0, 800.0, 160.0, "ms"), C("color", "Color", [1.0, 1.0, 1.0, 1.0])] },
    Def { id: "bc:ritmo_color", nombre: "Cambio de color al ritmo", grupo: RI, ctl: &[BPM, CADA, DESF, n("paso", "Giro por golpe", 0.0, 360.0, 60.0, "°")] },
    Def { id: "bc:ritmo_sacudida", nombre: "Sacudida al ritmo", grupo: RI, ctl: &[BPM, CADA, DESF, n("fuerza", "Fuerza", 0.0, 200.0, 45.0, ""), n("caida", "Caída", 40.0, 800.0, 200.0, "ms")] },
    Def { id: "bc:ritmo_rgb", nombre: "Separar RGB al ritmo", grupo: RI, ctl: &[BPM, CADA, DESF, n("fuerza", "Fuerza", 0.0, 200.0, 50.0, ""), n("caida", "Caída", 40.0, 800.0, 220.0, "ms")] },
    Def { id: "bc:ritmo_vueltas", nombre: "Volteo al ritmo", grupo: RI, ctl: &[BPM, CADA, DESF] },
    Def { id: "bc:tv", nombre: "Televisión vieja", grupo: PA, ctl: &[n("curva", "Curvatura", 0.0, 100.0, 40.0, "%"), n("lineas", "Líneas", 0.0, 100.0, 55.0, "%"), n("vin", "Orillas oscuras", 0.0, 100.0, 60.0, "%")] },
    Def { id: "bc:cinta", nombre: "Cinta dañada", grupo: PA, ctl: &[n("cant", "Daño", 0.0, 200.0, 100.0, "%"), VEL] },
    Def { id: "bc:nocturna", nombre: "Visión nocturna", grupo: PA, ctl: &[n("brillo", "Ganancia", 50.0, 400.0, 180.0, "%"), n("ruido", "Ruido", 0.0, 100.0, 35.0, "%"), C("color", "Color", [0.25, 1.0, 0.35, 1.0])] },
    Def { id: "bc:termica", nombre: "Cámara térmica", grupo: PA, ctl: &[MEZ] },
    Def { id: "bc:consola", nombre: "Consola de bolsillo", grupo: PA, ctl: &[n("tam", "Pixel", 2.0, 60.0, 12.0, ""), MEZ] },
    Def { id: "bc:barras", nombre: "Barras de cine", grupo: PA, ctl: &[n("razon", "Formato", 1.0, 3.0, 2.39, ":1"), n("abrir", "Abiertas", 0.0, 100.0, 0.0, "%")] },
    Def { id: "bc:marco", nombre: "Marco de foto", grupo: PA, ctl: &[n("tam", "Orilla", 1.0, 20.0, 5.0, "%"), n("pie", "Pie", 0.0, 30.0, 12.0, "%"), C("color", "Color", [0.97, 0.96, 0.92, 1.0])] },
    Def { id: "bc:puntos", nombre: "Puntos de imprenta", grupo: CO, ctl: &[n("tam", "Tamaño", 3.0, 60.0, 14.0, ""), n("ang", "Ángulo", 0.0, 90.0, 25.0, "°"), MEZ] },
    Def { id: "bc:manga", nombre: "Manga", grupo: CO, ctl: &[n("tinta", "Tinta", 0.0, 100.0, 60.0, "%"), n("tam", "Trama", 3.0, 40.0, 9.0, "")] },
    Def { id: "bc:duotono", nombre: "Dos tonos", grupo: CO, ctl: &[C("osc", "Sombras", [0.16, 0.05, 0.42, 1.0]), C("cla", "Luces", [1.0, 0.78, 0.35, 1.0]), MEZ] },
    Def { id: "bc:neblina", nombre: "Neblina", grupo: AD, ctl: &[n("cant", "Densidad", 0.0, 100.0, 45.0, "%"), VEL, C("color", "Color", [0.85, 0.88, 0.95, 1.0])] },
    Def { id: "bc:velocidad", nombre: "Líneas de velocidad", grupo: AD, ctl: &[n("cant", "Cantidad", 0.0, 100.0, 55.0, "%"), n("hueco", "Centro libre", 5.0, 90.0, 40.0, "%"), VEL, C("color", "Color", [1.0, 1.0, 1.0, 1.0])] },
    Def { id: "bc:confeti", nombre: "Confeti", grupo: AD, ctl: &[CANT, TAM, VEL] },
    Def { id: "bc:corazones", nombre: "Corazones", grupo: AD, ctl: &[CANT, TAM, VEL, C("color", "Color", [1.0, 0.25, 0.42, 1.0])] },
    Def { id: "bc:luciernagas", nombre: "Luciérnagas", grupo: AD, ctl: &[CANT, TAM, VEL, C("color", "Color", [1.0, 0.85, 0.4, 1.0])] },
    Def { id: "bc:prisma", nombre: "Prisma", grupo: ES, ctl: &[n("cant", "Separación", 0.0, 100.0, 25.0, "%"), n("caras", "Caras", 2.0, 8.0, 5.0, ""), VEL, MEZ] },
    Def { id: "bc:tunel", nombre: "Túnel infinito", grupo: ES, ctl: &[n("paso", "Escala por vuelta", 30.0, 90.0, 62.0, "%"), n("giro", "Giro por vuelta", -90.0, 90.0, 0.0, "°"), VEL] },
    Def { id: "bc:agua", nombre: "Bajo el agua", grupo: ES, ctl: &[n("cant", "Fuerza", 0.0, 100.0, 35.0, "%"), n("tam", "Tamaño de ola", 2.0, 40.0, 12.0, "%"), VEL] },
];

/// Los que no están en el catálogo de efectos: la mezcla de color y los nodos.
pub const DEFS_NODO: [Def; 4] = [
    Def { id: "nodo:fusionar", nombre: "Fusionar", grupo: "Nodos", ctl: &[O("modo", "Modo", &["Normal", "Multiplicar", "Pantalla", "Superponer", "Luz suave", "Luz dura", "Sumar", "Aclarar", "Oscurecer", "Diferencia", "Tono", "Color"]), n("opacidad", "Opacidad", 0.0, 100.0, 100.0, "%")] },
    Def { id: "nodo:mascara", nombre: "Máscara", grupo: "Nodos", ctl: &[O("forma", "Forma", &["Elipse", "Rectángulo", "Degradado"]), n("x", "Posición X", -100.0, 100.0, 0.0, "%"), n("y", "Posición Y", -100.0, 100.0, 0.0, "%"), n("ancho", "Ancho", 1.0, 400.0, 70.0, "%"), n("alto", "Alto", 1.0, 300.0, 70.0, "%"), n("giro", "Giro", -180.0, 180.0, 0.0, "°"), n("suave", "Suavidad", 0.0, 100.0, 30.0, "%"), n("fuerza", "Fuerza", 0.0, 100.0, 100.0, "%"), O("invertir", "Invertir", &["No", "Sí"])] },
    Def { id: "nodo:calificador", nombre: "Calificador de color", grupo: "Nodos", ctl: &[n("tono", "Tono", 0.0, 360.0, 20.0, "°"), n("ancho", "Amplitud", 1.0, 360.0, 50.0, "°"), n("sat_min", "Saturación desde", 0.0, 100.0, 15.0, "%"), n("sat_max", "Saturación hasta", 0.0, 100.0, 100.0, "%"), n("luz_min", "Brillo desde", 0.0, 100.0, 0.0, "%"), n("luz_max", "Brillo hasta", 0.0, 100.0, 100.0, "%"), n("suave", "Suavidad", 0.0, 100.0, 15.0, "%"), O("invertir", "Invertir", &["No", "Sí"])] },
    Def { id: "nodo:fondo", nombre: "Color", grupo: "Nodos", ctl: &[C("color", "Color", [0.1, 0.1, 0.1, 1.0])] },
];

pub fn buscar(tipo: &str) -> Option<&'static Def> {
    DEFS.iter().chain(&DEFS_NODO).find(|d| d.id == tipo)
}

pub fn nuevo(id: u64, d: &Def) -> FxVideo {
    let p = d
        .ctl
        .iter()
        .map(|c| {
            let (k, v) = match c {
                N(k, _, _, _, x, _) => (*k, Valor::Num(*x)),
                C(k, _, x) => (*k, Valor::Color(*x)),
                O(k, _, _) => (*k, Valor::Op(0)),
                R(k, _) => (*k, Valor::Punto([0.0; 2])),
            };
            (k.to_string(), Prop { v, claves: vec![], curvas: vec![], libre: false })
        })
        .collect();
    FxVideo { id, tipo: d.id.into(), activo: true, p }
}

/// Pone el tempo del proyecto y alinea los golpes con la rejilla.
pub fn a_tempo(f: &mut FxVideo, bpm: f64, inicio: f64) {
    let golpe = 60.0 / bpm.max(1.0);
    for (k, v) in [("bpm", bpm), ("desf", -(inicio.rem_euclid(golpe)) * 1000.0)] {
        if let (true, Some(p)) = (f.tipo.starts_with("bc:ritmo"), f.p.get_mut(k)) {
            p.v = Valor::Num(v);
        }
    }
}

// ---- ayudas

fn mez(a: P4, b: P4, k: f32) -> P4 {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * k)
}

/// Pinta `c` encima, sin tocar la transparencia salvo donde haga falta.
fn sobre(a: P4, c: [f32; 4], k: f32) -> P4 {
    let k = k.clamp(0.0, 1.0);
    [a[0] + (c[0] - a[0]) * k, a[1] + (c[1] - a[1]) * k, a[2] + (c[2] - a[2]) * k, a[3].max(k)]
}

fn luz(p: P4) -> f32 {
    0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]
}

fn azar(a: i32, b: i32) -> f32 {
    let mut h = (a as u32).wrapping_mul(0x9E37_79B1) ^ (b as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 13;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 16;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

fn ruido(x: f32, y: f32) -> f32 {
    let (i, j) = (x.floor(), y.floor());
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (u, v) = (s(x - i), s(y - j));
    let (i, j) = (i as i32, j as i32);
    let a = azar(i, j) + (azar(i + 1, j) - azar(i, j)) * u;
    let b = azar(i, j + 1) + (azar(i + 1, j + 1) - azar(i, j + 1)) * u;
    a + (b - a) * v
}

fn nubes(x: f32, y: f32) -> f32 {
    0.5 * ruido(x, y) + 0.25 * ruido(x * 2.03 + 7.0, y * 2.03) + 0.15 * ruido(x * 4.1, y * 4.1 + 3.0) + 0.1 * ruido(x * 8.3, y * 8.3)
}

fn escalon(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn arcoiris(h: f32) -> [f32; 4] {
    let f = |d: f32| ((h + d).rem_euclid(1.0) * 6.0 - 3.0).abs().clamp(1.0, 2.0) - 1.0;
    [f(0.0), f(2.0 / 3.0), f(1.0 / 3.0), 1.0]
}

/// Gira el tono conservando el brillo (matriz de rotación sobre el eje gris).
fn girar_tono(p: P4, ang: f32) -> P4 {
    let (s, c) = ang.sin_cos();
    let (k, q) = ((1.0 - c) / 3.0, s / 3f32.sqrt());
    let [r, g, b, a] = p;
    let o = [r * (c + k) + g * (k - q) + b * (k + q), r * (k + q) + g * (c + k) + b * (k - q), r * (k - q) + g * (k + q) + b * (c + k)];
    [o[0].clamp(0.0, 1.0), o[1].clamp(0.0, 1.0), o[2].clamp(0.0, 1.0), a]
}

struct Img {
    w: usize,
    h: usize,
    d: Vec<P4>,
}

impl Img {
    fn px(&self, x: i32, y: i32) -> P4 {
        let x = x.clamp(0, self.w as i32 - 1) as usize;
        let y = y.clamp(0, self.h as i32 - 1) as usize;
        self.d[y * self.w + x]
    }
    /// Muestra entre pixeles; fuera de la imagen repite la orilla.
    fn en(&self, x: f32, y: f32) -> P4 {
        let (x, y) = (x - 0.5, y - 0.5);
        let (i, j) = (x.floor(), y.floor());
        let (u, v) = (x - i, y - j);
        let (i, j) = (i as i32, j as i32);
        mez(mez(self.px(i, j), self.px(i + 1, j), u), mez(self.px(i, j + 1), self.px(i + 1, j + 1), u), v)
    }
    /// Fuera de la imagen se refleja.
    fn espejo(&self, x: f32, y: f32) -> P4 {
        let r = |v: f32, n: f32| {
            let m = v.rem_euclid(2.0 * n);
            if m > n { 2.0 * n - m } else { m }
        };
        self.en(r(x, self.w as f32), r(y, self.h as f32))
    }
}

/// (número de golpe, qué tanto le queda al golpe de 1 a 0)
fn golpe(t: f64, bpm: f64, cada: f64, desf: f64, caida: f64) -> (i32, f32) {
    let b = 60.0 / bpm.max(1.0) * cada.round().max(1.0);
    let u = (t - desf / 1000.0) / b;
    let k = u.floor();
    (k as i32, (-((u - k) * b) / (caida / 1000.0).max(0.01)).exp() as f32)
}

fn recorrer<F: Fn(f32, f32, P4) -> P4 + Sync>(s: &Img, img: &Image, mezcla: f32, fun: F) -> Image {
    let w = s.w;
    let mut o = Image::new(img.width, img.height);
    o.data.par_chunks_mut(w.max(1)).enumerate().for_each(|(y, fila)| {
        for (x, q) in fila.iter_mut().enumerate() {
            let base = s.d[y * w + x];
            let mut p = fun(x as f32 + 0.5, y as f32 + 0.5, base);
            if mezcla != 1.0 {
                p = mez(base, p, mezcla);
            }
            let a = p[3].clamp(0.0, 1.0);
            *q = [p[0].clamp(0.0, 1.0) * a, p[1].clamp(0.0, 1.0) * a, p[2].clamp(0.0, 1.0) * a, a];
        }
    });
    o
}

pub fn aplicar(f: &FxVideo, t: f64, img: &Image, escala: f64) -> Image {
    let (w, h) = (img.width as usize, img.height as usize);
    let s = Img { w, h, d: img.data.par_iter().map(|p| if p[3] > 0.0 { [p[0] / p[3], p[1] / p[3], p[2] / p[3], p[3]] } else { [0.0; 4] }).collect() };
    let num = |k: &str| f.p.get(k).map_or(0.0, |p| p.en(t).num());
    let col = |k: &str| match f.p.get(k).map(|p| p.en(t)) {
        Some(Valor::Color(c)) => c,
        _ => [1.0; 4],
    };
    let por = |k: &str| (num(k) / 100.0) as f32;
    let (wf, hf) = (w as f32, h as f32);
    let (cx, cy) = (wf / 2.0, hf / 2.0);
    let e = escala as f32;
    let tv = t as f32 * por("vel");
    let g = golpe(t, num("bpm"), num("cada"), num("desf"), num("caida"));
    let mezcla = if f.p.contains_key("mezcla") { por("mezcla") } else { 1.0 };
    // Semilla que cambia 24 veces por segundo.
    let cua = (t * 24.0).floor() as i32;
    let c1 = col("color");
    let semilla = f.id as i32;

    // Recorre la imagen: `g(x, y, original)`. Es macro para que cada efecto quede compilado en su propio recorrido.
    macro_rules! pintar {
        ($fun:expr) => {
            recorrer(&s, img, mezcla, $fun)
        };
    }
    // Partículas que caen o suben: (x, y, tamaño, número).
    let particulas = |total: f32, abajo: f32, tam: f32| -> Vec<(f32, f32, f32, i32)> {
        (0..(total * por("cant").max(0.05)) as i32)
            .map(|i| {
                let a = azar(i, semilla);
                let v = 0.09 + 0.16 * azar(i, 11);
                let y = (azar(i, 5) + abajo * v * tv).rem_euclid(1.2) - 0.1;
                let x = a + 0.025 * (tv * (0.6 + azar(i, 3)) + a * 40.0).sin();
                (x * wf, y * hf, hf * tam * (0.5 + azar(i, 9)) * por("tam").max(0.1), i)
            })
            .collect()
    };
    // Pinta partículas solo en su recuadro. `fig` da la cobertura en (-1..1).
    let estampar = |ps: &[(f32, f32, f32, i32)], color: &dyn Fn(i32) -> [f32; 4], fig: &dyn Fn(f32, f32, i32) -> f32, suma: bool| -> Image {
        let mut d = s.d.clone();
        for (px, py, r, i) in ps {
            let c = color(*i);
            let (x0, x1) = (((px - r).floor().max(0.0)) as usize, ((px + r).ceil().clamp(0.0, wf)) as usize);
            let (y0, y1) = (((py - r).floor().max(0.0)) as usize, ((py + r).ceil().clamp(0.0, hf)) as usize);
            for y in y0..y1 {
                for x in x0..x1 {
                    let k = fig((x as f32 + 0.5 - px) / r, (y as f32 + 0.5 - py) / r, *i);
                    if k > 0.0 {
                        let q = &mut d[y * w + x];
                        if suma {
                            for j in 0..3 {
                                q[j] += c[j] * k;
                            }
                            q[3] = q[3].max(k.min(1.0));
                        } else {
                            *q = sobre(*q, c, k);
                        }
                    }
                }
            }
        }
        let mut o = Image::new(img.width, img.height);
        o.data.par_iter_mut().zip(d.par_iter()).for_each(|(q, p)| {
            let a = p[3].clamp(0.0, 1.0);
            *q = [p[0].clamp(0.0, 1.0) * a, p[1].clamp(0.0, 1.0) * a, p[2].clamp(0.0, 1.0) * a, a];
        });
        o
    };

    match f.tipo.as_str() {
        "bc:color" => {
            let pto = |k: &str| match f.p.get(k).map(|p| p.en(t)) {
                Some(Valor::Punto(v)) => [v[0] as f32, v[1] as f32],
                _ => [0.0; 2],
            };
            // Color de cada rueda, sin cambiar el brillo.
            let rueda = |k: &str| {
                let [x, y] = pto(k);
                let b = num(&format!("{k}_l")) as f32 / 100.0 * 0.5;
                let e = |g: f32| x * g.to_radians().cos() + y * g.to_radians().sin();
                [e(0.0) * 0.5 + b, e(120.0) * 0.5 + b, e(240.0) * 0.5 + b]
            };
            let (so, me, lu) = (rueda("sombras"), rueda("medios"), rueda("luces"));
            let exp = 2f32.powf(num("exp") as f32 / 2.2);
            let con = num("con") as f32 / 100.0;
            let con = if con > 0.0 { 1.0 + con * 1.5 } else { 1.0 + con * 0.8 };
            let (sat, vib) = (por("sat"), por("vib"));
            let (te, ti) = (por("temp") * 0.22, por("tinte") * 0.16);
            let giro = (num("giro") as f32).to_radians();
            let (des, vin, gra) = (por("deslavar"), por("vin"), por("grano"));
            pintar!(|x, y, p| {
                let mut c = [p[0] * exp * (1.0 + te), p[1] * exp * (1.0 + ti), p[2] * exp * (1.0 - te)];
                for k in 0..3 {
                    c[k] = (c[k] - 0.45) * con + 0.45;
                }
                let l = luz([c[0], c[1], c[2], 1.0]).clamp(0.0, 1.0);
                let (ws, wl) = ((1.0 - l) * (1.0 - l), l * l);
                let wm = 1.0 - ws - wl;
                for k in 0..3 {
                    c[k] += so[k] * ws + me[k] * wm * 1.4 + lu[k] * wl;
                }
                let l = luz([c[0], c[1], c[2], 1.0]);
                let (max, min) = (c[0].max(c[1]).max(c[2]), c[0].min(c[1]).min(c[2]));
                // La intensidad cuida lo que ya está saturado.
                let s = sat * (1.0 + vib * (1.0 - (max - min).clamp(0.0, 1.0)));
                for v in &mut c {
                    *v = l + (*v - l) * s;
                }
                if giro != 0.0 {
                    let q = girar_tono([c[0], c[1], c[2], 1.0], giro);
                    c = [q[0], q[1], q[2]];
                }
                if des > 0.0 {
                    for v in &mut c {
                        *v = v.max(0.0) * (1.0 - 0.28 * des) + 0.16 * des;
                    }
                }
                if vin > 0.0 {
                    let d = ((x - cx) / cx).powi(2) * 0.5 + ((y - cy) / cy).powi(2) * 0.5;
                    let k = 1.0 - vin * 0.85 * escalon(0.25, 1.05, d);
                    c.iter_mut().for_each(|v| *v *= k);
                }
                if gra > 0.0 {
                    let g = (azar((x / e.max(0.5)) as i32 + cua * 97, (y / e.max(0.5)) as i32) - 0.5) * gra * 0.35;
                    c.iter_mut().for_each(|v| *v += g);
                }
                [c[0], c[1], c[2], p[3]]
            })
        }
        "bc:ritmo_zoom" => {
            let z = 1.0 + por("fuerza") * g.1;
            pintar!(|x, y, _| s.en(cx + (x - cx) / z, cy + (y - cy) / z))
        }
        "bc:ritmo_destello" => {
            let k = por("fuerza") * g.1;
            pintar!(|_, _, b| if b[3] > 0.0 { let a = b[3]; let mut o = sobre(b, c1, k); o[3] = a; o } else { b })
        }
        "bc:ritmo_color" => {
            let a = ((g.0 + 1) as f32 * num("paso") as f32).to_radians();
            pintar!(|_, _, b| girar_tono(b, a))
        }
        "bc:ritmo_sacudida" => {
            let k = num("fuerza") as f32 * e * g.1;
            let tt = t as f32;
            let (dx, dy) = (k * (tt * 71.0).sin(), k * (tt * 93.0 + 1.0).cos());
            // Un poco de acercamiento para que no asomen las orillas.
            let z = 1.0 + 2.0 * k / wf.max(1.0);
            pintar!(|x, y, _| s.espejo(cx + (x - cx) / z + dx, cy + (y - cy) / z + dy))
        }
        "bc:ritmo_rgb" => {
            let k = num("fuerza") as f32 * e * g.1;
            pintar!(|x, y, b| [s.en(x - k, y)[0], b[1], s.en(x + k, y)[2], b[3]])
        }
        "bc:ritmo_vueltas" => {
            let m = g.0.rem_euclid(4);
            pintar!(|x, y, _| s.en(if m == 1 || m == 2 { wf - x } else { x }, if m >= 2 { hf - y } else { y }))
        }
        "bc:tv" => {
            let (cu, li, vi) = (por("curva") * 0.28, por("lineas"), por("vin"));
            let alto = (hf / 240.0).max(1.5);
            pintar!(|x, y, _| {
                let (u, v) = ((x - cx) / cx, (y - cy) / cy);
                let r2 = u * u + v * v;
                let (u2, v2) = (u * (1.0 + cu * v * v), v * (1.0 + cu * u * u));
                if u2.abs() > 1.0 || v2.abs() > 1.0 {
                    return [0.0, 0.0, 0.0, 1.0];
                }
                let (sx, sy) = (cx + u2 * cx, cy + v2 * cy);
                let d = 0.0012 * wf;
                let mut p = [s.en(sx - d, sy)[0], s.en(sx, sy)[1], s.en(sx + d, sy)[2], s.en(sx, sy)[3]];
                let l = 1.0 - li * 0.5 * (0.5 + 0.5 * (sy / alto * std::f32::consts::PI).cos());
                let o = (1.0 - vi * 0.45 * r2).max(0.0);
                let m = (x / alto) as i32 % 3;
                for (k, c) in p.iter_mut().take(3).enumerate() {
                    let fosf = 1.0 - li * 0.22 * f32::from(m != k as i32);
                    *c = *c * l * o * fosf * 1.22;
                }
                p
            })
        }
        "bc:cinta" => {
            let k = por("cant");
            let banda = hf * (1.0 - (tv * 0.23).fract() * 1.2);
            pintar!(|x, y, _| {
                let fila = (y / (hf / 270.0).max(1.0)) as i32;
                let tem = (azar(fila, cua) - 0.5) * 0.004 * wf * k;
                let en_banda = (-((y - banda) / (0.05 * hf)).powi(2)).exp();
                let dx = tem + en_banda * k * wf * 0.05 * (ruido(y * 0.05, cua as f32) - 0.5) * 2.0;
                let q = 0.004 * wf * k;
                let mut p = [s.en(x + dx - q, y)[0], s.en(x + dx, y)[1], s.en(x + dx + q, y)[2], s.en(x + dx, y)[3]];
                let r = azar((x / 2.0) as i32 + cua * 131, fila);
                if r > 1.0 - 0.18 * en_banda * k.min(1.0) {
                    p = [0.95, 0.95, 0.95, 1.0];
                }
                // Colores lavados.
                let l = luz(p);
                for c in p.iter_mut().take(3) {
                    *c = *c + (l - *c) * 0.2 * k.min(1.0) + (r - 0.5) * 0.06 * k;
                }
                p
            })
        }
        "bc:nocturna" => {
            let (b, ru) = (por("brillo"), por("ruido"));
            pintar!(|x, y, p| {
                let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() / (0.62 * wf);
                let l = (luz(p) * b).powf(0.8) + (azar(x as i32 + cua * 57, y as i32) - 0.5) * ru * 0.6;
                let v = l * (1.0 - escalon(0.65, 1.0, d));
                [c1[0] * v, c1[1] * v, c1[2] * v, p[3]]
            })
        }
        "bc:termica" => {
            const PAL: [[f32; 3]; 6] = [[0.0, 0.0, 0.12], [0.22, 0.0, 0.55], [0.78, 0.08, 0.5], [0.98, 0.4, 0.1], [1.0, 0.82, 0.15], [1.0, 1.0, 0.92]];
            pintar!(|_, _, p| {
                let v = luz(p).clamp(0.0, 1.0) * 5.0;
                let i = (v as usize).min(4);
                let k = v - i as f32;
                let c: [f32; 3] = std::array::from_fn(|j| PAL[i][j] + (PAL[i + 1][j] - PAL[i][j]) * k);
                [c[0], c[1], c[2], p[3]]
            })
        }
        "bc:consola" => {
            const PAL: [[f32; 3]; 4] = [[0.09, 0.17, 0.14], [0.22, 0.38, 0.27], [0.5, 0.62, 0.3], [0.8, 0.85, 0.55]];
            let t = (num("tam") as f32 * e).max(1.5);
            pintar!(|x, y, b| {
                let (i, j) = ((x / t).floor(), (y / t).floor());
                let p = s.en((i + 0.5) * t, (j + 0.5) * t);
                // Trama de 2×2 para que se noten más tonos.
                let tr = [[0.0, 0.5], [0.75, 0.25]][(j as usize) & 1][(i as usize) & 1];
                let v = (luz(p) * 3.0 + tr * 0.9).floor().clamp(0.0, 3.0) as usize;
                let c = PAL[v];
                [c[0], c[1], c[2], b[3]]
            })
        }
        "bc:barras" => {
            let alto = (wf / num("razon").max(0.5) as f32).min(hf);
            let b = (hf - alto) / 2.0 * (1.0 - por("abrir"));
            pintar!(|_, y, p| if y < b || y > hf - b { [0.0, 0.0, 0.0, 1.0] } else { p })
        }
        "bc:marco" => {
            let o = por("tam") * hf;
            let pie = o + por("pie") * hf;
            let (x0, y0, x1, y1) = (o, o, wf - o, hf - pie);
            pintar!(|x, y, _| {
                if x < x0 || x > x1 || y < y0 || y > y1 {
                    // Sombra suave hacia adentro de la foto.
                    return [c1[0], c1[1], c1[2], 1.0];
                }
                let d = (x - x0).min(x1 - x).min(y - y0).min(y1 - y);
                let u = (x - x0) / (x1 - x0);
                let v = (y - y0) / (y1 - y0);
                let q = s.en(u * wf, v * hf);
                let k = 0.65 + 0.35 * escalon(0.0, 0.02 * hf, d);
                [q[0] * k, q[1] * k, q[2] * k, 1.0]
            })
        }
        "bc:puntos" => {
            let t = (num("tam") as f32 * e).max(2.0);
            let (si, co) = (num("ang") as f32).to_radians().sin_cos();
            pintar!(|x, y, b| {
                let (u, v) = ((x * co + y * si) / t, (-x * si + y * co) / t);
                let (i, j) = (u.floor() + 0.5, v.floor() + 0.5);
                let p = s.en((i * co - j * si) * t, (i * si + j * co) * t);
                let l = luz(p).max(0.02);
                let r = (1.0 - l * 0.85).sqrt() * 0.66;
                let d = ((u - i).powi(2) + (v - j).powi(2)).sqrt();
                let k = escalon(r + 0.08, r - 0.08, d);
                // Tinta del color del punto, más saturada y oscura, sobre papel.
                let tinta = [p[0] / l * 0.45, p[1] / l * 0.45, p[2] / l * 0.45, b[3]];
                mez([0.97, 0.95, 0.9, b[3]], tinta, k)
            })
        }
        "bc:manga" => {
            let (ti, tr) = (por("tinta"), (num("tam") as f32 * e).max(2.0));
            let o = (1.5 * e).max(1.0);
            pintar!(|x, y, p| {
                let l = |dx: f32, dy: f32| luz(s.en(x + dx, y + dy));
                let gx = l(o, 0.0) - l(-o, 0.0);
                let gy = l(0.0, o) - l(0.0, -o);
                let borde = escalon(0.3 - 0.26 * ti, 0.4 - 0.26 * ti, (gx * gx + gy * gy).sqrt());
                let b = l(0.0, 0.0);
                let v = if b < 0.22 + 0.1 * ti {
                    0.0
                } else if b < 0.55 {
                    let (i, j) = ((x + y) / tr, (x - y) / tr);
                    let d = ((i - i.floor() - 0.5).powi(2) + (j - j.floor() - 0.5).powi(2)).sqrt();
                    escalon(0.6 * (0.62 - b).max(0.1), 0.6 * (0.62 - b).max(0.1) + 0.14, d)
                } else {
                    1.0
                } * (1.0 - borde);
                [v, v, v, p[3]]
            })
        }
        "bc:duotono" => {
            let (a, b) = (col("osc"), col("cla"));
            pintar!(|_, _, p| {
                let l = escalon(0.02, 0.95, luz(p));
                let o = mez(a, b, l);
                [o[0], o[1], o[2], p[3]]
            })
        }
        "bc:neblina" => {
            let k = por("cant");
            pintar!(|x, y, p| {
                let v = nubes(x / hf * 2.6 + tv * 0.12, y / hf * 3.2 + tv * 0.03);
                let m = escalon(0.25, 0.8, v) * (0.45 + 0.55 * y / hf) * k * 1.3;
                let mut o = sobre(p, c1, m);
                o[3] = p[3];
                o
            })
        }
        "bc:velocidad" => {
            let (k, hu) = (por("cant"), por("hueco"));
            let paso = (tv * 16.0).floor() as i32;
            pintar!(|x, y, p| {
                let (u, v) = ((x - cx) / cx, (y - cy) / cy);
                let r = (u * u + v * v).sqrt();
                let sec = ((v.atan2(u) / std::f32::consts::TAU + 0.5) * 180.0) as i32;
                let a = azar(sec, paso + semilla);
                if a > k {
                    return p;
                }
                let ini = hu + 0.5 * azar(sec, paso + 77);
                // Cada raya es más gruesa hacia la orilla.
                let f = ((v.atan2(u) / std::f32::consts::TAU + 0.5) * 180.0).fract() - 0.5;
                let ancho = 0.5 * escalon(ini, ini + 0.9, r);
                sobre(p, c1, escalon(ancho, ancho * 0.6, f.abs()) * escalon(ini, ini + 0.08, r))
            })
        }
        "bc:confeti" => estampar(
            &particulas(140.0, 1.0, 0.018),
            &|i| arcoiris(azar(i, 21)),
            &|u, v, i| {
                // Papelito que gira: un rectángulo que se adelgaza.
                let (si, co) = (tv * (1.0 + azar(i, 31) * 3.0) + i as f32).sin_cos();
                let gira = (tv * (2.0 + 4.0 * azar(i, 41)) + i as f32).cos().abs().max(0.12);
                let (a, b) = (u * co + v * si, -u * si + v * co);
                f32::from(a.abs() < 0.68 && b.abs() < 0.42 * gira)
            },
            false,
        ),
        "bc:corazones" => estampar(
            &particulas(34.0, -1.0, 0.045),
            &|i| { let k = 0.75 + 0.25 * azar(i, 51); [c1[0] * k, c1[1] * k, c1[2] * k, 1.0] },
            &|u, v, i| {
                let (x, y) = (u * 1.25, -v * 1.25 + 0.1);
                let q = x * x + y * y - 0.8;
                0.9 * escalon(0.06, -0.06, q * q * q - x * x * y * y * y) * (0.7 + 0.3 * azar(i, 61))
            },
            false,
        ),
        "bc:luciernagas" => estampar(
            &particulas(60.0, -0.35, 0.04),
            &|_| c1,
            &|u, v, i| {
                let late = 0.35 + 0.65 * (tv * (1.0 + azar(i, 71) * 2.0) + i as f32).sin().abs();
                let r = (u * u + v * v).sqrt();
                (1.0 - r).max(0.0).powi(3) * late * 0.9 + escalon(0.16, 0.08, r) * late
            },
            true,
        ),
        "bc:prisma" => {
            let n = num("caras").round().max(2.0) as usize;
            let k = por("cant") * 0.5 * hf;
            let giro = tv * 0.4;
            pintar!(|x, y, b| {
                let mut o = b.map(|c| c * 0.5);
                for i in 0..n {
                    let a = std::f32::consts::TAU * i as f32 / n as f32 + giro;
                    let q = s.espejo(x + a.cos() * k, y + a.sin() * k);
                    for j in 0..4 {
                        o[j] += q[j] * 0.5 / n as f32;
                    }
                }
                o
            })
        }
        "bc:tunel" => {
            let paso = por("paso").clamp(0.2, 0.95);
            let giro = (num("giro") as f32).to_radians();
            let f0 = (tv * 0.5).rem_euclid(1.0);
            pintar!(|x, y, _| {
                // Se busca en qué vuelta cae el punto: el cuadro más chico que lo contiene.
                let (u, v) = (x - cx, y - cy);
                let z0 = paso.powf(-f0);
                let mut mejor = [0.0; 4];
                for k in 0..10 {
                    let z = z0 * paso.powi(k);
                    let (si, co) = (giro * (k as f32 - f0)).sin_cos();
                    let (a, b) = ((u * co + v * si) / z, (-u * si + v * co) / z);
                    if a.abs() > cx || b.abs() > cy {
                        // Con giro, una vuelta chica puede no cubrir y otra tampoco: se queda la última que sí.
                        break;
                    }
                    let q = s.en(cx + a, cy + b);
                    let sombra = 1.0 - 0.04 * k as f32;
                    mejor = [q[0] * sombra, q[1] * sombra, q[2] * sombra, q[3].max(mejor[3])];
                }
                mejor
            })
        }
        "bc:agua" => {
            let (k, ta) = (por("cant") * 0.035 * hf, por("tam") * hf);
            pintar!(|x, y, _| {
                let dx = k * ((y / ta * 6.28 + tv * 2.1).sin() + 0.5 * (y / ta * 11.0 + x / ta * 2.0 - tv * 1.3).sin());
                let dy = k * ((x / ta * 5.1 + tv * 1.7).cos() + 0.5 * (x / ta * 9.3 + tv * 2.6).sin());
                let p = s.espejo(x + dx, y + dy);
                // Luz que baila.
                let l = 1.0 + 0.35 * (nubes(x / ta * 1.2 + tv * 0.5, y / ta * 1.2 - tv * 0.3) - 0.5);
                [p[0] * l * 0.88, p[1] * l * 0.98, p[2] * l * 1.05, p[3]]
            })
        }
        _ => img.clone(),
    }
}
