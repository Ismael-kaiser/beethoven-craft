//! Un plugin suelto, como va dentro de un VST3 o CLAP.
use beethoven_daw_demo::biblioteca::Biblioteca;
use beethoven_daw_demo::instancia::{Instancia, disponibles};
use beethoven_daw_demo::medir::correr_con;
use beethoven_daw_demo::plugin::{Evento, Tiempo};

const SR: f32 = 48000.0;

fn pasar(n: &Instancia, x: &[f32], bloque: usize) -> Vec<f32> {
    let (mut a, mut b) = (vec![0.0; x.len()], vec![0.0; x.len()]);
    let mut i = 0;
    while i < x.len() {
        let j = (i + bloque).min(x.len());
        let t = Tiempo { bpm: 120.0, pos: i as f64 / 24000.0, tocando: true, sr: SR as f64 };
        n.procesar(&[], [&x[i..j], &x[i..j]], [&[], &[]], [&mut a[i..j], &mut b[i..j]], &t);
        i = j;
    }
    a
}

#[test]
fn suena_igual_que_en_el_daw() {
    let x: Vec<f32> = (0..48000).map(|i| (i as f32 * 0.05).sin() * 0.4 * ((i / 6000) % 2) as f32).collect();
    let mut b = Biblioteca::nueva(SR, None);
    for k in ["compresor", "fuego", "ecualizador", "reverb", "convolucion", "filtro", "coro", "recortador"] {
        let g = b.ver(k).unwrap();
        let r = g.receta.clone().unwrap();
        let ctl: Vec<f32> = r.controles.iter().map(|c| c.valor).collect();
        let h = if r.con_huella() { b.huella(&beethoven_daw_demo::modelo::EfectoPuesto::nuevo(k)) } else { None };
        let y = correr_con(&r, &ctl, &[x.clone(), x.clone()], h.as_deref());
        let n = Instancia::nueva(k, SR, Default::default()).unwrap();
        for bloque in [64, 512, 37] {
            let n = if bloque == 64 { &n } else { &Instancia::nueva(k, SR, Default::default()).unwrap() };
            let z = pasar(n, &x, bloque);
            let peor = y[0].iter().zip(&z).skip(2000).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
            assert!(peor < 0.02, "{k} con bloques de {bloque}: {peor}");
        }
    }
}

#[test]
fn guarda_mueve_y_se_apaga() {
    let n = Instancia::nueva("balance", SR, Default::default()).unwrap();
    let x = vec![0.5; 9600];
    let k = n.receta.controles.iter().position(|c| c.clave == "volumen").unwrap();
    n.poner(k, 0.5);
    let y = pasar(&n, &x, 256);
    assert!((y[9000] - 0.25).abs() < 1e-3, "{}", y[9000]);
    n.poner_al_dia();
    let m = Instancia::nueva("balance", SR, Default::default()).unwrap();
    assert!(m.de_texto(&n.a_texto()) && m.valor(k) == 0.5 && !m.de_texto(b"basura"));
    assert!((pasar(&m, &x, 256)[9000] - 0.25).abs() < 1e-3);
    n.prender(false);
    assert_eq!(pasar(&n, &x, 256)[9000], 0.5);
    // El lateral llega por su entrada.
    let n = Instancia::nueva("ducker", SR, serde_json::from_str(r#"{"controles":{"pista":2,"cuanto":1}}"#).unwrap()).unwrap();
    let (mut a, mut b) = (vec![0.0; 9600], vec![0.0; 9600]);
    let t = Tiempo { bpm: 120.0, pos: 0.0, tocando: true, sr: SR as f64 };
    n.procesar(&[], [&x, &x], [&x, &x], [&mut a, &mut b], &t);
    assert!(a[9000] < 0.1, "{}", a[9000]);
}

#[test]
fn los_instrumentos_tocan_en_su_momento() {
    for k in ["barras", "monoanalogico", "tresondas"] {
        let n = Instancia::nueva(k, SR, Default::default()).unwrap();
        let (mut a, mut b) = (vec![0.0; 48000], vec![0.0; 48000]);
        let t = Tiempo { bpm: 120.0, pos: 0.0, tocando: false, sr: SR as f64 };
        let ev = [(5000, Evento::Nota { tono: 60, fuerza: 1.0 }), (20000, Evento::Suelta { tono: 60 })];
        n.procesar(&ev, [&[], &[]], [&[], &[]], [&mut a, &mut b], &t);
        let pico = |x: &[f32]| x.iter().fold(0f32, |m, v| m.max(v.abs()));
        assert!(
            pico(&a[..5000]) == 0.0 && pico(&a[5000..19000]) > 0.02 && pico(&a[45000..]) < 0.01,
            "{k}: {} {}",
            pico(&a[5000..19000]),
            pico(&a[45000..])
        );
    }
}

#[test]
fn se_ofrecen_todos_menos_los_controladores() {
    let v = disponibles();
    assert!(v.len() >= 56 && v.iter().all(|x| !x.0.starts_with("ctl") && x.0 != "muestra"), "{}", v.len());
    let nombres: std::collections::BTreeSet<_> = v.iter().map(|x| x.1.nombre.es.clone()).collect();
    assert_eq!(nombres.len(), v.len(), "dos se llaman igual");
}
