use toma_uno::midi::*;
use toma_uno::modelo::Nota;
#[test]
fn ida_y_vuelta() {
    let n = vec![Nota { tecla: 60, vel: 100, ini: 0.0, dur: 1.0 }, Nota { tecla: 64, vel: 80, ini: 0.5, dur: 0.25 }, Nota { tecla: 60, vel: 90, ini: 200.0, dur: 4.0 }];
    let b = escribir(&n, 97.0, "Piano ñ");
    let (bpm, p) = leer(&b).unwrap();
    assert!((bpm.unwrap() - 97.0).abs() < 0.01);
    assert_eq!(p[0].0, "Piano ñ");
    let mut l = p[0].1.clone();
    l.sort_by(|a, b| a.ini.total_cmp(&b.ini));
    assert_eq!(l, n);
    assert!(leer(b"no es midi").is_none());
    for k in 0..b.len() {
        // Un archivo cortado no truena.
        let _ = leer(&b[..k]);
    }
}
