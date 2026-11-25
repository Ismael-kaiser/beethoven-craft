//! Lo que reemplaza a los plugins de antes: convolución, vocoder, entrada lateral, controladores, pads y cortador.
use beethoven_daw_demo::biblioteca::Biblioteca;
use beethoven_daw_demo::impulso::Impulso;
use beethoven_daw_demo::modelo::*;
use beethoven_daw_demo::motor::{Modo, PRIMER_PAD, a_audio};
use beethoven_daw_demo::wav;
use std::f32::consts::TAU;

const SR: f32 = 48000.0;

fn bib() -> Biblioteca {
    Biblioteca::nueva(SR, None)
}

fn temporal(n: &str, x: &[[f32; 2]]) -> String {
    let r = std::env::temp_dir().join(format!("bdaw-nuevos-{}-{n}.wav", std::process::id()));
    wav::escribir(&r, 48000, x).unwrap();
    r.to_string_lossy().to_string()
}

fn seno(hz: f32, seg: f32) -> Vec<[f32; 2]> {
    (0..(SR * seg) as usize).map(|i| [(TAU * hz * i as f32 / SR).sin() * 0.5; 2]).collect()
}

fn rms(x: &[[f32; 2]]) -> f32 {
    (x.iter().map(|v| v[0] * v[0]).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

/// Cuánto hay de esa frecuencia.
fn cuanto(x: &[[f32; 2]], hz: f32) -> f32 {
    let n = x.len();
    let (a, b) = x.iter().enumerate().fold((0.0f64, 0.0f64), |(a, b), (i, v)| {
        let w = std::f64::consts::TAU * hz as f64 * i as f64 / SR as f64;
        let h = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos();
        (a + v[0] as f64 * h * w.cos(), b + v[0] as f64 * h * w.sin())
    });
    (a.hypot(b) * 4.0 / n as f64) as f32
}

/// Un proyecto con un audio en la pista 1 del Mixer (y otro, si se da, en la 2).
fn con_audio(a: &str, b: Option<&str>, seg: f32) -> Proyecto {
    let mut p = Proyecto { bpm: 120.0, ..Default::default() };
    for (i, r) in [Some(a), b].into_iter().enumerate() {
        if let Some(r) = r {
            let audio = ClipAudio { ruta: r.into(), nombre: "x".into(), mezcla: i + 1, ..Default::default() };
            p.clips.push(Clip { patron: 0, pista: i, tick: 0, dur: p.pulsos(seg), desfase: 0, audio: Some(audio) });
        }
    }
    p.mezcla.iter_mut().for_each(|m| {
        m.efectos.clear();
        m.volumen = 1.0;
    });
    p
}

#[test]
fn la_convolucion_es_exacta_y_sin_retraso() {
    let mut s = 5u32;
    let mut azar = || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as f32 / u32::MAX as f32 - 0.5
    };
    let h: Vec<f32> = (0..3000).map(|i| azar() * (-(i as f32) / 900.0).exp()).collect();
    let x: Vec<f32> = (0..6000).map(|_| azar()).collect();
    let imp = Impulso::nuevo([h.clone(), h.clone()], SR);
    // La misma cuenta, a mano (con la fuerza y el final que les pone).
    let n = h.len();
    let mut g = h.clone();
    let baja = (n / 8).clamp(1, (SR * 0.05) as usize + 1);
    (0..baja).for_each(|k| g[n - 1 - k] *= k as f32 / baja as f32);
    let fuerza = 1.0 / g.iter().map(|v| v * v).sum::<f32>().sqrt();
    let (mut mem, mut y) = (vec![0.0; Impulso::memoria(SR)], vec![0.0; x.len()]);
    // Con bloques disparejos, como los del motor.
    let (mut i, mut largo) = (0, 1);
    while i < x.len() {
        let j = (i + largo).min(x.len());
        imp.tocar(0, &x[i..j], &mut y[i..j], &mut mem, SR);
        i = j;
        largo = largo % 64 + 7;
    }
    for t in (0..x.len()).step_by(37) {
        let mano: f32 = (0..n.min(t + 1)).map(|k| g[k] * x[t - k]).sum::<f32>() * fuerza;
        assert!((mano - y[t]).abs() < 2e-3, "{t}: {mano} {}", y[t]);
    }
}

#[test]
fn cada_lugar_suena_distinto_y_carga_un_audio_tuyo() {
    let mut golpe = vec![[0.0f32; 2]; 96000];
    golpe[100] = [0.8, 0.8];
    let r = temporal("golpe", &golpe);
    let huella = temporal("huella", &seno(1000.0, 0.5));
    let mut colas = vec![];
    for espacio in 0..9 {
        let mut p = con_audio(&r, None, 2.0);
        let mut e = EfectoPuesto::nuevo("convolucion");
        e.controles = [("espacio".to_string(), espacio.min(7) as f32), ("mezcla".into(), 1.0), ("seco".into(), 0.0)].into();
        if espacio == 8 {
            e.muestra = Some(huella.clone());
        }
        p.mezcla[1].efectos.push(e);
        let y = a_audio(&p, &mut bib(), Modo::Cancion);
        assert!(rms(&y[..48000]) > 1e-4, "{espacio}");
        colas.push((rms(&y[12000..24000]), cuanto(&y[..24000], 1000.0)));
    }
    assert!(colas[3].0 > colas[0].0 * 5.0, "la catedral dura más que la cabina: {colas:?}");
    assert!(colas[8].1 > colas[1].1 * 2.0, "suena a su audio: {colas:?}");
}

#[test]
fn el_compresor_lateral_oye_otra_pista() {
    let (a, b) = (temporal("bajo", &seno(100.0, 2.0)), temporal("bombo", &[vec![[0.0; 2]; 48000], seno(60.0, 1.0)].concat()));
    let mut p = con_audio(&a, Some(&b), 2.0);
    // El bombo no llega a la salida, pero sí se oye como señal.
    p.mezcla[2].volumen = 0.0;
    let mut e = EfectoPuesto::nuevo("ducker");
    e.controles = [("pista".to_string(), 2.0), ("cuanto".into(), 1.0)].into();
    p.mezcla[1].efectos.push(e);
    let y = a_audio(&p, &mut bib(), Modo::Cancion);
    let (antes, luego) = (rms(&y[10000..40000]), rms(&y[60000..90000]));
    assert!(antes > 0.2 && luego < antes * 0.35, "{antes} {luego}");
}

#[test]
fn el_vocoder_moldea_la_otra_pista() {
    let voz: Vec<[f32; 2]> =
        (0..96000).map(|i| if i < 48000 { [0.4 * (i as f32 * 0.04).sin() + 0.2 * (i as f32 * 0.3).sin(); 2] } else { [0.0; 2] }).collect();
    let (a, b) = (temporal("voz", &voz), temporal("porta", &seno(300.0, 2.0)));
    let mut p = con_audio(&a, Some(&b), 2.0);
    p.mezcla[2].volumen = 0.0;
    let mut e = EfectoPuesto::nuevo("vocoder");
    e.controles = [("pista".to_string(), 2.0), ("soplo".into(), 0.0)].into();
    p.mezcla[1].efectos.push(e);
    let y = a_audio(&p, &mut bib(), Modo::Cancion);
    // Suena la portadora mientras hay voz; al callar, nada.
    assert!(cuanto(&y[10000..40000], 300.0) > 0.005, "{}", cuanto(&y[10000..40000], 300.0));
    assert!(rms(&y[70000..90000]) < rms(&y[10000..40000]) * 0.05);
}

#[test]
fn un_controlador_mueve_una_perilla() {
    let a = temporal("tono", &seno(440.0, 4.0));
    let mut p = con_audio(&a, None, 4.0);
    // Un LFO cuadrado de dos negras mueve el volumen: un segundo suena y medio... a 120, medio y medio.
    let mut lfo = EfectoPuesto::nuevo("ctllfo");
    lfo.id = 77;
    lfo.controles = [("forma".to_string(), 3.0), ("negras".into(), 4.0)].into();
    p.mezcla[3].efectos.push(lfo);
    let mut e = EfectoPuesto::nuevo("balance");
    e.vinculos.insert("volumen".into(), 77);
    p.mezcla[1].efectos.push(e);
    let y = a_audio(&p, &mut bib(), Modo::Cancion);
    let (alto, bajo) = (rms(&y[8000..40000]), rms(&y[60000..88000]));
    assert!(alto > 0.5 && bajo < 0.02, "{alto} {bajo}");
    // Se guarda y se abre con su vínculo; si el controlador ya no está, la perilla manda.
    let mut q = Proyecto::de_json(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(q.mezcla[1].efectos[0].vinculos["volumen"], 77);
    q.mezcla[3].efectos.clear();
    let y = a_audio(&q, &mut bib(), Modo::Cancion);
    assert!(rms(&y[60000..88000]) > 0.3);
}

fn con_canal(instrumento: &str, notas: &[(u32, u8)]) -> Proyecto {
    let mut p = Proyecto { bpm: 120.0, ..Default::default() };
    let id = p.agregar_canal(instrumento, "x");
    p.canales[0].volumen = 1.0;
    p.patrones[0].notas.insert(id, notas.iter().map(|(t, n)| Nota { tick: *t, dur: PASO, tono: *n, fuerza: 1.0 }).collect());
    p.mezcla.iter_mut().for_each(|m| m.efectos.clear());
    p
}

#[test]
fn los_pads_suenan_cada_uno_con_su_audio_y_se_cortan() {
    let mut p = con_canal("pads", &[(0, PRIMER_PAD), (PPQ * 2, PRIMER_PAD + 1)]);
    p.canales[0].pads = vec![temporal("pad1", &seno(500.0, 3.0)), temporal("pad2", &seno(1300.0, 0.4))];
    let y = a_audio(&p, &mut bib(), Modo::Patron(p.patrones[0].id));
    // Un negra dura medio segundo: el segundo entra al segundo 1.
    assert!(cuanto(&y[4000..40000], 500.0) > 0.2 && cuanto(&y[4000..40000], 1300.0) < 0.01);
    assert!(cuanto(&y[50000..62000], 1300.0) > 0.2 && cuanto(&y[50000..62000], 500.0) > 0.2);
    // En el mismo grupo, el segundo calla al primero. Y con una octava más sube el suyo.
    p.canales[0].controles = [("g1".to_string(), 1.0), ("g2".into(), 1.0), ("t2".into(), 12.0)].into();
    let y = a_audio(&p, &mut bib(), Modo::Patron(p.patrones[0].id));
    assert!(cuanto(&y[54000..58000], 500.0) < 0.02, "{}", cuanto(&y[54000..58000], 500.0));
    assert!(cuanto(&y[49000..56000], 2600.0) > 0.2);
}

#[test]
fn el_cortador_encuentra_los_golpes() {
    // Cuatro golpes distintos, cada medio segundo.
    let mut x = vec![[0.0f32; 2]; 96000];
    for (k, hz) in [300.0, 700.0, 1500.0, 2600.0].into_iter().enumerate() {
        for i in 0..14000 {
            x[k * 24000 + i + 10] = [(TAU * hz * i as f32 / SR).sin() * 0.6 * (-(i as f32) / 4000.0).exp(); 2];
        }
    }
    let r = temporal("ritmo", &x);
    let mut b = bib();
    let m = b.cortes(&r, &Default::default()).unwrap();
    assert_eq!(m.cortes.len(), 4, "{:?}", m.cortes);
    assert!(m.cortes.iter().enumerate().all(|(i, c)| (c - i as f32 * 0.25).abs() < 0.01), "{:?}", m.cortes);
    // La tecla 50 toca el tercer golpe, en su tono.
    let mut p = con_canal("cortador", &[(0, 50)]);
    p.canales[0].muestra = Some(r);
    let y = a_audio(&p, &mut b, Modo::Patron(p.patrones[0].id));
    assert!(cuanto(&y[200..5000], 1500.0) > 0.1 && cuanto(&y[200..5000], 700.0) < 0.02);
    // Partes iguales.
    let v = [("donde".to_string(), 0.0), ("partes".into(), 8.0)].into();
    assert_eq!(b.cortes(p.canales[0].muestra.as_ref().unwrap(), &v).unwrap().cortes.len(), 8);
}

#[test]
fn el_muestreador_empieza_donde_se_le_diga() {
    let r = temporal("dos", &[seno(400.0, 1.0), seno(900.0, 1.0)].concat());
    let mut p = con_canal("muestreador", &[(0, 60)]);
    p.canales[0].muestra = Some(r);
    let y = |p: &Proyecto| a_audio(p, &mut bib(), Modo::Patron(p.patrones[0].id));
    assert!(cuanto(&y(&p)[500..6000], 400.0) > 0.1);
    p.canales[0].controles.insert("inicio".into(), 0.6);
    assert!(cuanto(&y(&p)[500..6000], 900.0) > 0.1);
}
