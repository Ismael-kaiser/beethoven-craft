//! MIDI: archivos .mid y teclados conectados.
use crate::modelo::Nota;
use crossbeam_channel::{Receiver, unbounded};

/// Lee un .mid. Devuelve (tempo en bpm si lo dice, notas por pista o canal).
pub fn leer(b: &[u8]) -> Option<(Option<f64>, Vec<(String, Vec<Nota>)>)> {
    if b.get(..4)? != b"MThd" {
        return None;
    }
    let u16_ = |i: usize| -> Option<u16> { Some(u16::from_be_bytes(b.get(i..i + 2)?.try_into().ok()?)) };
    let u32_ = |i: usize| -> Option<u32> { Some(u32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?)) };
    let div = u16_(12)?;
    if div & 0x8000 != 0 || div == 0 {
        return None;
    }
    let (mut i, mut bpm, mut out) = (8 + u32_(4)? as usize, None, vec![]);
    while i + 8 <= b.len() {
        let largo = u32_(i + 4)? as usize;
        let es = &b[i..i + 4] == b"MTrk";
        let t = b.get(i + 8..(i + 8 + largo).min(b.len()))?;
        i += 8 + largo;
        if !es {
            continue;
        }
        let (mut k, mut tick, mut estado, mut nombre) = (0usize, 0u64, 0u8, String::new());
        // Por canal: notas cerradas y notas que siguen sonando.
        let mut notas: Vec<Vec<Nota>> = vec![vec![]; 16];
        let mut abiertas: Vec<(u8, u8, u8, u64)> = vec![];
        let var = |k: &mut usize| -> Option<u64> {
            let mut v = 0u64;
            for _ in 0..4 {
                let x = *t.get(*k)?;
                *k += 1;
                v = v << 7 | (x & 0x7f) as u64;
                if x & 0x80 == 0 {
                    break;
                }
            }
            Some(v)
        };
        while k < t.len() {
            let Some(d) = var(&mut k) else { break };
            tick += d;
            let Some(&x) = t.get(k) else { break };
            if x & 0x80 != 0 {
                estado = x;
                k += 1;
            }
            match estado {
                0xff => {
                    let Some(&tipo) = t.get(k) else { break };
                    k += 1;
                    let Some(n) = var(&mut k) else { break };
                    let Some(d) = t.get(k..k + n as usize) else { break };
                    k += n as usize;
                    match tipo {
                        0x51 if d.len() == 3 && bpm.is_none() => {
                            let us = (d[0] as u32) << 16 | (d[1] as u32) << 8 | d[2] as u32;
                            bpm = Some(60e6 / us.max(1) as f64);
                        }
                        0x03 => nombre = String::from_utf8_lossy(d).trim().to_string(),
                        0x2f => break,
                        _ => {}
                    }
                }
                0xf0 | 0xf7 => {
                    let Some(n) = var(&mut k) else { break };
                    k += n as usize;
                }
                _ => {
                    let n = if matches!(estado & 0xf0, 0xc0 | 0xd0) { 1 } else { 2 };
                    let Some(d) = t.get(k..k + n) else { break };
                    k += n;
                    let c = estado & 0x0f;
                    let cerrar = |abiertas: &mut Vec<(u8, u8, u8, u64)>, notas: &mut Vec<Vec<Nota>>| {
                        if let Some(j) = abiertas.iter().position(|a| a.0 == c && a.1 == d[0]) {
                            let a = abiertas.remove(j);
                            let p = |x: u64| x as f64 / div as f64;
                            notas[c as usize].push(Nota { tecla: a.1, vel: a.2, ini: p(a.3), dur: p(tick - a.3).max(0.05) });
                        }
                    };
                    match estado & 0xf0 {
                        0x90 if d[1] > 0 => {
                            cerrar(&mut abiertas, &mut notas);
                            abiertas.push((c, d[0], d[1], tick));
                        }
                        0x80 | 0x90 => cerrar(&mut abiertas, &mut notas),
                        _ => {}
                    }
                }
            }
        }
        for (c, n) in notas.into_iter().enumerate().filter(|(_, n)| !n.is_empty()) {
            let nom = if c == 9 { "Batería".into() } else if nombre.is_empty() { crate::idioma::tf("MIDI {}", &[&(out.len() + 1)]) } else { nombre.clone() };
            out.push((nom, n));
        }
    }
    Some((bpm, out))
}

/// Escribe un .mid de una pista.
pub fn escribir(notas: &[Nota], bpm: f64, nombre: &str) -> Vec<u8> {
    const DIV: f64 = 480.0;
    let mut ev: Vec<(u64, [u8; 3])> = vec![];
    for n in notas {
        let t = |x: f64| (x * DIV).round() as u64;
        ev.push((t(n.ini), [0x90, n.tecla, n.vel.max(1)]));
        ev.push((t(n.ini + n.dur).max(t(n.ini) + 1), [0x80, n.tecla, 0]));
    }
    ev.sort_by_key(|e| (e.0, e.1[0]));
    let var = |v: u64, o: &mut Vec<u8>| {
        let mut b = vec![(v & 0x7f) as u8];
        let mut v = v >> 7;
        while v > 0 {
            b.push((v & 0x7f) as u8 | 0x80);
            v >>= 7;
        }
        o.extend(b.iter().rev());
    };
    let mut t = vec![0, 0xff, 0x51, 3];
    t.extend(&((60e6 / bpm).round() as u32).to_be_bytes()[1..]);
    t.extend([0, 0xff, 0x03]);
    var(nombre.len() as u64, &mut t);
    t.extend(nombre.as_bytes());
    let mut antes = 0;
    for (k, e) in ev {
        var(k - antes, &mut t);
        antes = k;
        t.extend(e);
    }
    t.extend([0, 0xff, 0x2f, 0]);
    let mut o = b"MThd\0\0\0\x06\0\0\0\x01".to_vec();
    o.extend((DIV as u16).to_be_bytes());
    o.extend(b"MTrk");
    o.extend((t.len() as u32).to_be_bytes());
    o.extend(t);
    o
}

/// Un teclado conectado. Entrega (tecla, fuerza); fuerza 0 es soltar.
pub struct Teclado {
    _c: midir::MidiInputConnection<()>,
    pub rx: Receiver<(u8, u8)>,
    pub nombre: String,
}

pub fn teclados() -> Vec<String> {
    let Ok(m) = midir::MidiInput::new("Beethoven Craft") else { return vec![] };
    m.ports().iter().filter_map(|p| m.port_name(p).ok()).collect()
}

pub fn conectar(nombre: &str) -> Result<Teclado, String> {
    let m = midir::MidiInput::new("Beethoven Craft").map_err(|e| e.to_string())?;
    let p = m.ports().into_iter().find(|p| m.port_name(p).is_ok_and(|n| n == nombre)).ok_or("Ya no está ese teclado")?;
    let (tx, rx) = unbounded();
    let c = m
        .connect(
            &p,
            "entrada",
            move |_, b, _| {
                if b.len() >= 3 && matches!(b[0] & 0xf0, 0x80 | 0x90) {
                    let v = if b[0] & 0xf0 == 0x80 { 0 } else { b[2] };
                    let _ = tx.send((b[1], v));
                }
            },
            (),
        )
        .map_err(|e| crate::idioma::tf("No pude conectar el teclado: {}", &[&e]))?;
    Ok(Teclado { _c: c, rx, nombre: nombre.into() })
}
