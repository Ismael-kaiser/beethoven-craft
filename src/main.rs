// Beethoven Craft. Copyright (C) 2026. SPDX-License-Identifier: GPL-3.0-or-later
// En Windows, sin ventana de consola detrás de la app.
#![cfg_attr(windows, windows_subsystem = "windows")]
fn main() -> eframe::Result {
    // `--mudo` abre la app sin mandar sonido (para probar).
    toma_uno::tiempos::marca("main");
    let mudo = std::env::args().any(|a| a == "--mudo");
    let abrir = std::env::args().skip(1).find(|a| !a.starts_with("--"));
    // Sin archivo arranca en el menú, que es una ventana más chica.
    let tam = if abrir.is_some() { toma_uno::ui::inicio::EDITOR } else { toma_uno::ui::inicio::MENU };
    // Se dejan libres procesadores para la ventana y el sonido.
    let n = std::thread::available_parallelism().map_or(4, |n| n.get());
    let _ = rayon::ThreadPoolBuilder::new().num_threads(n.saturating_sub(2).max(2)).build_global();
    let op = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Beethoven Craft")
            .with_icon(eframe::egui::IconData { rgba: toma_uno::ui::ICONO.to_vec(), width: 512, height: 512 })
            .with_inner_size(tam)
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Beethoven Craft",
        op,
        Box::new(move |cc| {
            toma_uno::tiempos::marca("ventana creada");
            let mut a = toma_uno::ui::App::nueva(&cc.egui_ctx, false, mudo);
            toma_uno::tiempos::marca("App::nueva lista");
            // `--ventanas` abre las ventanas al arrancar.
            if std::env::args().any(|a| a == "--ventanas") {
                a.ventanas = [true; 3];
            }
            match abrir {
                Some(r) => {
                    let r = std::path::PathBuf::from(r);
                    if toma_uno::ui::es_proyecto(&r) { a.abrir(r) } else { a.importar(r) }
                }
                // Sin archivo, se abre en el menú principal.
                None => a.inicio.mostrar(),
            }
            Ok(Box::new(a))
        }),
    )
}
