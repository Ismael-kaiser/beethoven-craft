//! Lo que un guion puede inventar: curvas, ondas, lazos, el tempo y su propia interfaz.
use beethoven_daw_demo::interfaz::{Momento, Pieza, cargar};
use beethoven_daw_demo::lua::armar;
use beethoven_daw_demo::medir::correr;
use std::f32::consts::TAU;

const SR: f32 = 48000.0;

fn efecto(cuerpo: &str, x: &[f32]) -> Vec<f32> {
    let f = format!("return {{ nombre = 'x', tipo = 'efecto', sonido = function(e, c) {cuerpo} end }}");
    let r = armar(&f, SR).unwrap().1.unwrap();
    correr(&r, &[], &[x.to_vec(), x.to_vec()])[0].clone()
}

fn seno(hz: f32, n: usize) -> Vec<f32> {
    (0..n).map(|i| (TAU * hz * i as f32 / SR).sin()).collect()
}

/// Cuánto hay de esa frecuencia.
fn cuanto(x: &[f32], hz: f32) -> f32 {
    let n = x.len();
    let (a, b) = x.iter().enumerate().fold((0.0f64, 0.0f64), |(a, b), (i, v)| {
        let w = std::f64::consts::TAU * hz as f64 * i as f64 / SR as f64;
        let h = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos();
        (a + *v as f64 * h * w.cos(), b + *v as f64 * h * w.sin())
    });
    (a.hypot(b) * 4.0 / n as f64) as f32
}

#[test]
fn curva_propia_y_suavizada() {
    let x = seno(100.0, 4800);
    let y = efecto("return curva(e.izq, function(v) if v > 0.5 then return 0.5 end return v end)", &x);
    assert!(y.iter().zip(&x).all(|(y, x)| (y - x.min(0.5)).abs() < 0.003));
    // Una nota aguda muy recortada: sus armónicos que rebotan (aquí 5·7000 cae en 13000) bajan al suavizar.
    let x: Vec<f32> = seno(7000.0, 9600).iter().map(|v| v * 3.0).collect();
    let corta = "function(v) return math.max(-1, math.min(1, v)) end";
    let duro = efecto(&format!("return curva(e.izq, {corta})"), &x);
    let suave = efecto(&format!("return curva(e.izq, {corta}, 'suave')"), &x);
    let (a, b) = (cuanto(&duro, 13000.0), cuanto(&suave, 13000.0));
    assert!(b < a * 0.5, "{a} {b}");
}

#[test]
fn onda_propia_sin_raspar() {
    // Una sierra hecha en Lua, tocada aguda: su octavo armónico no debe rebotar.
    let y = efecto("return onda(5000, function(f) return 2 * f - 1 end)", &vec![0.0; 9600]);
    assert!((cuanto(&y, 5000.0) - 2.0 / std::f32::consts::PI).abs() < 0.05, "{}", cuanto(&y, 5000.0));
    assert!(cuanto(&y, 8000.0) < 0.01, "{}", cuanto(&y, 8000.0));
}

#[test]
fn lazo_con_lo_que_quieras_dentro() {
    let mut x = vec![0.0; 4800];
    x[0] = 1.0;
    // Cada vuelta, a la mitad: ecos cada 10 ms.
    let y = efecto("return lazo(0.01, function(vuelta) return e.izq + vuelta * 0.5 end)", &x);
    for (k, v) in [(1, 1.0), (2, 0.5), (3, 0.25), (4, 0.125)] {
        assert!((y[480 * k] - v).abs() < 1e-3, "{k}: {}", y[480 * k]);
    }
    assert!(y[1..480].iter().all(|v| *v == 0.0));
    // Un lazo que crece no revienta.
    let y = efecto("return lazo(0.002, function(v) return e.izq + v * 3 end)", &vec![1.0; 48000]);
    assert!(y.iter().all(|v| v.is_finite() && v.abs() <= 8.0));
}

#[test]
fn tempo_y_cuentas() {
    // Sin canción cuenta como 120: una negra dura medio segundo.
    let y = efecto("return negras(1)", &vec![0.0; 64]);
    assert!((y[0] - 0.5).abs() < 1e-5);
    for (cuenta, v) in
        [("raiz(e.izq)", 2.0), ("piso(e.izq / 3)", 1.0), ("resto(e.izq, 3)", 1.0), ("si(mayor(e.izq, 3), 7, 9)", 7.0), ("ln(exp(e.izq))", 4.0)]
    {
        let y = efecto(&format!("return {cuenta}"), &vec![4.0; 64]);
        assert!((y[63] - v).abs() < 1e-3, "{cuenta}: {}", y[63]);
    }
}

#[test]
fn la_interfaz_decide_y_oye() {
    let f = r##"return { nombre = "x", tipo = "efecto", controles = { { "a", "A", 0, 10, 1 } },
      sonido = function(e, c) return e.izq * c.a end,
      interfaz = { dibujar = function(d, c, m)
        if m.clic then d.poner("a", c.a + 1) end
        d.texto(0, 0, #m.onda .. " " .. #m.espectro)
      end } }"##;
    let mut i = cargar(f).unwrap().unwrap();
    let m = Momento { clic: true, onda: vec![0.0; 8], espectro: vec![0.5; 4], ..Default::default() };
    let p = i.cuadro(&[("a".to_string(), 2.0)].into(), &m);
    assert!(matches!(&p[..], [Pieza::Poner { v, .. }, Pieza::Texto { s, .. }] if *v == 3.0 && s == "8 4"), "{p:?} {:?}", i.error);
}

#[test]
fn el_ejemplo_del_manual_sirve() {
    let manual = include_str!("../README.md");
    let desde = manual.find("-- Eco a tempo").expect("el ejemplo");
    let f = &manual[desde..desde + manual[desde..].find("```").unwrap()];
    let r = armar(f, SR).unwrap().1.unwrap();
    let ctl: Vec<f32> = r.controles.iter().map(|k| k.valor).collect();
    let mut x = vec![0.0; 96000];
    x[0] = 1.0;
    let y = correr(&r, &ctl, &[x.clone(), x]);
    // Corchea con puntillo a 120: 0.375 s.
    assert!(y[0][17900..18100].iter().any(|v| v.abs() > 0.05));
}

/// Cuántas veces por segundo cruza hacia arriba.
fn cruces(x: &[f32]) -> f32 {
    x.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f32 / (x.len() as f32 / SR)
}

fn de_fabrica(clave: &str, cambios: &[(&str, f32)], x: &[f32]) -> Vec<f32> {
    let b = beethoven_daw_demo::biblioteca::Biblioteca::nueva(SR, None);
    let r = b.ver(clave).unwrap().receta.clone().unwrap();
    let ctl: Vec<f32> = r.controles.iter().map(|k| cambios.iter().find(|c| c.0 == k.clave).map_or(k.valor, |c| c.1)).collect();
    correr(&r, &ctl, &[x.to_vec(), x.to_vec()])[0].clone()
}

#[test]
fn pasos_sigue_el_ritmo() {
    // A 120, cuatro por negra: cada paso dura 6000 muestras; los pares callan.
    let y = de_fabrica("pasos", &[], &vec![0.5; 24000]);
    assert!((y[3000] - 0.5).abs() < 0.02 && y[9000].abs() < 0.02 && (y[15000] - 0.5).abs() < 0.02, "{} {} {}", y[3000], y[9000], y[15000]);
}

#[test]
fn cambia_tono_sube_y_baja() {
    for (semi, meta) in [(12.0, 880.0), (-12.0, 220.0)] {
        let y = de_fabrica("cambiatono", &[("semi", semi), ("mezcla", 1.0)], &seno(440.0, 48000));
        let hz = cruces(&y[4800..]);
        assert!((hz - meta).abs() < meta * 0.06, "{semi}: {hz}");
    }
}

#[test]
fn cada_interfaz_nueva_dibuja() {
    let b = beethoven_daw_demo::biblioteca::Biblioteca::nueva(SR, None);
    for clave in ["pasos", "empujon", "filtrovivo", "pedalera", "cambiatono", "visor"] {
        let g = b.ver(clave).unwrap();
        let mut i = cargar(&g.fuente).unwrap().unwrap();
        let v = g.ficha.controles.iter().map(|c| (c.clave.clone(), c.valor)).collect();
        let m = Momento { nivel: 0.5, onda: vec![0.1; 256], espectro: vec![0.5; 64], clic: true, raton: Some([20.0, 20.0]), ..Default::default() };
        assert!(i.cuadro(&v, &m).len() > 3 && i.error.is_none(), "{clave}: {:?}", i.error);
    }
}

#[test]
fn cada_sintetizador_suena_afinado_y_sin_reventar() {
    use beethoven_daw_demo::receta::{BLOQUE, DatosVoz, comunes, voz};
    let b = beethoven_daw_demo::biblioteca::Biblioteca::nueva(SR, None);
    for clave in ["tresondas", "frecuencias", "muralla", "acido", "retumbo", "pulsada", "barras"] {
        let g = b.ver(clave).unwrap();
        let r = g.receta.as_deref().unwrap();
        let mut variantes: Vec<_> = g.ficha.presets.iter().map(|p| p.1.clone()).collect();
        variantes.push(Default::default());
        assert!(variantes.len() >= 4, "{clave}");
        for v in variantes {
            let ctl: Vec<f32> = r.controles.iter().map(|k| v.get(&k.clave).copied().unwrap_or(k.valor)).collect();
            let mut mem = vec![0.0; r.memoria];
            let mut bufs = vec![[0.0; BLOQUE]; r.cuantos_nodos()];
            let d = DatosVoz { hz: 220.0, fuerza: 1.0, azar: 0.3, puerta: true, ..Default::default() };
            let mut pico = 0.0f32;
            for _ in 0..1500 {
                comunes(r, &ctl, &mut bufs, BLOQUE);
                let s = voz(r, &d, &ctl, None, None, &mut mem, &mut bufs, BLOQUE);
                pico = s.iter().flat_map(|s| bufs[*s as usize]).fold(pico, |m, v| m.max(v.abs()));
                assert!(pico.is_finite(), "{clave}");
            }
            assert!(pico > 0.02 && pico < 1.2, "{clave}: {pico}");
        }
        let mut i = cargar(&g.fuente).unwrap().unwrap();
        let valores = g.ficha.controles.iter().map(|c| (c.clave.clone(), c.valor)).collect();
        let m = Momento { nivel: 0.4, tocando: true, tiempo: 1.0, ..Default::default() };
        assert!(i.cuadro(&valores, &m).len() > 5 && i.error.is_none(), "{clave}: {:?}", i.error);
    }
}

/// Lo que sale de un guion con esos controles.
fn oir(
    r: &beethoven_daw_demo::receta::Receta,
    v: &std::collections::BTreeMap<String, f32>,
    instrumento: bool,
    h: Option<&beethoven_daw_demo::muestrario::Muestra>,
) -> Vec<f32> {
    use beethoven_daw_demo::receta::{BLOQUE, DatosVoz, comunes, voz};
    let ctl: Vec<f32> = r.controles.iter().map(|k| v.get(&k.clave).copied().unwrap_or(k.valor)).collect();
    if !instrumento {
        // Fuerte y bajito, con graves y agudos, distinto a cada lado y con silencios: hay algo para cada efecto.
        let mut s = 99u32;
        let mut azar = || {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            s as f32 / u32::MAX as f32 - 0.5
        };
        let lado = |hz: f32, azar: &mut dyn FnMut() -> f32| -> Vec<f32> {
            (0..102000)
                .map(|i| {
                    let t = i as f32 / SR;
                    let nivel = if (i / 6000) % 4 == 3 { 0.004 } else { 0.2 + 0.7 * (-((i % 6000) as f32) / 1500.0).exp() };
                    nivel * (0.4 * (TAU * 70.0 * t).sin() + 0.3 * (TAU * hz * t).sin() + 0.2 * (TAU * 7000.0 * t).sin() + 0.3 * azar())
                })
                .collect()
        };
        let x = [lado(500.0, &mut azar), lado(660.0, &mut azar)];
        return beethoven_daw_demo::medir::correr_con(r, &ctl, &x, h).concat();
    }
    let (mut mem, mut b) = (vec![0.0; r.memoria], vec![[0.0; BLOQUE]; r.cuantos_nodos()]);
    let mut sale = vec![];
    for i in 0..1500 {
        comunes(r, &ctl, &mut b, BLOQUE);
        let d = DatosVoz { hz: 110.0, fuerza: 0.8, azar: 0.3, puerta: i < 700, ..Default::default() };
        for n in voz(r, &d, &ctl, None, None, &mut mem, &mut b, BLOQUE) {
            sale.extend(b[n as usize]);
        }
    }
    sale
}

/// Lo que hace un guion en vivo: sonido, o las notas que manda.
fn oir_vivo(
    g: &beethoven_daw_demo::biblioteca::Guion,
    r: &beethoven_daw_demo::receta::Receta,
    v: &std::collections::BTreeMap<String, f32>,
) -> Vec<f32> {
    use beethoven_daw_demo::plugin::Evento;
    use beethoven_daw_demo::receta::BLOQUE;
    use beethoven_daw_demo::vivo::{Momento, Vivo};
    let claves: Vec<String> = r.controles.iter().map(|c| c.clave.clone()).collect();
    let ctl: Vec<f32> = r.controles.iter().map(|c| v.get(&c.clave).copied().unwrap_or(c.valor)).collect();
    let w = Vivo::nuevo(&g.fuente, &g.archivos, SR, &claves, &Default::default(), (&g.ficha.medidas, None, 0)).unwrap();
    let (mut sale, mut notas, mut s) = (vec![], Vec::with_capacity(64), 5u32);
    for b in 0..1200 {
        let mut y = [[0.0; BLOQUE]; 2];
        if g.ficha.tipo == beethoven_daw_demo::lua::Tipo::Efecto {
            for i in 0..BLOQUE {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                let t = (b * BLOQUE + i) as f32 / SR;
                // Una voz desafinada que entra y sale, con algo de ruido.
                let voz: f32 = (1..6).map(|k| (TAU * 187.0 * k as f32 * t).sin() / k as f32).sum::<f32>() * 0.2;
                let x = if (b / 200) % 2 == 0 { voz } else { 0.0 } + (s as f32 / u32::MAX as f32 - 0.5) * 0.05;
                (y[0][i], y[1][i]) = (x, x);
            }
        }
        let ev: &[Evento] = match b {
            3 => &[Evento::Nota { tono: 45, fuerza: 0.8 }, Evento::Mando { n: 1, v: 1.0 }],
            200 => &[Evento::Nota { tono: 52, fuerza: 0.8 }],
            700 => &[Evento::Suelta { tono: 45 }, Evento::Suelta { tono: 52 }],
            _ => &[],
        };
        let m = Momento { bpm: 120.0, pos: (b * BLOQUE) as f64 / SR as f64 * 2.0, tocando: true };
        // «Aprender» se suelta a la mitad: lo aprendido cuenta después.
        let mut q = ctl.clone();
        if let Some(k) = claves.iter().position(|c| c == "aprende").filter(|_| b > 500) {
            q[k] = 0.0;
        }
        w.procesar(ev, &q, &m, &mut y, &[[0.0; BLOQUE]; 2], BLOQUE, &mut notas);
        for n in notas.drain(..) {
            sale.extend([n.tono as f32, n.fuerza, n.segundos, b as f32]);
        }
        sale.extend(y[0]);
    }
    assert!(w.falla().is_none(), "{}", g.clave);
    sale.resize(1200 * BLOQUE + 4000, 0.0);
    sale
}

/// Ningún control de adorno: mover cualquiera cambia lo que se oye, al menos en uno de sus preajustes.
#[test]
fn cada_control_hace_algo() {
    use beethoven_daw_demo::lua::Tipo;
    let b = beethoven_daw_demo::biblioteca::Biblioteca::nueva(SR, None);
    let mut b2 = beethoven_daw_demo::biblioteca::Biblioteca::nueva(SR, None);
    let mut malos = vec![];
    for g in b.guiones.iter().filter(|g| {
        g.ficha.tipo != Tipo::Herramienta
            && !["muestra", "visor", "muestreador", "pads", "cortador", "ctlnivel", "ctllfo", "ctlformula"].contains(&g.clave.as_str())
    }) {
        let r = g.receta.as_deref().unwrap();
        let es = g.ficha.tipo == Tipo::Instrumento;
        let mut bases: Vec<_> = g.ficha.presets.iter().map(|p| p.1.clone()).collect();
        bases.push(Default::default());
        // También con todo a la mitad: hay controles que solo cuentan si otro está prendido.
        for k in [0.5, 0.75] {
            bases.push(r.controles.iter().map(|c| (c.clave.clone(), c.min + (c.max - c.min) * k)).collect());
        }
        // «pista» elige otra pista del Mixer: aquí no hay.
        for c in r.controles.iter().filter(|c| c.clave != "pista" && g.clave != "ducker") {
            let cambia = bases.iter().any(|base| {
                let mut con = |v: f32| {
                    let mut x = base.clone();
                    x.insert(c.clave.clone(), v);
                    let mut e = beethoven_daw_demo::modelo::EfectoPuesto::nuevo(&g.clave);
                    e.controles = x.clone();
                    let h = if r.con_huella() { b2.huella(&e) } else { None };
                    if r.vivo {
                        return oir_vivo(g, r, &x);
                    }
                    oir(r, &x, es, h.as_deref())
                };
                let (a, z) = (con(c.min), con(c.max));
                let n = |v: &[f32]| v.iter().map(|x| x * x).sum::<f32>().sqrt();
                // Las notas se comparan tal cual.
                if g.ficha.tipo == Tipo::Notas {
                    return a != z;
                }
                // Por pedazos: un soplido solo se nota en los silencios.
                a.chunks(3000).zip(z.chunks(3000)).any(|(a, z)| {
                    let d: Vec<f32> = a.iter().zip(z).map(|(a, z)| a - z).collect();
                    n(&d) > 0.05 * n(a).max(n(z)).max(0.02)
                })
            });
            if !cambia {
                malos.push(format!("{}.{}", g.clave, c.clave));
            }
        }
    }
    assert!(malos.is_empty(), "no hacen nada: {malos:?}");
}
