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
    a.video[0].mov.escala = Prop { v: Valor::Num(100.0), claves: vec![(10.0, Valor::Num(100.0)), (40.0, Valor::Num(80.0))], curvas: vec![] };
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
    for s in ["Estilos listos", "Video", "Transiciones", "Audio", "Títulos"] {
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
