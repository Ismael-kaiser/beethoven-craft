// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Abre proyectos `.flp`: tempo, canales, notas de los patrones y los clips de patrón.
//! Los plugins no se cargan: cada canal recibe el instrumento de Lua que más se le parece.
use crate::idioma::{t, tf};
use crate::modelo::*;
use std::collections::BTreeMap;

#[derive(Default)]
struct CanalF {
    nombre: String,
    plugin: String,
    muestra: String,
}

fn texto(d: &[u8], utf16: bool) -> String {
    let s = if utf16 {
        let v: Vec<u16> = d.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&v)
    } else {
        d.iter().map(|c| *c as char).collect()
    };
    s.trim_end_matches('\0').to_string()
}

fn instrumento(c: &CanalF) -> &'static str {
    if !c.muestra.is_empty() {
        return "muestra";
    }
    let n = format!("{} {}", c.nombre, c.plugin).to_lowercase();
    let tiene = |v: &[&str]| v.iter().any(|x| n.contains(x));
    if tiene(&["kick", "bombo"]) {
        "bombo"
    } else if tiene(&["snare", "caja", "tarola"]) {
        "caja"
    } else if tiene(&["clap", "aplauso"]) {
        "aplauso"
    } else if tiene(&["hat", "cymbal", "ride", "crash", "shaker", "platillo"]) {
        "platillo"
    } else if tiene(&["bass", "bajo", "808", "sub"]) {
        "bajo"
    } else if tiene(&["string", "cuerdas", "violin"]) {
        "orquesta_cuerdas"
    } else if tiene(&["choir", "coro", "vox", "voice"]) {
        "coro"
    } else if tiene(&["pad"]) {
        "colchon"
    } else if tiene(&["piano", "keys", "rhodes", "ep "]) {
        "piano"
    } else if tiene(&["bell", "campana", "pluck"]) {
        "campana"
    } else if tiene(&["organ", "órgano"]) {
        "organo"
    } else if tiene(&["guitar", "guitarra", "harp", "arpa"]) {
        "cuerda"
    } else {
        "clasico"
    }
}

/// El proyecto y lo que no se pudo traer.
pub fn leer(b: &[u8]) -> Result<(Proyecto, Vec<String>), String> {
    let malo = || t("El .flp está cortado o dañado").to_string();
    let u16_ = |d: &[u8], i: usize| u16::from_le_bytes([d[i], d[i + 1]]) as u32;
    let u32_ = |d: &[u8], i: usize| u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]);
    if b.len() < 22 || &b[..4] != b"FLhd" {
        return Err(t("No es un proyecto .flp").into());
    }
    let cab = u32_(b, 4) as usize;
    let i0 = 8usize.checked_add(cab).filter(|i| cab >= 6 && i + 8 <= b.len()).ok_or_else(malo)?;
    let ppq = u16_(b, 12).max(1);
    if &b[i0..i0 + 4] != b"FLdt" {
        return Err(malo());
    }
    let mut d = &b[i0 + 8..];

    let mut version = (0u32, 0u32);
    let mut bpm = None;
    let mut titulo = String::new();
    let mut canales: BTreeMap<u32, CanalF> = BTreeMap::new();
    let mut patrones: BTreeMap<u32, (String, Vec<[u32; 5]>)> = BTreeMap::new();
    let mut playlist: Vec<Vec<u8>> = vec![];
    let (mut canal, mut patron) = (None, 0u32);
    while let Some((&id, resto)) = d.split_first() {
        let (dato, sigue) = match id {
            0..=63 => resto.split_at_checked(1).ok_or_else(malo)?,
            64..=127 => resto.split_at_checked(2).ok_or_else(malo)?,
            128..=191 => resto.split_at_checked(4).ok_or_else(malo)?,
            _ => {
                let (mut n, mut k) = (0usize, 0);
                loop {
                    let x = *resto.get(k).ok_or_else(malo)?;
                    n |= ((x & 127) as usize) << (7 * k);
                    k += 1;
                    if x & 128 == 0 {
                        break;
                    }
                    if k > 4 {
                        return Err(malo());
                    }
                }
                resto[k..].split_at_checked(n).ok_or_else(malo)?
            }
        };
        d = sigue;
        let utf16 = version >= (11, 5);
        match id {
            64 => {
                canal = Some(u16_(dato, 0));
                canales.entry(u16_(dato, 0)).or_default();
            }
            65 => patron = u16_(dato, 0),
            // Tempo entero de los archivos viejos.
            66 if bpm.is_none() => bpm = Some(u16_(dato, 0) as f32),
            156 => bpm = Some(u32_(dato, 0) as f32 / 1000.0),
            199 => {
                let s = texto(dato, false);
                let mut it = s.split('.').map(|x| x.parse().unwrap_or(0));
                version = (it.next().unwrap_or(0), it.next().unwrap_or(0));
            }
            193 => patrones.entry(patron).or_default().0 = texto(dato, utf16),
            194 => titulo = texto(dato, utf16),
            196 | 201 | 203 | 192 => {
                if let Some(c) = canal.and_then(|c| canales.get_mut(&c)) {
                    let t = texto(dato, utf16);
                    match id {
                        196 => c.muestra = t,
                        201 => c.plugin = t,
                        _ => c.nombre = t,
                    }
                }
            }
            224 => {
                let v = &mut patrones.entry(patron).or_default().1;
                for n in dato.chunks_exact(24) {
                    v.push([u32_(n, 0), u16_(n, 6), u32_(n, 8), u16_(n, 12), n[21] as u32]);
                }
            }
            233 => playlist.push(dato.to_vec()),
            // El primer evento de mezcla: lo que sigue ya no habla de canales.
            98 => canal = None,
            _ => {}
        }
    }

    let mut avisos = vec![];
    let mut p = Proyecto { nombre: if titulo.is_empty() { t("Proyecto .flp").into() } else { titulo }, patrones: vec![], ..Default::default() };
    if let Some(x) = bpm {
        p.bpm = x.clamp(20.0, 400.0);
    }
    let escala = |t: u32| (t as u64 * PPQ as u64 / ppq as u64) as u32;
    let mut mapa = BTreeMap::new();
    for (i, c) in &canales {
        let nombre = if !c.nombre.is_empty() {
            c.nombre.clone()
        } else if !c.plugin.is_empty() {
            c.plugin.clone()
        } else {
            tf("Canal {}", &[&(i + 1)])
        };
        let es_wav = c.muestra.to_lowercase().ends_with(".wav") && std::path::Path::new(&c.muestra).exists();
        let sin = CanalF { muestra: if es_wav { c.muestra.clone() } else { String::new() }, nombre: c.nombre.clone(), plugin: c.plugin.clone() };
        let ins = instrumento(&sin);
        let id = p.agregar_canal(ins, &nombre);
        if es_wav {
            p.canales.last_mut().unwrap().muestra = Some(c.muestra.clone());
        } else if !c.muestra.is_empty() {
            avisos.push(tf("«{}»: no se encontró o no es WAV «{}»; suena con «{}»", &[&nombre, &c.muestra, &ins]));
        }
        if !c.plugin.is_empty() {
            avisos.push(tf("«{}»: el plugin «{}» no se carga; suena con «{}»", &[&nombre, &c.plugin, &ins]));
        }
        mapa.insert(*i, id);
    }
    for (id, (nombre, notas)) in patrones.iter().filter(|(_, v)| !v.1.is_empty() || !v.0.is_empty()) {
        let mut pat = Patron {
            id: *id,
            nombre: if nombre.is_empty() { tf("Patrón {}", &[&id]) } else { nombre.clone() },
            color: COLORES[*id as usize % COLORES.len()],
            notas: BTreeMap::new(),
            compases: 1,
        };
        for [pos, ch, dur, tono, vel] in notas {
            let Some(c) = mapa.get(ch) else { continue };
            pat.notas.entry(*c).or_default().push(Nota {
                tick: escala(*pos),
                // Los pasos del secuenciador vienen sin largo.
                dur: if *dur == 0 { PASO } else { escala(*dur).max(1) },
                tono: (*tono).min(127) as u8,
                fuerza: (*vel as f32 / 128.0).clamp(0.01, 1.0),
            });
        }
        p.patrones.push(pat);
    }
    let (mut otros, mut raro) = (0, false);
    for lista in &playlist {
        // El tamaño de cada clip cambió en la versión 21.
        let orden = if version.0 >= 21 { [60, 32] } else { [32, 60] };
        let Some(ancho) = orden.into_iter().find(|a| !lista.is_empty() && lista.len() % a == 0) else {
            raro |= !lista.is_empty();
            continue;
        };
        for x in lista.chunks_exact(ancho) {
            let (pos, base, item, dur, pista) = (u32_(x, 0), u16_(x, 4), u16_(x, 6), u32_(x, 8), u16_(x, 12));
            if item > base && p.patrones.iter().any(|q| q.id == item - base) {
                let ultima = if version.0 >= 20 { 499 } else { 198 };
                let pista = ultima.max(pista) - pista;
                p.clips.push(Clip::de_patron(item - base, (pista as usize).min(PISTAS_ARREGLO - 1), escala(pos), escala(dur).max(1)));
            } else {
                otros += 1;
            }
        }
    }
    if otros > 0 {
        avisos.push(tf("{} clips de audio o automatización no se trajeron", &[&otros]));
    }
    if raro {
        avisos.push(t("El arreglo usa un formato que esta demo no conoce; los patrones sí se trajeron").into());
    }
    if p.patrones.is_empty() {
        p.agregar_patron();
    }
    Ok((p, avisos))
}
