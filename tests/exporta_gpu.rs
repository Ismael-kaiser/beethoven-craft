//! El archivo exportado sale igual por la tarjeta que por el procesador. `VIDEO=… -- --ignored --nocapture`
use std::sync::atomic::{AtomicBool, AtomicU32};
use toma_uno::exportar::*;
use toma_uno::modelo::*;

#[test]
#[ignore]
fn mismos_colores() {
    let v = std::path::PathBuf::from(std::env::var("VIDEO").unwrap());
    let i = toma_uno::medios::sondear(&v).unwrap();
    let mut p = Proyecto::nuevo();
    (p.ancho, p.alto, p.fps) = i.video.unwrap();
    let mut c = p.nuevo_video(v.clone(), Contenido::Archivo, 0.0, 1.5);
    (c.ancho, c.alto, c.fps, c.total, c.desde) = (p.ancho, p.alto, p.fps, i.dur, 60.0);
    p.video = vec![c];
    let dir = std::env::temp_dir();
    let saca = |p: &Proyecto, n: &str, codec| {
        let s = dir.join(format!("bc-color-{n}.{}", if codec == Codec::ProRes { "mov" } else { "mp4" }));
        exportar_con(p, &Default::default(), &s, Modo::Video, &Opciones { codec, calidad: 2, ..Default::default() }, &AtomicU32::new(0), &AtomicBool::new(false)).unwrap();
        s
    };
    let psnr = |a: &std::path::Path, b: &std::path::Path, desde: f64| {
        let o = std::process::Command::new("ffmpeg").args(["-hide_banner", "-ss", &desde.to_string(), "-i"]).arg(a).arg("-i").arg(b).args(["-t", "1.4", "-lavfi", "[0:v]setpts=PTS-STARTPTS,fps=24000/1001,format=yuv420p[a];[1:v]setpts=PTS-STARTPTS,fps=24000/1001,format=yuv420p[b];[a][b]psnr", "-f", "null", "-"]).output().unwrap();
        let s = String::from_utf8_lossy(&o.stderr).to_string();
        s.lines().rev().find(|l| l.contains("PSNR")).unwrap_or("?").to_string()
    };
    // Sin cambios, contra el original.
    let a = saca(&p, "limpio", Codec::H264);
    println!("limpio contra original: {}", psnr(&v, &a, 60.0));
    p.video[0].fx.push(toma_uno::filtros::efecto(50, 1).unwrap());
    for codec in [Codec::H264, Codec::ProRes] {
        toma_uno::vgpu::apagar(false);
        let a = saca(&p, "gpu", codec);
        toma_uno::vgpu::apagar(true);
        let b = saca(&p, "cpu", codec);
        println!("{codec:?} tarjeta contra procesador: {}", psnr(&a, &b, 0.0));
    }
}
