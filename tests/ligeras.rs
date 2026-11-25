//! Copias ligeras y chip de video con un archivo pesado. `VIDEO=/4k.mp4 cargo test --release --test ligeras -- --ignored --nocapture`
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::time::Instant;
use toma_uno::composicion::Fuente;
use toma_uno::modelo::*;
use toma_uno::video::{Lectores, Modo};

#[test]
#[ignore]
fn video_pesado() {
    let v = std::path::PathBuf::from(std::env::var("VIDEO").unwrap());
    let i = toma_uno::medios::sondear(&v).unwrap();
    let mut p = Proyecto::nuevo();
    (p.ancho, p.alto, p.fps) = i.video.unwrap();
    let mut c = p.nuevo_video(v.clone(), Contenido::Archivo, 0.0, 2.0);
    (c.ancho, c.alto, c.fps, c.total, c.desde) = (p.ancho, p.alto, p.fps, i.dur, 5.2);
    let ver = |n: &str, ligera: bool| -> Vec<f64> {
        let mut l = Lectores::nuevos(60.0, true).con(Modo { yuv: true, ligera, ..Default::default() });
        l.tocando = false;
        // Brincos, como al mover el cursor.
        let mut ms = vec![];
        let mut ts = vec![];
        for t in [1.3, 9.7, 4.1, 12.6, 2.2, 7.9] {
            let t0 = Instant::now();
            let q = l.cuadro(&c, t, 1600, 900).unwrap();
            ms.push(t0.elapsed().as_secs_f64() * 1e3);
            ts.push(q.t);
        }
        ms.sort_by(f64::total_cmp);
        println!("{n}: brinco {:.0} ms (peor {:.0})", ms[3], ms[5]);
        ts
    };
    let a = ver("original", false);
    let d = std::env::temp_dir().join(format!("bc-ligeras-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    assert!(toma_uno::ligeras::pesado(&v));
    let t0 = Instant::now();
    toma_uno::ligeras::pedir(&v, &d);
    while toma_uno::ligeras::para_ver(&v) == v && t0.elapsed().as_secs() < 400 {
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    let q = toma_uno::ligeras::para_ver(&v);
    assert_ne!(q, v);
    println!("copia: {:.0} s, {} MB", t0.elapsed().as_secs_f64(), std::fs::metadata(&q).unwrap().len() >> 20);
    let b = ver("copia ligera", true);
    assert!(a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-3), "los mismos cuadros: {a:?} {b:?}");
    // La exportación lee el original, y sale aunque el chip no sepa el formato.
    p.video = vec![c];
    p.video[0].fx.push(toma_uno::filtros::efecto(50, 1).unwrap());
    let s = d.join("sale.mp4");
    let op = toma_uno::exportar::Opciones { tam: Some((1920, 1080)), ..Default::default() };
    let t0 = Instant::now();
    toma_uno::exportar::exportar_con(&p, &Default::default(), &s, toma_uno::exportar::Modo::Video, &op, &AtomicU32::new(0), &AtomicBool::new(false)).unwrap();
    println!("exportar 2 s: {:.1} s, {} KB", t0.elapsed().as_secs_f64(), std::fs::metadata(&s).unwrap().len() >> 10);
    let o = std::process::Command::new("ffmpeg").args(["-v", "error", "-ss", "1", "-i"]).arg(&s).args(["-frames:v", "1", "-vf", "scale=32:18", "-pix_fmt", "gray", "-f", "rawvideo", "pipe:1"]).output().unwrap().stdout;
    let o2 = std::process::Command::new("ffmpeg").args(["-v", "error", "-ss", "6.2", "-i"]).arg(&v).args(["-frames:v", "1", "-vf", "scale=32:18", "-pix_fmt", "gray", "-f", "rawvideo", "pipe:1"]).output().unwrap().stdout;
    let media = |o: &[u8]| o.iter().map(|x| *x as f64).sum::<f64>() / o.len() as f64;
    println!("brillo medio: exportado {:.0}, original {:.0}", media(&o), media(&o2));
    assert!(media(&o) > 4.0 || media(&o2) < 8.0, "el video no debe salir negro");
    std::fs::remove_dir_all(d).unwrap();
}

/// Reproducir un video pesado: cuadros distintos por segundo una vez que el chip tomó el relevo.
/// `VIDEO=/4k.mp4 /usr/bin/time cargo test --release --test ligeras reproducir -- --ignored --nocapture`
/// (`BC_SIN_TARJETA=1`: achicar con el procesador, para comparar).
#[test]
#[ignore]
fn reproducir_pesado() {
    use std::sync::Arc;
    if std::env::var_os("BC_SIN_TARJETA").is_some() {
        toma_uno::video::SIN_TARJETA.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    let v = std::path::PathBuf::from(std::env::var("VIDEO").unwrap());
    let i = toma_uno::medios::sondear(&v).unwrap();
    let mut p = Proyecto::nuevo();
    (p.ancho, p.alto, p.fps) = i.video.unwrap();
    let mut c = p.nuevo_video(v.clone(), Contenido::Archivo, 0.0, i.dur);
    (c.ancho, c.alto, c.fps, c.total) = (p.ancho, p.alto, p.fps, i.dur);
    p.video.push(c);
    let p = Arc::new(p);
    let mut vp = toma_uno::video::VistaPrevia::default();
    let t0 = Instant::now();
    let mut vistos = std::collections::BTreeSet::new();
    let dur: f64 = std::env::var("DUR").ok().and_then(|d| d.parse().ok()).unwrap_or(6.0);
    while t0.elapsed().as_secs_f64() < dur {
        let t = t0.elapsed().as_secs_f64();
        if let Some(q) = vp.actualizar(&p, 1, t, true, (1600, 900)) {
            if t > 1.5 {
                vistos.insert((q.t * 1000.0) as i64);
            }
        }
        std::thread::sleep(std::time::Duration::from_micros(8300));
    }
    println!("cuadros distintos por segundo (1.5 s a {dur} s): {:.1}", vistos.len() as f64 / (dur - 1.5));
}

/// Cuánto cuesta componer un cuadro con transición (dos clips de color, sin decodificar nada).
/// `cargo test --release --test ligeras transicion -- --ignored --nocapture`
#[test]
#[ignore]
fn costo_transicion() {
    use toma_uno::composicion::{componer, transicion_nueva};
    struct Nada;
    impl toma_uno::composicion::Fuente for Nada {
        fn cuadro(&mut self, _: &ClipVideo, _: f64, _: usize, _: usize) -> Option<std::sync::Arc<toma_uno::composicion::Cuadro>> {
            None
        }
    }
    let ids: Vec<String> = std::env::var("TRANS").unwrap_or("fc:cross_dissolve".into()).split(',').map(String::from).collect();
    for id in std::iter::once(String::new()).chain(ids) {
        let mut p = Proyecto::nuevo();
        let a = p.nuevo_video("a".into(), Contenido::Color([0.8, 0.2, 0.1, 1.0]), 0.0, 2.0);
        let mut b = p.nuevo_video("b".into(), Contenido::Color([0.1, 0.3, 0.9, 1.0]), 2.0, 2.0);
        if !id.is_empty() {
            b.entrada = Some(transicion_nueva(&id));
        }
        p.video = vec![a, b];
        componer(&p, 2.2, 1600, 900, &mut Nada);
        let t0 = Instant::now();
        for k in 0..20 {
            componer(&p, 2.0 + k as f64 * 0.04, 1600, 900, &mut Nada);
        }
        println!("{:24} {:.1} ms/cuadro", if id.is_empty() { "(sin transición)" } else { &id }, t0.elapsed().as_secs_f64() * 1e3 / 20.0);
    }
}
