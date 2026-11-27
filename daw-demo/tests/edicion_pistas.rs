//! La curva y el tirador de volumen de los audios, el color de las pistas, los grupos y las clases del Mixer, y que
//! los proyectos de antes abran igual. `CAPTURAS=/carpeta` guarda imágenes.
use beethoven_daw_demo::biblioteca::Biblioteca;
use beethoven_daw_demo::idioma::{self, Idioma};
use beethoven_daw_demo::modelo::*;
use beethoven_daw_demo::motor::*;
use beethoven_daw_demo::ui::{App, Vista};
use egui_kittest::Harness;

const SR: f32 = 48000.0;

fn tono() -> String {
    let r = std::env::temp_dir().join(format!("bdaw-curva-{}.wav", std::process::id()));
    if !r.exists() {
        beethoven_daw_demo::wav::escribir(&r, 48000, &(0..96000).map(|i| [(i as f32 * 0.04).sin() * 0.5; 2]).collect::<Vec<_>>()).unwrap();
    }
    r.to_string_lossy().to_string()
}

/// Nivel de cada décima de segundo durante `seg`.
fn niveles(p: &Proyecto, seg: f32) -> Vec<f32> {
    let mut b = Biblioteca::nueva(SR, None);
    let (mut m, mando) = Motor::nuevo(Escena::nueva(p, &mut b, Modo::Cancion));
    mando.mandar(Orden::Tocar(true));
    let mut x = [[0.0f32; 2]; 480];
    (0..(seg * 10.0) as usize)
        .map(|_| {
            let mut e = 0.0;
            for _ in 0..10 {
                m.tocar(&mut x);
                e += x.iter().map(|v| v[0] * v[0]).sum::<f32>();
            }
            (e / 4800.0).sqrt()
        })
        .collect()
}

fn con_audio(a: ClipAudio) -> Proyecto {
    let mut p = Proyecto { bpm: 120.0, ..Default::default() };
    p.mezcla.iter_mut().for_each(|m| {
        m.efectos.clear();
        m.volumen = 1.0;
    });
    p.clips.push(Clip { patron: 0, pista: 0, tick: 0, dur: p.pulsos(2.0), desfase: 0, audio: Some(a) });
    p
}

#[test]
fn la_curva_de_volumen_suena() {
    assert_eq!(envolvente(&[], 3.0), 1.0);
    let e = [[1.0, 0.0], [2.0, 2.0]];
    assert_eq!((envolvente(&e, 0.0), envolvente(&e, 1.5), envolvente(&e, 9.0)), (0.0, 1.0, 2.0));
    let a = ClipAudio { ruta: tono(), nombre: "x".into(), mezcla: 1, ..Default::default() };
    let plano = niveles(&con_audio(a.clone()), 2.0);
    // Sube de nada a todo en el primer segundo y se queda.
    let curva = niveles(&con_audio(ClipAudio { envolvente: vec![[0.0, 0.0], [1.0, 1.0]], ..a.clone() }), 2.0);
    println!("plano {:.3} · curva al inicio {:.3}, a la mitad {:.3}, al final {:.3}", plano[15], curva[0], curva[5], curva[15]);
    assert!(curva[0] < plano[0] * 0.15 && (curva[5] / plano[5] - 0.55).abs() < 0.15 && (curva[15] / plano[15] - 1.0).abs() < 0.02);
    // El tirador: el volumen de todo el audio.
    let medio = niveles(&con_audio(ClipAudio { volumen: 0.5, ..a.clone() }), 2.0);
    assert!((medio[15] / plano[15] - 0.5).abs() < 0.02);
    // Recortado desde el inicio, la curva sigue en el mismo lugar del archivo.
    let recortado = niveles(&con_audio(ClipAudio { inicio: 0.5, envolvente: vec![[0.0, 0.0], [1.0, 1.0]], ..a }), 1.0);
    assert!(recortado[0] / plano[0] > 0.4 && recortado[0] / plano[0] < 0.7, "{}", recortado[0] / plano[0]);
}

#[test]
fn los_proyectos_de_antes_abren_igual() {
    // Uno de la 0.12.4: 17 pistas de mezcla y los envíos en 13 a 16.
    let mut p = Proyecto::default();
    let mut m: Vec<serde_json::Value> =
        p.mezcla.iter().take(13).chain(p.mezcla.iter().skip(PRIMER_ENVIO)).map(|m| serde_json::to_value(m).unwrap()).collect();
    m[14]["nombre"] = "Mi eco".into();
    m[5]["nombre"] = "Voz".into();
    p.agregar_canal("bombo", "Bombo");
    p.canales[0].pista = 14;
    let mut v = serde_json::to_value(&p).unwrap();
    v["mezcla"] = m.into();
    for k in ["grupos", "clases", "vistas", "sig"] {
        v.as_object_mut().unwrap().remove(k);
    }
    let q = Proyecto::de_json(&v.to_string()).unwrap();
    assert_eq!(q.mezcla.len(), PISTAS_MEZCLA);
    assert_eq!(q.mezcla[5].nombre, "Voz");
    assert_eq!(q.mezcla[PRIMER_ENVIO + 1].nombre, "Mi eco");
    assert_eq!(q.canales[0].pista, PRIMER_ENVIO + 1, "lo que iba a un envío lo sigue");
    assert!(q.vistas >= 12 && q.grupos.is_empty() && q.clases.is_empty());
    // Y lo nuevo se guarda y se abre.
    let mut n = q.clone();
    n.pistas[0].color = Some(COLORES[2]);
    let g = n.id_nuevo();
    n.grupos.push(GrupoMezcla { id: g, nombre: "Bat".into(), color: COLORES[1], plegado: true });
    n.mezcla[1].grupo = Some(g);
    n.vistas = 20;
    assert_eq!(Proyecto::de_json(&serde_json::to_string(&n).unwrap()).unwrap(), n);
}

#[test]
fn las_clases_comparten_sus_efectos() {
    let mut p = Proyecto::default();
    p.mezcla.iter_mut().for_each(|m| m.efectos.clear());
    p.mezcla[3].efectos.push(EfectoPuesto::nuevo("eco"));
    let id = p.id_nuevo();
    p.clases.push(ClaseMezcla { id, nombre: "Voces".into(), color: COLORES[0], efectos: vec![EfectoPuesto::nuevo("coro")] });
    for i in [1, 2, 3] {
        p.mezcla[i].clase = Some(id);
    }
    p.igualar_clases(None);
    for i in [1, 2, 3] {
        assert!(p.mezcla[i].efectos[0].de_clase && p.mezcla[i].efectos[0].efecto == "coro", "pista {i}");
    }
    assert_eq!(p.mezcla[3].efectos[1].efecto, "eco", "lo suyo se queda después");
    // Mover un control en una pista lo mueve en todas.
    let antes = p.clone();
    p.mezcla[2].efectos[0].controles.insert("mezcla".into(), 0.25);
    p.igualar_clases(Some(&antes));
    for i in [1, 2, 3] {
        assert_eq!(p.mezcla[i].efectos[0].controles.get("mezcla"), Some(&0.25));
    }
    // Suena igual en todas: cada una con su copia.
    let mut b = Biblioteca::nueva(SR, None);
    let _ = Escena::nueva(&p, &mut b, Modo::Cancion);
    // Al salir de la clase se queda solo con lo suyo.
    p.mezcla[3].clase = None;
    p.igualar_clases(None);
    assert_eq!(p.mezcla[3].efectos.len(), 1);
    assert_eq!(p.mezcla[3].efectos[0].efecto, "eco");
}

#[test]
fn se_ve_en_la_ventana() {
    idioma::poner(Idioma::Es);
    let mut h = Harness::builder().with_size([1360.0, 820.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true));
    h.run_steps(3);
    {
        let a = h.state_mut();
        let p = &mut a.p;
        p.pistas[0].color = Some(COLORES[3]);
        p.pistas[0].nombre = "Voz".into();
        let audio = ClipAudio {
            ruta: tono(),
            nombre: "voz".into(),
            mezcla: 1,
            envolvente: vec![[0.2, 0.3], [0.9, 1.6], [1.6, 0.8]],
            volumen: 0.7,
            ..Default::default()
        };
        p.clips.push(Clip { patron: 0, pista: 0, tick: 0, dur: p.pulsos(2.0), desfase: 0, audio: Some(audio) });
        let g = p.id_nuevo();
        p.grupos.push(GrupoMezcla { id: g, nombre: "Batería".into(), color: COLORES[0], plegado: true });
        let c = p.id_nuevo();
        p.clases.push(ClaseMezcla { id: c, nombre: "Coros".into(), color: COLORES[2], efectos: vec![EfectoPuesto::nuevo("coro")] });
        for i in [2, 3, 4] {
            p.mezcla[i].grupo = Some(g);
        }
        for i in [5, 6] {
            p.mezcla[i].clase = Some(c);
            p.mezcla[i].color = Some(COLORES[5]);
        }
        let g2 = p.id_nuevo();
        p.grupos.push(GrupoMezcla { id: g2, nombre: "Voces".into(), color: COLORES[4], plegado: false });
        p.mezcla[7].grupo = Some(g2);
        p.mezcla[8].grupo = Some(g2);
    }
    h.run_steps(4);
    assert!(h.state().p.mezcla[5].efectos.first().is_some_and(|e| e.de_clase));
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/curva.png")).unwrap();
    }
    h.state_mut().mostrar(Vista::Mezcla);
    h.run_steps(4);
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/grupos.png")).unwrap();
    }
}
