//! La app en inglés (en su propio programa de pruebas: el idioma es global y las demás pruebas buscan en español).
//! `CAPTURAS=/carpeta` guarda imágenes.
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use toma_uno::idioma::{self, Idioma};
use toma_uno::ui::App;

fn foto(h: &mut Harness<App>, n: &str) {
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/en-{n}.png")).unwrap();
    }
}

#[test]
fn en_ingles() {
    idioma::poner(Idioma::En);
    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.state_mut().inicio.mostrar();
    h.run_steps(3);
    foto(&mut h, "menu");
    h.get_by_label("Open project…");
    h.get_by_label("Settings").click();
    h.run_steps(3);
    foto(&mut h, "configuracion");
    h.get_by_label("Done").click();
    h.run_steps(2);
    h.get_by_label("New video").click();
    for _ in 0..100 {
        if !h.state().inicio.visible {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        h.run_steps(1);
    }
    h.run_steps(3);
    for (n, v) in [("Video", "video"), ("Audio", "audio"), ("Effects", "efectos"), ("Playlist", "playlist")] {
        let r = h.get_all_by_label(n).map(|x| x.rect()).min_by(|a, b| a.top().total_cmp(&b.top())).unwrap();
        let o = eframe::egui::pos2(r.left() + 40.0, r.center().y);
        h.hover_at(o);
        h.run_steps(2);
        h.drag_at(o);
        h.run_steps(1);
        h.drop_at(o);
        h.run_steps(4);
        foto(&mut h, v);
    }
    h.get_by_label("FILE").click();
    h.run_steps(2);
    foto(&mut h, "archivo");
}
