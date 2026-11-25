// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! MIDI: lo que cualquier estudio sabe abrir y guardar.
use crate::idioma::t;
use crate::modelo::*;
use std::collections::BTreeMap;

/// Trae un MIDI al proyecto como un patrón nuevo. Devuelve el id del patrón.
pub fn importar(p: &mut Proyecto, b: &[u8]) -> Result<u32, String> {
    let malo = || t("El MIDI está cortado o dañado").to_string();
    if b.len() < 14 || &b[..4] != b"MThd" {
        return Err(t("No es un archivo MIDI").into());
    }
    let div = u16::from_be_bytes([b[12], b[13]]);
    if div & 0x8000 != 0 || div == 0 {
        return Err(t("MIDI con tiempo SMPTE: no se puede abrir").into());
    }
    let escala = |t: u64| (t * PPQ as u64 / div as u64).min(10_000_000) as u32;
    let mut i = 8 + u32::from_be_bytes([b[4], b[5], b[6], b[7]]) as usize;
    // (pista, canal MIDI) → notas
    let mut todo: BTreeMap<(usize, u8), Vec<Nota>> = BTreeMap::new();
    let mut nombres: BTreeMap<usize, String> = BTreeMap::new();
    let mut bpm = None;
    let mut pista = 0;
    while i + 8 <= b.len() {
        let n = u32::from_be_bytes([b[i + 4], b[i + 5], b[i + 6], b[i + 7]]) as usize;
        let d = b.get(i + 8..(i + 8).saturating_add(n)).ok_or_else(malo)?;
        if &b[i..i + 4] == b"MTrk" {
            let (mut k, mut t, mut estado) = (0usize, 0u64, 0u8);
            let mut abiertas: BTreeMap<(u8, u8), Vec<(u64, u8)>> = BTreeMap::new();
            let var = |k: &mut usize| -> Result<u64, String> {
                let mut v = 0u64;
                for _ in 0..4 {
                    let x = *d.get(*k).ok_or_else(malo)?;
                    *k += 1;
                    v = v << 7 | (x & 127) as u64;
                    if x & 128 == 0 {
                        return Ok(v);
                    }
                }
                Err(malo())
            };
            while k < d.len() {
                t += var(&mut k)?;
                let x = *d.get(k).ok_or_else(malo)?;
                if x & 0x80 != 0 {
                    estado = x;
                    k += 1;
                }
                match estado {
                    0xFF => {
                        let tipo = *d.get(k).ok_or_else(malo)?;
                        k += 1;
                        let n = var(&mut k)? as usize;
                        let m = d.get(k..k + n).ok_or_else(malo)?;
                        k += n;
                        match tipo {
                            0x51 if n == 3 && bpm.is_none() => {
                                let us = u32::from_be_bytes([0, m[0], m[1], m[2]]).max(1);
                                bpm = Some(60e6 / us as f32);
                            }
                            0x03 => {
                                nombres.insert(pista, String::from_utf8_lossy(m).trim().to_string());
                            }
                            _ => {}
                        }
                    }
                    0xF0 | 0xF7 => {
                        let n = var(&mut k)? as usize;
                        k += n;
                    }
                    0x80..=0xEF => {
                        let n = if matches!(estado >> 4, 0xC | 0xD) { 1 } else { 2 };
                        let m = d.get(k..k + n).ok_or_else(malo)?;
                        k += n;
                        let c = estado & 15;
                        match estado >> 4 {
                            9 if m[1] > 0 => abiertas.entry((c, m[0])).or_default().push((t, m[1])),
                            8 | 9 => {
                                let v = abiertas.entry((c, m[0])).or_default();
                                if !v.is_empty() {
                                    let (t0, vel) = v.remove(0);
                                    let tick = escala(t0);
                                    todo.entry((pista, c)).or_default().push(Nota {
                                        tick,
                                        dur: (escala(t) - tick).max(1),
                                        tono: m[0] & 127,
                                        fuerza: vel as f32 / 127.0,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => return Err(malo()),
                }
            }
            pista += 1;
        }
        i += 8 + n;
    }
    if todo.is_empty() {
        return Err(t("El MIDI no tiene notas").into());
    }
    if let Some(x) = bpm {
        p.bpm = x.clamp(20.0, 400.0);
    }
    let id = p.agregar_patron();
    for ((pista, c), mut notas) in todo {
        notas.sort_by_key(|n| (n.tick, n.tono));
        // El canal 10 es la batería: se reparte por pieza.
        if c == 9 {
            for (ins, nombre, tonos) in [
                ("bombo", "Bombo", &[35u8, 36][..]),
                ("caja", "Caja", &[37, 38, 40]),
                ("aplauso", "Aplauso", &[39]),
                ("platillo", "Platillo", &[42, 44, 46, 49, 51, 52, 53, 55, 57, 59]),
            ] {
                let v: Vec<Nota> = notas.iter().filter(|n| tonos.contains(&n.tono)).map(|n| Nota { tono: 60, ..*n }).collect();
                if !v.is_empty() {
                    let canal = p.agregar_canal(ins, t(nombre));
                    p.patrones.last_mut().unwrap().notas.insert(canal, v);
                }
            }
        } else {
            let nombre = nombres.get(&pista).filter(|n| !n.is_empty()).cloned().unwrap_or_else(|| format!("MIDI {}", pista + 1));
            let canal = p.agregar_canal("piano", &nombre);
            p.patrones.last_mut().unwrap().notas.insert(canal, notas);
        }
    }
    let pat = p.patrones.last_mut().unwrap();
    pat.nombre = "MIDI".into();
    pat.compases = pat.largo() / COMPAS;
    Ok(id)
}

/// La canción (todos los clips) como MIDI de varias pistas; sin clips, el patrón que se diga.
pub fn exportar(p: &Proyecto, patron: u32) -> Vec<u8> {
    let var = |v: &mut Vec<u8>, x: u32| {
        for s in [21, 14, 7] {
            if x >> s != 0 {
                v.push((x >> s & 127) as u8 | 128);
            }
        }
        v.push((x & 127) as u8);
    };
    let clips = if p.clips.iter().all(|c| c.audio.is_some()) {
        vec![Clip::de_patron(patron, 0, 0, p.patron(patron).map_or(0, |x| x.largo()))]
    } else {
        p.clips.clone()
    };
    let mut pistas: Vec<Vec<u8>> = vec![];
    let us = (60e6 / p.bpm) as u32;
    let mut t0 = vec![0, 0xFF, 0x51, 3];
    t0.extend_from_slice(&us.to_be_bytes()[1..]);
    t0.extend_from_slice(&[0, 0xFF, 0x2F, 0]);
    pistas.push(t0);
    for (k, c) in p.canales.iter().enumerate() {
        let mut ev: Vec<(u32, u8, u8, u8)> = vec![];
        for cl in &clips {
            let Some(pat) = p.patron(cl.patron) else { continue };
            for (_, rel, n, d) in pat.notas_en(cl.desfase, cl.dur).filter(|x| x.0 == c.id) {
                let t = cl.tick + rel;
                ev.push((t, 1, n.tono, ((n.fuerza * 127.0).round() as u8).clamp(1, 127)));
                ev.push((t + d, 0, n.tono, 0));
            }
        }
        if ev.is_empty() {
            continue;
        }
        // Primero se sueltan, luego se tocan.
        ev.sort();
        let mut v = vec![0, 0xFF, 3];
        let nombre = c.nombre.as_bytes();
        var(&mut v, nombre.len() as u32);
        v.extend_from_slice(nombre);
        // Se salta el canal 10, que es batería.
        let ch = [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15][k % 15];
        let mut antes = 0;
        for (t, on, tono, vel) in ev {
            var(&mut v, t - antes);
            antes = t;
            v.extend_from_slice(&[if on == 1 { 0x90 } else { 0x80 } | ch, tono, vel]);
        }
        v.extend_from_slice(&[0, 0xFF, 0x2F, 0]);
        pistas.push(v);
    }
    let mut b = b"MThd\0\0\0\x06\0\x01".to_vec();
    b.extend_from_slice(&(pistas.len() as u16).to_be_bytes());
    b.extend_from_slice(&(PPQ as u16).to_be_bytes());
    for v in pistas {
        b.extend_from_slice(b"MTrk");
        b.extend_from_slice(&(v.len() as u32).to_be_bytes());
        b.extend_from_slice(&v);
    }
    b
}
