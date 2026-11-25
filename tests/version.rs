//! La versión vive en Cargo.toml: el Info.plist de la Mac (y con él el instalador de Windows) dice lo mismo.
#[test]
fn una_sola_version() {
    let plist = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/empaque/Info.plist")).unwrap();
    let esperado = format!("<key>CFBundleShortVersionString</key><string>{}</string>", env!("CARGO_PKG_VERSION"));
    assert!(plist.contains(&esperado), "Info.plist no dice la versión {} de Cargo.toml", env!("CARGO_PKG_VERSION"));
}
