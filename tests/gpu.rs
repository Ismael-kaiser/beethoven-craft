//! Que la tarjeta gráfica dibuje lo mismo que el procesador. Sin tarjeta, no prueba nada.
//! `CUANTO=1 cargo test --release --test gpu -- --nocapture` dice cuánto se parece y tarda cada efecto.
use std::sync::Arc;
use std::time::Instant;
use toma_uno::composicion::*;
use toma_uno::modelo::*;
use toma_uno::nombres::EFECTOS;
use toma_uno::vgpu;

/// Un cuadro con de todo: degradados, orillas, luces y detalle.
struct Fija;
impl Fuente for Fija {
    fn cuadro(&mut self, _: &ClipVideo, t: f64, w: usize, h: usize) -> Option<Arc<Cuadro>> {
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let (u, v) = (x as f32 / w as f32, y as f32 / h as f32);
                let d = ((u - 0.6).powi(2) * 3.0 + (v - 0.4).powi(2)).sqrt();
                let l = if d < 0.08 { 1.0 } else { 0.15 + 0.5 * (u * 9.0 + (v * 7.0).sin()).sin().abs() };
                let q = if (x / 40 + y / 40) % 7 == 0 { 0.9 } else { 0.4 };
                rgba.extend([(l * 255.0) as u8, (l * q * 255.0) as u8, (v * 200.0) as u8, 255]);
            }
        }
        Some(Arc::new(Cuadro { t, ancho: w, alto: h, rgba, ..Default::default() }))
    }
}

fn base() -> Proyecto {
    let mut p = Proyecto::nuevo();
    (p.ancho, p.alto, p.fps) = (1920, 1080, 30.0);
    let mut c = p.nuevo_video("x.mp4".into(), Contenido::Archivo, 0.0, 20.0);
    (c.ancho, c.alto, c.total) = (1920, 1080, 20.0);
    p.video = vec![c];
    p
}

fn dif(a: &[u8], b: &[u8]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x.abs_diff(*y) as u64).sum::<u64>() as f64 / (a.len() as f64 * 0.75)
}

#[test]
fn igual_que_el_procesador() {
    if vgpu::gpu().is_none() {
        println!("sin tarjeta");
        return;
    }
    let cuanto = std::env::var_os("CUANTO").is_some();
    let (w, h) = if cuanto { (1920, 1080) } else { (640, 360) };
    let mut p = base();
    let mut f = Fija;
    let mut mal = vec![];
    let mut tabla = vec![];
    let t = 3.37;
    let solo = std::env::var("SOLO").unwrap_or_default();
    for e in EFECTOS.iter().filter(|e| e.0 != "Para títulos y recortes" && e.2.starts_with(&solo)) {
        let fx = fx_nuevo(5, e.2).unwrap();
        if !vgpu::en_tarjeta(&fx) {
            continue;
        }
        p.video[0].fx = vec![fx];
        let a = componer_cpu(&p, t, w, h, &mut f);
        let Some(b) = vgpu::componer(&p, t, w, h, &mut f, vgpu::Salida::Rgba) else {
            mal.push((e.2, -1.0));
            continue;
        };
        let d = dif(&a, &b);
        // Estos dos caen justo en la orilla de una línea o de un pixel.
        if d > 1.0 && !["bc:tv", "ec.perspective.ccenvironment"].contains(&e.2) {
            mal.push((e.2, d));
        }
        if cuanto {
            let t0 = Instant::now();
            f.cuadro(&p.video[0], t, w, h);
            let fuente = t0.elapsed().as_secs_f64();
            let cuesta = |g: &mut dyn FnMut()| {
                g();
                let t0 = Instant::now();
                g();
                (t0.elapsed().as_secs_f64() - fuente) * 1e3
            };
            let c = cuesta(&mut || { componer_cpu(&p, t, w, h, &mut f); });
            let g = cuesta(&mut || { vgpu::componer(&p, t, w, h, &mut f, vgpu::Salida::Rgba); });
            tabla.push((e.2, d, c, g));
        }
    }
    tabla.sort_by(|a, b| b.3.total_cmp(&a.3));
    for (n, d, c, g) in &tabla {
        println!("{n}: diferencia {d:.2}, procesador {c:.1} ms, tarjeta {g:.1} ms");
    }
    println!("{} distintos: {mal:?}", mal.len());
    assert!(mal.is_empty());
}

/// Las transiciones que se hacen en la tarjeta (disolvencias y pasar por un color) dan lo mismo que en el procesador,
/// de un video a un color y de un clip al vacío.
#[test]
fn transiciones_igual_que_el_procesador() {
    if vgpu::gpu().is_none() {
        println!("sin tarjeta");
        return;
    }
    let (w, h) = (640, 360);
    let mut f = Fija;
    let mut mal = vec![];
    for tipo in ["cross_dissolve", "additive_dissolve", "film_dissolve", "dip_to_black", "dip_to_white"] {
        let mut p = base();
        p.video[0].dur = 5.0;
        let mut b = p.nuevo_video("color".into(), Contenido::Color([0.2, 0.5, 0.9, 1.0]), 5.0, 5.0);
        b.entrada = Some(transicion_nueva(tipo));
        b.salida = Some(transicion_nueva(tipo));
        p.video.push(b);
        for t in [5.1, 5.5, 5.85, 9.5] {
            let a = componer_cpu(&p, t, w, h, &mut f);
            let Some(g) = vgpu::componer(&p, t, w, h, &mut f, vgpu::Salida::Rgba) else {
                mal.push((tipo, t, -1.0));
                continue;
            };
            let d = dif(&a, &g);
            if d > 1.0 {
                mal.push((tipo, t, d));
            }
        }
    }
    println!("distintas: {mal:?}");
    assert!(mal.is_empty());
}

/// Qué estilos tienen efectos que la tarjeta no hace, y cuánto cuesta un cuadro con cada uno.
/// `cargo test --release --test gpu estilos -- --ignored --nocapture`
#[test]
#[ignore]
fn estilos_en_la_tarjeta() {
    let (w, h) = (1600, 900);
    let mut f = Fija;
    let mut filas = vec![];
    for (i, e) in toma_uno::estilos::ESTILOS.iter().enumerate() {
        let mut p = base();
        let id = p.video[0].id;
        toma_uno::estilos::aplicar(&mut p, id, i);
        let fuera: Vec<String> = p.video[0].fx.iter().filter(|x| !vgpu::en_tarjeta(x)).map(|x| x.tipo.clone()).collect();
        componer(&p, 3.0, w, h, &mut f);
        let t0 = Instant::now();
        for k in 0..5 {
            componer(&p, 3.0 + k as f64 * 0.033, w, h, &mut f);
        }
        filas.push((t0.elapsed().as_secs_f64() * 1e3 / 5.0, e.nombre, fuera));
    }
    filas.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (ms, n, fuera) in filas {
        println!("{ms:6.1} ms  {n:28} {}", if fuera.is_empty() { String::new() } else { format!("procesador: {fuera:?}") });
    }
}

/// Las barras de cine son del cuadro: con zoom al clip, se quedan del mismo alto (en la tarjeta y en el procesador).
#[test]
fn barras_de_cine_no_hacen_zoom() {
    use toma_uno::composicion::{componer_cpu, fx_nuevo};
    use toma_uno::modelo::*;
    let alto_barra = |rgba: &[u8], w: usize, h: usize| (0..h).take_while(|y| {
        let i = (y * w + w / 2) * 4;
        rgba[i] < 20 && rgba[i + 1] < 20 && rgba[i + 2] < 20
    }).count();
    let hacer = |esc: f64| {
        let mut p = Proyecto::nuevo();
        (p.ancho, p.alto) = (640, 360);
        let mut c = p.nuevo_video("c".into(), Contenido::Color([0.8, 0.8, 0.8, 1.0]), 0.0, 2.0);
        c.fx.push(fx_nuevo(7, "bc:barras").unwrap());
        c.mov.escala = Prop::num(esc);
        p.video.push(c);
        p
    };
    let (w, h) = (640, 360);
    let cpu: Vec<usize> = [100.0, 150.0].iter().map(|e| alto_barra(&componer_cpu(&hacer(*e), 1.0, w, h, &mut Fija), w, h)).collect();
    println!("procesador: barras {cpu:?}");
    assert!(cpu[0] > 5, "debía haber barras");
    assert_eq!(cpu[0], cpu[1], "en el procesador las barras crecieron con el zoom");
    if toma_uno::vgpu::gpu().is_some() {
        let gpu: Vec<usize> = [100.0, 150.0].iter().map(|e| {
            let v = toma_uno::vgpu::componer(&hacer(*e), 1.0, w, h, &mut Fija, toma_uno::vgpu::Salida::Rgba).expect("en la tarjeta");
            alto_barra(&v, w, h)
        }).collect();
        println!("tarjeta: barras {gpu:?}");
        assert!((gpu[0] as i64 - gpu[1] as i64).abs() <= 1, "en la tarjeta las barras crecieron con el zoom");
    }
}
