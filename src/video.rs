//! Vista previa: un hilo decodifica con ffmpeg, compone el cuadro y lo entrega; el reloj del audio decide cuál toca.
use crate::composicion::{self, Cuadro, Fuente};
use crate::medios;
use crate::modelo::{ClipVideo, Contenido, Proyecto};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TryRecvError, bounded, unbounded};
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cuadros seguidos de un archivo desde un punto.
pub struct Lector {
    hijo: Child,
    rx: Receiver<Cuadro>,
    ruta: PathBuf,
    tam: (usize, usize),
    ultimo: Option<Arc<Cuadro>>,
    /// Los últimos que pasaron, para los efectos de tiempo.
    cola: std::collections::VecDeque<Arc<Cuadro>>,
    sig: Option<Cuadro>,
    pub fin: bool,
    paso: f64,
}

impl Drop for Lector {
    fn drop(&mut self) {
        let _ = self.hijo.kill();
        let _ = self.hijo.wait();
    }
}

impl Lector {
    pub fn abrir(c: &ClipVideo, desde: f64, tam: (usize, usize), fps_max: f64) -> std::io::Result<Lector> {
        let imagen = c.contenido == Contenido::Imagen;
        let fps = c.fps.min(fps_max).max(1.0);
        let (w, h) = tam;
        // Se alinea a la rejilla de cuadros para que el mismo instante dé siempre el mismo cuadro.
        let desde = ((desde * fps + 0.01).floor() / fps).max(0.0);
        let mut com = medios::ffmpeg();
        com.args(["-threads", "4"]);
        if !imagen {
            com.args(["-ss", &format!("{desde:.6}")]);
        }
        com.arg("-i").arg(&c.ruta).args(["-an", "-sn", "-map", "0:v:0", "-vf"]);
        let escala = format!("scale={w}:{h}:flags=bilinear,setsar=1");
        com.arg(if imagen { escala } else { format!("fps={}:start_time=0,{escala}", medios::fps_fraccion(fps)) });
        if imagen {
            com.args(["-frames:v", "1"]);
        }
        let mut hijo =
            com.args(["-pix_fmt", "rgba", "-f", "rawvideo", "pipe:1"]).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
        let mut o = hijo.stdout.take().unwrap();
        let (tx, rx) = bounded(4);
        std::thread::spawn(move || {
            let mut i = 0u64;
            loop {
                let mut rgba = vec![0u8; w * h * 4];
                if o.read_exact(&mut rgba).is_err() {
                    return;
                }
                let t = desde + i as f64 / fps;
                i += 1;
                if tx.send(Cuadro { t, ancho: w, alto: h, rgba }).is_err() {
                    return;
                }
            }
        });
        Ok(Lector { hijo, rx, ruta: c.ruta.clone(), tam, ultimo: None, cola: Default::default(), sig: None, fin: false, paso: 1.0 / fps })
    }

    /// Avanza hasta el cuadro que toca en `t`. Sin `esperar`, entrega lo más nuevo que haya.
    fn hasta(&mut self, t: f64, esperar: bool) -> Option<Arc<Cuadro>> {
        loop {
            if self.sig.is_none() {
                let r = if esperar {
                    self.rx.recv().map_err(|_| TryRecvError::Disconnected)
                } else if self.ultimo.is_none() {
                    // El primero se espera un momento: es el que se pidió ver.
                    self.rx.recv_timeout(Duration::from_secs(4)).map_err(|e| match e {
                        RecvTimeoutError::Timeout => TryRecvError::Empty,
                        RecvTimeoutError::Disconnected => TryRecvError::Disconnected,
                    })
                } else {
                    self.rx.try_recv()
                };
                match r {
                    Ok(c) => self.sig = Some(c),
                    Err(e) => {
                        self.fin |= e == TryRecvError::Disconnected;
                        return self.ultimo.clone();
                    }
                }
            }
            if self.ultimo.is_some() && self.sig.as_ref().unwrap().t > t + 0.001 {
                return self.ultimo.clone();
            }
            self.ultimo = self.sig.take().map(Arc::new);
            if let Some(u) = &self.ultimo {
                self.cola.push_back(u.clone());
                if self.cola.len() > 10 {
                    self.cola.pop_front();
                }
            }
        }
    }

    fn sirve(&self, c: &ClipVideo, t: f64, tam: (usize, usize), tocando: bool) -> bool {
        if self.ruta != c.ruta || self.tam != tam {
            return false;
        }
        if c.contenido == Contenido::Imagen {
            return true;
        }
        let Some(u) = &self.ultimo else { return !self.fin };
        // Hacia adelante y cerca: más barato seguir leyendo que volver a abrir.
        let lejos = if tocando { 1.5 } else { 0.6 };
        t >= u.t - self.paso * 0.5 && (self.fin || t < u.t + lejos)
    }
}

/// Lectores abiertos, uno por clip.
pub struct Lectores {
    v: HashMap<u64, (Lector, u32)>,
    ronda: u32,
    pub fps_max: f64,
    pub tocando: bool,
    /// Exportación: espera cada cuadro en vez de mostrar el más nuevo.
    pub exacto: bool,
}

impl Lectores {
    pub fn nuevos(fps_max: f64, exacto: bool) -> Self {
        Lectores { v: HashMap::new(), ronda: 0, fps_max, tocando: exacto, exacto }
    }
    /// Cierra los que no se usaron en las últimas rondas.
    pub fn limpiar(&mut self) {
        self.ronda += 1;
        let r = self.ronda;
        self.v.retain(|_, (_, u)| r - *u < 40);
    }
}

impl Fuente for Lectores {
    fn recientes(&self, c: &ClipVideo) -> Vec<Arc<Cuadro>> {
        self.v.get(&c.id).map(|(l, _)| l.cola.iter().cloned().collect()).unwrap_or_default()
    }
    fn cuadro(&mut self, c: &ClipVideo, t: f64, ancho: usize, alto: usize) -> Option<Arc<Cuadro>> {
        let tam = (ancho, alto);
        if !self.v.get(&c.id).is_some_and(|(l, _)| l.sirve(c, t, tam, self.tocando)) {
            self.v.remove(&c.id);
            self.v.insert(c.id, (Lector::abrir(c, t, tam, self.fps_max).ok()?, 0));
        }
        let (l, u) = self.v.get_mut(&c.id)?;
        *u = self.ronda;
        l.hasta(t, self.exacto)
    }
}

pub struct Pedido {
    pub p: Arc<Proyecto>,
    pub t: f64,
    pub tocando: bool,
    pub tam: (usize, usize),
}

/// Escalas de la vista previa mientras se reproduce: si un cuadro no alcanza, se baja un escalón.
const ESCALAS: [f64; 4] = [1.0, 0.75, 0.5, 0.35];

/// El tamaño de la vista previa a la escala `s`, en pares.
pub fn a_escala((w, h): (usize, usize), s: f64) -> (usize, usize) {
    let par = |x: usize| ((x as f64 * s / 2.0).round() as usize * 2).max(2);
    (par(w), par(h))
}

/// Hilo que compone la vista previa. Siempre atiende el pedido más nuevo.
pub struct VistaPrevia {
    tx: Sender<Pedido>,
    rx: Receiver<Cuadro>,
    /// Pedidos sin respuesta.
    pub pendientes: u32,
    ultimo: Option<(f64, u64, (usize, usize))>,
}

impl Default for VistaPrevia {
    fn default() -> Self {
        let (tx, entra) = unbounded::<Pedido>();
        let (sale, rx) = unbounded();
        std::thread::spawn(move || {
            let mut l = Lectores::nuevos(30.0, false);
            // Escalón de `ESCALAS`: sube al reproducir si no alcanza, y vuelve a 0 al pausar.
            let mut nivel = 0;
            // Cuadros lentos seguidos: uno solo (abrir el video, saltar) no cuenta.
            let mut lentos = 0;
            while let Ok(mut q) = entra.recv() {
                while let Ok(n) = entra.try_recv() {
                    q = n;
                }
                l.tocando = q.tocando;
                l.limpiar();
                if !q.tocando {
                    (nivel, lentos) = (0, 0);
                }
                let (w, h) = a_escala(q.tam, ESCALAS[nivel]);
                let t0 = Instant::now();
                let rgba = composicion::componer(&q.p, q.t, w, h, &mut l);
                // Tres cuartos de lo que dura un cuadro: el resto es para pedirlo y mostrarlo.
                let presupuesto = 0.75 / q.p.fps.clamp(1.0, 30.0);
                lentos = if q.tocando && t0.elapsed().as_secs_f64() > presupuesto { lentos + 1 } else { 0 };
                if lentos >= 3 && nivel + 1 < ESCALAS.len() {
                    (nivel, lentos) = (nivel + 1, 0);
                }
                if sale.send(Cuadro { t: q.t, ancho: w, alto: h, rgba }).is_err() {
                    return;
                }
            }
        });
        VistaPrevia { tx, rx, pendientes: 0, ultimo: None }
    }
}

impl VistaPrevia {
    /// Pide el cuadro de `t` si hace falta y devuelve el más nuevo que esté listo. `version` cambia con cada edición.
    pub fn actualizar(&mut self, p: &Arc<Proyecto>, version: u64, t: f64, tocando: bool, tam: (usize, usize)) -> Option<Cuadro> {
        let mut r = None;
        while let Ok(c) = self.rx.try_recv() {
            r = Some(c);
            self.pendientes = 0;
        }
        let fps = p.fps.clamp(1.0, 30.0);
        // Un pedido por cuadro de video, no por cuadro de pantalla.
        let clave = ((t * fps + 0.01).floor() / fps, version, tam);
        if self.ultimo != Some(clave) && self.pendientes == 0 {
            self.ultimo = Some(clave);
            self.pendientes = 1;
            let _ = self.tx.send(Pedido { p: p.clone(), t: clave.0, tocando, tam });
        }
        r
    }
}

/// Tira de miniaturas de un video, una cada `paso` segundos.
pub struct Tira {
    pub paso: f64,
    pub ancho: usize,
    pub alto: usize,
    pub rgba: Vec<Vec<u8>>,
}

pub fn tira(ruta: &std::path::Path, dur: f64, razon: f64) -> Option<Tira> {
    let paso = (dur / 60.0).max(1.0);
    let alto = 40;
    let ancho = ((alto as f64 * razon / 2.0).round() as usize * 2).clamp(16, 120);
    let s = medios::ffmpeg()
        .args(["-skip_frame", "nokey", "-i"])
        .arg(ruta)
        .args(["-an", "-vf", &format!("fps=1/{paso}:round=near,scale={ancho}:{alto}"), "-pix_fmt", "rgba", "-f", "rawvideo", "pipe:1"])
        .output()
        .ok()?;
    let rgba: Vec<Vec<u8>> = s.stdout.chunks_exact(ancho * alto * 4).map(|c| c.to_vec()).collect();
    (!rgba.is_empty()).then_some(Tira { paso, ancho, alto, rgba })
}
