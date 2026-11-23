//! Los estilos listos usan efectos y controles que existen, y cambian la imagen.
use toma_uno::composicion::fx_nuevo;
use toma_uno::estilos::*;
use toma_uno::modelo::*;

#[test]
fn estilos_validos() {
    for (i, e) in ESTILOS.iter().enumerate() {
        for (tipo, vals) in e.fx {
            let f = fx_nuevo(1, tipo).unwrap_or_else(|| panic!("{}: no existe {tipo}", e.nombre));
            for (k, ..) in *vals {
                assert!(matches!(f.p.get(*k).map(|q| &q.v), Some(Valor::Num(_))), "{}: {tipo} no tiene {k}: {:?}", e.nombre, f.p.keys().collect::<Vec<_>>());
            }
        }
        let mut p = Proyecto::nuevo();
        let c = p.nuevo_video("/x".into(), Contenido::Archivo, 0.0, 4.0);
        let id = c.id;
        p.video.push(c);
        assert!(aplicar(&mut p, id, i) > 0, "{}", e.nombre);
    }
}

#[test]
fn late_en_los_golpes() {
    // Un golpe por segundo.
    let n = SR as usize * 6;
    let cuadros: Vec<[f32; 2]> = (0..n)
        .map(|i| {
            let t = (i % SR as usize) as f32 / SR as f32;
            let x = 0.8 * (-t * 14.0).exp() * (t * 900.0).sin() + 0.01 * (i as f32 * 0.05).sin();
            [x, x]
        })
        .collect();
    let a = toma_uno::medios::Audio::nuevo(cuadros);
    let g = golpes(&a, 0.0, 6.0);
    for s in 1..6 {
        assert!(g.iter().any(|x| (x.0 - s as f64).abs() < 0.04), "falta el de {s}: {g:?}");
    }
    assert!(g.len() <= 7, "{g:?}");
    let mut p = Proyecto::nuevo();
    let mut c = p.nuevo_video("/x".into(), Contenido::Archivo, 0.0, 6.0);
    assert!(latir(&mut c, &g, 0.1) >= 4);
    assert!(c.mov.escala.claves.windows(2).all(|w| w[0].0 < w[1].0));
    assert!(c.mov.escala.en(3.02).num() > 101.0 && c.mov.escala.en(3.6).num() < 100.5);
}

/// Cada efecto propio cambia la imagen, no truena a ningún tamaño y los que se mueven, se mueven.
#[test]
fn efectos_propios_de_video() {
    use toma_uno::vpropios::{DEFS, aplicar, nuevo};
    let img = |w: u32, h: u32| {
        let mut i = effectcraft_raster::Image::new(w, h);
        for (k, p) in i.data.iter_mut().enumerate() {
            let (x, y) = ((k as u32 % w) as f32 / w as f32, (k as u32 / w) as f32 / h as f32);
            let c = ((x * 9.0).sin() * (y * 7.0).cos() * 0.4 + 0.5).clamp(0.0, 1.0);
            *p = [c, x, 1.0 - y, 1.0];
        }
        i
    };
    let dif = |a: &effectcraft_raster::Image, b: &effectcraft_raster::Image| a.data.iter().zip(&b.data).map(|(a, b)| (0..4).map(|k| (a[k] - b[k]).abs()).sum::<f32>()).sum::<f32>() / a.data.len() as f32;
    let o = img(320, 180);
    // La mezcla de color llega en neutro: se prueba en `nodos`.
    for d in DEFS.iter().filter(|d| d.id != "bc:color") {
        let f = nuevo(7, d);
        // El volteo no cambia en el primer golpe: se prueba en el segundo.
        let ini = std::time::Instant::now();
        let r = aplicar(&f, 0.52, &o, 1.0);
        let ms = ini.elapsed().as_secs_f64() * 1000.0;
        // Algunos solo actúan a ratos (un latido, un relámpago).
        let c = [0.1, 0.37, 0.52, 2.2].into_iter().map(|t| dif(&aplicar(&f, t, &o, 1.0), &o)).fold(0f32, f32::max);
        let m = [1.13, 2.2].into_iter().map(|t| dif(&r, &aplicar(&f, t, &o, 1.0))).fold(0f32, f32::max);
        println!("{}: cambia {c:.3}, se mueve {m:.3}, {ms:.1} ms", d.nombre);
        assert!(c > 0.002, "{} no cambia nada", d.nombre);
        assert!(r.data.iter().flatten().all(|v| v.is_finite() && (0.0..=1.0).contains(v)), "{}", d.nombre);
        // El neón vive en los bordes y esta imagen no tiene.
        if d.id == "bc:neon" {
            continue;
        }
        if d.id.contains("ritmo") || d.ctl.iter().any(|k| matches!(k, toma_uno::vpropios::Ctl::N("vel", ..))) {
            assert!(m > 0.0005, "{} no se mueve", d.nombre);
        }
        for (w, h) in [(1, 1), (3, 2), (33, 400)] {
            aplicar(&f, 0.3, &img(w, h), 0.05);
        }
        // Parejo entre vista previa y exportar: mismo cuadro a la mitad se parece al entero.
        let chico = aplicar(&f, 0.52, &img(160, 90), 0.5);
        let k = chico.data.iter().flatten().sum::<f32>() / chico.data.len() as f32 - r.data.iter().flatten().sum::<f32>() / r.data.len() as f32;
        assert!(k.abs() < 0.25, "{}: {k}", d.nombre);
    }
}
