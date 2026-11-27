// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Pinta lo que pide la interfaz de un guion y mueve sus controles.
use super::torno;
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

/// Lo que la interfaz puede tocar además de sus controles.
#[derive(Default)]
pub struct Entorno<'a> {
    pub activo: Option<&'a mut bool>,
    /// El audio de la huella; se prende `elegir` si se pide otro.
    pub elegir: bool,
    pub quitar: bool,
    pub receta: Option<&'a crate::receta::Receta>,
    pub sr: f32,
    /// Lo que suena en su pista, para el analizador.
    pub onda: Option<Vec<f32>>,
    /// Teclas que se tocan (o se sueltan) en la interfaz.
    pub toca: Vec<(u8, bool)>,
    /// Los nombres de los pads, y el que pide audio.
    pub pads: Vec<String>,
    pub pad_archivo: Option<usize>,
    pub escribir: bool,
    /// Los textos del plugin.
    pub datos: Option<BTreeMap<String, String>>,
    /// Los nombres de las pistas del Mixer.
    pub pistas: Vec<String>,
    /// Los controladores que hay, y qué controles están vinculados.
    pub controladores: Vec<((usize, usize), String)>,
    pub vinculados: Vec<String>,
    /// El vínculo que se pidió poner o quitar.
    pub vincular: Option<(String, Option<(usize, usize)>)>,
}

pub fn pintar(ui: &mut Ui, it: &mut Interfaz, f: &Ficha, v: &mut BTreeMap<String, f32>, mut m: Momento, ent: &mut Entorno) {
    let libre = ui.available_size();
    // Se agranda o encoge con la ventana, sin deformarse.
    let k = (libre.x / it.ancho).min(libre.y / it.alto).clamp(0.4, 3.0);
    let tam = vec2(it.ancho, it.alto) * k;
    // Centrado si sobra lugar a los lados.
    let (lugar, _) = ui.allocate_exact_size(vec2(libre.x.max(tam.x), tam.y), Sense::hover());
    let todo = Rect::from_min_size(pos2(lugar.left() + ((lugar.width() - tam.x) / 2.0).max(0.0), lugar.top()), tam);
    let id = ui.id().with("lienzo");
    let en = |x: f32, y: f32| todo.min + vec2(x, y) * k;
    let rect = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(en(x, y), vec2(w.max(0.0), h.max(0.0)) * k);
    let rad = |r: f32| (r * k).clamp(0.0, 80.0) as u8;
    m.raton = ui.input(|i| i.pointer.hover_pos()).filter(|o| todo.contains(*o)).map(|o| ((o - todo.min) / k).into());
    let dentro = m.raton.is_some();
    m.apretado = dentro && ui.input(|i| i.pointer.primary_down());
    m.clic = dentro && ui.input(|i| i.pointer.primary_pressed());
    let lee = |v: &BTreeMap<String, f32>, c: &crate::receta::Control| v.get(&c.clave).copied().unwrap_or(c.valor);
    let valores: BTreeMap<String, f32> = f.controles.iter().map(|c| (c.clave.clone(), lee(v, c))).collect();
    m.activo = ent.activo.as_ref().is_none_or(|x| **x);
    m.nombre = f.nombre.ver().to_string();
    m.nombres = f.controles.iter().map(|c| (c.clave.clone(), c.nombre.ver().to_string())).collect();
    let elegido = Id::new("pad-elegido");
    m.pad = ui.data(|d| d.get_temp(elegido)).unwrap_or(0);
    m.datos = ent.datos.as_ref().map_or(vec![], |d| d.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
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
    let mut valor: Option<(Pos2, String)> = None;
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
            Pieza::Hueco { x, y, w, h, r } => torno::pantalla(&p, rect(x, y, w, h), r * k, k),
            Pieza::Vidrio { x, y, w, h, r, c } => vidrio_en(&p, rect(x, y, w, h), rad(r), col(c), false, h * k * 0.12),
            Pieza::Tornillo { x, y, r } => torno::tornillo(&p, en(x, y), r * k, x * 0.7 + y),
            Pieza::Imagen { x, y, w, h, archivo, a } => {
                if let Some(t) = textura(ui.ctx(), &it.archivos, &archivo) {
                    p.image(t.id(), rect(x, y, w, h), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE.gamma_multiply(a));
                }
            }
            Pieza::Perilla { x, y, r, clave, c, m: metal } => {
                let Some(q) = control(&clave) else { continue };
                let lado = r * 2.0 * k;
                let mut h = ui.new_child(UiBuilder::new().max_rect(Rect::from_center_size(en(x, y), Vec2::splat(lado))).id_salt((id, n)));
                let mut val = lee(v, q);
                let resp = perilla_vieja(&mut h, &mut val, q.min, q.max, q.valor, col(c), lado, col(metal), k);
                if resp.changed() {
                    pone(v, q, val);
                }
                let atado = ent.vinculados.contains(&clave);
                if atado {
                    torno::foco(&p, en(x + r * 0.8, y - r * 0.8), 2.6 * k, VERDE, 1.0);
                }
                if !ent.controladores.is_empty() || atado {
                    resp.context_menu(|ui| {
                        ui.label(RichText::new(t("Moverlo con…")).size(11.0).color(TENUE));
                        for (donde, nombre) in &ent.controladores {
                            if ui.button(nombre).clicked() {
                                ent.vincular = Some((clave.clone(), Some(*donde)));
                                ui.close();
                            }
                        }
                        if atado && ui.button(t("Soltarlo")).clicked() {
                            ent.vincular = Some((clave.clone(), None));
                            ui.close();
                        }
                    });
                }
                // Al moverla dice cuánto vale.
                if resp.dragged() || resp.hovered() {
                    valor = Some((en(x, y - r) - vec2(0.0, 9.0 * k), numero(val)));
                }
            }
            Pieza::Tirador { x, y, w, h, clave, c } => {
                let Some(q) = control(&clave) else { continue };
                let r = rect(x, y, w, h);
                let resp = ui.interact(r, id.with((n, "t")), Sense::click_and_drag());
                let parado = h > w;
                let mut a = (lee(v, q) - q.min) / (q.max - q.min);
                if let Some(o) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.drag_started() || resp.clicked()) {
                    // La tapa no se sale del riel.
                    let m = 11.0 * k;
                    a = if parado { (r.bottom() - m - o.y) / (r.height() - 2.0 * m) } else { (o.x - r.left() - m) / (r.width() - 2.0 * m) }
                        .clamp(0.0, 1.0);
                    pone(v, q, q.min + (q.max - q.min) * a);
                }
                if resp.double_clicked() {
                    pone(v, q, q.valor);
                }
                torno::corredera(&p, r, a, col(c), resp.hovered() || resp.dragged(), k);
            }
            Pieza::Boton { x, y, w, h, s, clave, c } => {
                let Some(q) = control(&clave) else { continue };
                let r = rect(x, y, w, h);
                let resp = ui.interact(r, id.with((n, "b")), Sense::click());
                let si = lee(v, q) > (q.min + q.max) / 2.0;
                torno::tecla(&p, r, 5.0 * k, col(c), si, resp.hovered(), resp.is_pointer_button_down_on(), k);
                p.text(r.center(), Align2::CENTER_CENTER, s, letra(14.0 * k), if si { FONDO } else { TEXTO });
                if resp.clicked() {
                    pone(v, q, if si { q.min } else { q.max });
                }
            }
            Pieza::Acero { x, y, w, h, r, c } => torno::acero(&p, rect(x, y, w, h), r * k, col(c), k),
            Pieza::Led { x, y, r, c, v } => torno::foco(&p, en(x, y), r * k, col(c), v),
            Pieza::Bandeja { x, y, w, h, r, c } => torno::bandeja(&p, rect(x, y, w, h), r * k, col(c), k),
            Pieza::Grabado { x, y, s, n, c, donde } => {
                let a = [Align2::LEFT_CENTER, Align2::CENTER_CENTER, Align2::RIGHT_CENTER][donde as usize];
                torno::grabado(&p, en(x, y), a, &s, n * k, col(c));
            }
            Pieza::Pisoton { x, y, r, clave, c } => {
                let Some(q) = control(&clave) else { continue };
                let resp = ui.interact(Rect::from_center_size(en(x, y), Vec2::splat(r * 2.4 * k)), id.with((n, "pi")), Sense::click());
                let si = lee(v, q) > (q.min + q.max) / 2.0;
                torno::pisoton(&p, en(x, y), r * k, resp.is_pointer_button_down_on());
                torno::lampara(&p, en(x, y - r * 2.1), 4.2 * k, col(c), if si { 1.0 } else { 0.0 });
                resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, si, q.nombre.ver()));
                if resp.clicked() {
                    pone(v, q, if si { q.min } else { q.max });
                }
            }
            Pieza::Aguja { x, y, w, h, v, s } => aguja(&p, rect(x, y, w, h), v, &s, k),
            Pieza::Palanca { x, y, clave, c } => {
                let Some(q) = control(&clave) else { continue };
                let r = Rect::from_center_size(en(x, y), vec2(26.0, 44.0) * k);
                let resp = ui.interact(r, id.with((n, "pa")), Sense::click());
                let si = lee(v, q) > (q.min + q.max) / 2.0;
                torno::palanca(&p, en(x, y), si, k);
                if si {
                    torno::lampara(&p, en(x - 22.0, y), 3.6 * k, col(c), 1.0);
                }
                if resp.clicked() {
                    pone(v, q, if si { q.min } else { q.max });
                }
            }
            Pieza::Lista { x, y, w, h, clave, textos, c } => {
                let Some(q) = control(&clave) else { continue };
                let cuantos = textos.len().max(1) as f32;
                let parada = h > w;
                let elegido = (lee(v, q) - q.min).round() as usize;
                torno::bandeja(&p, rect(x, y, w, h).expand(1.5 * k), 7.0 * k, Color32::from_gray(0x12), k);
                for (i, s) in textos.iter().enumerate() {
                    let r = if parada {
                        rect(x, y + h / cuantos * i as f32, w, h / cuantos)
                    } else {
                        rect(x + w / cuantos * i as f32, y, w / cuantos, h)
                    }
                    .shrink(2.0 * k);
                    let resp = ui.interact(r, id.with((n, "l", i)), Sense::click());
                    torno::tecla(&p, r, 5.0 * k, col(c), i == elegido, resp.hovered(), resp.is_pointer_button_down_on(), k);
                    let tinta = if i == elegido { FONDO } else { TEXTO };
                    p.with_clip_rect(r.intersect(p.clip_rect())).text(r.center(), Align2::CENTER_CENTER, s, letra(13.0 * k), tinta);
                    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, i == elegido, s));
                    if resp.clicked() {
                        pone(v, q, q.min + i as f32);
                    }
                }
            }
            Pieza::Grafica { x, y, w, h, clase } => {
                // La que pide el guion o, si no dice, la de su familia.
                let cual = super::grafica::Clase::pedida(&clase).or_else(|| Some((super::grafica::Clase::de(&f.grupo.es)?, false)));
                if let (Some((c, agudo)), Some(r)) = (cual, ent.receta) {
                    super::grafica::en(ui, rect(x, y, w, h), c, r, v, ent.sr, agudo, id.with((n, "g")), k, &m);
                }
            }
            Pieza::Onda { x, y, w, h, c } => super::grafica::onda(&p, rect(x, y, w, h), &m, col(c), k),
            Pieza::Historia { x, y, w, h, c } => super::grafica::historia(&p, rect(x, y, w, h), &m, col(c), k),
            Pieza::Ecualizador { x, y, w, h } => {
                let r = rect(x, y, w, h);
                let mut dentro = ui.new_child(UiBuilder::new().max_rect(r).id_salt((id, n, "eq")));
                super::ecualizador::curva(&mut dentro, f, v, id.with((n, "curva")), r.height(), ent.onda.as_deref(), ent.sr);
            }
            Pieza::Rota { x, y, w, h, clave, textos, c } => {
                let Some(q) = control(&clave) else { continue };
                let r = rect(x, y, w, h);
                let resp = ui.interact(r, id.with((n, "ro")), Sense::click());
                let i = ((lee(v, q) - q.min).round().max(0.0) as usize).min(textos.len().max(1) - 1);
                torno::tecla(&p, r, 5.0 * k, col(c), false, resp.hovered(), resp.is_pointer_button_down_on(), k);
                let pr = p.with_clip_rect(r.intersect(p.clip_rect()));
                pr.circle_filled(r.left_center() + vec2(10.0 * k, 0.0), 3.6 * k, col(c));
                let s = textos.get(i).cloned().unwrap_or_default();
                pr.text(r.center() + vec2(6.0 * k, 0.0), Align2::CENTER_CENTER, &s, letra(11.5 * k), TEXTO);
                resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &s));
                if resp.clicked() && !textos.is_empty() {
                    pone(v, q, q.min + ((i + 1) % textos.len()) as f32);
                }
            }
            Pieza::Prender { x, y, r, c, pie } => {
                let Some(si) = ent.activo.as_deref_mut() else { continue };
                let zona = Rect::from_center_size(
                    en(x, y) - if pie { Vec2::ZERO } else { vec2(10.0, 0.0) * k },
                    if pie { Vec2::splat(r * 2.4) } else { vec2(44.0, 40.0) } * k,
                );
                let resp = ui.interact(zona, id.with((n, "on")), Sense::click());
                if pie {
                    torno::pisoton(&p, en(x, y), r * k, resp.is_pointer_button_down_on());
                    torno::lampara(&p, en(x, y - r * 2.1), 4.6 * k, col(c), if *si { 1.0 } else { 0.0 });
                } else {
                    // Palanca con su lámpara al lado.
                    torno::palanca(&p, en(x, y), *si, k);
                    torno::lampara(&p, en(x - 22.0, y), 4.2 * k, col(c), if *si { 1.0 } else { 0.0 });
                }
                resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, *si, t(if *si { "Prendido" } else { "Apagado" })));
                if resp.clicked() {
                    *si = !*si;
                }
            }
            Pieza::Archivo { x, y, w, h, s } => {
                let r = rect(x, y, w, h);
                let resp = ui.interact(r, id.with((n, "ar")), Sense::click()).on_hover_text(t("Clic derecho: quitarlo"));
                torno::tecla(&p, r, 5.0 * k, ACENTO, false, resp.hovered(), resp.is_pointer_button_down_on(), k);
                p.with_clip_rect(r.intersect(p.clip_rect())).text(r.center(), Align2::CENTER_CENTER, &s, letra(13.0 * k), TEXTO);
                resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &s));
                ent.elegir |= resp.clicked();
                ent.quitar |= resp.secondary_clicked();
            }
            Pieza::Escribir { x, y, w, h, s } => {
                let r = rect(x, y, w, h);
                let resp = ui.interact(r, id.with((n, "es")), Sense::click());
                torno::tecla(&p, r, 5.0 * k, ACENTO, false, resp.hovered(), resp.is_pointer_button_down_on(), k);
                p.with_clip_rect(r.intersect(p.clip_rect())).text(r.center(), Align2::CENTER_CENTER, &s, letra(13.0 * k), TEXTO);
                resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &s));
                ent.escribir |= resp.clicked();
            }
            Pieza::Campo { x, y, w, clave } => {
                let Some(d) = ent.datos.as_mut() else { continue };
                let memoria = id.with((n, "campo"));
                let mut s: String = ui.data(|q| q.get_temp(memoria)).unwrap_or_else(|| d.get(&clave).cloned().unwrap_or_default());
                let mut h = ui.new_child(UiBuilder::new().max_rect(rect(x, y, w, 26.0)).id_salt((id, n, "campo")));
                let resp = h.add(TextEdit::singleline(&mut s).desired_width(w * k).char_limit(400));
                // Se guarda al terminar: cada cambio vuelve a armar el sonido.
                if resp.lost_focus() {
                    d.insert(clave, s.clone());
                }
                if resp.has_focus() {
                    ui.data_mut(|q| q.insert_temp(memoria, s));
                } else {
                    ui.data_mut(|q| q.remove::<String>(memoria));
                }
            }
            Pieza::Pista { x, y, w, clave } => {
                let Some(q) = control(&clave) else { continue };
                let actual = (lee(v, q) as usize).min(ent.pistas.len().max(1) - 1);
                let mut elegida = actual;
                let nombre = |i: usize| if i == 0 { t("Ninguna").to_string() } else { ent.pistas.get(i).cloned().unwrap_or_default() };
                let mut h = ui.new_child(UiBuilder::new().max_rect(rect(x, y, w, 26.0)).id_salt((id, n, "pista")));
                ComboBox::from_id_salt((id, n)).selected_text(nombre(actual)).width(w * k).show_ui(&mut h, |ui| {
                    (0..ent.pistas.len()).for_each(|i| _ = ui.selectable_value(&mut elegida, i, nombre(i)));
                });
                if elegida != actual {
                    pone(v, q, elegida as f32);
                }
            }
            Pieza::Pads { x, y, w, h, c } => {
                let (cw, ch) = (w / 4.0, h / 4.0);
                for i in 0..16 {
                    // De abajo a arriba, como en las cajas de ritmo.
                    let r = rect(x + (i % 4) as f32 * cw, y + (3 - i / 4) as f32 * ch, cw, ch).shrink(4.0 * k);
                    let resp = ui.interact(r, id.with((n, "pad", i)), Sense::click());
                    let nombre =
                        ent.pads.get(i).filter(|x| !x.is_empty()).map(|x| Path::new(x).file_stem().unwrap_or_default().to_string_lossy().to_string());
                    let abajo = resp.is_pointer_button_down_on();
                    if abajo && ui.input(|i| i.pointer.primary_pressed()) {
                        ui.data_mut(|d| d.insert_temp(elegido, i));
                        if nombre.is_some() {
                            ent.toca.push((crate::motor::PRIMER_PAD + i as u8, true));
                        }
                    }
                    if resp.clicked() && nombre.is_none() || resp.secondary_clicked() {
                        ent.pad_archivo = Some(i);
                    }
                    torno::tecla(&p, r, 7.0 * k, col(c), abajo, resp.hovered(), abajo, k);
                    if m.pad == i {
                        p.rect_stroke(r, 7.0 * k, Stroke::new(2.0, col(c)), StrokeKind::Outside);
                    }
                    let pr = p.with_clip_rect(r.shrink(4.0 * k).intersect(p.clip_rect()));
                    pr.text(r.left_top() + vec2(8.0, 6.0) * k, Align2::LEFT_TOP, format!("{}", i + 1), letra(11.0 * k), TENUE);
                    let s = nombre.clone().unwrap_or_else(|| "+".into());
                    pr.text(r.center() + vec2(0.0, 6.0 * k), Align2::CENTER_CENTER, s, letra(12.5 * k), if abajo { FONDO } else { TEXTO });
                    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, format!("Pad {}", i + 1)));
                    resp.on_hover_text(t("Clic: tocarlo · clic derecho: elegir su audio"));
                }
            }
            Pieza::Teclado { x, y, w, h, desde, cuantas } => {
                let z = rect(x, y, w, h);
                let resp = ui.interact(z, id.with((n, "te")), Sense::click_and_drag());
                let negra = |t: u8| matches!(t % 12, 1 | 3 | 6 | 8 | 10);
                let blancas: Vec<u8> = (desde..desde + cuantas).filter(|t| !negra(*t)).collect();
                let ancho = z.width() / blancas.len() as f32;
                let caja = |t: u8| {
                    let antes = blancas.iter().filter(|b| **b < t).count() as f32;
                    if negra(t) {
                        Rect::from_min_size(pos2(z.left() + antes * ancho - ancho * 0.31, z.top()), vec2(ancho * 0.62, z.height() * 0.62))
                    } else {
                        Rect::from_min_size(pos2(z.left() + antes * ancho, z.top()), vec2(ancho, z.height()))
                    }
                };
                let o = resp.interact_pointer_pos().filter(|_| resp.is_pointer_button_down_on() || resp.dragged());
                let bajo = o.and_then(|o| {
                    let hit = |n: bool| (desde..desde + cuantas).find(|t| negra(*t) == n && caja(*t).contains(o));
                    hit(true).or_else(|| hit(false))
                });
                let memoria = id.with((n, "nota"));
                let antes: Option<u8> = ui.data(|d| d.get_temp(memoria)).flatten();
                if bajo != antes {
                    antes.iter().for_each(|t| ent.toca.push((*t, false)));
                    bajo.iter().for_each(|t| ent.toca.push((*t, true)));
                    ui.data_mut(|d| d.insert_temp(memoria, bajo));
                }
                p.rect_filled(z.expand(3.0 * k), 4.0 * k, Color32::from_gray(0x0a));
                torno::bisel(&p, z.expand(3.0 * k), 4.0 * k, 2.0 * k, true, 150.0);
                for (t, es) in (desde..desde + cuantas)
                    .map(|t| (t, negra(t)))
                    .filter(|x| !x.1)
                    .chain((desde..desde + cuantas).map(|t| (t, negra(t))).filter(|x| x.1))
                {
                    let r = caja(t).shrink2(vec2(0.6, 0.0));
                    let si = bajo == Some(t);
                    let (a, b) = match (es, si) {
                        (true, false) => (0x3a, 0x0c),
                        (true, true) => (0x22, 0x18),
                        (false, false) => (0xc9, 0xf4),
                        (false, true) => (0xa8, 0xd2),
                    };
                    degradado(&p, r, (2.0 * k) as u8, Color32::from_gray(a), Color32::from_gray(b));
                    torno::bisel(&p, r, 2.0 * k, 1.4 * k, false, if es { 120.0 } else { 90.0 });
                }
            }
            Pieza::Valvula { x, y, w, h, v } => torno::valvula(&p, rect(x, y, w, h), v, k),
            Pieza::Carrete { x, y, r, giro, lleno } => torno::carrete(&p, en(x, y), r * k, giro, lleno, k),
            Pieza::Asa { x, y, w, h } => torno::asa(&p, rect(x, y, w, h), k),
            Pieza::Madera { x, y, w, h } => torno::madera(&p, rect(x, y, w, h), 6.0 * k, k),
            Pieza::Poner { clave, v: x } => {
                if let Some(q) = control(&clave) {
                    pone(v, q, x);
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
    if let Some((o, s)) = valor {
        let g = p.layout_no_wrap(s, letra(12.0 * k), ACENTO);
        let r = Rect::from_center_size(o, g.size() + vec2(12.0, 5.0) * k);
        torno::pantalla(&p, r, 4.0 * k, k * 0.6);
        p.galley(r.center() - g.size() / 2.0, g, ACENTO);
    }
    if m.tocando {
        ui.ctx().request_repaint();
    }
}
