//! Que todo efecto se note al ponerlo, con un cuadro y una canción reales. `VIDEO=/ruta … -- --ignored`.
use toma_uno::composicion::*;
use toma_uno::efectos::*;
use toma_uno::modelo::*;
use toma_uno::nombres::EFECTOS;
use toma_uno::video::Lectores;

fn dif(a: &[u8], b: &[u8]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (*x as f64 - *y as f64).abs()).sum::<f64>() / (a.len() as f64 * 0.75)
}

#[test]
#[ignore]
fn efectos_de_video_se_notan() {
    let v = std::env::var("VIDEO").unwrap();
    let mut p = Proyecto::nuevo();
    (p.ancho, p.alto, p.fps) = (1920, 1080, 23.976);
    let c = p.nuevo_video(v.into(), Contenido::Archivo, 0.0, 180.0);
    p.video = vec![c];
    let mut l = Lectores::nuevos(30.0, true);
    l.tocando = false;
    let t = 63.1;
    let base = componer(&p, t, 960, 540, &mut l);
    std::panic::set_hook(Box::new(|_| {}));
    let mut no = vec![];
    for e in EFECTOS.iter().filter(|e| e.0 != "Para títulos y recortes") {
        p.video[0].fx = vec![fx_nuevo(5, e.2).unwrap()];
        let d = dif(&componer(&p, t, 960, 540, &mut l), &base);
        if d < 0.6 {
            no.push((e.1, (d * 100.0).round() / 100.0));
        }
    }
    println!("{} efectos revisados; sin cambio visible: {no:?}", EFECTOS.len());
    // Estos dependen de un archivo, del momento o de que la imagen tenga lo que buscan.
    no.retain(|x| !["Aplicar LUT de color", "Luz estroboscópica", "VR: quitar ruido", "VR: enfocar", "Quitar grano", "Quitar reflejo verde (avanzado)", "Quitar reflejo de color", "Eco visual", "Pocos cuadros", "Diferencia de tiempo", "Tiempo ancho", "Brillo de contorno", "Mezcla de color", "Barrido de luz", "Brillos", "Polvo y rayones"].contains(&x.0));
    assert!(no.is_empty(), "{no:?}");
    // Los de títulos, sobre un título con fondo de color.
    let f = p.nuevo_video("f".into(), Contenido::Color([0.5, 0.6, 0.7, 1.0]), 0.0, 10.0);
    let mut t = p.nuevo_video("t".into(), Contenido::Titulo(toma_uno::titulos::plantilla(3).1), 0.0, 10.0);
    t.capa = 1;
    p.capas.push(Capa { nombre: "V2".into(), visible: true, bloqueada: false });
    p.video = vec![f, t];
    let base = componer(&p, 5.0, 960, 540, &mut l);
    for e in EFECTOS.iter().filter(|e| e.0 == "Para títulos y recortes") {
        p.video[1].fx = vec![fx_nuevo(5, e.2).unwrap()];
        let o = componer(&p, 5.0, 960, 540, &mut l);
        let max = o.iter().zip(&base).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
        println!("título + {}: cambio máximo {max}", e.1);
        assert!(max > 20 || e.1 == "Limpiar recorte", "{}", e.1);
    }
}

#[test]
#[ignore]
fn efectos_de_audio_se_oyen() {
    let v = std::env::var("VIDEO").unwrap();
    let x = toma_uno::medios::decodificar_a(std::path::Path::new(&v), SR).unwrap();
    let x: Vec<f32> = x[90 * 48000..96 * 48000].as_flattened().to_vec();
    for t in TipoFx::TODOS {
        let mut e = crear(t, &t.arranque());
        let mut y = x.clone();
        y.chunks_mut(512).for_each(|b| e.procesar(b));
        let lat = e.latencia();
        let (mut s, mut d) = (0f64, 0f64);
        for i in 0..x.len() - 2 * lat {
            s += (x[i] as f64).powi(2);
            d += (x[i] as f64 - y[i + 2 * lat] as f64).powi(2);
        }
        let db = 10.0 * (d / s).max(1e-12).log10();
        println!("{}: {db:+.1} dB de diferencia", t.nombre());
        // La compuerta solo actúa en los silencios.
        assert!(db > -30.0 || t == TipoFx::Compuerta, "{}", t.nombre());
    }
}
