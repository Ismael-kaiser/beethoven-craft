//! El panel de la biblioteca con sus tarjetas y miniaturas. `CAPTURAS=/carpeta` guarda imágenes.
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use toma_uno::ui::App;

#[test]
fn tarjetas_con_vista() {
    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    let r = h.get_all_by_label("Video").map(|x| x.rect()).min_by(|a, b| a.top().total_cmp(&b.top())).unwrap();
    let o = eframe::egui::pos2(r.left() + 40.0, r.center().y);
    h.hover_at(o);
    h.run_steps(2);
    h.drag_at(o);
    h.run_steps(1);
    h.drop_at(o);
    h.run_steps(3);
    for (n, foto) in [("Transiciones", "lib-transiciones"), ("Títulos", "lib-titulos"), ("Estilos", "lib-estilos")] {
        h.get_by_label(n).click();
        // Las miniaturas se hacen en segundo plano.
        for _ in 0..120 {
            std::thread::sleep(std::time::Duration::from_millis(25));
            h.run_steps(1);
        }
        if let Ok(c) = std::env::var("CAPTURAS") {
            h.render().unwrap().save(format!("{c}/{foto}.png")).unwrap();
        }
    }
}
