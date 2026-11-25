//! Aviso de video que el chip no lee y ventana de exportación. `VIDEO=… CAPTURAS=/carpeta … -- --ignored --nocapture`
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use toma_uno::ui::App;

fn foto(h: &mut Harness<App>, n: &str) {
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/{n}.png")).unwrap();
    }
}

#[test]
#[ignore]
fn convertir_y_exportar() {
    let v = std::path::PathBuf::from(std::env::var("VIDEO").unwrap());
    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    h.state_mut().importar(v);
    for _ in 0..300 {
        h.run_steps(1);
        if h.state().convertir.is_some() || h.state().preguntar.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    h.run_steps(4);
    assert!(h.state().convertir.is_some(), "debía preguntar si convertir");
    foto(&mut h, "convertir");
    h.get_by_label("Usar el original").click();
    h.run_steps(3);
    assert!(h.state().preguntar.is_some());
    h.get_by_label("Todo junto").click();
    for _ in 0..600 {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(20));
        if !h.state().ocupada() && !h.state().p.pistas.is_empty() {
            break;
        }
    }
    h.state_mut().hacer_atajo("exportar");
    h.run_steps(6);
    foto(&mut h, "exportar");
    h.get_by_label("TikTok / Reels / Shorts").click();
    h.run_steps(4);
    foto(&mut h, "exportar-tiktok");
    h.get_by_label("Cancelar").click();
    h.run_steps(3);
}

/// Soltar un video (como desde el panel de archivos): revisa el chip, pregunta por el audio y lo pone donde se soltó.
#[test]
#[ignore]
fn soltar_pregunta_y_respeta_el_lugar() {
    let v = std::path::PathBuf::from(std::env::var("VIDEO").unwrap());
    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    h.state_mut().poner_archivos(&[v], 3.0, None, None);
    for _ in 0..300 {
        h.run_steps(1);
        if h.state().convertir.is_some() || h.state().preguntar.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    h.run_steps(3);
    if h.state().convertir.is_some() {
        h.get_by_label("Usar el original").click();
        h.run_steps(3);
    }
    assert!(h.state().preguntar.is_some(), "debía preguntar qué hacer con el audio");
    foto(&mut h, "soltar-pregunta");
    h.get_by_label("Todo junto").click();
    h.run_steps(3);
    let c = &h.state().p.video[0];
    assert!((c.inicio - 3.0).abs() < 1e-6, "debía quedar donde se soltó: {}", c.inicio);
}
