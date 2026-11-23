//! Animaciones listas: cuadros clave con su curva sobre el movimiento del clip y, en algunas, sobre un efecto. Las de
//! entrada van al principio, las de salida al final y las de énfasis en el cursor o repetidas en todo el clip. Parten
//! del valor que el clip ya tiene, así que se pueden juntar.
use crate::curvas::{Curva, Familia, Sentido};
use crate::modelo::{ClipVideo, FxVideo, Prop, Valor};

/// Propiedad del movimiento de un clip.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Propiedad {
    X,
    Y,
    Escala,
    Ancho,
    Alto,
    Giro,
    Opacidad,
}

impl Propiedad {
    pub const TODAS: [Propiedad; 7] = [Propiedad::X, Propiedad::Y, Propiedad::Escala, Propiedad::Ancho, Propiedad::Alto, Propiedad::Giro, Propiedad::Opacidad];
    pub fn nombre(self) -> &'static str {
        match self {
            Propiedad::X => "Posición X",
            Propiedad::Y => "Posición Y",
            Propiedad::Escala => "Escala",
            Propiedad::Ancho => "Ancho",
            Propiedad::Alto => "Alto",
            Propiedad::Giro => "Giro",
            Propiedad::Opacidad => "Opacidad",
        }
    }
    pub fn de(self, c: &ClipVideo) -> &Prop {
        let m = &c.mov;
        match self {
            Propiedad::X => &m.x,
            Propiedad::Y => &m.y,
            Propiedad::Escala => &m.escala,
            Propiedad::Ancho => &m.ancho,
            Propiedad::Alto => &m.alto,
            Propiedad::Giro => &m.giro,
            Propiedad::Opacidad => &m.opacidad,
        }
    }
    pub fn de_mut(self, c: &mut ClipVideo) -> &mut Prop {
        let m = &mut c.mov;
        match self {
            Propiedad::X => &mut m.x,
            Propiedad::Y => &mut m.y,
            Propiedad::Escala => &mut m.escala,
            Propiedad::Ancho => &mut m.ancho,
            Propiedad::Alto => &mut m.alto,
            Propiedad::Giro => &mut m.giro,
            Propiedad::Opacidad => &mut m.opacidad,
        }
    }
}

/// Lo que se puede animar de un clip: su movimiento o el control de uno de sus efectos.
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum Pista {
    Mov(Propiedad),
    /// Efecto (por su id) y control.
    Fx(u64, String),
}

impl Pista {
    pub fn de<'a>(&self, c: &'a ClipVideo) -> Option<&'a Prop> {
        match self {
            Pista::Mov(p) => Some(p.de(c)),
            Pista::Fx(id, k) => c.fx.iter().find(|f| f.id == *id)?.p.get(k),
        }
    }
    pub fn de_mut<'a>(&self, c: &'a mut ClipVideo) -> Option<&'a mut Prop> {
        match self {
            Pista::Mov(p) => Some(p.de_mut(c)),
            Pista::Fx(id, k) => c.fx.iter_mut().find(|f| f.id == *id)?.p.get_mut(k),
        }
    }
    /// Las de un clip: todo el movimiento y los controles ya animados de sus efectos.
    pub fn todas(c: &ClipVideo) -> Vec<Pista> {
        let mut v: Vec<Pista> = Propiedad::TODAS.into_iter().map(Pista::Mov).collect();
        for f in &c.fx {
            v.extend(f.p.iter().filter(|p| !p.1.claves.is_empty()).map(|p| Pista::Fx(f.id, p.0.clone())));
        }
        v
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Momento {
    Entrada,
    Enfasis,
    /// Énfasis repetido en todo el clip.
    Bucle,
    Salida,
}

/// Un cambio de una propiedad: desde qué valor (o hasta cuál, en las de salida) y con qué curva.
type Paso = (Propiedad, f64, Curva);
/// Énfasis: parte del ciclo (0 a 1), cuánto se suma al valor propio (en giro y posición) o por cuánto se multiplica en
/// por ciento (en escala, ancho, alto y opacidad).
type Vaiven = (Propiedad, &'static [(f64, f64)], Curva);

pub struct Animacion {
    pub nombre: &'static str,
    pub ayuda: &'static str,
    pub momento: Momento,
    /// Segundos que dura (o cada ciclo).
    pub dur: f64,
    pub pasos: &'static [Paso],
    pub vaiven: &'static [Vaiven],
    /// Efecto que se anima: tipo, control, valor fuera y valor en su lugar.
    pub fx: &'static [(&'static str, &'static str, f64, f64)],
}

const fn f(fam: Familia, s: Sentido) -> Curva {
    Curva::Familia(fam, s)
}
use Familia::*;
use Propiedad::*;
use Sentido::*;

const BASE: Animacion = Animacion { nombre: "", ayuda: "", momento: Momento::Entrada, dur: 0.8, pasos: &[], vaiven: &[], fx: &[] };
const fn entra(nombre: &'static str, ayuda: &'static str, dur: f64, pasos: &'static [Paso]) -> Animacion {
    Animacion { nombre, ayuda, dur, pasos, ..BASE }
}
const fn sale(nombre: &'static str, ayuda: &'static str, dur: f64, pasos: &'static [Paso]) -> Animacion {
    Animacion { nombre, ayuda, dur, pasos, momento: Momento::Salida, ..BASE }
}
const fn enf(nombre: &'static str, ayuda: &'static str, dur: f64, vaiven: &'static [Vaiven]) -> Animacion {
    Animacion { nombre, ayuda, dur, vaiven, momento: Momento::Enfasis, ..BASE }
}
const fn bucle(nombre: &'static str, ayuda: &'static str, dur: f64, vaiven: &'static [Vaiven]) -> Animacion {
    Animacion { nombre, ayuda, dur, vaiven, momento: Momento::Bucle, ..BASE }
}
const fn con_fx(nombre: &'static str, ayuda: &'static str, momento: Momento, dur: f64, pasos: &'static [Paso], fx: &'static [(&'static str, &'static str, f64, f64)]) -> Animacion {
    Animacion { nombre, ayuda, momento, dur, pasos, fx, ..BASE }
}
const SV: Curva = Curva::Suave;

pub const ANIMACIONES: &[Animacion] = &[
    entra("Aparecer", "Pasa de invisible a visible", 0.6, &[(Opacidad, 0.0, f(Sine, Ambos))]),
    entra("Entrar desde la izquierda", "Se desliza desde fuera del cuadro", 0.8, &[(X, -100.0, f(Cubic, Salir))]),
    entra("Entrar desde la derecha", "Se desliza desde fuera del cuadro", 0.8, &[(X, 100.0, f(Cubic, Salir))]),
    entra("Entrar desde arriba", "Baja desde fuera del cuadro", 0.8, &[(Y, -100.0, f(Cubic, Salir))]),
    entra("Subir desde abajo", "Sube con un pequeño impulso", 0.8, &[(Y, 60.0, f(Back, Salir)), (Opacidad, 0.0, f(Sine, Salir))]),
    entra("Caer y rebotar", "Cae desde arriba y rebota al llegar", 1.2, &[(Y, -100.0, f(Bounce, Salir))]),
    entra("Crecer", "Crece desde un punto con impulso", 0.7, &[(Escala, 0.0, f(Back, Salir))]),
    entra("Crecer elástico", "Crece y tiembla como resorte", 1.2, &[(Escala, 0.0, f(Elastic, Salir))]),
    entra("Girar al entrar", "Gira mientras crece", 0.9, &[(Giro, -180.0, f(Cubic, Salir)), (Escala, 0.0, f(Cubic, Salir))]),
    entra("Acercarse", "Llega desde muy grande", 0.7, &[(Escala, 250.0, f(Quart, Salir)), (Opacidad, 0.0, f(Sine, Salir))]),
    entra("Abrir a lo ancho", "Se abre como puerta", 0.6, &[(Ancho, 0.0, f(Back, Salir))]),
    entra("Abrir a lo alto", "Se abre como persiana", 0.6, &[(Alto, 0.0, f(Back, Salir))]),
    entra("Voltear tarjeta", "Da media vuelta sobre sí misma", 0.8, &[(Ancho, -100.0, f(Cubic, Ambos))]),
    entra("Espiral", "Entra dando vueltas desde un lado", 1.1, &[(X, -70.0, f(Cubic, Salir)), (Giro, -540.0, f(Cubic, Salir)), (Escala, 20.0, f(Cubic, Salir))]),
    con_fx("Enfocarse", "Llega desenfocado y se aclara", Momento::Entrada, 0.9, &[(Opacidad, 0.0, f(Sine, Salir))], &[("ec.blur.gaussian", "blurriness", 90.0, 0.0)]),
    con_fx("Entrar con falla", "Colores separados que se juntan", Momento::Entrada, 0.7, &[(Opacidad, 0.0, Curva::Salto)], &[("fc:rgb_split", "amount", 90.0, 0.0)]),
    con_fx("Girar en 3D", "Da una vuelta en el espacio", Momento::Entrada, 1.0, &[(Opacidad, 0.0, f(Sine, Salir))], &[("fc:rotate_3d", "rot_y", 180.0, 0.0)]),
    con_fx("Entrar con destello", "Un brillo blanco que se apaga", Momento::Entrada, 0.6, &[(Opacidad, 0.0, f(Sine, Salir))], &[("ec.color.exposure", "master/exposure", 4.0, 0.0)]),
    enf("Salto", "Brinca una vez", 0.6, &[(Y, &[(0.0, 0.0), (0.45, -12.0), (1.0, 0.0)], f(Quad, Salir))]),
    enf("Golpe de zoom", "Se acerca de golpe y regresa", 0.5, &[(Escala, &[(0.0, 100.0), (0.25, 118.0), (1.0, 100.0)], f(Cubic, Salir))]),
    enf("Sacudida", "Tiembla de lado a lado", 0.6, &[(X, &[(0.0, 0.0), (0.15, -3.0), (0.3, 3.0), (0.45, -2.4), (0.6, 2.4), (0.8, -1.2), (1.0, 0.0)], Curva::Lineal)]),
    enf("Gelatina", "Se aplasta y se estira", 0.9, &[(Ancho, &[(0.0, 100.0), (0.2, 125.0), (0.4, 85.0), (0.6, 110.0), (0.8, 96.0), (1.0, 100.0)], SV), (Alto, &[(0.0, 100.0), (0.2, 75.0), (0.4, 115.0), (0.6, 92.0), (0.8, 104.0), (1.0, 100.0)], SV)]),
    enf("Vuelta completa", "Gira 360 grados", 0.9, &[(Giro, &[(0.0, 0.0), (1.0, 360.0)], f(Back, Ambos))]),
    enf("Parpadeo", "Se apaga y se prende", 0.6, &[(Opacidad, &[(0.0, 100.0), (0.25, 0.0), (0.5, 100.0), (0.75, 0.0), (1.0, 100.0)], Curva::Salto)]),
    enf("Negar", "Mueve la cabeza", 0.7, &[(Giro, &[(0.0, 0.0), (0.2, -9.0), (0.4, 9.0), (0.6, -6.0), (0.8, 6.0), (1.0, 0.0)], SV)]),
    bucle("Flotar", "Sube y baja todo el tiempo", 3.0, &[(Y, &[(0.0, 0.0), (0.5, -3.5), (1.0, 0.0)], f(Sine, Ambos))]),
    bucle("Latir", "Crece y se encoge todo el tiempo", 0.8, &[(Escala, &[(0.0, 100.0), (0.2, 108.0), (1.0, 100.0)], f(Sine, Ambos))]),
    bucle("Péndulo", "Se mece de lado a lado", 2.0, &[(Giro, &[(0.0, 0.0), (0.25, 8.0), (0.75, -8.0), (1.0, 0.0)], f(Sine, Ambos))]),
    bucle("Girar sin parar", "Da vueltas todo el tiempo", 4.0, &[(Giro, &[(0.0, 0.0), (1.0, 360.0)], Curva::Lineal)]),
    bucle("Respirar", "La transparencia va y viene", 2.4, &[(Opacidad, &[(0.0, 100.0), (0.5, 45.0), (1.0, 100.0)], f(Sine, Ambos))]),
    bucle("Vaivén", "Va y viene de lado", 3.0, &[(X, &[(0.0, 0.0), (0.25, 4.0), (0.75, -4.0), (1.0, 0.0)], f(Sine, Ambos))]),
    bucle("Nervioso", "Tiembla sin parar", 0.24, &[(X, &[(0.0, 0.0), (0.25, 0.7), (0.5, -0.6), (0.75, 0.4), (1.0, 0.0)], Curva::Lineal), (Y, &[(0.0, 0.0), (0.25, -0.6), (0.5, 0.5), (0.75, 0.6), (1.0, 0.0)], Curva::Lineal)]),
    sale("Desvanecer al salir", "Se vuelve invisible al final", 0.6, &[(Opacidad, 0.0, f(Sine, Ambos))]),
    sale("Salir por la derecha", "Se va deslizando al final", 0.8, &[(X, 100.0, f(Cubic, Entrar))]),
    sale("Salir por la izquierda", "Se va deslizando al final", 0.8, &[(X, -100.0, f(Cubic, Entrar))]),
    sale("Salir por arriba", "Se va hacia arriba al final", 0.8, &[(Y, -100.0, f(Cubic, Entrar))]),
    sale("Caerse", "Se cae girando al final", 1.0, &[(Y, 110.0, f(Quad, Entrar)), (Giro, 40.0, f(Quad, Entrar))]),
    sale("Encoger al salir", "Se encoge con impulso al final", 0.6, &[(Escala, 0.0, f(Back, Entrar))]),
    sale("Cerrar a lo ancho", "Se cierra como puerta", 0.5, &[(Ancho, 0.0, f(Cubic, Entrar))]),
    sale("Explotar", "Crece y desaparece", 0.6, &[(Escala, 300.0, f(Cubic, Entrar)), (Opacidad, 0.0, f(Sine, Entrar))]),
    con_fx("Desenfocarse", "Se vuelve borroso y se va", Momento::Salida, 0.9, &[(Opacidad, 0.0, f(Sine, Entrar))], &[("ec.blur.gaussian", "blurriness", 90.0, 0.0)]),
    con_fx("Salir con falla", "Los colores se separan y se va", Momento::Salida, 0.7, &[(Opacidad, 0.0, f(Quart, Entrar))], &[("fc:rgb_split", "amount", 90.0, 0.0)]),
];

fn limpiar(p: &mut Prop, t0: f64, t1: f64) {
    p.claves.retain(|k| k.0 < t0 - 0.02 || k.0 > t1 + 0.02);
    p.curvas.retain(|k| k.0 < t0 - 0.02 || k.0 > t1 + 0.02);
}

fn tramo(p: &mut Prop, t0: f64, t1: f64, a: f64, b: f64, cv: Curva) {
    limpiar(p, t0, t1);
    p.claves.push((t0, Valor::Num(a)));
    p.claves.push((t1, Valor::Num(b)));
    p.claves.sort_by(|a, b| a.0.total_cmp(&b.0));
    p.poner_curva(t0, cv);
}

/// Pone la animación en el clip. `t` es el segundo del clip donde va un énfasis; `nuevo_id` numera los efectos.
pub fn poner_en(c: &mut ClipVideo, a: &Animacion, t: f64, mut nuevo_id: impl FnMut() -> u64) {
    let dur = a.dur.min(c.dur);
    let (t0, t1) = match a.momento {
        Momento::Salida => ((c.dur - dur).max(0.0), c.dur),
        Momento::Enfasis => {
            let t0 = t.clamp(0.0, (c.dur - dur).max(0.0));
            (t0, t0 + dur)
        }
        _ => (0.0, dur),
    };
    let salida = a.momento == Momento::Salida;
    for &(prop, otro, curva) in a.pasos {
        let p = prop.de_mut(c);
        // El valor que tiene el clip en esa punta: a él llega (al entrar) o de él parte (al salir).
        let propio = p.en(if salida { t0 } else { t1 }).num();
        let (va, vb) = if salida { (propio, otro) } else { (otro, propio) };
        tramo(p, t0, t1, va, vb, curva);
    }
    for &(prop, puntos, curva) in a.vaiven {
        let largo = c.dur;
        let p = prop.de_mut(c);
        let por = !matches!(prop, Propiedad::X | Propiedad::Y | Propiedad::Giro);
        let (ini, fin) = if a.momento == Momento::Bucle { (0.0, largo) } else { (t0, t1) };
        let base = p.en(ini).num();
        let valor = |d: f64, vuelta: f64| if por { base * d / 100.0 } else { base + d + vuelta };
        // Lo que no regresa a cero (dar vueltas) se va sumando.
        let avance = if por { 0.0 } else { puntos.last().map_or(0.0, |k| k.1) };
        limpiar(p, ini, fin);
        let ciclos = if a.momento == Momento::Bucle { ((largo / a.dur).ceil() as usize).clamp(1, 400) } else { 1 };
        for n in 0..ciclos {
            for (i, (k, d)) in puntos.iter().enumerate() {
                // El primer punto de cada vuelta es el último de la anterior.
                if n > 0 && i == 0 {
                    continue;
                }
                let tk = ini + (n as f64 + k) * if a.momento == Momento::Bucle { a.dur } else { dur };
                p.claves.push((tk, Valor::Num(valor(*d, avance * n as f64))));
                p.curvas.retain(|x| (x.0 - tk).abs() >= 0.02);
                if curva != Curva::Suave {
                    p.curvas.push((tk, curva));
                }
            }
        }
        p.claves.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    for &(tipo, control, fuera, dentro) in a.fx {
        // Si ya tiene ese efecto, se anima el que tiene.
        if !c.fx.iter().any(|x| x.tipo == tipo) {
            if let Some(mut x) = crate::composicion::fx_nuevo(nuevo_id(), tipo) {
                neutro(&mut x, control, dentro);
                c.fx.push(x);
            }
        }
        if let Some(p) = c.fx.iter_mut().find(|x| x.tipo == tipo).and_then(|x| x.p.get_mut(control)) {
            let (va, vb) = if salida { (dentro, fuera) } else { (fuera, dentro) };
            tramo(p, t0, t1, va, vb, f(Cubic, if salida { Entrar } else { Salir }));
        }
    }
}

/// El efecto recién puesto no debe notarse fuera de su animación.
fn neutro(x: &mut FxVideo, control: &str, v: f64) {
    if let Some(p) = x.p.get_mut(control) {
        p.v = Valor::Num(v);
    }
}

/// Quita todos los cuadros clave del clip; se queda con los valores del segundo `t`.
pub fn quitar_todo(c: &mut ClipVideo, t: f64) {
    for p in Pista::todas(c) {
        if let Some(p) = p.de_mut(c) {
            p.v = p.en(t);
            p.claves.clear();
            p.curvas.clear();
        }
    }
}
