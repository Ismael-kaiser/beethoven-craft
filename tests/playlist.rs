//! Cuánto tarda en dibujarse la Playlist al editar un proyecto grande. `cargo test --release --test playlist -- --ignored --nocapture`
use eframe::egui::{self, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use std::sync::Arc;
use std::time::Instant;
use toma_uno::composicion::fx_nuevo;
use toma_uno::modelo::*;
use toma_uno::ui::App;

#[test]
#[ignore]
fn editar_en_grande() {
    let mut h = Harness::builder().with_size([1440.0, 860.0]).with_step_dt(1.0 / 60.0).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    let seg = 600usize;
    let audio = toma_uno::medios::Audio::nuevo(vec![[0.1; 2]; seg * 48000]);
    let audio = Arc::new(audio);
    let app = h.state_mut();
    for k in 0..8 {
        app.almacen.insert(format!("/no/{k}.wav").into(), audio.clone());
    }
    for _ in 0..3 {
        app.p.capas.push(Capa { nombre: "V".into(), visible: true, bloqueada: false });
    }
    let fx = ["ec.color.lumetri", "bc:color", "ec.blur.gaussian", "bc:vhs"];
    for k in 0..160 {
        let capa = k % 4;
        let contenido = if capa == 0 { Contenido::Color([0.3, 0.5, 0.2, 1.0]) } else { Contenido::Titulo(toma_uno::titulos::plantilla(1).1) };
        let mut c = app.p.nuevo_video("t".into(), contenido, (k / 4) as f64 * 5.0, 4.5);
        c.capa = capa;
        c.fx = fx.iter().enumerate().map(|(i, f)| fx_nuevo(9000 + k as u64 * 10 + i as u64, f).unwrap()).collect();
        app.p.video.push(c);
    }
    for n in 0..8 {
        let id = app.p.nueva_pista(&format!("Audio {n}"));
        for k in 0..40 {
            let cid = app.p.id();
            let clip = Clip { id: cid, ruta: format!("/no/{n}.wav").into(), nombre: "x".into(), inicio: k as f64 * 5.0, desde: k as f64, dur: 4.5, total: 600.0, ganancia_db: 0.0, fade_in: 0.1, fade_out: 0.1, invertido: false, grupo: None, velocidad: 1.0, bus: None, color: None };
            app.p.pista(id).unwrap().clips.push(clip);
        }
    }
    h.run_steps(5);
    let mide = |h: &mut Harness<App>, n: &str, f: &mut dyn FnMut(&mut Harness<App>, usize)| {
        let mut v: Vec<f64> = (0..40)
            .map(|i| {
                f(h, i);
                let t = Instant::now();
                h.step();
                t.elapsed().as_secs_f64() * 1e3
            })
            .collect();
        v.sort_by(f64::total_cmp);
        println!("{n}: mediana {:.1} ms, peor {:.1} ms", v[20], v[39]);
    };
    mide(&mut h, "quieta", &mut |_, _| {});
    mide(&mut h, "mover el ratón", &mut |h, i| h.hover_at(egui::pos2(500.0 + i as f32 * 4.0, 650.0)));
    // Un clip de audio: buscar su caja por la posición.
    let a = h.get_all_by_label("Silenciar la pista").map(|x| x.rect().center()).min_by(|a, b| a.y.total_cmp(&b.y)).unwrap();
    for (n, y) in [("arrastrar audio", a.y), ("arrastrar video", a.y - 80.0)] {
        let o = egui::pos2(400.0, y);
        h.hover_at(o);
        h.step();
        h.drag_at(o);
        h.step();
        mide(&mut h, n, &mut |h, i| h.hover_at(o + vec2(3.0 * i as f32, 0.0)));
        h.drop_at(o + vec2(120.0, 0.0));
        mide(&mut h, "  al soltar", &mut |_, _| {});
    }
    mide(&mut h, "cambiar volumen", &mut |h, i| h.state_mut().p.pistas[0].vol_db = i as f32 * 0.1);
    mide(&mut h, "cambiar un efecto", &mut |h, i| h.state_mut().p.video[0].mov.opacidad.v = Valor::Num(50.0 + i as f64));
    mide(&mut h, "partir", &mut |h, i| { let id = h.state().p.pistas[2].clips[i].id; let t = h.state().p.pistas[2].clips[i].inicio + 2.0; h.state_mut().p.dividir(id, t); });
}
