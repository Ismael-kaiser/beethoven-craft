//! Clips con cristal, etiquetas y línea de volumen. `CAPTURAS=/carpeta` guarda imágenes.
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use toma_uno::modelo::*;
use toma_uno::ui::App;

fn foto(h: &mut Harness<App>, n: &str) {
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/{n}.png")).unwrap();
    }
}

#[test]
fn etiquetas_y_volumen() {
    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    let ruta = std::path::PathBuf::from("/tono.wav");
    let tono: Vec<[f32; 2]> = (0..SR as usize * 8).map(|i| { let x = (i as f32 * 0.03).sin() * 0.4 * (1.0 + (i as f32 / 9000.0).sin()) / 2.0; [x, x] }).collect();
    h.state_mut().almacen.insert(ruta.clone(), std::sync::Arc::new(toma_uno::medios::Audio::nuevo(tono)));
    let clip_audio;
    {
        let p = &mut h.state_mut().p;
        // Video: uno con efectos y animado, uno con transición y velocidad, uno limpio.
        let mut a = p.nuevo_video("Rojo".into(), Contenido::Color([0.7, 0.2, 0.2, 1.0]), 0.0, 3.0);
        a.fx.push(toma_uno::filtros::efecto(a.id, 1).unwrap());
        a.fx.push(toma_uno::filtros::efecto(a.id, 2).unwrap());
        a.animado = true;
        let mut b = p.nuevo_video("Azul".into(), Contenido::Color([0.2, 0.3, 0.7, 1.0]), 3.0, 3.0);
        b.entrada = Some(toma_uno::composicion::transicion_nueva("cross_dissolve"));
        b.mov.velocidad = 1.5;
        let c = p.nuevo_video("Gris".into(), Contenido::Color([0.4, 0.4, 0.4, 1.0]), 6.0, 3.0);
        p.video.extend([a, b, c]);
        let pista = p.nueva_pista("Voz");
        let id = p.id();
        let clip = Clip { id, ruta: ruta.clone(), nombre: "Tono".into(), inicio: 0.0, desde: 0.0, dur: 8.0, total: 8.0, ganancia_db: -6.0, fade_in: 0.5, fade_out: 0.0, invertido: false, grupo: None, velocidad: 1.0, bus: None, color: None };
        p.pistas.iter_mut().find(|x| x.id == pista).unwrap().clips.push(clip);
        clip_audio = id;
    }
    h.state_mut().linea.zoom = 120.0;
    h.state_mut().linea.encuadrar = false;
    h.state_mut().ir(1.4);
    h.run_steps(5);
    foto(&mut h, "clips");
    // Las etiquetas se ven.
    let _ = h.get_by_label("Volumen del clip");
    // Arrastrar la línea de volumen hacia arriba sube la ganancia.
    let r = h.get_by_label("Volumen del clip").rect();
    let o = r.center();
    h.hover_at(o);
    h.run_steps(2);
    h.drag_at(o);
    h.run_steps(1);
    for k in 1..=6 {
        h.hover_at(eframe::egui::pos2(o.x, o.y - 4.0 * k as f32));
        h.run_steps(1);
    }
    foto(&mut h, "clips-volumen");
    h.drop_at(eframe::egui::pos2(o.x, o.y - 24.0));
    h.run_steps(3);
    let g = h.state_mut().p.clip(clip_audio).unwrap().ganancia_db;
    assert!(g > -6.0 + 1.0, "la ganancia debía subir: {g}");
    // Clic derecho: el volumen está a la mano.
    h.get_by_label("Tono").click_secondary();
    h.run_steps(4);
    foto(&mut h, "clips-menu");
    h.get_by_label("+3 dB").click();
    h.run_steps(3);
    assert_eq!(h.state_mut().p.clip(clip_audio).unwrap().ganancia_db, 3.0);
}

/// Cuánto tarda en dibujarse la línea de tiempo con muchos clips (en la ventana de pruebas, sin la tarjeta en pantalla).
#[test]
#[ignore]
fn costo_de_dibujar() {
    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    {
        let p = &mut h.state_mut().p;
        for k in 0..400 {
            let mut a = p.nuevo_video("C".into(), Contenido::Color([0.3, 0.3, 0.6, 1.0]), k as f64 * 0.5, 0.5);
            a.capa = k % 3;
            if k % 2 == 0 {
                a.fx.push(toma_uno::filtros::efecto(a.id, 1).unwrap());
            }
            p.video.push(a);
        }
    }
    h.state_mut().linea.zoom = 30.0;
    h.state_mut().linea.encuadrar = false;
    h.run_steps(5);
    let t = std::time::Instant::now();
    for _ in 0..60 {
        h.state_mut().linea.zoom += 0.01;
        h.run_steps(1);
    }
    println!("por cuadro: {:.2} ms", t.elapsed().as_secs_f64() * 1000.0 / 60.0);
}

/// Cuántas figuras y cuánto tiempo cuesta dibujar la pestaña de audio con el mezclador (mucho metal).
#[test]
#[ignore]
fn costo_del_metal() {
    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    for k in 0..8 {
        let p = &mut h.state_mut().p;
        let id = p.nueva_pista(&format!("Pista {k}"));
        let _ = id;
    }
    for _ in 0..3 {
        h.run_steps(2);
    }
    // Vértices que salen de dibujar un cuadro: lo que carga la tarjeta.
    let vertices = |h: &Harness<App>| {
        let o = h.output();
        let prims = h.ctx.tessellate(o.shapes.clone(), o.pixels_per_point);
        prims.iter().map(|p| match &p.primitive { eframe::egui::epaint::Primitive::Mesh(m) => m.vertices.len(), _ => 0 }).sum::<usize>()
    };
    let t = std::time::Instant::now();
    for k in 0..60 {
        h.state_mut().linea.zoom = 60.0 + k as f64 * 0.01;
        h.run_steps(1);
    }
    let ms = t.elapsed().as_secs_f64() * 1000.0 / 60.0;
    let t = std::time::Instant::now();
    let mut v = 0;
    for _ in 0..20 {
        v = vertices(&h);
    }
    let o = h.output();
    let dibujos = h.ctx.tessellate(o.shapes.clone(), o.pixels_per_point).len();
    println!("por cuadro: {ms:.2} ms; vértices: {v}; dibujos: {dibujos}; teselar: {:.2} ms", t.elapsed().as_secs_f64() * 1000.0 / 20.0);
}

/// Estirar la orilla del recuadro del minimapa acerca o aleja la línea de tiempo.
#[test]
fn minimapa_con_orillas() {
    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    {
        let p = &mut h.state_mut().p;
        let c = p.nuevo_video("C".into(), Contenido::Color([0.3, 0.3, 0.6, 1.0]), 0.0, 60.0);
        p.video.push(c);
    }
    h.state_mut().linea.encuadrar = true;
    h.run_steps(4);
    let z0 = h.state().linea.zoom;
    let o = h.get_by_label("Orilla derecha: acercar o alejar").rect().center();
    h.hover_at(o);
    h.run_steps(2);
    h.drag_at(o);
    h.run_steps(2);
    for k in 1..=8 {
        h.hover_at(o - eframe::egui::vec2(30.0 * k as f32, 0.0));
        h.run_steps(1);
    }
    h.drop_at(o - eframe::egui::vec2(240.0, 0.0));
    h.run_steps(3);
    let z1 = h.state().linea.zoom;
    assert!(z1 > z0 * 1.2, "debía acercarse: {z0} → {z1}");
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/minimapa.png")).unwrap();
    }
}
