// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! El editor de audio: la onda del clip elegido, para seleccionar y editar. Cada cambio guarda un audio nuevo y el clip
//! pasa a usarlo, así el archivo original queda intacto y deshacer funciona como en todo lo demás.
use super::*;
use crate::editar::{self, Audio};
use crate::motor::Escucha;
use std::ops::Range;

/// Lo que se deja en el portapapeles al copiar sonido.
const MARCA: &str = "Beethoven DAW Demo: audio";
/// Muestras que resume cada pico guardado.
const PASO_PICOS: usize = 256;

#[derive(Default)]
pub struct Editor {
    /// De qué clip es lo cargado: archivo, inicio, duración, velocidad y tempo.
    de: Option<(String, u32, u32, u32, u32)>,
    audio: Arc<Audio>,
    sr: f32,
    picos: Vec<[f32; 2]>,
    /// Lo seleccionado, en cuadros. Vacío es solo la rayita.
    pub sel: Range<usize>,
    ancla: usize,
    /// Lo que se ve, en cuadros.
    ve: (f64, f64),
    copiado: (Audio, f32),
    /// El ruido aprendido.
    ruido: Audio,
    duracion: f32,
    semitonos: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum Que {
    Cortar,
    Copiar,
    Pegar,
    Quitar,
    Recortar,
    Silenciar,
    Normalizar,
    Ganancia(f32),
    Fundido(bool),
    Invertir,
    Centrar,
    Mono,
    Lados,
    Fase,
    Silencios,
    Aprender,
    Ruido,
    Duracion,
    Tono,
}

impl Editor {
    pub fn cuadros(&self) -> usize {
        self.audio.len()
    }

    /// Lo seleccionado; sin nada, todo.
    fn rango(&self) -> Range<usize> {
        if self.sel.is_empty() { 0..self.audio.len() } else { self.sel.clone() }
    }
}

impl App {
    /// Carga el trozo que suena en el clip elegido, si cambió.
    fn cargar_editor(&mut self) -> bool {
        let Some(c) = self.clip.and_then(|i| self.p.clips.get(i)) else { return false };
        let Some(a) = &c.audio else { return false };
        let de = (a.ruta.clone(), a.inicio.to_bits(), c.dur, a.velocidad.to_bits(), self.p.bpm.to_bits());
        if self.audio_ed.de.as_ref() == Some(&de) {
            return !self.audio_ed.audio.is_empty();
        }
        let (ruta, inicio) = (a.ruta.clone(), a.inicio);
        let seg = c.dur as f32 * 60.0 / self.p.bpm / PPQ as f32 * a.velocidad;
        let otro = self.audio_ed.de.as_ref().is_none_or(|x| x.0 != ruta);
        self.audio_ed.de = Some(de);
        self.mando.mandar(Orden::Oir(None));
        let (x, sr) = match crate::decodificar::leer(Path::new(&ruta)) {
            Ok(w) => {
                let a = ((inicio * w.sr) as usize).min(w.izq.len());
                let b = (a + (seg * w.sr).round() as usize).min(w.izq.len());
                (w.izq[a..b].iter().zip(&w.der[a..b]).map(|(i, d)| [*i, *d]).collect::<Audio>(), w.sr)
            }
            Err(e) => {
                self.avisar(format!("{ruta}: {e}"), true);
                (vec![], 48000.0)
            }
        };
        let e = &mut self.audio_ed;
        e.picos = x.chunks(PASO_PICOS).map(extremos).collect();
        let n = x.len();
        e.audio = Arc::new(x);
        e.sr = sr;
        if otro || e.duracion == 0.0 {
            (e.sel, e.ve, e.duracion, e.semitonos) = (0..0, (0.0, n as f64), 100.0, 0.0);
        }
        e.sel = e.sel.start.min(n)..e.sel.end.min(n);
        e.ve = (e.ve.0.min(n as f64), e.ve.1.min(n as f64));
        if e.ve.1 - e.ve.0 < 16.0 {
            e.ve = (0.0, n as f64);
        }
        n > 0
    }

    /// Guarda el audio nuevo en su propio archivo y se lo pone al clip.
    fn poner_editado(&mut self, x: Audio) {
        let Some(i) = self.clip else { return };
        if x.is_empty() {
            return self.avisar(t("No se puede dejar el clip vacío"), true);
        }
        let dir = self.carpeta.clone().unwrap_or_else(std::env::temp_dir).join("Ediciones");
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        let r = dir.join(format!("edicion-{n}.wav"));
        let sr = self.audio_ed.sr;
        if let Err(e) = std::fs::create_dir_all(&dir).map_err(|e| e.to_string()).and_then(|_| wav::escribir(&r, sr as u32, &x)) {
            return self.avisar(tf("No se pudo guardar: {}", &[&e]), true);
        }
        let largo = x.len() as f32 / sr;
        let Some(velocidad) = self.p.clips.get(i).and_then(|c| c.audio.as_ref()).map(|a| a.velocidad) else { return };
        let dur = self.p.pulsos(largo / velocidad);
        let c = &mut self.p.clips[i];
        c.dur = dur;
        let a = c.audio.as_mut().unwrap();
        a.ruta = r.to_string_lossy().to_string();
        a.inicio = 0.0;
        // El archivo nuevo es del mismo clip: se conservan lo que se ve y lo seleccionado.
        if let Some(de) = &mut self.audio_ed.de {
            de.0 = a.ruta.clone();
            de.1 = u32::MAX;
        }
        let todo = self.audio_ed.ve == (0.0, self.audio_ed.cuadros() as f64);
        self.cargar_editor();
        if todo {
            self.audio_ed.ve = (0.0, self.audio_ed.cuadros() as f64);
        }
    }

    fn editar_audio(&mut self, ui: &Ui, que: Que) {
        let e = &mut self.audio_ed;
        let (r, sel, sr) = (e.rango(), e.sel.clone(), e.sr);
        match que {
            Que::Copiar | Que::Cortar => {
                e.copiado = (e.audio[r.clone()].to_vec(), sr);
                ui.ctx().copy_text(MARCA.into());
                if que == Que::Copiar {
                    return;
                }
            }
            Que::Aprender => {
                if sel.len() < 2048 {
                    return self.avisar(t("Selecciona primero un trozo donde solo se oiga el ruido"), true);
                }
                e.ruido = e.audio[sel].to_vec();
                return self.avisar(t("Ruido aprendido: ahora selecciona dónde quitarlo, o nada para todo"), false);
            }
            Que::Ruido if e.ruido.is_empty() => {
                return self.avisar(t("Selecciona primero un trozo donde solo se oiga el ruido"), true);
            }
            Que::Pegar if e.copiado.0.is_empty() => return,
            _ => {}
        }
        let mut x = e.audio.to_vec();
        // Lo que cambia de largo reemplaza el trozo; lo demás lo modifica en su lugar.
        let mut reemplazo: Option<Audio> = None;
        match que {
            Que::Cortar | Que::Quitar => {
                if sel.is_empty() {
                    return;
                }
                reemplazo = Some(vec![]);
            }
            Que::Pegar => {
                let (c, de) = &e.copiado;
                // De otro audio con otra frecuencia: se ajusta para que suene igual.
                let c = if *de == sr { c.clone() } else { editar::estirar(c, *de, sr / de) };
                let en = sel.start..sel.end;
                x.splice(en.clone(), c.iter().copied());
                e.sel = en.start..en.start + c.len();
                return self.poner_editado(x);
            }
            Que::Recortar => {
                if sel.is_empty() {
                    return;
                }
                x = x[sel].to_vec();
                e.sel = 0..0;
                e.ve = (0.0, x.len() as f64);
                return self.poner_editado(x);
            }
            Que::Silenciar => editar::silenciar(&mut x, r.clone()),
            Que::Normalizar => editar::normalizar(&mut x, r.clone()),
            Que::Ganancia(db) => editar::ganancia(&mut x, r.clone(), db),
            Que::Fundido(entra) => editar::fundido(&mut x, r.clone(), entra),
            Que::Invertir => editar::invertir(&mut x, r.clone()),
            Que::Centrar => editar::centrar(&mut x, r.clone()),
            Que::Mono => editar::mono(&mut x, r.clone()),
            Que::Lados => editar::cambiar_lados(&mut x, r.clone()),
            Que::Fase => editar::voltear_fase(&mut x, r.clone()),
            Que::Silencios => reemplazo = Some(editar::quitar_silencios(&x[r.clone()], sr)),
            Que::Ruido => reemplazo = Some(editar::quitar_ruido(&x[r.clone()], &e.ruido)),
            Que::Duracion => reemplazo = Some(editar::estirar(&x[r.clone()], sr, e.duracion / 100.0)),
            Que::Tono => reemplazo = Some(editar::cambiar_tono(&x[r.clone()], sr, e.semitonos)),
            Que::Copiar | Que::Aprender => return,
        }
        if let Some(n) = reemplazo {
            let largo = n.len();
            x.splice(r.clone(), n);
            e.sel = if sel.is_empty() { 0..0 } else { r.start..r.start + largo };
        }
        self.poner_editado(x);
    }

    /// Oye lo seleccionado (o desde la rayita), o lo calla si ya suena.
    pub(super) fn oir_editor(&mut self) {
        if self.estado.oyendo.load(Relaxed) != u32::MAX {
            self.mando.mandar(Orden::Oir(None));
            return;
        }
        let velocidad = self.clip_audio().map_or(1.0, |a| a.velocidad);
        let e = &self.audio_ed;
        if e.audio.is_empty() {
            return;
        }
        let fin = if e.sel.is_empty() { e.audio.len() } else { e.sel.end };
        let paso = (e.sr * velocidad / self.bib.sr) as f64;
        self.mando.mandar(Orden::Oir(Some(Escucha { datos: e.audio.clone(), pos: e.sel.start as f64, fin: fin as f64, paso })));
    }

    pub(super) fn ver_editor_audio(&mut self, ui: &mut Ui) {
        if !self.cargar_editor() {
            ui.centered_and_justified(|ui| ui.label(t("Haz doble clic en un audio de la Playlist para editarlo")));
            return;
        }
        let mut que = None;
        let hay = !self.audio_ed.sel.is_empty();
        let suena = self.estado.oyendo.load(Relaxed);
        ui.horizontal_wrapped(|ui| {
            if ui.button(if suena == u32::MAX { t("Oír") } else { t("Callar") }).on_hover_text(t("Espacio")).clicked() {
                self.oir_editor();
            }
            ui.separator();
            for (s, q, activo) in [
                ("Cortar", Que::Cortar, hay),
                ("Copiar", Que::Copiar, true),
                ("Pegar", Que::Pegar, !self.audio_ed.copiado.0.is_empty()),
                ("Quitar", Que::Quitar, hay),
                ("Recortar", Que::Recortar, hay),
                ("Silenciar", Que::Silenciar, true),
            ] {
                if ui.add_enabled(activo, Button::new(t(s))).clicked() {
                    que = Some(q);
                }
            }
            ui.separator();
            let mut op = |ui: &mut Ui, s: &str, q: Que| {
                if ui.button(t(s)).clicked() {
                    que = Some(q);
                    ui.close();
                }
            };
            ui.menu_button(t("Nivel"), |ui| {
                op(ui, "Normalizar", Que::Normalizar);
                op(ui, "Subir 3 dB", Que::Ganancia(3.0));
                op(ui, "Bajar 3 dB", Que::Ganancia(-3.0));
                op(ui, "Fundido de entrada", Que::Fundido(true));
                op(ui, "Fundido de salida", Que::Fundido(false));
                op(ui, "Centrar la onda", Que::Centrar);
            });
            ui.menu_button(t("Limpiar"), |ui| {
                op(ui, "Aprender el ruido de lo seleccionado", Que::Aprender);
                op(ui, "Quitar ese ruido", Que::Ruido);
                op(ui, "Acortar silencios", Que::Silencios);
            });
            ui.menu_button(t("Forma"), |ui| {
                op(ui, "Al revés", Que::Invertir);
                op(ui, "Hacer mono", Que::Mono);
                op(ui, "Cambiar izquierda y derecha", Que::Lados);
                op(ui, "Voltear la fase", Que::Fase);
            });
            let e = &mut self.audio_ed;
            ui.menu_button(t("Tiempo y tono"), |ui| {
                ui.add(Slider::new(&mut e.duracion, 50.0..=200.0).suffix(" %").text(t("Duración")));
                if ui.button(t("Cambiar duración sin cambiar el tono")).clicked() {
                    que = Some(Que::Duracion);
                    ui.close();
                }
                ui.add(Slider::new(&mut e.semitonos, -12.0..=12.0).step_by(1.0).text(t("Semitonos")));
                if ui.button(t("Cambiar el tono sin cambiar la duración")).clicked() {
                    que = Some(Que::Tono);
                    ui.close();
                }
            });
            ui.separator();
            let r = self.audio_ed.rango();
            let pico = self.audio_ed.audio[r.clone()].iter().step_by(1 + r.len() / 200_000).flatten().fold(0.0f32, |m, v| m.max(v.abs()));
            let sr = self.audio_ed.sr;
            let db = 20.0 * pico.max(1e-6).log10();
            let s =
                tf("{} s a {} s · pico {} dB", &[&format!("{:.2}", r.start as f32 / sr), &format!("{:.2}", r.end as f32 / sr), &format!("{db:.1}")]);
            ui.label(RichText::new(s).color(TENUE));
        });

        // Las teclas de siempre, cuando esta ventana tiene el teclado.
        if self.activa == Vista::Editor && !ui.ctx().text_edit_focused() {
            let toco = |m: Modifiers, k: Key| ui.input_mut(|i| i.consume_key(m, k));
            if toco(Modifiers::COMMAND, Key::A) {
                self.audio_ed.sel = 0..self.audio_ed.cuadros();
            }
            if toco(Modifiers::NONE, Key::Delete) || toco(Modifiers::NONE, Key::Backspace) {
                que = Some(Que::Quitar);
            }
            ui.input(|i| {
                for e in &i.events {
                    match e {
                        Event::Copy => que = Some(Que::Copiar),
                        Event::Cut => que = Some(Que::Cortar),
                        Event::Paste(s) if s == MARCA => que = Some(Que::Pegar),
                        _ => {}
                    }
                }
            });
        }

        self.dibujar_onda(ui, suena);
        if suena != u32::MAX {
            ui.ctx().request_repaint();
        }
        if let Some(q) = que {
            self.editar_audio(ui, q);
        }
    }

    fn dibujar_onda(&mut self, ui: &mut Ui, suena: u32) {
        let (r, resp) = ui.allocate_exact_size(ui.available_size().max(vec2(200.0, 80.0)), Sense::click_and_drag());
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, t("Onda del audio")));
        let p = ui.painter_at(r);
        hueco(&p, r, 5, HUECO);
        let e = &mut self.audio_ed;
        let n = e.cuadros() as f64;
        let (mut a, mut b) = e.ve;
        // La rueda acerca donde está el ratón; de lado, o con mayúsculas, recorre.
        if let Some(o) = resp.hover_pos() {
            let g = rueda(ui, r);
            let zoom = ui.input(|i| i.zoom_delta()) as f64 * (g.y as f64 * 0.004).exp();
            let k = ((o.x - r.left()) / r.width()) as f64;
            let en = a + (b - a) * k;
            let ancho = ((b - a) / zoom).clamp(64.0f64.min(n), n);
            let corre = -(g.x as f64) / r.width() as f64 * ancho;
            a = (en - ancho * k + corre).clamp(0.0, n - ancho);
            b = a + ancho;
            e.ve = (a, b);
        }
        let por_punto = (b - a) / r.width() as f64;
        let cuadro_en = |x: f32| (a + (x - r.left()) as f64 * por_punto).clamp(0.0, n) as usize;
        let x_de = |c: f64| r.left() + ((c - a) / por_punto) as f32;
        if let Some(o) = resp.interact_pointer_pos() {
            let c = cuadro_en(o.x);
            if resp.drag_started() {
                // Desde donde se apretó, no desde donde egui notó el arrastre.
                let apretado = ui.input(|i| i.pointer.press_origin()).map_or(c, |o| cuadro_en(o.x));
                e.ancla = apretado;
            }
            if resp.dragged() {
                e.sel = e.ancla.min(c)..e.ancla.max(c);
            } else if resp.clicked() {
                // Con mayúsculas alarga lo seleccionado.
                if ui.input(|i| i.modifiers.shift) {
                    e.sel = e.sel.start.min(c)..e.sel.end.max(c);
                } else {
                    e.sel = c..c;
                }
            }
        }
        if resp.double_clicked() {
            e.sel = 0..e.cuadros();
        }
        if !e.sel.is_empty() {
            let x = (x_de(e.sel.start as f64)..=x_de(e.sel.end as f64)).into_inner();
            p.rect_filled(Rect::from_x_y_ranges(x.0..=x.1, r.y_range()), 0.0, ACENTO.gamma_multiply(0.18));
        }
        let alto = r.height() / 2.0;
        for l in 0..2 {
            let medio = r.top() + alto * (l as f32 + 0.5);
            p.hline(r.x_range(), medio, Stroke::new(1.0, Color32::from_white_alpha(18)));
        }
        let color = Color32::from_rgb(0xe8, 0x9a, 0x8a);
        let mut x = r.left();
        while x < r.right() {
            let i = cuadro_en(x);
            let j = cuadro_en(x + 1.0).max(i + 1).min(e.cuadros());
            if i >= j {
                break;
            }
            for l in 0..2 {
                // De lejos bastan los picos ya sumados (de los dos lados juntos); de cerca, cada lado.
                let v = if por_punto > 2.0 * PASO_PICOS as f64 {
                    let q = &e.picos[i / PASO_PICOS..(j / PASO_PICOS).max(i / PASO_PICOS + 1).min(e.picos.len())];
                    q.iter().fold([0.0f32; 2], |m, v| [m[0].min(v[0]), m[1].max(v[1])])
                } else {
                    e.audio[i..j].iter().fold([f32::MAX, f32::MIN], |m, v| [m[0].min(v[l]), m[1].max(v[l])])
                };
                let medio = r.top() + alto * (l as f32 + 0.5);
                let k = alto * 0.48;
                p.vline(x + 0.5, (medio - v[1] * k - 0.5)..=(medio - v[0] * k + 0.5), Stroke::new(1.0, color));
            }
            x += 1.0;
        }
        if e.sel.is_empty() {
            p.vline(x_de(e.sel.start as f64), r.y_range(), Stroke::new(1.0, ACENTO));
        }
        if suena != u32::MAX {
            p.vline(x_de(suena as f64), r.y_range(), Stroke::new(1.5, VERDE));
        }
        let seg = |c: f64| format!("{:.2} s", c / e.sr as f64);
        p.text(r.left_bottom() + vec2(5.0, -3.0), Align2::LEFT_BOTTOM, seg(a), letra(11.0), TENUE);
        p.text(r.right_bottom() + vec2(-5.0, -3.0), Align2::RIGHT_BOTTOM, seg(b), letra(11.0), TENUE);
    }
}

/// El punto más bajo y el más alto de un trozo, juntando los dos lados.
fn extremos(c: &[[f32; 2]]) -> [f32; 2] {
    c.iter().flatten().fold([0.0f32; 2], |m, v| [m[0].min(*v), m[1].max(*v)])
}
