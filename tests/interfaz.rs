//! La ventana sin ventana: dibuja la app en memoria y la maneja por etiquetas.
//! `CAPTURAS=/carpeta` guarda imágenes.
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use eframe::egui::accesskit::Role;
use toma_uno::ui::App;

fn arnes<'a>() -> Harness<'a, App> {
    let mut h = Harness::builder()
        .with_size([1440.0, 860.0])
        .wgpu()
        .build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    h
}

/// La pestaña de arriba (hay otros botones con el mismo nombre más abajo).
fn pestana(h: &mut Harness<App>, n: &str) {
    let r = h.get_all_by_label(n).map(|x| x.rect()).min_by(|a, b| a.top().total_cmp(&b.top())).unwrap();
    clic(h, r.left() + 40.0, r.center().y);
}

/// Al soltar un video con sonido se pregunta qué hacer con el audio: «Todo junto». Si no se pregunta (imagen, sonido
/// solo, varios archivos), no hace nada.
fn todo_junto(h: &mut Harness<App>) {
    for _ in 0..60 {
        h.run_steps(1);
        if h.state().preguntar.is_some() || h.state().convertir.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    h.run_steps(2);
    if h.state().convertir.is_some() {
        h.get_by_label("Usar el original").click();
        h.run_steps(3);
    }
    if h.state().preguntar.is_some() {
        h.get_by_label("Todo junto").click();
        h.run_steps(3);
    }
}

fn foto(h: &mut Harness<App>, n: &str) {
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/{n}.png")).unwrap();
    }
}

#[test]
fn arranca_y_responde() {
    let mut h = arnes();
    foto(&mut h, "vacia");
    pestana(&mut h, "Video");
    h.run_steps(3);
    let n = h.state().p.pistas.len();
    h.get_by_label("+  Agregar pista").click();
    h.run_steps(3);
    foto(&mut h, "agregar-pista");
    h.get_by_label("Pista de grabación (tu voz)").click();
    h.run_steps(3);
    assert_eq!(h.state().p.pistas.len(), n + 1);
    let p = h.state().p.pistas.last().unwrap();
    assert!(p.armada && p.fx.len() == 2, "la pista de grabación trae ecualizador y compresor");
    h.get_by_label("Deshacer (⌘Z)").click();
    h.run_steps(3);
    assert_eq!(h.state().p.pistas.len(), n);
    for s in ["Efectos de video", "Transiciones", "Títulos", "Efectos de audio"] {
        h.get_by_label(s).click();
        h.run_steps(3);
        foto(&mut h, s);
    }
    for v in ["Audio", "Playlist", "Efectos", "Video"] {
        pestana(&mut h, v);
        h.run_steps(4);
        assert_eq!(format!("{:?}", h.state().espacio), v);
        foto(&mut h, &format!("pestana-{v}"));
    }
    for p in ["Origen", "Color", "Controles"] {
        h.get_by_label(p).click();
        h.run_steps(3);
        foto(&mut h, &format!("panel-{p}"));
    }
}

#[test]
#[ignore]
fn con_video_real() {
    use toma_uno::composicion::fx_nuevo;
    use toma_uno::modelo::*;
    let v = std::path::PathBuf::from(std::env::var("VIDEO").unwrap());
    let mut h = arnes();
    h.state_mut().importar(v);
    for _ in 0..200 {
        h.run_steps(1);
        if h.state().preguntar.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    h.run_steps(5);
    h.get_by_label("Todo junto").click();
    for _ in 0..600 {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(20));
        if !h.state().ocupada() && h.state().p.pistas.len() == 2 {
            break;
        }
    }
    h.state_mut().ir(40.0);
    h.state_mut().poner_titulo(1, 35.0, None);
    let id = h.state().p.video[0].id;
    let a = &mut h.state_mut().p;
    a.dividir(id, 60.0);
    a.video[0].mov.escala = Prop { v: Valor::Num(100.0), claves: vec![(10.0, Valor::Num(100.0)), (40.0, Valor::Num(80.0))], curvas: vec![], libre: false };
    let f = fx_nuevo(77, "ec.color.huesaturation").unwrap();
    a.video[0].fx.push(f);
    let f = fx_nuevo(78, "ec.stylize.glow").unwrap();
    a.video[0].fx.push(f);
    a.video[1].entrada = Some(Transicion { tipo: "cross_zoom".into(), dur: 3.0, p: Default::default() });
    h.state_mut().clip = Some(id);
    for _ in 0..150 {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    foto(&mut h, "playlist");
    pestana(&mut h, "Efectos");
    for _ in 0..150 {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    foto(&mut h, "efectos");
    pestana(&mut h, "Video");
    h.run_steps(3);
    h.get_by_label("Color").click();
    for _ in 0..20 {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    foto(&mut h, "color");
    let ruta = h.state().p.video[0].ruta.clone();
    h.state_mut().abrir_origen(&ruta);
    for _ in 0..60 {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    foto(&mut h, "origen");
    // Insertar 2 s del origen en el cursor empuja lo demás, con su sonido.
    let antes = h.state().p.fin();
    h.state_mut().poner_medio(&ruta, 50.0, 0, Some((10.0, 12.0)), true);
    h.run_steps(3);
    println!("insertar 2 s: {antes:.2} -> {:.2}", h.state().p.fin());
    assert!((h.state().p.fin() - antes - 2.0).abs() < 0.01);
    assert!(h.state().p.video.iter().filter(|c| c.capa == 0).count() == 4);
    foto(&mut h, "insertado");
    // Velocidad: la imagen y el sonido vinculados, y su audio estirado se prepara solo.
    let id = h.state().p.video.iter().find(|c| (c.inicio - 50.0).abs() < 0.01 && c.capa == 0).unwrap().id;
    h.state_mut().p.velocidad(id, 2.0);
    let t = std::time::Instant::now();
    loop {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(30));
        let s = h.state();
        let c = s.p.pistas.iter().flat_map(|p| &p.clips).find(|c| c.velocidad == 2.0).expect("el sonido cambió de velocidad");
        if s.almacen.contains_key(&c.clave()) || t.elapsed().as_secs() > 60 {
            let a = &s.almacen[&c.clave()];
            println!("audio a 2x listo en {:.1} s; dura {:.2} s de {:.2}", t.elapsed().as_secs_f32(), a.dur(), c.total);
            assert!((a.dur() - c.total / 2.0).abs() < 0.3);
            break;
        }
    }
    h.state_mut().poner_subtitulos("1\n00:00:02,000 --> 00:00:04,000\nHola\n");
    h.run_steps(2);
    assert!(h.state().p.capas.last().unwrap().nombre == "Subtítulos");
}

#[test]
#[ignore]
fn arrastrar_recortar_y_separar() {
    use eframe::egui::pos2;
    let v = std::path::PathBuf::from(std::env::var("VIDEO").unwrap());
    let mut h = arnes();
    let info = toma_uno::medios::sondear(&v).unwrap();
    h.state_mut().importar_video(v, info, toma_uno::ui::AudioVideo::Separado);
    h.state_mut().linea.iman = false;
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 900 {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(50));
        if !h.state().ocupada() {
            break;
        }
    }
    h.run_steps(5);
    let nombres: Vec<String> = h.state().p.pistas.iter().map(|p| p.nombre.clone()).collect();
    println!("separado en {:.0} s: {nombres:?}", t.elapsed().as_secs_f32());
    assert!(nombres.contains(&"Instrumental".into()) && nombres.contains(&"Voz original".into()));
    for p in &h.state().p.pistas[1..] {
        println!("{}: {:.2} s de {:.2}", p.nombre, p.clips[0].dur, h.state().p.video[0].dur);
        assert!((p.clips[0].dur - h.state().p.video[0].dur).abs() < 0.1);
    }
    foto(&mut h, "separado");
    // Arrastrar el clip de la segunda fila de audio 80 px a la derecha.
    // Imagen, voz e instrumental quedan vinculados: se mueven juntos.
    let vid = h.state().p.video[0].id;
    println!("vinculados: {}", h.state().p.vinculados(vid).len());
    assert_eq!(h.state().p.vinculados(vid).len(), 3);
    let antes = h.state().p.pistas[1].clips[0].clone();
    let zoom = h.state().linea.zoom;
    let y = {
        // Fila de la segunda pista de audio: video + una pista por encima.
        // La cabecera de la primera pista de audio dice dónde empieza; la segunda va 62 px abajo.
        let nombre = h.get_all_by_value("Instrumental").next().unwrap().rect().top();
        nombre + 40.0
    };
    let x = 500.0;
    let arrastra = |h: &mut Harness<App>, a: f32, b: f32| {
        h.hover_at(pos2(a, y));
        h.run_steps(2);
        h.drag_at(pos2(a, y));
        h.run_steps(2);
        for k in 1..=8 {
            h.hover_at(pos2(a + (b - a) * k as f32 / 8.0, y));
            h.run_steps(1);
        }
        h.drop_at(pos2(b, y));
        h.run_steps(3);
    };
    arrastra(&mut h, x, x + 80.0);
    let c = h.state().p.pistas[1].clips[0].clone();
    println!("mover 80 px = {:.2} s: inicio {:.2} -> {:.2}", 80.0 / zoom, antes.inicio, c.inicio);
    assert!((c.inicio - antes.inicio - 80.0 / zoom).abs() < 10.0 / zoom && c.dur == antes.dur);
    assert!((h.state().p.video[0].inicio - c.inicio).abs() < 1e-6, "el video se movió con su sonido");
    // Recortar la orilla izquierda.
    let x = toma_uno::ui::linea::CAB + 38.0 + (c.inicio * zoom) as f32 + 3.0;
    arrastra(&mut h, x, x + 60.0);
    let d = h.state().p.pistas[1].clips[0].clone();
    println!("recorte: desde {:.2} -> {:.2}, dur {:.2} -> {:.2}", c.desde, d.desde, c.dur, d.dur);
    assert!(d.desde > 1.0 && (d.desde + d.dur - c.dur).abs() < 0.01 && (d.inicio - c.inicio - d.desde).abs() < 0.01);
    h.get_by_label("Deshacer (⌘Z)").click();
    h.run_steps(3);
    assert_eq!(h.state().p.pistas[1].clips[0].desde, 0.0);
}

#[test]
fn rack_y_piano() {
    use toma_uno::modelo::Instrumento;
    use toma_uno::ui::Pestana;
    let mut h = arnes();
    h.state_mut().agregar_instrumento(Instrumento::Bateria, "Batería");
    h.run_steps(5);
    assert!(h.state().pestana == Pestana::Rack && h.state().espacio == toma_uno::ui::Espacio::Audio);
    for p in [1, 5, 9, 13] {
        h.get_by_label(&format!("Bombo paso {p}")).click();
        h.run_steps(2);
    }
    h.get_by_label("Tarola paso 5").click();
    h.run_steps(2);
    assert_eq!(h.state().p.patrones[0].notas.len(), 5);
    foto(&mut h, "rack");
    h.get_by_label("Tarola paso 5").click();
    h.run_steps(2);
    assert_eq!(h.state().p.patrones[0].notas.len(), 4);
    h.state_mut().agregar_instrumento(Instrumento::Trio(toma_uno::instrumentos::SONIDOS[1].1.to_vec()), "Piano");
    h.run_steps(6);
    assert!(h.state().pestana == Pestana::Piano);
    // Clic en la rejilla pone una nota; clic derecho la quita.
    let donde = eframe::egui::pos2(300.0, 300.0);
    h.hover_at(donde);
    h.run_steps(2);
    h.drag_at(donde);
    h.run_steps(1);
    h.drop_at(donde);
    h.run_steps(4);
    println!("notas: {:?}", h.state().p.patrones[1].notas);
    assert_eq!(h.state().p.patrones[1].notas.len(), 1);
    foto(&mut h, "piano");
}

#[test]
fn automatizar_en_pantalla() {
    use toma_uno::modelo::*;
    let mut h = arnes();
    // El proyecto nuevo viene sin pistas.
    let id = h.state_mut().p.nueva_pista_grabacion();
    h.state_mut().p.pistas[0].auto.push(Automatizacion { destino: Destino::Volumen, activa: true, puntos: vec![(2.0, 0.0), (6.0, -20.0)] });
    h.state_mut().linea.auto_ver.insert(id, 0);
    h.run_steps(4);
    foto(&mut h, "automatizar");
    assert_eq!(h.state().p.pistas[0].auto[0].puntos.len(), 2);
}

/// Tres clips de título pegados en la pista de video, 100 px por segundo.
fn con_clips<'a>() -> (Harness<'a, App>, Vec<u64>) {
    use toma_uno::modelo::*;
    let mut h = arnes();
    let mut ids = vec![];
    for k in 0..3 {
        let p = &mut h.state_mut().p;
        let mut c = p.nuevo_video("/v.mp4".into(), Contenido::Archivo, 1.0 + k as f64 * 2.0, 2.0);
        (c.desde, c.total) = (1.0, 4.0);
        ids.push(c.id);
        p.video.push(c);
    }
    h.state_mut().linea.zoom = 100.0;
    h.state_mut().linea.iman = false;
    h.state_mut().linea.encuadrar = false;
    h.run_steps(4);
    (h, ids)
}

fn fila(h: &Harness<App>) -> (f32, f32) {
    let v1 = h.get_all_by_value("V1").next().unwrap().rect();
    (toma_uno::ui::linea::CAB + 38.0, v1.center().y + 14.0)
}

fn clic(h: &mut Harness<App>, x: f32, y: f32) {
    let o = eframe::egui::pos2(x, y);
    h.hover_at(o);
    h.run_steps(2);
    h.drag_at(o);
    h.run_steps(1);
    h.drop_at(o);
    h.run_steps(3);
}

fn jalar(h: &mut Harness<App>, x: f32, y: f32, d: f32) {
    use eframe::egui::pos2;
    h.hover_at(pos2(x, y));
    h.run_steps(2);
    h.drag_at(pos2(x, y));
    h.run_steps(2);
    for k in 1..=8 {
        h.hover_at(pos2(x + d * k as f32 / 8.0, y));
        h.run_steps(1);
    }
    h.drop_at(pos2(x + d, y));
    h.run_steps(3);
}

fn medidas(h: &Harness<App>) -> Vec<(f64, f64, f64)> {
    let r = |x: f64| (x * 100.0).round() / 100.0;
    let mut v: Vec<_> = h.state().p.video.iter().map(|c| (r(c.inicio), r(c.dur), r(c.desde))).collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    v
}

#[test]
fn herramientas_en_pantalla() {
    use toma_uno::ui::linea::Herramienta::*;
    let cerca = |a: &[(f64, f64, f64)], b: &[(f64, f64, f64)]| {
        assert_eq!(a.len(), b.len(), "{a:?} contra {b:?}");
        for (x, y) in a.iter().zip(b) {
            assert!((x.0 - y.0).abs() < 0.06 && (x.1 - y.1).abs() < 0.06 && (x.2 - y.2).abs() < 0.06, "{a:?} contra {b:?}");
        }
    };
    // Seleccionar: solo queda elegido el último.
    let (mut h, ids) = con_clips();
    let (x0, y) = fila(&h);
    clic(&mut h, x0 + 200.0, y);
    clic(&mut h, x0 + 400.0, y);
    assert_eq!(h.state().seleccion.iter().copied().collect::<Vec<_>>(), vec![ids[1]]);
    // Cuchilla: parte donde se hace clic y no deja nada elegido de antes.
    h.state_mut().linea.herramienta = Cuchilla;
    clic(&mut h, x0 + 350.0, y);
    cerca(&medidas(&h), &[(1.0, 2.0, 1.0), (3.0, 0.5, 1.0), (3.5, 1.5, 1.5), (5.0, 2.0, 1.0)]);
    println!("elegidos tras la cuchilla: {:?}", h.state().seleccion);
    assert!(h.state().seleccion.len() <= 1);
    // Borrar quita solo lo elegido.
    h.state_mut().linea.herramienta = Flecha;
    clic(&mut h, x0 + 600.0, y);
    h.key_press(eframe::egui::Key::Delete);
    h.run_steps(3);
    assert_eq!(h.state().p.video.len(), 3);
    for (t, x, d, espera) in [
        (Flecha, 400.0, 30.0, vec![(1.0, 2.0, 1.0), (3.3, 2.0, 1.0), (5.3, 1.7, 1.3)]),
        (Flecha, 298.0, -50.0, vec![(1.0, 1.5, 1.0), (3.0, 2.0, 1.0), (5.0, 2.0, 1.0)]),
        (Ondulado, 297.0, -50.0, vec![(1.0, 1.5, 1.0), (2.5, 2.0, 1.0), (4.5, 2.0, 1.0)]),
        (Rodar, 297.0, 50.0, vec![(1.0, 2.5, 1.0), (3.5, 1.5, 1.5), (5.0, 2.0, 1.0)]),
        (Deslizar, 400.0, -50.0, vec![(1.0, 2.0, 1.0), (3.0, 2.0, 1.5), (5.0, 2.0, 1.0)]),
        (Correr, 400.0, 50.0, vec![(1.0, 2.5, 1.0), (3.5, 2.0, 1.0), (5.5, 1.5, 1.5)]),
        (Adelante, 400.0, 100.0, vec![(1.0, 2.0, 1.0), (4.0, 2.0, 1.0), (6.0, 2.0, 1.0)]),
        (Estirar, 697.0, 100.0, vec![(1.0, 2.0, 1.0), (3.0, 2.0, 1.0), (5.0, 3.0, 1.0)]),
    ] {
        let (mut h, _) = con_clips();
        let (x0, y) = fila(&h);
        h.state_mut().linea.herramienta = t;
        jalar(&mut h, x0 + x, y, d);
        println!("{t:?}: {:?}", medidas(&h));
        cerca(&medidas(&h), &espera);
    }
}

/// Lo de la captura del usuario: dos clips marcados y la cuchilla a la mano.
#[test]
fn una_sola_seleccion() {
    use toma_uno::ui::linea::Herramienta;
    let (mut h, ids) = con_clips();
    // Poner un efecto en un clip con otro elegido deja marcado solo al del efecto.
    h.state_mut().clip = Some(ids[0]);
    h.state_mut().seleccion = [ids[0]].into();
    h.run_steps(2);
    h.state_mut().agregar_fx_video(ids[1], "ec.blur.gaussian");
    h.run_steps(2);
    assert_eq!(h.state().clip, Some(ids[1]));
    assert_eq!(h.state().seleccion, [ids[1]].into());
    // La cuchilla no deja nada marcado.
    let (x0, y) = fila(&h);
    let m = medidas(&h)[0];
    h.state_mut().linea.herramienta = Herramienta::Cuchilla;
    let x = x0 - 38.0 + ((m.0 + m.1 * 0.4) * h.state().linea.zoom) as f32;
    clic(&mut h, x, y);
    h.run_steps(3);
    assert!(h.state().clip.is_none() && h.state().seleccion.is_empty());
    // Un clip que ya no existe no se queda elegido.
    h.state_mut().clip = Some(999_999);
    h.run_steps(2);
    assert!(h.state().clip.is_none() && h.state().seleccion.is_empty());
}

#[test]
fn pestanas_y_catalogo() {
    use toma_uno::ui::Espacio;
    let (mut h, ids) = con_clips();
    pestana(&mut h, "Efectos");
    for _ in 0..1500 {
        h.run_steps(1);
        if h.state().muestras_listas() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(h.state().muestras_listas(), "{} muestras", h.state().muestras.len());
    for s in ["Estilos listos", "Filtros", "Video", "Transiciones", "Audio", "Títulos"] {
        // Hay pestañas con el mismo nombre: la pastilla del catálogo es la última.
        h.get_all_by_label(s).last().unwrap().click();
        h.run_steps(4);
        foto(&mut h, &format!("catalogo-{s}"));
    }
    // El catálogo no debe ensancharse solo de un cuadro a otro.
    let ancho = |h: &Harness<App>| h.get_all_by_role(Role::TextInput).map(|x| x.rect().width()).fold(0.0, f32::max);
    let antes = ancho(&h);
    h.run_steps(20);
    assert_eq!(antes, ancho(&h), "el panel del catálogo crece solo");
    h.state_mut().clip = Some(ids[0]);
    h.state_mut().poner_estilo(Some(ids[0]), 0);
    h.run_steps(3);
    assert!(h.state().p.video.iter().find(|c| c.id == ids[0]).unwrap().fx.len() >= 4);
    foto(&mut h, "estilo-puesto");
    // Cerrar y volver a abrir
    h.get_by_label("Cerrar Efectos").click();
    h.run_steps(3);
    assert!(!h.state().pestanas.contains(&Espacio::Efectos) && h.state().espacio != Espacio::Efectos);
    // El de la tira de pestañas (la consola tiene otro «+» más abajo).
    let r = h.get_all_by_label("+").map(|x| x.rect()).min_by(|a, b| a.top().total_cmp(&b.top())).unwrap();
    clic(&mut h, r.center().x, r.center().y);
    h.run_steps(3);
    h.get_by_role_and_label(Role::Button, "Efectos").click();
    h.run_steps(3);
    assert_eq!(h.state().espacio, Espacio::Efectos);
}

/// El proyecto se guarda en binario y vuelve igual.
#[test]
fn guarda_en_binario() {
    let (mut h, _) = con_clips();
    let r = std::env::temp_dir().join(format!("prueba-{}.bcraft", std::process::id()));
    h.state_mut().guardar_en(r.clone());
    let b = std::fs::read(&r).unwrap();
    assert_eq!(&b[..6], b"BCRAFT");
    let antes = h.state().p.clone();
    h.state_mut().p = toma_uno::modelo::Proyecto::nuevo();
    h.state_mut().abrir(r.clone());
    h.run_steps(2);
    assert!(h.state().p == antes);
    let _ = std::fs::remove_file(r);
}

/// El piano roll sigue al instrumento elegido y se puede cambiar su sonido.
#[test]
fn elegir_instrumento() {
    use toma_uno::instrumentos::SONIDOS;
    use toma_uno::modelo::Instrumento;
    let mut h = arnes();
    let trio = |n: &str| Instrumento::Trio(SONIDOS.iter().find(|s| s.0 == n).unwrap().1.to_vec());
    h.state_mut().agregar_instrumento(trio("Piano suave"), "Piano suave");
    h.state_mut().agregar_instrumento(trio("Bajo"), "Bajo");
    h.run_steps(5);
    let pats: Vec<u64> = h.state().p.patrones.iter().map(|p| p.id).collect();
    assert_eq!(h.state().musica.patron, Some(pats[1]));
    h.get_by_label("Tocar Piano suave").click();
    h.run_steps(4);
    assert_eq!(h.state().musica.patron, Some(pats[0]));
    h.get_by_label("Tocar Bajo").click();
    h.run_steps(4);
    assert_eq!(h.state().musica.patron, Some(pats[1]));
    foto(&mut h, "instrumentos");
    h.get_by_label("Bajo ▾").click();
    h.run_steps(3);
    h.get_by_label("Vientos ⏵").hover();
    h.run_steps(8);
    h.get_by_label("Vientos ⏵").click();
    h.run_steps(4);
    foto(&mut h, "menu-sonidos");
    h.get_by_label("Flauta").click();
    h.run_steps(4);
    let t = h.state().p.pistas.last().unwrap();
    assert_eq!(t.instrumento, Some(trio("Flauta")));
    assert_eq!(t.nombre, "Flauta");
}

#[test]
fn menu_principal() {
    let mut h = arnes();
    h.state_mut().inicio.mostrar();
    h.run_steps(3);
    foto(&mut h, "menu-principal");
    h.get_by_label("Abrir proyecto…");
    h.get_by_label("Nuevo video").click();
    h.run_steps(3);
    assert!(h.state().inicio.cargando.is_some(), "primero sale la pantalla de carga");
    foto(&mut h, "pantalla-carga");
    for _ in 0..100 {
        if !h.state().inicio.visible {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        h.run_steps(1);
    }
    assert!(!h.state().inicio.visible, "después de cargar entra al editor");
    h.run_steps(3);
    assert_eq!(format!("{:?}", h.state().espacio), "Video");
    h.get_by_label("ARCHIVO").click();
    h.run_steps(2);
    h.get_by_label("Menú principal").click();
    h.run_steps(3);
    assert!(h.state().inicio.visible, "ARCHIVO › Menú principal regresa al menú");
    foto(&mut h, "menu-principal-volver");
}

/// Un título va a la pestaña de animación, se le pone una animación lista y su posición cambia en el tiempo.
#[test]
fn animar_un_titulo() {
    use toma_uno::animaciones::{ANIMACIONES, Propiedad};
    let mut h = arnes();
    h.state_mut().poner_titulo(0, 0.0, None);
    h.run_steps(3);
    let id = h.state().p.video.last().unwrap().id;
    h.state_mut().animar(id);
    h.run_steps(4);
    assert_eq!(format!("{:?}", h.state().espacio), "Animacion");
    let a = ANIMACIONES.iter().find(|a| a.nombre == "Entrar desde la izquierda").unwrap();
    h.get_by_label("Entrar desde la izquierda").click();
    h.run_steps(4);
    let c = h.state().p.video.iter().find(|c| c.id == id).unwrap().clone();
    let x = Propiedad::X.de(&c);
    assert_eq!(x.claves.len(), 2, "un cuadro clave en cada punta");
    assert!(x.en(0.0).num() < -90.0 && x.en(a.dur).num().abs() < 1e-6, "entra desde la izquierda");
    // Con curva «al llegar» va más adelantada a la mitad que una línea recta.
    assert!(x.en(a.dur / 2.0).num() > -50.0);
    foto(&mut h, "animacion");
}

/// Clip vacío desde el menú y desde los efectos; lo nuevo entra debajo de él.
#[test]
fn clip_vacio_en_pantalla() {
    use toma_uno::modelo::Contenido;
    let (mut h, _) = con_clips();
    h.get_by_label("AGREGAR").click();
    h.run_steps(3);
    h.get_by_label_contains("Clip vacío (efectos para todo)").click();
    h.run_steps(3);
    let v = h.state().p.video.iter().find(|c| c.contenido == Contenido::Ajuste).unwrap().clone();
    assert_eq!((v.inicio, v.capa), (0.0, 1));
    assert!((v.fin() - 7.0).abs() < 1e-6, "cubre todo el video");
    // Un texto nuevo entra debajo, para que le toque el efecto.
    h.state_mut().poner_titulo(0, 1.0, None);
    h.run_steps(3);
    let capa = |h: &Harness<App>, f: fn(&Contenido) -> bool| h.state().p.video.iter().find(|c| f(&c.contenido)).unwrap().capa;
    assert!(capa(&h, |c| matches!(c, Contenido::Titulo(_))) < capa(&h, |c| *c == Contenido::Ajuste));
    // «Aplicar a todo» usa ese mismo clip.
    pestana(&mut h, "Efectos");
    h.run_steps(3);
    h.get_by_label("Estilos listos").click();
    h.run_steps(3);
    h.get_by_label("Aplicar a todo el video").click();
    h.run_steps(3);
    foto(&mut h, "a-todo");
    assert!(h.state().lib.a_todo);
}

#[test]
fn pestana_animacion() {
    use toma_uno::animaciones::{Pista, Propiedad};
    use toma_uno::modelo::*;
    let mut h = arnes();
    h.state_mut().poner_titulo(0, 0.0, None);
    let id = h.state().p.video[0].id;
    h.state_mut().animar(id);
    h.run_steps(4);
    // Está más abajo en la lista: se busca.
    let r = h.get_by_label("Énfasis en el cursor").rect();
    h.hover_at(r.center());
    h.state_mut().anim.buscar = "flotar".into();
    h.run_steps(3);
    h.get_by_label("Flotar").click();
    h.run_steps(3);
    let c = |h: &Harness<App>| h.state().p.video.iter().find(|c| c.id == id).unwrap().clone();
    assert!(c(&h).mov.y.claves.len() > 3);
    // Elegir, copiar y pegar cuadros clave.
    h.state_mut().anim.claves = vec![(id, Pista::Mov(Propiedad::Y), 0.0)];
    h.run_steps(2);
    h.get_by_label("Copiar").click();
    h.run_steps(2);
    h.state_mut().ir(4.2);
    h.run_steps(2);
    let n = c(&h).mov.y.claves.len();
    h.get_by_label("Pegar").click();
    h.run_steps(2);
    assert_eq!(c(&h).mov.y.claves.len(), n + 1);
    h.get_by_label("Curva Rebote").click();
    h.run_steps(2);
    assert!(format!("{:?}", c(&h).mov.y.curva_en(4.2)).contains("Bounce"));
    h.state_mut().hacer_atajo("borrar");
    assert_eq!(c(&h).mov.y.claves.len(), n, "borra el cuadro clave, no el clip");
    // Grabar: cambiar un valor sin animar pone cuadros clave.
    h.get_by_label("Grabar claves").click();
    h.run_steps(2);
    assert!(h.state().anim.auto);
    h.state_mut().ir(1.0);
    h.run_steps(2);
    foto(&mut h, "anim-pistas");
    h.get_by_label("Gráfica").click();
    h.run_steps(3);
    foto(&mut h, "anim-grafica");
    h.get_by_label("Elementos").click();
    h.run_steps(3);
    h.get_by_label("Figura").click();
    h.run_steps(3);
    h.get_by_label("Estrella").click();
    h.run_steps(4);
    assert!(h.state().p.video.iter().any(|c| matches!(c.contenido, Contenido::Forma(f) if f.tipo == TipoForma::Estrella)));
    foto(&mut h, "anim-elementos");
    h.state_mut().clip = Some(id);
    h.run_steps(2);
    // «Texto» también agrega uno desde Elementos: la pastilla es la de arriba.
    let r = h.get_all_by_label("Texto").map(|x| x.rect()).min_by(|a, b| a.top().total_cmp(&b.top())).unwrap();
    clic(&mut h, r.center().x, r.center().y);
    h.run_steps(2);
    h.get_by_label("Cascada").click();
    h.run_steps(3);
    foto(&mut h, "anim-texto");
}

#[test]
fn cambiar_atajos() {
    use eframe::egui::{Key, Modifiers};
    use toma_uno::atajos;
    let mut h = arnes();
    h.state_mut().hacer_atajo("atajos");
    h.run_steps(10);
    foto(&mut h, "atajos");
    // Escrito aquí con la primera acción visible para no depender del desplazamiento.
    h.get_by_label("Agregar teclas a Guardar").click();
    h.run_steps(4);
    foto(&mut h, "atajos-espera");
    h.key_press_modifiers(Modifiers::COMMAND, Key::F7);
    h.run_steps(3);
    assert!(atajos::de("guardar").iter().any(|k| atajos::escribir(k) == "Cmd+F7"), "{:?}", atajos::de("guardar"));
    // Dárselas a otra acción se las quita a la primera.
    h.run_steps(8);
    h.get_by_label("Agregar teclas a Importar video o audio…").click();
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::COMMAND, Key::F7);
    h.run_steps(3);
    assert!(!atajos::de("guardar").iter().any(|k| atajos::escribir(k) == "Cmd+F7"));
    foto(&mut h, "atajos-cambiado");
    h.get_by_label("Regresar todos a los de fábrica").click();
    h.run_steps(2);
    assert!(!atajos::cambiado("guardar") && !atajos::cambiado("importar"));
    h.get_by_label("Listo").click();
    h.run_steps(3);
    assert!(!h.state().atajos.abierto);
}

#[test]
fn pestana_nodos() {
    use toma_uno::nodos::{Clase, ENTRADA, SALIDA};
    let (mut h, ids) = con_clips();
    h.state_mut().clip = Some(ids[0]);
    pestana(&mut h, "Nodos");
    h.run_steps(4);
    let g = |h: &Harness<App>| h.state().p.video.iter().find(|c| c.id == ids[0]).unwrap().nodos.clone();
    assert!(g(&h).is_none());
    h.get_by_label("Nodo Mezcla de color").click();
    h.run_steps(4);
    let x = g(&h).unwrap();
    assert!(x.nodos.len() == 3 && x.llega(ENTRADA, SALIDA) && x.cables.len() == 2);
    h.get_by_label("Nodo Máscara").click();
    h.run_steps(4);
    assert!(g(&h).unwrap().cables.iter().any(|c| c.puerto == 1), "la máscara se engancha al efecto elegido");
    h.get_by_label("Nodo Resplandor").click();
    h.run_steps(5);
    assert!(g(&h).unwrap().nodos.iter().any(|n| n.clase == Clase::Fusionar));
    foto(&mut h, "nodos");
    // Jalar un cable de la entrada a otro nodo lo conecta de verdad.
    let antes = g(&h).unwrap().cables.clone();
    let r = h.get_by_label("En el lienzo: Fusionar").rect();
    let e = h.get_by_label("En el lienzo: Clip").rect();
    let (a, b) = (eframe::egui::pos2(e.right(), e.top() + 13.0 * e.width() / 150.0), eframe::egui::pos2(r.left(), r.top() + (26.0 + 22.0 * 2.5) * r.width() / 150.0));
    h.hover_at(a);
    h.run_steps(2);
    h.drag_at(a);
    h.run_steps(2);
    for k in 1..=6 {
        h.hover_at(a + (b - a) * (k as f32 / 6.0));
        h.run_steps(1);
    }
    foto(&mut h, "nodos-cable");
    h.drop_at(b);
    h.run_steps(3);
    assert_eq!(g(&h).unwrap().cables.len(), antes.len() + 1, "{:?}", g(&h).unwrap().cables);
    // Borrar el nodo elegido, no el clip.
    let n = g(&h).unwrap().nodos.len();
    h.state_mut().nodos.elegido = g(&h).unwrap().nodos.iter().find(|n| n.clase == Clase::Fusionar).map(|n| n.id);
    h.state_mut().hacer_atajo("borrar");
    h.run_steps(2);
    assert_eq!(g(&h).unwrap().nodos.len(), n - 1);
    h.get_by_label("Nodo Mezcla de color").click();
    h.run_steps(4);
    foto(&mut h, "nodos-ruedas");
}

/// Carpeta con un video, un sonido, una imagen y una subcarpeta.
fn carpeta_de_prueba(n: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("beethoven-{n}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("Día 1")).unwrap();
    let ff = |args: &[&str], f: &str| {
        let s = toma_uno::medios::ffmpeg().args(["-v", "error", "-y"]).args(args).arg(d.join(f)).status().unwrap();
        assert!(s.success(), "{f}");
    };
    let video = ["-f", "lavfi", "-i", "testsrc2=s=320x180:r=30:d=3", "-f", "lavfi", "-i", "sine=440:d=3", "-pix_fmt", "yuv420p", "-shortest"];
    ff(&video, "entrevista.mp4");
    ff(&video, "Día 1/playa.mp4");
    ff(&["-f", "lavfi", "-i", "sine=330:d=4"], "tema.wav");
    ff(&["-f", "lavfi", "-i", "gradients=s=320x240", "-frames:v", "1"], "portada.png");
    std::fs::write(d.join("notas.txt"), "x").unwrap();
    d
}

/// Deja pasar cuadros hasta que se cumpla algo.
fn esperar(h: &mut Harness<App>, que: &str, f: impl Fn(&mut Harness<App>) -> bool) {
    for _ in 0..300 {
        h.run_steps(1);
        if f(h) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("no llegó: {que}");
}

#[test]
fn panel_de_archivos() {
    let d = carpeta_de_prueba("archivos");
    // Pasos cortos, para que un doble clic quepa.
    let mut h = Harness::builder().with_size([1440.0, 860.0]).with_step_dt(1.0 / 60.0).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    h.get_by_label("Panel de archivos").click();
    h.run_steps(3);
    h.get_by_label("Descargas");
    foto(&mut h, "archivos-lugares");
    h.state_mut().archivos.ir_a(Some(d.clone()));
    esperar(&mut h, "la lista", |h| h.query_by_label("entrevista.mp4").is_some());
    assert!(h.query_by_label("notas.txt").is_none(), "solo lo que se puede usar");
    // La subcarpeta se abre ahí mismo.
    h.get_by_label("Ver dentro de Día 1").click();
    esperar(&mut h, "la subcarpeta", |h| h.query_by_label("playa.mp4").is_some());
    // Vista previa
    h.get_by_label("entrevista.mp4").click();
    let r = d.join("entrevista.mp4");
    esperar(&mut h, "los datos", |h| h.state().archivos.dur(&r).is_some_and(|x| (x - 3.0).abs() < 0.2));
    esperar(&mut h, "todas las miniaturas", |h| ["tema.wav", "Día 1/playa.mp4", "portada.png"].iter().all(|n| h.state().archivos.dur(&d.join(n)).is_some()));
    h.run_steps(3);
    std::thread::sleep(std::time::Duration::from_millis(400));
    h.run_steps(4);
    foto(&mut h, "archivos");
    h.get_by_label("Ver y oír este archivo").click();
    esperar(&mut h, "que suene", |h| h.state().motor.lock().unwrap().origen.as_ref().is_some_and(|o| o.0.cuadros.len() > 40_000));
    h.get_by_label("Ver y oír este archivo").click();
    h.run_steps(3);
    assert!(h.state().motor.lock().unwrap().origen.is_none(), "se detiene");
    // Doble clic: al cursor. El segundo no tapa al primero: sube a otra pista.
    let doble = |h: &mut Harness<App>, n: &str| {
        let c = h.get_by_label(n).rect().center();
        h.hover_at(c);
        // Lejos del clic anterior, o contaría como triple.
        h.run_steps(60);
        for pressed in [true, false, true, false] {
            h.event(eframe::egui::Event::PointerButton { pos: c, button: eframe::egui::PointerButton::Primary, pressed, modifiers: Default::default() });
        }
        h.run_steps(30);
        todo_junto(h);
    };
    doble(&mut h, "entrevista.mp4");
    doble(&mut h, "playa.mp4");
    let p = &h.state().p;
    assert_eq!(p.video.iter().map(|c| (c.capa, c.inicio)).collect::<Vec<_>>(), [(0, 0.0), (1, 0.0)]);
    assert_eq!((p.ancho, p.alto), (320, 180));
    assert!(p.vinculados(p.video[0].id).len() == 2, "el sonido queda vinculado");
    // Arrastrar a las pistas, con varios elegidos.
    let mut filas = vec![];
    for n in ["portada.png", "tema.wav"] {
        filas.push(h.get_by_label(n).rect().center());
    }
    h.state_mut().archivos.elegidos = vec![d.join("portada.png"), d.join("entrevista.mp4")];
    h.run_steps(2);
    let n = h.state().p.video.len();
    // La pista V2, después de lo que tiene.
    let destino = eframe::egui::pos2(1300.0, 540.0);
    h.hover_at(filas[0]);
    h.run_steps(2);
    h.drag_at(filas[0]);
    h.run_steps(2);
    for k in 1..=8 {
        h.hover_at(filas[0] + (destino - filas[0]) * (k as f32 / 8.0));
        h.run_steps(1);
    }
    h.run_steps(2);
    foto(&mut h, "archivos-arrastre");
    h.drop_at(destino);
    h.run_steps(4);
    let p = &h.state().p;
    assert_eq!(p.video.len(), n + 2);
    let (a, b) = (&p.video[n], &p.video[n + 1]);
    assert!((b.inicio - a.inicio - 5.0).abs() < 1e-6, "uno tras otro");
    assert_eq!((a.capa, b.capa), (1, 1));
    // Búsqueda en subcarpetas, sin acentos; y cuadrícula.
    h.state_mut().archivos.buscar = "PLAYA".into();
    esperar(&mut h, "la búsqueda", |h| h.query_by_label("playa.mp4").is_some() && h.query_by_label("tema.wav").is_none());
    h.state_mut().archivos.buscar.clear();
    h.get_by_label("Ver como cuadrícula").click();
    h.run_steps(4);
    foto(&mut h, "archivos-cuadros");
    doble(&mut h, "Día 1");
    esperar(&mut h, "entrar a la carpeta", |h| h.query_by_label("playa.mp4").is_some() && h.query_by_label("tema.wav").is_none());
    h.get_by_label("Carpeta anterior").click();
    h.run_steps(3);
    assert_eq!(h.state().archivos.rec.carpeta.as_deref(), Some(d.as_path()));
    h.get_by_label("Cerrar el panel de archivos").click();
    h.run_steps(3);
    assert!(h.query_by_label("tema.wav").is_none());
    let _ = std::fs::remove_dir_all(d);
}

fn arnes_corto<'a>() -> Harness<'a, App> {
    let mut h = Harness::builder().with_size([1440.0, 860.0]).with_step_dt(1.0 / 60.0).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    h
}

fn boton(h: &mut Harness<App>, o: eframe::egui::Pos2, b: eframe::egui::PointerButton) {
    h.hover_at(o);
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(eframe::egui::Event::PointerButton { pos: o, button: b, pressed, modifiers: Default::default() });
    }
    h.run_steps(4);
}

/// Llaves con clic derecho, puntos con clic y un camino dibujado a mano.
#[test]
fn animacion_libre() {
    use eframe::egui::{PointerButton::*, pos2, vec2};
    use toma_uno::modelo::*;
    let mut h = arnes_corto();
    let id = h.state_mut().poner_forma(TipoForma::TODOS[0].0);
    h.state_mut().clip = Some(id);
    h.run_steps(4);
    let clip = |h: &Harness<App>| h.state().p.video.iter().find(|c| c.id == id).unwrap().clone();
    let img = h.get_by_label("Imagen").rect();
    // Clic derecho en la imagen: una llave de giro en el cursor, y nada más.
    h.state_mut().ir(1.0);
    h.run_steps(2);
    boton(&mut h, img.center() + vec2(150.0, 90.0), Secondary);
    foto(&mut h, "libre-menu");
    h.get_by_label("◆ Poner llave de Giro").click();
    h.run_steps(4);
    let c = clip(&h);
    assert_eq!((c.mov.giro.claves.len(), c.mov.x.claves.len()), (1, 0));
    assert!((c.mov.giro.claves[0].0 - 1.0).abs() < 0.02);
    // Puntos: tres clics, tres posiciones, y el cursor avanza.
    h.state_mut().ir(0.0);
    h.get_by_label("◆ Animar").click();
    h.run_steps(3);
    h.get_by_label("Puntos").click();
    h.run_steps(3);
    for k in 0..3 {
        boton(&mut h, img.center() + vec2(-220.0 + 200.0 * k as f32, [60.0, -60.0, 70.0][k]), Primary);
    }
    let c = clip(&h);
    assert_eq!(c.mov.x.claves.iter().map(|k| (k.0 * 100.0).round()).collect::<Vec<_>>(), [0.0, 50.0, 100.0]);
    assert!(c.mov.x.libre, "curvo");
    assert!((h.state().ahora() - 1.5).abs() < 0.05);
    foto(&mut h, "libre-puntos");
    // Un punto se arrastra.
    let antes = c.mov.x.claves[1].1.num();
    let o = h.get_by_label("Punto 2 del camino").rect().center();
    h.hover_at(o);
    h.run_steps(2);
    h.drag_at(o);
    h.run_steps(2);
    for k in 1..=6 {
        h.hover_at(o + vec2(10.0 * k as f32, 0.0));
        h.run_steps(1);
    }
    h.drop_at(o + vec2(60.0, 0.0));
    h.run_steps(3);
    let c = clip(&h);
    assert_eq!(c.mov.x.claves.len(), 3);
    assert!(c.mov.x.claves[1].1.num() > antes + 4.0, "{antes} -> {:?}", c.mov.x.claves[1]);
    // Las llaves del clip se mueven en la línea de tiempo.
    let r = h.get_all_by_label("Llave: Posición").map(|n| n.rect().center()).min_by(|a, b| b.x.total_cmp(&a.x)).unwrap();
    foto(&mut h, "libre-linea");
    let n = c.mov.x.claves.len();
    h.hover_at(r);
    h.run_steps(2);
    h.drag_at(r);
    h.run_steps(1);
    for k in 1..=6 {
        h.hover_at(r + vec2(12.0 * k as f32, 0.0));
        h.run_steps(1);
    }
    h.drop_at(r + vec2(72.0, 0.0));
    h.run_steps(3);
    let c2 = clip(&h);
    assert_eq!(c2.mov.x.claves.len(), n);
    assert!(c2.mov.x.claves != c.mov.x.claves, "se movió una llave");
    // A mano: un arco desde el cursor.
    h.state_mut().ir(2.0);
    h.get_by_label("A mano").click();
    h.run_steps(3);
    let p = |k: f32| img.center() + vec2(-200.0 * (k * 3.14).cos(), -120.0 * (k * 3.14).sin());
    h.hover_at(p(0.0));
    h.run_steps(2);
    h.drag_at(p(0.0));
    h.run_steps(1);
    for k in 1..=60 {
        h.hover_at(p(k as f32 / 60.0));
        h.run_steps(1);
    }
    foto(&mut h, "libre-mano");
    h.drop_at(p(1.0));
    h.run_steps(4);
    let c = clip(&h);
    let en = |t: f64| [c.mov.x.en(t).num(), c.mov.y.en(t).num()];
    let dur = c.mov.x.claves.last().unwrap().0 - 2.0;
    assert!(dur > 0.8 && dur < 1.6, "dura lo que se tardó: {dur}");
    assert!(en(2.0 + dur / 2.0)[1] < -15.0, "por arriba a la mitad: {:?}", en(2.0 + dur / 2.0));
    assert!(en(2.0)[0] < -10.0 && en(2.0 + dur)[0] > 10.0);
    foto(&mut h, "libre-trazo");
    // Menú del clip en la línea de tiempo.
    let _ = pos2(0.0, 0.0);
}

#[test]
fn ajustes_del_proyecto() {
    let mut h = arnes_corto();
    let d = carpeta_de_prueba("ajustes");
    h.state_mut().poner_archivos(&[d.join("entrevista.mp4")], 0.0, None, None);
    todo_junto(&mut h);
    h.state_mut().abrir_ajustes_proyecto();
    h.run_steps(3);
    h.get_by_label("Vertical · 1080×1920").click();
    h.run_steps(2);
    h.get_by_label("60").click();
    h.run_steps(2);
    h.get_by_label("Que llenen el cuadro").click();
    h.run_steps(2);
    foto(&mut h, "ajustes-proyecto");
    h.get_by_label("Aplicar").click();
    h.run_steps(4);
    let p = &h.state().p;
    assert_eq!((p.ancho, p.alto, p.fps), (1080, 1920, 60.0));
    // 16:9 llenando 9:16.
    assert!((p.video[0].mov.escala.v.num() - 316.0).abs() < 1.0, "{:?}", p.video[0].mov.escala);
    // El primer video de un proyecto con tamaño elegido no lo cambia.
    h.state_mut().p.video.clear();
    h.state_mut().poner_archivos(&[d.join("entrevista.mp4")], 0.0, None, None);
    assert_eq!(h.state().p.ancho, 1080);
    foto(&mut h, "vertical");
    let _ = std::fs::remove_dir_all(d);
}

/// Los puntos del ecualizador del DAW se arrastran sobre la curva.
#[test]
fn ecualizador_parametrico() {
    use eframe::egui::vec2;
    use toma_uno::efectos::TipoFx;
    let mut h = arnes_corto();
    let id = h.state_mut().p.nueva_pista("Voz");
    let f = h.state_mut().p.nuevo_fx(TipoFx::Parametrico);
    h.state_mut().p.pista(id).unwrap().fx.push(f);
    h.state_mut().canal = toma_uno::ui::Canal::Pista(id);
    pestana(&mut h, "Audio");
    h.run_steps(5);
    let antes = h.state_mut().p.pista(id).unwrap().fx[0].p.clone();
    let o = h.get_by_role_and_label(Role::Button, "Banda 4").rect().center();
    h.hover_at(o);
    h.run_steps(2);
    h.drag_at(o);
    h.run_steps(2);
    for k in 1..=6 {
        h.hover_at(o + vec2(5.0 * k as f32, -6.0 * k as f32));
        h.run_steps(1);
    }
    h.drop_at(o + vec2(30.0, -36.0));
    h.run_steps(4);
    foto(&mut h, "parametrico");
    let p = h.state_mut().p.pista(id).unwrap().fx[0].p.clone();
    assert!(p[13] > antes[13] * 1.2 && p[14] > antes[14] + 4.0, "{:?} -> {:?}", &antes[12..16], &p[12..16]);
    assert_eq!(p[0..12], antes[0..12], "las demás quietas");
}

/// El menú: cambiar nombre, duplicar, buscar, mandar a la papelera y restaurar.
#[test]
fn proyectos_del_menu() {
    use toma_uno::modelo::Proyecto;
    let d = std::env::temp_dir().join(format!("beethoven-menu-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let d = d.canonicalize().unwrap();
    for (i, n) in ["Boda de Ana", "Video musical", "Vlog de la playa", "Comercial"].into_iter().enumerate() {
        let mut q = vec![255u8; 320 * 180 * 4];
        q.chunks_mut(4).enumerate().for_each(|(k, p)| {
            let (x, y) = ((k % 320) as f32 / 320.0, (k / 320) as f32 / 180.0);
            let c = [[220.0, 120.0, 90.0], [90.0, 110.0, 220.0], [80.0, 180.0, 190.0], [200.0, 170.0, 80.0]][i];
            (0..3).for_each(|j| p[j] = (c[j] * (0.35 + 0.65 * (1.0 - y)) * (0.8 + 0.2 * x)) as u8);
        });
        toma_uno::archivo::guardar(&Proyecto::nuevo(), &d.join(format!("{n}.bcraft")), Some((320, 180, &q[..])), false).unwrap();
    }
    std::fs::write(d.join("Cumbia.beethoven"), "x").unwrap();
    let mut h = Harness::builder().with_size([1180.0, 740.0]).with_step_dt(1.0 / 60.0).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.state_mut().inicio.base = Some(d.clone());
    h.state_mut().inicio.mostrar();
    // La canción entra a la lista como si se hubiera abierto.
    toma_uno::proyectos::Libro { base: d.clone() }.anotar(&d.join("Cumbia.beethoven"));
    h.run_steps(4);
    foto(&mut h, "menu-proyectos");
    let nombres = |h: &Harness<App>| {
        let mut v: Vec<String> = h.state().inicio.lista.iter().flatten().map(|f| f.nombre.clone()).collect();
        v.sort();
        v
    };
    assert_eq!(nombres(&h).len(), 5);
    h.get_by_label("Videos").click();
    h.run_steps(3);
    assert!(h.query_by_label("Cumbia").is_none() && h.query_by_label("Comercial").is_some());
    h.get_by_label("Todos").click();
    h.run_steps(3);
    // Cambiar nombre, desde los tres puntos.
    h.get_by_label("Más de Comercial").click();
    h.run_steps(4);
    foto(&mut h, "menu-opciones");
    h.get_by_label("Cambiar nombre…").click();
    h.run_steps(4);
    h.state_mut().inicio.renombrando.as_mut().unwrap().1 = "Comercial de tenis".into();
    h.run_steps(2);
    h.get_by_label("Guardar nombre").click();
    h.run_steps(4);
    assert!(d.join("Comercial de tenis.bcraft").exists() && !d.join("Comercial.bcraft").exists());
    // Duplicar
    h.get_by_label("Más de Vlog de la playa").click();
    h.run_steps(4);
    h.get_by_label("Duplicar").click();
    h.run_steps(4);
    assert!(nombres(&h).contains(&"Vlog de la playa copia".into()), "{:?}", nombres(&h));
    // Buscar, sin acentos ni mayúsculas.
    h.state_mut().inicio.buscar = "PLAYA".into();
    h.run_steps(3);
    assert!(h.query_by_label("Vlog de la playa").is_some() && h.query_by_label("Boda de Ana").is_none());
    h.state_mut().inicio.buscar.clear();
    h.run_steps(3);
    // Eliminar dos a la vez: pregunta, y se van a la papelera.
    for n in ["Boda de Ana", "Vlog de la playa copia"] {
        h.state_mut().inicio.elegidos.insert(d.join(format!("{n}.bcraft")));
    }
    h.run_steps(3);
    foto(&mut h, "menu-elegidos");
    h.get_by_label("Eliminar").click();
    h.run_steps(4);
    assert!(d.join("Boda de Ana.bcraft").exists(), "antes del sí no pasa nada");
    foto(&mut h, "menu-eliminar");
    h.get_by_label("Sí, eliminar").click();
    h.run_steps(5);
    assert!(!d.join("Boda de Ana.bcraft").exists());
    assert_eq!((nombres(&h).len(), h.state().inicio.borrados.len()), (4, 2));
    // Restaurar una y borrar la otra para siempre.
    h.get_by_label("Papelera").click();
    h.run_steps(4);
    foto(&mut h, "menu-papelera");
    let b = h.state().inicio.borrados.clone();
    let i = b.iter().position(|b| b.original.ends_with("Boda de Ana.bcraft")).unwrap();
    h.get_all_by_label("Restaurar").nth(i).unwrap().click();
    h.run_steps(5);
    assert!(d.join("Boda de Ana.bcraft").exists());
    h.get_by_label("Borrar para siempre").click();
    h.run_steps(4);
    h.get_by_label("Sí, borrar para siempre").click();
    h.run_steps(5);
    assert!(h.state().inicio.borrados.is_empty());
    h.get_by_label("Proyectos").click();
    h.run_steps(4);
    h.get_by_label("Ver como lista").click();
    h.run_steps(4);
    foto(&mut h, "menu-lista");
    // Formato a un clic.
    h.get_by_label("9:16").click();
    h.run_steps(3);
    assert_eq!((h.state().p.ancho, h.state().p.alto, h.state().p.fijo), (1080, 1920, true));
    let _ = std::fs::remove_dir_all(d);
}

/// Un audio elegido se manda a un mezclador nuevo, se le pone nombre y sus efectos van aparte; y los clips se tiñen.
#[test]
fn mezclador_y_color_de_clips() {
    use toma_uno::modelo::Clip;
    use toma_uno::ui::Canal;
    let mut h = arnes_corto();
    let a = &mut h.state_mut().p;
    let pista = a.nueva_pista("Pista de voz");
    let mut ids = vec![];
    for k in 0..2 {
        let id = a.id();
        let clip = Clip { id, ruta: format!("/no/{k}.wav").into(), nombre: format!("Toma {k}"), inicio: k as f64 * 6.0, desde: 0.0, dur: 5.0, total: 5.0, ganancia_db: 0.0, fade_in: 0.0, fade_out: 0.0, invertido: false, grupo: None, velocidad: 1.0, bus: None, color: None };
        a.pista(pista).unwrap().clips.push(clip);
        ids.push(id);
    }
    h.state_mut().clip = Some(ids[0]);
    h.state_mut().seleccion = ids.iter().copied().collect();
    pestana(&mut h, "Audio");
    h.run_steps(4);
    h.get_by_label("→ Por su pista").click();
    h.run_steps(4);
    foto(&mut h, "mezclador-menu");
    h.get_by_label("+ Nuevo mezclador").click();
    h.run_steps(4);
    // Los dos elegidos van al nuevo, y el panel enseña sus efectos.
    let b = h.state().p.buses[0].id;
    assert!(ids.iter().all(|i| h.state_mut().p.clip(*i).unwrap().bus == Some(b)));
    assert_eq!(h.state().canal, Canal::Bus(b));
    let f = h.state_mut().p.nuevo_fx(toma_uno::efectos::TipoFx::Reverberacion);
    h.state_mut().fx_de(Canal::Bus(b)).push(f);
    h.state_mut().p.bus(b).unwrap().nombre = "Voces".into();
    h.run_steps(4);
    let t = &h.state().p;
    assert!(t.pistas.iter().all(|p| p.fx.is_empty()) && t.buses[0].fx.len() == 1);
    // Color: a los dos a la vez.
    h.get_by_label("Color").click();
    h.run_steps(4);
    foto(&mut h, "color-menu");
    h.get_by_label("Turquesa").click();
    h.run_steps(4);
    assert!(ids.iter().all(|i| h.state_mut().p.clip(*i).unwrap().color == Some([0x2f, 0x9c, 0x92])));
    foto(&mut h, "mezclador");
    h.state_mut().borrar_bus(b);
    h.run_steps(3);
    assert!(h.state_mut().p.clip(ids[0]).unwrap().bus.is_none());
}
