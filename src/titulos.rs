//! Títulos: texto con borde, sombra y fondo.
use crate::curvas::{Curva, Familia, Sentido};
use crate::modelo::{AnimTexto, BucleTexto, MovTexto, PorTexto, Titulo};
use effectcraft_raster::Image;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Carpetas con las letras del sistema, según el sistema.
fn carpetas_de_letras() -> Vec<std::path::PathBuf> {
    let var = |v: &str| std::env::var_os(v).filter(|x| !x.is_empty()).map(std::path::PathBuf::from);
    if cfg!(windows) {
        vec![var("WINDIR").unwrap_or_else(|| "C:\\Windows".into()).join("Fonts")]
    } else if cfg!(target_os = "macos") {
        vec!["/System/Library/Fonts/Supplemental".into(), "/Library/Fonts".into()]
    } else {
        let mut v: Vec<std::path::PathBuf> = vec!["/usr/share/fonts".into(), "/usr/local/share/fonts".into()];
        v.extend(var("HOME").map(|h| h.join(".local/share/fonts")));
        v
    }
}

/// Los .ttf de `dir` y sus subcarpetas (en Linux las letras van por familias).
fn ttf_en(dir: &std::path::Path, hondo: u32, v: &mut Vec<std::path::PathBuf>) {
    for p in std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()) {
        if p.is_dir() && hondo > 0 {
            ttf_en(&p, hondo - 1, v);
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("ttf")) {
            v.push(p);
        }
    }
}

/// Letras disponibles: las dos de la app y las del sistema.
pub fn letras() -> &'static [(String, Option<std::path::PathBuf>)] {
    static L: OnceLock<Vec<(String, Option<std::path::PathBuf>)>> = OnceLock::new();
    L.get_or_init(|| {
        let mut v = vec![("Cuprum".to_string(), None), ("Barlow".to_string(), None)];
        let mut archivos = vec![];
        for d in carpetas_de_letras() {
            ttf_en(&d, 3, &mut archivos);
        }
        let mut sis: Vec<_> = archivos
            .into_iter()
            .filter_map(|p| Some((p.file_stem()?.to_str()?.to_string(), Some(p))))
            .filter(|(n, _)| {
                [
                    // Mac
                    "Arial", "Georgia", "Impact", "Comic", "Courier", "Chalkduster", "Brush", "Bradley", "DIN", "Futura", "Times", "Trebuchet",
                    "Verdana", "Tahoma", "Rockwell", "Luminari", "Herculanum", "Trattatello", "Apple Chancery",
                    // Windows
                    "Segoe", "Calibri", "Cambria", "Candara", "Consola", "Constantia", "Corbel", "Gabriola",
                    // Linux
                    "DejaVu", "Liberation", "NotoSans-", "NotoSerif-", "Ubuntu", "FreeSans", "FreeSerif",
                ]
                .iter()
                .any(|x| n.starts_with(x))
            })
            .collect();
        sis.sort();
        // El mismo nombre en dos carpetas cuenta una vez.
        sis.dedup_by(|a, b| a.0 == b.0);
        v.extend(sis);
        v
    })
}

fn fuente(n: &str) -> Option<&'static fontdue::Font> {
    static C: OnceLock<Mutex<HashMap<String, Option<&'static fontdue::Font>>>> = OnceLock::new();
    let mut c = C.get_or_init(Default::default).lock().unwrap();
    *c.entry(n.to_string()).or_insert_with(|| {
        let b = match n {
            "Barlow" => include_bytes!("../recursos/BarlowSemiCondensed-SemiBold.ttf").to_vec(),
            "Cuprum" => include_bytes!("../recursos/Cuprum.ttf").to_vec(),
            _ => std::fs::read(letras().iter().find(|l| l.0 == n)?.1.as_ref()?).ok()?,
        };
        let f = fontdue::Font::from_bytes(b, Default::default()).ok()?;
        // Se queda cargada: son pocas y se reutilizan en cada cuadro.
        Some(Box::leak(Box::new(f)))
    })
}

pub fn plantilla(n: usize) -> (&'static str, Titulo, f64) {
    let base = Titulo {
        texto: crate::idioma::t("Tu título").into(),
        letra: "Cuprum".into(),
        tam: 11.0,
        color: [1.0; 4],
        borde: 0.0,
        color_borde: [0.0, 0.0, 0.0, 1.0],
        fondo: [0.0; 4],
        sombra: 0.6,
        alinear: 0,
        anim: None,
    };
    match n {
        0 => ("Título al centro", base, 0.0),
        1 => ("Cintillo abajo", Titulo { texto: crate::idioma::t("Artista\nNombre de la canción").into(), tam: 6.0, alinear: -1, fondo: [0.0, 0.0, 0.0, 0.6], ..base }, 32.0),
        2 => ("Letra de la canción", Titulo { texto: crate::idioma::t("Aquí va la letra").into(), tam: 7.0, borde: 0.8, sombra: 0.0, ..base }, 36.0),
        3 => ("Título amarillo", Titulo { letra: "Barlow".into(), tam: 15.0, color: [1.0, 0.878, 0.4, 1.0], borde: 0.6, ..base }, 0.0),
        _ => ("Créditos", Titulo { texto: crate::idioma::t("Hecho por\nTu nombre").into(), tam: 7.0, ..base }, 0.0),
    }
}
pub const PLANTILLAS: usize = 5;

/// Suma un rectángulo difuminado de cobertura: `r` pasadas de caja.
fn difuminar(v: &mut [f32], w: usize, h: usize, r: usize) {
    if r == 0 {
        return;
    }
    let k = 1.0 / (2 * r + 1) as f32;
    let mut tmp = vec![0f32; w.max(h)];
    for _ in 0..2 {
        for y in 0..h {
            let f = &mut v[y * w..(y + 1) * w];
            tmp[..w].copy_from_slice(f);
            let mut s: f32 = (0..=r.min(w - 1)).map(|i| tmp[i]).sum();
            for x in 0..w {
                f[x] = s * k;
                s += if x + r + 1 < w { tmp[x + r + 1] } else { 0.0 } - if x >= r { tmp[x - r] } else { 0.0 };
            }
        }
        for x in 0..w {
            (0..h).for_each(|y| tmp[y] = v[y * w + x]);
            let mut s: f32 = (0..=r.min(h - 1)).map(|i| tmp[i]).sum();
            for y in 0..h {
                v[y * w + x] = s * k;
                s += if y + r + 1 < h { tmp[y + r + 1] } else { 0.0 } - if y >= r { tmp[y - r] } else { 0.0 };
            }
        }
    }
}

/// Dónde y cómo se dibuja un pedazo del texto en un momento.
#[derive(Clone, Copy)]
struct Pose {
    /// Desplazamiento en altos de letra.
    x: f32,
    y: f32,
    ex: f32,
    ey: f32,
    giro: f32,
    alfa: f32,
}

const QUIETO: Pose = Pose { x: 0.0, y: 0.0, ex: 1.0, ey: 1.0, giro: 0.0, alfa: 1.0 };

/// Pose a `k` (0 fuera, 1 en su lugar). Al salir se usa al revés.
fn pose(m: MovTexto, k: f32) -> Pose {
    let cv = |f, s| Curva::Familia(f, s).aplicar(k as f64) as f32;
    let suave = cv(Familia::Cubic, Sentido::Salir);
    let q = QUIETO;
    match m {
        MovTexto::Ninguno => q,
        MovTexto::Maquina => Pose { alfa: if k >= 1.0 { 1.0 } else { 0.0 }, ..q },
        MovTexto::Aparecer | MovTexto::Azar => Pose { alfa: k, ..q },
        MovTexto::Subir => Pose { y: (1.0 - suave) * 0.9, alfa: k, ..q },
        MovTexto::Bajar => Pose { y: (suave - 1.0) * 0.9, alfa: k, ..q },
        MovTexto::Lado => Pose { x: (suave - 1.0) * 1.6, alfa: k, ..q },
        MovTexto::Crecer => {
            let e = cv(Familia::Back, Sentido::Salir).max(0.0);
            Pose { ex: e, ey: e, alfa: (k * 3.0).min(1.0), ..q }
        }
        MovTexto::Encoger => {
            let e = 1.0 + (1.0 - suave) * 2.2;
            Pose { ex: e, ey: e, alfa: k, ..q }
        }
        MovTexto::Girar => Pose { giro: (1.0 - suave) * -std::f32::consts::PI, ex: suave, ey: suave, alfa: k, ..q },
        MovTexto::Voltear => Pose { ex: (1.0 - (1.0 - suave) * 1.0).abs().max(0.02), alfa: (k * 2.0).min(1.0), ..q },
        MovTexto::Rebotar => Pose { y: (cv(Familia::Bounce, Sentido::Salir) - 1.0) * 2.2, alfa: (k * 4.0).min(1.0), ..q },
    }
}

/// Revuelve 0..n siempre igual.
fn revuelto(i: usize, n: usize) -> usize {
    if n < 2 {
        return 0;
    }
    let mut v: Vec<usize> = (0..n).collect();
    let mut s = 0x9E37u32;
    for j in (1..n).rev() {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        v.swap(j, (s >> 8) as usize % (j + 1));
    }
    v[i]
}

/// Pose del pedazo `i` de `n` en el segundo `t` de un clip de `dur` segundos.
fn pose_de(a: &AnimTexto, i: usize, n: usize, t: f64, dur: f64) -> Pose {
    let d = a.dur.max(0.01);
    let avance = |m: MovTexto, t: f64| {
        let j = if m == MovTexto::Azar { revuelto(i, n) } else { i };
        ((t - j as f64 * a.paso) / d).clamp(0.0, 1.0) as f32
    };
    let mut q = QUIETO;
    if a.entrada != MovTexto::Ninguno {
        q = pose(a.entrada, avance(a.entrada, t));
    }
    if a.salida != MovTexto::Ninguno {
        // La salida termina justo al acabar el clip.
        let largo = d + a.paso * n.saturating_sub(1) as f64;
        let k = 1.0 - avance(a.salida, t - (dur - largo));
        if k < 1.0 {
            let s = pose(a.salida, k);
            q = Pose { x: q.x + s.x, y: q.y + s.y, ex: q.ex * s.ex, ey: q.ey * s.ey, giro: q.giro + s.giro, alfa: q.alfa * s.alfa };
        }
    }
    let f = a.fuerza as f32;
    let (tf, fi) = (t as f32, i as f32);
    match a.bucle {
        BucleTexto::Ninguno | BucleTexto::Arcoiris => {}
        BucleTexto::Ola => q.y += 0.18 * f * (tf * 5.0 - fi * 0.6).sin(),
        BucleTexto::Temblor => {
            q.x += 0.04 * f * (tf * 61.0 + fi * 9.0).sin();
            q.y += 0.04 * f * (tf * 73.0 + fi * 5.0).cos();
        }
        BucleTexto::Latido => {
            let e = 1.0 + 0.16 * f * ((tf * 6.3 - fi * 0.25).sin().max(0.0)).powi(3);
            q.ex *= e;
            q.ey *= e;
        }
        BucleTexto::Vaiven => q.giro += 0.16 * f * (tf * 4.0 - fi * 0.5).sin(),
    }
    q
}

pub fn dibujar(t: &Titulo, w: usize, h: usize) -> Image {
    dibujar_en(t, w, h, 0.0, f64::MAX)
}

/// El texto en el segundo `seg` de su clip, que dura `dur`.
pub fn dibujar_en(t: &Titulo, w: usize, h: usize, seg: f64, dur: f64) -> Image {
    let mut img = Image::new(w as u32, h as u32);
    let Some(f) = fuente(&t.letra).or_else(|| fuente("Cuprum")) else { return img };
    let px = (t.tam / 100.0 * h as f32).max(2.0);
    let alto_linea = px * 1.2;
    let lineas: Vec<&str> = t.texto.lines().collect();
    let medir = |s: &str| -> f32 {
        let mut x = 0.0;
        let mut ant = None;
        for c in s.chars() {
            if let Some(a) = ant {
                x += f.horizontal_kern(a, c, px).unwrap_or(0.0);
            }
            x += f.metrics(c, px).advance_width;
            ant = Some(c);
        }
        x
    };
    let anchos: Vec<f32> = lineas.iter().map(|l| medir(l)).collect();
    let mayor = anchos.iter().fold(0f32, |m, x| m.max(*x));
    let margen = w as f32 * 0.06;
    let izq = match t.alinear {
        -1 => margen,
        1 => w as f32 - margen - mayor,
        _ => (w as f32 - mayor) / 2.0,
    };
    let total = alto_linea * lineas.len() as f32;
    let arriba = (h as f32 - total) / 2.0;
    let asc = f.horizontal_line_metrics(px).map_or(px * 0.8, |m| m.ascent);
    let mut cob = vec![0f32; w * h];
    // Cuántos pedazos hay y cuál es cada letra.
    let anim = t.anim.filter(|a| a.entrada != MovTexto::Ninguno || a.salida != MovTexto::Ninguno || a.bucle != BucleTexto::Ninguno);
    let total_piezas = anim.map_or(0, |a| match a.por {
        PorTexto::Letra => t.texto.chars().filter(|c| !c.is_whitespace()).count(),
        PorTexto::Palabra => t.texto.split_whitespace().count(),
        PorTexto::Renglon => lineas.len(),
    });
    let (mut letra, mut palabra, mut en_palabra) = (0usize, 0usize, false);
    // El fondo de un texto animado entra y sale con él.
    let mut visto = 0f32;
    for (i, l) in lineas.iter().enumerate() {
        // Centro de cada palabra o renglón, para que gire y crezca completo.
        en_palabra = false;
        let mut x = match t.alinear {
            -1 => izq,
            1 => izq + mayor - anchos[i],
            _ => (w as f32 - anchos[i]) / 2.0,
        };
        let base = arriba + alto_linea * i as f32 + asc + (alto_linea - px) / 2.0;
        let mut ant = None;
        for c in l.chars() {
            if let Some(a) = ant {
                x += f.horizontal_kern(a, c, px).unwrap_or(0.0);
            }
            if c.is_whitespace() {
                en_palabra = false;
            } else if !en_palabra {
                en_palabra = true;
                palabra += 1;
            }
            if let Some(a) = anim.filter(|_| !c.is_whitespace()) {
                let pieza = match a.por {
                    PorTexto::Letra => letra,
                    PorTexto::Palabra => palabra - 1,
                    PorTexto::Renglon => i,
                };
                letra += 1;
                let q = pose_de(&a, pieza, total_piezas, seg, dur);
                visto = visto.max(q.alfa);
                if q.alfa > 0.0 && q.ex > 0.0 && q.ey > 0.0 {
                    let (m, b) = f.rasterize(c, px);
                    letra_movida(&mut cob, w, h, &b, m.width, m.height, [x + m.xmin as f32, base - m.height as f32 - m.ymin as f32], [x + m.advance_width / 2.0, base - px * 0.35], q, px);
                }
                x += f.metrics(c, px).advance_width;
                ant = Some(c);
                continue;
            }
            let (m, b) = f.rasterize(c, px);
            let (gx, gy) = ((x + m.xmin as f32).round() as i64, (base - m.height as f32 - m.ymin as f32).round() as i64);
            for yy in 0..m.height {
                for xx in 0..m.width {
                    let (px_, py_) = (gx + xx as i64, gy + yy as i64);
                    if px_ >= 0 && py_ >= 0 && (px_ as usize) < w && (py_ as usize) < h {
                        let d = &mut cob[py_ as usize * w + px_ as usize];
                        *d = d.max(b[yy * m.width + xx] as f32 / 255.0);
                    }
                }
            }
            x += m.advance_width;
            ant = Some(c);
        }
    }
    let mut poner = |cob: &(dyn Fn(usize, usize) -> f32 + Sync), col: [f32; 4]| {
        img.data.par_chunks_mut(w.max(1)).enumerate().for_each(|(y, fila)| {
            for (x, d) in fila.iter_mut().enumerate() {
                let a = cob(x, y) * col[3];
                if a > 0.0 {
                    let k = 1.0 - a;
                    *d = [col[0] * a + d[0] * k, col[1] * a + d[1] * k, col[2] * a + d[2] * k, a + d[3] * k];
                }
            }
        });
    };
    let _ = en_palabra;
    let fondo = if anim.is_some() { [t.fondo[0], t.fondo[1], t.fondo[2], t.fondo[3] * visto] } else { t.fondo };
    if fondo[3] > 0.0 {
        let p = px * 0.35;
        let (x0, x1, y0, y1) = (izq - p, izq + mayor + p, arriba - p * 0.6, arriba + total + p * 0.6);
        poner(&|x, y| f32::from(x as f32 >= x0 && (x as f32) < x1 && y as f32 >= y0 && (y as f32) < y1), fondo);
    }
    if t.sombra > 0.0 {
        let mut s = cob.clone();
        difuminar(&mut s, w, h, (px * 0.06).ceil() as usize);
        let d = (px * 0.05).round() as usize;
        poner(&|x, y| if x >= d && y >= d { s[(y - d) * w + x - d] } else { 0.0 }, [0.0, 0.0, 0.0, t.sombra]);
    }
    if t.borde > 0.0 {
        // Borde: lo más cercano a la letra dentro de un radio.
        let r = (t.borde / 100.0 * h as f32).max(1.0);
        let n = r.ceil() as i64;
        let disco: Vec<(i64, i64)> = (-n..=n).flat_map(|y| (-n..=n).map(move |x| (x, y))).filter(|(x, y)| ((x * x + y * y) as f32).sqrt() <= r + 0.5).collect();
        let mut o = vec![0f32; w * h];
        for y in 0..h {
            for x in 0..w {
                let c = cob[y * w + x];
                if c > 0.02 {
                    for (dx, dy) in &disco {
                        let (a, b) = (x as i64 + dx, y as i64 + dy);
                        if a >= 0 && b >= 0 && (a as usize) < w && (b as usize) < h {
                            let q = &mut o[b as usize * w + a as usize];
                            *q = q.max(c);
                        }
                    }
                }
            }
        }
        poner(&|x, y| o[y * w + x], t.color_borde);
    }
    poner(&|x, y| cob[y * w + x], t.color);
    if anim.is_some_and(|a| a.bucle == BucleTexto::Arcoiris) {
        for y in 0..h {
            for x in 0..w {
                let a = cob[y * w + x] * t.color[3];
                if a > 0.0 {
                    let hue = x as f32 / w as f32 * 2.0 - seg as f32 * 0.5;
                    let c = |d: f32| ((hue + d).rem_euclid(1.0) * 6.0 - 3.0).abs().clamp(1.0, 2.0) - 1.0;
                    let col = [c(0.0) * 0.8 + 0.2, c(2.0 / 3.0) * 0.8 + 0.2, c(1.0 / 3.0) * 0.8 + 0.2];
                    let d = &mut img.data[y * w + x];
                    (0..3).for_each(|k| d[k] = col[k] * a + d[k] * (1.0 - a));
                }
            }
        }
    }
    img
}

/// Estampa una letra movida, girada o estirada alrededor de `centro`.
#[allow(clippy::too_many_arguments)]
fn letra_movida(cob: &mut [f32], w: usize, h: usize, b: &[u8], bw: usize, bh: usize, esq: [f32; 2], centro: [f32; 2], q: Pose, px: f32) {
    if bw == 0 || bh == 0 {
        return;
    }
    let (s, c) = q.giro.sin_cos();
    let (ox, oy) = (centro[0] + q.x * px, centro[1] + q.y * px);
    let (rx, ry) = (centro[0] - esq[0], centro[1] - esq[1]);
    // Caja que la contiene ya transformada.
    let r = [(0.0, 0.0), (bw as f32, 0.0), (0.0, bh as f32), (bw as f32, bh as f32)].map(|(x, y)| {
        let (dx, dy) = ((x - rx) * q.ex, (y - ry) * q.ey);
        (ox + dx * c - dy * s, oy + dx * s + dy * c)
    });
    let min = |k: fn(&(f32, f32)) -> f32| r.iter().map(k).fold(f32::MAX, f32::min);
    let max = |k: fn(&(f32, f32)) -> f32| r.iter().map(k).fold(f32::MIN, f32::max);
    let (x0, x1) = ((min(|p| p.0).floor() - 1.0).max(0.0) as usize, (max(|p| p.0).ceil() + 1.0).clamp(0.0, w as f32) as usize);
    let (y0, y1) = ((min(|p| p.1).floor() - 1.0).max(0.0) as usize, (max(|p| p.1).ceil() + 1.0).clamp(0.0, h as f32) as usize);
    let tex = |x: i32, y: i32| if x < 0 || y < 0 || x >= bw as i32 || y >= bh as i32 { 0.0 } else { b[y as usize * bw + x as usize] as f32 / 255.0 };
    for y in y0..y1 {
        for x in x0..x1 {
            let (dx, dy) = (x as f32 + 0.5 - ox, y as f32 + 0.5 - oy);
            let u = (dx * c + dy * s) / q.ex + rx - 0.5;
            let v = (-dx * s + dy * c) / q.ey + ry - 0.5;
            let (i, j) = (u.floor(), v.floor());
            let (fu, fv) = (u - i, v - j);
            let (i, j) = (i as i32, j as i32);
            let k = (tex(i, j) * (1.0 - fu) + tex(i + 1, j) * fu) * (1.0 - fv) + (tex(i, j + 1) * (1.0 - fu) + tex(i + 1, j + 1) * fu) * fv;
            if k > 0.0 {
                let d = &mut cob[y * w + x];
                *d = d.max(k * q.alfa);
            }
        }
    }
}
