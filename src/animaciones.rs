//! Animaciones listas para textos e imágenes: cuadros clave con su curva sobre el movimiento del clip (posición,
//! escala, giro, opacidad). Las de entrada van al principio del clip y las de salida al final; parten del valor que
//! el clip ya tiene, así que se pueden juntar (entrar desde la izquierda y aparecer, por ejemplo).
use crate::curvas::{Curva, Familia, Sentido};
use crate::modelo::{ClipVideo, Prop, Valor};

/// Propiedad del movimiento de un clip.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Propiedad {
    X,
    Y,
    Escala,
    Giro,
    Opacidad,
}

impl Propiedad {
    pub const TODAS: [Propiedad; 5] = [Propiedad::X, Propiedad::Y, Propiedad::Escala, Propiedad::Giro, Propiedad::Opacidad];
    pub fn nombre(self) -> &'static str {
        match self {
            Propiedad::X => "Posición X",
            Propiedad::Y => "Posición Y",
            Propiedad::Escala => "Escala",
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
            Propiedad::Giro => &mut m.giro,
            Propiedad::Opacidad => &mut m.opacidad,
        }
    }
}

/// Un cambio de una propiedad: desde qué valor (o hasta cuál, en las de salida) y con qué curva.
type Paso = (Propiedad, f64, Curva);

pub struct Animacion {
    pub nombre: &'static str,
    pub ayuda: &'static str,
    /// Al final del clip (sale) o al principio (entra).
    pub salida: bool,
    /// Segundos que dura.
    pub dur: f64,
    pub pasos: &'static [Paso],
}

const fn f(fam: Familia, s: Sentido) -> Curva {
    Curva::Familia(fam, s)
}
use Familia::*;
use Propiedad::*;
use Sentido::*;

pub const ANIMACIONES: &[Animacion] = &[
    Animacion { nombre: "Aparecer", ayuda: "Pasa de invisible a visible", salida: false, dur: 0.6, pasos: &[(Opacidad, 0.0, f(Sine, Ambos))] },
    Animacion { nombre: "Entrar desde la izquierda", ayuda: "Se desliza desde fuera del cuadro", salida: false, dur: 0.8, pasos: &[(X, -100.0, f(Cubic, Salir))] },
    Animacion { nombre: "Entrar desde la derecha", ayuda: "Se desliza desde fuera del cuadro", salida: false, dur: 0.8, pasos: &[(X, 100.0, f(Cubic, Salir))] },
    Animacion { nombre: "Subir desde abajo", ayuda: "Sube con un pequeño impulso", salida: false, dur: 0.8, pasos: &[(Y, 60.0, f(Back, Salir)), (Opacidad, 0.0, f(Sine, Salir))] },
    Animacion { nombre: "Caer y rebotar", ayuda: "Cae desde arriba y rebota al llegar", salida: false, dur: 1.2, pasos: &[(Y, -100.0, f(Bounce, Salir))] },
    Animacion { nombre: "Crecer", ayuda: "Crece desde un punto con impulso", salida: false, dur: 0.7, pasos: &[(Escala, 0.0, f(Back, Salir))] },
    Animacion { nombre: "Crecer elástico", ayuda: "Crece y tiembla como resorte", salida: false, dur: 1.2, pasos: &[(Escala, 0.0, f(Elastic, Salir))] },
    Animacion { nombre: "Girar al entrar", ayuda: "Gira mientras crece", salida: false, dur: 0.9, pasos: &[(Giro, -180.0, f(Cubic, Salir)), (Escala, 0.0, f(Cubic, Salir))] },
    Animacion { nombre: "Acercarse", ayuda: "Llega desde muy grande", salida: false, dur: 0.7, pasos: &[(Escala, 250.0, f(Quart, Salir)), (Opacidad, 0.0, f(Sine, Salir))] },
    Animacion { nombre: "Desvanecer al salir", ayuda: "Se vuelve invisible al final", salida: true, dur: 0.6, pasos: &[(Opacidad, 0.0, f(Sine, Ambos))] },
    Animacion { nombre: "Salir por la derecha", ayuda: "Se va deslizando al final", salida: true, dur: 0.8, pasos: &[(X, 100.0, f(Cubic, Entrar))] },
    Animacion { nombre: "Salir por la izquierda", ayuda: "Se va deslizando al final", salida: true, dur: 0.8, pasos: &[(X, -100.0, f(Cubic, Entrar))] },
    Animacion { nombre: "Encoger al salir", ayuda: "Se encoge con impulso al final", salida: true, dur: 0.6, pasos: &[(Escala, 0.0, f(Back, Entrar))] },
];

/// Pone la animación `a` en el clip: un cuadro clave en cada punta de su tramo, con la curva en el primero.
pub fn poner(c: &mut ClipVideo, a: &Animacion) {
    let dur = a.dur.min(c.dur);
    let (t0, t1) = if a.salida { ((c.dur - dur).max(0.0), c.dur) } else { (0.0, dur) };
    for &(prop, otro, curva) in a.pasos {
        let p = prop.de_mut(c);
        // El valor que tiene el clip en esa punta: a él llega (al entrar) o de él parte (al salir).
        let propio = p.en(if a.salida { t0 } else { t1 }).num();
        let (va, vb) = if a.salida { (propio, otro) } else { (otro, propio) };
        // Fuera lo que hubiera en ese tramo.
        p.claves.retain(|k| k.0 < t0 - 0.02 || k.0 > t1 + 0.02);
        p.curvas.retain(|k| k.0 < t0 - 0.02 || k.0 > t1 + 0.02);
        p.claves.push((t0, Valor::Num(va)));
        p.claves.push((t1, Valor::Num(vb)));
        p.claves.sort_by(|a, b| a.0.total_cmp(&b.0));
        p.poner_curva(t0, curva);
    }
}
