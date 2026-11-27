//! Los plugins de Beethoven DAW dentro de Movie: suenan con el mismo motor, se guardan y abren su ventana.
//! `CAPTURAS=/carpeta` guarda imágenes.
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use std::path::PathBuf;
use std::sync::Arc;
use toma_uno::efectos::TipoFx;
use toma_uno::medios::Audio;
use toma_uno::modelo::*;
use toma_uno::motor::{self, Almacen};
use toma_uno::ui::{App, Canal};
use toma_uno::ui::consola::Agregar;

fn seno(n: usize) -> Vec<f32> {
    (0..n).flat_map(|i| [(std::f32::consts::TAU * 220.0 * i as f32 / SR as f32).sin() * 0.5; 2]).collect()
}

fn proyecto(x: Vec<f32>) -> (Proyecto, Almacen) {
    let mut p = Proyecto::nuevo();
    p.fx_maestro.clear();
    p.pistas.clear();
    let pista = p.nueva_pista("a");
    let a = Arc::new(Audio::nuevo(x.chunks(2).map(|f| [f[0], f[1]]).collect()));
    let total = a.dur();
    let id = p.id();
    let ruta = PathBuf::from("/x.wav");
    p.pista(pista).unwrap().clips.push(Clip {
        id, ruta: ruta.clone(), nombre: "x".into(), inicio: 0.0, desde: 0.0, dur: total, total,
        ganancia_db: 0.0, fade_in: 0.0, fade_out: 0.0, invertido: false, grupo: None, velocidad: 1.0, bus: None, color: None,
    });
    (p, Almacen::from([(ruta, a)]))
}

fn mezcla(p: &Proyecto, a: &Almacen) -> Vec<f32> {
    let mut v = vec![];
    motor::mezclar_todo(p, a, motor::cuadros(1.0), |_, b| {
        v.extend_from_slice(b);
        true
    });
    v
}

#[test]
fn se_ofrecen_los_del_daw() {
    let l = toma_uno::plugins::efectos();
    println!("{} plugins", l.len());
    assert!(l.len() > 30, "{}", l.len());
    assert!(l.iter().any(|x| x.0 == "fuego"));
}

#[test]
fn suena_se_guarda_y_se_apaga() {
    let x = seno(SR as usize);
    let (mut p, a) = proyecto(x.clone());
    let seco = mezcla(&p, &a);
    let id = p.id();
    let mut f = toma_uno::plugins::nuevo(id, "fuego").expect("abre el saturador de bulbos");
    assert_eq!(f.tipo, TipoFx::Plugin);
    assert!(!f.p.is_empty() && f.nombre() != TipoFx::Plugin.nombre());
    // Mucha saturación: tiene que cambiar la forma de la onda.
    f.p[0] = 9.0;
    p.fx_maestro.push(f.clone());
    let n = motor::preparar(&p, &a, &[]);
    assert_eq!(n.plugins.len(), 1, "el motor abre el plugin");
    let con = mezcla(&p, &a);
    let dif = con.iter().zip(&seco).skip(4000).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
    let pico = con.iter().skip(4000).fold(0f32, |m, v| m.max(v.abs()));
    println!("saturado: diferencia {dif:.3}, pico {pico:.3}");
    assert!(dif > 0.05 && pico > 0.05 && pico.is_finite());
    // Apagado, pasa igual.
    p.fx_maestro[0].activo = false;
    let apagado = mezcla(&p, &a);
    let d = apagado.iter().zip(&seco).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
    assert!(d < 1e-4, "{d}");
    // Se guarda y se abre igual; un proyecto viejo, sin plugin, también abre.
    let txt = serde_json::to_string(&p).unwrap();
    let q: Proyecto = serde_json::from_str(&txt).unwrap();
    assert!(q == p);
    let viejo = r#"{"id":1,"tipo":"Eco","activo":true,"p":[1.0]}"#;
    let g: Fx = serde_json::from_str(viejo).unwrap();
    assert!(g.plugin.is_none() && g.tipo == TipoFx::Eco);
}

#[test]
fn su_ventana_en_movie() {
    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    h.state_mut().inicio.visible = false;
    h.state_mut().agregar_fx(Canal::Maestro, Agregar::Plugin("fuego".into()));
    h.run_steps(8);
    let id = h.state().p.fx_maestro.last().map(|f| f.id).unwrap();
    assert!(h.state().plugins.contains_key(&id), "el motor lo abrió para la ventana");
    assert!(h.query_by_label("BEETHOVEN").is_some(), "se ve su ventana");
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/movie-plugin.png")).unwrap();
    }
    // Mover un control en el plugin llega al proyecto (y al historial).
    let n = h.state().plugins[&id].n.clone();
    n.poner(0, 2.5);
    n.movidos.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    h.run_steps(2);
    assert_eq!(h.state().p.fx_maestro.last().unwrap().p[0], 2.5);
    // Y al revés: cambiar el efecto (deshacer) llega al plugin.
    h.state_mut().p.fx_maestro.last_mut().unwrap().p[0] = 4.0;
    h.run_steps(3);
    assert_eq!(n.valor(0), 4.0);
}
