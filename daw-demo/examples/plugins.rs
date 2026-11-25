// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Prueba los plugins instalados: carga, suena, guarda su estado y lo recupera. `cargo run --example plugins [filtro]`
use beethoven_daw_demo::plugin::{Evento, Plugin, Tiempo, buscar, ofertas};
use beethoven_daw_demo::receta::BLOQUE;

fn main() {
    let filtro = std::env::args().nth(1).unwrap_or_default().to_lowercase();
    let (mut bien, mut mal) = (0, 0);
    for ruta in buscar().iter().filter(|r| r.to_string_lossy().to_lowercase().contains(&filtro)) {
        let of = match ofertas(ruta) {
            Ok(o) => o,
            Err(e) => {
                println!("MAL  {}: {e}", ruta.display());
                mal += 1;
                continue;
            }
        };
        for o in of {
            match probar(&o) {
                Ok(t) => {
                    bien += 1;
                    println!("bien {:<32} {t}", format!("{}.{}", o.nombre, ruta.extension().unwrap().to_string_lossy()));
                }
                Err(e) => {
                    mal += 1;
                    println!("MAL  {}: {e}", o.nombre);
                }
            }
        }
    }
    if filtro.is_empty() {
        proyecto();
    }
    println!("{bien} bien, {mal} mal");
    std::process::exit((mal > 0) as i32);
}

/// Un proyecto con un plugin de cada clase, exportado con copias nuevas.
fn proyecto() {
    use beethoven_daw_demo::{biblioteca::Biblioteca, modelo::*, motor};
    let todas: Vec<_> = buscar().iter().filter_map(|r| ofertas(r).ok()).flatten().collect();
    let mut p = Proyecto::demo();
    p.clips.retain(|c| c.patron == 3);
    let puesto = |o: &beethoven_daw_demo::plugin::Oferta, uid| PluginPuesto {
        uid,
        ruta: o.ruta.clone(),
        id: o.id.clone(),
        nombre: o.nombre.clone(),
        estado: String::new(),
    };
    let Some(ins) = todas.iter().find(|o| o.instrumento && o.nombre == "Citrus") else { return };
    p.canales[4].plugin = Some(puesto(ins, 1));
    p.mezcla[4].efectos.clear();
    let mut bib = Biblioteca::nueva(48000.0, None);
    let mut seco = motor::a_audio(&p, &mut bib, motor::Modo::Cancion);
    if let Some(fx) = todas.iter().find(|o| !o.instrumento && o.nombre.contains("Delay")) {
        p.mezcla[4].efectos.push(EfectoPuesto { plugin: Some(puesto(fx, 2)), ..EfectoPuesto::nuevo("") });
    }
    let moja = motor::a_audio(&p, &mut Biblioteca::nueva(48000.0, None), motor::Modo::Cancion);
    seco.truncate(moja.len());
    let pico = |x: &[[f32; 2]]| x.iter().flatten().fold(0f32, |m, v| m.max(v.abs()));
    let dif = seco.iter().zip(&moja).map(|(a, b)| (a[0] - b[0]).abs()).fold(0f32, f32::max);
    assert!(pico(&seco) > 0.05 && dif > 0.01, "pico {} diferencia {dif}", pico(&seco));
    println!("proyecto con instrumento y efecto: pico {:.2}, el efecto cambia {dif:.2}", pico(&moja));
}

fn probar(o: &beethoven_daw_demo::plugin::Oferta) -> Result<String, String> {
    let p = Plugin::cargar(&o.ruta, &o.id, 48000.0, &[])?;
    let t = Tiempo { bpm: 120.0, pos: 0.0, tocando: true, sr: 48000.0 };
    let mut pico = 0f32;
    let mut fin = 0f32;
    let mut sale = [[0.0; BLOQUE]; 2];
    for i in 0..1500 {
        let mut entra = [[0.0; BLOQUE]; 2];
        if !o.instrumento && i < 700 {
            for (k, x) in entra.iter_mut().flat_map(|c| c.iter_mut().enumerate()) {
                *x = (((i * BLOQUE + k) as f32) * 0.06).sin() * 0.4;
            }
        }
        let ev: &[Evento] = match i {
            2 => &[Evento::Nota { tono: 60, fuerza: 0.9 }, Evento::Nota { tono: 64, fuerza: 0.9 }],
            700 => &[Evento::Suelta { tono: 60 }, Evento::Suelta { tono: 64 }],
            _ => &[],
        };
        p.procesar(ev, &t, &entra, &mut sale, if i % 3 == 0 { 17 } else { BLOQUE });
        p.atender();
        for x in sale.iter().flatten() {
            if !x.is_finite() {
                return Err("sonido inválido".into());
            }
            pico = pico.max(x.abs());
            if i > 1490 {
                fin = fin.max(x.abs());
            }
        }
    }
    let estado = p.estado();
    // Mover un control y recuperar el estado lo devuelve.
    let mut vuelve = "sin controles";
    if let Some(c) = p.controles.iter().find(|c| !c.entero).or(p.controles.first()) {
        let antes = p.valor(c.id);
        let nuevo = if (antes - c.max).abs() > 1e-6 { c.max } else { c.min };
        p.mover(c.id, nuevo);
        p.procesar(&[], &t, &[[0.0; BLOQUE]; 2], &mut sale, BLOQUE);
        let movido = p.valor(c.id);
        if (movido - nuevo).abs() > 0.02 * (c.max - c.min) {
            return Err(format!("el control {} no se movió: {antes} → {movido}", c.nombre));
        }
        let q = Plugin::cargar(&o.ruta, &o.id, 48000.0, &p.estado())?;
        if (q.valor(c.id) - movido).abs() > 0.02 * (c.max - c.min) {
            return Err(format!("el estado no guardó {}", c.nombre));
        }
        vuelve = "estado ok";
    }
    Ok(format!(
        "{} pico {pico:.3} cola {fin:.4} controles {} estado {} B, {vuelve}",
        if o.instrumento { "INS" } else { "fx " },
        p.controles.len(),
        estado.len()
    ))
}
