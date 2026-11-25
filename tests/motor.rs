use std::path::PathBuf;
use std::sync::Arc;
use toma_uno::efectos::{TipoFx, crear};
use toma_uno::medios::Audio;
use toma_uno::modelo::*;
use toma_uno::motor::{self, Almacen};

const N: usize = SR as usize;

fn seno(hz: f32, amp: f32, n: usize) -> Vec<f32> {
    (0..n).flat_map(|i| [(std::f32::consts::TAU * hz * i as f32 / SR as f32).sin() * amp; 2]).collect()
}
fn pasar(t: TipoFx, p: &[f32], x: &[f32]) -> Vec<f32> {
    let mut e = crear(t, p);
    let mut y = x.to_vec();
    y.chunks_mut(512).for_each(|b| e.procesar(b));
    y
}
/// Nivel RMS en dB de la segunda mitad (ya estable).
fn nivel(x: &[f32]) -> f32 {
    let x = &x[x.len() / 2..];
    10.0 * (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).max(1e-20).log10()
}
fn cambio(t: TipoFx, p: &[f32], x: &[f32]) -> f32 {
    nivel(&pasar(t, p, x)) - nivel(x)
}

#[test]
fn ecualizador() {
    let d = TipoFx::Ecualizador.defaults();
    for hz in [60.0, 1000.0, 12000.0] {
        let g = cambio(TipoFx::Ecualizador, &d, &seno(hz, 0.2, N));
        assert!(g.abs() < 0.1, "plano a {hz}: {g}");
    }
    let mut p = d.clone();
    p[1] = 12.0;
    let g = cambio(TipoFx::Ecualizador, &p, &seno(40.0, 0.1, N));
    println!("graves +12: {g:.2} dB a 40 Hz");
    assert!((g - 12.0).abs() < 1.5, "{g}");
    let mut p = d.clone();
    p[0] = 200.0;
    let g = cambio(TipoFx::Ecualizador, &p, &seno(50.0, 0.2, N));
    println!("corte 200 Hz: {g:.2} dB a 50 Hz");
    assert!(g < -15.0, "{g}");
    let mut p = d.clone();
    p[6] = 3000.0;
    let g = cambio(TipoFx::Ecualizador, &p, &seno(12000.0, 0.2, N));
    println!("corte 3 kHz: {g:.2} dB a 12 kHz");
    assert!(g < -15.0, "{g}");
    let c = toma_uno::efectos::curva_eq(&p, [12000.0].into_iter())[0] as f32;
    assert!((c - g).abs() < 1.0, "la curva dibujada ({c}) coincide con el sonido ({g})");
}

#[test]
fn compresor() {
    let p = [-20.0, 4.0, 5.0, 100.0, 0.0];
    let fuerte = cambio(TipoFx::Compresor, &p, &seno(440.0, 1.0, 2 * N));
    let quedo = cambio(TipoFx::Compresor, &p, &seno(440.0, 0.02, 2 * N));
    println!("compresor 4:1 a -20: fuerte {fuerte:.2} dB, quedo {quedo:.2} dB");
    // 4:1 sobre unos 17-20 dB por encima del umbral da de -12 a -15 dB.
    assert!((-16.0..-11.0).contains(&fuerte), "{fuerte}");
    assert!(quedo.abs() < 0.5, "{quedo}");
    let mas = cambio(TipoFx::Compresor, &[-20.0, 4.0, 5.0, 100.0, 6.0], &seno(440.0, 0.02, N));
    assert!((mas - 6.0).abs() < 0.5, "{mas}");
}

#[test]
fn compuerta_y_limitador() {
    let p = TipoFx::Compuerta.defaults();
    let ruido = cambio(TipoFx::Compuerta, &p, &seno(440.0, db(-60.0), 2 * N));
    let voz = cambio(TipoFx::Compuerta, &p, &seno(440.0, db(-12.0), 2 * N));
    println!("compuerta -45: ruido {ruido:.1} dB, voz {voz:.2} dB");
    assert!(ruido < -30.0 && voz.abs() < 0.5);
    let y = pasar(TipoFx::Limitador, &[0.0, -1.0, 60.0], &seno(200.0, 2.5, N));
    let pico = y[N..].iter().fold(0f32, |m, v| m.max(v.abs()));
    println!("limitador: pico de +8 dB sale en {:.2} dB", a_db(pico));
    assert!(pico <= db(-1.0) * 1.01 && pico > db(-2.5));
    let g = cambio(TipoFx::Limitador, &TipoFx::Limitador.defaults(), &seno(200.0, 0.3, N));
    assert!(g.abs() < 0.05, "no toca lo quedo: {g}");
}

#[test]
fn espacio() {
    let mut x = vec![0f32; 2 * 4 * N];
    (x[0], x[1]) = (1.0, 1.0);
    let cola = |p: &[f32], a: f32, b: f32| {
        let y = pasar(TipoFx::Reverberacion, p, &x);
        let i = |s: f32| (s * SR as f32) as usize * 2;
        y[i(a)..i(b)].iter().map(|v| v * v).sum::<f32>()
    };
    let corta = [45.0, 0.5, 55.0, 20.0, 20.0, 12000.0];
    let larga = [45.0, 6.0, 55.0, 20.0, 20.0, 12000.0];
    let (c, l) = (cola(&corta, 1.0, 2.0), cola(&larga, 1.0, 2.0));
    println!("reverberación 1-2 s: corta {c:e}, larga {l:e}");
    assert!(l > 100.0 * c && l > 1e-6);
    assert!(cola(&[0.0, 6.0, 55.0, 20.0, 20.0, 12000.0], 0.1, 2.0) < 1e-9, "mezcla 0 no deja cola");

    let eco = |ms: f32, rep: f32| pasar(TipoFx::Eco, &[ms, rep, 100.0, 20000.0, 0.0], &x);
    for ms in [100.0, 350.0, 800.0] {
        let y = eco(ms, 0.0);
        let i = (N / 100..2 * N).max_by(|&a, &b| y[2 * a].abs().total_cmp(&y[2 * b].abs())).unwrap();
        let medido = i as f32 * 1000.0 / SR as f32;
        println!("eco de {ms} ms: medido {medido:.1} ms");
        assert!((medido - ms).abs() < ms * 0.02, "{ms} -> {medido}");
    }
    let e = |y: &[f32]| y[2 * N..].iter().map(|v| v * v).sum::<f32>();
    assert!(e(&eco(350.0, 60.0)) > 10.0 * e(&eco(350.0, 0.0)).max(1e-9));
}

#[test]
fn color() {
    let x = seno(440.0, 0.3, 2 * N);
    for t in [TipoFx::Coro, TipoFx::Faser, TipoFx::Saturador, TipoFx::Flanger] {
        let y = pasar(t, &t.defaults(), &x);
        assert!(y.iter().all(|v| v.is_finite()));
        let dif: f32 = y.iter().zip(&x).map(|(a, b)| (a - b) * (a - b)).sum::<f32>() / x.iter().map(|v| v * v).sum::<f32>();
        println!("{}: cambia {:.0} % de la energía, nivel {:+.1} dB", t.nombre(), dif * 100.0, nivel(&y) - nivel(&x));
        assert!(dif > 0.005);
        assert!((nivel(&y) - nivel(&x)).abs() < 12.0);
    }
    // Tercer armónico del saturador.
    let y = pasar(TipoFx::Saturador, &[80.0, 100.0, 0.0], &x);
    let arm = |h: f32| {
        let (mut a, mut b) = (0.0, 0.0);
        for i in 0..N {
            let w = std::f32::consts::TAU * 440.0 * h * i as f32 / SR as f32;
            a += y[2 * i] * w.cos();
            b += y[2 * i] * w.sin();
        }
        (a * a + b * b).sqrt() / N as f32 * 2.0
    };
    println!("saturador: 3er armónico {:.1} % del tono", 100.0 * arm(3.0) / arm(1.0));
    assert!(arm(3.0) > 0.05 * arm(1.0));
}

#[test]
fn todos_los_extremos_son_estables() {
    let mut x = seno(220.0, 0.9, N);
    x.extend(vec![0.0; N]);
    for t in TipoFx::TODOS {
        for lado in 0..2 {
            let p: Vec<f32> = t.params().iter().map(|p| if lado == 0 { p.min } else { p.max }).collect();
            let y = pasar(t, &p, &x);
            let m = y.iter().fold(0f32, |m, v| m.max(v.abs()));
            // Las siete bandas al tope en el mismo lugar suman 126 dB de verdad.
            let tope = if t == TipoFx::Parametrico { 1e5 } else { 50.0 };
            assert!(y.iter().all(|v| v.is_finite()) && m < tope, "{} lado {lado}: {m}", t.nombre());
        }
    }
}

/// El paramétrico, el balance, el moldeador y el vocoder hacen lo que dicen.
#[test]
fn parametrico_y_estereo() {
    let nivel = |y: &[f32], lado: usize| (y[y.len() / 2..].iter().skip(lado).step_by(2).map(|v| v * v).sum::<f32>() / (y.len() / 4) as f32).sqrt();
    let x = seno(1000.0, 0.2, N);
    let mut p = TipoFx::Parametrico.defaults();
    let plano = pasar(TipoFx::Parametrico, &p, &x);
    assert!((nivel(&plano, 0) / nivel(&x, 0) - 1.0).abs() < 0.03, "neutro no cambia");
    p[3 * 4 + 2] = 12.0;
    let g = 20.0 * (nivel(&pasar(TipoFx::Parametrico, &p, &x), 0) / nivel(&x, 0)).log10();
    assert!((g - 12.0).abs() < 0.6, "campana de 12 dB en 1 kHz: {g}");
    p[3 * 4] = 3.0;
    p[3 * 4 + 1] = 8000.0;
    let g = 20.0 * (nivel(&pasar(TipoFx::Parametrico, &p, &x), 0) / nivel(&x, 0)).log10();
    assert!(g < -12.0, "pasa agudos en 8 kHz: {g}");
    let y = pasar(TipoFx::Balance, &[100.0, 100.0], &x);
    assert!(nivel(&y, 0) < 0.05 * nivel(&y, 1), "todo a la derecha");
    // Solo el lado izquierdo suena; a mono suena en los dos.
    let izq: Vec<f32> = x.iter().enumerate().map(|(i, v)| if i % 2 == 0 { *v } else { 0.0 }).collect();
    let y = pasar(TipoFx::Moldeador, &[50.0, 50.0, 50.0, 50.0, 0.0, 0.0], &izq);
    assert!((nivel(&y, 0) / nivel(&y, 1) - 1.0).abs() < 0.05 && nivel(&y, 1) > 0.02, "{} {}", nivel(&y, 0), nivel(&y, 1));
    // Sin voz, el vocoder calla; con voz, suena su acorde.
    let quieto = pasar(TipoFx::Vocoder, &TipoFx::Vocoder.defaults(), &vec![0.0; x.len()]);
    let suena = pasar(TipoFx::Vocoder, &TipoFx::Vocoder.defaults(), &seno(220.0, 0.5, N));
    assert!(nivel(&quieto, 0) < 1e-3 && nivel(&suena, 0) > 5e-3, "{} {}", nivel(&quieto, 0), nivel(&suena, 0));
}

// ---- mezcladores

/// Un clip o una pista por un mezclador: su volumen y sus efectos, callar, solo, y que no se atrase.
#[test]
fn mezcladores() {
    let x: Vec<f32> = seno(440.0, 0.5, N);
    let rms = |y: &[f32]| (y.iter().map(|v| v * v).sum::<f32>() / y.len() as f32).sqrt();
    let (p0, a, clip) = proyecto(x.clone());
    let seco = mezcla(&p0, &a, 0.5);
    // Sin tocar nada, pasar por un mezclador no cambia el sonido.
    let con = |f: &dyn Fn(&mut Proyecto, u64)| {
        let mut p = p0.clone();
        let b = p.nuevo_bus("Voces");
        f(&mut p, b);
        mezcla(&p, &a, 0.5)
    };
    let por_clip = |p: &mut Proyecto, b: u64| p.clip(clip).unwrap().bus = Some(b);
    let por_pista = |p: &mut Proyecto, b: u64| p.pistas[0].bus = Some(b);
    let igual = |y: &[f32], z: &[f32]| y.iter().zip(z).all(|(a, b)| (a - b).abs() < 1e-5);
    assert!(igual(&con(&por_clip), &seco) && igual(&con(&por_pista), &seco));
    // El volumen del mezclador; el clip directo no pasa por el de la pista.
    let baja = con(&|p, b| {
        por_clip(p, b);
        p.buses[0].vol_db = -6.0;
        p.pistas[0].vol_db = -60.0;
    });
    assert!((rms(&baja) / rms(&seco) - 0.501).abs() < 0.01, "{}", rms(&baja) / rms(&seco));
    let baja = con(&|p, b| {
        por_pista(p, b);
        p.buses[0].vol_db = -6.0;
        p.pistas[0].vol_db = -6.0;
    });
    assert!((rms(&baja) / rms(&seco) - 0.251).abs() < 0.01, "la pista y luego el mezclador");
    // Callarlo, y callar la pista del clip.
    for f in [&(|p: &mut Proyecto, b: u64| { por_clip(p, b); p.buses[0].mudo = true; }) as &dyn Fn(&mut Proyecto, u64), &|p, b| { por_clip(p, b); p.pistas[0].mudo = true; }] {
        assert!(rms(&con(f)) < 1e-5);
    }
    // Solo: lo que va a ese mezclador se oye y lo demás no.
    let mut p = p0.clone();
    let b = p.nuevo_bus("A");
    let otra = p.nueva_pista("otra");
    let mut c = p.clip(clip).unwrap().clone();
    c.id = p.id();
    c.bus = Some(b);
    p.pista(otra).unwrap().clips.push(c);
    let doble = mezcla(&p, &a, 0.5);
    p.buses[0].solo = true;
    let sola = mezcla(&p, &a, 0.5);
    assert!((rms(&doble) / rms(&sola) - 2.0).abs() < 0.05 && igual(&sola, &seco));
    // Un efecto con retardo en el mezclador no atrasa: todo sale a tiempo, también desde en medio.
    for por in [&por_clip as &dyn Fn(&mut Proyecto, u64), &por_pista] {
        let mut p = p0.clone();
        let b = p.nuevo_bus("Limitado");
        por(&mut p, b);
        let f = p.nuevo_fx(TipoFx::Limitador);
        p.buses[0].fx.push(Fx { p: vec![0.0, 0.0, 60.0], ..f });
        assert!(crear(TipoFx::Limitador, &p.buses[0].fx[0].p).latencia() > 0);
        for ini in [0i64, 7000] {
            let mut y = vec![];
            motor::mezclar_desde(&p, &a, ini, 8000, |_, b| { y.extend_from_slice(b); true });
            let z = &x[ini as usize * 2..][..y.len().min(x.len() - ini as usize * 2)];
            let dif = y.iter().zip(z).skip(400).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
            assert!(dif < 0.03, "a tiempo desde {ini}: {dif}");
        }
    }
    // Quitarlo regresa todo a su pista.
    let mut p = p0.clone();
    let b = p.nuevo_bus("X");
    por_clip(&mut p, b);
    por_pista(&mut p, b);
    p.quitar_bus(b);
    assert!(p == Proyecto { sig_id: p.sig_id, ..p0.clone() });
}

// ---- mezcla

fn audio(x: Vec<f32>) -> Arc<Audio> {
    Arc::new(Audio::nuevo(x.chunks(2).map(|f| [f[0], f[1]]).collect()))
}

fn proyecto(x: Vec<f32>) -> (Proyecto, Almacen, u64) {
    let mut p = Proyecto::nuevo();
    p.fx_maestro.clear();
    p.pistas.clear();
    let pista = p.nueva_pista("a");
    let a = audio(x);
    let total = a.dur();
    let id = p.id();
    let ruta = PathBuf::from("/x.wav");
    p.pista(pista).unwrap().clips.push(Clip {
        id, ruta: ruta.clone(), nombre: "x".into(), inicio: 0.0, desde: 0.0, dur: total, total,
        ganancia_db: 0.0, fade_in: 0.0, fade_out: 0.0, invertido: false, grupo: None, velocidad: 1.0, bus: None, color: None
    });
    (p, Almacen::from([(ruta, a)]), id)
}

fn mezcla(p: &Proyecto, a: &Almacen, seg: f64) -> Vec<f32> {
    let mut v = vec![];
    motor::mezclar_todo(p, a, motor::cuadros(seg), |_, b| {
        v.extend_from_slice(b);
        true
    });
    v
}

fn ruido(n: usize) -> Vec<f32> {
    let mut s = 1u32;
    (0..2 * n)
        .map(|_| {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            (s >> 8) as f32 / (1 << 24) as f32 - 0.5
        })
        .collect()
}

#[test]
fn recorte_movimiento_volumen() {
    let x = ruido(N);
    let (mut p, a, id) = proyecto(x.clone());
    assert_eq!(mezcla(&p, &a, 1.0), x, "sin cambios sale idéntico");
    let c = p.clip(id).unwrap();
    (c.inicio, c.desde, c.dur, c.ganancia_db) = (0.5, 0.25, 0.5, -6.0);
    p.pistas[0].vol_db = -6.0;
    let y = mezcla(&p, &a, 1.5);
    assert!(y[..N].iter().all(|v| *v == 0.0), "silencio antes del clip");
    assert!(y[2 * N..].iter().all(|v| *v == 0.0), "silencio después");
    let k = 2 * (0.25 * N as f64) as usize;
    for i in 0..N {
        assert!((y[N + i] - x[k + i] * db(-12.0)).abs() < 1e-5);
    }
    p.pistas[0].mudo = true;
    assert!(mezcla(&p, &a, 1.5).iter().all(|v| *v == 0.0));
}

#[test]
fn dividir_no_cambia_el_sonido() {
    for inv in [false, true] {
        let (mut p, a, id) = proyecto(ruido(N));
        p.clip(id).unwrap().invertido = inv;
        let antes = mezcla(&p, &a, 1.0);
        assert!(p.dividir(id, 0.3));
        assert_eq!(p.pistas[0].clips.len(), 2);
        assert_eq!(mezcla(&p, &a, 1.0), antes, "invertido {inv}");
    }
}

#[test]
fn quitar_tramo() {
    let x = ruido(N);
    let (mut p, a, id) = proyecto(x.clone());
    p.quitar_tramo(id, 0.25, 0.5, true);
    let y = mezcla(&p, &a, 1.0);
    let q = N / 2;
    assert_eq!(y[..q], x[..q]);
    assert_eq!(y[q..2 * N - q], x[2 * q..]);
    assert!(y[2 * N - q..].iter().all(|v| *v == 0.0));
}

#[test]
fn efectos_con_retardo_quedan_alineados() {
    // Un clic en 0.5 s por una pista limpia y por otra con limitador; y con limitador en el maestro.
    let mut x = vec![0f32; 2 * N];
    (x[N], x[N + 1]) = (0.5, 0.5);
    for (en_pista, en_maestro) in [(false, false), (true, false), (false, true), (true, true)] {
        let (mut p, a, _) = proyecto(x.clone());
        let mut f = p.nuevo_fx(TipoFx::Limitador);
        let mut g = p.nuevo_fx(TipoFx::Limitador);
        (f.p, g.p) = (TipoFx::Limitador.defaults(), TipoFx::Limitador.defaults());
        if en_pista {
            p.pistas[0].fx.push(f);
        }
        if en_maestro {
            p.fx_maestro.push(g);
        }
        let y = mezcla(&p, &a, 1.0);
        let i = (0..N).max_by(|&a, &b| y[2 * a].abs().total_cmp(&y[2 * b].abs())).unwrap();
        println!("limitador pista={en_pista} maestro={en_maestro}: clic en el cuadro {i}");
        assert_eq!(i, N / 2);
    }
}

#[test]
fn aplicar_cambios_en_vivo_conserva_posicion() {
    let (mut p, a, _) = proyecto(ruido(4 * N));
    let mut m = motor::Motor::default();
    m.aplicar(motor::preparar(&p, &a, &[]));
    m.tocando = true;
    let mut b = [0f32; 512];
    (0..100).for_each(|_| m.bloque(&mut b));
    let pos = m.pos;
    let f = p.nuevo_fx(TipoFx::Limitador);
    p.pistas[0].fx.push(f);
    let ids = m.ids_fx();
    m.aplicar(motor::preparar(&p, &a, &ids));
    assert_eq!(m.pos, pos);
    m.bloque(&mut b);
    assert!(b.iter().any(|v| *v != 0.0));
}

#[test]
fn proyecto_ida_y_vuelta() {
    let (p, ..) = proyecto(ruido(10));
    let s = serde_json::to_string(&p).unwrap();
    assert_eq!(serde_json::from_str::<Proyecto>(&s).unwrap(), p);
}

#[test]
fn escucharme() {
    // La voz en vivo pasa por el compresor de la pista armada, con la canción detenida o sonando, y no la mueve.
    let (mut p, a, _) = proyecto(vec![0.0; 2 * N]);
    let id = p.pistas[0].id;
    p.armar(id);
    let mut f = p.nuevo_fx(TipoFx::Compresor);
    f.p = vec![-20.0, 4.0, 5.0, 100.0, 0.0];
    for (con_fx, tocando) in [(false, false), (true, false), (true, true)] {
        p.pistas[0].fx = if con_fx { vec![f.clone()] } else { vec![] };
        let vivo = motor::Vivo::default();
        let x = seno(440.0, 1.0, N);
        let mut puente = toma_uno::audio::Puente::nuevo(44100, vivo.clone());
        let mut m = motor::Motor::default();
        m.aplicar(motor::preparar(&p, &a, &[]));
        m.vivo = Some(vivo.clone());
        m.tocando = tocando;
        m.grabando = true;
        let mut y = vec![];
        let mut b = [0f32; 256];
        // El micrófono a 44.1 kHz entrega 118 cuadros por cada 128 que pide la salida (117.6 en promedio).
        let mut de = 0;
        for k in 0..340 {
            let hasta = ((k + 1) as f32 * 117.6) as usize;
            puente.meter(x[2 * de..2 * hasta].chunks(2).map(|f| [f[0], f[1]]));
            de = hasta;
            m.bloque(&mut b);
            y.extend_from_slice(&b);
        }
        let g = nivel(&y[..y.len()]) - nivel(&x);
        println!("escucharme fx={con_fx} tocando={tocando}: {g:+.2} dB, cola {} cuadros, posición {}", vivo.lock().unwrap().len(), m.pos);
        assert!(if con_fx { (-16.0..-10.0).contains(&g) } else { g.abs() < 0.3 }, "{g}");
        assert!(vivo.lock().unwrap().len() <= 768);
        assert_eq!(m.pos, if tocando { 340 * 128 } else { 0 });
    }
}

#[test]
fn los_demas_efectos() {
    // Recorte suave: lo quedo pasa igual, lo fuerte se redondea.
    let r = TipoFx::RecorteSuave.defaults();
    let (q, f) = (cambio(TipoFx::RecorteSuave, &r, &seno(440.0, 0.05, N)), cambio(TipoFx::RecorteSuave, &r, &seno(440.0, 1.5, N)));
    println!("recorte suave: quedo {q:+.2} dB, fuerte {f:+.2} dB");
    assert!(q.abs() < 0.2 && f < -1.5);
    // Multibanda: empujar los graves sube 60 Hz y casi no toca 6 kHz.
    let base = [200.0, 3000.0, 0.0, 0.0, 0.0, 0.0, 120.0, 0.0];
    let mut g = base;
    g[2] = 6.0;
    let d = |hz: f32| cambio(TipoFx::Multibanda, &g, &seno(hz, 0.05, 2 * N)) - cambio(TipoFx::Multibanda, &base, &seno(hz, 0.05, 2 * N));
    println!("multibanda graves +6: 60 Hz {:+.1} dB, 6 kHz {:+.1} dB", d(60.0), d(6000.0));
    assert!((d(60.0) - 6.0).abs() < 1.0 && d(6000.0).abs() < 0.5);
    let y = pasar(TipoFx::Multibanda, &[200.0, 3000.0, 6.0, 6.0, 6.0, 6.0, 120.0, -1.0], &seno(300.0, 0.9, N));
    let pico = y[N..].iter().fold(0f32, |m, v| m.max(v.abs()));
    println!("multibanda empujada: pico {:.2} dB", a_db(pico));
    assert!(pico <= 1.0);
    // Filtro pasa graves a 500 Hz.
    let f = [500.0, 0.0, 100.0, 0.0, 0.0];
    let (b, a) = (cambio(TipoFx::Filtro, &f, &seno(80.0, 0.2, N)), cambio(TipoFx::Filtro, &f, &seno(6000.0, 0.2, N)));
    println!("filtro 500 Hz: 80 Hz {b:+.1} dB, 6 kHz {a:+.1} dB");
    assert!(b.abs() < 1.5 && a < -25.0);
    // Amplitud: abrir sube la diferencia entre lados, cerrar la quita.
    let mut x = seno(440.0, 0.3, N);
    x.chunks_mut(2).enumerate().for_each(|(i, f)| f[1] = 0.3 * (std::f32::consts::TAU * 555.0 * i as f32 / SR as f32).sin());
    let lado = |y: &[f32]| y[N..].chunks(2).map(|f| (f[0] - f[1]).powi(2)).sum::<f32>();
    let (ab, ce) = (lado(&pasar(TipoFx::Amplitud, &[60.0, 0.0], &x)), lado(&pasar(TipoFx::Amplitud, &[-100.0, 0.0], &x)));
    println!("amplitud: lados abierto {:.2}x, cerrado {:.4}x", ab / lado(&x), ce / lado(&x));
    assert!(ab > 1.2 * lado(&x) && ce < 0.05 * lado(&x));
    // Transitorios: más ataque sube el golpe frente a la cola.
    let mut g = vec![0f32; 2 * N];
    for i in 0..N { let t = (i % 12000) as f32 / SR as f32; g[2 * i] = (-t * 25.0).exp() * (t * 900.0).sin() * 0.4; g[2 * i + 1] = g[2 * i]; }
    let golpe = |y: &[f32]| { let k = 2 * 24000; y[k..k + 960].iter().map(|v| v * v).sum::<f32>() / y[k + 4800..k + 9600].iter().map(|v| v * v).sum::<f32>() };
    let (mas, menos) = (golpe(&pasar(TipoFx::Transitorios, &[80.0, 0.0, 0.0], &g)), golpe(&pasar(TipoFx::Transitorios, &[-80.0, 0.0, 0.0], &g)));
    println!("transitorios: golpe/cola con más {mas:.1}, con menos {menos:.1}, sin efecto {:.1}", golpe(&g));
    assert!(mas > menos * 1.3);
}

#[test]
fn instrumentos_y_patrones() {
    use toma_uno::instrumentos::{PIEZAS, SONIDOS};
    let mut p = Proyecto::nuevo();
    p.fx_maestro.clear();
    p.pistas.clear();
    p.bpm = 120.0;
    let (pista, pat) = p.nueva_pista_instrumento("s", Instrumento::Trio(SONIDOS[4].1.to_vec()), 1.0);
    p.patron(pat).unwrap().notas = vec![Nota { tecla: 69, vel: 100, ini: 1.0, dur: 1.0 }];
    p.pista(pista).unwrap().patrones[0].dur = 4.0;
    let a = Almacen::new();
    // La nota va en el pulso 1: 1.0 + 0.5 = 1.5 s, y se repite a los 2 s (el patrón dura 4 pulsos).
    let y = mezcla(&p, &a, 6.0);
    let e = |s: f32, d: f32| y[(s * SR as f32) as usize * 2..((s + d) * SR as f32) as usize * 2].iter().map(|v| v * v).sum::<f32>();
    println!("antes {:.5}, nota {:.2}, entre {:.5}, repetición {:.2}", e(1.0, 0.49), e(1.5, 0.4), e(2.5, 0.9), e(3.5, 0.4));
    assert!(e(0.0, 1.49) == 0.0 && e(1.5, 0.4) > 1.0 && e(2.6, 0.85) < 0.01 * e(1.5, 0.4) && e(3.5, 0.4) > 1.0);
    // Afinación: La 4 son 440 Hz.
    let x: Vec<f32> = y[(1.6 * SR as f32) as usize * 2..(1.9 * SR as f32) as usize * 2].chunks(2).map(|f| f[0]).collect();
    let fuerza = |hz: f32| {
        let (mut c, mut s) = (0.0, 0.0);
        for (i, v) in x.iter().enumerate() {
            let w = std::f32::consts::TAU * hz * i as f32 / SR as f32;
            c += v * w.cos();
            s += v * w.sin();
        }
        c * c + s * s
    };
    let mejor = (800..960).map(|k| k as f32 / 2.0).max_by(|a, b| fuerza(*a).total_cmp(&fuerza(*b))).unwrap();
    println!("tono medido: {mejor} Hz");
    assert!((mejor - 440.0).abs() < 2.0);
    for (n, v) in SONIDOS {
        p.pista(pista).unwrap().instrumento = Some(Instrumento::Trio(v.to_vec()));
        let y = mezcla(&p, &a, 3.0);
        let pico = y.iter().fold(0f32, |m, v| m.max(v.abs()));
        println!("{n}: pico {:.1} dB", a_db(pico));
        assert!(pico > 0.02 && pico < 1.5 && y.iter().all(|v| v.is_finite()));
    }
    p.pista(pista).unwrap().instrumento = Some(Instrumento::Bateria);
    for (t, n) in PIEZAS {
        p.patron(pat).unwrap().notas = vec![Nota { tecla: t, vel: 110, ini: 0.0, dur: 0.25 }];
        let y = mezcla(&p, &a, 2.0);
        let pico = y.iter().fold(0f32, |m, v| m.max(v.abs()));
        println!("{n}: pico {:.1} dB", a_db(pico));
        assert!(pico > 0.1 && pico <= 1.2);
    }
    // Muestra: una octava arriba suena al doble de rápido.
    let ruta = PathBuf::from("/m.wav");
    let a = Almacen::from([(ruta.clone(), audio(seno(220.0, 0.5, 2 * N)))]);
    p.pista(pista).unwrap().instrumento = Some(Instrumento::Muestra { ruta, raiz: 60 });
    p.patron(pat).unwrap().notas = vec![Nota { tecla: 72, vel: 127, ini: 0.0, dur: 1.0 }];
    let y = mezcla(&p, &a, 2.0);
    let cruces = y[(1.05 * SR as f32) as usize * 2..(1.45 * SR as f32) as usize * 2].chunks(2).map(|f| f[0]).collect::<Vec<_>>().windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
    println!("muestra una octava arriba: {:.0} Hz", cruces as f32 / 0.4);
    assert!((cruces as f32 / 0.4 - 440.0).abs() < 8.0);
}

#[test]
fn automatizacion() {
    let (mut p, a, _) = proyecto(seno(440.0, 0.5, 4 * N));
    p.pistas[0].auto.push(Automatizacion { destino: Destino::Volumen, activa: true, puntos: vec![(1.0, 0.0), (3.0, -40.0)] });
    let y = mezcla(&p, &a, 4.0);
    let n = |s: f32| a_db(y[(s * SR as f32) as usize * 2..((s + 0.05) * SR as f32) as usize * 2].iter().fold(0f32, |m, v| m.max(v.abs())) / 0.5);
    println!("volumen: 0.5 s {:.1}, 2 s {:.1}, 3.5 s {:.1} dB", n(0.5), n(2.0), n(3.5));
    assert!(n(0.5).abs() < 0.2 && (n(2.0) + 20.0).abs() < 0.7 && (n(3.5) + 40.0).abs() < 0.3);
    // Un control de efecto: el corte de graves sube y quita el tono.
    let mut f = p.nuevo_fx(TipoFx::Ecualizador);
    f.p = TipoFx::Ecualizador.defaults();
    p.pistas[0].auto = vec![Automatizacion { destino: Destino::Fx(f.id, 6), activa: true, puntos: vec![(1.0, 20000.0), (1.5, 2000.0)] }];
    p.pistas[0].fx.push(f);
    let (mut q, b, _) = proyecto(seno(8000.0, 0.5, 4 * N));
    q.pistas[0].fx = p.pistas[0].fx.clone();
    q.pistas[0].auto = p.pistas[0].auto.clone();
    let y = mezcla(&q, &b, 4.0);
    let n = |s: f32| a_db(y[(s * SR as f32) as usize * 2..((s + 0.05) * SR as f32) as usize * 2].iter().fold(0f32, |m, v| m.max(v.abs())) / 0.5);
    println!("filtro automatizado a 8 kHz: antes {:.1}, después {:.1} dB", n(0.5), n(2.5));
    assert!(n(0.5) > -2.0 && n(2.5) < -12.0);
}

#[test]
fn efectos_propios() {
    use toma_uno::propios::*;
    // Detector: el tono de un diente de sierra.
    let sierra = |hz: f32, n: usize| -> Vec<f32> {
        (0..n).flat_map(|i| [(i as f32 * hz / SR as f32).fract() * 0.5 - 0.25; 2]).collect()
    };
    let tono = |x: &[f32]| {
        let mut d = Detector::nuevo();
        let mut v = vec![];
        for f in x.chunks_exact(4).skip(x.len() / 8) {
            d.meter(0.25 * (f[0] + f[1] + f[2] + f[3]));
            v.extend(d.hz);
        }
        v.sort_by(f32::total_cmp);
        v.get(v.len() / 2).copied().unwrap_or(0.0)
    };
    for hz in [110.0, 220.0, 440.0, 700.0] {
        let m = tono(&sierra(hz, N));
        println!("detector {hz}: {m:.1}");
        assert!((m - hz).abs() < hz * 0.01);
    }
    // Afinador: un La a 452 Hz (46 cents alto) queda en 440.
    let y = pasar(TipoFx::Afinador, &[0.0, 1.0, 10.0, 100.0, 0.0], &sierra(452.0, 2 * N));
    let m = tono(&y[2 * N..]);
    println!("afinador: 452 -> {m:.1} Hz");
    assert!((m - 440.0).abs() < 3.0);
    // En Do mayor, un Sol# cercano a Sol se va a Sol (392) o La (440), nunca se queda.
    let m = tono(&pasar(TipoFx::Afinador, &[0.0, 1.0, 10.0, 100.0, 0.0], &sierra(408.0, 2 * N))[2 * N..]);
    println!("afinador en Do mayor: 408 -> {m:.1} Hz");
    assert!((m - 392.0).abs() < 4.0);
    for s in [-5.0, 7.0] {
        let m = tono(&pasar(TipoFx::CambioTono, &[s, 0.0, 100.0], &sierra(300.0, 2 * N))[2 * N..]);
        let ideal = 300.0 * 2f32.powf(s / 12.0);
        println!("cambio de tono {s:+}: {m:.1} Hz (ideal {ideal:.1})");
        assert!((m - ideal).abs() < ideal * 0.015);
    }
    // Eses: baja 8 kHz fuerte y no toca 500 Hz.
    let p = [5500.0, -40.0, 18.0];
    let (alto, bajo) = (cambio(TipoFx::Seseo, &p, &seno(8000.0, 0.4, N)), cambio(TipoFx::Seseo, &p, &seno(500.0, 0.4, N)));
    println!("eses: 8 kHz {alto:+.1} dB, 500 Hz {bajo:+.1} dB");
    assert!(alto < -8.0 && bajo.abs() < 0.5);
    // Armonías: tercera de Do mayor sobre Do y sobre Re.
    let mayor = ESCALAS[1].1;
    assert_eq!((intervalo(60.0, 0, mayor, 2), intervalo(62.0, 0, mayor, 2), intervalo(60.0, 0, mayor, -1)), (4.0, 3.0, -1.0));
    // Graves profundos: de 80 Hz sale algo a 40 Hz.
    let y = pasar(TipoFx::SubGrave, &[110.0, 100.0, 0.0], &seno(80.0, 0.4, N));
    let en = |hz: f32| {
        let (mut a, mut b) = (0.0f32, 0.0f32);
        for (i, f) in y.chunks_exact(2).enumerate().skip(N / 2) {
            let w = std::f32::consts::TAU * hz * i as f32 / SR as f32;
            a += f[0] * w.sin();
            b += f[0] * w.cos();
        }
        a.hypot(b) * 2.0 / (N / 2) as f32
    };
    println!("sub: 40 Hz {:.3}, 80 Hz {:.3}", en(40.0), en(80.0));
    assert!(en(40.0) > 0.1);
    // Karaoke: quita el centro y deja los lados.
    let centro = cambio(TipoFx::Karaoke, &TipoFx::Karaoke.defaults(), &seno(1000.0, 0.4, N));
    let mut l = seno(1000.0, 0.4, N);
    l.chunks_mut(2).for_each(|f| f[1] = -f[1]);
    let lados = cambio(TipoFx::Karaoke, &TipoFx::Karaoke.defaults(), &l);
    let graves = cambio(TipoFx::Karaoke, &TipoFx::Karaoke.defaults(), &seno(40.0, 0.4, N));
    println!("karaoke: centro {centro:+.1} dB, lados {lados:+.1} dB, graves {graves:+.1} dB");
    assert!(centro < -12.0 && lados.abs() < 0.1 && graves.abs() < 2.0);
    let x = sierra(220.0, N);
    for t in [TipoFx::Doblador, TipoFx::Excitador, TipoFx::Tremolo, TipoFx::Autopaneo, TipoFx::Wah, TipoFx::Robot, TipoFx::Bits, TipoFx::Armonias, TipoFx::Vibrato, TipoFx::Telefono, TipoFx::Vinilo, TipoFx::Tartamudeo, TipoFx::SubGrave] {
        let y = pasar(t, &t.defaults(), &x);
        let d: f32 = y.iter().zip(&x).map(|(a, b)| (a - b).powi(2)).sum::<f32>() / x.iter().map(|v| v * v).sum::<f32>();
        println!("{}: cambia {:.0} %, nivel {:+.1} dB", t.nombre(), d * 100.0, nivel(&y) - nivel(&x));
        assert!(d > 0.005 && y.iter().all(|v| v.is_finite()) && (nivel(&y) - nivel(&x)).abs() < 12.0);
    }
}
