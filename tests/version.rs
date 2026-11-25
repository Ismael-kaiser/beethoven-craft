//! La versión vive en Cargo.toml: el Info.plist de la Mac (y con él el instalador de Windows) dice lo mismo.
#[test]
fn una_sola_version() {
    let plist = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/empaque/Info.plist")).unwrap();
    let esperado = format!("<key>CFBundleShortVersionString</key><string>{}</string>", env!("CARGO_PKG_VERSION"));
    assert!(plist.contains(&esperado), "Info.plist no dice la versión {} de Cargo.toml", env!("CARGO_PKG_VERSION"));
}

/// El menú muestra la versión; clic abre «Acerca de», con el creador y el enlace a GitHub.
#[test]
fn version_en_el_menu_y_acerca_de() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    let mut h = Harness::builder().with_size([1180.0, 740.0]).wgpu().build_eframe(|cc| toma_uno::ui::App::nueva(&cc.egui_ctx, true, true));
    h.state_mut().inicio.mostrar();
    h.run_steps(4);
    let v = format!("Versión {}", env!("CARGO_PKG_VERSION"));
    let foto = |h: &mut Harness<toma_uno::ui::App>, n: &str| {
        if let Ok(c) = std::env::var("CAPTURAS") {
            h.render().unwrap().save(format!("{c}/{n}.png")).unwrap();
        }
    };
    foto(&mut h, "menu-version");
    h.get_by_label(&v).click();
    h.run_steps(4);
    h.get_by_label("Ver el proyecto en GitHub");
    foto(&mut h, "acerca");
}
