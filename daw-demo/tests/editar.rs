//! Las operaciones del editor de audio.
use beethoven_daw_demo::editar::*;
use std::f32::consts::TAU;

const SR: f32 = 48000.0;

fn tono(hz: f32, seg: f32) -> Audio {
    (0..(SR * seg) as usize).map(|i| [(TAU * hz * i as f32 / SR).sin() * 0.5; 2]).collect()
}

fn ruido(n: usize, k: f32) -> Audio {
    let mut s = 7u32;
    (0..n)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            [(s as f32 / u32::MAX as f32 - 0.5) * k; 2]
        })
        .collect()
}

fn rms(a: &[[f32; 2]]) -> f32 {
    (a.iter().map(|x| x[0] * x[0]).sum::<f32>() / a.len() as f32).sqrt()
}

/// Cuántas veces por segundo cruza hacia arriba.
fn hz(a: &[[f32; 2]]) -> f32 {
    a.windows(2).filter(|w| w[0][0] <= 0.0 && w[1][0] > 0.0).count() as f32 / (a.len() as f32 / SR)
}

#[test]
fn niveles_y_orden() {
    let mut a = tono(440.0, 0.1);
    let n = a.len();
    normalizar(&mut a, 0..n);
    assert!((a.iter().fold(0.0f32, |m, x| m.max(x[0].abs())) - PICO).abs() < 0.001);
    ganancia(&mut a, 0..n, -6.0);
    assert!((rms(&a) - PICO * 0.5012 * 0.7071).abs() < 0.01);
    fundido(&mut a, 0..n, true);
    assert_eq!(a[0][0], 0.0);
    silenciar(&mut a, 100..200);
    assert!(a[100..200].iter().all(|x| x[0] == 0.0));
    let b = a.clone();
    invertir(&mut a, 0..n);
    invertir(&mut a, 0..n);
    assert_eq!(a, b);
}

#[test]
fn estirar_conserva_el_tono() {
    let a = tono(440.0, 1.0);
    for veces in [0.5, 1.5, 2.0] {
        let b = estirar(&a, SR, veces);
        assert_eq!(b.len(), (a.len() as f32 * veces).round() as usize);
        assert!((hz(&b) - 440.0).abs() < 8.0, "{veces}: {}", hz(&b));
        assert!((rms(&b) - rms(&a)).abs() < 0.05, "{veces}: {}", rms(&b));
    }
}

#[test]
fn cambiar_tono_conserva_el_largo() {
    let a = tono(440.0, 1.0);
    for (s, meta) in [(12.0, 880.0), (-12.0, 220.0), (7.0, 659.3)] {
        let b = cambiar_tono(&a, SR, s);
        assert_eq!(b.len(), a.len());
        assert!((hz(&b) - meta).abs() < meta * 0.02, "{s}: {}", hz(&b));
    }
}

#[test]
fn quita_el_ruido_y_deja_el_tono() {
    let n = SR as usize;
    let soplo = ruido(n, 0.05);
    let mut a = ruido(n * 2, 0.05);
    for (x, t) in a[n..].iter_mut().zip(tono(440.0, 1.0)) {
        *x = [x[0] + t[0], x[1] + t[1]];
    }
    let b = quitar_ruido(&a, &soplo);
    assert_eq!(b.len(), a.len());
    assert!(rms(&b[4800..n - 4800]) < rms(&a[4800..n - 4800]) * 0.2, "{}", rms(&b[4800..n - 4800]));
    assert!((rms(&b[n + 4800..2 * n - 4800]) - 0.3536).abs() < 0.03, "{}", rms(&b[n + 4800..2 * n - 4800]));
}

#[test]
fn acorta_los_silencios() {
    let mut a = tono(440.0, 0.5);
    a.extend(vec![[0.0; 2]; SR as usize * 2]);
    a.extend(tono(440.0, 0.5));
    let b = quitar_silencios(&a, SR);
    assert!((b.len() as f32 / SR - 1.1).abs() < 0.03, "{}", b.len() as f32 / SR);
}
