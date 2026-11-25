//! Nodos: en lugar de una lista, los efectos de un clip se conectan como quieras, con máscaras, calificadores de color
//! y fusiones. El grafo se aplica después de los efectos normales del clip.
use crate::composicion::{self, FUSIONES, Fuente};
use crate::modelo::{ClipVideo, Contenido, FxVideo, Proyecto, Valor};
use crate::vpropios;
use effectcraft_color::BlendMode;
use effectcraft_effects as ec;
use effectcraft_raster::Image;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Clase {
    Entrada,
    Salida,
    /// Cualquier efecto de video.
    Efecto,
    Fusionar,
    Mascara,
    Calificador,
    Fondo,
    /// Otro clip, imagen o video.
    Medio,
}

impl Clase {
    pub const NUEVAS: [(Clase, &'static str, &'static str); 4] = [
        (Clase::Fusionar, "Fusionar", "Pone una imagen sobre otra, con modo de fusión y máscara"),
        (Clase::Mascara, "Máscara", "Una forma: limita un efecto o una fusión a una zona"),
        (Clase::Calificador, "Calificador de color", "Una máscara a partir de un color, su saturación y su brillo"),
        (Clase::Fondo, "Color", "Un color liso"),
    ];
    /// Nombre de cada entrada, en orden.
    pub fn entradas(self) -> &'static [&'static str] {
        match self {
            Clase::Entrada | Clase::Fondo | Clase::Medio => &[],
            Clase::Salida | Clase::Calificador => &["Imagen"],
            Clase::Efecto => &["Imagen", "Máscara"],
            Clase::Fusionar => &["Fondo", "Frente", "Máscara"],
            Clase::Mascara => &["Máscara"],
        }
    }
    pub fn da_mascara(self) -> bool {
        matches!(self, Clase::Mascara | Clase::Calificador)
    }
    fn tipo(self) -> &'static str {
        match self {
            Clase::Fusionar => "nodo:fusionar",
            Clase::Mascara => "nodo:mascara",
            Clase::Calificador => "nodo:calificador",
            Clase::Fondo => "nodo:fondo",
            Clase::Medio => "nodo:medio",
            _ => "",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Nodo {
    pub id: u64,
    pub clase: Clase,
    pub x: f32,
    pub y: f32,
    /// Sus controles (y qué efecto es).
    pub fx: FxVideo,
    /// De la clase Medio: el clip, la imagen o el video que pone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub medio: Option<Box<ClipVideo>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cable {
    pub de: u64,
    pub a: u64,
    /// Cuál entrada.
    pub puerto: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grafo {
    pub activo: bool,
    pub nodos: Vec<Nodo>,
    pub cables: Vec<Cable>,
}

pub const ENTRADA: u64 = 1;
pub const SALIDA: u64 = 2;

impl Default for Grafo {
    fn default() -> Self {
        let n = |id, clase, x| Nodo { id, clase, x, y: 120.0, fx: FxVideo { id, tipo: String::new(), activo: true, p: Default::default() }, medio: None };
        Grafo { activo: true, nodos: vec![n(ENTRADA, Clase::Entrada, 40.0), n(SALIDA, Clase::Salida, 640.0)], cables: vec![Cable { de: ENTRADA, a: SALIDA, puerto: 0 }] }
    }
}

impl Grafo {
    pub fn nodo(&self, id: u64) -> Option<&Nodo> {
        self.nodos.iter().find(|n| n.id == id)
    }
    pub fn nodo_mut(&mut self, id: u64) -> Option<&mut Nodo> {
        self.nodos.iter_mut().find(|n| n.id == id)
    }
    fn id(&self) -> u64 {
        self.nodos.iter().map(|n| n.id).max().unwrap_or(SALIDA) + 1
    }
    /// Nodo nuevo. `fx` es el efecto si la clase es Efecto.
    pub fn agregar(&mut self, clase: Clase, fx: Option<FxVideo>, x: f32, y: f32) -> u64 {
        let id = self.id();
        let mut fx = fx.unwrap_or_else(|| vpropios::buscar(clase.tipo()).map_or(FxVideo { id, tipo: String::new(), activo: true, p: Default::default() }, |d| vpropios::nuevo(id, d)));
        fx.id = id;
        self.nodos.push(Nodo { id, clase, x, y, fx, medio: None });
        id
    }
    /// Verdadero si desde `de` se llega a `a` siguiendo los cables.
    pub fn llega(&self, de: u64, a: u64) -> bool {
        de == a || self.cables.iter().filter(|c| c.de == de).any(|c| self.llega(c.a, a))
    }
    /// Conecta; cada entrada tiene un solo cable y no se permiten vueltas.
    pub fn conectar(&mut self, de: u64, a: u64, puerto: u8) -> bool {
        let ok = self.nodo(de).is_some_and(|n| n.clase != Clase::Salida) && self.nodo(a).is_some_and(|n| (puerto as usize) < n.clase.entradas().len());
        if !ok || self.llega(a, de) {
            return false;
        }
        self.cables.retain(|c| !(c.a == a && c.puerto == puerto));
        self.cables.push(Cable { de, a, puerto });
        true
    }
    /// Pone `nuevo` en medio de todo lo que salía de `antes`.
    pub fn insertar(&mut self, antes: u64, nuevo: u64) {
        if self.nodo(nuevo).is_none_or(|n| n.clase.entradas().is_empty()) {
            return;
        }
        for c in self.cables.iter_mut().filter(|c| c.de == antes && c.a != nuevo) {
            c.de = nuevo;
        }
        self.conectar(antes, nuevo, 0);
    }
    /// Quita un nodo y une lo que tenía a los lados.
    pub fn quitar(&mut self, id: u64) {
        if id == ENTRADA || id == SALIDA {
            return;
        }
        let antes = self.cables.iter().find(|c| c.a == id && c.puerto == 0).map(|c| c.de);
        let mascara = self.nodo(id).is_some_and(|n| n.clase.da_mascara());
        let despues: Vec<Cable> = self.cables.iter().filter(|c| c.de == id).copied().collect();
        self.cables.retain(|c| c.a != id && c.de != id);
        self.nodos.retain(|n| n.id != id);
        if let (Some(de), false) = (antes, mascara) {
            for c in despues {
                self.conectar(de, c.a, c.puerto);
            }
        }
    }
    /// Acomoda de izquierda a derecha según qué depende de qué.
    pub fn acomodar(&mut self) {
        let mut col: HashMap<u64, usize> = HashMap::new();
        fn hondo(g: &Grafo, id: u64, m: &mut HashMap<u64, usize>) -> usize {
            if let Some(d) = m.get(&id) {
                return *d;
            }
            let d = g.cables.iter().filter(|c| c.a == id).map(|c| hondo(g, c.de, m) + 1).max().unwrap_or(0);
            m.insert(id, d);
            d
        }
        for n in &self.nodos {
            hondo(self, n.id, &mut col);
        }
        // Las fuentes sueltas (máscaras, colores) van justo antes de lo que alimentan.
        for n in self.nodos.iter().filter(|n| n.id != ENTRADA && !self.cables.iter().any(|c| c.a == n.id)) {
            if let Some(m) = self.cables.iter().filter(|c| c.de == n.id).filter_map(|c| col.get(&c.a)).min() {
                col.insert(n.id, m.saturating_sub(1));
            }
        }
        let ultimo = col.values().copied().max().unwrap_or(1);
        col.insert(SALIDA, ultimo.max(1));
        let mut fila: HashMap<usize, usize> = HashMap::new();
        for n in &mut self.nodos {
            let c = col[&n.id];
            let f = fila.entry(c).or_default();
            n.x = 40.0 + c as f32 * 215.0;
            n.y = 60.0 + *f as f32 * 200.0;
            *f += 1;
        }
    }
}

/// Todo del tamaño de la capa: los efectos que agrandan se recortan.
fn cuadrar(b: ec::Buf, w: u32, h: u32) -> ec::Buf {
    if (b.img.width, b.img.height) == (w, h) && b.offset == [0.0; 2] {
        return b;
    }
    let (dx, dy) = (b.offset[0].round() as i64, b.offset[1].round() as i64);
    let mut o = Image::new(w, h);
    o.data.par_chunks_mut(w.max(1) as usize).enumerate().for_each(|(y, fila)| {
        fila.iter_mut().enumerate().for_each(|(x, q)| *q = b.img.get(x as i64 + dx, y as i64 + dy));
    });
    ec::Buf { img: o, offset: [0.0; 2], scale: b.scale }
}

/// Aplica el grafo a la imagen del clip. `fx` aplica un efecto normal; `medios`, las imágenes de los nodos Medio.
pub fn evaluar(g: &Grafo, entrada: ec::Buf, t: f64, medios: &HashMap<u64, ec::Buf>, fx: &dyn Fn(&FxVideo, ec::Buf) -> ec::Buf) -> ec::Buf {
    let (w, h) = (entrada.img.width, entrada.img.height);
    let mut hechos: HashMap<u64, ec::Buf> = HashMap::new();
    let base = ec::Buf { offset: [0.0; 2], ..entrada.clone() };
    let r = nodo(g, SALIDA, &base, t, medios, fx, &mut hechos, 0);
    ec::Buf { offset: entrada.offset, ..r.unwrap_or_else(|| ec::Buf { img: Image::new(w, h), offset: [0.0; 2], scale: entrada.scale }) }
}

#[allow(clippy::too_many_arguments)]
fn nodo(g: &Grafo, id: u64, base: &ec::Buf, t: f64, medios: &HashMap<u64, ec::Buf>, fx: &dyn Fn(&FxVideo, ec::Buf) -> ec::Buf, hechos: &mut HashMap<u64, ec::Buf>, hondo: u32) -> Option<ec::Buf> {
    if let Some(b) = hechos.get(&id) {
        return Some(b.clone());
    }
    let n = g.nodo(id)?;
    if hondo > 200 {
        return None;
    }
    let (w, h) = (base.img.width, base.img.height);
    let mut ent = |p: u8| {
        let c = g.cables.iter().find(|c| c.a == id && c.puerto == p)?;
        nodo(g, c.de, base, t, medios, fx, hechos, hondo + 1)
    };
    let num = |k: &str| n.fx.p.get(k).map_or(0.0, |p| p.en(t).num());
    let op = |k: &str| match n.fx.p.get(k).map(|p| p.en(t)) {
        Some(Valor::Op(i)) => i,
        _ => 0,
    };
    let nuevo = |img: Image| ec::Buf { img, offset: [0.0; 2], scale: base.scale };
    let r = match n.clase {
        Clase::Entrada => base.clone(),
        Clase::Salida => ent(0)?,
        Clase::Efecto => {
            let a = ent(0)?;
            if !n.fx.activo {
                a
            } else {
                let b = cuadrar(fx(&n.fx, a.clone()), w, h);
                match ent(1) {
                    Some(m) => mezclar(a, &b, &m.img),
                    None => b,
                }
            }
        }
        Clase::Fusionar => {
            let (fondo, frente, masc) = (ent(0), ent(1), ent(2));
            match (fondo, frente) {
                (Some(f), _) if !n.fx.activo => f,
                (Some(f), Some(mut e)) => {
                    if let Some(m) = masc {
                        e.img.data.par_iter_mut().zip(m.img.data.par_iter()).for_each(|(q, m)| *q = q.map(|v| v * m[3]));
                    }
                    let mut f = f;
                    let modo = FUSIONES.get(op("modo") as usize).map_or(BlendMode::Normal, |x| x.1);
                    f.img.blend_from(&e.img, modo, (num("opacidad") / 100.0).clamp(0.0, 1.0) as f32, 0);
                    f
                }
                (f, e) => f.or(e)?,
            }
        }
        Clase::Fondo => {
            let c = match n.fx.p.get("color").map(|p| p.en(t)) {
                Some(Valor::Color(c)) => c,
                _ => [0.0, 0.0, 0.0, 1.0],
            };
            nuevo(Image::filled(w, h, [c[0] * c[3], c[1] * c[3], c[2] * c[3], c[3]]))
        }
        Clase::Mascara => {
            let mut o = mascara(n, t, w, h);
            if let Some(z) = ent(0) {
                o.data.par_iter_mut().zip(z.img.data.par_iter()).for_each(|(q, z)| *q = q.map(|v| v * z[3]));
            }
            nuevo(o)
        }
        Clase::Calificador => nuevo(calificador(n, t, &ent(0)?.img)),
        Clase::Medio => medios.get(&id)?.clone(),
    };
    hechos.insert(id, r.clone());
    Some(r)
}

fn mezclar(mut a: ec::Buf, b: &ec::Buf, m: &Image) -> ec::Buf {
    a.img.data.par_iter_mut().zip(b.img.data.par_iter()).zip(m.data.par_iter()).for_each(|((a, b), m)| {
        for k in 0..4 {
            a[k] += (b[k] - a[k]) * m[3];
        }
    });
    a
}

/// La forma de un nodo Máscara: cobertura en los cuatro canales.
pub fn mascara(n: &Nodo, t: f64, w: u32, h: u32) -> Image {
    let num = |k: &str| n.fx.p.get(k).map_or(0.0, |p| p.en(t).num());
    let op = |k: &str| opcion(n, k, t);
    let (wf, hf) = (w as f32, h as f32);
    let cx = wf / 2.0 + num("x") as f32 / 100.0 * wf;
    let cy = hf / 2.0 + num("y") as f32 / 100.0 * hf;
    let (a, b) = ((num("ancho") as f32 / 200.0 * hf).max(0.5), (num("alto") as f32 / 200.0 * hf).max(0.5));
    let (s, c) = (num("giro") as f32).to_radians().sin_cos();
    let suave = (num("suave") as f32 / 100.0 * hf * 0.5).max(0.75);
    let (forma, inv) = (op("forma"), op("invertir") == 1);
    let k = (num("fuerza") as f32 / 100.0).clamp(0.0, 1.0);
    let mut o = Image::new(w, h);
    o.data.par_chunks_mut(w.max(1) as usize).enumerate().for_each(|(y, fila)| {
        for (x, q) in fila.iter_mut().enumerate() {
            let (px, py) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let (u, v) = (px * c + py * s, -px * s + py * c);
            let m = match forma {
                0 => 0.5 - (((u / a).hypot(v / b)) - 1.0) * a.min(b) / suave,
                1 => 0.5 - ((u.abs() - a).max(v.abs() - b)) / suave,
                // Degradado: de un lado sí, del otro no.
                _ => 0.5 - v / (b * 2.0).max(1.0),
            }
            .clamp(0.0, 1.0);
            let m = m * m * (3.0 - 2.0 * m);
            *q = [if inv { 1.0 - m } else { m } * k; 4];
        }
    });
    o
}

fn opcion(n: &Nodo, k: &str, t: f64) -> u32 {
    match n.fx.p.get(k).map(|p| p.en(t)) {
        Some(Valor::Op(i)) => i,
        _ => 0,
    }
}

/// La máscara que saca un nodo Calificador de la imagen `a`.
pub fn calificador(n: &Nodo, t: f64, a: &Image) -> Image {
    let num = |k: &str| n.fx.p.get(k).map_or(0.0, |p| p.en(t).num());
    let (tono, ancho) = (num("tono") as f32, (num("ancho") as f32 / 2.0).max(0.5));
    let (s0, s1) = (num("sat_min") as f32 / 100.0, num("sat_max") as f32 / 100.0);
    let (l0, l1) = (num("luz_min") as f32 / 100.0, num("luz_max") as f32 / 100.0);
    let suave = (num("suave") as f32 / 100.0).max(0.01);
    let inv = opcion(n, "invertir", t) == 1;
    let dentro = |v: f32, a: f32, b: f32| (((v - a) / suave + 0.5).clamp(0.0, 1.0)).min(((b - v) / suave + 0.5).clamp(0.0, 1.0));
    let mut o = Image::new(a.width, a.height);
    o.data.par_iter_mut().zip(a.data.par_iter()).for_each(|(q, p)| {
        let al = p[3].max(1e-4);
        let (r, g, b) = (p[0] / al, p[1] / al, p[2] / al);
        let (max, min) = (r.max(g).max(b), r.min(g).min(b));
        let d = max - min;
        let luz = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let sat = if max > 0.0 { d / max } else { 0.0 };
        let hue = if d < 1e-5 {
            0.0
        } else if max == r {
            60.0 * ((g - b) / d).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        let dh = ((hue - tono + 540.0).rem_euclid(360.0) - 180.0).abs();
        // Los grises no tienen tono: solo entran si la rueda está completa.
        let kh = if ancho >= 180.0 { 1.0 } else { ((ancho - dh) / (suave * 60.0).max(1.0) + 0.5).clamp(0.0, 1.0) * (d * 20.0).min(1.0) };
        let m = kh * dentro(sat, s0, s1) * dentro(luz, l0, l1) * p[3];
        *q = [if inv { p[3] - m } else { m }; 4];
    });
    o
}

// ---- nodos Medio: otro clip, imagen o video dentro del grafo

/// El clip de un nodo Medio, con un número propio según quién lo usa (dos clips con los mismos nodos lo leen en momentos
/// distintos), y el segundo en que va cuando el clip dueño lleva `tl`.
fn medio_de(n: &Nodo, dueno: u64, tl: f64) -> Option<(ClipVideo, f64)> {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut c = (**n.medio.as_ref()?).clone();
    let mut h = DefaultHasher::new();
    (dueno, n.id, "medio").hash(&mut h);
    c.id = h.finish() | 1 << 63;
    let num = |k: &str| n.fx.p.get(k).map_or(0.0, |p| p.en(tl).num());
    (c.inicio, c.mov.velocidad) = (0.0, 1.0);
    let t = match c.contenido {
        Contenido::Archivo => {
            c.desde = (c.desde + num("desde")).clamp(0.0, (c.total - 0.05).max(0.0));
            let largo = (c.total - c.desde).max(0.05);
            if opcion(n, "final", tl) == 1 { tl.max(0.0).rem_euclid(largo) } else { tl.clamp(0.0, largo) }
        }
        _ => tl.clamp(0.0, c.dur),
    };
    Some((c, t))
}

/// Dónde va un medio de `an` × `al` en una capa de `w` × `h`: ancho, alto y centro.
pub fn lugar(n: &Nodo, tl: f64, (an, al): (f64, f64), (w, h): (f64, f64)) -> (f64, f64, f64, f64) {
    let num = |k: &str| n.fx.p.get(k).map_or(0.0, |p| p.en(tl).num());
    let (kx, ky) = (w / an.max(1.0), h / al.max(1.0));
    let (kx, ky) = match opcion(n, "ajuste", tl) {
        0 => (kx.max(ky), kx.max(ky)),
        1 => (kx.min(ky), kx.min(ky)),
        _ => (kx, ky),
    };
    let e = (num("escala") / 100.0).max(0.001);
    (an * kx * e, al * ky * e, w / 2.0 + num("x") / 100.0 * w, h / 2.0 + num("y") / 100.0 * h)
}

/// A qué tamaño se decodifica: el que se va a ver, sin pasar del doble de la capa.
fn tam_medio(n: &Nodo, c: &ClipVideo, tl: f64, w: usize, h: usize) -> (usize, usize) {
    let (mut a, mut b, ..) = lugar(n, tl, (c.ancho as f64, c.alto as f64), (w as f64, h as f64));
    // Con la escala animada, siempre al 100 %: cambiar de tamaño es volver a abrir el archivo.
    if let Some(e) = n.fx.p.get("escala").filter(|p| !p.claves.is_empty()) {
        let k = (e.en(tl).num() / 100.0).max(0.001);
        (a, b) = (a / k, b / k);
    }
    let k = (2.0 * w as f64 / a).min(2.0 * h as f64 / b).min(1.0);
    let par = |v: f64| ((v * k / 2.0).round() as usize * 2).max(2);
    // Con un tamaño que no baila: se redondea a pasos de 16.
    let paso = |v: usize| v.div_ceil(16) * 16;
    (paso(par(a)), paso(par(b)))
}

fn activos(c: &ClipVideo) -> impl Iterator<Item = &Nodo> {
    c.nodos.iter().filter(|g| g.activo).flat_map(|g| g.nodos.iter().filter(|n| n.clase == Clase::Medio && n.fx.activo && g.llega(n.id, SALIDA)))
}

/// Los cuadros de video que necesitan los nodos del clip `c` en `t`, para una capa de `w` × `h`.
pub fn pedidos(c: &ClipVideo, t: f64, w: usize, h: usize) -> Vec<(ClipVideo, f64, usize, usize)> {
    let tl = t - c.inicio;
    activos(c)
        .filter_map(|n| {
            let (m, tm) = medio_de(n, c.id, tl)?;
            let ts = match m.contenido {
                Contenido::Archivo => m.fuente(tm).clamp(0.0, (m.total - 0.02).max(0.0)),
                Contenido::Imagen => 0.0,
                _ => return None,
            };
            let (a, b) = tam_medio(n, &m, tl, w, h);
            Some((m, ts, a, b))
        })
        .collect()
}

/// Un medio listo para dibujarse: su imagen suelta, o un cuadro de video.
pub enum Dibujo {
    Cuadro(std::sync::Arc<composicion::Cuadro>),
    Imagen(Image),
}

/// Las imágenes de los nodos Medio del clip `c` en `t`: nodo, imagen y en qué lugar va (ancho, alto, centro).
pub fn dibujos(c: &ClipVideo, t: f64, p: &Proyecto, w: usize, h: usize, f: &mut dyn Fuente) -> Vec<(u64, Dibujo, (f64, f64, f64, f64), f32)> {
    let tl = t - c.inicio;
    activos(c)
        .filter_map(|n| {
            let (m, tm) = medio_de(n, c.id, tl)?;
            let op = (n.fx.p.get("opacidad").map_or(100.0, |p| p.en(tl).num()) / 100.0).clamp(0.0, 1.0) as f32;
            let archivo = matches!(m.contenido, Contenido::Archivo | Contenido::Imagen);
            let (d, tam) = if archivo {
                let ts = if m.contenido == Contenido::Imagen { 0.0 } else { m.fuente(tm).clamp(0.0, (m.total - 0.02).max(0.0)) };
                let (a, b) = tam_medio(n, &m, tl, w, h);
                (Dibujo::Cuadro(f.cuadro_libre(&m, ts, a, b)?), (m.ancho as f64, m.alto as f64))
            } else {
                // Títulos, figuras y colores: del tamaño de la capa.
                (Dibujo::Imagen(composicion::imagen_de(&m, tm, p, w, h, f)?.img), (w as f64, h as f64))
            };
            Some((n.id, d, lugar(n, tl, tam, (w as f64, h as f64)), op))
        })
        .collect()
}

/// Lo mismo, ya puesto en una capa de `w` × `h` (para el procesador).
pub fn medios(c: &ClipVideo, t: f64, p: &Proyecto, base: &ec::Buf, f: &mut dyn Fuente) -> HashMap<u64, ec::Buf> {
    let (w, h) = (base.img.width, base.img.height);
    dibujos(c, t, p, w as usize, h as usize, f)
        .into_iter()
        .map(|(id, d, (aw, ah, cx, cy), op)| {
            let q = match d {
                Dibujo::Cuadro(q) => Image::from_rgba8(q.ancho as u32, q.alto as u32, &q.en_rgba()),
                Dibujo::Imagen(i) => i,
            };
            let (kx, ky) = (q.width as f64 / aw, q.height as f64 / ah);
            let mut o = Image::new(w, h);
            o.data.par_chunks_mut(w.max(1) as usize).enumerate().for_each(|(y, fila)| {
                let v = (y as f64 + 0.5 - cy + ah / 2.0) * ky;
                for (x, z) in fila.iter_mut().enumerate() {
                    let u = (x as f64 + 0.5 - cx + aw / 2.0) * kx;
                    if u > -1.0 && v > -1.0 && u < q.width as f64 + 1.0 && v < q.height as f64 + 1.0 {
                        *z = q.sample_bilinear(u - 0.5, v - 0.5).map(|s| s * op);
                    }
                }
            });
            (id, ec::Buf { img: o, offset: [0.0; 2], scale: base.scale })
        })
        .collect()
}
