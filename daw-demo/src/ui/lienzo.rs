// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Pinta lo que pide la interfaz de un guion y mueve sus controles.
use super::*;
use crate::interfaz::{Interfaz, Momento, Pieza};
use crate::lua::Ficha;

/// La imagen de la carpeta de un guion, ya en la tarjeta. Se lee una vez por carpeta y nombre.
pub fn textura(ctx: &Context, a: &crate::paquete::Archivos, nombre: &str) -> Option<TextureHandle> {
    let id = Id::new(("imagen-lua", a.id, nombre));
    if let Some(t) = ctx.data(|d| d.get_temp::<Option<TextureHandle>>(id)) {
        return t;
    }
    let t = a.imagen(nombre).and_then(|b| crate::paquete::rgba(b)).map(|(w, h, px)| {
        ctx.load_texture(format!("lua-{nombre}"), ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &px), TextureOptions::LINEAR)
    });
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
}

fn col(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

/// Devuelve si se movió algo.
pub fn pintar(ui: &mut Ui, it: &mut Interfaz, f: &Ficha, v: &mut BTreeMap<String, f32>, mut m: Momento) {
    let libre = ui.available_size();
    // Se agranda o encoge con la ventana, sin deformarse.
    let k = (libre.x / it.ancho).min(libre.y / it.alto).clamp(0.4, 3.0);
    let (todo, _) = ui.allocate_exact_size(vec2(it.ancho, it.alto) * k, Sense::hover());
    let id = ui.id().with("lienzo");
    let en = |x: f32, y: f32| todo.min + vec2(x, y) * k;
    let rect = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(en(x, y), vec2(w.max(0.0), h.max(0.0)) * k);
    let rad = |r: f32| (r * k).clamp(0.0, 80.0) as u8;
    m.raton = ui.input(|i| i.pointer.hover_pos()).filter(|o| todo.contains(*o)).map(|o| ((o - todo.min) / k).into());
    let lee = |v: &BTreeMap<String, f32>, c: &crate::receta::Control| v.get(&c.clave).copied().unwrap_or(c.valor);
    let valores: BTreeMap<String, f32> = f.controles.iter().map(|c| (c.clave.clone(), lee(v, c))).collect();
    let piezas = it.cuadro(&valores, &m);
    if let Some(e) = &it.error {
        ui.painter().text(todo.center(), Align2::CENTER_CENTER, e, letra(14.0), ROJO);
        return;
    }
    let p = ui.painter().with_clip_rect(todo.intersect(ui.clip_rect()));
    let control = |k: &str| f.controles.iter().find(|c| c.clave == k);
    let pone = |v: &mut BTreeMap<String, f32>, c: &crate::receta::Control, x: f32| {
        let x = x.clamp(c.min, c.max);
        if x == c.valor { _ = v.remove(&c.clave) } else { _ = v.insert(c.clave.clone(), x) }
    };
    for (n, z) in piezas.into_iter().enumerate() {
        match z {
            Pieza::Caja { x, y, w, h, r, a, b } => degradado(&p, rect(x, y, w, h), rad(r), col(a), col(b)),
            Pieza::Borde { x, y, w, h, r, c, g } => _ = p.rect_stroke(rect(x, y, w, h), rad(r), Stroke::new(g * k, col(c)), StrokeKind::Inside),
            Pieza::Circulo { x, y, r, c } => _ = p.circle_filled(en(x, y), (r * k).max(0.0), col(c)),
            Pieza::Aro { x, y, r, c, g } => _ = p.circle_stroke(en(x, y), (r * k).max(0.0), Stroke::new(g * k, col(c))),
            Pieza::Linea { v, c, g } => _ = p.add(Shape::line(v.iter().map(|q| en(q[0], q[1])).collect(), Stroke::new(g * k, col(c)))),
            Pieza::Poligono { v, c } => _ = p.add(Shape::convex_polygon(v.iter().map(|q| en(q[0], q[1])).collect(), col(c), Stroke::NONE)),
            Pieza::Texto { x, y, s, n, c, donde } => {
                let a = [Align2::LEFT_CENTER, Align2::CENTER_CENTER, Align2::RIGHT_CENTER][donde as usize];
                p.text(en(x, y), a, s, letra(n * k), col(c));
            }
            Pieza::Metal { x, y, w, h, r } => placa(&p, rect(x, y, w, h), rad(r)),
            Pieza::Hueco { x, y, w, h, r } => hueco(&p, rect(x, y, w, h), rad(r), HUECO),
            Pieza::Vidrio { x, y, w, h, r, c } => vidrio_en(&p, rect(x, y, w, h), rad(r), col(c), false, h * k * 0.12),
            Pieza::Tornillo { x, y, r } => tornillo(&p, en(x, y), r * k, x * 0.7 + y, false),
            Pieza::Imagen { x, y, w, h, archivo, a } => {
                if let Some(t) = textura(ui.ctx(), &it.archivos, &archivo) {
                    p.image(t.id(), rect(x, y, w, h), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE.gamma_multiply(a));
                }
            }
            Pieza::Perilla { x, y, r, clave, c } => {
                let Some(q) = control(&clave) else { continue };
                let lado = r * 2.0 * k;
                let mut h = ui.new_child(UiBuilder::new().max_rect(Rect::from_center_size(en(x, y), Vec2::splat(lado))).id_salt((id, n)));
                let mut val = lee(v, q);
                if perilla(&mut h, "", &mut val, q.min, q.max, q.valor, col(c), lado).changed() {
                    pone(v, q, val);
                }
            }
            Pieza::Tirador { x, y, w, h, clave, c } => {
                let Some(q) = control(&clave) else { continue };
                let r = rect(x, y, w, h);
                let resp = ui.interact(r, id.with((n, "t")), Sense::click_and_drag());
                let parado = h > w;
                let mut a = (lee(v, q) - q.min) / (q.max - q.min);
                if let Some(o) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.drag_started() || resp.clicked()) {
                    a = if parado { (r.bottom() - o.y) / r.height() } else { (o.x - r.left()) / r.width() }.clamp(0.0, 1.0);
                    pone(v, q, q.min + (q.max - q.min) * a);
                }
                if resp.double_clicked() {
                    pone(v, q, q.valor);
                }
                let riel = Rect::from_center_size(r.center(), if parado { vec2(6.0 * k, r.height()) } else { vec2(r.width(), 6.0 * k) });
                hueco(&p, riel, rad(3.0), HUECO);
                let (o, tapa) = if parado {
                    (pos2(r.center().x, r.bottom() - a * r.height()), vec2(r.width(), 12.0 * k))
                } else {
                    (pos2(r.left() + a * r.width(), r.center().y), vec2(12.0 * k, r.height()))
                };
                let lleno = if parado {
                    Rect::from_min_max(pos2(riel.left(), o.y), riel.max)
                } else {
                    Rect::from_min_max(riel.min, pos2(o.x, riel.bottom()))
                };
                p.rect_filled(lleno.shrink(1.0), rad(2.0), col(c));
                relieve(&p, Rect::from_center_size(o, tapa), rad(3.0), Color32::from_gray(0x6a), resp.hovered(), false);
            }
            Pieza::Boton { x, y, w, h, s, clave, c } => {
                let Some(q) = control(&clave) else { continue };
                let r = rect(x, y, w, h);
                let resp = ui.interact(r, id.with((n, "b")), Sense::click());
                let si = lee(v, q) > (q.min + q.max) / 2.0;
                if si {
                    encendido(&p, r, rad(5.0), col(c));
                } else {
                    relieve(&p, r, rad(5.0), CAJA, resp.hovered(), resp.is_pointer_button_down_on());
                }
                p.text(r.center(), Align2::CENTER_CENTER, s, letra(14.0 * k), if si { FONDO } else { TEXTO });
                if resp.clicked() {
                    pone(v, q, if si { q.min } else { q.max });
                }
            }
            Pieza::Pad { x, y, w, h, cx, cy } => {
                let (Some(a), Some(b)) = (control(&cx), control(&cy)) else { continue };
                let resp = ui.interact(rect(x, y, w, h), id.with((n, "p")), Sense::click_and_drag());
                if resp.dragged() {
                    let d = resp.drag_delta() / (vec2(w, h) * k);
                    pone(v, a, lee(v, a) + d.x * (a.max - a.min));
                    pone(v, b, lee(v, b) - d.y * (b.max - b.min));
                }
                if resp.double_clicked() {
                    pone(v, a, a.valor);
                    pone(v, b, b.valor);
                }
            }
        }
    }
    if m.tocando {
        ui.ctx().request_repaint();
    }
}
