// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Cada efecto en su carpeta: su .lua, otros .lua con `incluir`, imágenes PNG con `d.imagen` y su `logo`.
use beethoven_daw_demo::biblioteca::Biblioteca;
use beethoven_daw_demo::interfaz::{Momento, Pieza, cargar_con};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn carpeta(n: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("bdemo-carpetas-{n}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Un PNG de 2 × 2 de verdad.
fn png() -> Vec<u8> {
    let mut v = vec![];
    {
        let mut e = png::Encoder::new(&mut v, 2, 2);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.write_header().unwrap().write_image_data(&[255; 16]).unwrap();
    }
    v
}

fn imagenes(p: &[Pieza]) -> Vec<String> {
    p.iter().filter_map(|x| if let Pieza::Imagen { archivo, .. } = x { Some(archivo.clone()) } else { None }).collect()
}

#[test]
fn el_de_fabrica_trae_logo_imagenes_y_su_otro_lua() {
    let b = Biblioteca::nueva(48_000.0, None);
    let g = b.ver("vozgruesa").expect("Voz gruesa está de fábrica");
    assert_eq!(g.ficha.logo.as_deref(), Some("logo.png"));
    assert!(g.archivos.imagen("logo.png").is_some() && g.archivos.imagen("fondo.png").is_some());
    assert!(g.archivos.lua.contains_key("ayuda"), "trae ayuda.lua para incluir");
    assert!(g.receta.is_some(), "suena");
    let mut i = cargar_con(&g.fuente, g.archivos.clone()).unwrap().expect("trae interfaz");
    let piezas = i.cuadro(&BTreeMap::new(), &Momento::default());
    assert!(i.error.is_none(), "{:?}", i.error);
    assert_eq!(imagenes(&piezas), ["fondo.png", "logo.png"]);
    let (w, h, px) = beethoven_daw_demo::paquete::rgba(g.archivos.imagen("logo.png").unwrap()).unwrap();
    assert_eq!((w, h, px.len()), (256, 256, 256 * 256 * 4));
}

#[test]
fn uno_tuyo_en_su_carpeta() {
    let d = carpeta("tuyo");
    let c = d.join("voz grave");
    std::fs::create_dir_all(c.join("fondos")).unwrap();
    std::fs::write(c.join("logo.png"), png()).unwrap();
    std::fs::write(c.join("fondos/noche.png"), png()).unwrap();
    std::fs::write(c.join("cuentas.lua"), "return { doble = function(x) return x * 2 end }").unwrap();
    std::fs::write(
        c.join("voz grave.lua"),
        r#"local k = incluir("cuentas")
return { nombre = "Voz grave", tipo = "efecto", logo = "logo.png",
  controles = { { "g", "Grave", 0, 1, 0.5 } },
  sonido = function(e, c) return e.izq * k.doble(c.g), e.der end,
  interfaz = { ancho = 200, alto = 120, dibujar = function(d, c, m) d.imagen(0, 0, 200, 120, "fondos/noche.png", 0.5) end } }"#,
    )
    .unwrap();
    // Uno suelto, como antes, también sirve.
    std::fs::write(d.join("suelto.lua"), r#"return { nombre = "Suelto", tipo = "efecto", sonido = function(e) return e.izq, e.der end }"#).unwrap();
    let b = Biblioteca::nueva(48_000.0, Some(d.clone()));
    assert!(b.avisos.is_empty(), "{:?}", b.avisos);
    let g = b.ver("voz grave").expect("se lee de su carpeta");
    assert!(g.propio && g.carpeta.as_deref() == Some(c.as_path()));
    assert_eq!(g.ficha.logo.as_deref(), Some("logo.png"));
    let mut i = cargar_con(&g.fuente, g.archivos.clone()).unwrap().unwrap();
    assert_eq!(imagenes(&i.cuadro(&BTreeMap::new(), &Momento::default())), ["fondos/noche.png"]);
    assert!(b.ver("suelto").is_some());
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn los_errores_se_explican() {
    let d = carpeta("errores");
    let poner = |n: &str, lua: &str, extra: &[(&str, &[u8])]| {
        let c = d.join(n);
        std::fs::create_dir_all(&c).unwrap();
        std::fs::write(c.join(format!("{n}.lua")), lua).unwrap();
        for (a, b) in extra {
            std::fs::write(c.join(a), b).unwrap();
        }
    };
    let base = |resto: &str| format!(r#"return {{ nombre = "X", tipo = "efecto", sonido = function(e) return e.izq, e.der end, {resto} }}"#);
    poner("sinlogo", &base(r#"logo = "logo.png""#), &[]);
    poner("noespng", &base(""), &[("foto.png", b"no soy una imagen")]);
    poner("sinmodulo", &format!("local m = incluir(\"nada\")\n{}", base("")), &[]);
    poner("afuera", &format!("local m = incluir(\"../sinlogo/sinlogo\")\n{}", base("")), &[]);
    let b = Biblioteca::nueva(48_000.0, Some(d.clone()));
    let aviso = |n: &str| b.avisos.iter().find(|a| a.starts_with(n)).cloned().unwrap_or_default();
    assert!(aviso("sinlogo").contains("logo.png"), "{}", aviso("sinlogo"));
    assert!(aviso("noespng").contains("PNG"), "{}", aviso("noespng"));
    assert!(aviso("sinmodulo").contains("nada.lua"), "{}", aviso("sinmodulo"));
    assert!(aviso("afuera").contains("no hay"), "solo ve los .lua de su carpeta: {}", aviso("afuera"));
    // Una imagen que no está en la carpeta, en la interfaz.
    let f = r#"return { nombre = "X", tipo = "efecto", sonido = function(e) return e.izq, e.der end,
      interfaz = { dibujar = function(d) d.imagen(0, 0, 10, 10, "/etc/passwd") end } }"#;
    let mut i = cargar_con(f, Default::default()).unwrap().unwrap();
    i.cuadro(&BTreeMap::new(), &Momento::default());
    assert!(i.error.as_deref().unwrap_or("").contains("no hay una imagen"), "{:?}", i.error);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn tu_copia_de_uno_de_fabrica_se_lleva_sus_archivos() {
    let d = carpeta("copia");
    let mut b = Biblioteca::nueva(48_000.0, Some(d.clone()));
    let fuente = b.ver("vozgruesa").unwrap().fuente.replace("Voz gruesa", "Mi voz gruesa");
    b.guardar("vozgruesa", &fuente).unwrap();
    let c = d.join("vozgruesa");
    for a in ["vozgruesa.lua", "ayuda.lua", "logo.png", "fondo.png"] {
        assert!(c.join(a).is_file(), "falta {a}");
    }
    let b = Biblioteca::nueva(48_000.0, Some(d.clone()));
    let g = b.ver("vozgruesa").unwrap();
    assert!(g.propio && g.ficha.nombre.ver().contains("Mi voz gruesa"), "{}", g.ficha.nombre.ver());
    let _ = std::fs::remove_dir_all(d);
}
