// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
#![cfg_attr(windows, windows_subsystem = "windows")]
use beethoven_daw_demo::{biblioteca::Biblioteca, modelo::Proyecto, motor, ui};

fn main() -> eframe::Result {
    if beethoven_daw_demo::ia::mcp::puente() {
        return Ok(());
    }
    if std::env::args().any(|a| a == "--medir") {
        medir();
        return Ok(());
    }
    let abrir = std::env::args().skip(1).find(|a| !a.starts_with("--"));
    let op = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Beethoven DAW Demo")
            .with_inner_size([1360.0, 820.0])
            .with_min_inner_size([1000.0, 620.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Beethoven DAW Demo",
        op,
        Box::new(|cc| {
            let mut a = ui::App::nueva(&cc.egui_ctx, false);
            if let Some(r) = abrir {
                a.abrir(std::path::Path::new(&r));
            }
            // `--ventanas` abre las ventanas de los plugins y `--tocar` empieza a sonar (para probar).
            if std::env::args().any(|a| a == "--ventanas") {
                a.abrir_ventanas();
            }
            if std::env::args().any(|a| a == "--tocar") {
                a.tocar(true);
            }
            Ok(Box::new(a))
        }),
    )
}

/// Cuántas veces más rápido que el tiempo real se toca la demo, en un solo núcleo.
fn medir() {
    beethoven_daw_demo::receta::sin_denormales();
    let mut bib = Biblioteca::nueva(48000.0, None);
    let p = Proyecto::demo();
    for _ in 0..3 {
        let t = std::time::Instant::now();
        let x = motor::a_audio(&p, &mut bib, motor::Modo::Cancion);
        let seg = x.len() as f64 / 48000.0;
        println!("{seg:.1} s de audio en {:.3} s: {:.0}x", t.elapsed().as_secs_f64(), seg / t.elapsed().as_secs_f64());
    }
}
