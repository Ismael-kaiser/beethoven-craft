//! Animación libre sobre la imagen: llaves por tipo con clic, puntos del camino que se mueven, y trazos a mano.
use super::estilo::*;
use super::*;
use crate::animaciones::{self as an, Llave, Pista};
use crate::curvas::Curva;
use crate::idioma::{t, tf};

/// Qué hace el ratón sobre la imagen con un clip elegido.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Modo {
    /// Mover, escalar y girar.
    #[default]
    Mover,
    /// Cada clic es una posición y el cursor avanza.
    Puntos,
    /// Se dibuja el camino.
    Mano,
}

pub struct Libre {
    /// La barra desplegada.
    pub abierta: bool,
    pub modo: Modo,
    /// Segundos que avanza el cursor con cada punto.
    pub paso: f64,
    /// El trazo dura lo que le queda al clip, y no lo que se tardó en dibujarlo.
    pub llenar: bool,
    /// Segundo del clip donde empezó el trazo, hora en que empezó y lo dibujado.
    trazo: Option<(f64, f64, Vec<(f64, [f64; 2])>)>,
    /// Punto del camino que se está moviendo: clip y segundo.
    punto: Option<(u64, f64)>,
}

impl Default for Libre {
    fn default() -> Self {
        Libre { abierta: false, modo: Modo::Mover, paso: 0.5, llenar: false, trazo: None, punto: None }
    }
}

pub fn color(l: Option<Llave>) -> Color32 {
    match l {
        Some(Llave::Posicion) => MAIN,
        Some(Llave::Giro) => c(0x5fb3ff),
        Some(Llave::Escala) => c(0x6fdc8c),
        Some(Llave::Estirado) => c(0x4fd1c5),
        Some(Llave::Opacidad) => c(0xff8fb1),
        _ => c(0xe9e9f2),
    }
}

/// «Posición, Giro» y el color de un momento con llaves.
pub fn resumen(v: &[Pista]) -> (String, Color32) {
    let mut tipos: Vec<Option<Llave>> = vec![];
    for p in v {
        let l = Llave::de(p);
        if !tipos.contains(&l) {
            tipos.push(l);
        }
    }
    let n = |l: &Option<Llave>| t(l.and_then(|l| Llave::TODAS.iter().find(|x| x.0 == l)).map_or("Efecto", |x| x.1));
    (tipos.iter().map(n).collect::<Vec<_>>().join(", "), color(if tipos.len() == 1 { tipos[0] } else { None }))
}

pub fn pinta_rombo(p: &Painter, o: Pos2, r: f32, col: Color32, borde: Color32) {
    p.add(Shape::convex_polygon([vec2(0.0, -r), vec2(r, 0.0), vec2(0.0, r), vec2(-r, 0.0)].map(|v| o + v).to_vec(), col, Stroke::new(1.2, borde)));
}

/// Menú de las curvas más usadas. Devuelve la elegida.
pub fn menu_curvas(ui: &mut Ui) -> Option<Curva> {
    let mut r = None;
    for (n, cv) in animacion::rapidas() {
        if ui.button(t(n)).clicked() {
            r = Some(cv);
        }
    }
    r
}

impl App {
    /// «Poner llave de…» para el clip en el segundo `s` de la línea de tiempo.
    pub fn menu_llaves(&mut self, ui: &mut Ui, id: u64, s: f64) {
        let Some(cl) = self.p.video.iter().find(|c| c.id == id) else { return };
        let tl = self.a_cuadro(s - cl.inicio).clamp(0.0, cl.dur);
        let aqui = an::momentos(cl).into_iter().find(|m| (m.0 - tl).abs() < 0.02).map(|m| m.1).unwrap_or_default();
        ui.label(RichText::new(tf("Llave en {}", &[&tiempo(cl.inicio + tl)])).color(TEXT_FAINT).size(12.5));
        for (l, n) in Llave::TODAS {
            let ya = l != Llave::Todo && aqui.iter().any(|p| Llave::de(p) == Some(l));
            let texto = if ya { tf("Quitar llave de {}", &[&t(n)]) } else { tf("Poner llave de {}", &[&t(n)]) };
            if ui.button(RichText::new(format!("◆ {texto}")).color(color((l != Llave::Todo).then_some(l)))).clicked() {
                if let Some(c) = self.p.clip_video(id) {
                    if ya { an::quitar_llaves(c, tl, Some(l)) } else { an::poner_llave(c, l, tl) }
                }
                self.clip = Some(id);
                self.ir(s);
                if !ya {
                    self.avisar(tf("Llave de {} puesta. Mueve el cursor, cambia el valor y se guarda sola la siguiente.", &[&t(n)]), false);
                }
            }
        }
        let fx: Vec<(u64, String)> = self.p.video.iter().find(|c| c.id == id).map(|c| c.fx.iter().map(|f| (f.id, t(crate::nombres::efecto(&f.tipo)).to_string())).collect()).unwrap_or_default();
        if !fx.is_empty() {
            ui.menu_button(t("Poner llave de un efecto"), |ui| {
                for (f, n) in fx {
                    if ui.button(n).clicked() {
                        if let Some(c) = self.p.clip_video(id) {
                            if let Some(x) = c.fx.iter_mut().find(|x| x.id == f) {
                                for q in x.p.values_mut().filter(|q| matches!(q.v, Valor::Num(_) | Valor::Color(_) | Valor::Punto(_))) {
                                    if q.clave_en(tl).is_none() {
                                        q.alternar_clave(tl);
                                    }
                                }
                            }
                            c.animado = true;
                        }
                        self.clip = Some(id);
                        self.ir(s);
                    }
                }
            });
        }
        if !aqui.is_empty() {
            ui.separator();
            ui.menu_button(t("Curva hasta la siguiente llave"), |ui| {
                if let (Some(cv), Some(c)) = (menu_curvas(ui), self.p.clip_video(id)) {
                    an::curva_de_momento(c, tl, cv);
                }
            });
            if ui.button(t("Quitar todas las llaves de aquí")).clicked() {
                if let Some(c) = self.p.clip_video(id) {
                    an::quitar_llaves(c, tl, None);
                }
            }
        }
    }

    /// Tamaño del clip en el cuadro: 0 completo, 1 llenar, 2 estirar.
    pub fn acomodar(&mut self, id: u64, como: usize) {
        let (w, h) = (self.p.ancho as f64, self.p.alto as f64);
        let Some(c) = self.p.clip_video(id).filter(|c| c.ancho > 0 && c.alto > 0) else { return };
        // Cuánto le falta a cada lado para llegar a la orilla cuando el clip se ve completo.
        let (kx, ky) = (w / c.ancho as f64, h / c.alto as f64);
        let k = kx.min(ky);
        let (ex, ey) = (kx / k * 100.0, ky / k * 100.0);
        let (esc, an, al) = match como {
            1 => (ex.max(ey), 100.0, 100.0),
            2 => (100.0, ex, ey),
            _ => (100.0, 100.0, 100.0),
        };
        (c.mov.escala, c.mov.ancho, c.mov.alto) = (Prop::num(esc), Prop::num(an), Prop::num(al));
        (c.mov.x, c.mov.y) = (Prop::num(0.0), Prop::num(0.0));
    }

    /// El segundo más cercano que cae en un cuadro.
    pub fn a_cuadro(&self, s: f64) -> f64 {
        let f = self.p.fps.max(1.0);
        (s * f).round() / f
    }

    /// Barra sobre la imagen: modos, grabar llaves y llave nueva.
    pub fn barra_libre(&mut self, ui: &mut Ui, r: Rect) {
        let Some(id) = self.clip.filter(|id| self.p.video.iter().any(|c| c.id == *id)) else { return };
        let abierta = self.anim.libre.abierta;
        let mut u = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(r.min + vec2(8.0, 8.0), r.max - vec2(8.0, 8.0))));
        let marco = Frame::new().fill(Color32::from_black_alpha(205)).corner_radius(9.0).inner_margin(Margin::symmetric(7, 5)).stroke(Stroke::new(1.0, Color32::from_white_alpha(28)));
        marco.show(&mut u, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(5.0, 4.0);
                if biblioteca::pastilla(ui, "◆ Animar", abierta).on_hover_text(t("Llaves, puntos y trazos sobre la imagen")).clicked() {
                    self.anim.libre.abierta = !abierta;
                    self.anim.libre.modo = Modo::Mover;
                }
                if !abierta {
                    return;
                }
                ui.separator();
                let l = &mut self.anim.libre;
                for (m, n, a) in [
                    (Modo::Mover, "Mover", "Mueve, escala y gira el clip"),
                    (Modo::Puntos, "Puntos", "Cada clic en la imagen es una posición; el cursor avanza solo. Los puntos se arrastran."),
                    (Modo::Mano, "A mano", "Dibuja el camino con el ratón, a la velocidad que quieras"),
                ] {
                    if biblioteca::pastilla(ui, n, l.modo == m).on_hover_text(t(a)).clicked() {
                        l.modo = m;
                    }
                }
                match l.modo {
                    Modo::Puntos => {
                        ui.label(RichText::new(t("cada")).color(TEXT_DIM).size(13.0));
                        ui.add(DragValue::new(&mut l.paso).range(0.04..=10.0).speed(0.02).fixed_decimals(2).suffix(" s")).on_hover_text(t("Tiempo entre un punto y el siguiente"));
                    }
                    Modo::Mano => {
                        ui.checkbox(&mut l.llenar, RichText::new(t("Que dure el clip")).size(13.0)).on_hover_text(t("El recorrido se reparte en lo que le queda al clip"));
                    }
                    Modo::Mover => {}
                }
                ui.separator();
                let on = self.anim.auto;
                if ui.add(Button::new(RichText::new(t("Grabar llaves")).size(13.5).color(if on { RECORD } else { TEXT })).fill(if on { RECORD.gamma_multiply(0.22) } else { BUTTON })).on_hover_text(t("Con esto prendido, cualquier cambio pone una llave en el cursor")).clicked() {
                    self.anim.auto = !on;
                }
                ui.menu_button(RichText::new(t("◆ Nueva llave")).size(13.5).color(MAIN), |ui| {
                    ui.set_min_width(210.0);
                    let s = self.ahora();
                    self.menu_llaves(ui, id, s);
                });
                let hay = self.p.video.iter().find(|c| c.id == id).is_some_and(|c| an::puntos(c).len() > 1);
                if hay {
                    ui.menu_button(RichText::new(t("Camino")).size(13.5), |ui| {
                        ui.set_min_width(210.0);
                        let razon = self.p.ancho as f64 / self.p.alto.max(1) as f64;
                        let Some(c) = self.p.clip_video(id) else { return };
                        let mut curvo = c.mov.x.libre && c.mov.y.libre;
                        if ui.checkbox(&mut curvo, t("Curvo, sin detenerse en los puntos")).changed() {
                            (c.mov.x.libre, c.mov.y.libre) = (curvo, curvo);
                        }
                        if ui.button(t("Velocidad pareja")).on_hover_text(t("Reparte los puntos en el tiempo según lo que recorre")).clicked() {
                            an::emparejar(c, razon);
                        }
                        if ui.button(t("Girar con el camino")).on_hover_text(t("El clip apunta hacia donde va")).clicked() {
                            an::orientar(c, razon);
                        }
                        if ui.button(t("Al revés")).on_hover_text(t("Recorre el camino del final al principio")).clicked() {
                            an::invertir(c);
                        }
                        ui.separator();
                        if ui.button(t("Borrar el camino")).clicked() {
                            let s = (c.mov.x.en(0.0), c.mov.y.en(0.0));
                            (c.mov.x, c.mov.y) = (Prop { v: s.0, ..Prop::num(0.0) }, Prop { v: s.1, ..Prop::num(0.0) });
                        }
                    });
                }
            });
        });
    }

    /// El camino del clip elegido y sus puntos, que se arrastran.
    pub fn camino(&mut self, ui: &mut Ui, d: Rect) {
        let Some(c) = self.clip.and_then(|id| self.p.video.iter().find(|c| c.id == id)) else { return };
        let (id, ini, dur) = (c.id, c.inicio, c.dur);
        let puntos = an::puntos(c);
        let ahora = self.ahora() - ini;
        let mostrar = self.anim.libre.abierta || self.espacio == Espacio::Animacion;
        if puntos.is_empty() || !mostrar {
            return;
        }
        let en = |tl: f64| d.center() + vec2(c.mov.x.en(tl).num() as f32 / 100.0 * d.width(), c.mov.y.en(tl).num() as f32 / 100.0 * d.height());
        let p = ui.painter_at(d.expand(2.0));
        if puntos.len() > 1 {
            let (a, z) = (puntos[0], puntos[puntos.len() - 1]);
            let n = ((z - a) * 40.0).clamp(40.0, 600.0) as usize;
            let v: Vec<Pos2> = (0..=n).map(|i| en(a + (z - a) * i as f64 / n as f64)).collect();
            p.add(Shape::line(v.clone(), Stroke::new(3.5, Color32::from_black_alpha(110))));
            p.add(Shape::line(v, Stroke::new(1.6, HIGHLIGHT.gamma_multiply(0.95))));
            // Una marca por cuadro: juntas es lento, separadas es rápido.
            let paso = (1.0 / self.p.fps.max(1.0)).max((z - a) / 240.0);
            let mut t0 = a;
            while t0 <= z {
                p.circle_filled(en(t0), 1.3, Color32::from_white_alpha(170));
                t0 += paso;
            }
        }
        if (0.0..=dur).contains(&ahora) {
            p.circle(en(ahora), 5.0, Color32::WHITE, Stroke::new(1.5, Color32::BLACK));
        }
        let sitios: Vec<(f64, Pos2)> = puntos.iter().map(|t| (*t, en(*t))).collect();
        let actual = c.mov.x.clave_en(ahora).or(c.mov.y.clave_en(ahora)).is_some();
        for (i, (tl, o)) in sitios.iter().enumerate() {
            let r = ui.interact(Rect::from_center_size(*o, Vec2::splat(16.0)).intersect(d.expand(10.0)), Id::new(("punto-camino", id, i)).with(ui.id()), Sense::click_and_drag());
            r.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tf("Punto {} del camino", &[&(i + 1)])));
            let sobre = r.hovered() || r.dragged();
            let es = actual && (tl - ahora).abs() < 0.02;
            pinta_rombo(&p, *o, if sobre { 8.0 } else { 6.5 }, if es { Color32::WHITE } else { MAIN }, Color32::BLACK);
            if r.drag_started() {
                self.anim.libre.punto = Some((id, *tl));
                self.ir(ini + tl);
            }
            if r.clicked() {
                self.ir(ini + tl);
            }
            r.context_menu(|ui| {
                ui.set_min_width(190.0);
                ui.label(RichText::new(tf("Punto en {}", &[&tiempo(ini + tl)])).color(TEXT_FAINT).size(12.5));
                if ui.button(t("Ir aquí")).clicked() {
                    self.ir(ini + tl);
                }
                ui.menu_button(t("Curva hasta el siguiente punto"), |ui| {
                    if let (Some(cv), Some(c)) = (menu_curvas(ui), self.p.clip_video(id)) {
                        c.mov.x.poner_curva(*tl, cv);
                        c.mov.y.poner_curva(*tl, cv);
                    }
                });
                if ui.button(t("Borrar punto")).clicked() {
                    if let Some(c) = self.p.clip_video(id) {
                        an::quitar_llaves(c, *tl, Some(Llave::Posicion));
                    }
                }
            });
            r.on_hover_text(tf("{} · arrástralo; clic derecho: más", &[&tiempo(ini + tl)]));
        }
        // El arrastre sigue al ratón aunque el orden de los puntos cambie.
        if let Some((cual, tl)) = self.anim.libre.punto {
            let (m, abajo) = ui.input(|i| (i.pointer.interact_pos(), i.pointer.primary_down()));
            if !abajo {
                self.anim.libre.punto = None;
            } else if let (Some(m), Some(c), true) = (m, self.p.clip_video(cual), cual == id) {
                an::poner_posicion(c, tl, a_por_ciento(d, m));
            }
        }
    }

    /// Los modos Puntos y A mano. `cuerpo` es la imagen.
    pub fn vista_libre(&mut self, ui: &mut Ui, d: Rect, cuerpo: &Response) {
        let Some((id, ini, dur)) = self.clip.and_then(|id| self.p.video.iter().find(|c| c.id == id)).map(|c| (c.id, c.inicio, c.dur)) else { return };
        let tl = self.a_cuadro(self.ahora() - ini).clamp(0.0, dur);
        ui.ctx().set_cursor_icon(if cuerpo.hovered() { CursorIcon::Crosshair } else { CursorIcon::Default });
        match self.anim.libre.modo {
            Modo::Mover => {}
            Modo::Puntos => {
                if let Some(m) = cuerpo.hover_pos() {
                    ui.painter_at(d).circle_stroke(m, 6.0, Stroke::new(1.5, MAIN));
                }
                if let (true, Some(m)) = (cuerpo.clicked(), cuerpo.interact_pointer_pos()) {
                    let paso = self.anim.libre.paso;
                    if let Some(c) = self.p.clip_video(id) {
                        if c.mov.x.claves.is_empty() && c.mov.y.claves.is_empty() {
                            (c.mov.x.libre, c.mov.y.libre) = (true, true);
                        }
                        an::poner_posicion(c, tl, a_por_ciento(d, m));
                    }
                    if tl + 0.02 >= dur {
                        self.avisar(t("Ya es el final del clip: alárgalo para seguir poniendo puntos"), false);
                    } else {
                        self.ir(ini + (tl + paso).min(dur));
                    }
                }
            }
            Modo::Mano => {
                let hora = ui.input(|i| i.time);
                if let (true, Some(m)) = (cuerpo.drag_started(), ui.input(|i| i.pointer.press_origin())) {
                    self.pausar();
                    self.anim.libre.trazo = Some((tl, hora, vec![(0.0, a_por_ciento(d, m))]));
                }
                let Some((t0, h0, v)) = &mut self.anim.libre.trazo else { return };
                if let (true, Some(m)) = (cuerpo.dragged(), cuerpo.interact_pointer_pos()) {
                    let q = (hora - *h0, a_por_ciento(d, m));
                    if v.last().is_none_or(|u| q.0 > u.0 + 0.004) {
                        v.push(q);
                    }
                    ui.ctx().request_repaint();
                }
                let pts: Vec<Pos2> = v.iter().map(|q| d.center() + vec2(q.1[0] as f32 / 100.0 * d.width(), q.1[1] as f32 / 100.0 * d.height())).collect();
                ui.painter_at(d).add(Shape::line(pts, Stroke::new(3.0, RECORD)));
                if cuerpo.drag_stopped() {
                    let (t0, llenar) = (*t0, self.anim.libre.llenar);
                    let v = std::mem::take(v);
                    self.anim.libre.trazo = None;
                    self.poner_trazo(id, t0, &v, llenar);
                }
            }
        }
    }

    /// Convierte lo dibujado en llaves desde el segundo `t0` del clip.
    pub fn poner_trazo(&mut self, id: u64, t0: f64, v: &[(f64, [f64; 2])], llenar: bool) {
        let Some(c) = self.p.clip_video(id) else { return };
        let (largo, queda) = (v.last().map_or(0.0, |q| q.0), c.dur - t0);
        if largo < 0.05 || queda < 0.1 {
            return;
        }
        // Más largo que el clip: se recorre más rápido para que quepa.
        let k = if llenar || largo > queda { queda / largo } else { 1.0 };
        let v: Vec<_> = v.iter().map(|q| (t0 + q.0 * k, q.1)).collect();
        an::trazar(c, &an::simplificar(&v, 0.7));
        let n = an::puntos(c).len();
        self.avisar(tf("Camino listo: {} puntos que puedes mover", &[&n]), false);
    }
}

fn a_por_ciento(d: Rect, m: Pos2) -> [f64; 2] {
    let v = (m - d.center()) / d.size() * 100.0;
    [v.x as f64, v.y as f64]
}
