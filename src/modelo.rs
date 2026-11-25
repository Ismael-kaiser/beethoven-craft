//! El proyecto: pistas de audio, pista de video y efectos. Se guarda tal cual como JSON (`.toma`).
use crate::curvas::Curva;
use crate::efectos::TipoFx;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Frecuencia interna fija. Todo se decodifica a estéreo a esta frecuencia.
pub const SR: u32 = 48_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fx {
    pub id: u64,
    pub tipo: TipoFx,
    pub activo: bool,
    pub p: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub id: u64,
    pub ruta: PathBuf,
    pub nombre: String,
    /// Segundo de la línea de tiempo en que empieza.
    pub inicio: f64,
    /// Segundo del archivo desde el que suena.
    pub desde: f64,
    pub dur: f64,
    /// Duración del archivo completo (límite del recorte).
    pub total: f64,
    pub ganancia_db: f32,
    pub fade_in: f64,
    pub fade_out: f64,
    pub invertido: bool,
    /// Clips con el mismo grupo se editan juntos (imagen y sonido de un video).
    #[serde(default)]
    pub grupo: Option<u64>,
    #[serde(default = "uno")]
    pub velocidad: f64,
    /// Mezclador al que va directo; `None`: suena por su pista.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bus: Option<u64>,
    /// Color con que se tiñe en la línea de tiempo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
}

fn uno() -> f64 {
    1.0
}

impl Clip {
    pub fn fin(&self) -> f64 {
        self.inicio + self.dur
    }
    /// Clave en el almacén: con otra velocidad suena una versión estirada del archivo.
    pub fn clave(&self) -> PathBuf {
        clave_velocidad(&self.ruta, self.velocidad)
    }
}

pub fn clave_velocidad(ruta: &std::path::Path, v: f64) -> PathBuf {
    if (v - 1.0).abs() < 1e-4 { ruta.to_path_buf() } else { format!("{}#v{v:.4}", ruta.display()).into() }
}

/// Qué mueve una automatización.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Destino {
    Volumen,
    Paneo,
    /// Control `1` del efecto con id `0`.
    Fx(u64, usize),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Automatizacion {
    pub destino: Destino,
    pub activa: bool,
    /// (segundo, valor), en orden.
    pub puntos: Vec<(f64, f32)>,
}

impl Automatizacion {
    pub fn en(&self, t: f64) -> Option<f32> {
        let p = &self.puntos;
        let (a, z) = (p.first()?, p.last()?);
        if t <= a.0 {
            return Some(a.1);
        }
        if t >= z.0 {
            return Some(z.1);
        }
        let i = p.partition_point(|q| q.0 <= t) - 1;
        let k = ((t - p[i].0) / (p[i + 1].0 - p[i].0).max(1e-9)) as f32;
        Some(p[i].1 + (p[i + 1].1 - p[i].1) * k)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Marca {
    pub t: f64,
    pub nombre: String,
}

/// Valor de un control de video.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Valor {
    Num(f64),
    Punto([f64; 2]),
    Color([f32; 4]),
    Si(bool),
    Op(u32),
    Texto(String),
}

impl Valor {
    pub fn num(&self) -> f64 {
        match self {
            Valor::Num(v) => *v,
            _ => 0.0,
        }
    }
    fn mezclar(&self, b: &Valor, k: f64) -> Valor {
        let m = |x: f64, y: f64| x + (y - x) * k;
        match (self, b) {
            (Valor::Num(x), Valor::Num(y)) => Valor::Num(m(*x, *y)),
            (Valor::Punto(x), Valor::Punto(y)) => Valor::Punto([m(x[0], y[0]), m(x[1], y[1])]),
            (Valor::Color(x), Valor::Color(y)) => Valor::Color(std::array::from_fn(|i| m(x[i] as f64, y[i] as f64) as f32)),
            _ => self.clone(),
        }
    }
}

/// Un control que puede animarse. Las claves van en segundos desde el inicio del clip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Prop {
    pub v: Valor,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claves: Vec<(f64, Valor)>,
    /// La curva del tramo que empieza en cada cuadro clave, por su segundo. Sin entrada: `Curva::Suave`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub curvas: Vec<(f64, Curva)>,
    /// Pasa por sus claves sin detenerse en cada una, como un trazo a mano. Los tramos con curva propia la respetan.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub libre: bool,
}

impl Prop {
    pub fn num(v: f64) -> Prop {
        Prop { v: Valor::Num(v), claves: vec![], curvas: vec![], libre: false }
    }
    pub fn en(&self, t: f64) -> Valor {
        let c = &self.claves;
        let (Some(a), Some(z)) = (c.first(), c.last()) else { return self.v.clone() };
        if t <= a.0 {
            return a.1.clone();
        }
        if t >= z.0 {
            return z.1.clone();
        }
        let i = c.iter().rposition(|k| k.0 <= t).unwrap_or(0);
        let k = (t - c[i].0) / (c[i + 1].0 - c[i].0).max(1e-9);
        let curva = self.curva_en(c[i].0);
        if self.libre && curva == Curva::Suave {
            return self.trazo(i, k);
        }
        c[i].1.mezclar(&c[i + 1].1, curva.aplicar(k))
    }
    /// Curva que pasa por todas las claves (Hermite, con la pendiente de las vecinas).
    fn trazo(&self, i: usize, k: f64) -> Valor {
        let c = &self.claves;
        let cubica = |n: &dyn Fn(usize) -> f64| {
            let pend = |j: usize| {
                let (a, b) = (j.saturating_sub(1), (j + 1).min(c.len() - 1));
                (n(b) - n(a)) / (c[b].0 - c[a].0).max(1e-9)
            };
            let h = c[i + 1].0 - c[i].0;
            let (k2, k3) = (k * k, k * k * k);
            (2.0 * k3 - 3.0 * k2 + 1.0) * n(i) + (k3 - 2.0 * k2 + k) * h * pend(i) + (3.0 * k2 - 2.0 * k3) * n(i + 1) + (k3 - k2) * h * pend(i + 1)
        };
        match (&c[i].1, &c[i + 1].1) {
            (Valor::Num(_), Valor::Num(_)) => Valor::Num(cubica(&|j| c[j].1.num())),
            (Valor::Punto(_), Valor::Punto(_)) => Valor::Punto([0, 1].map(|e| {
                cubica(&|j| match &c[j].1 {
                    Valor::Punto(p) => p[e],
                    _ => 0.0,
                })
            })),
            (a, b) => a.mezclar(b, k),
        }
    }
    /// La curva del tramo que empieza en el cuadro clave de `t`.
    pub fn curva_en(&self, t: f64) -> Curva {
        self.curvas.iter().find(|c| (c.0 - t).abs() < 0.02).map_or(Curva::Suave, |c| c.1)
    }
    /// Cambia la curva del tramo que empieza en el cuadro clave de `t`.
    pub fn poner_curva(&mut self, t: f64, c: Curva) {
        self.curvas.retain(|x| (x.0 - t).abs() >= 0.02);
        if c != Curva::Suave {
            self.curvas.push((t, c));
        }
    }
    /// Mueve el cuadro clave `i` al segundo `t`; su curva va con él.
    pub fn mover_clave(&mut self, i: usize, t: f64) {
        let Some(viejo) = self.claves.get(i).map(|k| k.0) else { return };
        let curva = self.curva_en(viejo);
        self.curvas.retain(|x| (x.0 - viejo).abs() >= 0.02);
        self.claves[i].0 = t;
        self.claves.sort_by(|a, b| a.0.total_cmp(&b.0));
        self.poner_curva(t, curva);
    }
    /// Corre todas las claves (y sus curvas) `d` segundos.
    pub fn correr(&mut self, d: f64) {
        self.claves.iter_mut().for_each(|k| k.0 += d);
        self.curvas.iter_mut().for_each(|k| k.0 += d);
    }
    /// Índice de la clave que cae en `t` (dentro de media centésima).
    pub fn clave_en(&self, t: f64) -> Option<usize> {
        self.claves.iter().position(|k| (k.0 - t).abs() < 0.02)
    }
    /// Cambia el valor: si está animado, pone o mueve la clave de `t`.
    pub fn poner(&mut self, t: f64, v: Valor) {
        if self.claves.is_empty() {
            self.v = v;
        } else {
            match self.clave_en(t) {
                Some(i) => self.claves[i].1 = v,
                None => {
                    self.claves.push((t, v));
                    self.claves.sort_by(|a, b| a.0.total_cmp(&b.0));
                }
            }
        }
    }
    pub fn alternar_clave(&mut self, t: f64) {
        match self.clave_en(t) {
            Some(i) => {
                let (tk, v) = self.claves.remove(i);
                self.curvas.retain(|x| (x.0 - tk).abs() >= 0.02);
                if self.claves.is_empty() {
                    self.v = v;
                }
            }
            None => {
                let v = self.en(t);
                self.claves.push((t, v));
                self.claves.sort_by(|a, b| a.0.total_cmp(&b.0));
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FxVideo {
    pub id: u64,
    /// Identificador en EffectCraft.
    pub tipo: String,
    pub activo: bool,
    pub p: BTreeMap<String, Prop>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transicion {
    /// Identificador en FilmCraft.
    pub tipo: String,
    pub dur: f64,
    #[serde(default)]
    pub p: BTreeMap<String, Valor>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Titulo {
    pub texto: String,
    pub letra: String,
    /// Alto de la letra, en por ciento del alto del video.
    pub tam: f32,
    pub color: [f32; 4],
    pub borde: f32,
    pub color_borde: [f32; 4],
    pub fondo: [f32; 4],
    pub sombra: f32,
    /// -1 izquierda, 0 centro, 1 derecha.
    pub alinear: i32,
    /// Animación por letra, palabra o renglón.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anim: Option<AnimTexto>,
}

/// Cómo entra o sale cada pedazo de un texto.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovTexto {
    #[default]
    Ninguno,
    Maquina,
    Aparecer,
    Subir,
    Bajar,
    Lado,
    Crecer,
    Encoger,
    Girar,
    Voltear,
    Rebotar,
    Azar,
}

impl MovTexto {
    pub const TODOS: [(MovTexto, &'static str); 12] = [
        (MovTexto::Ninguno, "Nada"),
        (MovTexto::Maquina, "Máquina de escribir"),
        (MovTexto::Aparecer, "Aparecer"),
        (MovTexto::Subir, "Subir"),
        (MovTexto::Bajar, "Bajar"),
        (MovTexto::Lado, "Deslizar de lado"),
        (MovTexto::Crecer, "Crecer"),
        (MovTexto::Encoger, "Llegar desde grande"),
        (MovTexto::Girar, "Girar"),
        (MovTexto::Voltear, "Voltear"),
        (MovTexto::Rebotar, "Rebotar"),
        (MovTexto::Azar, "Aparecer en desorden"),
    ];
}

/// Movimiento que se repite mientras el texto está en pantalla.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BucleTexto {
    #[default]
    Ninguno,
    Ola,
    Temblor,
    Latido,
    Vaiven,
    Arcoiris,
}

impl BucleTexto {
    pub const TODOS: [(BucleTexto, &'static str); 6] = [
        (BucleTexto::Ninguno, "Nada"),
        (BucleTexto::Ola, "Ola"),
        (BucleTexto::Temblor, "Temblor"),
        (BucleTexto::Latido, "Latido"),
        (BucleTexto::Vaiven, "Vaivén"),
        (BucleTexto::Arcoiris, "Arcoíris"),
    ];
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PorTexto {
    #[default]
    Letra,
    Palabra,
    Renglon,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnimTexto {
    pub entrada: MovTexto,
    pub salida: MovTexto,
    pub bucle: BucleTexto,
    pub por: PorTexto,
    /// Segundos que tarda cada pedazo.
    pub dur: f64,
    /// Segundos entre un pedazo y el siguiente.
    pub paso: f64,
    /// Fuerza del bucle (1 = normal).
    pub fuerza: f64,
}

impl Default for AnimTexto {
    fn default() -> Self {
        AnimTexto { entrada: MovTexto::Subir, salida: MovTexto::Ninguno, bucle: BucleTexto::Ninguno, por: PorTexto::Letra, dur: 0.45, paso: 0.05, fuerza: 1.0 }
    }
}

/// Figura para animar.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Forma {
    pub tipo: TipoForma,
    pub color: [f32; 4],
    /// Ancho y alto, en por ciento del alto del video.
    pub tam: [f32; 2],
    /// Grueso de la línea en por ciento del alto; 0 = rellena.
    pub linea: f32,
    /// Esquinas redondas (rectángulo) o puntas (estrella), de 0 a 100.
    pub extra: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoForma {
    #[default]
    Rectangulo,
    Circulo,
    Triangulo,
    Estrella,
    Corazon,
    Flecha,
}

impl TipoForma {
    pub const TODOS: [(TipoForma, &'static str); 6] = [
        (TipoForma::Rectangulo, "Rectángulo"),
        (TipoForma::Circulo, "Círculo"),
        (TipoForma::Triangulo, "Triángulo"),
        (TipoForma::Estrella, "Estrella"),
        (TipoForma::Corazon, "Corazón"),
        (TipoForma::Flecha, "Flecha"),
    ];
}

impl Default for Forma {
    fn default() -> Self {
        Forma { tipo: TipoForma::Rectangulo, color: [1.0, 0.878, 0.4, 1.0], tam: [40.0, 40.0], linea: 0.0, extra: 0.0 }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Contenido {
    #[default]
    Archivo,
    Imagen,
    Titulo(Titulo),
    Color([f32; 4]),
    /// Clip vacío: sus efectos se aplican a todo lo de abajo.
    Ajuste,
    Forma(Forma),
}

/// Posición, tamaño y mezcla de un clip de video.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Movimiento {
    /// Desplazamiento del centro en por ciento del cuadro.
    pub x: Prop,
    pub y: Prop,
    pub escala: Prop,
    /// Estirado a lo ancho y a lo alto, además de la escala.
    #[serde(default = "cien")]
    pub ancho: Prop,
    #[serde(default = "cien")]
    pub alto: Prop,
    pub giro: Prop,
    pub opacidad: Prop,
    /// Punto de apoyo del giro y la escala, desde el centro, en por ciento del clip.
    #[serde(default)]
    pub ancla: [f64; 2],
    /// Recorte en por ciento: izquierda, arriba, derecha, abajo.
    pub recorte: [f32; 4],
    pub fusion: u32,
    pub velocidad: f64,
}

fn cien() -> Prop {
    Prop::num(100.0)
}

impl Movimiento {
    pub fn props(&self) -> [&Prop; 7] {
        [&self.x, &self.y, &self.escala, &self.ancho, &self.alto, &self.giro, &self.opacidad]
    }
    pub fn props_mut(&mut self) -> [&mut Prop; 7] {
        [&mut self.x, &mut self.y, &mut self.escala, &mut self.ancho, &mut self.alto, &mut self.giro, &mut self.opacidad]
    }
}

impl Default for Movimiento {
    fn default() -> Self {
        Movimiento {
            x: Prop::num(0.0),
            y: Prop::num(0.0),
            escala: Prop::num(100.0),
            ancho: cien(),
            alto: cien(),
            ancla: [0.0; 2],
            giro: Prop::num(0.0),
            opacidad: Prop::num(100.0),
            recorte: [0.0; 4],
            fusion: 0,
            velocidad: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Capa {
    pub nombre: String,
    pub visible: bool,
    #[serde(default)]
    pub bloqueada: bool,
}

fn bpm_inicial() -> f64 {
    120.0
}

fn una_capa() -> Vec<Capa> {
    vec![Capa { nombre: "V1".into(), visible: true, bloqueada: false }]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClipVideo {
    pub id: u64,
    pub ruta: PathBuf,
    pub inicio: f64,
    pub desde: f64,
    pub dur: f64,
    pub total: f64,
    pub ancho: u32,
    pub alto: u32,
    pub fps: f64,
    /// Pista de video: 0 es la de abajo.
    #[serde(default)]
    pub capa: usize,
    #[serde(default)]
    pub contenido: Contenido,
    #[serde(default)]
    pub mov: Movimiento,
    #[serde(default)]
    pub fx: Vec<FxVideo>,
    #[serde(default)]
    pub entrada: Option<Transicion>,
    #[serde(default)]
    pub salida: Option<Transicion>,
    #[serde(default)]
    pub grupo: Option<u64>,
    /// Está en la ventana de animación.
    #[serde(default)]
    pub animado: bool,
    /// Efectos conectados por nodos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodos: Option<crate::nodos::Grafo>,
    /// Color con que se tiñe en la línea de tiempo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
}

impl ClipVideo {
    pub fn fin(&self) -> f64 {
        self.inicio + self.dur
    }
    /// Segundo del archivo que se ve en el segundo `t` de la línea de tiempo.
    pub fn fuente(&self, t: f64) -> f64 {
        self.desde + (t - self.inicio) * self.mov.velocidad
    }
    /// Los archivos tienen principio y fin; un título o una imagen se puede estirar sin límite.
    pub fn estirable(&self) -> bool {
        self.contenido != Contenido::Archivo
    }
    /// Se puede mostrar el cuadro tal cual, sin componer.
    pub fn simple(&self) -> bool {
        self.contenido == Contenido::Archivo
            && self.mov == Movimiento::default()
            && !self.fx.iter().any(|f| f.activo)
            && !self.nodos.as_ref().is_some_and(|g| g.activo)
            && self.entrada.is_none()
            && self.salida.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Instrumento {
    /// Sintetizador de tres osciladores; valores de `instrumentos::TRIO`.
    Trio(Vec<f32>),
    Bateria,
    Muestra { ruta: PathBuf, raiz: u8 },
}

/// Tiempos en pulsos (negras).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Nota {
    pub tecla: u8,
    pub vel: u8,
    pub ini: f64,
    pub dur: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Patron {
    pub id: u64,
    pub nombre: String,
    /// Largo en pulsos; al acabar se repite.
    pub largo: f64,
    pub notas: Vec<Nota>,
}

/// Un patrón puesto en la Playlist. Inicio y duración en segundos.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClipPatron {
    pub id: u64,
    pub patron: u64,
    pub inicio: f64,
    pub dur: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pista {
    pub id: u64,
    pub nombre: String,
    pub vol_db: f32,
    /// -1 izquierda, 1 derecha.
    pub pan: f32,
    pub mudo: bool,
    pub solo: bool,
    /// Aquí caen las grabaciones.
    pub armada: bool,
    pub clips: Vec<Clip>,
    pub fx: Vec<Fx>,
    #[serde(default)]
    pub instrumento: Option<Instrumento>,
    #[serde(default)]
    pub patrones: Vec<ClipPatron>,
    #[serde(default)]
    pub bloqueada: bool,
    #[serde(default)]
    pub auto: Vec<Automatizacion>,
    /// Mezclador al que sale; `None`: al maestro.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bus: Option<u64>,
}

/// Mezclador: junta lo que se le mande, con sus efectos, volumen y paneo, y sale al maestro.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bus {
    pub id: u64,
    pub nombre: String,
    pub vol_db: f32,
    pub pan: f32,
    pub mudo: bool,
    pub solo: bool,
    pub fx: Vec<Fx>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Proyecto {
    pub pistas: Vec<Pista>,
    pub video: Vec<ClipVideo>,
    #[serde(default = "una_capa")]
    pub capas: Vec<Capa>,
    pub maestro_db: f32,
    pub fx_maestro: Vec<Fx>,
    #[serde(default)]
    pub buses: Vec<Bus>,
    pub sig_id: u64,
    /// Formato del video exportado; lo fija el primer video importado.
    pub ancho: u32,
    pub alto: u32,
    pub fps: f64,
    /// Corrección manual de las grabaciones: positivo las adelanta.
    pub ajuste_ms: f32,
    #[serde(default = "bpm_inicial")]
    pub bpm: f64,
    #[serde(default)]
    pub patrones: Vec<Patron>,
    #[serde(default)]
    pub marcas: Vec<Marca>,
    /// Tramo elegido (entrada y salida) en segundos.
    #[serde(default)]
    pub tramo: (Option<f64>, Option<f64>),
    /// El usuario eligió tamaño y cuadros por segundo: el primer video ya no los cambia.
    #[serde(default)]
    pub fijo: bool,
}

impl Proyecto {
    pub fn nuevo() -> Self {
        let mut p = Proyecto {
            pistas: vec![],
            video: vec![],
            capas: una_capa(),
            maestro_db: 0.0,
            fx_maestro: vec![],
            buses: vec![],
            sig_id: 1,
            ancho: 1920,
            alto: 1080,
            fps: 30.0,
            ajuste_ms: 0.0,
            bpm: 120.0,
            patrones: vec![],
            marcas: vec![],
            tramo: (None, None),
            fijo: false,
        };
        let mut f = p.nuevo_fx(TipoFx::Limitador);
        // El del maestro solo cuida que no truene.
        f.p = TipoFx::Limitador.defaults();
        p.fx_maestro.push(f);
        // Sin pistas: cada quien agrega las que necesite desde la consola o la línea de tiempo.
        p
    }

    pub fn id(&mut self) -> u64 {
        self.sig_id += 1;
        self.sig_id
    }

    pub fn nuevo_fx(&mut self, tipo: TipoFx) -> Fx {
        let mut p = tipo.arranque();
        if tipo == TipoFx::Tartamudeo {
            p[0] = self.bpm as f32;
        }
        Fx { id: self.id(), tipo, activo: true, p }
    }

    /// Las pistas de video con su nombre de siempre («V1», «V2»…) lo siguen según su lugar, para que la placa y el
    /// nombre digan lo mismo al insertar o quitar pistas. Los nombres que puso la persona no se tocan.
    pub fn renombrar_capas(&mut self) {
        for (i, c) in self.capas.iter_mut().enumerate() {
            let de_siempre = c.nombre.strip_prefix('V').is_some_and(|n| !n.is_empty() && n.chars().all(|x| x.is_ascii_digit()));
            let toca = format!("V{}", i + 1);
            if de_siempre && c.nombre != toca {
                c.nombre = toca;
            }
        }
    }

    pub fn nueva_pista(&mut self, nombre: &str) -> u64 {
        let id = self.id();
        self.pistas.push(Pista {
            id,
            nombre: nombre.into(),
            vol_db: 0.0,
            pan: 0.0,
            mudo: false,
            solo: false,
            armada: false,
            clips: vec![],
            fx: vec![],
            instrumento: None,
            patrones: vec![],
            bloqueada: false,
            auto: vec![],
            bus: None,
        });
        id
    }

    pub fn nuevo_bus(&mut self, nombre: &str) -> u64 {
        let id = self.id();
        self.buses.push(Bus { id, nombre: nombre.into(), vol_db: 0.0, pan: 0.0, mudo: false, solo: false, fx: vec![] });
        id
    }

    pub fn bus(&mut self, id: u64) -> Option<&mut Bus> {
        self.buses.iter_mut().find(|b| b.id == id)
    }

    /// Quita un mezclador: lo que iba a él vuelve a su pista o al maestro.
    pub fn quitar_bus(&mut self, id: u64) {
        self.buses.retain(|b| b.id != id);
        for p in &mut self.pistas {
            p.bus = p.bus.filter(|b| *b != id);
            p.clips.iter_mut().for_each(|c| c.bus = c.bus.filter(|b| *b != id));
        }
    }

    /// Pista lista para cantar: armada y con ecualizador y compresor de voz (se pueden quitar o cambiar).
    pub fn nueva_pista_grabacion(&mut self) -> u64 {
        let n = self.pistas.iter().filter(|p| p.nombre.starts_with("Grabación")).count() + 1;
        let id = self.nueva_pista(&crate::idioma::tf("Grabación {}", &[&n]));
        let mut fx = vec![];
        for t in [TipoFx::Ecualizador, TipoFx::Compresor] {
            let mut f = self.nuevo_fx(t);
            f.p = t.ajustes()[0].1.to_vec();
            fx.push(f);
        }
        self.pista(id).unwrap().fx = fx;
        self.armar(id);
        id
    }

    /// Segundos que dura un pulso.
    pub fn pulso(&self) -> f64 {
        60.0 / self.bpm.clamp(20.0, 400.0)
    }

    /// Pista con instrumento y un patrón vacío de un compás puesto en `t`.
    pub fn nueva_pista_instrumento(&mut self, nombre: &str, ins: Instrumento, t: f64) -> (u64, u64) {
        let pista = self.nueva_pista(nombre);
        self.pista(pista).unwrap().instrumento = Some(ins);
        let pat = self.nuevo_patron(nombre);
        let clip = self.id();
        let dur = 4.0 * self.pulso();
        self.pista(pista).unwrap().patrones.push(ClipPatron { id: clip, patron: pat, inicio: t, dur });
        (pista, pat)
    }

    pub fn nuevo_patron(&mut self, nombre: &str) -> u64 {
        let id = self.id();
        self.patrones.push(Patron { id, nombre: nombre.into(), largo: 4.0, notas: vec![] });
        id
    }

    pub fn patron(&mut self, id: u64) -> Option<&mut Patron> {
        self.patrones.iter_mut().find(|p| p.id == id)
    }

    /// Notas de una pista en cuadros (inicio, fin, tecla, velocidad), ordenadas, con las repeticiones del patrón.
    pub fn notas_de(&self, p: &Pista) -> Vec<(i64, i64, u8, u8)> {
        let s = self.pulso() * SR as f64;
        let mut v = vec![];
        for c in &p.patrones {
            let Some(pat) = self.patrones.iter().find(|x| x.id == c.patron) else { continue };
            let (ini, fin) = (c.inicio * SR as f64, (c.inicio + c.dur) * SR as f64);
            let vuelta = pat.largo.max(0.25) * s;
            let mut k = 0.0;
            while ini + k * vuelta < fin - 1.0 {
                for n in pat.notas.iter().filter(|n| n.ini < pat.largo) {
                    let a = ini + k * vuelta + n.ini * s;
                    if a < fin - 1.0 {
                        v.push((a.round() as i64, (a + n.dur * s).min(fin).round() as i64, n.tecla, n.vel));
                    }
                }
                k += 1.0;
            }
        }
        v.sort_by_key(|n| n.0);
        v
    }

    pub fn armar(&mut self, id: u64) {
        for p in &mut self.pistas {
            p.armada = p.id == id;
        }
    }

    pub fn pista(&mut self, id: u64) -> Option<&mut Pista> {
        self.pistas.iter_mut().find(|p| p.id == id)
    }

    pub fn clip(&mut self, id: u64) -> Option<&mut Clip> {
        self.pistas.iter_mut().flat_map(|p| p.clips.iter_mut()).find(|c| c.id == id)
    }

    pub fn fin_audio(&self) -> f64 {
        let clips = self.pistas.iter().flat_map(|p| &p.clips).map(|c| c.inicio + c.dur);
        let pat = self.pistas.iter().flat_map(|p| &p.patrones).map(|c| c.inicio + c.dur);
        clips.chain(pat).fold(0.0, f64::max)
    }

    pub fn fin(&self) -> f64 {
        self.video.iter().map(|c| c.inicio + c.dur).fold(self.fin_audio(), f64::max)
    }

    /// El clip de una capa en `t` (el último agregado tapa a los demás).
    pub fn video_en(&self, capa: usize, t: f64) -> Option<&ClipVideo> {
        self.video.iter().rev().find(|c| c.capa == capa && t >= c.inicio && t < c.fin())
    }

    pub fn clip_video(&mut self, id: u64) -> Option<&mut ClipVideo> {
        self.video.iter_mut().find(|c| c.id == id)
    }

    /// Todo el video sale de una sola capa sin nada encima: se puede exportar por el camino rápido.
    pub fn video_simple(&self) -> bool {
        self.video.iter().all(|c| c.simple() && c.capa == 0) && self.capas.iter().all(|c| c.visible)
    }

    /// Clip listo para usar en la capa 0.
    pub fn nuevo_video(&mut self, ruta: PathBuf, contenido: Contenido, inicio: f64, dur: f64) -> ClipVideo {
        ClipVideo {
            id: self.id(),
            ruta,
            inicio,
            desde: 0.0,
            dur,
            total: dur,
            ancho: self.ancho,
            alto: self.alto,
            fps: self.fps,
            capa: 0,
            contenido,
            mov: Movimiento::default(),
            fx: vec![],
            entrada: None,
            salida: None,
            grupo: None,
            animado: false,
            nodos: None,
            color: None,
        }
    }

    /// Parte un clip (de audio o video) en el segundo `t` de la línea de tiempo.
    pub fn dividir(&mut self, id: u64, t: f64) -> bool {
        let nuevo = self.id();
        for p in &mut self.pistas {
            if let Some(i) = p.clips.iter().position(|c| c.id == id) {
                let c = &mut p.clips[i];
                let corte = t - c.inicio;
                if corte < 0.01 || corte > c.dur - 0.01 {
                    return false;
                }
                let mut b = c.clone();
                b.id = nuevo;
                b.inicio = t;
                b.dur = c.dur - corte;
                b.fade_in = 0.0;
                b.fade_out = b.fade_out.min(b.dur);
                // Un clip invertido lee el archivo de atrás hacia adelante.
                if c.invertido {
                    c.desde += (c.dur - corte) * c.velocidad;
                } else {
                    b.desde += corte * c.velocidad;
                }
                c.dur = corte;
                c.fade_out = 0.0;
                c.fade_in = c.fade_in.min(corte);
                p.clips.insert(i + 1, b);
                return true;
            }
        }
        if let Some(i) = self.video.iter().position(|c| c.id == id) {
            let c = &mut self.video[i];
            let corte = t - c.inicio;
            if corte < 0.01 || corte > c.dur - 0.01 {
                return false;
            }
            let mut b = c.clone();
            b.id = nuevo;
            b.inicio = t;
            b.desde += corte * c.mov.velocidad;
            b.dur = c.dur - corte;
            b.entrada = None;
            c.salida = None;
            c.dur = corte;
            // Las animaciones siguen donde iban.
            let m = &mut b.mov;
            for p in m.props_mut() {
                p.claves.iter_mut().for_each(|k| k.0 -= corte);
            }
            for p in b.fx.iter_mut().flat_map(|f| f.p.values_mut()) {
                p.claves.iter_mut().for_each(|k| k.0 -= corte);
            }
            let ids: Vec<u64> = (0..b.fx.len() as u64).map(|i| nuevo + 1 + i).collect();
            b.fx.iter_mut().zip(&ids).for_each(|(f, id)| f.id = *id);
            self.sig_id += ids.len() as u64;
            self.video.insert(i + 1, b);
            return true;
        }
        false
    }

    pub fn borrar_clip(&mut self, id: u64) {
        for p in &mut self.pistas {
            p.patrones.retain(|c| c.id != id);
            p.clips.retain(|c| c.id != id);
        }
        self.video.retain(|c| c.id != id);
    }

    /// Quita del clip el tramo [a, b] (segundos de la línea de tiempo). Con `juntar`, lo que sigue del
    /// mismo clip se recorre para cerrar el hueco.
    pub fn quitar_tramo(&mut self, id: u64, a: f64, b: f64, juntar: bool) {
        let Some(c) = self.clip(id) else { return };
        let (ini, fin) = (c.inicio, c.inicio + c.dur);
        let (a, b) = (a.max(ini), b.min(fin));
        if b - a < 0.01 {
            return;
        }
        let mut medio = id;
        if a - ini >= 0.01 && self.dividir(id, a) {
            medio = self.sig_id;
        }
        if fin - b >= 0.01 && self.dividir(medio, b) && juntar {
            let resto = self.sig_id;
            if let Some(r) = self.clip(resto) {
                r.inicio = a;
            }
        }
        self.borrar_clip(medio);
    }
}

pub fn db(v: f32) -> f32 {
    10f32.powf(v / 20.0)
}

pub fn a_db(v: f32) -> f32 {
    20.0 * v.max(1e-6).log10()
}
