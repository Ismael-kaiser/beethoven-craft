//! Pestaña ANIMACIÓN: los textos e imágenes que se mandaron aquí («Agregar a la ventana de animación»), sus
//! animaciones listas y, abajo, sus pistas de animación: una por propiedad, con sus cuadros clave y la curva de cada
//! tramo (curvas de AniCraft, `crate::curvas`).
use super::estilo::*;
use super::*;
use crate::animaciones::{self, ANIMACIONES, Propiedad};
use crate::curvas::Curva;
use crate::idioma::{t, tf};

/// Ancho de la cabecera de cada pista.
const CAB: f32 = 170.0;
const FILA_CLIP: f32 = 26.0;
const FILA: f32 = 24.0;

#[derive(Default)]
pub struct Estado {
    /// El clip elegido en la lista.
    pub clip: Option<u64>,
    /// Cuadro clave elegido: clip, propiedad y su segundo dentro del clip.
    pub clave: Option<(u64, Propiedad, f64)>,
    /// Cuadro clave que se está arrastrando (mismo formato).
    arrastre: Option<(u64, Propiedad, f64)>,
    /// El cuadro clave del menú del clic derecho.
    menu: Option<(u64, Propiedad, f64)>,
}

enum Hacer {
    Ir(f64),
    Elegir(u64, Propiedad, f64),
    Agregar(u64, Propiedad, f64),
    Mover(u64, Propiedad, f64, f64),
    Curva(u64, Propiedad, f64, Curva),
    Borrar(u64, Propiedad, f64),
    Quitar(u64),
}

fn nombre_clip(c: &ClipVideo) -> String {
    match &c.contenido {
        Contenido::Titulo(x) => x.texto.lines().next().unwrap_or("").chars().take(28).collect(),
        _ => nombre_de(&c.ruta),
    }
}

impl App {
    /// Manda un clip de texto o imagen a la pestaña de animación y la abre.
    pub fn animar(&mut self, id: u64) {
        if let Some(c) = self.p.clip_video(id) {
            c.animado = true;
        }
        if !self.pestanas.contains(&Espacio::Animacion) {
            self.pestanas.push(Espacio::Animacion);
        }
        self.espacio = Espacio::Animacion;
        self.anim.clip = Some(id);
        self.clip = Some(id);
    }

    pub fn espacio_animacion(&mut self, ui: &mut Ui) {
        let alto = ui.available_height();
        Panel::bottom("anim-pistas").frame(Frame::new().fill(WORKSPACE)).resizable(true).default_size(alto * 0.45).size_range(160.0..=(alto - 160.0).max(200.0)).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            self.pistas_animacion(ui);
        });
        Panel::left("anim-izq").frame(Frame::new().fill(WINDOW_TITLE).inner_margin(8.0)).resizable(true).default_size(320.0).min_size(250.0).show(ui, |ui| {
            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.panel_animacion(ui));
        });
        CentralPanel::default().frame(Frame::new().fill(Color32::BLACK)).show(ui, |ui| self.preview(ui));
    }

    fn panel_animacion(&mut self, ui: &mut Ui) {
        let animados: Vec<(u64, String)> = self.p.video.iter().filter(|c| c.animado).map(|c| (c.id, nombre_clip(c))).collect();
        ui.label(RichText::new(t("Clips animados")).font(barlow(17.0)).color(TEXT));
        ui.add_space(4.0);
        if animados.is_empty() {
            ui.label(RichText::new(t("Haz clic derecho en un clip de texto o imagen y elige «Agregar a la ventana de animación».")).size(13.0).color(TEXT_DIM));
            return;
        }
        if self.anim.clip.is_none_or(|id| !animados.iter().any(|a| a.0 == id)) {
            self.anim.clip = animados.first().map(|a| a.0);
        }
        for (id, n) in &animados {
            if ui.selectable_label(self.anim.clip == Some(*id), n).clicked() {
                self.anim.clip = Some(*id);
                self.clip = Some(*id);
            }
        }
        let Some(id) = self.anim.clip else { return };
        ui.add_space(12.0);
        ui.label(RichText::new(t("Animaciones")).font(barlow(17.0)).color(TEXT));
        ui.label(RichText::new(t("Se ponen sobre el clip elegido; puedes juntar varias.")).size(12.5).color(TEXT_FAINT));
        ui.add_space(4.0);
        for (k, a) in ANIMACIONES.iter().enumerate() {
            if k > 0 && a.salida && !ANIMACIONES[k - 1].salida {
                ui.add_space(6.0);
            }
            if ui.button(t(a.nombre)).on_hover_text(t(a.ayuda)).clicked() {
                if let Some(c) = self.p.clip_video(id) {
                    animaciones::poner(c, a);
                }
                self.avisar(tf("Animación «{}» puesta", &[&t(a.nombre)]), false);
            }
        }
        // La curva del cuadro clave elegido.
        if let Some((cid, prop, tk)) = self.anim.clave.filter(|c| c.0 == id) {
            ui.add_space(12.0);
            ui.label(RichText::new(t("Curva del cuadro clave")).font(barlow(17.0)).color(TEXT));
            ui.label(RichText::new(tf("{} en {} s", &[&t(prop.nombre()), &format!("{tk:.2}")])).size(12.5).color(TEXT_DIM));
            let actual = self.p.video.iter().find(|c| c.id == cid).map(|c| prop.de(c).curva_en(tk));
            ComboBox::from_id_salt("anim-curva").width(260.0).selected_text(actual.map_or(String::new(), |c| c.nombre())).show_ui(ui, |ui| {
                for cv in Curva::menu() {
                    if ui.selectable_label(actual == Some(cv), cv.nombre()).clicked() {
                        if let Some(c) = self.p.clip_video(cid) {
                            prop.de_mut(c).poner_curva(tk, cv);
                        }
                    }
                }
            });
        }
        ui.add_space(12.0);
        ui.label(RichText::new(t("Valores en el cursor")).font(barlow(17.0)).color(TEXT));
        ui.label(RichText::new(t("Con un cuadro clave en la pista, cambiar un valor pone o mueve el del cursor.")).size(12.5).color(TEXT_FAINT));
        self.clip = Some(id);
        self.inspector(ui);
    }

    fn pistas_animacion(&mut self, ui: &mut Ui) {
        let todo = ui.available_rect_before_wrap();
        let resp = ui.allocate_rect(todo, Sense::click_and_drag());
        let p = ui.painter_at(todo);
        p.rect_filled(todo, 0, WORKSPACE);
        let fin = self.p.fin().max(5.0) + 1.0;
        let carril = Rect::from_min_max(pos2(todo.left() + CAB, todo.top()), todo.max);
        let a_x = |s: f64| carril.left() + 8.0 + ((s / fin) as f32) * (carril.width() - 16.0);
        let a_t = |x: f32| (((x - carril.left() - 8.0) / (carril.width() - 16.0)) as f64 * fin).clamp(0.0, fin);
        // Regla
        let regla = Rect::from_min_max(pos2(carril.left(), todo.top()), pos2(carril.right(), todo.top() + 22.0));
        p.rect_filled(regla, 0, RULER);
        let paso = [0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0].into_iter().find(|s| (s / fin) as f32 * carril.width() > 60.0).unwrap_or(60.0);
        let mut s = 0.0;
        while s <= fin {
            let x = a_x(s);
            p.vline(x, regla.y_range(), Stroke::new(1.0, GRID_BAR));
            p.text(pos2(x + 3.0, regla.center().y), Align2::LEFT_CENTER, tiempo(s), fuente(12.0), RULER_TEXT);
            s += paso;
        }
        let animados: Vec<ClipVideo> = self.p.video.iter().filter(|c| c.animado).cloned().collect();
        let raton = ui.input(|i| i.pointer.hover_pos());
        let mut hacer = vec![];
        let mut y = regla.bottom();
        // (rect del rombo, clip, propiedad, segundo en el clip)
        let mut rombos: Vec<(Rect, u64, Propiedad, f64)> = vec![];
        // (rect del carril, clip, propiedad)
        let mut carriles: Vec<(Rect, ClipVideo, Propiedad)> = vec![];
        if animados.is_empty() {
            p.text(carril.center(), Align2::CENTER_CENTER, t("Aquí aparecen las pistas de animación de cada clip"), fuente(15.0), TEXT_FAINT);
        }
        for c in &animados {
            let cab = Rect::from_min_size(pos2(todo.left(), y), vec2(todo.width(), FILA_CLIP));
            let elegido = self.anim.clip == Some(c.id);
            p.rect_filled(cab, 0, if elegido { BUTTON } else { TRACK_HEADER });
            p.text(pos2(cab.left() + 10.0, cab.center().y), Align2::LEFT_CENTER, nombre_clip(c), fuente(14.5), TEXT);
            let x = Rect::from_center_size(pos2(todo.left() + CAB - 16.0, cab.center().y), vec2(18.0, 18.0));
            let sobre_x = raton.is_some_and(|m| x.contains(m));
            p.text(x.center(), Align2::CENTER_CENTER, "✕", fuente(13.0), if sobre_x { TEXT } else { TEXT_FAINT });
            if sobre_x && resp.clicked() {
                hacer.push(Hacer::Quitar(c.id));
            }
            // El tramo del clip en la línea de tiempo.
            p.rect_filled(Rect::from_min_max(pos2(a_x(c.inicio), cab.top() + 5.0), pos2(a_x(c.fin()), cab.bottom() - 5.0)), 4, mezcla(TRACK_HEADER, MAIN, 0.25));
            y += FILA_CLIP;
            for prop in Propiedad::TODAS {
                let fila = Rect::from_min_size(pos2(todo.left(), y), vec2(todo.width(), FILA));
                p.text(pos2(fila.left() + 22.0, fila.center().y), Align2::LEFT_CENTER, t(prop.nombre()), fuente(13.0), TEXT_DIM);
                let zona = Rect::from_min_max(pos2(carril.left(), fila.top()), fila.max);
                p.rect_filled(Rect::from_min_max(pos2(a_x(c.inicio), zona.top() + 2.0), pos2(a_x(c.fin()), zona.bottom() - 2.0)), 2, STRIP_DARK);
                p.hline(fila.x_range(), fila.bottom() - 0.5, Stroke::new(1.0, GRID_GAP));
                let pr = prop.de(c);
                // Forma de la curva entre cada par de cuadros clave.
                for w in pr.claves.windows(2) {
                    let (ka, kb) = (&w[0], &w[1]);
                    let cv = pr.curva_en(ka.0);
                    let (xa, xb) = (a_x(c.inicio + ka.0), a_x(c.inicio + kb.0));
                    let h = zona.height() - 8.0;
                    let pts: Vec<Pos2> = (0..=24).map(|i| {
                        let k = i as f64 / 24.0;
                        pos2(xa + (xb - xa) * k as f32, zona.bottom() - 4.0 - (cv.aplicar(k) as f32).clamp(-0.4, 1.4) * h)
                    }).collect();
                    p.add(Shape::line(pts, Stroke::new(1.5, MAIN.gamma_multiply(0.7))));
                }
                for k in &pr.claves {
                    let x = a_x(c.inicio + k.0);
                    let r = Rect::from_center_size(pos2(x, zona.center().y), vec2(12.0, 12.0));
                    let elegida = self.anim.clave.is_some_and(|e| e.0 == c.id && e.1 == prop && (e.2 - k.0).abs() < 0.02);
                    let sobre = raton.is_some_and(|m| r.expand(3.0).contains(m));
                    let q = r.center();
                    let rombo = vec![q + vec2(0.0, -6.0), q + vec2(6.0, 0.0), q + vec2(0.0, 6.0), q + vec2(-6.0, 0.0)];
                    p.add(Shape::convex_polygon(rombo, if elegida { MAIN } else if sobre { HIGHLIGHT } else { TEXT_DIM }, Stroke::new(1.0, Color32::BLACK)));
                    rombos.push((r.expand(3.0), c.id, prop, k.0));
                }
                carriles.push((zona, c.clone(), prop));
                y += FILA;
            }
        }
        // Cursor de reproducción
        let xa = a_x(self.ahora());
        p.vline(xa, todo.top()..=todo.bottom(), Stroke::new(1.5, if self.grab.is_some() { RECORD } else { MAIN }));
        // Ratón
        let en_rombo = |m: Pos2| rombos.iter().find(|r| r.0.contains(m)).copied();
        if resp.drag_started() {
            if let Some((_, id, prop, tk)) = ui.input(|i| i.pointer.press_origin()).and_then(en_rombo) {
                self.anim.arrastre = Some((id, prop, tk));
                hacer.push(Hacer::Elegir(id, prop, tk));
            }
        }
        if resp.dragged() {
            if let (Some((id, prop, tk)), Some(m)) = (self.anim.arrastre, raton) {
                if let Some(c) = animados.iter().find(|c| c.id == id) {
                    let nuevo = (a_t(m.x) - c.inicio).clamp(0.0, c.dur);
                    if (nuevo - tk).abs() > 0.005 {
                        hacer.push(Hacer::Mover(id, prop, tk, nuevo));
                        self.anim.arrastre = Some((id, prop, nuevo));
                    }
                }
            } else if let Some(m) = raton.filter(|m| carril.contains(*m)) {
                hacer.push(Hacer::Ir(a_t(m.x)));
            }
        }
        if resp.drag_stopped() {
            self.anim.arrastre = None;
        }
        if let Some(m) = raton {
            if resp.double_clicked() {
                if let Some((_, c, prop)) = carriles.iter().find(|c| c.0.contains(m)) {
                    let tk = a_t(m.x) - c.inicio;
                    if (0.0..=c.dur).contains(&tk) && en_rombo(m).is_none() {
                        hacer.push(Hacer::Agregar(c.id, *prop, tk));
                    }
                }
            } else if resp.clicked() {
                match en_rombo(m) {
                    Some((_, id, prop, tk)) => {
                        hacer.push(Hacer::Elegir(id, prop, tk));
                        if let Some(c) = animados.iter().find(|c| c.id == id) {
                            hacer.push(Hacer::Ir(c.inicio + tk));
                        }
                    }
                    None if m.x >= carril.left() => hacer.push(Hacer::Ir(a_t(m.x))),
                    None => {}
                }
            }
            if resp.secondary_clicked() {
                self.anim.menu = en_rombo(m).map(|r| (r.1, r.2, r.3));
            }
        }
        if let Some((id, prop, tk)) = self.anim.menu {
            let actual = animados.iter().find(|c| c.id == id).map(|c| prop.de(c).curva_en(tk));
            resp.context_menu(|ui| {
                ui.label(RichText::new(t("Curva hasta el siguiente cuadro clave")).color(TEXT_DIM).size(12.5));
                for cv in Curva::menu() {
                    if ui.selectable_label(actual == Some(cv), cv.nombre()).clicked() {
                        hacer.push(Hacer::Curva(id, prop, tk, cv));
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button(t("Borrar cuadro clave")).clicked() {
                    hacer.push(Hacer::Borrar(id, prop, tk));
                    ui.close();
                }
            });
        }
        if resp.hovered() {
            resp.on_hover_text(t("Doble clic en una pista: cuadro clave nuevo. Arrastra un rombo para moverlo; clic derecho: su curva o borrarlo."));
        }
        for h in hacer {
            match h {
                Hacer::Ir(s) => self.ir(s),
                Hacer::Elegir(id, prop, tk) => {
                    self.anim.clave = Some((id, prop, tk));
                    self.anim.clip = Some(id);
                }
                Hacer::Agregar(id, prop, tk) => {
                    if let Some(c) = self.p.clip_video(id) {
                        prop.de_mut(c).alternar_clave(tk);
                    }
                    self.anim.clave = Some((id, prop, tk));
                }
                Hacer::Mover(id, prop, de, a) => {
                    if let Some(c) = self.p.clip_video(id) {
                        let pr = prop.de_mut(c);
                        if let Some(i) = pr.clave_en(de) {
                            pr.mover_clave(i, a);
                        }
                    }
                    self.anim.clave = Some((id, prop, a));
                }
                Hacer::Curva(id, prop, tk, cv) => {
                    if let Some(c) = self.p.clip_video(id) {
                        prop.de_mut(c).poner_curva(tk, cv);
                    }
                }
                Hacer::Borrar(id, prop, tk) => {
                    if let Some(c) = self.p.clip_video(id) {
                        prop.de_mut(c).alternar_clave(tk);
                    }
                    self.anim.clave = None;
                }
                Hacer::Quitar(id) => {
                    if let Some(c) = self.p.clip_video(id) {
                        c.animado = false;
                    }
                    if self.anim.clip == Some(id) {
                        self.anim.clip = None;
                    }
                }
            }
        }
    }
}
