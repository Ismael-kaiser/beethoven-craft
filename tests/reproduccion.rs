//! Cuántos cuadros distintos se ven por segundo al reproducir con efectos, en tiempo real.
//! `VIDEO=… cargo test --release --test reproduccion -- --ignored --nocapture`
use std::sync::Arc;
use std::time::{Duration, Instant};
use toma_uno::composicion::fx_nuevo;
use toma_uno::modelo::*;
use toma_uno::video::VistaPrevia;

#[test]
#[ignore]
fn fluidez() {
    let v = std::path::PathBuf::from(std::env::var("VIDEO").unwrap());
    let i = toma_uno::medios::sondear(&v).unwrap();
    let base = |fx: &[&str], estilo: &str| {
        let mut p = Proyecto::nuevo();
        (p.ancho, p.alto, p.fps) = i.video.unwrap();
        let mut c = p.nuevo_video(v.clone(), Contenido::Archivo, 0.0, 120.0);
        (c.ancho, c.alto, c.fps, c.total) = (p.ancho, p.alto, p.fps, i.dur);
        c.desde = 60.0;
        c.fx = fx.iter().enumerate().map(|(k, f)| fx_nuevo(10 + k as u64, f).unwrap()).collect();
        let id = c.id;
        p.video = vec![c];
        if let Some(k) = toma_uno::estilos::ESTILOS.iter().position(|e| e.nombre == estilo) {
            toma_uno::estilos::aplicar(&mut p, id, k);
        }
        Arc::new(p)
    };
    let casos = [
        ("sin efectos", base(&[], "")),
        ("color y viñeta", base(&["ec.color.lumetri", "bc:color"], "")),
        ("VHS + neón", base(&["bc:vhs", "bc:neon"], "")),
        ("estilo Concierto", base(&[], "Concierto")),
        ("desenfoque direccional", base(&["ec.blur.directional"], "")),
        ("tres pesados", base(&["fc:volumetric_rays", "ec.blur.gaussian", "fc:glint"], "")),
    ];
    let tam = if std::env::var_os("GRANDE").is_some() { (1920, 1080) } else { (960, 540) };
    let filtro = std::env::var("CASO").unwrap_or_default();
    for (n, p) in casos.iter().filter(|c| c.0.contains(&filtro)) {
        for vuelta in 0..2 {
            let mut v = VistaPrevia::default();
            let corre = |v: &mut VistaPrevia, ini: f64| {
                let t0 = Instant::now();
                let (mut vistos, mut atraso, mut ancho) = (vec![], vec![], 0);
                while t0.elapsed().as_secs_f64() < 5.0 {
                    let t = ini + t0.elapsed().as_secs_f64();
                    if let Some(q) = v.actualizar(p, 1, t, true, tam) {
                        vistos.push(t0.elapsed().as_secs_f64());
                        atraso.push(t - q.t);
                        ancho = q.ancho;
                    }
                    std::thread::sleep(Duration::from_millis(8));
                }
                // El hueco más largo sin cuadro nuevo, ya arrancado.
                let hueco = vistos.windows(2).filter(|w| w[0] > 1.0).map(|w| w[1] - w[0]).fold(0.0, f64::max);
                atraso.sort_by(f64::total_cmp);
                println!("{n} · {}: {:.1} cps, hueco máx {:.0} ms, atraso medio {:.0} ms, ancho {ancho}", ["fría", "repetida"][vuelta], vistos.len() as f64 / 5.0, hueco * 1e3, atraso.get(atraso.len() / 2).copied().unwrap_or(9.0) * 1e3);
            };
            if vuelta == 0 {
                corre(&mut v, 2.0);
            } else {
                // La segunda pasada por el mismo pedazo.
                let t0 = Instant::now();
                while t0.elapsed().as_secs_f64() < 5.0 {
                    v.actualizar(p, 1, 2.0 + t0.elapsed().as_secs_f64(), true, (960, 540));
                    std::thread::sleep(Duration::from_millis(8));
                }
                corre(&mut v, 2.0);
            }
        }
    }
}

/// Al reproducir, el procesador le pasa la decodificación al chip sin que se note ni cambie ningún cuadro.
#[test]
#[ignore]
fn relevo_del_chip() {
    use toma_uno::composicion::Fuente;
    use toma_uno::video::{Lectores, Modo};
    let v = std::path::PathBuf::from(std::env::var("VIDEO").unwrap());
    let i = toma_uno::medios::sondear(&v).unwrap();
    let mut p = Proyecto::nuevo();
    let mut c = p.nuevo_video(v, Contenido::Archivo, 0.0, 60.0);
    let (w, h, fps) = i.video.unwrap();
    (c.ancho, c.alto, c.fps, c.total) = (w, h, fps, i.dur);
    let modo = Modo { yuv: true, ..Default::default() };
    let mut l = Lectores::nuevos(60.0, true).con(modo);
    let mut s = Lectores::nuevos(60.0, true).con(modo);
    l.seguido = true;
    let (mut cambio, mut peor) = (None, 0.0f64);
    for n in 0..(3.0 * fps) as usize {
        let t = 63.0 + (n as f64 + 0.02) / fps;
        let t0 = Instant::now();
        let a = l.cuadro(&c, t, 960, 540).unwrap();
        peor = peor.max(if n > 0 { t0.elapsed().as_secs_f64() } else { 0.0 });
        let b = s.cuadro(&c, t, 960, 540).unwrap();
        assert!((a.t - b.t).abs() < 1e-3, "{n}: {} {}", a.t, b.t);
        let d = a.yuv.iter().zip(&b.yuv).map(|(x, y)| x.abs_diff(*y) as u64).sum::<u64>() as f64 / a.yuv.len() as f64;
        assert!(d < 1.0, "{n}: {d}");
        if cambio.is_none() && l.con_chip() > 0 {
            cambio = Some(n);
        }
        std::thread::sleep(Duration::from_secs_f64(0.8 / fps));
    }
    println!("pasó al chip en el cuadro {cambio:?}; espera más larga {:.0} ms", peor * 1e3);
    assert!(cambio.is_some());
}
