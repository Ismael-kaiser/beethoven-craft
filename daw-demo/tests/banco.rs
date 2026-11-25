//! El banco de instrumentos grabados (MuseScore General, .sf3). Lo baja `empaque/sonidos.sh`; sin él, no se prueba.
use beethoven_daw_demo::muestrario;
use std::path::Path;

fn banco() -> Option<std::path::PathBuf> {
    let r = Path::new(env!("CARGO_MANIFEST_DIR")).join("sonidos/MuseScore_General.sf3");
    r.is_file().then_some(r)
}

#[test]
fn se_abre_el_sf3() {
    let Some(r) = banco() else { return eprintln!("sin banco: corre empaque/sonidos.sh") };
    let v = muestrario::presets(&r);
    if std::env::var_os("LISTA").is_some() {
        for (i, n) in v.iter().enumerate() {
            println!("{i} {n}");
        }
    }
    assert!(v.len() >= 128, "{}", v.len());
    let t = std::time::Instant::now();
    let piano = muestrario::abrir(&r, 0).unwrap();
    println!(
        "piano: {} zonas, {:.1} s de audio, soltar {:?}, {:?}",
        piano.zonas.len(),
        piano.datos.len() as f32 / 44100.0,
        piano.soltar,
        t.elapsed()
    );
    assert!(piano.zonas.len() > 4);
    assert!(piano.datos.iter().any(|x| x.abs() > 0.1), "suena");
    for z in &piano.zonas {
        assert!(z.fin <= piano.datos.len() && z.ini < z.fin);
        if let Some((a, b)) = z.bucle {
            assert!(z.ini <= a && a < b && b <= z.fin);
        }
    }
}

#[test]
fn cada_instrumento_esta_en_el_banco() {
    if banco().is_none() {
        return;
    }
    use beethoven_daw_demo::banco;
    let falta: Vec<&str> = banco::INSTRUMENTOS.iter().filter(|x| banco::indice(x.preset).is_none()).map(|x| x.preset).collect();
    assert!(falta.is_empty(), "{falta:?}");
    for x in banco::INSTRUMENTOS.iter().filter(|x| x.nota.is_some()) {
        let m = muestrario::abrir(banco::archivo().unwrap(), banco::indice(x.preset).unwrap()).unwrap();
        let (_, n) = m.elegir(x.nota.unwrap(), 0.8);
        assert!(n > 0, "{} no tiene la tecla {:?}", x.clave, x.nota);
    }
}
