//! El motor: mezcla las pistas, aplica los efectos y lleva la posición. Lo usan igual la reproducción y la exportación.
//!
//! Los efectos con retardo (el limitador mira hacia adelante) se compensan leyendo el material por adelantado, así todo
//! sale alineado con el video.
use crate::efectos::{self, Procesador, TipoFx};
use crate::medios::Audio;
use crate::instrumentos::InstrRT;
use crate::modelo::{Automatizacion, Destino, Fx, Instrumento, Proyecto, SR, db};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type Almacen = HashMap<PathBuf, Arc<Audio>>;
pub const BLOQUE: usize = 256;
/// Sonido del micrófono en camino a los audífonos (48 kHz estéreo).
pub type Vivo = Arc<Mutex<VecDeque<[f32; 2]>>>;

struct FxRT {
    id: u64,
    activo: bool,
    p: Vec<f32>,
    proc: Box<dyn Procesador>,
}

#[derive(Default)]
struct Cadena(Vec<FxRT>);

impl Cadena {
    fn latencia(&self) -> usize {
        self.0.iter().filter(|f| f.activo).map(|f| f.proc.latencia()).sum()
    }
    fn procesar(&mut self, buf: &mut [f32]) {
        for f in self.0.iter_mut().filter(|f| f.activo) {
            f.proc.procesar(buf);
        }
    }
    fn limpiar(&mut self) {
        self.0.iter_mut().for_each(|f| f.proc.limpiar());
    }
}

struct ClipRT {
    inicio: i64,
    desde: i64,
    dur: i64,
    g: f32,
    fi: i64,
    fo: i64,
    inv: bool,
    audio: Arc<Audio>,
}

impl ClipRT {
    /// Suma el clip en `buf`, que empieza en el cuadro `t`.
    fn sumar(&self, t: i64, buf: &mut [f32]) {
        let n = (buf.len() / 2) as i64;
        let a = (self.inicio - t).clamp(0, n);
        let b = (self.inicio + self.dur - t).clamp(0, n);
        let s = &self.audio.cuadros;
        for i in a..b {
            let k = t + i - self.inicio;
            let j = self.desde + if self.inv { self.dur - 1 - k } else { k };
            let Some(x) = usize::try_from(j).ok().and_then(|j| s.get(j)) else { continue };
            let mut g = self.g;
            if k < self.fi {
                g *= k as f32 / self.fi as f32;
            }
            if self.dur - k < self.fo {
                g *= (self.dur - k) as f32 / self.fo as f32;
            }
            buf[2 * i as usize] += x[0] * g;
            buf[2 * i as usize + 1] += x[1] * g;
        }
    }
}

struct PistaRT {
    id: u64,
    g: [f32; 2],
    meta: [f32; 2],
    clips: Vec<ClipRT>,
    cadena: Cadena,
    pico: [f32; 2],
    instr: Option<InstrRT>,
    ins: Option<Instrumento>,
    muestra: Option<Arc<Audio>>,
    auto: Vec<Automatizacion>,
    vol_db: f32,
    pan: f32,
    suena: bool,
    /// Encendidos y apagados por cuadro: (cuadro, encender, tecla, velocidad). Los apagados van primero.
    eventos: Vec<(i64, bool, u8, u8)>,
}

impl PistaRT {
    /// Pone volumen, paneo y controles de efectos como van en el segundo `t`.
    fn automatizar(&mut self, t: f64) {
        if self.auto.is_empty() {
            return;
        }
        let (mut vol, mut pan) = (self.vol_db, self.pan);
        for a in &self.auto {
            let Some(v) = a.en(t) else { continue };
            match a.destino {
                Destino::Volumen => vol = v,
                Destino::Paneo => pan = v,
                Destino::Fx(id, k) => {
                    if let Some(f) = self.cadena.0.iter_mut().find(|f| f.id == id) {
                        if f.p.get(k).is_some_and(|x| (*x - v).abs() > 1e-4) {
                            f.p[k] = v;
                            f.proc.ajustar(&f.p);
                        }
                    }
                }
            }
        }
        self.meta = ganancias(if self.suena { db(vol) } else { 0.0 }, pan);
    }

    /// Suma clips e instrumento desde el cuadro `t`.
    fn fuente(&mut self, t: i64, b: &mut [f32]) {
        self.clips.iter().for_each(|c| c.sumar(t, b));
        let Some(ins) = &mut self.instr else { return };
        let n = (b.len() / 2) as i64;
        let mut i = self.eventos.partition_point(|e| e.0 < t);
        let mut de = 0i64;
        while let Some(e) = self.eventos.get(i).filter(|e| e.0 < t + n) {
            let k = e.0 - t;
            if k > de {
                ins.sonar(&mut b[2 * de as usize..2 * k as usize]);
                de = k;
            }
            ins.nota(e.2, e.3, e.1);
            i += 1;
        }
        ins.sonar(&mut b[2 * de as usize..]);
    }
}

/// Todo lo que se arma fuera del hilo de audio antes de entregarlo al motor.
pub struct Preparado {
    pistas: Vec<(PistaRT, Vec<Fx>)>,
    fx_maestro: Vec<Fx>,
    g_maestro: f32,
    nuevos: HashMap<u64, FxRT>,
    fin: i64,
    armada: Option<u64>,
}

fn ganancias(g: f32, pan: f32) -> [f32; 2] {
    [g * (1.0 - pan).min(1.0), g * (1.0 + pan).min(1.0)]
}

pub fn cuadros(s: f64) -> i64 {
    (s * SR as f64).round() as i64
}

pub fn preparar(p: &Proyecto, almacen: &Almacen, existentes: &[u64]) -> Preparado {
    let hay_solo = p.pistas.iter().any(|x| x.solo);
    let mut nuevos = HashMap::new();
    for f in p.pistas.iter().flat_map(|t| &t.fx).chain(&p.fx_maestro) {
        if !existentes.contains(&f.id) {
            nuevos.insert(
                f.id,
                FxRT { id: f.id, activo: f.activo, p: f.p.clone(), proc: efectos::crear(f.tipo, &f.p) },
            );
        }
    }
    let pistas = p
        .pistas
        .iter()
        .map(|t| {
            let suena = !t.mudo && (!hay_solo || t.solo);
            let g = if suena { db(t.vol_db) } else { 0.0 };
            let meta = ganancias(g, t.pan);
            let clips = t
                .clips
                .iter()
                .filter_map(|c| {
                    Some(ClipRT {
                        inicio: cuadros(c.inicio),
                        // La versión estirada dura `1/velocidad`: el punto de arranque se corre igual.
                        desde: cuadros(c.desde / c.velocidad.max(0.01)),
                        dur: cuadros(c.dur),
                        g: db(c.ganancia_db),
                        fi: cuadros(c.fade_in),
                        fo: cuadros(c.fade_out),
                        inv: c.invertido,
                        audio: almacen.get(&c.clave())?.clone(),
                    })
                })
                .collect();
            let mut eventos: Vec<(i64, bool, u8, u8)> = vec![];
            for (a, b, tecla, vel) in p.notas_de(t) {
                eventos.push((a, true, tecla, vel));
                eventos.push((b.max(a + 1), false, tecla, 0));
            }
            eventos.sort_by_key(|e| (e.0, e.1));
            let muestra = match &t.instrumento {
                Some(Instrumento::Muestra { ruta, .. }) => almacen.get(ruta).cloned(),
                _ => None,
            };
            // El instrumento se crea siempre aquí; si la pista ya sonaba, el motor conserva el suyo.
            let instr = t.instrumento.as_ref().map(|i| InstrRT::nuevo(i, muestra.clone()));
            let rt = PistaRT {
                id: t.id,
                g: meta,
                meta,
                clips,
                cadena: Cadena::default(),
                pico: [0.0; 2],
                instr,
                ins: t.instrumento.clone(),
                muestra,
                eventos,
                auto: t.auto.iter().filter(|a| a.activa && !a.puntos.is_empty()).cloned().collect(),
                vol_db: t.vol_db,
                pan: t.pan,
                suena,
            };
            (rt, t.fx.clone())
        })
        .collect();
    Preparado {
        pistas,
        fx_maestro: p.fx_maestro.clone(),
        g_maestro: db(p.maestro_db),
        nuevos,
        fin: cuadros(p.fin()),
        armada: p.pistas.iter().find(|t| t.armada).map(|t| t.id),
    }
}

#[derive(Default)]
pub struct Motor {
    pistas: Vec<PistaRT>,
    maestro: Cadena,
    g_maestro: f32,
    meta_maestro: f32,
    /// Posición que sale por la salida, en cuadros.
    pub pos: i64,
    /// Posición de la mezcla antes de los efectos maestros (adelantada por su retardo).
    pos_bus: i64,
    pub tocando: bool,
    pub grabando: bool,
    pub fin: i64,
    pico_maestro: [f32; 2],
    /// «Escucharme»: el micrófono pasa por los efectos de la pista armada.
    pub vivo: Option<Vivo>,
    /// Monitor de origen: audio y cuadro en el que va.
    pub origen: Option<(Arc<Audio>, i64)>,
    /// Cuadros que faltan de una nota de prueba.
    previa: i64,
    /// Cuadros por pulso del metrónomo; 0 = apagado.
    pub metronomo: i64,
    armada: Option<u64>,
    tmp: Vec<f32>,
}

impl Motor {
    pub fn ids_fx(&self) -> Vec<u64> {
        self.pistas.iter().flat_map(|p| &p.cadena.0).chain(&self.maestro.0).map(|f| f.id).collect()
    }

    /// Cambia al estado preparado conservando los efectos que ya sonaban. Devuelve lo viejo para soltarlo fuera.
    pub fn aplicar(&mut self, mut n: Preparado) -> impl Sized + use<> {
        let antes: usize = self.latencias();
        let mut viejos: HashMap<u64, FxRT> = HashMap::new();
        let mut pistas_viejas = std::mem::take(&mut self.pistas);
        for p in &mut pistas_viejas {
            for f in p.cadena.0.drain(..) {
                viejos.insert(f.id, f);
            }
        }
        for f in self.maestro.0.drain(..) {
            viejos.insert(f.id, f);
        }
        let mut cadena = |lista: &[Fx]| {
            let mut c = Cadena::default();
            for f in lista {
                if let Some(mut r) = viejos.remove(&f.id).or_else(|| n.nuevos.remove(&f.id)) {
                    if r.p != f.p {
                        r.proc.ajustar(&f.p);
                        r.p = f.p.clone();
                    }
                    if f.activo && !r.activo {
                        r.proc.limpiar();
                    }
                    r.activo = f.activo;
                    c.0.push(r);
                }
            }
            c
        };
        for (mut p, fx) in n.pistas {
            p.cadena = cadena(&fx);
            if let Some(v) = pistas_viejas.iter_mut().find(|v| v.id == p.id) {
                p.g = v.g;
                p.pico = v.pico;
                if let (Some(_), Some(mut viejo)) = (&p.instr, v.instr.take()) {
                    if p.ins != v.ins || p.muestra.is_some() {
                        viejo.ajustar(p.ins.as_ref().unwrap(), p.muestra.clone());
                    }
                    if p.eventos != v.eventos {
                        // Al editar notas sonando no debe quedar ninguna pegada.
                        viejo.callar();
                    }
                    p.instr = Some(viejo);
                }
            }
            self.pistas.push(p);
        }
        for p in &mut self.pistas {
            p.automatizar(self.pos as f64 / SR as f64);
        }
        self.maestro = cadena(&n.fx_maestro);
        self.meta_maestro = n.g_maestro;
        self.fin = n.fin;
        self.armada = n.armada;
        if self.latencias() != antes {
            self.ir(self.pos);
        }
        (viejos, pistas_viejas)
    }

    fn latencias(&self) -> usize {
        self.pistas.iter().map(|p| p.cadena.latencia()).sum::<usize>() * 1009 + self.maestro.latencia()
    }

    /// Salta a `pos`: limpia los efectos y precarga los que tienen retardo.
    pub fn ir(&mut self, pos: i64) {
        self.pos = pos;
        self.pos_bus = pos;
        self.g_maestro = self.meta_maestro;
        self.maestro.limpiar();
        let mut b = [0f32; 2 * BLOQUE];
        for p in &mut self.pistas {
            p.automatizar(pos as f64 / SR as f64);
            p.g = p.meta;
            p.cadena.limpiar();
            if let Some(i) = &mut p.instr {
                i.callar();
            }
            let (mut t, fin) = (pos, pos + p.cadena.latencia() as i64);
            while t < fin {
                let n = (fin - t).min(BLOQUE as i64) as usize;
                let b = &mut b[..2 * n];
                b.fill(0.0);
                p.fuente(t, b);
                p.cadena.procesar(b);
                t += n as i64;
            }
        }
        let mut falta = self.maestro.latencia();
        while falta > 0 {
            let n = falta.min(BLOQUE);
            let b = &mut b[..2 * n];
            self.mezclar(b, true);
            self.maestro.procesar(b);
            falta -= n;
        }
    }

    fn mezclar(&mut self, out: &mut [f32], precarga: bool) {
        out.fill(0.0);
        let n = out.len() / 2;
        self.tmp.resize(out.len(), 0.0);
        let t0 = self.pos_bus;
        for p in &mut self.pistas {
            if self.tocando {
                p.automatizar(t0 as f64 / SR as f64);
            }
            let b = &mut self.tmp[..];
            b.fill(0.0);
            let t = t0 + p.cadena.latencia() as i64;
            if self.tocando || precarga {
                p.fuente(t, b);
            } else if let Some(i) = &mut p.instr {
                i.sonar(b);
            }
            if let Some(v) = self.vivo.as_ref().filter(|_| !precarga && self.armada == Some(p.id)) {
                let mut q = v.lock().unwrap();
                for f in b.chunks_exact_mut(2) {
                    let Some(x) = q.pop_front() else { break };
                    f[0] += x[0];
                    f[1] += x[1];
                }
            }
            p.cadena.procesar(b);
            let paso = [(p.meta[0] - p.g[0]) / n as f32, (p.meta[1] - p.g[1]) / n as f32];
            for (f, o) in b.chunks_exact(2).zip(out.chunks_exact_mut(2)) {
                for k in 0..2 {
                    p.g[k] += paso[k];
                    let x = f[k] * p.g[k];
                    p.pico[k] = p.pico[k].max(x.abs());
                    o[k] += x;
                }
            }
            p.g = p.meta;
        }
        self.pos_bus += n as i64;
    }

    /// Toca una nota desde el piano roll.
    pub fn probar(&mut self, pista: u64, tecla: u8, vel: u8, on: bool) {
        if let Some(i) = self.pistas.iter_mut().find(|p| p.id == pista).and_then(|p| p.instr.as_mut()) {
            i.nota(tecla, vel, on);
            self.previa = 4 * SR as i64;
        }
    }

    fn clic(&self, out: &mut [f32]) {
        for (k, f) in out.chunks_exact_mut(2).enumerate() {
            let t = self.pos + k as i64;
            let en = t.rem_euclid(self.metronomo);
            if en < 1400 {
                let hz = if t.rem_euclid(4 * self.metronomo) < 1400 { 1760.0 } else { 1175.0 };
                let x = (std::f32::consts::TAU * hz * en as f32 / SR as f32).sin() * (-(en as f32) / 350.0).exp() * 0.35;
                f[0] += x;
                f[1] += x;
            }
        }
    }

    /// Produce el siguiente bloque (estéreo intercalado, a lo más `BLOQUE` cuadros). Detenido da silencio.
    pub fn bloque(&mut self, out: &mut [f32]) {
        self.bloque_mezcla(out);
        if let Some((a, pos)) = &mut self.origen {
            for f in out.chunks_exact_mut(2) {
                if let Some(x) = a.cuadros.get(*pos as usize) {
                    f[0] += x[0];
                    f[1] += x[1];
                }
                *pos += 1;
            }
        }
    }

    fn bloque_mezcla(&mut self, out: &mut [f32]) {
        if !self.tocando && self.vivo.is_none() && self.previa <= 0 {
            out.fill(0.0);
            return;
        }
        let n = out.len() / 2;
        self.mezclar(out, false);
        self.maestro.procesar(out);
        let paso = (self.meta_maestro - self.g_maestro) / n as f32;
        for f in out.chunks_exact_mut(2) {
            self.g_maestro += paso;
            for k in 0..2 {
                f[k] *= self.g_maestro;
                self.pico_maestro[k] = self.pico_maestro[k].max(f[k].abs());
            }
        }
        self.g_maestro = self.meta_maestro;
        if self.tocando && self.metronomo > 0 {
            self.clic(out);
        }
        if !self.tocando {
            // Solo se oye el micrófono o una nota de prueba: la canción no avanza.
            self.previa -= n as i64;
            self.pos_bus -= n as i64;
            return;
        }
        self.pos += n as i64;
        if self.pos >= self.fin && !self.grabando {
            self.tocando = false;
        }
    }

    /// Picos desde la última lectura: por pista y del maestro.
    pub fn picos(&mut self) -> (HashMap<u64, [f32; 2]>, [f32; 2]) {
        let m = self.pistas.iter_mut().map(|p| (p.id, std::mem::take(&mut p.pico))).collect();
        (m, std::mem::take(&mut self.pico_maestro))
    }
}

/// Mezcla completa fuera de tiempo real. `cada` recibe cada bloque; si devuelve false se cancela.
pub fn mezclar_todo(p: &Proyecto, almacen: &Almacen, total: i64, cada: impl FnMut(i64, &[f32]) -> bool) -> bool {
    mezclar_desde(p, almacen, 0, total, cada)
}

pub fn mezclar_desde(p: &Proyecto, almacen: &Almacen, ini: i64, total: i64, mut cada: impl FnMut(i64, &[f32]) -> bool) -> bool {
    let mut m = Motor::default();
    m.aplicar(preparar(p, almacen, &[]));
    m.fin = i64::MAX;
    m.ir(ini);
    m.tocando = true;
    let mut b = [0f32; 2 * BLOQUE];
    let mut t = 0;
    while t < total {
        let n = (total - t).min(BLOQUE as i64) as usize;
        m.bloque(&mut b[..2 * n]);
        t += n as i64;
        if !cada(t, &b[..2 * n]) {
            return false;
        }
    }
    true
}

/// Segundos extra al final para que no se corten las colas de eco y reverberación.
pub fn cola(p: &Proyecto) -> f64 {
    p.pistas
        .iter()
        .flat_map(|t| &t.fx)
        .chain(&p.fx_maestro)
        .filter(|f| f.activo)
        .map(|f| match f.tipo {
            TipoFx::Reverberacion => (f.p[1] as f64).min(8.0),
            TipoFx::Eco => 3.0,
            _ => 0.0,
        })
        .fold(0.0, f64::max)
}
