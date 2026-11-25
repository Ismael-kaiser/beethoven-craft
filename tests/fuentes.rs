//! Apartado Texto y agregar una fuente (en su propio programa: cambia HOME para no tocar la carpeta real).
//! `CAPTURAS=/carpeta` guarda imágenes.
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use toma_uno::ui::App;
use toma_uno::ui::texto::FuenteNueva;

fn foto(h: &mut Harness<App>, n: &str) {
    if let Ok(c) = std::env::var("CAPTURAS") {
        h.render().unwrap().save(format!("{c}/{n}.png")).unwrap();
    }
}

fn pestana(h: &mut Harness<App>, n: &str) {
    let r = h.get_all_by_label(n).map(|x| x.rect()).min_by(|a, b| a.top().total_cmp(&b.top())).unwrap();
    let o = eframe::egui::pos2(r.left() + 40.0, r.center().y);
    h.hover_at(o);
    h.run_steps(2);
    h.drag_at(o);
    h.run_steps(1);
    h.drop_at(o);
    h.run_steps(3);
}

#[test]
fn texto_y_fuentes() {
    let casa = std::env::temp_dir().join(format!("bc-fuentes-{}", std::process::id()));
    std::fs::create_dir_all(&casa).unwrap();
    // SAFETY: este programa de pruebas tiene una sola prueba; nadie más lee las variables a la vez.
    unsafe {
        std::env::set_var("HOME", &casa);
        std::env::set_var("USERPROFILE", &casa);
    }
    // Revisar: lo que no sirve se explica.
    let web = casa.join("letra.woff2");
    std::fs::write(&web, b"x").unwrap();
    assert!(toma_uno::titulos::revisar_fuente(&web).unwrap_err().contains("web"));

    let mut h = Harness::builder().with_size([1440.0, 860.0]).wgpu().build_eframe(|cc| App::nueva(&cc.egui_ctx, true, true));
    h.run_steps(3);
    pestana(&mut h, "Efectos");
    h.get_by_label("Texto").click();
    for _ in 0..30 {
        h.run_steps(1);
    }
    foto(&mut h, "texto");
    h.get_by_label("+  Agregar fuente").click();
    h.run_steps(3);
    assert!(matches!(h.state().fuente_nueva, Some(FuenteNueva::Explicar)));
    foto(&mut h, "fuente-explicar");

    // El diálogo del sistema no se puede manejar aquí: se elige el archivo directo.
    let candidatas = ["/System/Library/Fonts/Supplemental/Chalkduster.ttf", "C:\\Windows\\Fonts\\georgia.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf"];
    let Some(r) = candidatas.iter().map(std::path::PathBuf::from).find(|p| p.exists()) else { return };
    let nombre = toma_uno::titulos::revisar_fuente(&r).unwrap();
    h.state_mut().fuente_nueva = Some(FuenteNueva::Vista { ruta: r.clone(), nombre: nombre.clone(), texto: "Hola Beethoven".into(), tex: None });
    h.run_steps(4);
    foto(&mut h, "fuente-vista");
    h.get_by_label("Guardar fuente").click();
    h.run_steps(4);
    assert!(h.state().fuente_nueva.is_none());
    let guardada = toma_uno::titulos::carpeta_fuentes().join(r.file_name().unwrap());
    assert!(guardada.exists(), "no se copió a {}", guardada.display());
    assert!(toma_uno::titulos::letras().iter().any(|l| l.0 == nombre && l.1.as_deref().is_some_and(toma_uno::titulos::es_tuya)));
    for _ in 0..30 {
        h.run_steps(1);
    }
    foto(&mut h, "texto-con-tuya");
    // Sin título elegido: se crea uno con esa letra (lo que hace el doble clic).
    h.state_mut().usar_letra(&nombre);
    h.run_steps(3);
    let t = h.state().p.video.iter().find_map(|c| match &c.contenido {
        toma_uno::modelo::Contenido::Titulo(t) => Some(t.letra.clone()),
        _ => None,
    });
    assert_eq!(t.as_deref(), Some(nombre.as_str()));
    let _ = std::fs::remove_dir_all(&casa);
}
