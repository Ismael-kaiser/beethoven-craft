use beethoven_daw_demo::biblioteca::Biblioteca;
use beethoven_daw_demo::lua::{self, Tipo};
use beethoven_daw_demo::modelo::*;
use beethoven_daw_demo::motor::{Escena, Modo, Motor, Orden, a_audio};
use beethoven_daw_demo::{flp, midi, muestrario, receta, wav};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::BTreeMap;

// Cuenta la memoria pedida o soltada en este hilo.
struct Contador;
thread_local!(static CUANTAS: Cell<u64> = const { Cell::new(0) });
unsafe impl GlobalAlloc for Contador {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let _ = CUANTAS.try_with(|c| c.set(c.get() + 1));
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        let _ = CUANTAS.try_with(|c| c.set(c.get() + 1));
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static A: Contador = Contador;

fn bib() -> Biblioteca {
    Biblioteca::nueva(48000.0, None)
}

fn una_nota(ins: &str, tono: u8, pasos: u32) -> Proyecto {
    let mut p = Proyecto::default();
    let c = p.agregar_canal(ins, ins);
    p.patrones[0].notas.insert(c, vec![Nota { tick: 0, dur: pasos * PASO, tono, fuerza: 1.0 }]);
    p
}

/// Frecuencia por autocorrelación.
fn afinacion(x: &[[f32; 2]], sr: f32, lo: f32, hi: f32) -> f32 {
    let x: Vec<f32> = x.iter().map(|s| s[0]).collect();
    let (a, b) = ((sr / hi) as usize, (sr / lo) as usize);
    let n = x.len() - b - 2;
    let c = |d: usize| -> f64 { (0..n).map(|i| x[i] as f64 * x[i + d] as f64).sum() };
    let m = (a..=b).max_by(|p, q| c(*p).total_cmp(&c(*q))).unwrap();
    let (y0, y1, y2) = (c(m - 1), c(m), c(m + 1));
    let d = m as f64 + 0.5 * (y0 - y2) / (y0 - 2.0 * y1 + y2);
    sr / d as f32
}

#[test]
fn todos_los_instrumentos_suenan_y_se_apagan() {
    // De fábrica solo queda el muestreador en Lua; los instrumentos son grabaciones del banco.
    assert_eq!(bib().de_tipo(Tipo::Instrumento).map(|g| g.clave.as_str()).collect::<Vec<_>>(), ["muestra"]);
    if beethoven_daw_demo::banco::archivo().is_none() {
        return eprintln!("sin banco: corre empaque/sonidos.sh");
    }
    // Uno de cada familia y todas las piezas de batería; cada uno con su biblioteca para no juntar memoria.
    let mut claves: Vec<&str> = beethoven_daw_demo::banco::familias().iter().map(|(_, v)| v[0].clave).collect();
    claves.extend(beethoven_daw_demo::banco::INSTRUMENTOS.iter().filter(|x| x.nota.is_some()).map(|x| x.clave));
    for k in claves {
        let mut b = bib();
        let x = a_audio(&una_nota(k, 60, 2), &mut b, Modo::Patron(1));
        let pico = x.iter().flatten().fold(0f32, |m, v| m.max(v.abs()));
        assert!(pico > 0.05 && pico <= 0.981, "{k}: pico {pico}");
        assert!(x.iter().flatten().all(|v| v.is_finite()));
        assert!(x[x.len() - 100..].iter().flatten().all(|v| v.abs() < 1e-3), "{k} no se apaga");
    }
}

#[test]
fn afinan() {
    let mut b = bib();
    if beethoven_daw_demo::banco::archivo().is_none() {
        return;
    }
    // Grabaciones con un tono claro; el órgano de barras tiene tantos armónicos que el medidor se confunde.
    for k in ["piano", "flauta", "violin", "cuerda"] {
        let mut p = una_nota(k, 69, 16);
        // Sin lo que mueve el tono a propósito.
        for c in ["ancho", "fuerzafm", "gira", "b1"] {
            p.canales[0].controles.insert(c.into(), 0.0);
        }
        let x = a_audio(&p, &mut b, Modo::Patron(1));
        let f = afinacion(&x[4800..24000], 48000.0, 300.0, 600.0);
        let cents = 1200.0 * (f / 440.0).log2();
        // Un piano de verdad va afinado un poco abierto: unos cuantos cents son normales en una grabación.
        assert!(cents.abs() < 12.0, "{k}: {f} Hz ({cents} cents)");
    }
}

#[test]
fn las_notas_caen_en_su_muestra() {
    let mut b = bib();
    let mut p = una_nota("bombo", 60, 1);
    let c = p.canales[0].id;
    // 120 bpm: medio compás = 1 s.
    p.patrones[0].notas.insert(c, vec![Nota { tick: COMPAS / 2, dur: PASO, tono: 60, fuerza: 1.0 }]);
    let x = a_audio(&p, &mut b, Modo::Patron(1));
    let i = x.iter().position(|s| s[0].abs() > 1e-4).unwrap();
    // La grabación del bombo arranca casi en silencio: menos de un milisegundo.
    assert!((48000..48048).contains(&i), "{i}");
}

#[test]
fn la_demo_suena_y_el_bucle_no_pierde_notas() {
    let mut b = bib();
    let p = Proyecto::demo();
    let x = a_audio(&p, &mut b, Modo::Cancion);
    let seg = x.len() as f32 / 48000.0;
    assert!(seg > 28.0 && seg < 42.0, "{seg}");
    // En bucle, dos vueltas de un patrón dan los mismos golpes en el mismo lugar.
    let (mut m, mando) = Motor::nuevo(Escena::nueva(&una_nota("bombo", 60, 1), &mut b, Modo::Patron(1)));
    mando.mandar(Orden::Tocar(true));
    let mut v = vec![[0.0; 2]; 96000 * 2 + 48000];
    v.chunks_mut(333).for_each(|c| m.tocar(c));
    for vuelta in [96000, 192000] {
        assert!(v[vuelta - 50..vuelta].iter().all(|s| s[0].abs() < 1e-3));
        assert!(v[vuelta..vuelta + 300].iter().any(|s| s[0].abs() > 0.1), "sin golpe en {vuelta}");
    }
}

#[test]
fn el_hilo_de_audio_no_pide_memoria() {
    let mut b = bib();
    let mut p = Proyecto::demo();
    for c in ["organo", "cuerda", "clasico", "aplauso"] {
        p.agregar_canal(c, c);
    }
    let ruta = std::env::temp_dir().join(format!("bdd-rt-{}.wav", std::process::id()));
    wav::escribir(&ruta, 48000, &[[0.1; 2]; 48000]).unwrap();
    let audio = ClipAudio { ruta: ruta.to_string_lossy().into(), nombre: "x".into(), inicio: 0.0, volumen: 1.0, mezcla: 1, ..Default::default() };
    p.clips.push(Clip { audio: Some(audio), ..Clip::de_patron(0, 5, 0, COMPAS * 4) });
    let (mut m, mando) = Motor::nuevo(Escena::nueva(&p, &mut b, Modo::Cancion));
    mando.mandar(Orden::Metronomo(true));
    let mut buf = [[0.0f32; 2]; 256];
    mando.mandar(Orden::Tocar(true));
    let mut total = 0;
    for i in 0..4000 {
        // Cambios de proyecto, notas a mano y saltos mientras suena.
        match i % 40 {
            3 => {
                p.canales[i % 5].volumen = 0.5;
                mando.mandar(Orden::Escena(Box::new(Escena::nueva(&p, &mut b, Modo::Cancion))));
            }
            9 => _ = mando.mandar(Orden::Nota { canal: 7 + (i as u32 / 40) % 4, tono: 60, fuerza: 0.9 }),
            20 => _ = mando.mandar(Orden::Soltar { canal: 7 + (i as u32 / 40) % 4, tono: 60 }),
            30 => _ = mando.mandar(Orden::Ir(i as f64)),
            _ => {}
        }
        let antes = CUANTAS.with(|c| c.get());
        m.tocar(&mut buf);
        total += CUANTAS.with(|c| c.get()) - antes;
    }
    assert_eq!(total, 0);
    std::fs::remove_file(ruta).unwrap();
}

#[test]
fn cambiar_el_proyecto_no_corta_lo_que_suena() {
    let mut b = bib();
    let p = una_nota("organo", 60, 16);
    let (mut m, mando) = Motor::nuevo(Escena::nueva(&p, &mut b, Modo::Patron(1)));
    mando.mandar(Orden::Tocar(true));
    let mut v = vec![];
    for i in 0..200 {
        if i > 20 {
            mando.mandar(Orden::Escena(Box::new(Escena::nueva(&p, &mut b, Modo::Patron(1)))));
        }
        let mut x = [[0.0; 2]; 128];
        m.tocar(&mut x);
        v.extend_from_slice(&x);
    }
    let salto = v[2000..].windows(2).map(|w| (w[0][0] - w[1][0]).abs()).fold(0f32, f32::max);
    assert!(salto < 0.08, "salto {salto}");
}

#[test]
fn la_receta_se_simplifica() {
    let f = r#"return { nombre="x", sonido=function(n,c)
      local basura = sierra(123) * 5
      local a = seno(n.hz) * (2 * 0.5)
      return a, seno(n.hz) * 1 end }"#;
    let (_, r) = lua::armar(f, 48000.0).unwrap();
    // hz, 0, 0.5, 1, seno y producto: lo repetido y lo que no suena se van.
    assert_eq!(r.unwrap().cuantos_nodos(), 6);
}

#[test]
fn lua_esta_cerrado_y_explica_los_errores() {
    let e = |s: &str| lua::armar(s, 48000.0).unwrap_err();
    for malo in [
        "return { nombre='x', sonido=function() return io.open('/etc/passwd') end }",
        "return { nombre='x', sonido=function() return os.execute('ls') end }",
        "return { nombre='x', sonido=function() return load('return 1')() end }",
        "return { nombre='x', sonido=function() return require('io') end }",
        "return { nombre='x', sonido=function() return string.dump(function() end) end }",
    ] {
        e(malo);
    }
    assert!(e("while true do end").contains("tardó demasiado"));
    e("local t = {} while true do t[#t+1] = string.rep('x', 100000) end");
    e("return pcall(function() while true do end end) and {} ");
    assert!(e("return { nombre='x', sonido=function(n) end }").contains("return"));
    assert!(e("return {\n nombre='x',\n sonido=function(n) return seno(nada) end }").contains("línea 3"));
    assert!(e("return { nombre='x', sonido=function(n) local s = n.hz for i=1,9999 do s = s + 1 end return s end }").contains("grande"));
    assert!(e("esto no es lua").contains("línea 1"));
}

#[test]
fn las_herramientas_transforman_notas() {
    let b = bib();
    let n = vec![Nota { tick: 0, dur: 4 * PASO, tono: 60, fuerza: 0.8 }];
    let corre = |k: &str, n: &[Nota], v: &[(&str, f32)]| {
        let v: BTreeMap<String, f32> = v.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        lua::herramienta(&b.ver(k).unwrap().fuente, n, COMPAS, &v).unwrap()
    };
    let acorde = corre("acorde", &n, &[("menor", 1.0)]);
    assert_eq!(acorde.iter().map(|n| n.tono).collect::<Vec<_>>(), [60, 63, 67]);
    let arp = corre("arpegio", &acorde, &[("paso", 1.0)]);
    assert_eq!(arp.iter().map(|n| (n.tick / PASO, n.tono)).collect::<Vec<_>>(), [(0, 60), (1, 63), (2, 67), (3, 60)]);
    assert_eq!(corre("reves", &n, &[])[0].tick, 12 * PASO);
    let chueca = [Nota { tick: PASO + 5, ..n[0] }];
    assert_eq!(corre("cuadrar", &chueca, &[])[0].tick, PASO);
    assert_eq!(corre("humano", &n, &[]).len(), 1);
}

#[test]
fn los_efectos_no_se_disparan() {
    let mut b = bib();
    let claves: Vec<String> = b.de_tipo(Tipo::Efecto).map(|g| g.clave.clone()).collect();
    for k in claves {
        for al_maximo in [false, true] {
            let mut p = una_nota("caja", 60, 1);
            let controles =
                if al_maximo { b.ver(&k).unwrap().ficha.controles.iter().map(|c| (c.clave.clone(), c.max)).collect() } else { BTreeMap::new() };
            p.mezcla[0].efectos.push(EfectoPuesto { controles, ..EfectoPuesto::nuevo(&k) });
            let x = a_audio(&p, &mut b, Modo::Patron(1));
            assert!(x.iter().flatten().all(|v| v.is_finite() && v.abs() <= 0.99), "{k}");
            let fin = x[x.len() - 480..].iter().flatten().fold(0f32, |m, v| m.max(v.abs()));
            assert!(fin < 0.5, "{k} sigue creciendo: {fin}");
        }
    }
}

#[test]
fn seno_preciso() {
    for i in -1000..1000 {
        let x = i as f32 / 337.0;
        assert!((receta::seno(x) - (x * std::f32::consts::TAU).sin()).abs() < 4e-4);
    }
}

#[test]
fn midi_ida_y_vuelta() {
    let mut p = Proyecto::demo();
    p.clips = vec![Clip::de_patron(2, 0, 0, 4 * COMPAS)];
    let m = midi::exportar(&p, 2);
    let mut q = Proyecto::default();
    let id = midi::importar(&mut q, &m).unwrap();
    assert!((q.bpm - 100.0).abs() < 0.01);
    let cuenta = |p: &Proyecto, id| {
        let mut v: Vec<(u32, u32, u8)> = p.patron(id).unwrap().notas.values().flatten().map(|n| (n.tick, n.dur, n.tono)).collect();
        v.sort();
        v
    };
    assert_eq!(cuenta(&p, 2), cuenta(&q, id));
    // Cortado en cualquier byte no truena.
    for i in 0..m.len() {
        let _ = midi::importar(&mut Proyecto::default(), &m[..i]);
    }
}

fn flp_de_prueba(version: &str, ancho: usize) -> Vec<u8> {
    let mut d: Vec<u8> = vec![];
    let dato = |d: &mut Vec<u8>, id: u8, x: &[u8]| {
        d.push(id);
        let mut n = x.len();
        while n > 127 {
            d.push(n as u8 & 127 | 128);
            n >>= 7;
        }
        d.push(n as u8);
        d.extend_from_slice(x);
    };
    let u = |s: &str| -> Vec<u8> { s.encode_utf16().chain([0]).flat_map(|c| c.to_le_bytes()).collect() };
    dato(&mut d, 199, format!("{version}\0").as_bytes());
    d.push(156);
    d.extend_from_slice(&128_500u32.to_le_bytes());
    for (i, nombre) in ["Kick", "808 Bass"].iter().enumerate() {
        d.push(64);
        d.extend_from_slice(&(i as u16).to_le_bytes());
        dato(&mut d, 201, &u(if i == 1 { "Sytrus" } else { "" }));
        dato(&mut d, 203, &u(nombre));
    }
    d.push(65);
    d.extend_from_slice(&1u16.to_le_bytes());
    dato(&mut d, 193, &u("Coro ñ"));
    let mut notas = vec![];
    for (pos, ch, dur, tono, vel) in [(0u32, 0u16, 0u32, 60u16, 128u8), (192, 1, 96, 36, 64)] {
        let mut n = [0u8; 24];
        n[0..4].copy_from_slice(&pos.to_le_bytes());
        n[6..8].copy_from_slice(&ch.to_le_bytes());
        n[8..12].copy_from_slice(&dur.to_le_bytes());
        n[12..14].copy_from_slice(&tono.to_le_bytes());
        n[21] = vel;
        notas.extend_from_slice(&n);
    }
    dato(&mut d, 224, &notas);
    let mut pl = vec![];
    for (item, pista) in [(20481u16, 497u16), (3, 499)] {
        let mut c = vec![0u8; ancho];
        c[0..4].copy_from_slice(&384u32.to_le_bytes());
        c[4..6].copy_from_slice(&20480u16.to_le_bytes());
        c[6..8].copy_from_slice(&item.to_le_bytes());
        c[8..12].copy_from_slice(&768u32.to_le_bytes());
        c[12..14].copy_from_slice(&pista.to_le_bytes());
        pl.extend_from_slice(&c);
    }
    dato(&mut d, 233, &pl);
    let mut b = b"FLhd\x06\0\0\0\0\0\x02\0".to_vec();
    b.extend_from_slice(&192u16.to_le_bytes());
    b.extend_from_slice(b"FLdt");
    b.extend_from_slice(&(d.len() as u32).to_le_bytes());
    b.extend_from_slice(&d);
    b
}

#[test]
fn abre_flp() {
    for (v, ancho) in [("20.8.3.2304", 32), ("21.2.3.4004", 60)] {
        let b = flp_de_prueba(v, ancho);
        let (p, avisos) = flp::leer(&b).unwrap();
        assert_eq!(p.bpm, 128.5);
        assert_eq!(
            p.canales.iter().map(|c| (c.nombre.as_str(), c.instrumento.as_str())).collect::<Vec<_>>(),
            [("Kick", "bombo"), ("808 Bass", "bajo")]
        );
        let pat = p.patron(1).unwrap();
        assert_eq!(pat.nombre, "Coro ñ");
        // 192 por negra → 96.
        assert_eq!(pat.notas[&2][0], Nota { tick: 96, dur: 48, tono: 36, fuerza: 0.5 });
        assert_eq!(pat.notas[&1][0].dur, PASO);
        assert_eq!(p.clips, [Clip::de_patron(1, 2, 192, 384)]);
        assert!(avisos.iter().any(|a| a.contains("Sytrus")) && avisos.iter().any(|a| a.contains("1 clips")));
        for i in 0..b.len() {
            let _ = flp::leer(&b[..i]);
        }
    }
}

#[test]
fn wav_ida_y_vuelta_y_muestra() {
    let ruta = std::env::temp_dir().join(format!("bdd-{}.wav", std::process::id()));
    let x: Vec<[f32; 2]> = (0..48000).map(|i| [(i as f32 * 440.0 / 48000.0 * 6.2832).sin() * 0.8, 0.25]).collect();
    wav::escribir(&ruta, 48000, &x).unwrap();
    let m = wav::leer(&ruta).unwrap();
    assert!(m.izq.len() == 48000 && m.izq.iter().zip(&x).all(|(a, b)| (a - b[0]).abs() < 1e-5) && (m.der[7] - 0.25).abs() < 1e-5);
    let mut p = una_nota("muestra", 72, 16);
    p.canales[0].muestra = Some(ruta.to_string_lossy().to_string());
    let y = a_audio(&p, &mut bib(), Modo::Patron(1));
    // Una octava arriba.
    let f = afinacion(&y[4000..16000], 48000.0, 700.0, 1100.0);
    assert!((f - 880.0).abs() < 2.0, "{f}");
    std::fs::remove_file(ruta).unwrap();
    for i in 0..60 {
        let _ = wav::de_bytes(&b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0\x01\0\x02\0\x44\xac\0\0\0\0\0\0\x04\0\x10\0data\x08\0\0\0abcdefgh"[..i.min(52)]);
    }
}

#[test]
fn el_proyecto_se_guarda_y_se_abre() {
    let p = Proyecto::demo();
    assert_eq!(Proyecto::de_json(&serde_json::to_string(&p).unwrap()).unwrap(), p);
    assert!(Proyecto::de_json("{}").is_err());
}

/// Un SoundFont mínimo: un preset con dos zonas de teclado.
fn sf2_de_prueba() -> Vec<u8> {
    fn trozo(id: &[u8; 4], d: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend_from_slice(&(d.len() as u32).to_le_bytes());
        v.extend_from_slice(d);
        if d.len() % 2 == 1 {
            v.push(0);
        }
        v
    }
    let lista = |tipo: &[u8; 4], t: &[Vec<u8>]| trozo(b"LIST", &[tipo.to_vec(), t.concat()].concat());
    let w = |v: &[u16]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    let nombre = |s: &str| {
        let mut v = s.as_bytes().to_vec();
        v.resize(20, 0);
        v
    };
    // 480 muestras: diez vueltas exactas de 1000 Hz a 48 kHz.
    let pcm: Vec<u8> = (0..480).flat_map(|i| (((i as f32 / 48.0 * std::f32::consts::TAU).sin() * 20000.0) as i16).to_le_bytes()).collect();
    let phdr = [nombre("Piano"), w(&[0, 0, 0]), vec![0; 12], nombre("EOP"), w(&[0, 0, 1]), vec![0; 12]].concat();
    let pbag = w(&[0, 0, 1, 0]);
    let pgen = w(&[41, 0, 0, 0]);
    let inst = [nombre("Uno"), w(&[0]), nombre("EOI"), w(&[2])].concat();
    let ibag = w(&[0, 0, 4, 0, 8, 0]);
    let r = |a: u16, b: u16| a | b << 8;
    let igen = w(&[43, r(0, 70), 54, 1, 17, (-500i16) as u16, 53, 0, 43, r(60, 127), 54, 1, 17, 500, 53, 0, 0, 0]);
    let cab = |n: &str, v: [u32; 5], raiz: u8| [nombre(n), v.iter().flat_map(|x| x.to_le_bytes()).collect(), vec![raiz, 0], w(&[0, 1])].concat();
    let shdr = [cab("onda", [0, 480, 48, 432, 48000], 60), cab("EOS", [0; 5], 0)].concat();
    let t = |id: &[u8; 4], d: &[u8]| trozo(id, d);
    let cuerpo = [
        b"sfbk".to_vec(),
        lista(b"INFO", &[t(b"ifil", &[2, 0, 1, 0])]),
        lista(b"sdta", &[t(b"smpl", &pcm)]),
        lista(
            b"pdta",
            &[
                t(b"phdr", &phdr),
                t(b"pbag", &pbag),
                t(b"pmod", &[0; 10]),
                t(b"pgen", &pgen),
                t(b"inst", &inst),
                t(b"ibag", &ibag),
                t(b"imod", &[0; 10]),
                t(b"igen", &igen),
                t(b"shdr", &shdr),
            ],
        ),
    ]
    .concat();
    trozo(b"RIFF", &cuerpo)
}

#[test]
fn toca_soundfont() {
    let b = sf2_de_prueba();
    let m = muestrario::sf2(&b, 0).unwrap();
    assert_eq!(m.zonas.len(), 2);
    assert_eq!((m.elegir(40, 1.0).1, m.elegir(65, 1.0).1), (1, 2));
    // Se comparte la grabación.
    assert_eq!(m.datos.len(), 480);
    let ruta = std::env::temp_dir().join(format!("bdd-{}.sf2", std::process::id()));
    std::fs::write(&ruta, &b).unwrap();
    assert_eq!(muestrario::presets(&ruta), ["Piano"]);
    let mut p = una_nota("muestra", 48, 16);
    p.canales[0].muestra = Some(ruta.to_string_lossy().to_string());
    let y = a_audio(&p, &mut bib(), Modo::Patron(1));
    // Una octava abajo de 1000 Hz, en bucle, y solo a la izquierda.
    let f = afinacion(&y[4000..24000], 48000.0, 350.0, 650.0);
    assert!((f - 500.0).abs() < 1.0, "{f}");
    assert!(y.iter().any(|s| s[0].abs() > 0.1) && y.iter().all(|s| s[1] == 0.0));
    std::fs::remove_file(ruta).unwrap();
    for i in 0..b.len() {
        let _ = muestrario::sf2(&b[..i], 0);
    }
}

#[test]
fn toca_sfz() {
    let dir = std::env::temp_dir().join(format!("bdd-sfz-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("son idos")).unwrap();
    let x: Vec<[f32; 2]> = (0..48000).map(|i| [(i as f32 * 440.0 / 48000.0 * std::f32::consts::TAU).sin() * 0.5; 2]).collect();
    wav::escribir(&dir.join("son idos/la 4.wav"), 48000, &x).unwrap();
    let t = "// piano\n<control> default_path=son idos/\n<group> hivel=60 <region> sample=la 4.wav lokey=c4 hikey=b4\npitch_keycenter=a4\n<group>\n<region> sample=la 4.wav lovel=61 key=69 volume=-6\n<region> sample=no.wav key=20\n";
    std::fs::write(dir.join("p.sfz"), t).unwrap();
    let m = muestrario::abrir(&dir.join("p.sfz"), 0).unwrap();
    assert_eq!(m.zonas.len(), 4);
    assert_eq!((m.elegir(60, 0.3).1, m.elegir(60, 1.0).1, m.elegir(69, 1.0).1, m.elegir(72, 0.3).1), (2, 0, 2, 0));
    assert_eq!((m.zonas[0].raiz, m.zonas[0].tonos, m.zonas[2].raiz), (69.0, (60, 71), 69.0));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn los_clips_de_audio_suenan_en_su_lugar() {
    let ruta = std::env::temp_dir().join(format!("bdd-clip-{}.wav", std::process::id()));
    // Un escalón: 0.25 el primer segundo y 0.5 el segundo, a 44.1 kHz (se oye igual a 48).
    let x: Vec<[f32; 2]> = (0..88200).map(|i| if i < 44100 { [0.25, -0.25] } else { [0.5, -0.5] }).collect();
    wav::escribir(&ruta, 44100, &x).unwrap();
    let mut p = Proyecto::default();
    p.mezcla.iter_mut().for_each(|m| m.volumen = 1.0);
    let audio = ClipAudio { ruta: ruta.to_string_lossy().into(), nombre: "x".into(), inicio: 0.5, volumen: 1.0, mezcla: 3, ..Default::default() };
    // 120 bpm: un compás son 2 s. Empieza en 2 s y dura 1 s.
    p.clips.push(Clip { audio: Some(audio), ..Clip::de_patron(0, 2, COMPAS, COMPAS / 2) });
    let y = a_audio(&p, &mut bib(), Modo::Cancion);
    let en = |s: f32| y[(s * 48000.0) as usize];
    assert!(en(1.9)[0].abs() < 1e-6);
    assert!((en(2.25)[0] - 0.25).abs() < 1e-3 && (en(2.25)[1] + 0.25).abs() < 1e-3, "{:?}", en(2.25));
    // Saltó medio segundo: el escalón llega a los 2.5 s.
    assert!((en(2.75)[0] - 0.5).abs() < 1e-3);
    assert!(y.get(48000 * 3 + 500).is_none_or(|s| s[0].abs() < 1e-6));
    p.pistas[2].mudo = true;
    assert!(a_audio(&p, &mut bib(), Modo::Cancion).iter().all(|s| s[0] == 0.0));
    std::fs::remove_file(ruta).unwrap();
}

#[test]
fn el_metronomo_marca_cada_negra() {
    let mut b = bib();
    let (mut m, mando) = Motor::nuevo(Escena::nueva(&Proyecto::default(), &mut b, Modo::Patron(1)));
    mando.mandar(Orden::Metronomo(true));
    mando.mandar(Orden::Tocar(true));
    let mut v = vec![[0.0; 2]; 96000];
    v.chunks_mut(256).for_each(|c| m.tocar(c));
    for i in 0..4 {
        let pico = |a: usize| v[a..a + 400].iter().fold(0f32, |m, s| m.max(s[0].abs()));
        assert!(pico(i * 24000) > 0.1 && pico(i * 24000 + 12000) < 1e-3);
    }
}

#[test]
fn abre_todos_los_formatos() {
    use beethoven_daw_demo::decodificar as d;
    let Some(f) = d::ffmpeg() else { return eprintln!("sin ffmpeg: no se prueban los formatos") };
    let dir = std::env::temp_dir().join(format!("bdd-fmt-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let x: Vec<[f32; 2]> = (0..96000).map(|i| [(i as f32 * 440.0 / 48000.0 * std::f32::consts::TAU).sin() * 0.5; 2]).collect();
    let base = dir.join("base.wav");
    wav::escribir(&base, 48000, &x).unwrap();
    for (n, extra) in [
        ("a.mp3", "libmp3lame"),
        ("a.ogg", "vorbis"),
        ("b.ogg", "libopus"),
        ("a.opus", "libopus"),
        ("a.flac", "flac"),
        ("a.m4a", "aac"),
        ("a.aiff", "pcm_s16be"),
        ("a.caf", "pcm_s16le"),
        ("b.wav", "adpcm_ms"),
    ] {
        let r = dir.join(n);
        let o = std::process::Command::new(&f)
            .args(["-v", "error", "-y", "-i"])
            .arg(&base)
            .args(["-codec:a", extra, "-strict", "-2", "-ac", "2"])
            .arg(&r)
            .output()
            .unwrap();
        assert!(o.status.success(), "{n}: {}", String::from_utf8_lossy(&o.stderr));
        let w = d::leer(&r).unwrap_or_else(|e| panic!("{n}: {e}"));
        let seg = w.izq.len() as f32 / w.sr;
        assert!((seg - 2.0).abs() < 0.12, "{n}: {seg} s");
        let y: Vec<[f32; 2]> = w.izq.iter().map(|v| [*v; 2]).collect();
        let h = afinacion(&y[(w.sr * 0.5) as usize..(w.sr * 0.8) as usize], w.sr, 350.0, 550.0);
        assert!((h - 440.0).abs() < 2.0, "{n}: {h} Hz");
    }
    for n in ["s.mp3", "s.flac", "s.ogg", "s.m4a"] {
        let r = dir.join(n);
        d::escribir(&r, 48000, &x).unwrap_or_else(|e| panic!("{n}: {e}"));
        assert!((d::leer(&r).unwrap().izq.len() as f32 / 48000.0 - 2.0).abs() < 0.12);
    }
    std::fs::write(dir.join("roto.mp3"), [7u8; 5000]).unwrap();
    assert!(d::leer(&dir.join("roto.mp3")).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

/// Lo que hace un efecto con un tono de esa frecuencia y nivel: el pico ya asentado.
fn por_efecto(clave: &str, hz: f32, nivel: f32, controles: &[(&str, f32)]) -> f32 {
    let ruta = std::env::temp_dir().join(format!("bdd-fx-{}-{clave}-{hz}-{nivel}.wav", std::process::id()));
    let x: Vec<[f32; 2]> = (0..48000).map(|i| [(i as f32 * hz / 48000.0 * std::f32::consts::TAU).sin() * nivel; 2]).collect();
    wav::escribir(&ruta, 48000, &x).unwrap();
    let mut p = Proyecto::default();
    p.mezcla.iter_mut().for_each(|m| m.volumen = 1.0);
    let controles = controles.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    p.mezcla[1].efectos.push(EfectoPuesto { controles, ..EfectoPuesto::nuevo(clave) });
    let audio = ClipAudio { ruta: ruta.to_string_lossy().into(), nombre: "x".into(), inicio: 0.0, volumen: 1.0, mezcla: 1, ..Default::default() };
    p.clips.push(Clip { audio: Some(audio), ..Clip::de_patron(0, 0, 0, COMPAS / 2) });
    let y = a_audio(&p, &mut bib(), Modo::Cancion);
    std::fs::remove_file(ruta).unwrap();
    y[30000..45000].iter().fold(0f32, |m, s| m.max(s[0].abs()))
}

#[test]
fn el_ecualizador_y_la_dinamica_miden_lo_que_dicen() {
    let db = |a: f32, b: f32| 20.0 * (a / b).log10();
    // Ecualizador: +6 dB donde se pide, nada lejos, y los estantes de cada lado.
    let eq = |hz, c: &[(&str, f32)]| db(por_efecto("ecualizador", hz, 0.2, c), 0.2);
    assert!(eq(1000.0, &[]).abs() < 0.05);
    let sube = [("h4", 1000.0), ("g4", 6.0), ("q4", 2.0)];
    assert!((eq(1000.0, &sube) - 6.0).abs() < 0.1 && eq(100.0, &sube).abs() < 0.2);
    assert!((eq(40.0, &[("h1", 200.0), ("g1", -9.0)]) + 9.0).abs() < 0.5);
    assert!((eq(15000.0, &[("h6", 5000.0), ("g6", 8.0)]) - 8.0).abs() < 0.6);
    // La curva que se dibuja es la que suena.
    let k = receta::banda(receta::Banda::Campana, 1000.0, 6.0, 2.0, 48000.0);
    assert!((receta::respuesta(&k, 1300.0, 48000.0) - eq(1300.0, &sube)).abs() < 0.1);
    // Compresor 4:1 desde -20 dB: un tono de -8 dB queda en -17.
    let c = [("umbral", -20.0), ("razon", 4.0), ("ataque", 0.0005), ("soltar", 1.0)];
    let x = 10f32.powf(-8.0 / 20.0);
    let y = 20.0 * por_efecto("compresor", 1000.0, x, &c).log10();
    assert!((y + 17.0).abs() < 1.0, "{y}");
    assert!(db(por_efecto("compresor", 1000.0, 0.03, &c), 0.03).abs() < 0.1);
    // Limitador: nunca pasa del techo.
    let y = por_efecto("limitador", 300.0, 0.5, &[("entra", 18.0), ("techo", -6.0)]);
    assert!((20.0 * y.log10() + 6.0).abs() < 0.4, "{y}");
    // Compuerta: cierra lo débil y deja lo fuerte.
    assert!(por_efecto("compuerta", 500.0, 0.0005, &[("umbral", -40.0)]) < 1e-5);
    assert!(db(por_efecto("compuerta", 500.0, 0.3, &[("umbral", -40.0)]), 0.3).abs() < 0.2);
}

#[test]
fn los_envios_y_el_solo_de_la_mezcla() {
    let mut b = bib();
    let mut p = una_nota("caja", 60, 1);
    p.canales[0].pista = 2;
    let seco = a_audio(&p, &mut b, Modo::Patron(1));
    p.mezcla[2].envios[0] = 0.8;
    let moja = a_audio(&p, &mut b, Modo::Patron(1));
    let en = |v: &[[f32; 2]], a: usize| v[a..a + 4000].iter().fold(0f32, |m, s| m.max(s[0].abs()));
    // Medio segundo después sigue la reverberación del envío.
    assert!(moja.len() > seco.len() && en(&moja, 24000) > 20.0 * en(&seco, 24000).max(1e-5));
    p.mezcla[3].solo = true;
    assert!(a_audio(&p, &mut b, Modo::Patron(1)).iter().all(|s| s[0].abs() < 1e-3));
    p.mezcla[2].solo = true;
    assert!(a_audio(&p, &mut b, Modo::Patron(1)).iter().any(|s| s[0].abs() > 0.05));
}

#[test]
fn exporta_pistas_separadas() {
    let dir = std::env::temp_dir().join(format!("bdd-pistas-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = Proyecto::demo();
    let n = beethoven_daw_demo::ui::pistas(&p, &mut bib(), Modo::Cancion, &dir).unwrap();
    assert_eq!((n, std::fs::read_dir(&dir).unwrap().count()), (5, 5));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn el_clip_de_audio_y_la_pista_se_ajustan() {
    let ruta = std::env::temp_dir().join(format!("bdd-aj-{}.wav", std::process::id()));
    let x: Vec<[f32; 2]> = (0..96000).map(|i| [(i as f32 * 440.0 / 48000.0 * std::f32::consts::TAU).sin() * 0.25; 2]).collect();
    wav::escribir(&ruta, 48000, &x).unwrap();
    let mut p = Proyecto::default();
    p.mezcla.iter_mut().for_each(|m| m.volumen = 1.0);
    let i = p.pista_para("voz");
    assert_eq!((i, p.mezcla[i].nombre.as_str()), (1, "voz"));
    let audio = ClipAudio { ruta: ruta.to_string_lossy().into(), mezcla: i, velocidad: 2.0, pan: -1.0, entra: 0.4, ..Default::default() };
    p.clips.push(Clip { audio: Some(audio), ..Clip::de_patron(0, 0, 0, COMPAS / 2) });
    let y = a_audio(&p, &mut bib(), Modo::Cancion);
    // Doble velocidad: una octava arriba; solo a la izquierda; a media entrada suena a la mitad.
    let f = afinacion(&y[24000..40000], 48000.0, 700.0, 1100.0);
    assert!((f - 880.0).abs() < 1.5 && y.iter().all(|s| s[1].abs() < 1e-6), "{f}");
    let pico = |a: usize, b: usize| y[a..b].iter().fold(0f32, |m, s| m.max(s[0].abs()));
    assert!((pico(9500, 9700) - 0.125).abs() < 0.01 && (pico(30000, 31000) - 0.25).abs() < 0.01);
    // El ecualizador de la pista: -12 dB en los medios bajan un tono de 1200 Hz (aquí 880 Hz, casi todo).
    p.mezcla[i].eq = [0.0, -12.0, 0.0];
    let z = a_audio(&p, &mut bib(), Modo::Cancion);
    let q = z[30000..31000].iter().fold(0f32, |m, s| m.max(s[0].abs()));
    let k = receta::banda(receta::Banda::Campana, 1200.0, -12.0, 0.6, 48000.0);
    assert!((20.0 * (q / 0.25).log10() - receta::respuesta(&k, 880.0, 48000.0)).abs() < 0.2, "{q}");
    // Con el efecto a cero se oye el sonido limpio.
    p.mezcla[i].eq = [0.0; 3];
    p.mezcla[i].efectos.push(EfectoPuesto { mezcla: 0.0, ..EfectoPuesto::nuevo("trituradora") });
    let w = a_audio(&p, &mut bib(), Modo::Cancion);
    assert!(w.iter().zip(&y).all(|(a, b)| (a[0] - b[0]).abs() < 1e-5));
    // Otra pista libre para el siguiente.
    assert_eq!(p.pista_para("coro"), 2);
    std::fs::remove_file(ruta).unwrap();
}

#[test]
fn cortar_un_clip_no_cambia_lo_que_suena() {
    let mut b = bib();
    let mut p = Proyecto::demo();
    p.swing = 0.2;
    let entero = a_audio(&p, &mut b, Modo::Cancion);
    // Cada clip partido en un compás entero, donde ninguna nota queda a medias.
    let mut nuevos = vec![];
    for c in &mut p.clips {
        if c.dur > COMPAS {
            nuevos.extend(c.cortar(c.tick + COMPAS, 100.0));
        }
    }
    assert!(nuevos.len() >= 3 && nuevos.iter().all(|c| c.desfase == COMPAS));
    p.clips.extend(nuevos);
    let partido = a_audio(&p, &mut b, Modo::Cancion);
    // El azar de las notas cambia de orden; lo demás es igual: mismas notas en los mismos lugares.
    let cuenta = |p: &Proyecto| {
        let mut v: Vec<(u32, u32, u8, u32)> = p
            .clips
            .iter()
            .flat_map(|c| p.patron(c.patron).unwrap().notas_en(c.desfase, c.dur).map(|(k, t, n, d)| (c.tick + t, k, n.tono, d)))
            .collect();
        v.sort();
        v
    };
    assert_eq!(cuenta(&p), cuenta(&Proyecto::demo()));
    assert!((partido.len() as i64 - entero.len() as i64).abs() < 48000);
    // Un audio: el segundo pedazo sigue donde quedó el primero, también a otra velocidad.
    let mut c = Clip {
        audio: Some(ClipAudio { velocidad: 2.0, inicio: 1.0, entra: 0.5, sale: 0.5, ..Default::default() }),
        ..Clip::de_patron(0, 0, 0, COMPAS * 2)
    };
    let d = c.cortar(COMPAS, 120.0).unwrap();
    let (x, y) = (c.audio.unwrap(), d.audio.unwrap());
    assert_eq!((c.dur, d.tick, d.dur), (COMPAS, COMPAS, COMPAS));
    assert!((y.inicio - 5.0).abs() < 1e-4 && x.sale == 0.0 && y.entra == 0.0 && y.sale == 0.5);
    assert!(Clip::de_patron(1, 0, 0, 96).cortar(96, 120.0).is_none());
}

#[test]
fn oye_con_numeros() {
    use beethoven_daw_demo::analizar as a;
    let sr = 48000.0;
    // Un tono con dos huecos.
    let x: Vec<f32> = (0..240000)
        .map(|i| {
            let s = i as f32 / sr;
            if (1.0..1.6).contains(&s) || (3.0..4.0).contains(&s) { 0.0 } else { (s * 440.0 * std::f32::consts::TAU).sin() * 0.5 }
        })
        .collect();
    let v = a::silencios(&x, sr, -45.0, 0.3);
    assert_eq!(v.len(), 2);
    assert!((v[0].0 - 1.0).abs() < 0.02 && (v[0].1 - 1.6).abs() < 0.02 && (v[1].1 - 4.0).abs() < 0.02, "{v:?}");
    assert!((a::db(a::pico(&x)) + 6.02).abs() < 0.1);
    // El tempo de la batería de la demo, a varias velocidades; y la tonalidad de su armonía (la menor o su relativa, do).
    for bpm in [84.0, 100.0, 128.0, 140.0] {
        let mut p = Proyecto::demo();
        (p.bpm, p.swing) = (bpm, 0.0);
        p.clips = (0..10).map(|i| Clip::de_patron(1, 0, i * COMPAS, COMPAS)).collect();
        let y: Vec<f32> = a_audio(&p, &mut bib(), Modo::Cancion).iter().map(|s| s[0]).collect();
        let t = a::tempo(&y, sr).unwrap();
        // Con platillos grabados a semicorcheas, a 84 se oye igual el doble: se acepta, como en la medición
        // de tempo de siempre («accuracy 2»). Lo que no se acepta es un tempo que no sea de la canción.
        assert!([1.0, 2.0, 0.5].iter().any(|k| (t.0 - bpm * k).abs() < 0.6 * k), "{bpm}: {t:?}");
    }
    let mut p = Proyecto::demo();
    p.clips.retain(|c| c.patron == 2);
    let y: Vec<f32> = a_audio(&p, &mut bib(), Modo::Cancion).iter().map(|s| s[0]).collect();
    let k = a::tonalidad(&y, sr).unwrap();
    assert!([(9, false), (0, true)].contains(&k), "{} {}", a::NOTAS[k.0], k.1);
    // Un grave pesa en los graves; un agudo, en los agudos.
    let tono = |hz: f32| -> Vec<f32> { (0..48000).map(|i| (i as f32 / sr * hz * std::f32::consts::TAU).sin() * 0.3).collect() };
    let (g, m, b) = (a::bandas(&tono(60.0), sr), a::bandas(&tono(900.0), sr), a::bandas(&tono(9000.0), sr));
    assert!(g[0] > g[1] + 4.0 && m[1] > m[0] + 6.0 && m[1] > m[2] + 6.0 && b[2] > b[1] + 3.0, "{g:?} {m:?} {b:?}");
    assert!(a::tempo(&[0.0; 480000], sr).is_none() && a::tonalidad(&[0.0; 48000], sr).is_none());
}
