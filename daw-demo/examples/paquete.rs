// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Abre Beethoven.vst3 y Beethoven.clap como lo haría otro programa y prueba cada plugin.
//! `cargo run --release --example paquete <carpeta>`
use beethoven_daw_demo::plugin::{Evento, Oferta, Plugin, Tiempo, ofertas};
use beethoven_daw_demo::receta::BLOQUE;

fn main() {
    let base = std::path::PathBuf::from(std::env::args().nth(1).expect("falta la carpeta"));
    let mut mal = 0;
    for ext in ["clap", "vst3"] {
        let of = match ofertas(&base.join(format!("Beethoven.{ext}"))) {
            Ok(o) => o,
            Err(e) => {
                println!("MAL {ext}: {e}");
                mal += 1;
                continue;
            }
        };
        let fallas: Vec<String> = of.iter().filter_map(|o| probar(o).err().map(|e| format!("{}: {e}", o.nombre))).collect();
        println!("{ext}: {} plugins, {} fallas", of.len(), fallas.len());
        fallas.iter().for_each(|f| println!("  MAL {f}"));
        mal += fallas.len() + (of.len() < 50) as usize;
    }
    std::process::exit((mal > 0) as i32);
}

fn probar(o: &Oferta) -> Result<(), String> {
    let p = Plugin::cargar(&o.ruta, &o.id, 48000.0, &[])?;
    let mut sale = [[0.0; BLOQUE]; 2];
    let t = |i: usize| Tiempo { bpm: 120.0, pos: i as f64 * 0.0026, tocando: true, sr: 48000.0 };
    for i in 0..600 {
        let mut x = [[0.0; BLOQUE]; 2];
        if !o.instrumento {
            for (k, v) in x.iter_mut().flat_map(|c| c.iter_mut().enumerate()) {
                *v = ((i * BLOQUE + k) as f32 * 0.06).sin() * 0.4;
            }
        }
        let ev: &[Evento] = match i {
            2 => &[Evento::Nota { tono: 48, fuerza: 0.9 }],
            300 => &[Evento::Suelta { tono: 48 }],
            _ => &[],
        };
        p.procesar(ev, &t(i), &x, &mut sale, if i % 3 == 0 { 17 } else { BLOQUE });
        p.atender();
        if sale.iter().flatten().any(|v| !v.is_finite()) {
            return Err("infinito".into());
        }
    }
    if let Some(c) = p.controles.iter().find(|c| !c.entero) {
        let nuevo = if (p.valor(c.id) - c.max).abs() > 1e-6 { c.max } else { c.min };
        p.mover(c.id, nuevo);
        p.procesar(&[], &t(0), &[[0.0; BLOQUE]; 2], &mut sale, BLOQUE);
        let q = Plugin::cargar(&o.ruta, &o.id, 48000.0, &p.estado())?;
        for v in [p.valor(c.id), q.valor(c.id)] {
            if (v - nuevo).abs() > 0.02 * (c.max - c.min) {
                return Err(format!("{}: {v} en vez de {nuevo}", c.nombre));
            }
        }
    }
    Ok(())
}
