//! Tirones con transiciones pesadas: un rato en pausa (como cuando se edita) y luego reproducir.
//! `cargo test --release --test fluidez_transiciones -- --ignored --nocapture` (PAUSA=segundos, GRANDE=1)
use std::sync::Arc;
use std::time::{Duration, Instant};
use toma_uno::modelo::*;
use toma_uno::video::VistaPrevia;

#[test]
#[ignore]
fn con_transiciones() {
    let mut p = Proyecto::nuevo();
    (p.ancho, p.alto, p.fps) = (1920, 1080, 24.0);
    let colores = [[0.7, 0.2, 0.2, 1.0], [0.2, 0.5, 0.8, 1.0], [0.3, 0.7, 0.3, 1.0], [0.8, 0.6, 0.2, 1.0]];
    // Cuatro clips de 3 s; las uniones en 3, 6 y 9 s con transiciones pesadas de 1 s.
    for (k, col) in colores.iter().enumerate() {
        let mut c = p.nuevo_video("c".into(), Contenido::Color(*col), k as f64 * 3.0, 3.0);
        if k > 0 {
            c.entrada = Some(toma_uno::composicion::transicion_nueva(["earthquake", "radial_blur", "flip_motion"][k - 1]));
        }
        p.video.push(c);
    }
    let p = Arc::new(p);
    let tam = if std::env::var_os("GRANDE").is_some() { (1920, 1080) } else { (1280, 720) };
    let pausa: f64 = std::env::var("PAUSA").ok().and_then(|s| s.parse().ok()).unwrap_or(8.0);
    let mut v = VistaPrevia::default();
    let t0 = Instant::now();
    while t0.elapsed().as_secs_f64() < pausa {
        let _ = v.actualizar(&p, 1, 0.0, false, tam);
        std::thread::sleep(Duration::from_millis(16));
    }
    // Reproducir de 0 a 12 s en tiempo real.
    let t0 = Instant::now();
    let (mut vistos, mut chicos, mut atraso) = (vec![], 0, vec![]);
    while t0.elapsed().as_secs_f64() < 12.0 {
        let t = t0.elapsed().as_secs_f64();
        if let Some(q) = v.actualizar(&p, 1, t, true, tam) {
            vistos.push(t);
            atraso.push(t - q.t);
            if q.ancho < tam.0 {
                chicos += 1;
            }
        }
        std::thread::sleep(Duration::from_millis(8));
    }
    let hueco = vistos.windows(2).filter(|w| w[0] > 0.5).map(|w| w[1] - w[0]).fold(0.0, f64::max);
    atraso.sort_by(f64::total_cmp);
    println!(
        "pausa {pausa:.0} s, {}x{}: {:.1} cps, hueco máx {:.0} ms, {} de {} cuadros en calidad baja, atraso medio {:.0} ms",
        tam.0,
        tam.1,
        vistos.len() as f64 / 12.0,
        hueco * 1e3,
        chicos,
        vistos.len(),
        atraso.get(atraso.len() / 2).copied().unwrap_or(9.0) * 1e3
    );
}

/// La barra de la regla: primero hay tramos pendientes y, en pausa, se van llenando hasta quedar listos.
#[test]
#[ignore]
fn barra_de_preparacion() {
    use egui_kittest::Harness;
    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| toma_uno::ui::App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    {
        let p = &mut h.state_mut().p;
        for (k, col) in [[0.7, 0.2, 0.2, 1.0], [0.2, 0.5, 0.8, 1.0], [0.3, 0.7, 0.3, 1.0]].iter().enumerate() {
            let mut c = p.nuevo_video("c".into(), Contenido::Color(*col), k as f64 * 3.0, 3.0);
            if k > 0 {
                c.entrada = Some(toma_uno::composicion::transicion_nueva(["earthquake", "flip_motion"][k - 1]));
            }
            p.video.push(c);
        }
    }
    h.state_mut().linea.encuadrar = true;
    let foto = |h: &mut Harness<toma_uno::ui::App>, n: &str| {
        if let Ok(c) = std::env::var("CAPTURAS") {
            h.render().unwrap().save(format!("{c}/{n}.png")).unwrap();
        }
    };
    let espera = |h: &mut Harness<toma_uno::ui::App>, s: f64| {
        let t0 = Instant::now();
        while t0.elapsed().as_secs_f64() < s {
            std::thread::sleep(Duration::from_millis(30));
            h.run_steps(1);
        }
    };
    espera(&mut h, 1.2);
    let al_inicio = h.state().preparado();
    foto(&mut h, "barra-antes");
    espera(&mut h, 14.0);
    let despues = h.state().preparado();
    foto(&mut h, "barra-despues");
    println!("al inicio: {al_inicio:?}\ndespués: {despues:?}");
    assert!(al_inicio.iter().any(|x| !x.2), "al principio algo debía faltar");
    assert!(!despues.is_empty() && despues.iter().all(|x| x.2), "en pausa todo debía quedar listo");
}
