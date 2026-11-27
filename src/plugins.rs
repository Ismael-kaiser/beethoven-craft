// Beethoven Craft. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Los plugins de Beethoven DAW dentro de Movie: el mismo motor y la misma ventana que en el DAW, así que suenan igual.
//! Los de fábrica y los que instala la persona en el DAW («Tus plugins»).
use crate::efectos::{Procesador, TipoFx};
use crate::modelo::{Fx, FxPlugin, SR};
use beethoven_daw_demo::instancia::{self, Guardado, Instancia};
use beethoven_daw_demo::lua::Tipo;
use beethoven_daw_demo::plugin::Tiempo;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Clave, nombre y grupo de cada efecto que se puede poner. Se vuelve a leer cada poco, por si se instaló otro.
pub fn efectos() -> Vec<(String, String, String)> {
    static LISTA: Mutex<Option<(Instant, Vec<(String, String, String)>)>> = Mutex::new(None);
    let mut l = LISTA.lock().unwrap_or_else(|e| e.into_inner());
    if l.as_ref().is_none_or(|(t, _)| t.elapsed() > Duration::from_secs(10)) {
        let v = instancia::disponibles()
            .into_iter()
            .filter(|(_, f)| f.tipo == Tipo::Efecto)
            .map(|(k, f)| (k, f.nombre.ver().to_string(), f.grupo.ver().to_string()))
            .collect();
        *l = Some((Instant::now(), v));
    }
    l.as_ref().unwrap().1.clone()
}

/// Lo que se guarda en el proyecto, como lo pide el DAW.
fn guardado(f: &Fx, n: &Instancia) -> Guardado {
    let x = f.plugin.clone().unwrap_or_default();
    Guardado {
        controles: n.receta.controles.iter().zip(&f.p).map(|(c, v)| (c.clave.clone(), *v)).collect(),
        muestra: x.muestra,
        pads: vec![],
        datos: x.datos,
    }
}

/// El efecto nuevo, con sus controles como vienen. `None` si el plugin no abre.
pub fn nuevo(id: u64, clave: &str) -> Option<Fx> {
    let n = Instancia::nueva(clave, SR as f32, Guardado::default())?;
    let p = (0..n.receta.controles.len()).map(|k| n.valor(k)).collect();
    let nombre = n.ficha.nombre.ver().to_string();
    Some(Fx { id, tipo: TipoFx::Plugin, activo: true, p, plugin: Some(FxPlugin { clave: clave.into(), nombre, ..Default::default() }) })
}

/// Abre el plugin de `f` con lo que tiene guardado.
pub fn abrir(f: &Fx) -> Option<Arc<Instancia>> {
    let x = f.plugin.as_ref()?;
    let mut g = Guardado { muestra: x.muestra.clone(), datos: x.datos.clone(), ..Default::default() };
    let n = Instancia::nueva(&x.clave, SR as f32, g.clone())?;
    g = guardado(f, &n);
    for (k, v) in f.p.iter().enumerate() {
        n.poner(k, *v);
    }
    n.cambiar(g);
    Some(Arc::new(n))
}

/// Lo último que se igualó entre un efecto y su plugin.
#[derive(Default)]
pub struct Igual {
    movidos: u64,
    fx: Option<Fx>,
}

/// Pasa a `f` lo que se movió en la ventana; si en cambio cambió `f` (deshacer, otro panel), se lo pasa al plugin. Lo
/// que mueve la automatización al sonar no toca `f`, así que no se pelean. Devuelve si cambió `f`.
pub fn igualar(f: &mut Fx, n: &Instancia, i: &mut Igual) -> bool {
    let movidos = n.movidos.load(std::sync::atomic::Ordering::Relaxed);
    let mut cambio = false;
    if movidos != i.movidos {
        i.movidos = movidos;
        let p: Vec<f32> = (0..n.receta.controles.len()).map(|k| n.valor(k)).collect();
        let g = n.guardado();
        let activo = n.activo();
        let x = f.plugin.get_or_insert_with(Default::default);
        if f.p != p || x.muestra != g.muestra || x.datos != g.datos || f.activo != activo {
            (f.p, x.muestra, x.datos, f.activo) = (p, g.muestra, g.datos, activo);
            cambio = true;
        }
    } else if i.fx.as_ref() != Some(f) {
        let antes = i.fx.as_ref();
        if antes.is_some_and(|a| a.plugin != f.plugin) {
            n.cambiar(guardado(f, n));
        }
        for (k, v) in f.p.iter().enumerate() {
            if antes.is_none_or(|a| a.p.get(k) != Some(v)) {
                n.poner(k, *v);
            }
        }
        n.prender(f.activo);
    }
    i.fx = Some(f.clone());
    n.poner_al_dia();
    cambio
}

/// El plugin en la cadena de efectos del motor de Movie: de intercalado a dos canales y de vuelta, en tramos fijos para
/// no pedir memoria mientras suena.
pub struct Proc {
    pub n: Arc<Instancia>,
    entra: [Vec<f32>; 2],
    sale: [Vec<f32>; 2],
}

const TRAMO: usize = 1024;

impl Proc {
    pub fn nuevo(n: Arc<Instancia>) -> Proc {
        Proc { n, entra: [vec![0.0; TRAMO], vec![0.0; TRAMO]], sale: [vec![0.0; TRAMO], vec![0.0; TRAMO]] }
    }
}

impl Procesador for Proc {
    fn procesar(&mut self, buf: &mut [f32]) {
        let t = Tiempo { bpm: 120.0, pos: 0.0, tocando: true, sr: SR as f64 };
        for tramo in buf.chunks_mut(TRAMO * 2) {
            let m = tramo.len() / 2;
            for i in 0..m {
                self.entra[0][i] = tramo[i * 2];
                self.entra[1][i] = tramo[i * 2 + 1];
            }
            let [a, b] = &mut self.sale;
            self.n.procesar(&[], [&self.entra[0][..m], &self.entra[1][..m]], [&[], &[]], [&mut a[..m], &mut b[..m]], &t);
            for i in 0..m {
                tramo[i * 2] = a[i];
                tramo[i * 2 + 1] = b[i];
            }
        }
    }
    fn ajustar(&mut self, p: &[f32]) {
        for (k, v) in p.iter().enumerate() {
            self.n.poner(k, *v);
        }
    }
    fn limpiar(&mut self) {}
    fn latencia(&self) -> usize {
        self.n.latencia as usize
    }
}
