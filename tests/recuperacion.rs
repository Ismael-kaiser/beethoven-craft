//! Si la app se cierra de golpe, el trabajo se recupera al abrir.
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use toma_uno::recuperacion::{Sesion, anotar, cerrada_bien, pendiente};
use toma_uno::ui::App;

#[test]
fn se_recupera_solo_si_se_cerro_mal() {
    let dir = std::env::temp_dir().join(format!("bc-recuperacion-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut p = toma_uno::modelo::Proyecto::nuevo();
    p.bpm = 133.0;
    toma_uno::archivo::guardar(&p, &dir.join("Autoguardado.bcraft"), None, false).unwrap();
    let original = dir.join("Mi canción.bcraft");
    // Se cerró bien: nada que recuperar.
    anotar(&dir, &Sesion { ruta: Some(original.clone()), sin_guardar: true });
    cerrada_bien(&dir);
    assert_eq!(pendiente(&dir), None);
    // Todo guardado y se cayó: tampoco.
    anotar(&dir, &Sesion { ruta: Some(original.clone()), sin_guardar: false });
    assert_eq!(pendiente(&dir), None);
    // Cambios sin guardar y se cayó: se aparta una copia y se ofrece.
    anotar(&dir, &Sesion { ruta: Some(original.clone()), sin_guardar: true });
    let r = pendiente(&dir).expect("debía ofrecer recuperar");
    assert_eq!(r.ruta.as_deref(), Some(original.as_path()));
    assert!(r.archivo.starts_with(dir.join("Recuperación")) && r.archivo.is_file());
    // Solo una vez.
    assert_eq!(pendiente(&dir), None);

    // En la app: «Recuperar» abre lo apartado, sigue con su archivo de siempre y queda sin guardar.
    let mut h = Harness::builder().with_size([1200.0, 800.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    h.state_mut().recuperar = Some(r);
    h.run_steps(3);
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/recuperar.png")).unwrap();
    }
    h.get_by_label("Recuperar").click();
    h.run_steps(4);
    assert_eq!(h.state().p.bpm, 133.0);
    assert!(h.state().recuperar.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
