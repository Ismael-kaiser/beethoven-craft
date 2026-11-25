// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Instrumentos grabados nota por nota: SoundFont (.sf2), SFZ y WAV sueltos.
use crate::idioma::t;
use std::collections::HashMap;
use std::path::Path;

/// Grabaciones que puede juntar una sola nota (estéreo y capas).
pub const CAPAS: usize = 8;

#[derive(Debug, Clone)]
pub struct Zona {
    pub tonos: (u8, u8),
    pub fuerzas: (u8, u8),
    /// Nota en la que suena tal cual, con decimales.
    pub raiz: f32,
    pub ini: usize,
    pub fin: usize,
    pub bucle: Option<(usize, usize)>,
    pub sr: f32,
    /// Izquierda y derecha.
    pub gan: [f32; 2],
}

#[derive(Debug, Default)]
pub struct Muestra {
    pub zonas: Vec<Zona>,
    pub datos: Vec<f32>,
    /// Cuánto tarda en apagarse al soltar la tecla, según el SoundFont (la mediana de sus zonas).
    pub soltar: Option<f32>,
    /// La huella de un lugar, para los efectos que hacen convolución.
    pub impulso: Option<Box<crate::impulso::Impulso>>,
    /// Dónde empieza cada corte (0 a 1), si está cortada.
    pub cortes: Vec<f32>,
}

impl Muestra {
    /// Las zonas que tocan esa nota.
    pub fn elegir(&self, tono: u8, fuerza: f32) -> ([u16; CAPAS], u8) {
        let f = (fuerza * 127.0).round().clamp(1.0, 127.0) as u8;
        let (mut v, mut n) = ([0; CAPAS], 0);
        for (i, z) in self.zonas.iter().enumerate() {
            if (z.tonos.0..=z.tonos.1).contains(&tono) && (z.fuerzas.0..=z.fuerzas.1).contains(&f) && n < CAPAS {
                v[n] = i as u16;
                n += 1;
            }
        }
        (v, n as u8)
    }

    /// De un WAV: izquierda y derecha por separado, en toda la extensión.
    pub fn de_canales(canales: [Vec<f32>; 2], sr: f32) -> Muestra {
        let mut m = Muestra::default();
        for (c, gan) in canales.into_iter().zip([[1.0, 0.0], [0.0, 1.0]]) {
            m.agregar(&c, sr, gan);
        }
        m
    }

    /// Agrega otra muestra para que suene tal cual en una sola tecla.
    pub fn agregar_en(&mut self, x: &Muestra, tecla: u8) {
        for z in x.zonas.iter().filter(|z| (z.tonos.0..=z.tonos.1).contains(&60)) {
            let n = self.agregar(&x.datos[z.ini..z.fin], z.sr, z.gan);
            (n.tonos, n.raiz) = ((tecla, tecla), tecla as f32 + z.raiz - 60.0);
        }
    }

    /// Agrega otra muestra para esas teclas y fuerzas; en `nota` suena tal cual.
    pub fn agregar_entre(&mut self, x: &Muestra, nota: u8, teclas: (u8, u8), fuerzas: (u8, u8)) {
        for z in x.zonas.iter().filter(|z| (z.tonos.0..=z.tonos.1).contains(&60)) {
            let n = self.agregar(&x.datos[z.ini..z.fin], z.sr, z.gan);
            (n.tonos, n.fuerzas, n.raiz) = (teclas, fuerzas, nota as f32 + z.raiz - 60.0);
        }
    }

    /// En partes, desde la tecla 48. `golpes`: donde sube el sonido de repente; `fino` (0 a 1), qué tanto se buscan.
    pub fn cortada(&self, partes: usize, golpes: bool, fino: f32) -> Muestra {
        let mut m = Muestra::default();
        let Some(largo) = self.zonas.iter().map(|z| z.fin - z.ini).max().filter(|n| *n > 64) else { return m };
        let mut ini: Vec<usize> = (0..partes).map(|k| k * largo / partes).collect();
        if golpes {
            let z = &self.zonas[0];
            let x = &self.datos[z.ini..z.fin];
            let paso = ((z.sr * 0.005) as usize).max(1);
            let nivel: Vec<f32> = x.chunks(paso).map(|c| (c.iter().map(|v| v * v).sum::<f32>() / c.len() as f32).sqrt()).collect();
            let alto = nivel.iter().cloned().fold(1e-9, f32::max);
            let umbral = 1.15 + (1.0 - fino) * 2.5;
            ini = vec![0];
            for k in 2..nivel.len() {
                let antes = nivel[k.saturating_sub(6)..k].iter().sum::<f32>() / nivel[k.saturating_sub(6)..k].len() as f32;
                let en = (k * paso).saturating_sub(paso / 2);
                // Separados al menos 60 ms.
                if nivel[k] > alto * 0.06 && nivel[k] > antes * umbral && en > ini[ini.len() - 1] + (z.sr * 0.06) as usize && ini.len() < 32 {
                    ini.push(en);
                }
            }
        }
        for (k, a) in ini.iter().enumerate() {
            let b = ini.get(k + 1).copied().unwrap_or(largo);
            for z in &self.zonas {
                let (a, b) = ((z.ini + a).min(z.fin), (z.ini + b).min(z.fin));
                let mut x = self.datos[a..b].to_vec();
                // Que no truene al cortar.
                let n = x.len();
                for q in 0..(z.sr * 0.002) as usize {
                    if q < n {
                        x[n - 1 - q] *= q as f32 / (z.sr * 0.002);
                    }
                }
                let w = m.agregar(&x, z.sr, z.gan);
                let tecla = 48 + k as u8;
                (w.tonos, w.raiz) = ((tecla, tecla), tecla as f32 + z.raiz - 60.0);
            }
            m.cortes.push(*a as f32 / largo as f32);
        }
        m
    }

    /// Cuánto dura, en segundos.
    pub fn segundos(&self) -> f32 {
        self.zonas.iter().map(|z| (z.fin - z.ini) as f32 / z.sr).fold(0.0, f32::max)
    }

    fn agregar(&mut self, x: &[f32], sr: f32, gan: [f32; 2]) -> &mut Zona {
        let ini = self.datos.len();
        self.datos.extend_from_slice(x);
        self.zonas.push(Zona { tonos: (0, 127), fuerzas: (0, 127), raiz: 60.0, ini, fin: self.datos.len(), bucle: None, sr, gan });
        self.zonas.last_mut().unwrap()
    }
}

fn db(x: f32) -> f32 {
    10f32.powf(x / 20.0)
}

/// Abre cualquiera de los tres. `preset` solo cuenta en los SoundFont.
pub fn abrir(ruta: &Path, preset: usize) -> Result<Muestra, String> {
    match ruta.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref() {
        Some("sf2" | "sf3") => sf2(&std::fs::read(ruta).map_err(|e| e.to_string())?, preset),
        Some("sfz") => sfz(ruta),
        _ => {
            let w = crate::decodificar::leer(ruta)?;
            Ok(Muestra::de_canales([w.izq, w.der], w.sr))
        }
    }
}

// ---- SoundFont 2 ----

struct Sf2<'a> {
    smpl: &'a [u8],
    trozos: HashMap<[u8; 4], &'a [u8]>,
}

fn u16_(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}
fn u32_(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

impl<'a> Sf2<'a> {
    fn nuevo(b: &'a [u8]) -> Option<Sf2<'a>> {
        if b.get(..4)? != b"RIFF" || b.get(8..12)? != b"sfbk" {
            return None;
        }
        let mut s = Sf2 { smpl: &[], trozos: HashMap::new() };
        let mut i = 12;
        while i + 12 <= b.len() {
            let n = u32_(b, i + 4)? as usize;
            let lista = &b[i + 8..(i + 8 + n).min(b.len())];
            if &b[i..i + 4] == b"LIST" {
                let mut k = 4;
                while k + 8 <= lista.len() {
                    let n = u32_(lista, k + 4)? as usize;
                    let d = &lista[k + 8..(k + 8 + n).min(lista.len())];
                    let id: [u8; 4] = lista[k..k + 4].try_into().ok()?;
                    if &id == b"smpl" {
                        s.smpl = d;
                    } else {
                        s.trozos.insert(id, d);
                    }
                    k += 8 + n + (n & 1);
                }
            }
            i += 8 + n + (n & 1);
        }
        Some(s)
    }

    fn fila(&self, id: &[u8; 4], i: usize, ancho: usize) -> Option<&'a [u8]> {
        self.trozos.get(id)?.get(i * ancho..(i + 1) * ancho)
    }

    /// Los generadores de cada zona de un preset o instrumento.
    fn zonas(&self, bag: &[u8; 4], gen_: &[u8; 4], desde: usize, hasta: usize) -> Option<Vec<HashMap<u16, u16>>> {
        let mut v = vec![];
        for z in desde..hasta {
            let (a, b) = (u16_(self.fila(bag, z, 4)?, 0)? as usize, u16_(self.fila(bag, z + 1, 4)?, 0)? as usize);
            let mut m = HashMap::new();
            for g in a..b {
                let f = self.fila(gen_, g, 4)?;
                m.insert(u16_(f, 0)?, u16_(f, 2)?);
            }
            v.push(m);
        }
        Some(v)
    }

    fn presets(&self) -> Vec<(String, usize, usize)> {
        let n = (self.trozos.get(b"phdr").map_or(0, |d| d.len()) / 38).saturating_sub(1);
        let mut v: Vec<(u16, u16, String, usize, usize)> = (0..n)
            .filter_map(|i| {
                let (f, g) = (self.fila(b"phdr", i, 38)?, self.fila(b"phdr", i + 1, 38)?);
                let nombre: String = f[..20].iter().take_while(|c| **c != 0).map(|c| *c as char).collect();
                Some((u16_(f, 22)?, u16_(f, 20)?, nombre.trim().to_string(), u16_(f, 24)? as usize, u16_(g, 24)? as usize))
            })
            .collect();
        v.sort();
        v.into_iter().map(|(_, _, n, a, b)| (n, a, b)).collect()
    }

    fn leer(&self, preset: usize) -> Option<Muestra> {
        let presets = self.presets();
        let (_, a, b) = presets.get(preset).or(presets.first())?;
        let mut m = Muestra::default();
        let mut ya: HashMap<(usize, usize, usize), usize> = HashMap::new();
        // En un .sf3 cada grabación viene aparte en Ogg Vorbis: se abre una sola vez.
        let mut abiertas: HashMap<u16, Option<Vec<f32>>> = HashMap::new();
        let mut sueltas = vec![];
        let rango = |g: &HashMap<u16, u16>, k: u16| g.get(&k).map_or((0, 127), |v| ((v & 255) as u8, (v >> 8) as u8));
        let cruce = |a: (u8, u8), b: (u8, u8)| (a.0.max(b.0), a.1.min(b.1));
        let mut gp = HashMap::new();
        for (i, zp) in self.zonas(b"pbag", b"pgen", *a, *b)?.into_iter().enumerate() {
            let Some(ins) = zp.get(&41).copied() else {
                if i == 0 {
                    gp = zp;
                }
                continue;
            };
            let mut p = gp.clone();
            p.extend(zp);
            let ia = u16_(self.fila(b"inst", ins as usize, 22)?, 20)? as usize;
            let ib = u16_(self.fila(b"inst", ins as usize + 1, 22)?, 20)? as usize;
            let mut gi = HashMap::new();
            for (j, zi) in self.zonas(b"ibag", b"igen", ia, ib)?.into_iter().enumerate() {
                let Some(sid) = zi.get(&53).copied() else {
                    if j == 0 {
                        gi = zi;
                    }
                    continue;
                };
                let mut g: HashMap<u16, u16> = gi.clone();
                g.extend(zi);
                let tonos = cruce(rango(&g, 43), rango(&p, 43));
                let fuerzas = cruce(rango(&g, 44), rango(&p, 44));
                if tonos.0 > tonos.1 || fuerzas.0 > fuerzas.1 {
                    continue;
                }
                let h = self.fila(b"shdr", sid as usize, 46)?;
                // Las muestras en ROM no vienen en el archivo.
                if u16_(h, 44)? & 0x8000 != 0 {
                    continue;
                }
                let ii = |k: u16| g.get(&k).map_or(0, |v| *v as i16 as i64);
                let suma = |k: u16| ii(k) + p.get(&k).map_or(0, |v| *v as i16 as i64);
                let comprimida = u16_(h, 44)? & 0x10 != 0;
                let abierta = if comprimida {
                    let x = abiertas.entry(sid).or_insert_with(|| {
                        let ogg = self.smpl.get(u32_(h, 20)? as usize..u32_(h, 24)? as usize)?;
                        crate::decodificar::de_bytes(ogg.to_vec(), "ogg").ok().map(|w| w.izq)
                    });
                    let Some(x) = x.as_ref() else { continue };
                    Some(x)
                } else {
                    None
                };
                // En un .sf3 el inicio y el fin son de su propia grabación, y el bucle cuenta desde su inicio.
                let (total, a0, b0) = match abierta {
                    Some(x) => (x.len() as i64, 0, x.len() as i64),
                    None => ((self.smpl.len() / 2) as i64, u32_(h, 20)? as i64, u32_(h, 24)? as i64),
                };
                let lim = |x: i64| x.clamp(0, total) as usize;
                let ini = lim(a0 + ii(0) + ii(4) * 32768);
                let fin = lim(b0 + ii(1) + ii(12) * 32768);
                let la = lim(u32_(h, 28)? as i64 + ii(2) + ii(45) * 32768);
                let lb = lim(u32_(h, 32)? as i64 + ii(3) + ii(50) * 32768);
                if fin <= ini + 2 {
                    continue;
                }
                let llave = (if comprimida { sid as usize + 1 } else { 0 }, ini, fin);
                let base = *ya.entry(llave).or_insert_with(|| {
                    let k = m.datos.len();
                    match abierta {
                        Some(x) => m.datos.extend_from_slice(&x[ini..fin]),
                        None => {
                            m.datos.extend(self.smpl[ini * 2..fin * 2].chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0))
                        }
                    }
                    k
                });
                // Tiempo de soltar (generador 38) en timecents.
                sueltas.push(2f32.powf(suma(38).max(-12000) as f32 / 1200.0));
                let raiz = match g.get(&58).map(|v| *v as i16) {
                    Some(r) if r >= 0 => r as f32,
                    _ => h[40].min(127) as f32,
                };
                let afina = suma(51) as f32 + (suma(52) as f32 + h[41] as i8 as f32) / 100.0;
                let pan = (suma(17) as f32 / 500.0).clamp(-1.0, 1.0);
                // Los sintetizadores de referencia aplican 0.4 de la atenuación pedida.
                let nivel = db(-(suma(48).max(0) as f32) * 0.04);
                let modo = ii(54) & 3;
                m.zonas.push(Zona {
                    tonos,
                    fuerzas,
                    raiz: raiz - afina,
                    ini: base,
                    fin: base + fin - ini,
                    bucle: (modo & 1 == 1 && la >= ini && lb <= fin && lb > la + 8).then(|| (base + la - ini, base + lb - ini)),
                    sr: (u32_(h, 36)? as f32).max(1000.0),
                    gan: [nivel * (1.0 - pan.max(0.0)), nivel * (1.0 + pan.min(0.0))],
                });
            }
        }
        sueltas.sort_by(f32::total_cmp);
        m.soltar = sueltas.get(sueltas.len() / 2).copied();
        (!m.zonas.is_empty()).then_some(m)
    }
}

pub fn presets(ruta: &Path) -> Vec<String> {
    let Ok(b) = std::fs::read(ruta) else { return vec![] };
    Sf2::nuevo(&b).map_or(vec![], |s| s.presets().into_iter().map(|x| x.0).collect())
}

pub fn sf2(b: &[u8], preset: usize) -> Result<Muestra, String> {
    Sf2::nuevo(b).and_then(|s| s.leer(preset)).ok_or_else(|| t("el SoundFont está vacío, dañado o viene comprimido").into())
}

// ---- SFZ ----

fn nota(s: &str) -> Option<i32> {
    if let Ok(n) = s.parse() {
        return Some(n);
    }
    let mut c = s.chars();
    let base = match c.next()?.to_ascii_lowercase() {
        'c' => 0,
        'd' => 2,
        'e' => 4,
        'f' => 5,
        'g' => 7,
        'a' => 9,
        'b' => 11,
        _ => return None,
    };
    let r = c.as_str();
    let (alt, r) = if let Some(r) = r.strip_prefix('#') {
        (1, r)
    } else if let Some(r) = r.strip_prefix('b') {
        (-1, r)
    } else {
        (0, r)
    };
    Some((r.parse::<i32>().ok()? + 1) * 12 + base + alt)
}

/// Los pares `clave=valor` de un renglón; el valor puede llevar espacios.
fn opciones(l: &str) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = vec![];
    for p in l.split_whitespace() {
        match p.split_once('=') {
            Some((k, x)) => v.push((k.to_lowercase(), x.into())),
            None => {
                if let Some(u) = v.last_mut() {
                    u.1.push(' ');
                    u.1.push_str(p);
                }
            }
        }
    }
    v
}

pub fn sfz(ruta: &Path) -> Result<Muestra, String> {
    let crudo = std::fs::read(ruta).map_err(|e| e.to_string())?;
    let texto = String::from_utf8_lossy(&crudo);
    let dir = ruta.parent().unwrap_or(Path::new("."));
    type Op = HashMap<String, String>;
    // control, global, master, group, region
    let mut nivel: [Op; 5] = Default::default();
    let mut en = 1usize;
    let mut regiones: Vec<Op> = vec![];
    let cerrar = |nivel: &mut [Op; 5], en: usize, regiones: &mut Vec<Op>| {
        if en == 4 {
            let mut o = Op::new();
            nivel.iter().for_each(|n| o.extend(n.clone()));
            regiones.push(o);
        }
    };
    for l in texto.lines() {
        let mut l = l.split("//").next().unwrap_or("");
        loop {
            let (antes, cab) = match l.find('<') {
                Some(i) => match l[i..].find('>') {
                    Some(j) => (&l[..i], Some((&l[i + 1..i + j], &l[i + j + 1..]))),
                    None => (l, None),
                },
                None => (l, None),
            };
            nivel[en].extend(opciones(antes));
            let Some((nombre, sigue)) = cab else { break };
            cerrar(&mut nivel, en, &mut regiones);
            en = match nombre {
                "control" => 0,
                "global" => 1,
                "master" => 2,
                "group" => 3,
                _ => 4,
            };
            nivel[en..].iter_mut().for_each(|n| n.clear());
            if en == 4 && nombre != "region" {
                // Curvas y efectos: no son regiones.
                nivel[4].insert("trigger".into(), "nada".into());
            }
            l = sigue;
        }
    }
    cerrar(&mut nivel, en, &mut regiones);
    let mut m = Muestra::default();
    let mut leidas: HashMap<String, (usize, usize, usize, f32)> = HashMap::new();
    let mut falla = None;
    for o in regiones {
        let Some(s) = o.get("sample") else { continue };
        if o.get("trigger").is_some_and(|t| t != "attack") {
            continue;
        }
        let ruta = format!("{}{}", o.get("default_path").map_or("", |s| s), s).replace('\\', "/");
        if !leidas.contains_key(&ruta) {
            match crate::decodificar::leer(&dir.join(&ruta)) {
                Ok(w) => {
                    let a = m.datos.len();
                    m.datos.extend(&w.izq);
                    let b = m.datos.len();
                    m.datos.extend(&w.der);
                    leidas.insert(ruta.clone(), (a, b, w.izq.len(), w.sr));
                }
                Err(e) => {
                    falla = Some(format!("{ruta}: {e}"));
                    continue;
                }
            }
        }
        let (a, b, n, sr) = leidas[&ruta];
        let num = |k: &str| o.get(k).and_then(|v| v.trim().parse::<f32>().ok());
        let tono = |k: &str| o.get(k).and_then(|v| nota(v.trim())).map(|n| n.clamp(-1, 127));
        let key = tono("key");
        let (lo, hi) = (tono("lokey").or(key).unwrap_or(0), tono("hikey").or(key).unwrap_or(127));
        // hikey=-1 es «no con el teclado».
        if hi < 0 || n < 3 {
            continue;
        }
        let raiz = tono("pitch_keycenter").or(key).unwrap_or(60) as f32;
        let raiz = raiz - num("transpose").unwrap_or(0.0) - num("tune").unwrap_or(0.0) / 100.0;
        let nivel = db(num("volume").unwrap_or(0.0).clamp(-100.0, 24.0));
        let pan = (num("pan").unwrap_or(0.0) / 100.0).clamp(-1.0, 1.0);
        let con_bucle = o.get("loop_mode").is_some_and(|l| l.starts_with("loop"));
        let lazo = (num("loop_start").unwrap_or(0.0).max(0.0) as usize, (num("loop_end").map_or(n, |x| x as usize + 1)).min(n));
        for (ini, gan) in [(a, [nivel * (1.0 - pan.max(0.0)), 0.0]), (b, [0.0, nivel * (1.0 + pan.min(0.0))])] {
            m.zonas.push(Zona {
                tonos: (lo.max(0) as u8, hi as u8),
                fuerzas: (num("lovel").unwrap_or(0.0) as u8, num("hivel").unwrap_or(127.0) as u8),
                raiz,
                ini,
                fin: ini + n,
                bucle: (con_bucle && lazo.1 > lazo.0 + 8).then_some((ini + lazo.0, ini + lazo.1)),
                sr,
                gan,
            });
        }
    }
    if m.zonas.is_empty() { Err(falla.unwrap_or_else(|| t("el SFZ no tiene sonidos").into())) } else { Ok(m) }
}
