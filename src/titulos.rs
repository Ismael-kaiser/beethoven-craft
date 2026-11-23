//! Títulos: texto con borde, sombra y fondo.
use crate::modelo::Titulo;
use effectcraft_raster::Image;
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

pub fn dibujar(t: &Titulo, w: usize, h: usize) -> Image {
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
    for (i, l) in lineas.iter().enumerate() {
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
    let mut poner = |cob: &dyn Fn(usize, usize) -> f32, col: [f32; 4]| {
        for y in 0..h {
            for x in 0..w {
                let a = cob(x, y) * col[3];
                if a > 0.0 {
                    let d = &mut img.data[y * w + x];
                    let k = 1.0 - a;
                    *d = [col[0] * a + d[0] * k, col[1] * a + d[1] * k, col[2] * a + d[2] * k, a + d[3] * k];
                }
            }
        }
    };
    if t.fondo[3] > 0.0 {
        let p = px * 0.35;
        let (x0, x1, y0, y1) = (izq - p, izq + mayor + p, arriba - p * 0.6, arriba + total + p * 0.6);
        poner(&|x, y| f32::from(x as f32 >= x0 && (x as f32) < x1 && y as f32 >= y0 && (y as f32) < y1), t.fondo);
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
    img
}
