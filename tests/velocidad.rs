//! La velocidad desde el clic derecho de un clip: en porcentaje, −50 % es cámara lenta a la mitad.
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use toma_uno::modelo::*;
use toma_uno::ui::App;

#[test]
fn una_imagen_mas_lenta_dura_mas() {
    let mut p = Proyecto::nuevo();
    let c = p.nuevo_video("".into(), Contenido::Color([0.5, 0.2, 0.2, 1.0]), 0.0, 4.0);
    let id = c.id;
    p.video.push(c);
    p.velocidad(id, 0.5);
    let c = p.video.iter().find(|c| c.id == id).unwrap();
    assert_eq!((c.dur, c.mov.velocidad), (8.0, 0.5));
    p.velocidad(id, 2.0);
    let c = p.video.iter().find(|c| c.id == id).unwrap();
    assert_eq!((c.dur, c.mov.velocidad), (2.0, 2.0));
}

#[test]
fn desde_el_clic_derecho() {
    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    h.state_mut().inicio.visible = false;
    let id = {
        let p = &mut h.state_mut().p;
        let c = p.nuevo_video("".into(), Contenido::Color([0.2, 0.5, 0.8, 1.0]), 0.0, 4.0);
        let id = c.id;
        p.video.push(c);
        id
    };
    h.state_mut().linea.encuadrar = true;
    h.run_steps(4);
    h.get_by_label("Clip de video 0").click_secondary();
    h.run_steps(4);
    h.get_by(|n| n.label().is_some_and(|l| l.starts_with("Velocidad"))).click();
    h.run_steps(4);
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/velocidad-menu.png")).unwrap();
    }
    h.get_by(|n| n.label().is_some_and(|l| l.contains("50 %") && l.starts_with('−'))).click();
    h.run_steps(3);
    let c = h.state().p.video.iter().find(|c| c.id == id).unwrap();
    assert_eq!((c.mov.velocidad, c.dur), (0.5, 8.0), "cámara lenta a la mitad: dura el doble");
}

#[test]
fn solo_el_clip_o_tambien_lo_vinculado() {
    let mut p = Proyecto::nuevo();
    p.pistas.clear();
    let pista = p.nueva_pista("a");
    let mut v = p.nuevo_video("/x.mp4".into(), Contenido::Archivo, 0.0, 4.0);
    let g = p.id();
    v.grupo = Some(g);
    let vid = v.id;
    p.video.push(v);
    let aid = p.id();
    p.pista(pista).unwrap().clips.push(Clip {
        id: aid, ruta: "/x.mp4".into(), nombre: "x".into(), inicio: 0.0, desde: 0.0, dur: 4.0, total: 4.0,
        ganancia_db: 0.0, fade_in: 0.0, fade_out: 0.0, invertido: false, grupo: Some(g), velocidad: 1.0, bus: None, color: None,
    });
    // Solo el video: el sonido se queda como estaba.
    p.velocidad_con(vid, 0.5, false);
    assert_eq!(p.video[0].mov.velocidad, 0.5);
    assert_eq!(p.clip(aid).unwrap().velocidad, 1.0);
    // Con su audio: los dos.
    p.velocidad_con(vid, 2.0, true);
    assert_eq!((p.video[0].mov.velocidad, p.clip(aid).unwrap().velocidad), (2.0, 2.0));
    // Desde el audio, solo el audio.
    p.velocidad_con(aid, 0.25, false);
    assert_eq!((p.video[0].mov.velocidad, p.clip(aid).unwrap().velocidad), (2.0, 0.25));
}
