// Beethoven Craft. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Los íconos, dibujados para Beethoven Craft en una rejilla de 24: `M` mover, `L` línea, `C` curva, `Z` cerrar y
//! `O x y radio` círculo.
use eframe::egui::{Color32, Painter, Pos2, Shape, Stroke, epaint::PathStroke};

pub struct Icono {
    pub relleno: bool,
    pub d: &'static str,
}

impl Icono {
    /// `en` lleva un punto de la rejilla a la pantalla; `k` es el tamaño de un cuadro de la rejilla.
    pub fn pintar(&self, p: &Painter, en: impl Fn(f32, f32) -> Pos2, grueso: f32, col: Color32) {
        let k = en(1.0, 0.0).x - en(0.0, 0.0).x;
        let mut linea: Vec<Pos2> = vec![];
        let suelta = |v: &mut Vec<Pos2>, cerrada: bool| {
            if v.len() > 1 {
                let v = std::mem::take(v);
                if self.relleno {
                    p.add(Shape::convex_polygon(v, col, Stroke::new(grueso * 0.6, col)));
                } else if cerrada {
                    p.add(Shape::closed_line(v, PathStroke::new(grueso, col)));
                } else {
                    p.add(Shape::line(v, PathStroke::new(grueso, col)));
                }
            }
            v.clear();
        };
        let mut t = self.d.split_ascii_whitespace();
        while let Some(orden) = t.next() {
            let mut n = || t.next().and_then(|x| x.parse::<f32>().ok()).unwrap_or(0.0);
            match orden {
                "M" => {
                    suelta(&mut linea, false);
                    linea.push(en(n(), n()));
                }
                "L" => linea.push(en(n(), n())),
                "C" => {
                    let (a, b, c) = (en(n(), n()), en(n(), n()), en(n(), n()));
                    let o = linea.last().copied().unwrap_or(a);
                    for i in 1..=10 {
                        let s = i as f32 / 10.0;
                        let (u, v) = (1.0 - s, s);
                        let q = o.to_vec2() * (u * u * u) + a.to_vec2() * (3.0 * u * u * v) + b.to_vec2() * (3.0 * u * v * v) + c.to_vec2() * (v * v * v);
                        linea.push(q.to_pos2());
                    }
                }
                "Z" => {
                    if linea.len() > 2 && linea[0].distance(linea[linea.len() - 1]) < 0.1 {
                        linea.pop();
                    }
                    suelta(&mut linea, true);
                }
                "O" => {
                    suelta(&mut linea, false);
                    let (o, r) = (en(n(), n()), n() * k);
                    // Un círculo muy chico es un punto.
                    if self.relleno || r < grueso {
                        p.circle_filled(o, r.max(grueso * 0.7), col);
                    } else {
                        p.circle_stroke(o, r, Stroke::new(grueso, col));
                    }
                }
                _ => {}
            }
        }
        suelta(&mut linea, false);
    }
}

pub const PLUS: Icono = Icono { relleno: false, d: "M 12 5 L 12 19 M 5 12 L 19 12" };
pub const MINUS: Icono = Icono { relleno: false, d: "M 5 12 L 19 12" };
pub const X: Icono = Icono { relleno: false, d: "M 6 6 L 18 18 M 18 6 L 6 18" };
pub const TRASH: Icono = Icono { relleno: false, d: "M 4 7 L 20 7 M 9 7 L 9 4 L 15 4 L 15 7 M 6 7 L 7 20 L 17 20 L 18 7 M 10 11 L 10 16 M 14 11 L 14 16" };
pub const FILE: Icono = Icono { relleno: false, d: "M 6 3 L 14 3 L 19 8 L 19 20 L 18 21 L 6 21 L 5 20 L 5 4 L 6 3 M 14 3 L 14 8 L 19 8" };
pub const FILE_TEXT: Icono = Icono { relleno: false, d: "M 6 3 L 14 3 L 19 8 L 19 20 L 18 21 L 6 21 L 5 20 L 5 4 L 6 3 M 14 3 L 14 8 L 19 8 M 9 12 L 15 12 M 9 16 L 15 16" };
pub const FILE_MUSIC: Icono = Icono { relleno: false, d: "M 6 3 L 14 3 L 19 8 L 19 20 L 18 21 L 6 21 L 5 20 L 5 4 L 6 3 M 14 3 L 14 8 L 19 8 O 10 16.5 1.8 M 11.8 16.5 L 11.8 11 L 15 12.5" };
pub const FILE_STAR: Icono = Icono { relleno: false, d: "M 6 3 L 14 3 L 19 8 L 19 20 L 18 21 L 6 21 L 5 20 L 5 4 L 6 3 M 14 3 L 14 8 L 19 8 M 12.0 10.5 L 13.0 13.12 L 15.8 13.26 L 13.62 15.03 L 14.35 17.74 L 12.0 16.2 L 9.65 17.74 L 10.38 15.03 L 8.2 13.26 L 11.0 13.12 Z" };
pub const FILE_TYPOGRAPHY: Icono = Icono { relleno: false, d: "M 6 3 L 14 3 L 19 8 L 19 20 L 18 21 L 6 21 L 5 20 L 5 4 L 6 3 M 14 3 L 14 8 L 19 8 M 9 11 L 15 11 M 12 11 L 12 17" };
pub const FOLDER: Icono = Icono { relleno: false, d: "M 3 6 L 4 5 L 9 5 L 11.5 7.5 L 20 7.5 L 21 8.5 L 21 18 L 20 19 L 4 19 L 3 18 Z" };
pub const FOLDER_PLUS: Icono = Icono { relleno: false, d: "M 3 6 L 4 5 L 9 5 L 11.5 7.5 L 20 7.5 L 21 8.5 L 21 18 L 20 19 L 4 19 L 3 18 Z M 12 10.5 L 12 16 M 9.2 13.2 L 14.8 13.2" };
pub const FOLDER_OPEN: Icono = Icono { relleno: false, d: "M 3 18 L 3 6 L 4 5 L 9 5 L 11.5 7.5 L 19 7.5 L 19 11 M 3 18 L 6 11 L 22 11 L 19 19 L 4 19 Z" };
pub const FOLDERS: Icono = Icono { relleno: false, d: "M 7 7 L 8 6 L 11 6 L 13 8 L 20 8 L 21 9 L 21 16 L 20 17 L 8 17 L 7 16 Z M 7 10 L 4 10 L 3 11 L 3 20 L 4 21 L 16 21 L 17 20 L 17 17" };
pub const MUSIC: Icono = Icono { relleno: false, d: "O 6.5 17.5 3 O 16.5 17.5 3 M 9.5 17.5 L 9.5 4 L 19.5 4 L 19.5 17.5 M 9.5 8 L 19.5 8" };
pub const MICROPHONE: Icono = Icono { relleno: false, d: "M 12 2.5 L 12 2.5 C 13.65 2.5 15 3.85 15 5.5 L 15 11.0 C 15 12.65 13.65 14.0 12 14.0 L 12 14.0 C 10.35 14.0 9 12.65 9 11.0 L 9 5.5 C 9 3.85 10.35 2.5 12 2.5 Z M 5 11 C 5 20 19 20 19 11 M 12 17.7 L 12 21 M 8 21 L 16 21" };
pub const KEYBOARD: Icono = Icono { relleno: false, d: "M 4.5 6 L 19.5 6 C 20.6 6 21.5 6.9 21.5 8 L 21.5 16 C 21.5 17.1 20.6 18 19.5 18 L 4.5 18 C 3.4 18 2.5 17.1 2.5 16 L 2.5 8 C 2.5 6.9 3.4 6 4.5 6 Z M 6 10 L 6.4 10 M 10 10 L 10.4 10 M 14 10 L 14.4 10 M 17.6 10 L 18 10 M 8 14 L 16 14" };
pub const PIANO: Icono = Icono { relleno: false, d: "M 5 4 L 19 4 C 20.1 4 21 4.9 21 6 L 21 18 C 21 19.1 20.1 20 19 20 L 5 20 C 3.9 20 3 19.1 3 18 L 3 6 C 3 4.9 3.9 4 5 4 Z M 9 4 L 9 20 M 15 4 L 15 20 M 7.6 4 L 7.6 13 L 10.4 13 L 10.4 4 M 13.6 4 L 13.6 13 L 16.4 13 L 16.4 4" };
pub const PLAYLIST: Icono = Icono { relleno: false, d: "M 4 6 L 15 6 M 4 11 L 12 11 M 4 16 L 10 16 O 15.5 17 2.5 M 18 17 L 18 9 L 21 10" };
pub const MOVIE: Icono = Icono { relleno: false, d: "M 5 4 L 19 4 C 20.1 4 21 4.9 21 6 L 21 18 C 21 19.1 20.1 20 19 20 L 5 20 C 3.9 20 3 19.1 3 18 L 3 6 C 3 4.9 3.9 4 5 4 Z M 8 4 L 8 20 M 16 4 L 16 20 M 3 9 L 8 9 M 3 15 L 8 15 M 16 9 L 21 9 M 16 15 L 21 15 M 8 12 L 16 12" };
pub const SQUARES: Icono = Icono { relleno: false, d: "M 5 3 L 13 3 C 14.1 3 15 3.9 15 5 L 15 13 C 15 14.1 14.1 15 13 15 L 5 15 C 3.9 15 3 14.1 3 13 L 3 5 C 3 3.9 3.9 3 5 3 Z M 15 9 L 19 9 L 21 11 L 21 19 L 19 21 L 11 21 L 9 19 L 9 15" };
pub const LAYOUT_ROWS: Icono = Icono { relleno: false, d: "M 6 4 L 18 4 C 19.1 4 20 4.9 20 6 L 20 18 C 20 19.1 19.1 20 18 20 L 6 20 C 4.9 20 4 19.1 4 18 L 4 6 C 4 4.9 4.9 4 6 4 Z M 4 12 L 20 12" };
pub const GRID_DOTS: Icono = Icono { relleno: false, d: "O 5 5 0.9 O 5 12 0.9 O 5 19 0.9 O 12 5 0.9 O 12 12 0.9 O 12 19 0.9 O 19 5 0.9 O 19 12 0.9 O 19 19 0.9" };
pub const BOX: Icono = Icono { relleno: false, d: "M 12 3 L 20 7.5 L 20 16.5 L 12 21 L 4 16.5 L 4 7.5 Z M 4 7.5 L 12 12 L 20 7.5 M 12 12 L 12 21" };
pub const PLUG: Icono = Icono { relleno: false, d: "M 8 3 L 8 8 M 16 3 L 16 8 M 5.5 8 L 18.5 8 L 18.5 11 C 18.5 19 5.5 19 5.5 11 Z M 12 17 L 12 21" };
pub const WAVE_SINE: Icono = Icono { relleno: false, d: "M 3 12 C 6 2 9 2 12 12 C 15 22 18 22 21 12" };
pub const FLASK_2: Icono = Icono { relleno: false, d: "M 8.5 3 L 15.5 3 M 10 3 L 10 9 L 5 18.5 C 4.3 20 5 21 6.5 21 L 17.5 21 C 19 21 19.7 20 19 18.5 L 14 9 L 14 3 M 7.4 14 L 16.6 14" };
pub const ADJUSTMENTS: Icono = Icono { relleno: false, d: "M 6 3 L 6 7 M 6 12 L 6 21 M 12 3 L 12 13 M 12 18 L 12 21 M 18 3 L 18 5 M 18 10 L 18 21 O 6 9.5 2.5 O 12 15.5 2.5 O 18 7.5 2.5" };
pub const ADJUSTMENTS_HORIZONTAL: Icono = Icono { relleno: false, d: "M 3 6 L 7 6 M 12 6 L 21 6 M 3 12 L 13 12 M 18 12 L 21 12 M 3 18 L 5 18 M 10 18 L 21 18 O 9.5 6 2.5 O 15.5 12 2.5 O 7.5 18 2.5" };
pub const SETTINGS: Icono = Icono { relleno: false, d: "M 18.38 10.3 L 20.87 10.48 L 20.87 13.52 L 18.38 13.7 L 17.71 15.31 L 19.35 17.2 L 17.2 19.35 L 15.31 17.71 L 13.7 18.38 L 13.52 20.87 L 10.48 20.87 L 10.3 18.38 L 8.69 17.71 L 6.8 19.35 L 4.65 17.2 L 6.29 15.31 L 5.62 13.7 L 3.13 13.52 L 3.13 10.48 L 5.62 10.3 L 6.29 8.69 L 4.65 6.8 L 6.8 4.65 L 8.69 6.29 L 10.3 5.62 L 10.48 3.13 L 13.52 3.13 L 13.7 5.62 L 15.31 6.29 L 17.2 4.65 L 19.35 6.8 L 17.71 8.69 Z O 12 12 2.8" };
pub const ARROWS_HORIZONTAL: Icono = Icono { relleno: false, d: "M 3 12 L 21 12 M 7 8 L 3 12 L 7 16 M 17 8 L 21 12 L 17 16" };
pub const ARROW_AUTOFIT_RIGHT: Icono = Icono { relleno: false, d: "M 20 4 L 20 20 M 3 12 L 16 12 M 12 8 L 16 12 L 12 16" };
pub const ARROW_BACK_UP: Icono = Icono { relleno: false, d: "M 8 5 L 4 9 L 8 13 M 4 9 L 15 9 C 21.5 9 21.5 19 15 19 L 11 19" };
pub const ARROW_LEFT: Icono = Icono { relleno: false, d: "M 5 12 L 19 12 M 11 6 L 5 12 L 11 18" };
pub const REPEAT: Icono = Icono { relleno: false, d: "M 4 12 L 4 10 C 4 8 5.5 7 7.5 7 L 20 7 M 17 4 L 20 7 L 17 10 M 20 12 L 20 14 C 20 16 18.5 17 16.5 17 L 4 17 M 7 14 L 4 17 L 7 20" };
pub const ROTATE_CLOCKWISE_2: Icono = Icono { relleno: false, d: "M 16.0 5.07 L 17.4 6.1 L 18.55 7.41 L 19.39 8.94 L 19.88 10.61 L 19.99 12.35 L 19.73 14.07 L 19.1 15.69 L 18.13 17.14 L 16.87 18.35 L 15.38 19.25 L 13.73 19.81 L 12.0 20.0 L 10.27 19.81 L 8.62 19.25 L 7.13 18.35 L 5.87 17.14 L 4.9 15.69 L 4.27 14.07 L 4.01 12.35 L 4.12 10.61 L 4.61 8.94 L 5.45 7.41 L 6.6 6.1 L 8.0 5.07 M 16.5 2 L 16 5.1 L 19.3 5.9" };
pub const MAXIMIZE: Icono = Icono { relleno: false, d: "M 4 9 L 4 4 L 9 4 M 15 4 L 20 4 L 20 9 M 20 15 L 20 20 L 15 20 M 9 20 L 4 20 L 4 15" };
pub const ZOOM_IN: Icono = Icono { relleno: false, d: "O 10 10 7 M 15 15 L 21 21 M 7 10 L 13 10 M 10 7 L 10 13" };
pub const MAGNET: Icono = Icono { relleno: false, d: "M 4 4 L 9 4 L 9 13 C 9 17 15 17 15 13 L 15 4 L 20 4 L 20 13 C 20 24 4 24 4 13 Z M 4 8 L 9 8 M 15 8 L 20 8" };
pub const METRONOME: Icono = Icono { relleno: false, d: "M 9 3 L 15 3 L 18.5 21 L 5.5 21 Z M 12 16 L 18 6 M 7 16 L 17 16" };
pub const STOPWATCH: Icono = Icono { relleno: false, d: "O 12 13.5 7.5 M 9.5 2.5 L 14.5 2.5 M 12 2.5 L 12 6 M 12 13.5 L 12 9.5 M 17.5 7.5 L 19 6" };
pub const SLICE: Icono = Icono { relleno: false, d: "M 4 20 L 16 4 C 18.5 2.5 21.5 5.5 20 8 L 11 16 L 8 14.5" };
pub const BLEND_MODE: Icono = Icono { relleno: false, d: "O 9 10 6 O 15 14 6" };
pub const HAND_FINGER: Icono = Icono { relleno: false, d: "M 9 13 L 9 5 C 9 3 12 3 12 5 L 12 11 L 17 12 C 19 12.5 19.5 13.5 19 15.5 L 18 21 L 10 21 L 5.5 15 C 4.5 13 7 11.5 9 13" };
pub const KEYFRAME: Icono = Icono { relleno: false, d: "M 12 4 L 19 12 L 12 20 L 5 12 Z" };
pub const STAR: Icono = Icono { relleno: false, d: "M 12.0 3.5 L 14.35 9.26 L 20.56 9.72 L 15.8 13.74 L 17.29 19.78 L 12.0 16.5 L 6.71 19.78 L 8.2 13.74 L 3.44 9.72 L 9.65 9.26 Z" };
pub const DEVICE_FLOPPY: Icono = Icono { relleno: false, d: "M 5 4 L 16 4 L 20 8 L 20 19 L 19 20 L 5 20 L 4 19 L 4 5 L 5 4 M 8 4 L 8 9 L 14 9 L 14 4 O 12 15 2.4" };
pub const PLAYER_PLAY: Icono = Icono { relleno: true, d: "M 7 4.5 L 20 12 L 7 19.5 Z" };
pub const PLAYER_STOP: Icono = Icono { relleno: true, d: "M 8 6 L 16 6 C 17.1 6 18 6.9 18 8 L 18 16 C 18 17.1 17.1 18 16 18 L 8 18 C 6.9 18 6 17.1 6 16 L 6 8 C 6 6.9 6.9 6 8 6 Z" };
pub const CARET_RIGHT: Icono = Icono { relleno: true, d: "M 9 6 L 16 12 L 9 18 Z" };
pub const CARET_LEFT: Icono = Icono { relleno: true, d: "M 15 6 L 8 12 L 15 18 Z" };
pub const CARET_UP: Icono = Icono { relleno: true, d: "M 6 15 L 12 8 L 18 15 Z" };
