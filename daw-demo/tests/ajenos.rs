//! Como lo haría alguien de fuera: escribe su plugin, lo empaca, otro lo suelta en el programa y funciona. Nada de esto
//! conoce los plugins de fábrica.
use beethoven_daw_demo::biblioteca::Biblioteca;
use beethoven_daw_demo::instalar::{Cambio, escribir_zip, leer_zip};
use beethoven_daw_demo::interfaz::{Momento, Pieza, cargar_con};
use beethoven_daw_demo::modelo::*;
use beethoven_daw_demo::motor::{Modo, PRIMER_PAD, a_audio};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const SR: f32 = 48000.0;

fn lugar(n: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("bdaw-ajenos-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn bib(d: &Path) -> Biblioteca {
    Biblioteca::nueva(SR, Some(d.join("Lua")))
}

fn seno(hz: f32, seg: f32) -> Vec<[f32; 2]> {
    (0..(SR * seg) as usize).map(|i| [(std::f32::consts::TAU * hz * i as f32 / SR).sin() * 0.5; 2]).collect()
}

fn cuanto(x: &[[f32; 2]], hz: f32) -> f32 {
    let (a, b) = x.iter().enumerate().fold((0.0f64, 0.0f64), |(a, b), (i, v)| {
        let w = std::f64::consts::TAU * hz as f64 * i as f64 / SR as f64;
        (a + v[0] as f64 * w.cos(), b + v[0] as f64 * w.sin())
    });
    (a.hypot(b) * 2.0 / x.len() as f64) as f32
}

fn rms(x: &[[f32; 2]]) -> f32 {
    (x.iter().map(|v| v[0] * v[0]).sum::<f32>() / x.len() as f32).sqrt()
}

/// Un zip como los que arma el Finder: comprimido, dentro de una carpeta y con su basura.
fn zip_comprimido(v: &[(&str, &[u8])]) -> Vec<u8> {
    let (mut b, mut indice) = (vec![], vec![]);
    for (n, x) in v {
        let z = miniz_oxide::deflate::compress_to_vec(x, 6);
        let lugar = b.len() as u32;
        let mut cab = vec![];
        [20u16, 0, 8, 0, 0].iter().for_each(|w| cab.extend(w.to_le_bytes()));
        [0u32, z.len() as u32, x.len() as u32].iter().for_each(|w| cab.extend(w.to_le_bytes()));
        cab.extend((n.len() as u16).to_le_bytes());
        cab.extend([0, 0]);
        b.extend(b"PK\x03\x04");
        b.extend(&cab);
        b.extend(n.as_bytes());
        b.extend(&z);
        indice.extend(b"PK\x01\x02\x14\x00");
        indice.extend(&cab);
        indice.extend([0u8; 10]);
        indice.extend(lugar.to_le_bytes());
        indice.extend(n.as_bytes());
    }
    let (lugar, n) = (b.len() as u32, v.len() as u16);
    b.extend(&indice);
    b.extend(b"PK\x05\x06\0\0\0\0");
    [n, n].iter().for_each(|w| b.extend(w.to_le_bytes()));
    [indice.len() as u32, lugar].iter().for_each(|w| b.extend(w.to_le_bytes()));
    b.extend([0, 0]);
    b
}

const MITAD: &str = r##"local u = incluir("util")
return { nombre = { es = "Mitad", en = "Half" }, tipo = "efecto", autor = "Alguien", version = "1.0", api = 1,
  controles = { { "k", { es = "Cuánto", en = "Amount" }, 0, 1, 0.5 } },
  sonido = function(e, c) return u.por(e.izq, c.k), u.por(e.der, c.k) end,
  interfaz = { dibujar = function(d, c, m) d.acero(0, 0, 420, 260) d.perilla(100, 100, 30, "k") d.grafica(150, 40, 200, 90, "niveles") end } }"##;
const UTIL: &str = "return { por = function(x, k) return x * k end }";

/// Un proyecto con un tono que pasa por ese efecto.
fn con_efecto(d: &Path, clave: &str) -> Proyecto {
    let r = d.join("tono.wav");
    beethoven_daw_demo::wav::escribir(&r, 48000, &seno(440.0, 1.0)).unwrap();
    let mut p = Proyecto { bpm: 120.0, ..Default::default() };
    p.mezcla.iter_mut().for_each(|m| {
        m.efectos.clear();
        m.volumen = 1.0;
    });
    let audio = ClipAudio { ruta: r.to_string_lossy().into(), nombre: "x".into(), mezcla: 1, ..Default::default() };
    p.clips.push(Clip { patron: 0, pista: 0, tick: 0, dur: p.pulsos(1.0), desfase: 0, audio: Some(audio) });
    p.mezcla[1].efectos.push(EfectoPuesto::nuevo(clave));
    p
}

#[test]
fn de_un_zip_a_sonar_y_de_vuelta_a_un_zip() {
    let d = lugar("zip");
    let zip = d.join("mitad-1.0.zip");
    let v: [(&str, &[u8]); 4] = [
        ("mitad/mitad.lua", MITAD.as_bytes()),
        ("mitad/util.lua", UTIL.as_bytes()),
        ("__MACOSX/mitad/._mitad.lua", b"x"),
        ("mitad/LEEME.txt", b"hola"),
    ];
    std::fs::write(&zip, zip_comprimido(&v)).unwrap();
    let mut b = bib(&d);
    let (clave, inf) = b.instalar(&zip).unwrap();
    assert_eq!(clave, "mitad");
    assert!(inf.avisos.is_empty() && inf.cpu < 20.0, "{inf:?}");
    let g = b.ver("mitad").unwrap();
    assert!(g.propio && g.ficha.autor == "Alguien" && g.ficha.version == "1.0");
    // Suena: la mitad de fuerte.
    let p = con_efecto(&d, "mitad");
    let seco = con_efecto(&d, "nada");
    let (y, x) = (a_audio(&p, &mut b, Modo::Cancion), a_audio(&seco, &mut b, Modo::Cancion));
    assert!((rms(&y[8000..40000]) / rms(&x[8000..40000]) - 0.5).abs() < 0.02);
    // Su ventana trae sus piezas.
    let g = b.ver("mitad").unwrap();
    let mut i = cargar_con(&g.fuente, g.archivos.clone()).unwrap().unwrap();
    let piezas = i.cuadro(&[("k".to_string(), 0.5)].into(), &Momento::default());
    assert!(piezas.iter().any(|z| matches!(z, Pieza::Grafica { clase, .. } if clase == "niveles")), "{:?}", i.error);
    // Se empaca y otra computadora lo instala igual.
    let otro = lugar("zip2");
    let paquete = otro.join("cualquier nombre.bplugin");
    std::fs::write(&paquete, b.empacar("mitad").unwrap()).unwrap();
    let mut b2 = bib(&otro);
    assert_eq!(b2.instalar(&paquete).unwrap().0, "mitad");
    let y2 = a_audio(&p, &mut b2, Modo::Cancion);
    assert!((rms(&y2[8000..40000]) - rms(&y[8000..40000])).abs() < 1e-4);
    // Y al abrir de nuevo el programa, sigue ahí.
    assert!(bib(&otro).ver("mitad").is_some());
}

#[test]
fn una_carpeta_o_un_guion_suelto_y_su_proyecto_guardado() {
    let d = lugar("carpeta");
    let c = d.join("Mi Plugin");
    std::fs::create_dir_all(&c).unwrap();
    std::fs::write(c.join("plugin.lua"), MITAD).unwrap();
    std::fs::write(c.join("util.lua"), UTIL).unwrap();
    let mut b = bib(&d);
    let (clave, _) = b.instalar(&c).unwrap();
    assert_eq!(clave, "plugin");
    // Un instrumento en un solo archivo.
    let l = d.join("tono puro.lua");
    std::fs::write(&l, "return { nombre = 'Tono puro', sonido = function(n, c) return seno(n.hz) * adsr(0.01, 0.1, 1, 0.1) * 0.5 end }").unwrap();
    let (clave, _) = b.instalar(&l).unwrap();
    let mut p = Proyecto { bpm: 120.0, ..Default::default() };
    let id = p.agregar_canal(&clave, "x");
    p.patrones[0].notas.insert(id, vec![Nota { tick: 0, dur: PPQ * 2, tono: 69, fuerza: 1.0 }]);
    p.mezcla.iter_mut().for_each(|m| m.efectos.clear());
    let p = Proyecto::de_json(&serde_json::to_string(&p).unwrap()).unwrap();
    let y = a_audio(&p, &mut bib(&d), Modo::Patron(p.patrones[0].id));
    assert!(cuanto(&y[4000..40000], 440.0) > 0.1);
}

#[test]
fn los_cambios_en_el_disco_entran_solos() {
    let d = lugar("vivo");
    let mut b = bib(&d);
    let c = d.join("Lua").join("vivo");
    std::fs::create_dir_all(&c).unwrap();
    let guion = |k: &str| format!("return {{ nombre = 'Vivo', tipo = 'efecto', sonido = function(e, c) return e.izq * {k}, e.der * {k} end }}");
    let poner = |s: &str| std::fs::write(c.join("vivo.lua"), s).unwrap();
    poner(&guion("0.25"));
    assert!(matches!(&b.vigilar()[..], [Cambio::Nuevo(k)] if k == "vivo"));
    assert!(b.vigilar().is_empty(), "sin cambios no hace nada");
    let p = con_efecto(&d, "vivo");
    let suena = |b: &mut Biblioteca| rms(&a_audio(&p, b, Modo::Cancion)[8000..40000]);
    let antes = suena(&mut b);
    poner(&guion("0.125 * 0.5"));
    assert!(matches!(&b.vigilar()[..], [Cambio::Mejor(_)]));
    assert!((suena(&mut b) / antes - 0.25).abs() < 0.02);
    // Mal escrito: avisa y sigue la versión buena.
    poner("return { nombre = 'Vivo', tipo = 'efecto', sonido = function(e, c) return e.izq * * 2 end }");
    assert!(matches!(&b.vigilar()[..], [Cambio::Mal(..)]));
    assert!((suena(&mut b) / antes - 0.25).abs() < 0.02);
    // Se borra y se va; quitarlo desde el programa lo aparta, no lo destruye.
    std::fs::remove_dir_all(&c).unwrap();
    assert!(matches!(&b.vigilar()[..], [Cambio::Fuera(_)]) && b.ver("vivo").is_none());
    std::fs::create_dir_all(&c).unwrap();
    poner(&guion("1"));
    b.vigilar();
    b.quitar("vivo").unwrap();
    assert!(b.ver("vivo").is_none() && std::fs::read_dir(d.join("Lua/Quitados")).unwrap().count() == 1 && b.vigilar().is_empty());
    assert!(bib(&d).ver("vivo").is_none());
}

#[test]
fn no_pisa_a_los_de_fabrica_y_se_actualiza_a_si_mismo() {
    let d = lugar("choque");
    let mut b = bib(&d);
    let l = d.join("eco.lua");
    let guion = |v: &str| {
        format!("return {{ nombre = 'Mi eco', autor = 'Yo', version = '{v}', tipo = 'efecto', sonido = function(e, c) return e.izq, e.der end }}")
    };
    std::fs::write(&l, guion("1")).unwrap();
    let (clave, _) = b.instalar(&l).unwrap();
    assert!(clave != "eco" && !b.ver("eco").unwrap().propio);
    // Uno con su propia clave libre: la versión dos toma su lugar.
    let l = d.join("rebote.lua");
    for v in ["1", "2"] {
        std::fs::write(&l, guion(v)).unwrap();
        assert_eq!(b.instalar(&l).unwrap().0, "rebote");
    }
    assert_eq!(b.ver("rebote").unwrap().ficha.version, "2");
    // Otro distinto con el mismo archivo no lo borra.
    std::fs::write(&l, "return { nombre = 'Otra cosa', autor = 'Ajena', tipo = 'efecto', sonido = function(e, c) return e.izq, e.der end }").unwrap();
    assert_eq!(b.instalar(&l).unwrap().0, "rebote_2");
    assert_eq!(b.ver("rebote").unwrap().ficha.version, "2");
}

#[test]
fn trae_sus_propios_audios_y_su_lugar() {
    let d = lugar("audios");
    let c = d.join("campanas");
    std::fs::create_dir_all(c.join("sonidos")).unwrap();
    beethoven_daw_demo::wav::escribir(&c.join("sonidos/grave.wav"), 48000, &seno(300.0, 1.0)).unwrap();
    beethoven_daw_demo::wav::escribir(&c.join("sonidos/agudo.wav"), 48000, &seno(1000.0, 1.0)).unwrap();
    let guion = r#"return { nombre = "Campanas",
      muestras = { { archivo = "sonidos/grave.wav", nota = 48, hasta = 59 }, { archivo = "sonidos/agudo.wav", nota = 72, desde = 60 } },
      sonido = function(n, c) return muestra(n.hz / 261.6256, "izq") * adsr(0.001, 0.1, 1, 0.05), muestra(n.hz / 261.6256, "der") * adsr(0.001, 0.1, 1, 0.05) end }"#;
    std::fs::write(c.join("campanas.lua"), guion).unwrap();
    let mut b = bib(&d);
    let (clave, _) = b.instalar(&c).unwrap();
    let mut p = Proyecto { bpm: 120.0, ..Default::default() };
    let id = p.agregar_canal(&clave, "x");
    let nota = |t: u32, tono: u8| Nota { tick: t, dur: PPQ, tono, fuerza: 1.0 };
    p.patrones[0].notas.insert(id, vec![nota(0, 48), nota(PPQ * 2, 84)]);
    p.mezcla.iter_mut().for_each(|m| m.efectos.clear());
    let y = a_audio(&p, &mut b, Modo::Patron(p.patrones[0].id));
    assert!(cuanto(&y[2000..20000], 300.0) > 0.1, "la nota 48 suena tal cual");
    assert!(cuanto(&y[50000..66000], 2000.0) > 0.1, "una octava arriba de su nota");
    // Le falta un audio: no entra.
    let mut roto = guion.replace("grave.wav", "noesta.wav");
    std::fs::write(c.join("campanas.lua"), &roto).unwrap();
    assert!(b.instalar(&c).unwrap_err().contains("noesta.wav"));
    // Una convolución con la huella que trae en su carpeta.
    roto = r#"return { nombre = "Mi lugar", tipo = "efecto", huella = "sonidos/agudo.wav",
      sonido = function(e, c) return convolucion(e.izq, "izq"), convolucion(e.der, "der") end }"#
        .into();
    std::fs::write(c.join("campanas.lua"), &roto).unwrap();
    let (clave, _) = b.instalar(&c).unwrap();
    let m = b.huella(&EfectoPuesto::nuevo(&clave)).unwrap();
    assert!((m.impulso.as_ref().unwrap().segundos - 1.0).abs() < 0.01);
}

#[test]
fn lo_que_hacen_los_de_fabrica_tambien_lo_puede_uno_ajeno() {
    let d = lugar("iguales");
    let mut b = bib(&d);
    // Los de fábrica más completos, con otro nombre, instalados como ajenos: deben sonar igual.
    for k in ["pads", "cortador", "ecualizador", "convolucion", "vocoder", "ctllfo", "compresor", "deseseo", "pedalera"] {
        let g = b.ver(k).unwrap();
        let fuente = g.fuente.clone();
        let l = d.join(format!("ajeno {k}.lua"));
        std::fs::write(&l, &fuente).unwrap();
        let (clave, _) = b.instalar(&l).unwrap_or_else(|e| panic!("{k}: {e}"));
        assert!(clave.starts_with("ajeno_"), "{clave}");
        let g = b.ver(&clave).unwrap();
        assert_eq!(g.ficha.muestras, b.ver(k).unwrap().ficha.muestras);
        let mut i = cargar_con(&g.fuente, g.archivos.clone()).unwrap().unwrap();
        let v = g.ficha.controles.iter().map(|c| (c.clave.clone(), c.valor)).collect();
        let m = Momento { nombres: g.ficha.controles.iter().map(|c| (c.clave.clone(), c.nombre.es.clone())).collect(), ..Default::default() };
        assert!(!i.cuadro(&v, &m).is_empty() && i.error.is_none(), "{k}: {:?}", i.error);
        if g.ficha.tipo == beethoven_daw_demo::lua::Tipo::Efecto {
            let (a, z) = (con_efecto(&d, k), con_efecto(&d, &clave));
            let (a, z) = (a_audio(&a, &mut b, Modo::Cancion), a_audio(&z, &mut b, Modo::Cancion));
            assert!((rms(&a[8000..40000]) - rms(&z[8000..40000])).abs() < 1e-3, "{k}");
        }
    }
    // La batería ajena toca sus pads en sus teclas.
    let r = d.join("golpe.wav");
    beethoven_daw_demo::wav::escribir(&r, 48000, &seno(700.0, 0.5)).unwrap();
    let mut p = Proyecto { bpm: 120.0, ..Default::default() };
    let id = p.agregar_canal("ajeno_pads", "x");
    p.canales[0].pads = vec![String::new(), r.to_string_lossy().into()];
    p.patrones[0].notas.insert(id, vec![Nota { tick: 0, dur: PASO, tono: PRIMER_PAD + 1, fuerza: 1.0 }]);
    p.mezcla.iter_mut().for_each(|m| m.efectos.clear());
    let y = a_audio(&p, &mut b, Modo::Patron(p.patrones[0].id));
    assert!(cuanto(&y[1000..20000], 700.0) > 0.1);
}

#[test]
fn lo_malo_no_entra() {
    let d = lugar("malos");
    let mut b = bib(&d);
    let efecto = |cuerpo: &str| format!("return {{ nombre = 'Malo', tipo = 'efecto', sonido = function(e, c) {cuerpo} end }}");
    let mut prueba = |n: &str, contenido: &str, espera: &str| {
        let l = d.join(format!("{n}.lua"));
        std::fs::write(&l, contenido).unwrap();
        let e = b.instalar(&l).expect_err(n);
        assert!(e.contains(espera), "{n}: {e}");
        assert!(b.ver(n).is_none());
    };
    prueba("sintaxis", "return { nombre = ", "");
    prueba("ciclo", "while true do end", "tardó demasiado");
    prueba("fuerte", &efecto("return e.izq * 1e9, e.der * 1e9"), "demasiado fuerte");
    prueba("archivos", &efecto("return io.open('/etc/passwd')"), "");
    prueba("sistema", &efecto("return os.execute('ls')"), "");
    prueba("cargar", &efecto("return load('return 1')()"), "");
    prueba("patron", &format!("local x = ('a'):rep(1500):find('.-.-.-.-b')\n{}", efecto("return e.izq")), "demasiado largo");
    prueba("memoria", &format!("local x = ('a'):rep(1e9)\n{}", efecto("return e.izq")), "demasiado largo");
    prueba("caro", &efecto("local x = e.izq for i = 1, 1500 do x = x + sala(x * 0.001, 0.5, 0.5) end return x"), "");
    prueba(
        "ventana",
        "return { nombre = 'Malo', tipo = 'efecto', sonido = function(e) return e.izq end, interfaz = { dibujar = function(d) while true do end end } }",
        "ventana",
    );
    prueba("futuro", "return { nombre = 'Malo', tipo = 'efecto', api = 99, sonido = function(e) return e.izq end }", "más nueva");
    // Zips hechos con mala idea.
    let bueno = efecto("return e.izq, e.der");
    for (n, v) in [
        ("sube", vec![("../fuera.lua", bueno.as_bytes())]),
        ("raiz", vec![("/tmp/fuera.lua", bueno.as_bytes())]),
        ("hondo", vec![("a/b/c/x.lua", bueno.as_bytes()), ("x.lua", bueno.as_bytes())]),
        ("sinlua", vec![("foto.txt", b"hola".as_slice())]),
    ] {
        let z = d.join(format!("{n}.zip"));
        std::fs::write(&z, zip_comprimido(&v)).unwrap();
        assert!(b.instalar(&z).is_err(), "{n}");
    }
    assert!(!d.join("fuera.lua").exists() && !d.join("Lua").join("fuera.lua").exists() && !Path::new("/tmp/fuera.lua").exists());
    // Un zip que dice pesar menos de lo que pesa, o cortado en cualquier parte, no truena.
    let z = zip_comprimido(&[("x.lua", &vec![b'-'; 200_000])]);
    for i in (0..z.len()).step_by(7) {
        let _ = leer_zip(&z[..i]);
        let mut m = z.clone();
        m[i] ^= 0x5a;
        let _ = leer_zip(&m);
    }
    // Ida y vuelta de los nuestros.
    let v: BTreeMap<String, Vec<u8>> = [("a/á.lua".to_string(), b"hola".to_vec()), ("a/b.png".into(), vec![0; 1000])].into();
    assert_eq!(leer_zip(&escribir_zip(&v)).unwrap(), v);
    // Nada quedó a medias.
    let quedan: Vec<_> = std::fs::read_dir(d.join("Lua")).unwrap().flatten().map(|e| e.file_name()).collect();
    assert!(quedan.is_empty(), "{quedan:?}");
}

#[test]
fn las_plantillas_sirven() {
    let d = lugar("plantillas");
    let mut b = bib(&d);
    for n in ["efecto", "instrumento"] {
        let (_, inf) = b.instalar(&Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("plantillas/{n}.lua"))).unwrap();
        assert!(inf.avisos.is_empty(), "{n}: {inf:?}");
    }
}

/// Los de fábrica pasan la misma revisión que se le pide a uno ajeno.
#[test]
fn los_de_fabrica_pasan_la_revision() {
    let b = Biblioteca::nueva(SR, None);
    for g in &b.guiones {
        let r = beethoven_daw_demo::instalar::revisar(&g.fuente, &g.archivos, SR);
        assert!(r.is_ok(), "{}: {:?}", g.clave, r.err());
    }
}
