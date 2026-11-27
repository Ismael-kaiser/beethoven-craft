// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Un solo efecto o instrumento, suelto del programa: lo que hay dentro de cada plugin VST3 o CLAP de Beethoven. Usa el
//! mismo motor, así que suena igual que en el DAW.
use crate::biblioteca::Biblioteca;
use crate::lua::{Ficha, Tipo};
use crate::modelo::{EfectoPuesto, PistaMezcla, Proyecto};
use crate::motor::{Escena, Estado, Externo, Mando, Modo, Motor, Orden};
use crate::plugin::{Evento, Tiempo};
use crate::receta::{BLOQUE, Receta};
use serde::{Deserialize, Serialize};
use std::cell::UnsafeCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::*};
use std::sync::{Arc, Mutex};

/// Lo que se guarda con el proyecto del otro programa.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Guardado {
    #[serde(default)]
    pub controles: BTreeMap<String, f32>,
    #[serde(default)]
    pub muestra: Option<String>,
    #[serde(default)]
    pub pads: Vec<String>,
    #[serde(default)]
    pub datos: BTreeMap<String, String>,
}

/// Lo del hilo de la ventana.
struct Principal {
    bib: Biblioteca,
    mando: Mando,
    g: Guardado,
    /// Con qué valores se armó la última escena.
    armado: Vec<f32>,
}

struct Audio {
    motor: Motor,
    puestos: Vec<f32>,
    sale: [[f32; 2]; BLOQUE],
}

pub struct Instancia {
    pub clave: String,
    pub ficha: Ficha,
    pub receta: Arc<Receta>,
    pub sr: f32,
    pub estado: Arc<Estado>,
    pub latencia: u32,
    valores: Vec<AtomicU32>,
    activo: AtomicBool,
    /// Sube cuando la ventana mueve algo: el otro programa debe enterarse.
    pub movidos: AtomicU64,
    principal: Mutex<Principal>,
    audio: UnsafeCell<Audio>,
    ocupado: AtomicBool,
}

// `audio` solo se toca en `procesar`, que se cuida con `ocupado`; lo demás es atómico o va con candado.
unsafe impl Send for Instancia {}
unsafe impl Sync for Instancia {}

/// Un evento y en qué cuadro del bloque pasa.
pub type En = (usize, Evento);

fn carpeta() -> PathBuf {
    crate::ui::carpeta().join("Lua")
}

/// Los efectos e instrumentos que se ofrecen: clave y ficha.
pub fn disponibles() -> Vec<(String, Ficha)> {
    let b = Biblioteca::nueva(48000.0, Some(carpeta()));
    b.guiones
        .iter()
        .filter(|g| !g.ficha.oculto && matches!(g.ficha.tipo, Tipo::Efecto | Tipo::Instrumento))
        // Los que mueven perillas de otros no tienen a quién fuera del DAW.
        .filter(|g| !g.receta.as_ref().is_some_and(|r| r.nombres_medidas().any(|m| m == "control")))
        .map(|g| (g.clave.clone(), g.ficha.clone()))
        .collect()
}

fn proyecto(clave: &str, tipo: Tipo, g: &Guardado) -> Proyecto {
    let mut p = Proyecto { bpm: 120.0, ..Default::default() };
    for m in &mut p.mezcla {
        *m = PistaMezcla {
            efectos: vec![],
            volumen: 1.0,
            pan: 0.0,
            mudo: false,
            solo: false,
            envios: Default::default(),
            eq: [0.0; 3],
            ancho: 1.0,
            fase: false,
            nombre: m.nombre.clone(),
        };
    }
    // Lo que entra de lado solo se oye por dentro.
    p.mezcla[2].volumen = 0.0;
    if tipo == Tipo::Efecto {
        let mut e = EfectoPuesto::nuevo(clave);
        (e.controles, e.muestra, e.datos) = (g.controles.clone(), g.muestra.clone(), g.datos.clone());
        if e.controles.contains_key("pista") {
            e.controles.insert("pista".into(), 2.0);
        }
        p.mezcla[1].efectos.push(e);
    } else {
        p.agregar_canal(clave, clave);
        let c = &mut p.canales[0];
        (c.volumen, c.pan, c.pista) = (1.0, 0.0, 1);
        (c.controles, c.muestra, c.pads, c.datos) = (g.controles.clone(), g.muestra.clone(), g.pads.clone(), g.datos.clone());
    }
    p
}

impl Instancia {
    /// En el hilo de la ventana.
    pub fn nueva(clave: &str, sr: f32, g: Guardado) -> Option<Instancia> {
        let mut bib = Biblioteca::solo(sr, Some(carpeta()), clave)?;
        let x = bib.ver(clave)?;
        let (ficha, receta) = (x.ficha.clone(), x.receta.clone()?);
        let p = proyecto(clave, ficha.tipo, &g);
        let (mut motor, mando) = Motor::nuevo(Escena::nueva(&p, &mut bib, Modo::Cancion));
        motor.externo = Some(Box::new(Externo {
            tiempo: Tiempo { bpm: 120.0, pos: 0.0, tocando: false, sr: sr as f64 },
            entra: [[0.0; BLOQUE]; 2],
            lateral: [[0.0; BLOQUE]; 2],
        }));
        let lee = |c: &crate::receta::Control| g.controles.get(&c.clave).copied().unwrap_or(c.valor).clamp(c.min, c.max);
        let v: Vec<f32> = receta.controles.iter().map(lee).collect();
        let latencia = bib.vivos().map(|v| v.1.latencia).max().unwrap_or(0);
        let estado = motor.estado.clone();
        estado.escucha.store(1, Relaxed);
        Some(Instancia {
            clave: clave.into(),
            valores: v.iter().map(|x| AtomicU32::new(x.to_bits())).collect(),
            activo: AtomicBool::new(true),
            movidos: AtomicU64::new(0),
            principal: Mutex::new(Principal { bib, mando, g, armado: v.clone() }),
            audio: UnsafeCell::new(Audio { motor, puestos: v, sale: [[0.0; 2]; BLOQUE] }),
            ocupado: AtomicBool::new(false),
            ficha,
            receta,
            sr,
            estado,
            latencia,
        })
    }

    pub fn valor(&self, k: usize) -> f32 {
        f32::from_bits(self.valores[k].load(Relaxed))
    }

    pub fn poner(&self, k: usize, v: f32) {
        if let (Some(x), Some(c)) = (self.valores.get(k), self.receta.controles.get(k))
            && v.is_finite()
        {
            x.store(v.clamp(c.min, c.max).to_bits(), Relaxed);
        }
    }

    pub fn activo(&self) -> bool {
        self.activo.load(Relaxed)
    }

    pub fn prender(&self, si: bool) {
        self.activo.store(si, Relaxed);
    }

    pub fn guardado(&self) -> Guardado {
        let mut g = self.principal.lock().unwrap_or_else(|e| e.into_inner()).g.clone();
        g.controles = self.receta.controles.iter().enumerate().map(|(k, c)| (c.clave.clone(), self.valor(k))).collect();
        g
    }

    pub fn a_texto(&self) -> Vec<u8> {
        serde_json::to_vec(&self.guardado()).unwrap_or_default()
    }

    pub fn de_texto(&self, b: &[u8]) -> bool {
        let Ok(g) = serde_json::from_slice::<Guardado>(b) else { return false };
        for (k, c) in self.receta.controles.iter().enumerate() {
            self.poner(k, g.controles.get(&c.clave).copied().unwrap_or(c.valor));
        }
        self.cambiar(g);
        true
    }

    /// Otros audios, textos o pads: se vuelve a armar.
    pub fn cambiar(&self, g: Guardado) {
        let mut p = self.principal.lock().unwrap_or_else(|e| e.into_inner());
        p.g = g;
        p.armado.clear();
        drop(p);
        self.poner_al_dia();
    }

    /// En el hilo de la ventana: arma otra escena si cambió algo que no es solo mover un número (el lugar, los cortes…).
    pub fn poner_al_dia(&self) {
        let ahora: Vec<f32> = (0..self.valores.len()).map(|k| self.valor(k)).collect();
        let Ok(mut p) = self.principal.try_lock() else { return };
        if p.armado == ahora {
            return;
        }
        p.g.controles = self.receta.controles.iter().zip(&ahora).map(|(c, v)| (c.clave.clone(), *v)).collect();
        let q = proyecto(&self.clave, self.ficha.tipo, &p.g);
        let e = Box::new(Escena::nueva(&q, &mut p.bib, Modo::Cancion));
        if p.mando.mandar(Orden::Escena(e)) {
            p.armado = ahora;
        }
    }

    /// Lo que mide, la huella o la onda de su audio, y sus cortes: para la ventana.
    pub fn para_ver(&self) -> (Vec<(String, f32)>, Vec<f32>, Vec<f32>, Guardado, crate::ojo::Visto) {
        let g = self.guardado();
        let Ok(mut p) = self.principal.try_lock() else { return (vec![], vec![], vec![], g, Default::default()) };
        let lugar = if self.ficha.tipo == Tipo::Efecto { "efecto 1 0" } else { "canal 1" };
        let visto = p.bib.ojo(lugar).ver();
        let medidas =
            p.bib.medidas.get(&(1, 0)).map_or(vec![], |x| {
                self.receta.nombres_medidas().zip(x.iter()).map(|(n, v)| (n.to_string(), f32::from_bits(v.load(Relaxed)))).collect()
            });
        let mut huella = vec![];
        let mut cortes = vec![];
        if self.receta.con_huella() {
            let mut e = EfectoPuesto::nuevo(&self.clave);
            (e.controles, e.muestra) = (g.controles.clone(), g.muestra.clone());
            huella = p.bib.huella(&e).and_then(|m| m.impulso.as_ref().map(|h| h.picos.clone())).unwrap_or_default();
        } else if let Some(r) = &g.muestra {
            huella = p.bib.muestra(r, 0).map_or(vec![], |m| crate::ui::picos(&m));
            if self.ficha.muestras == crate::lua::Muestras::Cortes {
                cortes = p.bib.cortes(r, &g.controles).map_or(vec![], |m| m.cortes.clone());
            }
        }
        (medidas, huella, cortes, g, visto)
    }

    /// Los archivos de su carpeta y su guion, para armar la ventana.
    pub fn interfaz(&self) -> Option<crate::interfaz::Interfaz> {
        let p = self.principal.lock().unwrap_or_else(|e| e.into_inner());
        let g = p.bib.ver(&self.clave)?;
        crate::interfaz::cargar_con(&g.fuente, g.archivos.clone()).ok().flatten()
    }

    /// Nota desde la ventana.
    pub fn tocar(&self, tono: u8, si: bool) {
        if let Ok(p) = self.principal.try_lock() {
            let canal = 1;
            p.mando.mandar(if si { Orden::Nota { canal, tono, fuerza: 0.9 } } else { Orden::Soltar { canal, tono } });
        }
    }

    /// Hilo de audio. Los eventos van en orden. `entra` y `lateral` pueden venir vacíos.
    pub fn procesar(&self, ev: &[En], entra: [&[f32]; 2], lateral: [&[f32]; 2], sale: [&mut [f32]; 2], t: &Tiempo) {
        let [izq, der] = sale;
        let n = izq.len().min(der.len());
        if self.ocupado.swap(true, Acquire) {
            izq.fill(0.0);
            der.fill(0.0);
            return;
        }
        let a = unsafe { &mut *self.audio.get() };
        crate::receta::sin_denormales();
        for (k, (v, p)) in self.valores.iter().zip(a.puestos.iter_mut()).enumerate() {
            let v = f32::from_bits(v.load(Relaxed));
            if v != *p {
                *p = v;
                a.motor.control(k, v);
            }
        }
        let activo = self.activo() || self.ficha.tipo != Tipo::Efecto;
        let (mut i, mut e) = (0, 0);
        while i < n {
            while e < ev.len() && ev[e].0 <= i {
                match ev[e].1 {
                    Evento::Nota { tono, fuerza } => a.motor.hacer(Orden::Nota { canal: 1, tono, fuerza }),
                    Evento::Suelta { tono } => a.motor.hacer(Orden::Soltar { canal: 1, tono }),
                    x @ (Evento::Rueda(_) | Evento::Mando { .. }) => a.motor.hacer(Orden::Mando { canal: 1, evento: x }),
                    Evento::Control { .. } => {}
                }
                e += 1;
            }
            let fin = ev.get(e).map_or(n, |x| x.0.min(n)).min(i + BLOQUE);
            let m = fin - i;
            if let Some(x) = &mut a.motor.externo {
                x.tiempo = Tiempo { pos: t.pos + i as f64 * t.bpm / 60.0 / t.sr, ..*t };
                for l in 0..2 {
                    for (b, s) in [(&mut x.entra[l], entra[l]), (&mut x.lateral[l], lateral[l])] {
                        match s.get(i..fin) {
                            Some(s) => b[..m].copy_from_slice(s),
                            None => b.fill(0.0),
                        }
                    }
                }
            }
            a.motor.tocar(&mut a.sale[..m]);
            for j in 0..m {
                let s = if activo { a.sale[j] } else { [entra[0].get(i + j).copied().unwrap_or(0.0), entra[1].get(i + j).copied().unwrap_or(0.0)] };
                izq[i + j] = s[0];
                der[i + j] = s[1];
            }
            i = fin;
        }
        self.ocupado.store(false, Release);
    }
}
