use std::path::PathBuf;
use toma_uno::archivo::*;
use toma_uno::composicion::fx_nuevo;
use toma_uno::modelo::*;

fn dir(n: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("bc-prueba-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn proyecto(medio: PathBuf) -> Proyecto {
    let mut p = Proyecto::nuevo();
    let mut c = p.nuevo_video(medio, Contenido::Archivo, 0.0, 3.0);
    c.mov.escala = Prop { v: Valor::Num(100.0), claves: vec![(0.0, Valor::Num(1.0)), (2.0, Valor::Num(90.0))], curvas: vec![] };
    c.fx.push(fx_nuevo(9, "ec.color.lumetri").unwrap());
    c.fx.push(fx_nuevo(10, "fc:light_leaks").unwrap());
    c.entrada = Some(Transicion { tipo: "cross_dissolve".into(), dur: 1.0, p: Default::default() });
    p.video.push(c);
    let t = p.nuevo_video("t".into(), Contenido::Titulo(toma_uno::titulos::plantilla(1).1), 0.0, 3.0);
    p.video.push(t);
    let (pista, pat) = p.nueva_pista_instrumento("b", Instrumento::Trio(vec![1.0; 14]), 0.0);
    p.patron(pat).unwrap().notas.push(Nota { tecla: 60, vel: 100, ini: 0.0, dur: 1.0 });
    p.pista(pista).unwrap().auto.push(Automatizacion { destino: Destino::Fx(3, 2), activa: true, puntos: vec![(0.0, 1.0)] });
    p.marcas.push(Marca { t: 1.0, nombre: "Coro ñ".into() });
    p
}

#[test]
fn ida_y_vuelta() {
    let d = dir("ida");
    let p = proyecto("/no/existe.mp4".into());
    let r = d.join("a.bcraft");
    guardar(&p, &r, Some((2, 1, &[1, 2, 3, 4, 5, 6, 7, 8])), false).unwrap();
    let b = std::fs::read(&r).unwrap();
    assert_eq!(&b[..6], b"BCRAFT");
    assert!(serde_json::from_slice::<serde_json::Value>(&b).is_err(), "ya no es texto");
    println!("binario: {} bytes; en JSON serían {}", b.len(), serde_json::to_string_pretty(&p).unwrap().len());
    assert_eq!(abrir(&r, &d).unwrap(), p);
    // Un bit cambiado se detecta; un archivo cortado en cualquier punto no truena.
    let mut m = b.clone();
    let k = 60;
    m[k] ^= 1;
    std::fs::write(&r, &m).unwrap();
    println!("dañado: {:?}", abrir(&r, &d).unwrap_err());
    for n in (0..b.len()).step_by(7) {
        std::fs::write(&r, &b[..n]).unwrap();
        // Si el proyecto llegó entero se recupera; si no, se avisa.
        assert!(abrir(&r, &d).map_or(true, |q| q == p));
    }
    // Los proyectos viejos en JSON se siguen abriendo.
    std::fs::write(&r, serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(abrir(&r, &d).unwrap(), p);
    std::fs::remove_dir_all(d).unwrap();
}

#[test]
fn con_todo_incluido() {
    let d = dir("todo");
    let medio = d.join("cancion.bin");
    let datos: Vec<u8> = (0..3_000_000u32).map(|i| (i * 7 % 251) as u8).collect();
    std::fs::write(&medio, &datos).unwrap();
    let p = proyecto(medio.clone());
    let r = d.join("b.bcraft");
    guardar(&p, &r, None, true).unwrap();
    assert!(std::fs::metadata(&r).unwrap().len() > 3_000_000);
    // Con el medio en su lugar no se saca nada.
    let sal = d.join("medios");
    assert_eq!(abrir(&r, &sal).unwrap(), p);
    assert!(!sal.exists());
    // Sin él, se saca del proyecto y los clips apuntan a la copia.
    std::fs::remove_file(&medio).unwrap();
    let q = abrir(&r, &sal).unwrap();
    assert_eq!(q.video[0].ruta, sal.join("cancion.bin"));
    assert_eq!(std::fs::read(&q.video[0].ruta).unwrap(), datos);
    // Guardar no deja archivos a medias si falta un medio.
    std::fs::remove_file(&q.video[0].ruta).unwrap();
    assert!(guardar(&q, &r, None, true).is_err());
    assert!(r.exists() && !r.with_extension("parcial").exists());
    std::fs::remove_dir_all(d).unwrap();
}

#[test]
fn lee_la_miniatura() {
    let d = std::env::temp_dir().join(format!("bc-mini-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let r = d.join("p.bcraft");
    let q: Vec<u8> = (0..4 * 4 * 3).map(|i| i as u8).collect();
    toma_uno::archivo::guardar(&toma_uno::modelo::Proyecto::nuevo(), &r, Some((4, 3, &q)), false).unwrap();
    assert_eq!(toma_uno::archivo::miniatura(&r), Some((4, 3, q)));
    toma_uno::archivo::guardar(&toma_uno::modelo::Proyecto::nuevo(), &r, None, false).unwrap();
    assert_eq!(toma_uno::archivo::miniatura(&r), None);
    std::fs::remove_dir_all(&d).unwrap();
}
