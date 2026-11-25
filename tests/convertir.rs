//! Detectar si el chip lee un video y convertirlo. `VIDEO=… cargo test --release --test convertir -- --ignored --nocapture`
use std::sync::atomic::{AtomicBool, AtomicU32};
use toma_uno::convertir;

#[test]
#[ignore]
fn detectar_y_convertir() {
    let v = std::path::PathBuf::from(std::env::var("VIDEO").unwrap());
    let i = toma_uno::medios::sondear(&v).unwrap();
    let t = std::time::Instant::now();
    let a = convertir::revisar(&v, i.video.unwrap(), i.dur);
    println!("revisar: {a:?} en {:.2} s", t.elapsed().as_secs_f64());
    let Some(_) = a else { return };
    // Solo 3 s, para no tardar: un pedazo del original.
    let dir = std::env::temp_dir().join("bc-convertir");
    let corto = dir.join("corto.mkv");
    std::fs::create_dir_all(&dir).unwrap();
    let ok = std::process::Command::new("ffmpeg").args(["-v", "error", "-y", "-i"]).arg(&v).args(["-t", "3", "-c", "copy"]).arg(&corto).status().unwrap();
    assert!(ok.success());
    let _ = std::fs::remove_file(convertir::destino(&corto, &dir));
    let r = convertir::convertir(&corto, &dir, 3.0, &AtomicU32::new(0), &AtomicBool::new(false)).unwrap();
    println!("copia: {}", r.display());
    assert_eq!(convertir::lo_lee_el_chip(&r), Some(true));
    let j = toma_uno::medios::sondear(&r).unwrap();
    assert_eq!(j.video.map(|x| (x.0, x.1)), i.video.map(|x| (x.0, x.1)));
    assert_eq!(j.audio, i.audio);
}
