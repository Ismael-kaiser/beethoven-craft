//! Animaciones listas: cuadros clave con su curva sobre el movimiento del clip y, en algunas, sobre un efecto. Las de
//! entrada van al principio, las de salida al final y las de énfasis en el cursor o repetidas en todo el clip. Parten
//! del valor que el clip ya tiene, así que se pueden juntar.
use crate::curvas::{Curva, Familia, Sentido};
use crate::modelo::{ClipVideo, FxVideo, Prop, Valor};
#[cfg(test)]
use crate::modelo::{Contenido, Proyecto};

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

// ---- Llaves puestas a mano y trazos libres

/// Qué guarda una llave: un tipo de movimiento, o todo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Llave {
    Posicion,
    Escala,
    Estirado,
    Giro,
    Opacidad,
    Todo,
}

impl Llave {
    pub const TODAS: [(Llave, &'static str); 6] =
        [(Llave::Posicion, "Posición"), (Llave::Giro, "Giro"), (Llave::Escala, "Escala"), (Llave::Estirado, "Ancho y alto"), (Llave::Opacidad, "Opacidad"), (Llave::Todo, "Todo")];

    pub fn propiedades(self) -> &'static [Propiedad] {
        use Propiedad::*;
        match self {
            Llave::Posicion => &[X, Y],
            Llave::Escala => &[Escala],
            Llave::Estirado => &[Ancho, Alto],
            Llave::Giro => &[Giro],
            Llave::Opacidad => &[Opacidad],
            Llave::Todo => &[X, Y, Escala, Ancho, Alto, Giro, Opacidad],
        }
    }

    pub fn de(p: &Pista) -> Option<Llave> {
        Some(match p {
            Pista::Mov(Propiedad::X | Propiedad::Y) => Llave::Posicion,
            Pista::Mov(Propiedad::Escala) => Llave::Escala,
            Pista::Mov(Propiedad::Ancho | Propiedad::Alto) => Llave::Estirado,
            Pista::Mov(Propiedad::Giro) => Llave::Giro,
            Pista::Mov(Propiedad::Opacidad) => Llave::Opacidad,
            Pista::Fx(..) => return None,
        })
    }
}

/// Guarda el valor que cada propiedad tiene en `tl`. Las llaves que ya estaban se quedan igual.
pub fn poner_llave(c: &mut ClipVideo, l: Llave, tl: f64) {
    for p in l.propiedades() {
        let q = p.de_mut(c);
        if q.clave_en(tl).is_none() {
            q.alternar_clave(tl);
        }
    }
    c.animado = true;
}

/// Segundos con llaves y qué se guarda en cada uno.
pub fn momentos(c: &ClipVideo) -> Vec<(f64, Vec<Pista>)> {
    let mut v: Vec<(f64, Vec<Pista>)> = vec![];
    for p in Pista::todas(c) {
        for k in p.de(c).map_or(&[][..], |q| &q.claves) {
            match v.iter_mut().find(|m| (m.0 - k.0).abs() < 0.02) {
                Some(m) => m.1.push(p.clone()),
                None => v.push((k.0, vec![p.clone()])),
            }
        }
    }
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    v
}

/// Quita las llaves de `tl`: las de un tipo, o todas.
pub fn quitar_llaves(c: &mut ClipVideo, tl: f64, solo: Option<Llave>) {
    for p in Pista::todas(c) {
        if solo.is_some_and(|l| l != Llave::Todo && Llave::de(&p) != Some(l)) {
            continue;
        }
        if let Some(q) = p.de_mut(c).filter(|q| q.clave_en(tl).is_some()) {
            q.alternar_clave(tl);
        }
    }
}

/// Lleva todas las llaves de un momento a otro. No se encima en un momento que ya tenga llaves.
pub fn mover_momento(c: &mut ClipVideo, de: f64, a: f64) -> bool {
    let a = a.clamp(0.0, c.dur);
    if momentos(c).iter().any(|m| (m.0 - a).abs() < 0.03 && (m.0 - de).abs() >= 0.02) {
        return false;
    }
    for p in Pista::todas(c) {
        if let Some(q) = p.de_mut(c) {
            if let Some(i) = q.clave_en(de) {
                q.mover_clave(i, a);
            }
        }
    }
    true
}

/// La curva que sigue a las llaves de un momento.
pub fn curva_de_momento(c: &mut ClipVideo, tl: f64, cv: Curva) {
    for p in Pista::todas(c) {
        if let Some(q) = p.de_mut(c).filter(|q| q.clave_en(tl).is_some()) {
            q.poner_curva(tl, cv);
        }
    }
}

/// Pone la posición en `tl`, siempre con llave.
pub fn poner_posicion(c: &mut ClipVideo, tl: f64, p: [f64; 2]) {
    for (prop, v) in [(Propiedad::X, p[0]), (Propiedad::Y, p[1])] {
        let q = prop.de_mut(c);
        if q.clave_en(tl).is_none() {
            q.alternar_clave(tl);
        }
        q.poner(tl, Valor::Num(v));
    }
    c.animado = true;
}

/// Los segundos con llave de posición.
pub fn puntos(c: &ClipVideo) -> Vec<f64> {
    let mut v: Vec<f64> = c.mov.x.claves.iter().chain(&c.mov.y.claves).map(|k| k.0).collect();
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| (*a - *b).abs() < 0.02);
    v
}

/// Menos puntos para el mismo trazo: quita los que caen a menos de `eps` de donde estaría el movimiento sin ellos.
pub fn simplificar(v: &[(f64, [f64; 2])], eps: f64) -> Vec<(f64, [f64; 2])> {
    let Some((a, z)) = v.first().zip(v.last()) else { return vec![] };
    let mut peor = (0, eps);
    for (i, q) in v.iter().enumerate().skip(1).take(v.len().saturating_sub(2)) {
        let k = (q.0 - a.0) / (z.0 - a.0).max(1e-9);
        let d = (0..2).map(|e| (a.1[e] + (z.1[e] - a.1[e]) * k - q.1[e]).powi(2)).sum::<f64>().sqrt();
        if d > peor.1 {
            peor = (i, d);
        }
    }
    if v.len() < 2 || peor.0 == 0 {
        return if v.len() < 2 { v.to_vec() } else { vec![*a, *z] };
    }
    let mut o = simplificar(&v[..=peor.0], eps);
    o.pop();
    o.extend(simplificar(&v[peor.0..], eps));
    o
}

/// Cambia la posición de ese tramo del clip por un trazo: (segundo del clip, posición en por ciento).
pub fn trazar(c: &mut ClipVideo, v: &[(f64, [f64; 2])]) {
    let Some((a, z)) = v.first().zip(v.last()) else { return };
    let (a, z) = (a.0 - 0.02, z.0 + 0.02);
    for p in [Propiedad::X, Propiedad::Y] {
        let q = p.de_mut(c);
        q.claves.retain(|k| k.0 < a || k.0 > z);
        q.curvas.retain(|k| k.0 < a || k.0 > z);
        q.libre = true;
    }
    for (t, p) in v {
        poner_posicion(c, (*t).clamp(0.0, c.dur), *p);
    }
}

/// Reparte los puntos del camino en el tiempo según lo que se recorre entre uno y otro.
pub fn emparejar(c: &mut ClipVideo, razon: f64) {
    let v = puntos(c);
    let (Some(a), Some(z)) = (v.first().copied(), v.last().copied()) else { return };
    let en = |t: f64| [c.mov.x.en(t).num() * razon, c.mov.y.en(t).num()];
    // Largo de cada tramo, medido sobre la curva.
    let largo: Vec<f64> = v
        .windows(2)
        .map(|w| {
            (0..24)
                .map(|i| {
                    let (p, q) = (en(w[0] + (w[1] - w[0]) * i as f64 / 24.0), en(w[0] + (w[1] - w[0]) * (i + 1) as f64 / 24.0));
                    (p[0] - q[0]).hypot(p[1] - q[1])
                })
                .sum()
        })
        .collect();
    let total: f64 = largo.iter().sum();
    if total < 1e-6 {
        return;
    }
    let sitios: Vec<[f64; 2]> = v.iter().map(|t| [c.mov.x.en(*t).num(), c.mov.y.en(*t).num()]).collect();
    let mut va = 0.0;
    let mut nuevos = vec![(a, sitios[0])];
    for (l, p) in largo.iter().zip(&sitios[1..]) {
        va += l;
        nuevos.push((a + (z - a) * va / total, *p));
    }
    // Los que quedan casi juntos no dejan de ser puntos aparte.
    for i in 1..nuevos.len() {
        nuevos[i].0 = nuevos[i].0.max(nuevos[i - 1].0 + 0.03);
    }
    rehacer(c, &nuevos);
}

/// Recorre el camino del final al principio.
pub fn invertir(c: &mut ClipVideo) {
    let v = puntos(c);
    let (Some(a), Some(z)) = (v.first().copied(), v.last().copied()) else { return };
    let mut n: Vec<(f64, [f64; 2])> = v.iter().map(|t| (a + z - t, [c.mov.x.en(*t).num(), c.mov.y.en(*t).num()])).collect();
    n.reverse();
    rehacer(c, &n);
}

/// Cambia todos los puntos del camino.
fn rehacer(c: &mut ClipVideo, v: &[(f64, [f64; 2])]) {
    for p in [Propiedad::X, Propiedad::Y] {
        let q = p.de_mut(c);
        (q.claves, q.curvas) = (vec![], vec![]);
        if !q.libre {
            // Con frenado en cada punto no habría velocidad pareja.
            q.curvas = v.iter().map(|k| (k.0, Curva::Lineal)).collect();
        }
    }
    for (t, p) in v {
        poner_posicion(c, *t, *p);
    }
}

/// Gira el clip hacia donde va su camino. `razon`: ancho entre alto del cuadro.
pub fn orientar(c: &mut ClipVideo, razon: f64) {
    let v = puntos(c);
    let (Some(a), Some(z)) = (v.first().copied(), v.last().copied()) else { return };
    if z - a < 0.05 {
        return;
    }
    let en = |t: f64| [c.mov.x.en(t).num() * razon, c.mov.y.en(t).num()];
    let n = (((z - a) / 0.12).ceil() as usize).clamp(v.len(), 400);
    let mut claves: Vec<(f64, f64)> = vec![];
    for i in 0..=n {
        let t = a + (z - a) * i as f64 / n as f64;
        let (p, q) = (en((t - 0.03).max(a)), en((t + 0.03).min(z)));
        let d = [q[0] - p[0], q[1] - p[1]];
        if d[0].hypot(d[1]) < 1e-4 {
            continue;
        }
        let mut g = d[1].atan2(d[0]).to_degrees();
        // La vuelta más cercana a la anterior, para que no gire de más.
        if let Some(u) = claves.last() {
            g += ((u.1 - g) / 360.0).round() * 360.0;
        }
        claves.push((t, g));
    }
    let v: Vec<(f64, [f64; 2])> = claves.into_iter().map(|(t, g)| (t, [g, 0.0])).collect();
    let q = &mut c.mov.giro;
    q.claves = simplificar(&v, 2.0).into_iter().map(|(t, g)| (t, Valor::Num(g[0]))).collect();
    // Recta entre giros: una curva se pasaría de largo en las esquinas del camino.
    (q.curvas, q.libre) = (q.claves.iter().map(|k| (k.0, Curva::Lineal)).collect(), false);
    c.animado = true;
}

#[cfg(test)]
mod pruebas_libres {
    use super::*;

    fn clip() -> ClipVideo {
        Proyecto::nuevo().nuevo_video("x".into(), Contenido::Color([1.0; 4]), 0.0, 4.0)
    }

    #[test]
    fn llaves_por_tipo() {
        let mut c = clip();
        c.mov.giro.v = Valor::Num(30.0);
        poner_llave(&mut c, Llave::Giro, 1.0);
        assert_eq!((c.mov.giro.claves.len(), c.mov.x.claves.len()), (1, 0), "solo lo pedido");
        assert_eq!(c.mov.giro.claves[0], (1.0, Valor::Num(30.0)));
        // Otra vez en el mismo lugar no la quita.
        poner_llave(&mut c, Llave::Giro, 1.0);
        assert_eq!(c.mov.giro.claves.len(), 1);
        poner_llave(&mut c, Llave::Posicion, 1.0);
        poner_llave(&mut c, Llave::Todo, 3.0);
        let m = momentos(&c);
        assert_eq!(m.iter().map(|x| (x.0, x.1.len())).collect::<Vec<_>>(), [(1.0, 3), (3.0, 7)]);
        assert!(mover_momento(&mut c, 1.0, 2.0));
        assert_eq!(c.mov.giro.claves[0].0, 2.0);
        assert!(!mover_momento(&mut c, 2.0, 3.0), "no se encima");
        assert_eq!(momentos(&c).len(), 2);
        quitar_llaves(&mut c, 2.0, Some(Llave::Giro));
        assert_eq!(momentos(&c)[0].1.len(), 2);
        quitar_llaves(&mut c, 2.0, None);
        assert_eq!(momentos(&c).len(), 1);
    }

    #[test]
    fn el_trazo_pasa_por_sus_puntos() {
        let mut c = clip();
        // Un círculo dibujado a mano, con muchos puntos.
        let v: Vec<(f64, [f64; 2])> = (0..=240).map(|i| i as f64 / 240.0).map(|k| (0.5 + 3.0 * k, [30.0 * (k * 6.2832).cos(), 30.0 * (k * 6.2832).sin()])).collect();
        let s = simplificar(&v, 0.6);
        assert!(s.len() > 8 && s.len() < 60, "{}", s.len());
        assert_eq!((s[0], *s.last().unwrap()), (v[0], v[240]));
        trazar(&mut c, &s);
        for (t, p) in &v {
            let q = [c.mov.x.en(*t).num(), c.mov.y.en(*t).num()];
            assert!((q[0] - p[0]).hypot(q[1] - p[1]) < 1.2, "{t}: {q:?} {p:?}");
        }
        // Sin detenerse en cada punto: la velocidad es pareja.
        let vel = |t: f64| (c.mov.x.en(t + 0.01).num() - c.mov.x.en(t - 0.01).num()).hypot(c.mov.y.en(t + 0.01).num() - c.mov.y.en(t - 0.01).num());
        let vs: Vec<f64> = (0..50).map(|i| vel(0.8 + i as f64 * 0.05)).collect();
        let (lo, hi) = vs.iter().fold((f64::MAX, 0f64), |a, v| (a.0.min(*v), a.1.max(*v)));
        assert!(hi / lo < 2.0, "{lo} {hi}");
        // Un trazo nuevo cambia solo su tramo.
        trazar(&mut c, &[(1.0, [0.0, 0.0]), (2.0, [10.0, 5.0])]);
        assert_eq!(puntos(&c).iter().filter(|t| (0.99..=2.01).contains(*t)).count(), 2);
        assert!(puntos(&c).len() > 4);
        // Un tramo con curva propia la respeta.
        c.mov.x.poner_curva(1.0, Curva::Salto);
        assert_eq!(c.mov.x.en(1.5).num(), 0.0);
    }

    #[test]
    fn gira_con_el_camino() {
        let mut c = clip();
        for (t, p) in [(0.0, [0.0, 0.0]), (1.0, [20.0, 0.0]), (2.0, [20.0, 20.0])] {
            poner_posicion(&mut c, t, p);
        }
        for q in [&mut c.mov.x, &mut c.mov.y] {
            for t in [0.0, 1.0] {
                q.poner_curva(t, Curva::Lineal);
            }
        }
        orientar(&mut c, 1.0);
        assert!(c.mov.giro.en(0.5).num().abs() < 4.0, "a la derecha");
        assert!((c.mov.giro.en(1.6).num() - 90.0).abs() < 4.0, "hacia abajo");
        // Con libre guardado y vuelto a leer.
        let s = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<ClipVideo>(&s).unwrap(), c);
    }
}
