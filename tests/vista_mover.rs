//! En la vista previa: un texto nuevo con una letra, escribirlo ahí, elegir con clic, estirar y girar.
//! `CAPTURAS=/carpeta` guarda imágenes.
use eframe::egui::{Event, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use toma_uno::modelo::*;
use toma_uno::ui::App;

fn foto(h: &mut Harness<App>, n: &str) {
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/{n}.png")).unwrap();
    }
}

fn jalar(h: &mut Harness<App>, a: eframe::egui::Pos2, b: eframe::egui::Pos2) {
    h.hover_at(a);
    h.run_steps(2);
    h.drag_at(a);
    h.run_steps(2);
    for k in 1..=8 {
        h.hover_at(a + (b - a) * (k as f32 / 8.0));
        h.run_steps(1);
    }
    h.drop_at(b);
    h.run_steps(3);
}

fn titulo<'a>(h: &'a Harness<App>, id: u64) -> &'a ClipVideo {
    h.state().p.video.iter().find(|c| c.id == id).unwrap()
}

#[test]
fn mover_escribir_estirar() {
    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    // Un fondo para que haya imagen.
    {
        let p = &mut h.state_mut().p;
        let c = p.nuevo_video("Fondo".into(), Contenido::Color([0.15, 0.2, 0.35, 1.0]), 0.0, 6.0);
        p.video.push(c);
    }
    h.run_steps(4);
    let d = h.get_by_label("Imagen").rect();
    // Un texto con Barlow a la izquierda de arriba (como si se soltara la letra ahí).
    h.state_mut().titulo_con_letra("Barlow", 0.0, None, Some((-20.0, -20.0)));
    h.run_steps(4);
    let a = h.state().clip.unwrap();
    assert!(h.state().escribiendo.is_some());
    // Se escribe encima: el texto de ejemplo va elegido y lo de ahora lo reemplaza.
    h.event(Event::Text("Hola".into()));
    h.run_steps(4);
    foto(&mut h, "vista-escribiendo");
    match &titulo(&h, a).contenido {
        Contenido::Titulo(t) => {
            assert_eq!(t.texto, "Hola");
            assert_eq!(t.letra, "Barlow");
        }
        _ => panic!(),
    }
    h.event(Event::Key { key: eframe::egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() });
    h.run_steps(3);
    assert!(h.state().escribiendo.is_none());
    // Otro texto abajo a la derecha; luego clic en el primero lo elige.
    h.state_mut().titulo_con_letra("Cuprum", 0.0, None, Some((20.0, 20.0)));
    h.state_mut().escribiendo = None;
    h.run_steps(4);
    let b = h.state().clip.unwrap();
    assert_ne!(a, b);
    let centro_a = d.center() + vec2(-0.2 * d.width(), -0.2 * d.height());
    h.hover_at(centro_a);
    h.run_steps(2);
    h.drag_at(centro_a);
    h.run_steps(1);
    h.drop_at(centro_a);
    h.run_steps(3);
    assert_eq!(h.state().clip, Some(a), "el clic debía elegir el primer texto");
    foto(&mut h, "vista-elegido");
    // Estirar desde el lado derecho del recuadro: crece a lo ancho, no a lo alto.
    let ancho0 = titulo(&h, a).mov.ancho.en(0.0).num();
    let alto0 = titulo(&h, a).mov.alto.en(0.0).num();
    // El asa del lado derecho.
    let lado = h.state().asas_en_vista(d)[5];
    jalar(&mut h, lado, lado + vec2(60.0, 0.0));
    let t = titulo(&h, a);
    assert!(t.mov.ancho.en(0.0).num() > ancho0 + 5.0, "debía estirarse a lo ancho");
    assert!((t.mov.alto.en(0.0).num() - alto0).abs() < 0.01);
    // Girar desde el asa de arriba.
    let giro = h.state().asas_en_vista(d)[8];
    jalar(&mut h, giro, giro + vec2(80.0, 40.0));
    assert!(titulo(&h, a).mov.giro.en(0.0).num().abs() > 5.0, "debía girar");
    // La vista previa se compone en segundo plano: se le da tiempo.
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(25));
        h.run_steps(1);
    }
    foto(&mut h, "vista-girado");
}
