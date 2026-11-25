// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! El piano roll.
use super::*;

pub const ESCALAS: [(&str, &[u8]); 6] = [
    ("Todas las notas", &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
    ("Mayor", &[0, 2, 4, 5, 7, 9, 11]),
    ("Menor", &[0, 2, 3, 5, 7, 8, 10]),
    ("Menor armónica", &[0, 2, 3, 5, 7, 8, 11]),
    ("Pentatónica", &[0, 3, 5, 7, 10]),
    ("Blues", &[0, 3, 5, 6, 7, 10]),
];
pub const TONOS: [&str; 12] = ["Do", "Do#", "Re", "Re#", "Mi", "Fa", "Fa#", "Sol", "Sol#", "La", "La#", "Si"];

#[derive(Clone, Copy, PartialEq)]
enum Gesto {
    Mover,
    Alargar,
    Elegir(Pos2),
    Borrar,
    Fuerza,
}

pub struct Notas {
    zoom: f32,
    alto: f32,
    /// Desplazamiento en puntos.
    scroll: Vec2,
    largo: u32,
    rejilla: u32,
    raiz: u8,
    escala: usize,
    elegidas: Vec<usize>,
    gesto: Option<(Gesto, Pos2, Vec<Nota>)>,
    suena: Option<(u32, u8)>,
    herramienta: BTreeMap<String, f32>,
    centrado: bool,
    /// Lo que va el ratón de más respecto a la rejilla, para dibujar sin brincos: puntos.
    flota: Vec2,
}

impl Default for Notas {
    fn default() -> Self {
        Notas {
            zoom: 0.36,
            alto: 15.0,
            scroll: Vec2::ZERO,
            largo: PASO * 2,
            rejilla: PASO,
            raiz: 9,
            escala: 0,
            elegidas: vec![],
            gesto: None,
            suena: None,
            herramienta: BTreeMap::new(),
            centrado: false,
            flota: Vec2::ZERO,
        }
    }
}

impl Notas {
    pub fn soltar(&mut self) {
        self.elegidas.clear();
        self.gesto = None;
    }

    /// La próxima vez se acomoda donde están las notas.
    pub fn centrar(&mut self) {
        self.centrado = false;
        self.soltar();
    }

    pub fn rejilla(&self) -> u32 {
        self.rejilla.max(1)
    }

    fn en_escala(&self, tono: u8) -> bool {
        ESCALAS[self.escala].1.contains(&((tono as i32 - self.raiz as i32).rem_euclid(12) as u8))
    }

    /// La nota de la escala más cercana.
    fn ajustar(&self, tono: i32) -> u8 {
        let tono = tono.clamp(0, 127);
        (0..12).flat_map(|d| [tono - d, tono + d]).find(|x| (0..=127).contains(x) && self.en_escala(*x as u8)).unwrap_or(tono) as u8
    }
}

impl App {
    fn lista(&mut self) -> &mut Vec<Nota> {
        let (pat, canal) = (self.patron, self.canal);
        let i = self.p.patrones.iter().position(|x| x.id == pat).unwrap_or(0);
        self.p.patrones[i].notas.entry(canal).or_default()
    }

    fn leer(&self) -> Vec<Nota> {
        self.p.patron(self.patron).and_then(|x| x.notas.get(&self.canal)).cloned().unwrap_or_default()
    }

    fn oir(&mut self, tono: u8) {
        self.parar_oir();
        self.sonar(self.canal, tono, 0.8);
        self.notas.suena = Some((self.canal, tono));
    }

    fn parar_oir(&mut self) {
        if let Some((c, n)) = self.notas.suena.take() {
            self.callar(c, n);
        }
    }

    pub(super) fn ver_notas(&mut self, ui: &mut Ui) {
        if self.canal == 0 {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(t("Elige un instrumento para escribir notas")).size(17.0).color(TENUE));
                if ui.button(RichText::new(t("Elegir instrumento…")).size(16.0).color(ACENTO)).clicked() {
                    self.elegir = Some(lado::Elegir::canal());
                }
            });
            return;
        }
        fila(ui, |ui| {
            self.barra_patron(ui);
            let antes = self.canal;
            let nombre = self.p.canales.iter().find(|c| c.id == antes).map_or(String::new(), |c| c.nombre.clone());
            let nombre = if nombre.is_empty() { t("Sin instrumento").into() } else { nombre };
            ComboBox::from_id_salt("canal").selected_text(nombre).width(110.0).show_ui(ui, |ui| {
                for c in &self.p.canales {
                    ui.selectable_value(&mut self.canal, c.id, &c.nombre);
                }
            });
            let nuevo = ui.button("+").on_hover_text(t("Instrumento nuevo para este patrón")).clicked();
            if nuevo {
                self.elegir = Some(lado::Elegir::canal());
            }
            self.boton_instrumento(ui);
            if antes != self.canal {
                self.notas.soltar();
                self.notas.centrado = false;
            }
            let rejillas = [("1/4", PPQ), ("1/8", PPQ / 2), ("1/16", PASO), ("1/32", PASO / 2), ("1/8 T", PPQ / 3), ("1/16 T", PPQ / 6)];
            let r = rejillas.iter().find(|x| x.1 == self.notas.rejilla).map_or("1/16", |x| x.0);
            ComboBox::from_id_salt("rejilla").selected_text(r).width(56.0).show_ui(ui, |ui| {
                for (s, v) in rejillas {
                    ui.selectable_value(&mut self.notas.rejilla, v, s);
                }
            });
            ComboBox::from_id_salt("raiz").selected_text(t(TONOS[self.notas.raiz as usize])).width(48.0).show_ui(ui, |ui| {
                for (i, s) in TONOS.iter().enumerate() {
                    ui.selectable_value(&mut self.notas.raiz, i as u8, t(s));
                }
            });
            ComboBox::from_id_salt("escala").selected_text(t(ESCALAS[self.notas.escala].0)).width(120.0).show_ui(ui, |ui| {
                for (i, s) in ESCALAS.iter().enumerate() {
                    ui.selectable_value(&mut self.notas.escala, i, t(s.0));
                }
            });
            self.menu_herramientas(ui);
            if foco(ui, self.anotar, t("Anotar"), ROJO)
                .on_hover_text(t("Escribe en el patrón lo que toques con el teclado o un MIDI mientras suena"))
                .clicked()
            {
                self.anotar = !self.anotar;
                self.cancion = false;
            }
        });
        ui.add_space(4.0);
        self.rollo(ui);
    }

    fn menu_herramientas(&mut self, ui: &mut Ui) {
        let mut usar = None;
        ui.menu_button(t("Herramientas"), |ui| {
            ui.set_min_width(190.0);
            let lista: Vec<_> = self.bib.de_tipo(Tipo::Herramienta).map(|g| (g.clave.clone(), g.ficha.clone(), g.fuente.clone())).collect();
            for (clave, f, fuente) in lista {
                ui.menu_button(f.nombre.ver(), |ui| {
                    for c in &f.controles {
                        let v = self.notas.herramienta.entry(format!("{clave}.{}", c.clave)).or_insert(c.valor);
                        ui.add(Slider::new(v, c.min..=c.max).text(c.nombre.ver()));
                    }
                    if ui.button(RichText::new(t("Aplicar")).color(ACENTO)).clicked() {
                        usar = Some((clave.clone(), fuente.clone()));
                        ui.close();
                    }
                });
            }
        })
        .response
        .on_hover_text(t("Transforman las notas elegidas, o todas. Son guiones de Lua: puedes hacer los tuyos"));
        if let Some((clave, fuente)) = usar {
            let pre = format!("{clave}.");
            let valores = self.notas.herramienta.iter().filter_map(|(k, v)| Some((k.strip_prefix(&pre)?.to_string(), *v))).collect();
            let largo = self.p.patron(self.patron).map_or(COMPAS, |x| x.largo());
            let el = std::mem::take(&mut self.notas.elegidas);
            let todas = self.leer();
            let (toca, queda): (Vec<_>, Vec<_>) = todas.iter().enumerate().partition(|(i, _)| el.is_empty() || el.contains(i));
            let toca: Vec<Nota> = toca.into_iter().map(|x| *x.1).collect();
            let archivos = self.bib.ver(&clave).map(|g| g.archivos.clone()).unwrap_or_default();
            match crate::lua::herramienta_con(&fuente, &archivos, &toca, largo, &valores) {
                Ok(v) => {
                    let base: Vec<Nota> = queda.into_iter().map(|x| *x.1).collect();
                    self.notas.elegidas = if el.is_empty() { vec![] } else { (base.len()..base.len() + v.len()).collect() };
                    *self.lista() = [base, v].concat();
                }
                Err(e) => self.avisar(e, true),
            }
        }
    }

    fn rollo(&mut self, ui: &mut Ui) {
        const TECLAS: f32 = 56.0;
        const REGLA: f32 = 20.0;
        const FUERZA: f32 = 70.0;
        let (todo, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let regla = Rect::from_min_max(pos2(todo.left() + TECLAS, todo.top()), pos2(todo.right(), todo.top() + REGLA));
        let r = Rect::from_min_max(pos2(todo.left() + TECLAS, regla.bottom()), pos2(todo.right(), todo.bottom() - FUERZA - 4.0));
        let rf = Rect::from_min_max(pos2(r.left(), todo.bottom() - FUERZA), todo.max);
        let teclas = Rect::from_min_max(pos2(todo.left(), r.top()), pos2(r.left(), r.bottom()));
        let largo = self.p.patron(self.patron).map_or(COMPAS, |x| x.largo());

        // Moverse y acercarse.
        let d = rueda(ui, todo);
        let (cmd, pin) = ui.input(|i| (i.modifiers.command, i.zoom_delta()));
        if let Some(o) = ui.input(|i| i.pointer.hover_pos()).filter(|o| todo.contains(*o) && ui.rect_contains_pointer(todo)) {
            let f = if cmd && d.y != 0.0 { (d.y / 300.0).exp() } else { pin };
            if f != 1.0 {
                let x = (o.x - r.left() + self.notas.scroll.x) / self.notas.zoom;
                self.notas.zoom = (self.notas.zoom * f).clamp(0.05, 3.0);
                self.notas.scroll.x = x * self.notas.zoom - (o.x - r.left());
            } else if !cmd {
                self.notas.scroll -= d;
            }
        }
        if !self.notas.centrado {
            self.notas.centrado = true;
            let v = self.leer();
            let medio = if v.is_empty() { 62.0 } else { v.iter().map(|n| n.tono as f32).sum::<f32>() / v.len() as f32 };
            self.notas.scroll.y = (127.0 - medio) * self.notas.alto - r.height() / 2.0;
        }
        let (zoom, alto) = (self.notas.zoom, self.notas.alto);
        let ancho_total = (largo + COMPAS) as f32 * zoom;
        self.notas.scroll.x = self.notas.scroll.x.clamp(0.0, (ancho_total - r.width()).max(0.0));
        self.notas.scroll.y = self.notas.scroll.y.clamp(0.0, (128.0 * alto - r.height()).max(0.0));
        let sc = self.notas.scroll;
        let x_de = |tick: f32| r.left() + tick * zoom - sc.x;
        let y_de = |tono: f32| r.top() + (127.0 - tono) * alto - sc.y;
        let tick_en = |x: f32| ((x - r.left() + sc.x) / zoom).max(0.0);
        let tono_en = |y: f32| 127 - ((y - r.top() + sc.y) / alto).floor() as i32;
        let caja = |n: &Nota| {
            Rect::from_min_size(pos2(x_de(n.tick as f32), y_de(n.tono as f32)), vec2((n.dur as f32 * zoom).max(4.0), alto)).shrink2(vec2(0.5, 1.0))
        };

        let p = ui.painter_at(r);
        let col = self.p.canales.iter().find(|c| c.id == self.canal).map_or(ACENTO, |c| color(c.color));
        for tono in tono_en(r.bottom()).max(0)..=tono_en(r.top()).min(127) {
            let f = Rect::from_min_size(pos2(r.left(), y_de(tono as f32)), vec2(r.width(), alto));
            let negra = matches!(tono % 12, 1 | 3 | 6 | 8 | 10);
            let dentro = self.notas.escala == 0 || self.notas.en_escala(tono as u8);
            let c = match (dentro, negra, self.notas.escala) {
                (false, ..) => Color32::from_rgb(14, 15, 19),
                (true, true, 0) => Color32::from_rgb(22, 24, 30),
                _ => Color32::from_rgb(30, 32, 40),
            };
            p.rect_filled(f.shrink2(vec2(0.0, 0.5)), 0.0, c);
        }
        let rej = self.notas.rejilla.max(1);
        let mut k = (tick_en(r.left()) as u32 / rej) * rej;
        while x_de(k as f32) < r.right() {
            let fuerte = if k.is_multiple_of(COMPAS) {
                1.0
            } else if k.is_multiple_of(PPQ) {
                0.6
            } else {
                0.25
            };
            if rej as f32 * zoom > 5.0 || k.is_multiple_of(PPQ) {
                p.vline(x_de(k as f32), r.y_range(), Stroke::new(1.0, CAJA2.gamma_multiply(fuerte)));
            }
            k += rej;
        }
        let fin = x_de(largo as f32);
        if fin < r.right() {
            p.rect_filled(Rect::from_min_max(pos2(fin.max(r.left()), r.top()), r.max), 0.0, Color32::from_black_alpha(90));
        }
        // Las notas de los otros canales, en sombra.
        if let Some(pat) = self.p.patron(self.patron) {
            for n in pat.notas.iter().filter(|(c, _)| **c != self.canal).flat_map(|(_, v)| v) {
                let b = caja(n);
                if b.intersects(r) {
                    p.rect_filled(b, 2.0, Color32::from_white_alpha(16));
                }
            }
        }
        let notas = self.leer();
        for (i, n) in notas.iter().enumerate() {
            let el = self.notas.elegidas.contains(&i);
            let mut b = caja(n);
            if el && self.notas.flota != Vec2::ZERO {
                // La sombra dice dónde va a caer.
                p.rect_stroke(b, 3.0, Stroke::new(1.0, col.gamma_multiply(0.7)), StrokeKind::Inside);
                match self.notas.gesto {
                    Some((Gesto::Alargar, ..)) => b.max.x = (b.max.x + self.notas.flota.x).max(b.min.x + 4.0),
                    _ => b = b.translate(self.notas.flota),
                }
            }
            if !b.intersects(r) {
                continue;
            }
            bloque(&p, b, 3, mezcla(mezcla(col, Color32::BLACK, 0.4), col, n.fuerza), el, el);
            if b.width() > 26.0 && alto > 11.0 {
                rotulo(&p, b.shrink2(vec2(5.0, 0.0)), &nombre(n.tono), 12.0, if el { FONDO } else { Color32::WHITE });
            }
        }
        // Fuerzas.
        let pf = ui.painter_at(rf);
        hueco(&pf, rf.shrink(1.0), 4, HUECO);
        for (i, n) in notas.iter().enumerate() {
            let x = x_de(n.tick as f32);
            let y = rf.bottom() - n.fuerza * (rf.height() - 6.0);
            let c = if self.notas.elegidas.contains(&i) { Color32::WHITE } else { col };
            pf.line_segment([pos2(x + 2.0, rf.bottom()), pos2(x + 2.0, y)], Stroke::new(2.0, c.gamma_multiply(0.7)));
            pf.circle_filled(pos2(x + 2.0, y), 3.5, c);
        }
        ui.painter().text(pos2(todo.left() + 4.0, rf.top() + 4.0), Align2::LEFT_TOP, t("Fuerza"), letra(11.0), TENUE);
        // Teclas.
        let pt = ui.painter_at(teclas);
        for tono in tono_en(r.bottom()).max(0)..=tono_en(r.top()).min(127) {
            let f = Rect::from_min_size(pos2(teclas.left(), y_de(tono as f32)), vec2(TECLAS - 2.0, alto));
            let negra = matches!(tono % 12, 1 | 3 | 6 | 8 | 10);
            let suena = self.notas.suena.is_some_and(|s| s.1 == tono as u8);
            // Marfil y ébano, con su luz.
            let f = f.shrink2(vec2(0.0, 0.5));
            if suena {
                degradado(&pt, f, 2, mezcla(col, Color32::WHITE, 0.3), col);
            } else if negra {
                degradado(&pt, f, 2, Color32::from_gray(0xc4), Color32::from_gray(0xb2));
                let q = Rect::from_min_max(f.min, pos2(f.left() + TECLAS * 0.62, f.bottom()));
                pt.rect_filled(q.translate(vec2(1.5, 1.0)), 2.0, Color32::from_black_alpha(90));
                degradado(&pt, q, 2, Color32::from_gray(0x4a), Color32::from_gray(0x14));
                pt.hline(q.x_range().shrink(2.0), q.top() + 1.0, Stroke::new(1.0, Color32::from_white_alpha(50)));
            } else {
                degradado(&pt, f, 2, Color32::from_gray(0xee), Color32::from_gray(0xc4));
            }
            if tono % 12 == 0 {
                pt.text(f.right_center() - vec2(4.0, 0.0), Align2::RIGHT_CENTER, nombre(tono as u8), letra(10.5), FONDO);
            }
        }
        // Regla y cabezal.
        let pr = ui.painter_at(regla);
        placa(&pr, regla, 0);
        let mut c = tick_en(r.left()) as u32 / COMPAS;
        while x_de((c * COMPAS) as f32) < r.right() {
            pr.text(pos2(x_de((c * COMPAS) as f32) + 4.0, regla.center().y), Align2::LEFT_CENTER, c + 1, letra(11.0), TENUE);
            c += 1;
        }
        if !self.cancion {
            let x = x_de(self.tick());
            ui.painter_at(regla.union(r)).vline(x, regla.union(r).y_range(), Stroke::new(1.5, ACENTO));
        }

        // Ratón.
        let Some(o) = ui.input(|i| i.pointer.interact_pos()) else { return self.al_soltar(ui) };
        let sobre = notas.iter().enumerate().rev().find(|(_, n)| r.contains(o) && caja(n).expand2(vec2(1.0, 0.0)).contains(o)).map(|x| x.0);
        let borde = |i: usize| o.x > caja(&notas[i]).right() - 6.0_f32.min(caja(&notas[i]).width() * 0.4);
        if resp.hovered() {
            if r.contains(o) {
                ui.ctx().set_cursor_icon(match sobre {
                    Some(i) if borde(i) => CursorIcon::ResizeHorizontal,
                    Some(_) => CursorIcon::Grab,
                    None => CursorIcon::Crosshair,
                });
            }
            self.ayuda("Clic: nota nueva · arrastra: mover · borde derecho: alargar · clic derecho: borrar · ⌘/Ctrl y arrastrar: elegir · ⌘/Ctrl y rueda: acercar");
        }
        let (abajo, sec, soltado, mods) =
            ui.input(|i| (i.pointer.primary_pressed(), i.pointer.secondary_pressed(), !i.pointer.any_down(), i.modifiers));
        let rej_de = |x: f32| (x / rej as f32).floor() as u32 * rej;
        if (abajo || sec) && resp.contains_pointer() {
            if regla.contains(o) {
                self.cancion = false;
                self.ir(tick_en(o.x) as f64);
            } else if teclas.contains(o) {
                self.oir(tono_en(o.y).clamp(0, 127) as u8);
            } else if rf.contains(o) {
                self.notas.gesto = Some((Gesto::Fuerza, o, notas.clone()));
            } else if r.contains(o) && sec {
                self.notas.gesto = Some((Gesto::Borrar, o, notas.clone()));
            } else if r.contains(o) && mods.command {
                self.notas.gesto = Some((Gesto::Elegir(o), o, notas.clone()));
            } else if r.contains(o) {
                let (i, g) = match sobre {
                    Some(i) => {
                        self.notas.largo = notas[i].dur;
                        (i, if borde(i) { Gesto::Alargar } else { Gesto::Mover })
                    }
                    None => {
                        let n = Nota { tick: rej_de(tick_en(o.x)), dur: self.notas.largo, tono: self.notas.ajustar(tono_en(o.y)), fuerza: 0.8 };
                        self.lista().push(n);
                        self.notas.elegidas.clear();
                        (notas.len(), Gesto::Mover)
                    }
                };
                if !self.notas.elegidas.contains(&i) {
                    self.notas.elegidas = vec![i];
                }
                let v = self.lista().clone();
                self.oir(v[i].tono);
                self.notas.gesto = Some((g, o, v));
            }
        }
        if let Some((g, ini, base)) = self.notas.gesto.clone() {
            let dx = (o.x - ini.x) / zoom;
            let paso = if mods.alt { 1 } else { rej as i32 };
            let dt = (dx / paso as f32).round() as i32 * paso;
            let el = self.notas.elegidas.clone();
            self.notas.flota = Vec2::ZERO;
            match g {
                Gesto::Mover => {
                    let dy = ((ini.y - o.y) / alto).round() as i32;
                    let lim = el.iter().filter_map(|i| base.get(*i)).map(|n| n.tick as i32).min().unwrap_or(0);
                    let dt = dt.max(-lim);
                    self.notas.flota = vec2((o.x - ini.x).max(-lim as f32 * zoom) - dt as f32 * zoom, (o.y - ini.y) + dy as f32 * alto);
                    let mut suena = None;
                    for i in &el {
                        if let Some(b) = base.get(*i).copied() {
                            let tono = if dy == 0 { b.tono } else { self.notas.ajustar(b.tono as i32 + dy) };
                            if el.len() == 1 && self.lista()[*i].tono != tono {
                                suena = Some(tono);
                            }
                            let n = &mut self.lista()[*i];
                            n.tick = (b.tick as i32 + dt) as u32;
                            n.tono = tono;
                        }
                    }
                    if let Some(x) = suena {
                        self.oir(x);
                    }
                }
                Gesto::Alargar => {
                    self.notas.flota.x = (o.x - ini.x) - dt as f32 * zoom;
                    for i in &el {
                        if let Some(b) = base.get(*i) {
                            let dur = (b.dur as i32 + dt).max(paso.min(b.dur as i32)) as u32;
                            self.lista()[*i].dur = dur;
                            self.notas.largo = dur;
                        }
                    }
                }
                Gesto::Elegir(a) => {
                    let m = Rect::from_two_pos(a, o);
                    ui.painter_at(r).rect_filled(m, 2.0, ACENTO.gamma_multiply(0.18));
                    self.notas.elegidas = notas.iter().enumerate().filter(|(_, n)| caja(n).intersects(m)).map(|x| x.0).collect();
                }
                Gesto::Borrar => {
                    if let Some(i) = sobre {
                        self.lista().remove(i);
                        self.notas.elegidas.clear();
                    }
                }
                Gesto::Fuerza => {
                    let f = ((rf.bottom() - o.y) / (rf.height() - 6.0)).clamp(0.02, 1.0);
                    let cerca = notas.iter().enumerate().filter(|(_, n)| (x_de(n.tick as f32) + 2.0 - o.x).abs() < 7.0).map(|x| x.0);
                    // Con varias elegidas en el mismo lugar, solo cambian las elegidas.
                    let cerca: Vec<usize> = cerca.collect();
                    let solo: Vec<usize> = cerca.iter().copied().filter(|i| el.contains(i)).collect();
                    for i in if solo.is_empty() { cerca } else { solo } {
                        self.lista()[i].fuerza = f;
                    }
                }
            }
        }
        if soltado {
            self.al_soltar(ui);
        }
    }

    /// Sin botones apretados: se cierra el gesto y el teclado edita.
    fn al_soltar(&mut self, ui: &mut Ui) {
        self.notas.flota = Vec2::ZERO;
        if self.notas.gesto.take().is_some() || self.notas.suena.is_some() {
            self.parar_oir();
        }
        let (pat, canal) = (self.patron, self.canal);
        if let Some(x) = self.p.patrones.iter_mut().find(|x| x.id == pat)
            && x.notas.get(&canal).is_some_and(|v| v.is_empty())
        {
            x.notas.remove(&canal);
        }
        self.teclas_notas(ui);
    }

    fn teclas_notas(&mut self, ui: &mut Ui) {
        if ui.ctx().text_edit_focused() || self.activa != Vista::Notas {
            return;
        }
        use egui::Key as K;
        let toco = |m: Modifiers, k: Key| ui.input_mut(|i| i.consume_key(m, k));
        if toco(Modifiers::COMMAND, K::A) {
            self.notas.elegidas = (0..self.leer().len()).collect();
        }
        let el = self.notas.elegidas.clone();
        if el.is_empty() {
            return;
        }
        if toco(Modifiers::NONE, K::Delete) || toco(Modifiers::NONE, K::Backspace) {
            let mut i = 0;
            self.lista().retain(|_| {
                i += 1;
                !el.contains(&(i - 1))
            });
            self.notas.elegidas.clear();
            return;
        }
        for (k, dy, m) in [
            (K::ArrowUp, 1, Modifiers::NONE),
            (K::ArrowDown, -1, Modifiers::NONE),
            (K::ArrowUp, 12, Modifiers::COMMAND),
            (K::ArrowDown, -12, Modifiers::COMMAND),
        ] {
            if toco(m, k) {
                for i in &el {
                    if let Some(n) = self.lista().get_mut(*i) {
                        n.tono = (n.tono as i32 + dy).clamp(0, 127) as u8;
                    }
                }
            }
        }
        for (k, dx) in [(K::ArrowRight, self.notas.rejilla as i64), (K::ArrowLeft, -(self.notas.rejilla as i64))] {
            if toco(Modifiers::NONE, k) {
                for i in &el {
                    if let Some(n) = self.lista().get_mut(*i) {
                        n.tick = (n.tick as i64 + dx).max(0) as u32;
                    }
                }
            }
        }
        if toco(Modifiers::COMMAND, K::D) {
            let v: Vec<Nota> = el.iter().filter_map(|i| self.lista().get(*i).copied()).collect();
            let a = v.iter().map(|n| n.tick).min().unwrap_or(0);
            let b = v.iter().map(|n| n.tick + n.dur).max().unwrap_or(0);
            let salto = (b - a).div_ceil(PPQ) * PPQ;
            let n = self.lista().len();
            self.lista().extend(v.iter().map(|x| Nota { tick: x.tick + salto, ..*x }));
            self.notas.elegidas = (n..n + v.len()).collect();
        }
    }
}

pub fn nombre(tono: u8) -> String {
    format!("{}{}", t(TONOS[tono as usize % 12]), tono / 12)
}
