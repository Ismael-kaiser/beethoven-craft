use std::path::PathBuf;
use toma_uno::edicion::*;
use toma_uno::modelo::*;

/// Tres clips de video pegados (0-4, 4-8, 8-12) con su audio vinculado, con 2 s de sobra a cada lado.
fn base() -> (Proyecto, [u64; 3]) {
    let mut p = Proyecto::nuevo();
    p.pistas.clear();
    let pista = p.nueva_pista("a");
    let mut ids = [0; 3];
    for k in 0..3 {
        let mut c = p.nuevo_video(PathBuf::from("/v.mp4"), Contenido::Archivo, k as f64 * 4.0, 4.0);
        (c.desde, c.total) = (2.0, 8.0);
        ids[k] = c.id;
        p.video.push(c);
        let id = p.id();
        p.pista(pista).unwrap().clips.push(Clip {
            id, ruta: "/v.mp4".into(), nombre: "v".into(), inicio: k as f64 * 4.0, desde: 2.0, dur: 4.0, total: 8.0,
            ganancia_db: 0.0, fade_in: 0.0, fade_out: 0.0, invertido: false, grupo: None, velocidad: 1.0, bus: None, color: None
        });
        p.vincular(&[ids[k], id]);
    }
    (p, ids)
}

/// (inicio, dur, desde) de los clips de video; comprueba que el audio los sigue.
fn foto(p: &Proyecto) -> Vec<(f64, f64, f64)> {
    let r = |x: f64| (x * 1000.0).round() / 1000.0;
    let mut v: Vec<_> = p.video.iter().map(|c| (r(c.inicio), r(c.dur), r(c.desde))).collect();
    let mut a: Vec<_> = p.pistas[0].clips.iter().map(|c| (r(c.inicio), r(c.dur), r(c.desde))).collect();
    v.sort_by(|x, y| x.0.total_cmp(&y.0));
    a.sort_by(|x, y| x.0.total_cmp(&y.0));
    assert_eq!(v, a, "el audio sigue a la imagen");
    v
}

#[test]
fn mover_y_vinculo() {
    let (mut p, i) = base();
    p.mover_grupo(i[2], 1.5);
    assert_eq!(foto(&p)[2], (9.5, 4.0, 2.0));
    p.mover_grupo(i[0], -5.0);
    assert_eq!(foto(&p)[0].0, 0.0);
    p.desvincular(i[2]);
    p.mover_grupo(i[2], 1.0);
    assert_eq!(p.video[2].inicio, 10.5);
    assert_eq!(p.pistas[0].clips[2].inicio, 9.5);
}

#[test]
fn recorte_normal_y_ondulado() {
    let (mut p, i) = base();
    p.recortar(i[1], Orilla::Izq, 1.0);
    assert_eq!(foto(&p), [(0.0, 4.0, 2.0), (5.0, 3.0, 3.0), (8.0, 4.0, 2.0)]);
    let (mut p, i) = base();
    p.recorte_ondulado(i[1], Orilla::Der, -1.5);
    assert_eq!(foto(&p), [(0.0, 4.0, 2.0), (4.0, 2.5, 2.0), (6.5, 4.0, 2.0)]);
    let (mut p, i) = base();
    p.recorte_ondulado(i[0], Orilla::Der, 1.0);
    assert_eq!(foto(&p), [(0.0, 5.0, 2.0), (5.0, 4.0, 2.0), (9.0, 4.0, 2.0)]);
    let (mut p, i) = base();
    p.recorte_ondulado(i[1], Orilla::Izq, 1.0);
    assert_eq!(foto(&p), [(0.0, 4.0, 2.0), (4.0, 3.0, 3.0), (7.0, 4.0, 2.0)]);
    // No hay más de 2 s de sobra.
    let (mut p, i) = base();
    assert_eq!(p.recorte_ondulado(i[2], Orilla::Der, 9.0), 2.0);
}

#[test]
fn rodar_deslizar_correr() {
    let (mut p, i) = base();
    assert_eq!(p.rodar(i[0], Orilla::Der, 1.0), 1.0);
    assert_eq!(foto(&p), [(0.0, 5.0, 2.0), (5.0, 3.0, 3.0), (8.0, 4.0, 2.0)]);
    let (mut p, i) = base();
    p.deslizar(i[1], 1.5);
    assert_eq!(foto(&p)[1], (4.0, 4.0, 3.5));
    p.deslizar(i[1], 99.0);
    assert_eq!(foto(&p)[1], (4.0, 4.0, 4.0));
    let (mut p, i) = base();
    assert_eq!(p.correr(i[1], -1.0), -1.0);
    assert_eq!(foto(&p), [(0.0, 3.0, 2.0), (3.0, 4.0, 2.0), (7.0, 5.0, 1.0)]);
}

#[test]
fn estirar_y_dividir() {
    let (mut p, i) = base();
    assert_eq!(p.estirar(i[2], 2.0), Some(2.0));
    assert_eq!(foto(&p)[2], (8.0, 2.0, 2.0));
    assert_eq!(p.pistas[0].clips[2].velocidad, 2.0);
    assert!(p.pistas[0].clips[2].clave().to_string_lossy().ends_with("#v2.0000"));
    // Dividir a 2x: el segundo pedazo arranca 2 s más adentro del archivo por cada segundo.
    assert!(p.dividir_grupo(i[2], 9.0));
    assert_eq!(foto(&p)[3], (9.0, 1.0, 4.0));
    let der = p.video.iter().find(|c| c.inicio == 9.0).unwrap().id;
    assert_eq!(p.vinculados(der).len(), 2, "los pedazos nuevos quedan vinculados entre sí");
    assert_eq!(p.vinculados(i[2]).len(), 2);
}

#[test]
fn borrar_levantar_extraer() {
    let (mut p, i) = base();
    p.borrar_ondulado(i[1]);
    assert_eq!(foto(&p), [(0.0, 4.0, 2.0), (4.0, 4.0, 2.0)]);
    let (mut p, _) = base();
    p.quitar_rango(3.0, 9.0, false);
    assert_eq!(foto(&p), [(0.0, 3.0, 2.0), (9.0, 3.0, 3.0)]);
    let (mut p, _) = base();
    p.quitar_rango(3.0, 9.0, true);
    assert_eq!(foto(&p), [(0.0, 3.0, 2.0), (3.0, 3.0, 3.0)]);
    let (mut p, _) = base();
    p.quitar_rango(1.0, 2.0, true);
    assert_eq!(foto(&p), [(0.0, 1.0, 2.0), (1.0, 2.0, 4.0), (3.0, 4.0, 2.0), (7.0, 4.0, 2.0)]);
}

#[test]
fn pistas_bloqueadas_no_se_tocan() {
    let (mut p, i) = base();
    p.pistas[0].bloqueada = true;
    p.borrar_ondulado(i[0]);
    assert_eq!(p.pistas[0].clips.len(), 3);
    assert_eq!(p.pistas[0].clips[1].inicio, 4.0);
    assert_eq!(p.video[0].inicio, 0.0);
}

#[test]
fn srt() {
    let v = leer_srt("1\r\n00:00:01,500 --> 00:00:03,000\r\nHola\r\nmundo\r\n\r\n2\n00:01:00,000 --> 00:01:02,250\nAdiós\n");
    assert_eq!(v, vec![(1.5, 1.5, "Hola\nmundo".to_string()), (60.0, 2.25, "Adiós".to_string())]);
}

#[test]
fn proyecto_viejo_se_abre() {
    // Un archivo de la primera versión, sin los campos nuevos.
    let s = r#"{"pistas":[{"id":2,"nombre":"V","vol_db":0.0,"pan":0.0,"mudo":false,"solo":false,"armada":true,
      "clips":[{"id":5,"ruta":"/a.wav","nombre":"a","inicio":0.0,"desde":0.0,"dur":1.0,"total":1.0,"ganancia_db":0.0,"fade_in":0.0,"fade_out":0.0,"invertido":false}],"fx":[]}],
      "video":[{"id":7,"ruta":"/v.mp4","inicio":0.0,"desde":0.0,"dur":1.0,"total":1.0,"ancho":1920,"alto":1080,"fps":30.0}],
      "maestro_db":0.0,"fx_maestro":[],"sig_id":9,"ancho":1920,"alto":1080,"fps":30.0,"ajuste_ms":0.0}"#;
    let p: Proyecto = serde_json::from_str(s).unwrap();
    assert_eq!((p.capas.len(), p.bpm, p.pistas[0].clips[0].velocidad), (1, 120.0, 1.0));
}

#[test]
fn nombres_de_pista_siguen_su_lugar() {
    let mut p = toma_uno::modelo::Proyecto::nuevo();
    p.capas.push(p.capas[0].clone());
    p.capas.push(p.capas[0].clone());
    p.capas[1].nombre = "V3".into();
    p.capas[2].nombre = "Fondo".into();
    p.renombrar_capas();
    let n: Vec<_> = p.capas.iter().map(|c| c.nombre.as_str()).collect();
    assert_eq!(n, ["V1", "V2", "Fondo"]);
}
