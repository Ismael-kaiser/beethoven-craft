//! Mide la separación con una mezcla de la que se conoce la voz: carpeta con mezcla.wav, voz.wav y musica.wav.
use std::path::Path;
use std::sync::atomic::*;
use toma_uno::{medios, separar};

fn sdr(real: &[[f32; 2]], est: &[[f32; 2]]) -> f32 {
    let (mut s, mut e) = (0.0, 1e-12);
    for (r, x) in real.as_flattened().iter().zip(est.as_flattened()) {
        s += r * r;
        e += (r - x) * (r - x);
    }
    10.0 * (s / e).log10()
}

fn main() {
    let d = std::env::args().nth(1).unwrap();
    let lee = |n: &str| medios::decodificar_a(&Path::new(&d).join(n), separar::SR44).unwrap();
    let (x, v, m) = (lee("mezcla.wav"), lee("voz.wav"), lee("musica.wav"));
    let t = std::time::Instant::now();
    let modelo = separar::cargar(&separar::ruta_modelo().unwrap()).unwrap();
    let (ev, ei) = separar::separar(&modelo, &x, &AtomicU32::new(0), &AtomicBool::new(false)).unwrap();
    println!("{:.1} s de audio en {:.1} s", x.len() as f32 / 44100.0, t.elapsed().as_secs_f32());
    println!("voz: {:.1} dB en la mezcla -> {:.1} dB separada", sdr(&v, &x), sdr(&v, &ev));
    println!("música: {:.1} dB en la mezcla -> {:.1} dB separada", sdr(&m, &x), sdr(&m, &ei));
}
