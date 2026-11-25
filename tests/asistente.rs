//! El asistente de IA edita el proyecto por medio de sus herramientas.
use egui_kittest::Harness;
use serde_json::{Value, json};
use std::sync::Arc;
use toma_uno::medios::Audio;
use toma_uno::modelo::*;
use toma_uno::ui::App;

fn abrir() -> Harness<'static, App> {
    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(2);
    h
}

fn hace(h: &mut Harness<App>, n: &str, d: Value) -> Value {
    let r = h.state_mut().hacer_ia(n, &d).unwrap_or_else(|e| panic!("{n} {d}: {e}"));
    h.run_steps(1);
    serde_json::from_str(&r).unwrap_or(Value::Null)
}

/// Un video de 60 s con su sonido vinculado: 3 s de tono y 2 de silencio, repetido.
fn con_video(h: &mut Harness<App>) -> (u64, u64) {
    let ruta = std::path::PathBuf::from("/charla.mp4");
    let x: Vec<[f32; 2]> = (0..SR as usize * 60).map(|i| if (i / SR as usize) % 5 < 3 { [(i as f32 * 0.04).sin() * 0.3; 2] } else { [0.0; 2] }).collect();
    let a = h.state_mut();
    a.almacen.insert(ruta.clone(), Arc::new(Audio::nuevo(x)));
    let mut v = a.p.nuevo_video(ruta.clone(), Contenido::Archivo, 0.0, 60.0);
    v.capa = 0;
    let vid = v.id;
    a.p.video.push(v);
    let pista = a.p.nueva_pista("Voz");
    let id = a.p.id();
    a.p.pista(pista).unwrap().clips.push(Clip {
        id,
        ruta,
        nombre: "charla".into(),
        inicio: 0.0,
        desde: 0.0,
        dur: 60.0,
        total: 60.0,
        ganancia_db: 0.0,
        fade_in: 0.0,
        fade_out: 0.0,
        invertido: false,
        grupo: None,
        velocidad: 1.0,
        bus: None,
        color: None,
    });
    a.p.vincular(&[vid, id]);
    h.run_steps(2);
    (vid, id)
}

#[test]
fn corta_el_video_donde_se_pide() {
    let mut h = abrir();
    let (v, a) = con_video(&mut h);
    // «Corta el video en el segundo 30 y borra lo que sigue.»
    let n = hace(&mut h, "cortar", json!({ "id": v, "t": 30 }))["nuevos"].clone();
    assert_eq!(n.as_array().unwrap().len(), 2, "imagen y sonido");
    hace(&mut h, "borrar", json!({ "ids": [n[0]] }));
    let p = &h.state().p;
    assert_eq!((p.video.len(), p.pistas[0].clips.len(), p.fin()), (1, 1, 30.0));
    // Se deshace.
    hace(&mut h, "deshacer", json!({}));
    assert!((h.state().p.fin() - 60.0).abs() < 1e-6);
    hace(&mut h, "deshacer", json!({}));
    assert_eq!(h.state().p.video.len(), 1);
    // Mover, recortar y velocidad van con su sonido.
    hace(&mut h, "mover", json!({ "id": v, "inicio": 5 }));
    hace(&mut h, "recortar", json!({ "id": v, "inicio": 10, "fin": 40 }));
    let x = h.state().p.tramo_de(a).unwrap();
    assert_eq!((x.inicio, x.fin(), x.desde), (10.0, 40.0, 5.0));
    hace(&mut h, "velocidad", json!({ "id": v, "velocidad": 2 }));
    assert!((h.state().p.tramo_de(a).unwrap().dur - 15.0).abs() < 1e-6);
}

#[test]
fn quita_los_silencios_y_los_junta() {
    let mut h = abrir();
    let (v, _) = con_video(&mut h);
    let m = hace(&mut h, "analizar_audio", json!({ "id": v }));
    assert_eq!(m["silencios"].as_array().unwrap().len(), 12);
    let r = hace(&mut h, "quitar_silencios", json!({ "id": v }));
    assert_eq!(r["quitados"], 12);
    let p = &h.state().p;
    // Quedan 12 pedazos de unos 3 s, pegados, y el sonido igual que la imagen.
    assert!((p.fin() - 37.5).abs() < 0.8, "{}", p.fin());
    assert_eq!((p.video.len(), p.pistas[0].clips.len()), (12, 12));
    let mut i: Vec<(f64, f64)> = p.video.iter().map(|c| (c.inicio, c.fin())).collect();
    i.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert!(i.windows(2).all(|w| (w[0].1 - w[1].0).abs() < 0.01), "{i:?}");
}

#[test]
fn titulos_efectos_y_animacion() {
    let mut h = abrir();
    let (v, _) = con_video(&mut h);
    let x = hace(&mut h, "agregar_titulo", json!({ "texto": "Hola", "inicio": 1, "dur": 4, "tam": 14, "color": "#ffcc00", "y": -20 }));
    assert!(x["capa"].as_u64().unwrap() > 0);
    let id = x["ids"][0].clone();
    hace(&mut h, "animar", json!({ "id": id, "propiedad": "opacidad", "llaves": [{ "t": 0, "valor": 0 }, { "t": 1, "valor": 100 }] }));
    hace(&mut h, "subtitulos", json!({ "lista": [{ "inicio": 6, "fin": 8, "texto": "uno" }, { "inicio": 8, "fin": 9.5, "texto": "dos" }] }));
    let cat = hace(&mut h, "ver_catalogo", json!({ "que": "efectos_video", "buscar": "desenfoque" }));
    let fx = cat[0][0].clone();
    let i = hace(&mut h, "agregar_efecto", json!({ "id": v, "efecto": fx }))["indice"].clone();
    let w = hace(&mut h, "ver_efecto", json!({ "id": v }));
    let (k, _) = w[0]["valores"].as_object().unwrap().iter().find(|x| x.1.is_number()).unwrap();
    hace(&mut h, "ajustar_efecto", json!({ "id": v, "indice": i, "valores": { k: 3 } }));
    hace(&mut h, "transicion", json!({ "id": v, "tipo": "cross_dissolve", "dur": 0.5 }));
    hace(&mut h, "formato", json!({ "ancho": 1080, "alto": 1920 }));
    hace(&mut h, "ajustar_video", json!({ "id": v, "escala": 180, "x": 5 }));
    let pista = h.state().p.pistas[0].id;
    hace(&mut h, "efecto_audio", json!({ "pista": pista, "efecto": "Compresor" }));
    hace(&mut h, "ajustar_pista", json!({ "pista": pista, "vol_db": -3 }));
    hace(&mut h, "varias", json!({ "acciones": [{ "herramienta": "marca", "datos": { "t": 12, "nombre": "aquí" } }, { "herramienta": "agregar_capa" }] }));
    let p = &h.state().p;
    let tit = p.video.iter().find(|c| json!(c.id) == id).unwrap();
    assert!(matches!(&tit.contenido, Contenido::Titulo(x) if x.texto == "Hola" && x.tam == 14.0) && tit.mov.opacidad.claves.len() == 2 && tit.mov.y.v.num() == -20.0);
    let c = p.video.iter().find(|c| c.id == v).unwrap();
    assert!(c.fx.len() == 1 && c.entrada.as_ref().unwrap().dur == 0.5 && c.mov.escala.v.num() == 180.0);
    assert_eq!((p.ancho, p.alto, p.pistas[0].fx.len(), p.pistas[0].vol_db, p.marcas.len()), (1080, 1920, 1, -3.0, 1));
    // Lo que no se puede, se dice y no cambia nada.
    let antes = h.state().p.clone();
    for (n, d) in [
        ("cortar", json!({ "t": 900 })),
        ("cortar", json!({ "id": 999999, "t": 3 })),
        ("agregar_efecto", json!({ "id": v, "efecto": "no_existe" })),
        ("ajustar_efecto", json!({ "id": v, "indice": 0, "valores": { k: "rojo" } })),
        ("transicion", json!({ "id": v, "tipo": "nada" })),
        ("agregar_titulo", json!({ "texto": "x", "inicio": 0, "dur": 1, "color": "rojo" })),
        ("ver_cuadro", json!({ "t": 1 })),
        ("importar", json!({ "ruta": "/no/hay.mp4" })),
        ("guardar", json!({})),
        ("borrar_disco", json!({})),
    ] {
        assert!(h.state_mut().hacer_ia(n, &d).is_err(), "{n} {d}");
    }
    assert!(h.state().p == antes);
    h.state_mut().asistente.abierto = true;
    h.run_steps(3);
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/movie-asistente.png")).unwrap();
    }
}

/// Con la IA instalada de verdad: gasta de la cuenta de quien la corre. `MODELO=haiku cargo test … -- --ignored`
#[test]
#[ignore]
fn con_una_ia_de_verdad() {
    let mut h = abrir();
    con_video(&mut h);
    assert!(h.state_mut().usar_ia("Claude Code", &std::env::var("MODELO").unwrap_or_default()));
    h.state_mut().asistente.abierto = true;
    h.state_mut().inicio.visible = false;
    h.state_mut().decir_ia("Corta el video en el segundo 20, borra lo que sigue y pon el título \"Bienvenidos\" en los primeros 3 segundos.");
    let t = std::time::Instant::now();
    while h.state().dicho_ia().1 && t.elapsed().as_secs() < 180 {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(60));
    }
    h.run_steps(3);
    println!("{:.0} s\n{}", t.elapsed().as_secs_f32(), h.state().dicho_ia().0.join("\n"));
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/movie-real.png")).unwrap();
    }
    let p = &h.state().p;
    assert!((p.fin() - 20.0).abs() < 0.01, "{}", p.fin());
    assert!(p.video.iter().any(|c| matches!(&c.contenido, Contenido::Titulo(x) if x.texto.contains("Bienvenidos"))));
}
