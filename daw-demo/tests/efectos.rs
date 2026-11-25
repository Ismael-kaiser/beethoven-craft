//! Los efectos de fábrica: sus preajustes y lo que dibujan.
use beethoven_daw_demo::biblioteca::Biblioteca;
use beethoven_daw_demo::lua::Tipo;
use beethoven_daw_demo::ui::grafica::{Clase, medida};

#[test]
fn cada_preajuste_suena_y_no_revienta() {
    let b = Biblioteca::nueva(48000.0, None);
    let x: Vec<f32> = (0..48000).map(|i| (i as f32 * 0.03).sin() * 0.5 + (i as f32 * 0.7).sin() * 0.2).collect();
    let mut b2 = Biblioteca::nueva(48000.0, None);
    let (mut efectos, mut presets) = (0, 0);
    for g in b.guiones.iter().filter(|g| g.ficha.tipo == Tipo::Efecto) {
        efectos += 1;
        let r = g.receta.as_deref().unwrap();
        for (nombre, v) in &g.ficha.presets {
            presets += 1;
            let ctl: Vec<f32> = r.controles.iter().map(|k| v.get(&k.clave).copied().unwrap_or(k.valor)).collect();
            let mut e = beethoven_daw_demo::modelo::EfectoPuesto::nuevo(&g.clave);
            e.controles = v.clone();
            let h = if r.con_huella() { b2.huella(&e) } else { None };
            let y = beethoven_daw_demo::medir::correr_con(r, &ctl, &[x.clone(), x.clone()], h.as_deref());
            let pico = y.iter().flatten().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(pico > 0.01 && pico < 4.0, "{} / {}: {pico}", g.clave, nombre.es);
            assert!(!v.is_empty(), "{} / {}", g.clave, nombre.es);
        }
        if g.clave != "vozencaja" {
            assert!(g.ficha.presets.len() >= 2, "{} sin preajustes", g.clave);
        }
        if let Some(c) = Clase::de(&g.ficha.grupo.es) {
            let ctl: Vec<f32> = r.controles.iter().map(|k| k.valor).collect();
            let m = medida(c, r, &ctl, 48000.0, false);
            assert!(m[0].len() > 2 && m[0].iter().all(|v| (0.0..=1.0).contains(v)), "{}", g.clave);
        }
    }
    assert!(efectos >= 31 && presets >= 88, "{efectos} {presets}");
}

#[test]
fn la_grafica_dice_la_verdad() {
    let b = Biblioteca::nueva(48000.0, None);
    let ver = |clave: &str, c: Clase, cambios: &[(&str, f32)]| {
        let r = b.ver(clave).unwrap().receta.clone().unwrap();
        let ctl: Vec<f32> = r.controles.iter().map(|k| cambios.iter().find(|x| x.0 == k.clave).map_or(k.valor, |x| x.1)).collect();
        medida(c, &r, &ctl, 48000.0, false)
    };
    // El limitador no deja pasar del techo: con -12 dB, arriba queda plano.
    let m = ver("limitador", Clase::Niveles, &[("techo", -12.0)]);
    assert!((m[0][30] - (60.0 - 12.0) / 66.0).abs() < 0.01 && (m[0][10] - m[1][10]).abs() < 0.01, "{:?}", m[0]);
    // El filtro que quita agudos los baja.
    let m = ver("filtro", Clase::Frecuencias, &[("bajo", 700.0)]);
    assert!(m[0][150] < 0.2 && (m[0][60] - 0.5).abs() < 0.05, "{} {}", m[0][150], m[0][60]);
}
