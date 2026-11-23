//! Cuántos cuadros por segundo se exportan en varios casos. `VIDEO=… cargo test --release --test velocidad -- --ignored --nocapture`
use std::sync::atomic::{AtomicBool, AtomicU32};
use toma_uno::composicion::fx_nuevo;
use toma_uno::exportar::*;
use toma_uno::modelo::*;

fn base(seg: f64) -> Proyecto {
    let v = std::path::PathBuf::from(std::env::var("VIDEO").unwrap());
    let i = toma_uno::medios::sondear(&v).unwrap();
    let (an, al, fps) = i.video.unwrap();
    let mut p = Proyecto::nuevo();
    (p.ancho, p.alto, p.fps) = (an, al, fps);
    let mut c = p.nuevo_video(v, Contenido::Archivo, 0.0, seg);
    (c.ancho, c.alto, c.fps, c.total, c.desde) = (an, al, fps, i.dur, 60.0);
    p.video = vec![c];
    p
}

#[test]
#[ignore]
fn velocidad() {
    let seg = 12.0;
    let capa = |p: &mut Proyecto| p.capas.push(Capa { nombre: "V2".into(), visible: true, bloqueada: false });
    let titulo = |p: &mut Proyecto| {
        capa(p);
        let mut t = p.nuevo_video("t".into(), Contenido::Titulo(toma_uno::titulos::plantilla(1).1), 0.0, seg);
        t.capa = 1;
        p.video.push(t);
    };
    let mut casos: Vec<(&str, Proyecto)> = vec![("solo video", base(seg))];
    let mut p = base(seg);
    titulo(&mut p);
    casos.push(("con título", p));
    let mut p = base(seg);
    p.video[0].fx.push(toma_uno::filtros::efecto(50, 1).unwrap());
    casos.push(("con filtro", p));
    let mut p = base(seg);
    p.video[0].mov.escala = Prop { claves: vec![(0.0, Valor::Num(100.0)), (12.0, Valor::Num(120.0))], ..Prop::num(100.0) };
    casos.push(("acercamiento", p));
    let mut p = base(seg);
    p.video[0].fx.push(fx_nuevo(50, "ec.blur.gaussian").unwrap());
    titulo(&mut p);
    casos.push(("desenfoque y título", p));
    let mut p = base(seg);
    capa(&mut p);
    let mut a = p.nuevo_video("Ajuste".into(), Contenido::Ajuste, 0.0, seg);
    a.capa = 1;
    a.fx.push(toma_uno::filtros::efecto(51, 1).unwrap());
    p.video.push(a);
    casos.push(("clip vacío con filtro", p));
    for (n, p) in casos {
        for c in [Codec::H264, Codec::H265] {
            let s = std::env::temp_dir().join(format!("velocidad-{}.mp4", std::process::id()));
            let t = std::time::Instant::now();
            exportar_con(&p, &Default::default(), &s, Modo::Video, &Opciones { codec: c, ..Default::default() }, &AtomicU32::new(0), &AtomicBool::new(false)).unwrap();
            let d = t.elapsed().as_secs_f64();
            println!("{n} {c:?}: {d:.1} s, {:.0} cps, {:.2}x tiempo real, {:.1} MB", seg * p.fps / d, seg / d, std::fs::metadata(&s).unwrap().len() as f64 / 1e6);
            let _ = std::fs::remove_file(s);
        }
    }
}
