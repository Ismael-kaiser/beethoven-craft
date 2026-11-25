//! Que nada se quede sonando: ni al parar, ni cuando ya se acabó el clip.
use beethoven_daw_demo::biblioteca::Biblioteca;
use beethoven_daw_demo::lua::Tipo;
use beethoven_daw_demo::modelo::*;
use beethoven_daw_demo::motor::*;
use std::collections::BTreeMap;

const SR: f32 = 48000.0;

fn tono() -> String {
    let r = std::env::temp_dir().join(format!("bdaw-colas-{}.wav", std::process::id()));
    if !r.exists() {
        beethoven_daw_demo::wav::escribir(&r, 48000, &(0..48000).map(|i| [(i as f32 * 0.04).sin() * 0.5; 2]).collect::<Vec<_>>()).unwrap();
    }
    r.to_string_lossy().to_string()
}

/// Un clip de un segundo en una canción larga, con ese efecto.
fn proyecto(clave: &str, v: &BTreeMap<String, f32>) -> Proyecto {
    let mut p = Proyecto { bpm: 120.0, ..Default::default() };
    p.mezcla.iter_mut().for_each(|m| {
        m.efectos.clear();
        m.volumen = 1.0;
    });
    let a = ClipAudio { ruta: tono(), nombre: "x".into(), mezcla: 1, ..Default::default() };
    p.clips.push(Clip { patron: 0, pista: 0, tick: 0, dur: p.pulsos(1.0), desfase: 0, audio: Some(a) });
    p.clips.push(Clip::de_patron(p.patrones[0].id, 1, p.pulsos(40.0), 96));
    let mut e = EfectoPuesto::nuevo(clave);
    e.controles = v.clone();
    p.mezcla[1].efectos.push(e);
    p
}

fn pico(m: &mut Motor, seg: f32) -> f32 {
    let mut x = [[0.0f32; 2]; 480];
    (0..(seg * 100.0) as usize).fold(0.0, |k, _| {
        m.tocar(&mut x);
        x.iter().flatten().fold(k, |a, v| a.max(v.abs()))
    })
}

#[test]
fn al_parar_todo_calla_y_al_acabar_el_clip_tambien() {
    let mut b = Biblioteca::nueva(SR, None);
    let todos: Vec<_> = b.de_tipo(Tipo::Efecto).map(|g| (g.clave.clone(), g.ficha.sostiene, g.ficha.presets.clone())).collect();
    for (k, sostiene, presets) in todos {
        for v in presets.iter().map(|p| p.1.clone()).chain([Default::default()]) {
            let p = proyecto(&k, &v);
            let (mut m, mando) = Motor::nuevo(Escena::nueva(&p, &mut b, Modo::Cancion));
            mando.mandar(Orden::Tocar(true));
            pico(&mut m, 0.9);
            mando.mandar(Orden::Tocar(false));
            pico(&mut m, 0.4);
            assert_eq!(pico(&mut m, 0.5), 0.0, "{k} {v:?} sigue al parar");
            // Lo congelado a propósito sí se queda mientras la canción sigue.
            if sostiene {
                continue;
            }
            mando.mandar(Orden::Ir(0.0));
            mando.mandar(Orden::Tocar(true));
            assert!(pico(&mut m, 1.1) > 0.005, "{k}: vuelve a sonar");
            pico(&mut m, 6.0);
            let queda = pico(&mut m, 0.5);
            assert!(queda < 0.06, "{k} {v:?} sigue {queda} a los 6 segundos");
        }
        b.soltar_vivos();
    }
}

#[test]
fn uno_ajeno_que_silba_se_calla_solo() {
    let d = std::env::temp_dir().join(format!("bdaw-colas-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for (n, mas, sigue) in [("silba", "", false), ("sostiene", "sostiene = true,", true)] {
        let l = d.join(format!("{n}.lua"));
        std::fs::write(
            &l,
            format!("return {{ nombre = '{n}', tipo = 'efecto', {mas} sonido = function(e, c) return e.izq + seno(500) * 0.2, e.der end }}"),
        )
        .unwrap();
        let mut b = Biblioteca::nueva(SR, Some(d.join("Lua")));
        let (k, _) = b.instalar(&l).unwrap();
        let p = proyecto(&k, &Default::default());
        let (mut m, mando) = Motor::nuevo(Escena::nueva(&p, &mut b, Modo::Cancion));
        mando.mandar(Orden::Tocar(true));
        pico(&mut m, 7.0);
        let q = pico(&mut m, 0.5);
        assert_eq!(q > 0.1, sigue, "{n}: {q}");
        // Vuelve en cuanto entra algo.
        mando.mandar(Orden::Ir(0.0));
        assert!(pico(&mut m, 0.5) > 0.4, "{n}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn parado_se_puede_tocar_con_su_reverb() {
    let mut p = proyecto("reverb", &[("mezcla".to_string(), 1.0)].into());
    let id = p.agregar_canal("pulsada", "x");
    p.canales[0].pista = 1;
    p.canales[0].notas = Some("arpegiador".into());
    let mut b = Biblioteca::nueva(SR, None);
    let (mut m, mando) = Motor::nuevo(Escena::nueva(&p, &mut b, Modo::Cancion));
    mando.mandar(Orden::Tocar(true));
    pico(&mut m, 0.5);
    mando.mandar(Orden::Tocar(false));
    pico(&mut m, 1.0);
    assert_eq!(pico(&mut m, 0.3), 0.0);
    mando.mandar(Orden::Nota { canal: id, tono: 60, fuerza: 1.0 });
    assert!(pico(&mut m, 1.0) > 0.02);
    mando.mandar(Orden::Soltar { canal: id, tono: 60 });
    pico(&mut m, 0.6);
    // Su cola sigue, y al rato se acaba.
    assert!(pico(&mut m, 0.2) > 1e-4);
    pico(&mut m, 8.0);
    assert!(pico(&mut m, 0.2) < 1e-3);
    // Un arpegio que venía de la canción se suelta al parar.
    let mut p = proyecto("balance", &Default::default());
    p.agregar_canal("barras", "x");
    p.canales[0].notas = Some("arpegiador".into());
    p.patrones[0].notas.insert(p.canales[0].id, vec![Nota { tick: 0, dur: PPQ * 64, tono: 60, fuerza: 1.0 }]);
    p.patrones[0].compases = 32;
    let (mut m, mando) = Motor::nuevo(Escena::nueva(&p, &mut b, Modo::Patron(p.patrones[0].id)));
    mando.mandar(Orden::Tocar(true));
    assert!(pico(&mut m, 1.0) > 0.01);
    mando.mandar(Orden::Tocar(false));
    pico(&mut m, 0.5);
    assert_eq!(pico(&mut m, 3.0), 0.0);
}
