//! La ventana, en español y en inglés. `CAPTURAS=/carpeta` guarda imágenes.
use beethoven_daw_demo::idioma::{self, Idioma};
use beethoven_daw_demo::ui::{App, Vista};
use eframe::egui::{self, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use std::sync::Mutex;

// El idioma es de todo el programa: de una en una.
static UNA: Mutex<()> = Mutex::new(());

fn abrir(i: Idioma) -> Harness<'static, App> {
    idioma::poner(i);
    let mut h = Harness::builder().with_size([1360.0, 820.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true));
    h.run_steps(3);
    h
}

fn foto(h: &mut Harness<App>, n: &str) {
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/{n}.png")).unwrap();
    }
}

#[test]
fn las_vistas() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    for i in Idioma::TODOS {
        let mut h = abrir(i);
        for (v, n) in [(Vista::Arreglo, "arreglo"), (Vista::Ritmo, "ritmo"), (Vista::Notas, "notas"), (Vista::Mezcla, "mezcla")] {
            h.state_mut().mostrar(v);
            if v == Vista::Notas {
                (h.state_mut().patron, h.state_mut().canal) = (3, 5);
            }
            if v == Vista::Mezcla {
                for k in ["ecualizador", "compresor"] {
                    h.state_mut().p.mezcla[1].efectos.push(beethoven_daw_demo::modelo::EfectoPuesto::nuevo(k));
                }
                let c = &mut h.state_mut().p.mezcla[1].efectos[0].controles;
                c.extend([("g1".into(), 7.0), ("g3".into(), -6.0), ("g5".into(), 4.0)]);
            }
            h.run_steps(3);
            foto(&mut h, &format!("{}-{n}", i.codigo()));
        }
        h.get_by_label(if i == Idioma::Es { "Archivo" } else { "File" }).click();
        h.run_steps(3);
        foto(&mut h, &format!("{}-archivo", i.codigo()));
        h.get_by_label_contains(if i == Idioma::Es { "Exportar audio…" } else { "Export audio…" });
    }
}

#[test]
fn usarla() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = abrir(Idioma::Es);
    // Mirar no cambia el proyecto.
    let antes = h.state().p.clone();
    for v in [Vista::Ritmo, Vista::Notas, Vista::Mezcla, Vista::Arreglo] {
        h.state_mut().mostrar(v);
        h.run_steps(3);
    }
    assert_eq!(h.state().p, antes);

    // Agregar un instrumento de la izquierda y deshacerlo.
    let n = h.state().p.canales.len();
    // Los instrumentos grabados van por familia: se abre la suya.
    h.get_by_label("Órganos y acordeones").click();
    h.run_steps(3);
    h.get_by_label("Órgano").click();
    h.run_steps(3);
    assert_eq!(h.state().p.canales.len(), n + 1);
    h.state_mut().deshacer();
    h.run_steps(2);
    assert_eq!(h.state().p, antes);
    h.state_mut().rehacer();
    h.run_steps(2);
    assert_eq!(h.state().p.canales.len(), n + 1);

    // Una nota con un clic en el piano roll.
    let canal = h.state().p.canales[n].id;
    h.state_mut().mostrar(Vista::Notas);
    (h.state_mut().patron, h.state_mut().canal) = (1, canal);
    h.run_steps(3);
    let o = pos2(600.0, 400.0);
    h.hover_at(o);
    h.run_steps(1);
    h.drag_at(o);
    h.run_steps(1);
    h.drop_at(o);
    h.run_steps(2);
    let v = h.state().p.patron(1).unwrap().notas[&canal].clone();
    assert_eq!(v.len(), 1);
    // Se transpone con las flechas y se borra.
    h.key_press(egui::Key::ArrowUp);
    h.run_steps(2);
    assert_eq!(h.state().p.patron(1).unwrap().notas[&canal][0].tono, v[0].tono + 1);
    h.key_press(egui::Key::Delete);
    h.run_steps(2);
    assert!(!h.state().p.patron(1).unwrap().notas.contains_key(&canal));
}

#[derive(Debug)]
struct Soltado(std::path::PathBuf);
impl egui::DroppedFile for Soltado {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

#[test]
fn soltar_audio_y_mezclarlo() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = abrir(Idioma::Es);
    let dir = std::env::temp_dir().join(format!("bdd-suelta-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ruta = dir.join("voz principal.wav");
    let x: Vec<[f32; 2]> = (0..96000).map(|i| [(i as f32 * 0.05).sin() * 0.4; 2]).collect();
    beethoven_daw_demo::wav::escribir(&ruta, 48000, &x).unwrap();
    let n = h.state().p.clips.len();
    h.input_mut().dropped_files.push(std::sync::Arc::new(Soltado(ruta.clone())));
    h.run_steps(3);
    // Quedó en la playlist y con su propia pista del Mixer.
    let p = &h.state().p;
    assert_eq!(p.clips.len(), n + 1);
    let a = p.clips[n].audio.clone().unwrap();
    assert_eq!((a.mezcla, p.mezcla[a.mezcla].nombre.as_str()), (6, "voz principal"));
    assert_eq!(h.state().clip, Some(n));
    foto(&mut h, "audio-playlist");

    // Suelto encima de una ventana, cae según dónde: la playlist, el ritmo (instrumento) o una tira del Mixer.
    let ctx = h.ctx.clone();
    h.state_mut().soltar(&ctx, &ruta, Some(pos2(700.0, 340.0)));
    h.run_steps(2);
    let c = h.state().p.clips.last().unwrap().clone();
    // La misma grabación usa la misma pista del Mixer.
    assert!(c.pista == 4 && c.tick > 0 && c.audio.unwrap().mezcla == 6, "pista {} tick {}", c.pista, c.tick);
    let canales = h.state().p.canales.len();
    h.state_mut().mostrar(Vista::Ritmo);
    h.run_steps(3);
    h.state_mut().soltar(&ctx, &ruta, Some(pos2(500.0, 700.0)));
    h.run_steps(2);
    assert_eq!(h.state().p.canales.len(), canales + 1);
    h.state_mut().mostrar(Vista::Mezcla);
    h.run_steps(3);
    h.state_mut().soltar(&ctx, &ruta, Some(pos2(430.0, 600.0)));
    h.run_steps(2);
    assert_eq!(h.state().p.clips.last().unwrap().audio.as_ref().unwrap().mezcla, 2);

    // En el Mixer: agregar un efecto en su ranura y abrir su ventana con un clic.
    h.state_mut().mostrar(Vista::Mezcla);
    h.run_steps(3);
    let i = 2;
    h.state_mut().p.mezcla[i].efectos.push(beethoven_daw_demo::modelo::EfectoPuesto::nuevo("compresor"));
    h.state_mut().p.mezcla[i].efectos.push(beethoven_daw_demo::modelo::EfectoPuesto::nuevo("ecualizador"));
    h.run_steps(2);
    assert!(h.state().fx_abiertos.is_empty());
    for k in [0, 1] {
        h.state_mut().abrir_efecto(i, k);
        h.run_steps(3);
    }
    assert_eq!(h.state().fx_abiertos, [(i, 0), (i, 1)]);
    foto(&mut h, "efectos");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn la_playlist_es_el_fondo() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = abrir(Idioma::Es);
    assert_eq!(h.state().pestana, Vista::Arreglo);
    // El Mixer es otra pestaña; cerrarlo regresa a la playlist completa.
    h.get_by_label("Mixer").click();
    h.run_steps(2);
    assert!(h.state().pestana == Vista::Mezcla && !h.state().abierta(Vista::Arreglo));
    h.key_press(egui::Key::F9);
    h.run_steps(2);
    assert_eq!(h.state().pestana, Vista::Arreglo);
    // Lo demás abre encima, sin quitar la playlist, y también se cierra con su tecla.
    h.key_press(egui::Key::F6);
    h.run_steps(2);
    assert!(h.state().abierta(Vista::Ritmo) && h.state().abierta(Vista::Arreglo));
    h.key_press(egui::Key::F6);
    h.run_steps(2);
    assert!(!h.state().abierta(Vista::Ritmo) && h.state().abierta(Vista::Arreglo));
}

#[test]
fn el_asistente_controla_el_proyecto() {
    use serde_json::{Value, json};
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = abrir(Idioma::Es);
    let dir = std::env::temp_dir().join(format!("bdd-asis-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ruta = dir.join("voz.wav");
    beethoven_daw_demo::wav::escribir(&ruta, 48000, &vec![[0.1; 2]; 48000 * 20]).unwrap();
    let a = h.state_mut();
    let mut hace =
        |n: &str, d: Value| -> Value { serde_json::from_str(&a.hacer_ia(n, &d).unwrap_or_else(|e| panic!("{n}: {e}"))).unwrap_or(Value::Null) };
    hace("ajustar_proyecto", json!({ "bpm": 120, "nombre": "Prueba" }));
    // «Corta la voz en el segundo 12 y baja la segunda parte.»
    let c = hace("poner_audio", json!({ "ruta": ruta, "segundo": 2 }))["clip"].as_u64().unwrap();
    let k = hace("cortar_clip", json!({ "clip": c, "segundo": 12 }));
    hace("ajustar_audio", json!({ "clip": k["derecha"], "volumen": 0.5, "entra": 1.5 }));
    let v = hace("ver_proyecto", json!({}));
    let clip = |v: &Value, i: &Value| v["clips"][i.as_u64().unwrap() as usize].clone();
    let (i, d) = (clip(&v, &k["izquierda"]), clip(&v, &k["derecha"]));
    assert_eq!(
        (i["segundo"].as_f64(), i["segundos"].as_f64(), d["segundo"].as_f64(), d["volumen"].as_f64()),
        (Some(2.0), Some(10.0), Some(12.0), Some(0.5))
    );
    // Un instrumento nuevo con notas, puesto en la canción, con un efecto.
    let canal = hace("agregar_canal", json!({ "instrumento": "cuerda", "nombre": "Guitarra" }))["canal"].clone();
    let patron = hace("crear_patron", json!({ "nombre": "Arpegio", "compases": 2 }))["patron"].clone();
    hace(
        "poner_notas",
        json!({ "patron": patron, "canal": canal, "notas": [{ "paso": 0, "largo": 4, "tono": 60 }, { "paso": 4, "largo": 4, "tono": 64, "fuerza": 0.5 }] }),
    );
    assert_eq!(hace("ver_notas", json!({ "patron": patron, "canal": canal }))[1], json!({ "paso": 4.0, "largo": 4.0, "tono": 64, "fuerza": 0.5 }));
    hace("poner_clip", json!({ "patron": patron, "pista": 9, "compas": 3, "compases": 4 }));
    let e = hace("agregar_efecto", json!({ "pista": 3, "efecto": "vozencaja", "controles": { "tam": 0.2, "fuego": 9 } }))["indice"].clone();
    hace("ajustar_efecto", json!({ "pista": 3, "indice": e, "mezcla": 0.4 }));
    hace("ajustar_mezcla", json!({ "pista": 3, "volumen": 0.6, "eq": [3, 0, -99], "envios": [0.5] }));
    hace("ritmo_listo", json!({ "patron": patron, "ritmo": "cumbia" }));
    hace("usar_herramienta_notas", json!({ "herramienta": "acorde", "patron": patron, "canal": canal, "controles": { "menor": 1 } }));
    hace(
        "escribir_guion",
        json!({ "clave": "de_la_ia", "codigo": "return { nombre='De la IA', sonido=function(n,c) return seno(n.hz) * golpe(0.3) end }" }),
    );
    hace("agregar_canal", json!({ "instrumento": "de_la_ia" }));
    assert!(hace("ver_biblioteca", json!({}))["guiones"].as_array().unwrap().len() > 30);
    // Varias de golpe, y lo que necesita oír: un audio con huecos.
    let huecos = dir.join("huecos.wav");
    let x: Vec<[f32; 2]> = (0..48000 * 6).map(|i| if (i / 48000) % 2 == 0 { [(i as f32 * 0.06).sin() * 0.25; 2] } else { [0.0; 2] }).collect();
    beethoven_daw_demo::wav::escribir(&huecos, 48000, &x).unwrap();
    let a = h.state_mut();
    let mut hace =
        |n: &str, d: Value| -> Value { serde_json::from_str(&a.hacer_ia(n, &d).unwrap_or_else(|e| panic!("{n}: {e}"))).unwrap_or(Value::Null) };
    let v = hace(
        "varias",
        json!({ "acciones": [
        { "herramienta": "poner_audio", "datos": { "ruta": huecos, "segundo": 10, "pista": 12 } },
        { "herramienta": "crear_patron", "datos": { "nombre": "Z" } },
    ] }),
    );
    let c = v[0]["clip"].clone();
    let m = hace("analizar_audio", json!({ "clip": c }));
    assert_eq!(m["silencios"].as_array().unwrap().len(), 3);
    assert!((m["silencios"][0][0].as_f64().unwrap() - 11.0).abs() < 0.05 && (m["pico_db"].as_f64().unwrap() + 12.04).abs() < 0.1, "{m}");
    assert!((hace("normalizar_audio", json!({ "clip": c, "pico_db": -8 }))["volumen"].as_f64().unwrap() - 1.592).abs() < 0.02);
    let k = hace("quitar_silencios", json!({ "clip": c, "cerrar": true }))["clips"].clone();
    let p = hace("ver_proyecto", json!({}));
    let l: Vec<f64> = k.as_array().unwrap().iter().map(|i| p["clips"][i.as_u64().unwrap() as usize]["segundo"].as_f64().unwrap()).collect();
    assert!(l.len() == 3 && (l[0] - 10.0).abs() < 0.01 && (l[1] - 11.04).abs() < 0.03, "{l:?}");
    let m = hace("medir_mezcla", json!({}));
    assert!(m["general"]["pico_db"].as_f64().unwrap() > -30.0 && m["pistas"].as_array().unwrap().len() >= 5);
    assert!(h.state_mut().hacer_ia("guia_lua", &json!({})).unwrap().contains("dibujar"));
    hacer_bien(&mut h, "mostrar", json!({ "ventana": "Mixer", "pista": 3 }));
    assert!(h.state().abierta(Vista::Mezcla));
    let e = h.state_mut().hacer_ia("varias", &json!({ "acciones": [{ "herramienta": "deshacer" }, { "herramienta": "nada" }] })).unwrap_err();
    assert!(e.contains("\"fallo_en\":1"), "{e}");
    // Sin permiso no busca en las carpetas.
    assert!(h.state_mut().hacer_ia("buscar_audios", &json!({ "texto": "x" })).is_err());
    // Lo que no se puede, se dice y no cambia nada.
    let a = h.state_mut();
    let antes = a.p.clone();
    for (n, d) in [
        ("cortar_clip", json!({ "clip": 999, "compas": 1 })),
        ("cortar_clip", json!({ "clip": 0, "compas": 900 })),
        ("poner_audio", json!({ "ruta": "/etc/passwd" })),
        ("poner_audio", json!({ "ruta": "../x.wav" })),
        ("agregar_canal", json!({ "instrumento": "reverb" })),
        ("ajustar_canal", json!({ "canal": 1, "controles": { "nada": 1 } })),
        ("ajustar_mezcla", json!({ "pista": 99 })),
        ("escribir_guion", json!({ "clave": "../malo", "codigo": "return {}" })),
        ("escribir_guion", json!({ "clave": "malo", "codigo": "return { nombre='x', sonido=function() return io.open('x') end }" })),
        ("formatear_disco", json!({})),
        ("poner_notas", Value::Null),
    ] {
        assert!(a.hacer_ia(n, &d).is_err(), "{n} {d}");
    }
    assert_eq!(a.p, antes);
    assert!(a.p.mezcla[3].efectos.iter().any(|e| e.controles.get("fuego") == Some(&1.0) && e.mezcla == 0.4) && a.p.mezcla[3].eq == [3.0, 0.0, -18.0]);
    h.state_mut().mostrar(Vista::Asistente);
    h.run_steps(3);
    foto(&mut h, "asistente");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn elegir_instrumento_y_editar_la_playlist() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = abrir(Idioma::Es);
    // Una pista nueva pregunta el instrumento y abre su piano roll.
    let (canales, clips) = (h.state().p.canales.len(), h.state().p.clips.len());
    h.get_by_label("+ Pista").click();
    h.run_steps(3);
    foto(&mut h, "elegir");
    // Las tejas de abajo quedan fuera de la vista: se trae la suya antes del clic.
    h.get_all_by_label("Guitarra clásica").last().unwrap().scroll_to_me();
    h.run_steps(3);
    h.get_all_by_label("Guitarra clásica").last().unwrap().click();
    h.run_steps(3);
    let a = h.state();
    assert_eq!((a.p.canales.len(), a.p.clips.len(), a.p.canales.last().unwrap().instrumento.as_str()), (canales + 1, clips + 1, "cuerda"));
    assert!(a.abierta(Vista::Notas) && a.patron == a.p.clips.last().unwrap().patron && a.canal == a.p.canales.last().unwrap().id);
    foto(&mut h, "pista-nueva");
    // Y en el piano roll se cambia con qué suena, sin perder el canal.
    let canal = h.state().canal;
    h.get_by_label("Suena: Guitarra clásica").click();
    h.run_steps(3);
    h.get_all_by_label("Órgano").last().unwrap().click();
    h.run_steps(3);
    let c = h.state().p.canales.last().unwrap().clone();
    assert_eq!((c.id, c.instrumento.as_str(), h.state().p.canales.len()), (canal, "organo", canales + 1));

    // Arrastrar un clip lo mueve; un clic no; ⌘E lo parte en el cabezal; Suprimir lo borra.
    h.key_press(egui::Key::F7);
    h.run_steps(2);
    h.state_mut().mostrar(Vista::Arreglo);
    h.run_steps(2);
    let o = pos2(700.0, 260.0);
    let i = h.state().p.clips.iter().position(|c| c.patron == 3 && c.tick == 0).unwrap();
    let antes = h.state().p.clips[i].clone();
    let dentro = pos2(420.0, 265.0);
    for (a, b) in [(dentro, dentro), (dentro, dentro + egui::vec2(123.0, 98.0))] {
        h.hover_at(a);
        h.run_steps(1);
        h.drag_at(a);
        h.run_steps(1);
        h.hover_at(a + (b - a) * 0.5);
        h.run_steps(1);
        h.hover_at(b);
        h.run_steps(1);
        if a == b {
            assert_eq!(h.state().p.clips[i], antes);
        } else {
            foto(&mut h, "arrastrando");
        }
        h.drop_at(b);
        h.run_steps(2);
    }
    let c = h.state().p.clips[i].clone();
    assert_eq!((c.pista, c.tick, c.dur), (antes.pista + 2, 2 * 384, antes.dur));
    let n = h.state().p.clips.len();
    h.state_mut().partir(&std::collections::BTreeSet::from([i]), c.tick + 384);
    assert_eq!((h.state().p.clips.len(), h.state().p.clips[n].desfase), (n + 1, 384));
    h.key_press(egui::Key::Delete);
    h.run_steps(2);
    assert_eq!(h.state().p.clips.len(), n);
    // La tijera parte donde se hace clic.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Num3);
    h.run_steps(2);
    let _ = o;
    h.hover_at(pos2(455.0, 215.0));
    h.run_steps(1);
    h.drag_at(pos2(455.0, 215.0));
    h.run_steps(1);
    h.drop_at(pos2(455.0, 215.0));
    h.run_steps(2);
    assert_eq!(h.state().p.clips.len(), n + 1);
}

#[test]
fn la_interfaz_en_lua() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    use beethoven_daw_demo::interfaz::{self, Momento, Pieza};
    use std::collections::BTreeMap;
    let mut it = interfaz::cargar("return { nombre='x', controles={{'a','A',0,1,0.5}}, interfaz={ ancho=300, dibujar=function(d,c,m) d.perilla(10,10,20,'a') d.texto(0,0,string.format('%.1f', c.a)) end } }").unwrap().unwrap();
    let v = BTreeMap::from([("a".to_string(), 0.5f32)]);
    let p = it.cuadro(&v, &Momento::default());
    assert!(matches!(&p[..], [Pieza::Perilla { .. }, Pieza::Texto { s, .. }] if s == "0.5"));
    // Un control que no existe, un color malo, un ciclo sin fin o dibujar sin parar: error, y no se vuelve a correr.
    for malo in [
        "d.perilla(0,0,9,'nada')",
        "d.caja(0,0,1,1,'rojo')",
        "d.caja(0,0,1,1,'#ñandú')",
        "while true do end",
        "for i=1,9999 do d.circulo(0,0,1) end",
        "d.linea({0/0,1}, '#fff000')",
        "io.open('x')",
    ] {
        let f = format!("return {{ nombre='x', interfaz={{ dibujar=function(d,c,m) {malo} end }} }}");
        let mut it = interfaz::cargar(&f).unwrap().unwrap();
        it.cuadro(&v, &Momento::default());
        assert!(it.error.is_some(), "{malo}");
    }
    assert!(interfaz::cargar("return { nombre='x' }").unwrap().is_none());
    let mut h = abrir(Idioma::Es);
    h.state_mut().p.mezcla[4].efectos.push(beethoven_daw_demo::modelo::EfectoPuesto::nuevo("vozencaja"));
    h.state_mut().mostrar(Vista::Mezcla);
    h.state_mut().abrir_efecto(4, 0);
    h.run_steps(4);
    foto(&mut h, "vozencaja");
    // Arrastrar sobre la caja cambia su tamaño.
    let antes = h.state().p.mezcla[4].efectos[0].controles.clone();
    let o = pos2(430.0, 330.0);
    h.hover_at(o);
    h.run_steps(1);
    h.drag_at(o);
    h.run_steps(1);
    h.hover_at(o + egui::vec2(40.0, 10.0));
    h.run_steps(1);
    h.drop_at(o + egui::vec2(60.0, 20.0));
    h.run_steps(2);
    assert_ne!(h.state().p.mezcla[4].efectos[0].controles, antes);
}

/// Escrito a la carrera, como escribe la gente. Gasta de la cuenta de quien la corre.
#[test]
#[ignore]
fn entiende_aunque_se_escriba_mal() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = abrir(Idioma::Es);
    let (ia, modelo) = (std::env::var("IA").unwrap_or("Claude Code".into()), std::env::var("MODELO").unwrap_or_default());
    assert!(h.state_mut().usar_ia(&ia, &modelo), "no está {ia}");
    h.state_mut().mostrar(Vista::Asistente);
    for (que, tope) in [("boraa todo", 120), ("asme una kumbia d 8 compases kon bajo y guira, qe se oiga en la plailist", 420)] {
        h.state_mut().decir_ia(que);
        let t = std::time::Instant::now();
        while h.state().dicho().1 && t.elapsed().as_secs() < tope {
            h.run_steps(1);
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
        h.run_steps(3);
        println!("{:.0} s\n{}\n", t.elapsed().as_secs_f32(), h.state().dicho().0.join("\n"));
        assert!(!h.state().tocando(), "no se pidió reproducir");
        if que.starts_with("bora") {
            assert!(h.state().p.clips.is_empty() && h.state().p.patrones.iter().all(|p| p.notas.is_empty()));
        }
    }
    foto(&mut h, "asistente-cumbia");
    let p = &h.state().p;
    assert!(p.clips.iter().any(|c| p.patrones.iter().any(|q| q.id == c.patron && q.notas.len() >= 2)) && p.largo() >= 8 * 384, "{}", p.largo());
}

/// Con la IA instalada de verdad: gasta de la cuenta de quien la corre. `IA="Claude Code" MODELO=haiku cargo test … -- --ignored`
#[test]
#[ignore]
fn con_una_ia_de_verdad() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = abrir(Idioma::Es);
    let (ia, modelo) = (std::env::var("IA").unwrap_or("Claude Code".into()), std::env::var("MODELO").unwrap_or_default());
    assert!(h.state_mut().usar_ia(&ia, &modelo), "no está {ia}");
    h.state_mut().mostrar(Vista::Asistente);
    let n = h.state().p.clips.len();
    h.state_mut().decir_ia("Pon el tempo en 93 y corta en dos, en el compás 3, el clip del patrón Melodía que empieza en el compás 1. Nada más.");
    let t = std::time::Instant::now();
    while h.state().dicho().1 && t.elapsed().as_secs() < 180 {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(60));
    }
    h.run_steps(3);
    foto(&mut h, "asistente-real");
    println!("{:.0} s\n{}", t.elapsed().as_secs_f32(), h.state().dicho().0.join("\n"));
    let p = &h.state().p;
    assert_eq!((p.bpm, p.clips.len()), (93.0, n + 1));
    assert!(p.clips.iter().any(|c| c.patron == 3 && c.tick == 2 * 384 && c.desfase == 2 * 384));
}

fn hacer_bien(h: &mut Harness<'static, App>, n: &str, d: serde_json::Value) -> String {
    h.state_mut().hacer_ia(n, &d).unwrap_or_else(|e| panic!("{n}: {e}"))
}

#[test]
fn arrastrar_no_da_saltos() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = abrir(Idioma::Es);
    let o = pos2(420.0, 265.0);
    h.hover_at(o);
    h.run_steps(1);
    h.drag_at(o);
    h.run_steps(1);
    let mut antes = h.state().arrastre().x;
    let mut peor = 0f32;
    let mut lejos = 0f32;
    for i in 1..=200 {
        // Dos puntos por cuadro, como una mano despacio.
        let x = i as f32 * 2.0;
        h.hover_at(o + egui::vec2(x, 0.0));
        h.run_steps(1);
        let a = h.state().arrastre().x;
        if i > 4 {
            peor = peor.max((a - antes).abs());
            lejos = lejos.max((a - x).abs());
        }
        antes = a;
    }
    // Nunca se mueve mucho más que el ratón ni se separa de él más que el imán.
    assert!(peor < 6.5 && lejos < 7.0, "salto {peor}, separado {lejos}");
    h.drop_at(o + egui::vec2(400.0, 0.0));
    h.run_steps(2);
}

#[test]
fn abre_vacio_y_con_menu_de_ventanas() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = abrir(Idioma::Es);
    h.get_by_label("Ventana").click();
    h.run_steps(3);
    foto(&mut h, "menu-ventana");
    h.get_by_label_contains("Piano roll").click();
    h.run_steps(3);
    assert!(h.state().abierta(Vista::Notas));
    h.key_press(egui::Key::F4);
    h.run_steps(2);
    assert!(h.query_by_label("Órgano").is_none());
    foto(&mut h, "sin-biblioteca");
}

#[test]
fn voz_gruesa_con_su_fondo_y_su_logo() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    for i in Idioma::TODOS {
        let mut h = abrir(i);
        h.state_mut().mostrar(Vista::Mezcla);
        h.state_mut().p.mezcla[1].efectos.push(beethoven_daw_demo::modelo::EfectoPuesto::nuevo("vozgruesa"));
        h.state_mut().abrir_efecto(1, 0);
        h.run_steps(4);
        assert!(h.query_all_by_label_contains(if i == Idioma::Es { "Voz gruesa" } else { "Thick voice" }).count() > 0);
        foto(&mut h, &format!("{}-voz-gruesa", i.codigo()));
    }
}

#[test]
fn un_audio_se_mueve_arrastrandolo() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("bdd-mueve-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // También en una canción sin instrumentos: antes, cada cuadro soltaba lo que se arrastraba.
    for (segundos, nombre, sin_canales) in [(2, "corto.wav", false), (180, "cancion.wav", false), (2, "sola.wav", true)] {
        let mut h = abrir(Idioma::Es);
        h.state_mut().p.clips.clear();
        if sin_canales {
            h.state_mut().p.canales.clear();
        }
        h.run_steps(2);
        let ruta = dir.join(nombre);
        let x: Vec<[f32; 2]> = (0..48000 * segundos).map(|i| [(i as f32 * 0.05).sin() * 0.4; 2]).collect();
        beethoven_daw_demo::wav::escribir(&ruta, 48000, &x).unwrap();
        // Como al soltarlo desde el Finder.
        h.input_mut().dropped_files.push(std::sync::Arc::new(Soltado(ruta.clone())));
        h.run_steps(4);
        let antes = h.state().p.clips[0].clone();
        // A la mitad de lo alto y un poco adentro, sin tocar sus orillas.
        let o = pos2(355.0, 168.0);
        h.hover_at(o);
        h.run_steps(1);
        h.drag_at(o);
        h.run_steps(1);
        for i in 1..=10 {
            h.hover_at(o + egui::vec2(20.0 * i as f32, 10.0 * i as f32));
            h.run_steps(1);
        }
        h.drop_at(o + egui::vec2(200.0, 100.0));
        h.run_steps(3);
        let c = &h.state().p.clips[0];
        assert!(c.tick > antes.tick && c.pista > antes.pista, "{nombre}: antes {} {} ahora {} {}", antes.tick, antes.pista, c.tick, c.pista);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn editar_un_audio_sin_tocar_el_original() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = abrir(Idioma::Es);
    let r = std::env::temp_dir().join(format!("bdaw-editar-{}.wav", std::process::id()));
    let x: Vec<[f32; 2]> = (0..96000).map(|i| [(i as f32 * 0.03).sin() * 0.4 * (i as f32 / 9000.0).sin().abs(); 2]).collect();
    beethoven_daw_demo::wav::escribir(&r, 48000, &x).unwrap();
    let original = std::fs::read(&r).unwrap();
    h.state_mut().poner_audio(&r, None, 0, None).unwrap();
    h.state_mut().mostrar(Vista::Editor);
    h.run_steps(4);
    assert_eq!(h.state().audio_ed.cuadros(), 96000);
    // Seleccionar arrastrando y silenciar.
    let z = h.get_by_label("Onda del audio").rect();
    let punto = |k: f32| pos2(z.left() + z.width() * k, z.center().y);
    h.drag_at(punto(0.25));
    h.run_steps(1);
    h.hover_at(punto(0.5));
    h.run_steps(2);
    h.drop_at(punto(0.5));
    h.run_steps(3);
    let sel = h.state().audio_ed.sel.clone();
    assert!(sel.start.abs_diff(24000) < 300 && sel.end.abs_diff(48000) < 300, "{sel:?}");
    foto(&mut h, "editor");
    let antes = h.state().p.clone();
    h.get_by_label("Silenciar").click();
    h.run_steps(4);
    let a = h.state().p.clips.last().unwrap().audio.clone().unwrap();
    assert_ne!(a.ruta, r.to_string_lossy());
    let w = beethoven_daw_demo::wav::leer(std::path::Path::new(&a.ruta)).unwrap();
    assert!(w.izq[sel.clone()].iter().all(|v| *v == 0.0) && w.izq[..sel.start].iter().any(|v| *v != 0.0));
    // Quitar acorta el clip.
    h.get_by_label("Quitar").click();
    h.run_steps(4);
    assert!(h.state().p.clips.last().unwrap().dur < antes.clips.last().unwrap().dur);
    assert!(h.state().audio_ed.cuadros().abs_diff(96000 - sel.len()) < 100);
    // Deshacer regresa y el archivo de siempre no cambió.
    h.state_mut().deshacer();
    h.state_mut().deshacer();
    h.run_steps(3);
    assert_eq!(h.state().p, antes);
    assert_eq!(h.state().audio_ed.cuadros(), 96000);
    assert_eq!(std::fs::read(&r).unwrap(), original);
    let _ = std::fs::remove_file(&r);
}

#[test]
fn los_efectos_con_su_propio_diseno() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    for g in beethoven_daw_demo::biblioteca::Biblioteca::nueva(48000.0, None).de_tipo(beethoven_daw_demo::lua::Tipo::Efecto) {
        let clave = g.clave.as_str();
        let mut h = abrir(Idioma::Es);
        h.state_mut().p.mezcla[1].efectos.push(beethoven_daw_demo::modelo::EfectoPuesto::nuevo(clave));
        h.state_mut().fx_abiertos = vec![(1, 0)];
        h.run_steps(5);
        foto(&mut h, clave);
    }
}

#[test]
fn los_sintetizadores() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    for clave in ["tresondas", "frecuencias", "muralla", "acido", "retumbo", "pulsada", "barras", "muestreador", "pads", "cortador"] {
        let mut h = abrir(Idioma::Es);
        let canal = h.state().canal;
        h.state_mut().p.canales.iter_mut().find(|c| c.id == canal).unwrap().instrumento = clave.into();
        h.state_mut().mostrar(Vista::Canal);
        h.run_steps(3);
        // A pantalla completa para ver todo su diseño.
        h.get_by_label("Agrandar").click();
        h.run_steps(4);
        foto(&mut h, clave);
    }
}

/// Que ninguna ventana de fábrica falle al dibujarse, en los dos idiomas.
#[test]
fn ninguna_interfaz_falla() {
    let b = beethoven_daw_demo::biblioteca::Biblioteca::nueva(48000.0, None);
    for g in &b.guiones {
        let Some(mut i) = beethoven_daw_demo::interfaz::cargar_con(&g.fuente, g.archivos.clone()).unwrap() else {
            assert!(
                ["muestra", "ecualizador"].contains(&g.clave.as_str()) || g.ficha.tipo == beethoven_daw_demo::lua::Tipo::Herramienta,
                "{} sin ventana",
                g.clave
            );
            continue;
        };
        let v = g.ficha.controles.iter().map(|c| (c.clave.clone(), c.valor)).collect();
        let m = beethoven_daw_demo::interfaz::Momento {
            nivel: 0.3,
            tocando: true,
            activo: true,
            tiempo: 2.0,
            onda: vec![0.2; 256],
            espectro: vec![0.4; 64],
            huella: vec![0.5; 256],
            cortes: vec![0.0, 0.5],
            nombres: g.ficha.controles.iter().map(|c| (c.clave.clone(), c.nombre.ver().to_string())).collect(),
            ..Default::default()
        };
        assert!(!i.cuadro(&v, &m).is_empty() && i.error.is_none(), "{}: {:?}", g.clave, i.error);
    }
}

/// Soltar un plugin en la ventana, usarlo y verlo en la lista, sin reiniciar.
#[test]
fn soltar_un_plugin_y_usarlo() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    let d = std::env::temp_dir().join(format!("bdaw-soltar-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let c = d.join("mi distorsion");
    std::fs::create_dir_all(&c).unwrap();
    let guion = include_str!("../plantillas/efecto.lua").replace("Mi efecto", "Distorsión ajena").replace("autor = \"\"", "autor = \"Ana\"");
    std::fs::write(c.join("mi distorsion.lua"), guion).unwrap();
    let mut h = abrir(Idioma::Es);
    h.state_mut().bib.carpeta = Some(d.join("Lua"));
    let ctx = h.ctx.clone();
    h.state_mut().soltar(&ctx, &c, None);
    h.run_steps(4);
    assert!(h.state().bib.ver("mi_distorsion").is_some_and(|g| g.propio));
    h.state_mut().p.mezcla[1].efectos.push(beethoven_daw_demo::modelo::EfectoPuesto::nuevo("mi_distorsion"));
    h.state_mut().fx_abiertos = vec![(1, 0)];
    h.run_steps(5);
    foto(&mut h, "plugin-ajeno");
    // Un cambio en el disco entra solo mientras el programa corre.
    let f = d.join("Lua/mi_distorsion/mi_distorsion.lua");
    let s = std::fs::read_to_string(&f).unwrap().replace("version = \"1.0\"", "version = \"2.0.1\"");
    std::fs::write(&f, s).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1000));
    h.run_steps(4);
    assert_eq!(h.state().bib.ver("mi_distorsion").unwrap().ficha.version, "2.0.1");
    let _ = std::fs::remove_dir_all(&d);
}

/// La ventana de un plugin en otro programa.
#[test]
fn la_ventana_en_otro_programa() {
    let _g = UNA.lock().unwrap_or_else(|e| e.into_inner());
    idioma::poner(Idioma::Es);
    for k in ["compresor", "fuego", "eco", "coro", "ecualizador", "convolucion", "barras", "afinador"] {
        let n = std::sync::Arc::new(beethoven_daw_demo::instancia::Instancia::nueva(k, 48000.0, Default::default()).unwrap());
        let ctx = egui::Context::default();
        let t = beethoven_daw_demo::ui::externa::Externa::nueva(&ctx, &n).tam(&n);
        let (mut e, q) = (None, n.clone());
        let mut h = Harness::builder().with_size(t).wgpu().build_ui(move |ui| {
            let e = e.get_or_insert_with(|| beethoven_daw_demo::ui::externa::Externa::nueva(ui.ctx(), &q));
            e.pintar(ui, &q);
        });
        h.run_steps(2);
        // Suena mientras se mira: golpes que se apagan, con graves y agudos.
        let x: Vec<f32> = (0..192_000)
            .map(|i| {
                let t = i as f32 / 48000.0;
                (0.4 * (691.0 * t).sin() + 0.3 * (2400.0 * t).sin() + 0.2 * (19000.0 * t).sin()) * (-((i % 18000) as f32) / 5000.0).exp()
            })
            .collect();
        for (b, c) in x.chunks(1024).enumerate() {
            let (mut a, mut d) = (vec![0.0; c.len()], vec![0.0; c.len()]);
            let t = beethoven_daw_demo::plugin::Tiempo { bpm: 120.0, pos: b as f64 * 0.04, tocando: true, sr: 48000.0 };
            let ev: &[beethoven_daw_demo::instancia::En] =
                if b == 100 { &[(0, beethoven_daw_demo::plugin::Evento::Nota { tono: 48, fuerza: 1.0 })] } else { &[] };
            n.procesar(ev, [c, c], [&[], &[]], [&mut a, &mut d], &t);
        }
        h.run_steps(1);
        if let Ok(c) = std::env::var("CAPTURAS") {
            h.render().unwrap().save(format!("{c}/externa-{k}.png")).unwrap();
        }
    }
}
