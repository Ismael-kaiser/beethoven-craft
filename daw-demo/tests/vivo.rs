//! Lua mientras suena.
use beethoven_daw_demo::biblioteca::Biblioteca;
use beethoven_daw_demo::modelo::*;
use beethoven_daw_demo::motor::{Escena, Modo, Motor, Orden, a_audio};
use beethoven_daw_demo::plugin::Evento;
use beethoven_daw_demo::receta::BLOQUE;
use beethoven_daw_demo::vivo::{Momento, Vivo};
use std::alloc::{GlobalAlloc, Layout, System};
use std::f32::consts::TAU;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// Cuenta lo que se pide en este hilo mientras se vigila.
struct Cuenta;
static PEDIDOS: AtomicUsize = AtomicUsize::new(0);
thread_local!(static VIGILA: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
unsafe impl GlobalAlloc for Cuenta {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if VIGILA.try_with(|v| v.get()).unwrap_or(false) {
            PEDIDOS.fetch_add(1, Relaxed);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static A: Cuenta = Cuenta;

const SR: f32 = 48000.0;

fn armar(cuerpo: &str) -> Vivo {
    let f = format!("return {{ nombre = 'x', tipo = 'efecto', controles = {{ {{ 'k', 'K', 0, 10, 1 }} }}, vivo = {cuerpo} }}");
    Vivo::nuevo(&f, &Default::default(), SR, &["k".to_string()], &Default::default(), (&[], None, 0)).unwrap()
}

fn pasar(v: &Vivo, x: &[f32], k: f32) -> Vec<f32> {
    let mut sale = vec![];
    let mut notas = vec![];
    for (b, t) in x.chunks(BLOQUE).enumerate() {
        let mut y = [[0.0; BLOQUE]; 2];
        y[0][..t.len()].copy_from_slice(t);
        y[1][..t.len()].copy_from_slice(t);
        let m = Momento { bpm: 120.0, pos: (b * BLOQUE) as f64 / SR as f64 * 2.0, tocando: true };
        v.procesar(&[], &[k], &m, &mut y, &[[0.0; BLOQUE]; 2], t.len(), &mut notas);
        sale.extend_from_slice(&y[0][..t.len()]);
    }
    sale
}

fn seno(hz: f32, n: usize) -> Vec<f32> {
    (0..n).map(|i| (TAU * hz * i as f32 / SR).sin() * 0.5).collect()
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
}

/// Cuánto hay de esa frecuencia.
fn hay(x: &[f32], hz: f32) -> f32 {
    let n = x.len() as f64;
    let (a, b) = x.iter().enumerate().fold((0.0f64, 0.0f64), |(a, b), (i, v)| {
        let w = std::f64::consts::TAU * hz as f64 * i as f64 / SR as f64;
        let h = 1.0 - (std::f64::consts::TAU * i as f64 / n).cos();
        (a + *v as f64 * h * w.cos(), b + *v as f64 * h * w.sin())
    });
    (a.hypot(b) * 2.0 / n) as f32
}

/// La frecuencia más fuerte cerca de una.
fn cerca(x: &[f32], hz: f32) -> f32 {
    (0..=240).map(|i| hz * 2f32.powf((i as f32 - 120.0) / 1440.0)).max_by(|a, b| hay(x, *a).total_cmp(&hay(x, *b))).unwrap()
}

#[test]
fn cada_muestra_pasa_por_lua() {
    // Un filtro de un polo, con memoria propia y una decisión por muestra: lo que un grafo no puede.
    let v = armar(
        "function(v) local z, n = 0.0, 0 return function(l, r, c) n = n + 1 z = z + (l - z) * 0.1 if n % 2 == 0 then return z * c.k else return -z * c.k end end end",
    );
    let x = seno(300.0, 4800);
    let y = pasar(&v, &x, 2.0);
    let mut z = 0.0f64;
    for (i, (x, y)) in x.iter().zip(&y).enumerate() {
        z += (*x as f64 - z) * 0.1;
        let q = if (i + 1) % 2 == 0 { z * 2.0 } else { -z * 2.0 };
        assert!((q as f32 - y).abs() < 1e-5, "{i}");
    }
    assert!(v.falla().is_none());
}

#[test]
fn por_frecuencias_reconstruye_y_dice_su_latencia() {
    let v = armar("function(v) return espectral(1024, function(re, im, l, c) end) end");
    assert_eq!(v.latencia, 1024);
    let x: Vec<f32> = seno(777.0, 24000).iter().zip(seno(3100.0, 24000)).map(|(a, b)| a + b * 0.5).collect();
    let y = pasar(&v, &x, 1.0);
    for i in 3000..20000 {
        assert!((y[i + 1024] - x[i]).abs() < 2e-3, "{i}: {} {}", y[i + 1024], x[i]);
    }
    // Dejar pasar solo abajo de 1 kHz.
    let v = armar(
        "function(v) return espectral(1024, function(re, im) for i = 1, 1024 do local f = (i - 1 <= 512) and i - 1 or 1025 - i if f > 21 then re[i], im[i] = 0.0, 0.0 end end end) end",
    );
    let y = pasar(&v, &seno(5000.0, 12000), 1.0);
    assert!(rms(&y[4000..]) < 0.005);
}

#[test]
fn no_pide_memoria_al_sonar() {
    let b = Biblioteca::nueva(SR, None);
    for k in ["escalera", "afinador", "quitaruido", "congelar", "monoanalogico", "arpegiador", "acordes"] {
        let g = b.ver(k).unwrap();
        let claves: Vec<String> = g.ficha.controles.iter().map(|c| c.clave.clone()).collect();
        let ctl: Vec<f32> = g.ficha.controles.iter().map(|c| c.valor).collect();
        let v = Vivo::nuevo(&g.fuente, &g.archivos, SR, &claves, &Default::default(), (&g.ficha.medidas, Some(Default::default()), 0)).unwrap();
        let mut notas = Vec::with_capacity(64);
        let x = seno(220.0, BLOQUE);
        let mut correr = |n: usize, vigila: bool| {
            let antes = PEDIDOS.load(Relaxed);
            VIGILA.with(|w| w.set(vigila));
            let mut peor = 0u128;
            for q in 0..n {
                let mut y = [[0.0; BLOQUE]; 2];
                y[0].copy_from_slice(&x);
                y[1].copy_from_slice(&x);
                let ev: &[Evento] = match q % 200 {
                    0 => &[Evento::Nota { tono: 60, fuerza: 1.0 }, Evento::Nota { tono: 67, fuerza: 1.0 }],
                    150 => &[Evento::Suelta { tono: 60 }, Evento::Suelta { tono: 67 }, Evento::Rueda(0.3), Evento::Mando { n: 1, v: 0.5 }],
                    _ => &[],
                };
                let m = Momento { bpm: 120.0, pos: q as f64 * 0.0027, tocando: true };
                let t = std::time::Instant::now();
                v.procesar(ev, &ctl, &m, &mut y, &[[0.0; BLOQUE]; 2], BLOQUE, &mut notas);
                peor = peor.max(t.elapsed().as_micros());
                notas.clear();
            }
            VIGILA.with(|w| w.set(false));
            (PEDIDOS.load(Relaxed) - antes, peor)
        };
        correr(800, false);
        let (pedidos, peor) = correr(3000, true);
        assert!(v.falla().is_none(), "{k}");
        assert_eq!(pedidos, 0, "{k} pidió memoria");
        println!("{k}: peor bloque {peor} us de 1333");
    }
}

#[test]
fn uno_malo_se_apaga_y_el_sonido_sigue() {
    for (cuerpo, por) in [
        ("function(v) local n = 0 return function(l, r) n = n + 1 if n > 2000 then while true do end end return l, r end end", "tardó"),
        ("function(v) local n = 0 return function(l, r) n = n + 1 if n > 2000 then return l / 0 end return l, r end end", "infinito"),
        ("function(v) local n = 0 return function(l, r) n = n + 1 if n > 2000 then return nada.x end return l, r end end", "nada"),
        (
            "function(v) local t, n = {}, 0 return function(l, r) n = n + 1 if n > 2000 then for i = 1, 200 do t[#t + 1] = {} end end return l, r end end",
            "",
        ),
    ] {
        let v = armar(cuerpo);
        let x = seno(300.0, 96000);
        let y = pasar(&v, &x, 1.0);
        let e = v.falla().unwrap_or_else(|| panic!("no avisó: {cuerpo}"));
        assert!(e.contains(por), "{e}");
        // Después de la falla pasa lo que entra.
        assert!(y[90000..].iter().zip(&x[90000..]).all(|(a, b)| a == b), "{cuerpo}");
    }
    // Al armarse también se revisa.
    for malo in ["5", "function(v) return 5 end", "function(v) while true do end end"] {
        let f = format!("return {{ nombre = 'x', tipo = 'efecto', vivo = {malo} }}");
        assert!(Vivo::nuevo(&f, &Default::default(), SR, &[], &Default::default(), (&[], None, 0)).is_err(), "{malo}");
    }
}

fn con_audio(x: &[f32], efecto: &str, v: &[(&str, f32)]) -> Vec<f32> {
    let r = std::env::temp_dir().join(format!("bdaw-vivo-{}-{efecto}-{}.wav", std::process::id(), x.len()));
    beethoven_daw_demo::wav::escribir(&r, 48000, &x.iter().map(|v| [*v; 2]).collect::<Vec<_>>()).unwrap();
    let mut p = Proyecto { bpm: 120.0, ..Default::default() };
    p.mezcla.iter_mut().for_each(|m| {
        m.efectos.clear();
        m.volumen = 1.0;
    });
    let audio = ClipAudio { ruta: r.to_string_lossy().into(), nombre: "x".into(), mezcla: 1, ..Default::default() };
    p.clips.push(Clip { patron: 0, pista: 0, tick: 0, dur: p.pulsos(x.len() as f32 / SR), desfase: 0, audio: Some(audio) });
    let mut e = EfectoPuesto::nuevo(efecto);
    e.controles = v.iter().map(|(k, x)| (k.to_string(), *x)).collect();
    p.mezcla[1].efectos.push(e);
    a_audio(&p, &mut Biblioteca::nueva(SR, None), Modo::Cancion).iter().map(|v| v[0]).collect()
}

#[test]
fn el_afinador_lleva_la_voz_a_la_nota() {
    // Como una voz: muchos armónicos, 35 centésimas arriba de La.
    for (hz, meta) in [(220.0 * 2f32.powf(0.35 / 12.0), 220.0), (330.0 * 2f32.powf(-0.4 / 12.0), 329.6)] {
        let x: Vec<f32> = (0..144000).map(|i| (1..8).map(|k| (TAU * hz * k as f32 * i as f32 / SR).sin() / k as f32).sum::<f32>() * 0.25).collect();
        let y = con_audio(&x, "afinador", &[("escala", 2.0), ("rapido", 1.0)]);
        let f = cerca(&y[48000..140000], meta);
        assert!((f / meta - 1.0).abs() < 0.004, "{hz}: {f} en vez de {meta}");
    }
}

#[test]
fn quita_el_ruido_aprendido() {
    let mut s = 9u32;
    let ruido: Vec<f32> = (0..192000)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            (s as f32 / u32::MAX as f32 - 0.5) * 0.08
        })
        .collect();
    let b = Biblioteca::nueva(SR, None);
    let g = b.ver("quitaruido").unwrap();
    let claves: Vec<String> = g.ficha.controles.iter().map(|c| c.clave.clone()).collect();
    let v = Vivo::nuevo(&g.fuente, &g.archivos, SR, &claves, &Default::default(), (&g.ficha.medidas, None, 0)).unwrap();
    let corre = |x: &[f32], ctl: &[f32]| {
        let mut sale = vec![];
        for t in x.chunks_exact(BLOQUE) {
            let mut y = [[0.0; BLOQUE]; 2];
            y[0].copy_from_slice(t);
            v.procesar(&[], ctl, &Momento::default(), &mut y, &[[0.0; BLOQUE]; 2], BLOQUE, &mut vec![]);
            sale.extend_from_slice(&y[0]);
        }
        sale
    };
    corre(&ruido[..96000], &[1.0, 2.0, -24.0, 1.0]);
    let y = corre(&ruido[96000..], &[0.0, 2.0, -24.0, 1.0]);
    assert!(rms(&y[20000..]) < rms(&ruido[96000..]) * 0.25, "{} {}", rms(&y[20000..]), rms(&ruido));
    let y = corre(&seno(600.0, 48000), &[0.0, 2.0, -24.0, 1.0]);
    assert!(rms(&y[10000..40000]) > 0.33, "el tono pasa: {}", rms(&y[10000..40000]));
}

fn canal(instrumento: &str, notas: &[(u32, u32, u8)]) -> Proyecto {
    let mut p = Proyecto { bpm: 120.0, ..Default::default() };
    let id = p.agregar_canal(instrumento, "x");
    p.canales[0].volumen = 1.0;
    p.patrones[0].notas.insert(id, notas.iter().map(|(t, d, n)| Nota { tick: *t, dur: *d, tono: *n, fuerza: 1.0 }).collect());
    p.mezcla.iter_mut().for_each(|m| m.efectos.clear());
    p
}

#[test]
fn el_sinte_mono_afina_desliza_y_obedece_la_rueda() {
    let sencillo = |k: &str| -> std::collections::BTreeMap<String, f32> {
        let v: &[(&str, f32)] = if k == "acido" {
            &[("corte", 300.0), ("filo", 0.0), ("abre", 0.0), ("fuego", 0.0)]
        } else {
            &[("corte", 300.0), ("filo", 0.0), ("abre", 0.0), ("sub", 0.0), ("empuje", 0.0)]
        };
        v.iter().map(|(k, x)| (k.to_string(), *x)).collect()
    };
    let mut p = canal("monoanalogico", &[(0, PPQ * 3, 57)]);
    p.canales[0].controles = sencillo("monoanalogico");
    let mut b = Biblioteca::nueva(SR, None);
    let y: Vec<f32> = a_audio(&p, &mut b, Modo::Patron(p.patrones[0].id)).iter().map(|v| v[0]).collect();
    assert!((cerca(&y[8000..60000], 220.0) - 220.0).abs() < 1.0, "{}", cerca(&y[8000..60000], 220.0));
    assert!(rms(&y[8000..60000]) > 0.02 && rms(&y[90000..95000]) < 0.002);
    // La rueda arriba sube dos semitonos: también al sinte de grafo.
    for k in ["monoanalogico", "acido"] {
        let mut p = canal(k, &[]);
        p.canales[0].controles = sencillo(k);
        let (mut m, mando) = Motor::nuevo(Escena::nueva(&p, &mut Biblioteca::nueva(SR, None), Modo::Patron(p.patrones[0].id)));
        mando.mandar(Orden::Nota { canal: p.canales[0].id, tono: 57, fuerza: 1.0 });
        mando.mandar(Orden::Mando { canal: p.canales[0].id, evento: Evento::Rueda(1.0) });
        let mut x = vec![[0.0f32; 2]; 48000];
        x.chunks_mut(256).for_each(|c| m.tocar(c));
        let y: Vec<f32> = x.iter().map(|v| v[0]).collect();
        assert!((cerca(&y[8000..], 246.9) - 246.9).abs() < 2.0, "{k}: {}", cerca(&y[8000..], 246.9));
    }
}

#[test]
fn el_arpegiador_y_los_acordes_mandan_notas() {
    // Con un acorde apretado, el arpegio da una nota por vez.
    let mut p = canal("barras", &[(0, PPQ * 4, 60), (0, PPQ * 4, 64), (0, PPQ * 4, 67)]);
    p.canales[0].controles =
        (1..=9).map(|i| (format!("h{i}"), if i == 3 { 1.0 } else { 0.0 })).chain([("tiembla".to_string(), 0.0), ("golpecito".into(), 0.0)]).collect();
    p.canales[0].notas = Some("arpegiador".into());
    p.canales[0].notas_controles = [("paso".to_string(), 2.0), ("octavas".into(), 1.0)].into();
    let p = Proyecto::de_json(&serde_json::to_string(&p).unwrap()).unwrap();
    let y: Vec<f32> = a_audio(&p, &mut Biblioteca::nueva(SR, None), Modo::Patron(p.patrones[0].id)).iter().map(|v| v[0]).collect();
    // Corcheas a 120: 12000 muestras cada una.
    for (k, hz) in [261.6, 329.6, 392.0].into_iter().enumerate() {
        let t = &y[k * 12000 + 1500..k * 12000 + 7500];
        for otra in [261.6, 329.6, 392.0] {
            if otra == hz { assert!(hay(t, otra) > 0.02, "{k}: {}", hay(t, otra)) } else { assert!(hay(t, otra) < hay(t, hz) * 0.2, "{k} {otra}") }
        }
    }
    let mut p = canal("barras", &[(0, PPQ * 2, 60)]);
    p.canales[0].notas = Some("acordes".into());
    let y: Vec<f32> = a_audio(&p, &mut Biblioteca::nueva(SR, None), Modo::Patron(p.patrones[0].id)).iter().map(|v| v[0]).collect();
    for hz in [130.8, 261.6, 329.6, 392.0] {
        assert!(hay(&y[5000..40000], hz) > 0.01, "{hz}");
    }
}

#[test]
fn la_latencia_se_compensa() {
    // Dos pistas con el mismo golpe; una pasa por un efecto que se tarda 2048 muestras sin cambiar nada.
    let mut x = vec![0.0f32; 48000];
    x[10000] = 0.5;
    let r = std::env::temp_dir().join(format!("bdaw-lat-{}.wav", std::process::id()));
    beethoven_daw_demo::wav::escribir(&r, 48000, &x.iter().map(|v| [*v; 2]).collect::<Vec<_>>()).unwrap();
    let mut p = Proyecto { bpm: 120.0, ..Default::default() };
    p.mezcla.iter_mut().for_each(|m| {
        m.efectos.clear();
        m.volumen = 1.0;
    });
    for i in 0..2 {
        let audio = ClipAudio { ruta: r.to_string_lossy().into(), nombre: "x".into(), mezcla: i + 1, ..Default::default() };
        p.clips.push(Clip { patron: 0, pista: i, tick: 0, dur: p.pulsos(1.0), desfase: 0, audio: Some(audio) });
    }
    let mut e = EfectoPuesto::nuevo("quitaruido");
    e.controles = [("mezcla".to_string(), 0.0)].into();
    p.mezcla[2].efectos.push(e);
    // Y un envío con otro que también se tarda.
    let y = a_audio(&p, &mut Biblioteca::nueva(SR, None), Modo::Cancion);
    let golpes: Vec<(usize, f32)> = y.iter().enumerate().filter(|(_, v)| v[0].abs() > 0.1).map(|(i, v)| (i, v[0])).collect();
    assert!(!golpes.is_empty() && golpes.iter().all(|g| g.0 >= 10000 + 2046 && g.0 <= 10000 + 2050), "{golpes:?}");
    assert!(golpes.iter().map(|g| g.1).sum::<f32>() > 0.9, "llegan juntos: {golpes:?}");
}

#[test]
fn graba_mide_y_recuerda_textos() {
    let f = "return { nombre = 'x', tipo = 'efecto', medidas = { 'n' }, vivo = function(v) local k = tonumber(v.datos.k) local i = 0 return function(l, r) i = i + 1 graba(i > 64 and i <= 64 * 3) mide('n', k) return l * k, r * k end end }";
    let medidas: std::sync::Arc<beethoven_daw_demo::biblioteca::Medidas> = Default::default();
    let datos = [("k".to_string(), "3".to_string())].into();
    let v = Vivo::nuevo(f, &Default::default(), SR, &[], &datos, (&["n".to_string()], Some(medidas.clone()), 2)).unwrap();
    let y = pasar(&v, &vec![0.1; 64 * 6], 0.0);
    assert!((y[0] - 0.3).abs() < 1e-6 && f32::from_bits(medidas[2].load(Relaxed)) == 3.0);
    let g = v.grabado();
    assert!(g.len() == 128 && (g[0][0] - 0.3).abs() < 1e-6, "{}", g.len());
}
