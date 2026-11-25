//! Clases de clips: los efectos que se le ponen a uno les pasan a todos.
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use toma_uno::composicion::fx_nuevo;
use toma_uno::modelo::*;
use toma_uno::ui::App;

fn proyecto() -> (Proyecto, Vec<u64>) {
    let mut p = Proyecto::nuevo();
    let mut ids = vec![];
    for k in 0..3 {
        let mut c = p.nuevo_video("c".into(), Contenido::Color([0.2 * k as f32, 0.4, 0.3, 1.0]), k as f64 * 2.0, 2.0);
        // Cada uno con algo propio.
        c.fx.push(fx_nuevo(p.id(), "ec.blur.gaussian").unwrap());
        ids.push(c.id);
        p.video.push(c);
    }
    (p, ids)
}
fn tipos(p: &Proyecto, id: u64) -> Vec<String> {
    p.video.iter().find(|c| c.id == id).unwrap().fx.iter().map(|f| f.tipo.clone()).collect()
}
/// Un cambio, como lo ve la app: antes y después.
fn cambiar(p: &mut Proyecto, f: impl FnOnce(&mut Proyecto)) {
    let antes = p.clone();
    f(p);
    p.igualar_clases(&antes);
}

#[test]
fn se_comparten() {
    let (mut p, ids) = proyecto();
    let k = p.nueva_clase("Coro", [1, 2, 3]);
    cambiar(&mut p, |p| ids[..2].iter().for_each(|i| p.clip_video(*i).unwrap().clase = Some(k)));
    assert!(ids.iter().all(|i| tipos(&p, *i) == ["ec.blur.gaussian"]), "entrar no cambia lo propio");
    // Poner uno: les llega a los dos de la clase y no al otro.
    let f = p.id();
    cambiar(&mut p, |p| p.clip_video(ids[0]).unwrap().fx.push(fx_nuevo(f, "bc:vhs").unwrap()));
    assert_eq!(tipos(&p, ids[1]), ["ec.blur.gaussian", "bc:vhs"]);
    assert_eq!(tipos(&p, ids[2]), ["ec.blur.gaussian"]);
    // Mover un control desde el otro: también. Lo propio no se comparte.
    cambiar(&mut p, |p| p.clip_video(ids[1]).unwrap().fx[1].p.get_mut("cant").unwrap().v = Valor::Num(33.0));
    assert_eq!(p.clip_video(ids[0]).unwrap().fx[1].p["cant"].v.num(), 33.0);
    cambiar(&mut p, |p| p.clip_video(ids[1]).unwrap().fx[0].activo = false);
    assert!(p.clip_video(ids[0]).unwrap().fx[0].activo);
    // El que entra después recibe lo compartido.
    cambiar(&mut p, |p| p.clip_video(ids[2]).unwrap().clase = Some(k));
    assert_eq!(tipos(&p, ids[2]), ["ec.blur.gaussian", "bc:vhs"]);
    assert_eq!(p.clip_video(ids[2]).unwrap().fx[1].p["cant"].v.num(), 33.0);
    // Los nodos también.
    cambiar(&mut p, |p| p.clip_video(ids[2]).unwrap().nodos = Some(Default::default()));
    assert!(ids.iter().all(|i| p.clip_video(*i).unwrap().nodos.is_some()));
    // Partir un clip: las dos mitades siguen en la clase.
    cambiar(&mut p, |p| { p.dividir(ids[0], 1.0); });
    assert_eq!(p.video.iter().filter(|c| c.clase == Some(k) && c.fx.len() == 2).count(), 4);
    // Quitarlo de uno lo quita de todos.
    cambiar(&mut p, |p| { p.clip_video(ids[1]).unwrap().fx.remove(1); });
    assert!(p.video.iter().all(|c| c.fx.len() == 1) && p.clase(k).unwrap().fx.is_empty());
    // Pasar también lo propio; y al borrar la clase, cada quien se queda con lo suyo.
    p.compartir_todo(ids[0]);
    assert_eq!(tipos(&p, ids[1]).len(), 2);
    p.quitar_clase(k);
    assert!(p.video.iter().all(|c| c.clase.is_none() && !c.fx.is_empty()));
    let r = std::env::temp_dir().join(format!("clases-{}.bcraft", std::process::id()));
    toma_uno::archivo::guardar(&p, &r, None, false).unwrap();
    assert!(toma_uno::archivo::abrir(&r, &std::env::temp_dir()).unwrap() == p);
    let _ = std::fs::remove_file(r);
}

/// Desde la ventana: clase nueva para los elegidos, y un efecto a uno les llega a todos; deshacer lo regresa.
#[test]
fn desde_la_ventana() {
    let mut h = Harness::builder().with_size([1440.0, 860.0]).with_step_dt(1.0 / 60.0).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    let (p, ids) = proyecto();
    h.state_mut().p = p;
    h.state_mut().linea.encuadrar = true;
    h.run_steps(3);
    let (a, id) = (h.state_mut(), ids[0]);
    a.seleccion = ids[..2].iter().copied().collect();
    a.clip = Some(id);
    let k = a.clase_nueva();
    for i in &ids[..2] {
        a.p.clip_video(*i).unwrap().clase = Some(k);
    }
    a.p.clase(k).unwrap().nombre = "Coro".into();
    h.run_steps(3);
    h.state_mut().agregar_fx_video(id, "bc:neon");
    h.run_steps(3);
    assert_eq!(tipos(&h.state().p, ids[1]), ["ec.blur.gaussian", "bc:neon"]);
    assert_eq!(tipos(&h.state().p, ids[2]), ["ec.blur.gaussian"]);
    // El menú del clip.
    h.get_by_label("Clip de video 0").click_secondary();
    h.run_steps(4);
    h.get_by(|n| n.label().is_some_and(|l| l.starts_with("Clase"))).click();
    h.run_steps(5);
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/clase-menu.png")).unwrap();
    }
    h.get_by_label("+ Nueva clase");
}
