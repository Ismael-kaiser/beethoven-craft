//! Actualizaciones: comparar versiones, leer latest.json, revisar la huella y la ventana.
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use toma_uno::actualizar::*;

#[test]
fn versiones_y_lista() {
    assert!(mas_nueva("0.12.0", "0.11.0"));
    assert!(mas_nueva("v0.11.1", "0.11.0"));
    assert!(mas_nueva("1.0", "0.99.9"));
    assert!(!mas_nueva("0.11.0", "0.11.0"));
    assert!(!mas_nueva("0.10.9", "0.11.0"));
    let json = r#"{"version":"0.12.0","notas":"Cosas nuevas","mac":{"url":"https://x/mac.zip","sha256":"AB","peso":10},"windows":{"url":"https://x/w.exe","sha256":"cd","peso":20},"linux":{"url":"https://x/l.tar.gz","sha256":"ef","peso":30}}"#;
    let n = leer(json, "0.11.0").unwrap().expect("hay nueva");
    assert_eq!(n.version, "0.12.0");
    assert_eq!(n.url, format!("https://x/{}", match sistema() { "mac" => "mac.zip", "windows" => "w.exe", _ => "l.tar.gz" }));
    assert!(leer(json, "0.12.0").unwrap().is_none(), "misma versión: nada");
    assert!(leer("no es json", "0.11.0").is_err());
}

#[test]
fn huella_sha256() {
    let r = std::env::temp_dir().join(format!("bc-huella-{}", std::process::id()));
    std::fs::write(&r, b"hola").unwrap();
    assert_eq!(huella(&r).as_deref(), Some("b221d9dbb083a7f33428d7c2a3c3198ae925614d70210e28716ccaa7cd4ddb79"));
    let _ = std::fs::remove_file(r);
}

#[test]
fn ventana_de_version_nueva() {
    let mut h = Harness::builder().with_size([1200.0, 800.0]).wgpu().build_eframe(|cc| toma_uno::ui::App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    h.state_mut().nueva = Some(Nueva { version: "0.12.0".into(), notas: "• Cosas nuevas\n• Más rápido".into(), url: "https://x".into(), sha256: "00".into(), peso: 180_000_000 });
    h.run_steps(3);
    h.get_by_label("↑ Actualizar a 0.12.0").click();
    h.run_steps(3);
    assert!(h.state().ver_nueva);
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/actualizar.png")).unwrap();
    }
    h.get_by_label("Más tarde").click();
    h.run_steps(2);
    assert!(!h.state().ver_nueva);
}
