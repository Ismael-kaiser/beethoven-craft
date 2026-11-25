//! Inspector: los controles de lo que esté elegido.
use super::biblioteca::pastilla;
use super::estilo::*;
use super::*;
use crate::composicion::FUSIONES;
use crate::nombres;
use crate::titulos;
use craft_trans::effect as ct;
use effectcraft_effects as ec;
use effectcraft_project::ParamUi;

/// Rombo de cuadro clave.
fn rombo(ui: &mut Ui, animado: bool, aqui: bool) -> Response {
    let (r, resp) = ui.allocate_exact_size(vec2(16.0, 18.0), Sense::click());
    let (o, k) = (r.center(), 5.0);
    let pts = vec![o - vec2(0.0, k), o + vec2(k, 0.0), o + vec2(0.0, k), o - vec2(k, 0.0)];
    let col = if resp.hovered() { HIGHLIGHT } else if animado { MAIN } else { TEXT_FAINT };
    if aqui {
        ui.painter().add(Shape::convex_polygon(pts, col, Stroke::NONE));
    } else {
        ui.painter().add(Shape::closed_line(pts, Stroke::new(1.3, col)));
    }
    resp.on_hover_text(crate::idioma::t("Cuadro clave en el cursor: pon uno, mueve el cursor, cambia el valor y se anima"))
}

fn etiqueta(ui: &mut Ui, n: &str) {
    ui.allocate_ui_with_layout(vec2(118.0, 18.0), Layout::left_to_right(Align::Center), |ui| {
        ui.set_width(118.0);
        ui.add(Label::new(RichText::new(crate::idioma::t(&n)).size(13.5).color(TEXT_DIM)).truncate()).on_hover_text(n);
    });
}

/// Renglón con rombo, nombre y deslizador.
pub fn numero(ui: &mut Ui, n: &str, p: &mut Prop, t: f64, min: f64, max: f64, def: f64, u: &str) {
    if fuera(ui) {
        return;
    }
    let r = ui.horizontal(|ui| {
        if rombo(ui, !p.claves.is_empty(), p.clave_en(t).is_some()).clicked() {
            p.alternar_clave(t);
        }
        etiqueta(ui, n);
        let mut v = p.en(t).num();
        let w = (ui.available_width() - 78.0).max(40.0);
        ui.spacing_mut().slider_width = w;
        let a = ui.add(Slider::new(&mut v, min..=max).show_value(false));
        let b = ui.add(DragValue::new(&mut v).speed((max - min) / 300.0).max_decimals(2).suffix(u));
        if a.double_clicked() {
            v = def;
            p.poner(t, Valor::Num(v));
        } else if a.changed() || b.changed() {
            p.poner(t, Valor::Num(v));
        }
    });
    ALTO.set(r.response.rect.height());
}

thread_local! {
    /// Lo que mide un renglón de control.
    static ALTO: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
}

/// Si el renglón que sigue no se ve, deja su lugar sin dibujarlo: un efecto puede traer cien controles.
fn fuera(ui: &mut Ui) -> bool {
    let alto = ALTO.get();
    let r = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), alto));
    // Con holgura, para que arrastrar un control cerca de la orilla no lo suelte.
    if alto <= 0.0 || ui.is_rect_visible(r.expand2(vec2(0.0, 120.0))) {
        return false;
    }
    ui.allocate_space(vec2(1.0, alto));
    true
}

fn color(ui: &mut Ui, c: &mut [f32; 4]) -> bool {
    ui.color_edit_button_rgba_unmultiplied(c).changed()
}

impl App {
    pub fn inspector(&mut self, ui: &mut Ui) {
        let Some(id) = self.clip else {
            self.instrumento_ui(ui);
            self.panel_fx(ui);
            return;
        };
        if self.p.video.iter().any(|c| c.id == id) {
            self.inspector_video(ui, id);
        } else if let Some(q) = self.p.clip(id).cloned() {
            let b = |ui: &mut Ui, t: &str, ayuda: &str| ui.button(t).on_hover_text(ayuda).clicked();
            Frame::new().inner_margin(8.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(crate::idioma::t("Clip de audio")).size(17.0));
                    ui.add(Label::new(RichText::new(crate::idioma::t(&q.nombre)).color(TEXT_DIM).size(13.0)).truncate());
                });
                self.perillas_clip(ui, id);
                self.ruta_clip(ui, id);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(crate::idioma::t("Velocidad")).color(TEXT_DIM));
                    let mut v = q.velocidad * 100.0;
                    if ui.add(DragValue::new(&mut v).range(10.0..=1000.0).speed(0.5).suffix(" %")).on_hover_text(crate::idioma::t("Conserva el tono. Cambia también la imagen vinculada.")).changed() {
                        self.p.velocidad(id, v / 100.0);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    if b(ui, "Editar audio", "Cortar, silenciar, normalizar, reversa") {
                        self.espacio = Espacio::Audio;
                        self.pestana = Pestana::Editor;
                    }
                    if b(ui, "Dividir", "Tecla S") {
                        self.dividir();
                    }
                    if b(ui, "Separar voz", "Voz e instrumental en dos pistas nuevas; puedes deshacerlo") {
                        self.separar(q.ruta.clone(), q.inicio, Some(q.clone()), vec![]);
                    }
                    if b(ui, "Borrar", "Tecla ⌫. No borra tu archivo.") {
                        self.borrar_clip();
                    }
                });
            });
            ui.separator();
            self.panel_fx(ui);
        } else {
            self.instrumento_ui(ui);
            self.panel_fx(ui);
        }
    }

    fn inspector_video(&mut self, ui: &mut Ui, id: u64) {
        let ahora = self.ahora();
        let mut quitar_fx = None;
        let mut subir_fx = None;
        let mut borrar = false;
        let mut vel = None;
        let mut latir = false;
        // Su clase: nombre, color, cuántos clips y qué efectos comparte.
        let clase = self.p.video.iter().find(|c| c.id == id).and_then(|c| c.clase).and_then(|k| self.p.clases.iter().find(|x| x.id == k)).cloned();
        let hermanos = clase.as_ref().map_or(0, |k| self.p.video.iter().filter(|c| c.clase == Some(k.id)).count());
        let Some(c) = self.p.video.iter_mut().find(|c| c.id == id) else { return };
        let t = (ahora - c.inicio).clamp(0.0, c.dur);
        // Los controles enseñan el valor de este momento, también de lo animado (color, tamaño…).
        let base = c.clone();
        let visto = base.en_su_momento(t);
        if !base.extra.is_empty() {
            (c.contenido, c.mov.recorte) = (visto.contenido.clone(), visto.mov.recorte);
        }
        Frame::new().inner_margin(Margin::symmetric(8, 5)).show(ui, |ui| {
            ui.horizontal(|ui| {
                let n = match &c.contenido {
                    Contenido::Archivo => "Clip de video",
                    Contenido::Imagen => "Imagen",
                    Contenido::Titulo(_) => "Título",
                    Contenido::Color(_) => "Fondo de color",
                    Contenido::Ajuste => "Clip vacío",
                    Contenido::Forma(_) => "Figura",
                };
                ui.label(RichText::new(crate::idioma::t(&n)).size(17.0));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    borrar = boton_color(ui, &tabler::X, false, "Borrar clip (⌫)", TEXT_DIM, 24.0).clicked();
                });
            });
        });
        ScrollArea::vertical().id_salt("inspector").auto_shrink([false, false]).show(ui, |ui| {
            Frame::new().inner_margin(Margin::symmetric(8, 0)).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                if let Contenido::Titulo(x) = &mut c.contenido {
                    ui.add(TextEdit::multiline(&mut x.texto).desired_rows(2).desired_width(f32::INFINITY).font(fuente(16.0)));
                    ui.horizontal(|ui| {
                        ComboBox::from_id_salt("letra").selected_text(&x.letra).width(150.0).show_ui(ui, |ui| {
                            for (n, _) in titulos::letras() {
                                ui.selectable_value(&mut x.letra, n.clone(), &n);
                            }
                        });
                        for (i, n) in [(-1, "Izq"), (0, "Centro"), (1, "Der")] {
                            if pastilla(ui, n, x.alinear == i).clicked() {
                                x.alinear = i;
                            }
                        }
                    });
                    ui.add(Slider::new(&mut x.tam, 2.0..=40.0).text("Tamaño"));
                    ui.add(Slider::new(&mut x.borde, 0.0..=2.5).text("Borde"));
                    ui.add(Slider::new(&mut x.sombra, 0.0..=1.0).text("Sombra"));
                    ui.horizontal(|ui| {
                        for (n, q) in [("Letra", &mut x.color), ("Borde", &mut x.color_borde), ("Fondo", &mut x.fondo)] {
                            ui.label(RichText::new(crate::idioma::t(&n)).color(TEXT_DIM));
                            color(ui, q);
                        }
                    });
                    ui.separator();
                }
                if let Contenido::Forma(f) = &mut c.contenido {
                    ui.horizontal(|ui| {
                        ComboBox::from_id_salt("forma").selected_text(crate::idioma::t(TipoForma::TODOS.iter().find(|x| x.0 == f.tipo).map_or("", |x| x.1))).show_ui(ui, |ui| {
                            for (v, n) in TipoForma::TODOS {
                                ui.selectable_value(&mut f.tipo, v, crate::idioma::t(n));
                            }
                        });
                        color(ui, &mut f.color);
                    });
                    ui.add(Slider::new(&mut f.tam[0], 0.5..=200.0).text(crate::idioma::t("Ancho")));
                    ui.add(Slider::new(&mut f.tam[1], 0.5..=200.0).text(crate::idioma::t("Alto")));
                    ui.add(Slider::new(&mut f.linea, 0.0..=20.0).text(crate::idioma::t("Contorno (0 = rellena)")));
                    if f.tipo == TipoForma::Rectangulo {
                        ui.add(Slider::new(&mut f.extra, 0.0..=100.0).text(crate::idioma::t("Esquinas redondas")));
                    } else if f.tipo == TipoForma::Estrella {
                        ui.add(Slider::new(&mut f.extra, 0.0..=100.0).text(crate::idioma::t("Puntas")));
                    }
                    ui.separator();
                }
                if let Contenido::Color(q) = &mut c.contenido {
                    ui.horizontal(|ui| {
                        ui.label(crate::idioma::t("Color"));
                        color(ui, q);
                    });
                    ui.separator();
                }
                CollapsingHeader::new("Movimiento").default_open(true).show(ui, |ui| {
                    let m = &mut c.mov;
                    numero(ui, "Posición X", &mut m.x, t, -100.0, 100.0, 0.0, " %");
                    numero(ui, "Posición Y", &mut m.y, t, -100.0, 100.0, 0.0, " %");
                    numero(ui, "Escala", &mut m.escala, t, 0.0, 300.0, 100.0, " %");
                    numero(ui, "Ancho", &mut m.ancho, t, -200.0, 300.0, 100.0, " %");
                    numero(ui, "Alto", &mut m.alto, t, -200.0, 300.0, 100.0, " %");
                    numero(ui, "Giro", &mut m.giro, t, -180.0, 180.0, 0.0, "°");
                    numero(ui, "Opacidad", &mut m.opacidad, t, 0.0, 100.0, 100.0, " %");
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        etiqueta(ui, "Punto de apoyo");
                        for (v, n) in m.ancla.iter_mut().zip(["X ", "Y "]) {
                            ui.add(DragValue::new(v).range(-100.0..=100.0).speed(0.3).prefix(n).suffix("%").max_decimals(1)).on_hover_text(crate::idioma::t("Alrededor de dónde gira y crece. 0 es el centro; -50 y 50 son las orillas."));
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        etiqueta(ui, "Fusión");
                        ComboBox::from_id_salt("fusion").selected_text(FUSIONES[m.fusion as usize % FUSIONES.len()].0).show_ui(ui, |ui| {
                            for (i, f) in FUSIONES.iter().enumerate() {
                                ui.selectable_value(&mut m.fusion, i as u32, f.0);
                            }
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        etiqueta(ui, "Recorte izq/arr/der/abj");
                        for r in &mut m.recorte {
                            ui.add(DragValue::new(r).range(0.0..=99.0).speed(0.2).suffix("%").max_decimals(1));
                        }
                    });
                    if c.contenido == Contenido::Archivo {
                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            etiqueta(ui, "Velocidad");
                            let mut v = m.velocidad * 100.0;
                            if ui.add(DragValue::new(&mut v).range(10.0..=1000.0).speed(0.5).suffix(" %")).changed() {
                                vel = Some(v / 100.0);
                            }
                            ui.label(RichText::new(crate::idioma::t("con su sonido")).size(12.0).color(TEXT_FAINT));
                        });
                    }
                    ui.horizontal(|ui| {
                        if ui.button(crate::idioma::t("Restablecer")).clicked() {
                            *m = Movimiento { velocidad: m.velocidad, ..Default::default() };
                        }
                        if ui.button(crate::idioma::t("Latir con la música")).on_hover_text(crate::idioma::t("Pone cuadros clave de escala en cada golpe")).clicked() {
                            latir = true;
                        }
                    });
                });
                CollapsingHeader::new("Transiciones").default_open(c.entrada.is_some() || c.salida.is_some()).show(ui, |ui| {
                    let tope = c.dur;
                    trans_ui(ui, "Al entrar", &mut c.entrada, tope);
                    trans_ui(ui, "Al salir", &mut c.salida, tope);
                });
                ui.add_space(4.0);
                ui.label(RichText::new(crate::idioma::t("Efectos de video")).size(16.0));
                if let Some(k) = &clase {
                    let [r, g, b] = k.color;
                    let col = mezcla(Color32::from_rgb(r, g, b), Color32::WHITE, 0.2);
                    Frame::new().fill(mezcla(col, c_gris(), 0.82)).stroke(Stroke::new(1.0, mezcla(col, c_gris(), 0.4))).corner_radius(6).inner_margin(6.0).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(crate::idioma::tf("● Clase «{}» · {} clips", &[&k.nombre, &hermanos])).color(col));
                        ui.label(RichText::new(crate::idioma::t("Los efectos que pongas o cambies aquí les pasan a todos.")).size(12.5).color(TEXT_DIM));
                    });
                }
                if c.fx.is_empty() {
                    ui.label(RichText::new(crate::idioma::t("Arrastra uno desde la biblioteca al clip")).color(TEXT_FAINT));
                }
                for (i, f) in c.fx.iter_mut().enumerate() {
                    Frame::new().fill(SLOT).stroke(Stroke::new(1.0, c_gris())).corner_radius(6).inner_margin(6.0).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let k = Id::new(("fxv", f.id));
                        let mut abierto = ui.data(|d| d.get_temp(k)).unwrap_or(true);
                        ui.horizontal(|ui| {
                            if led(ui, f.activo, nombres::efecto(&f.tipo)).clicked() {
                                f.activo = !f.activo;
                            }
                            if ui.add(Label::new(RichText::new(nombres::efecto(&f.tipo)).size(15.5)).sense(Sense::click())).clicked() {
                                abierto = !abierto;
                            }
                            if let Some(k) = clase.as_ref().filter(|k| k.fx.contains(&f.id)) {
                                let [r, g, b] = k.color;
                                ui.label(RichText::new("●").size(13.0).color(Color32::from_rgb(r, g, b))).on_hover_text(crate::idioma::tf("De la clase «{}»", &[&k.nombre]));
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if boton_color(ui, &tabler::X, false, "Quitar efecto", TEXT_DIM, 22.0).clicked() {
                                    quitar_fx = Some(i);
                                }
                                if boton_color(ui, &tabler::CARET_UP, false, "Subir", TEXT_DIM, 22.0).clicked() && i > 0 {
                                    subir_fx = Some(i);
                                }
                            });
                        });
                        ui.data_mut(|d| d.insert_temp(k, abierto));
                        if abierto {
                            fx_ui(ui, f, t);
                        }
                    });
                }
            });
        });
        crate::animaciones::devolver_campos(c, &visto, &base, t);
        if let Some(i) = quitar_fx {
            c.fx.remove(i);
        }
        if let Some(i) = subir_fx {
            c.fx.swap(i, i - 1);
        }
        if borrar {
            self.borrar_clip();
        }
        if let Some(v) = vel {
            self.p.velocidad(id, v);
        }
        if latir {
            self.latir(id);
        }
    }
}

fn c_gris() -> Color32 {
    c(0x222222)
}

fn trans_ui(ui: &mut Ui, n: &str, tr: &mut Option<Transicion>, tope: f64) {
    ui.horizontal(|ui| {
        etiqueta(ui, n);
        let txt = tr.as_ref().map_or("Ninguna".to_string(), |t| nombres::transicion(&t.tipo).to_string());
        ui.menu_button(txt, |ui| {
            if ui.button(crate::idioma::t("Ninguna")).clicked() {
                *tr = None;
            }
            let mut i = 0;
            let l = nombres::TRANSICIONES;
            while i < l.len() {
                let cat = l[i].0;
                let j = i + l[i..].iter().take_while(|e| e.0 == cat).count();
                ui.menu_button(crate::idioma::t(&cat), |ui| {
                    ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                        for e in &l[i..j] {
                            if ui.button(crate::idioma::t(&e.1)).clicked() {
                                let dur = tr.as_ref().map_or(1.0, |t| t.dur).min(tope / 2.0);
                                *tr = Some(Transicion { tipo: e.2.into(), dur, p: Default::default() });
                                ui.close();
                            }
                        }
                    });
                });
                i = j;
            }
        });
    });
    let Some(t) = tr else { return };
    ui.horizontal(|ui| {
        etiqueta(ui, "Duración");
        ui.add(Slider::new(&mut t.dur, 0.1..=tope.clamp(0.2, 5.0)).suffix(" s"));
    });
    let Some(d) = ct::find_transition(&t.tipo) else { return };
    for p in &d.params {
        ui.horizontal(|ui| {
            etiqueta(ui, nombres::parametro(p.label));
            let v = t.p.get(p.id).cloned();
            match (&p.kind, &p.default) {
                (ct::ParamKind::Float { soft_min, soft_max, .. }, ct::ParamValue::Float(x)) => {
                    let mut k = v.map_or(*x, |v| v.num());
                    if ui.add(Slider::new(&mut k, *soft_min..=*soft_max).max_decimals(1)).changed() {
                        t.p.insert(p.id.into(), Valor::Num(k));
                    }
                }
                (ct::ParamKind::Angle, ct::ParamValue::Float(x)) => {
                    let mut k = v.map_or(*x, |v| v.num());
                    if ui.add(DragValue::new(&mut k).suffix("°")).changed() {
                        t.p.insert(p.id.into(), Valor::Num(k));
                    }
                }
                (ct::ParamKind::Bool, ct::ParamValue::Bool(x)) => {
                    let mut k = match v {
                        Some(Valor::Si(b)) => b,
                        _ => *x,
                    };
                    if ui.checkbox(&mut k, "").changed() {
                        t.p.insert(p.id.into(), Valor::Si(k));
                    }
                }
                (ct::ParamKind::Color, ct::ParamValue::Color(x)) => {
                    let mut k = match v {
                        Some(Valor::Color(q)) => q,
                        _ => *x,
                    };
                    if color(ui, &mut k) {
                        t.p.insert(p.id.into(), Valor::Color(k));
                    }
                }
                (ct::ParamKind::Choice(o), ct::ParamValue::Choice(x)) => {
                    let mut k = match v {
                        Some(Valor::Op(q)) => q,
                        _ => *x,
                    };
                    let antes = k;
                    ComboBox::from_id_salt((p.id, n)).selected_text(nombres::opcion(o.get(k as usize).copied().unwrap_or(""))).show_ui(ui, |ui| {
                        for (i, s) in o.iter().enumerate() {
                            ui.selectable_value(&mut k, i as u32, nombres::opcion(s));
                        }
                    });
                    if k != antes {
                        t.p.insert(p.id.into(), Valor::Op(k));
                    }
                }
                _ => {}
            }
        });
    }
}

/// Controles de un efecto de video, según lo que declara EffectCraft.
pub fn fx_ui(ui: &mut Ui, f: &mut FxVideo, t: f64) {
    if let Some(d) = f.tipo.strip_prefix("fc:").and_then(ct::find_video) {
        fc_ui(ui, f, d, t);
        return;
    }
    if let Some(d) = crate::vpropios::buscar(&f.tipo) {
        for k in d.ctl {
            match k {
                crate::vpropios::Ctl::N("sombras_l" | "medios_l" | "luces_l", ..) => {}
                crate::vpropios::Ctl::N(id, n, a, b, def, u) => {
                    if let Some(p) = f.p.get_mut(*id) {
                        numero(ui, n, p, t, *a, *b, *def, u);
                    }
                }
                crate::vpropios::Ctl::O(id, n, o) => {
                    if let Some(p) = f.p.get_mut(*id) {
                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            etiqueta(ui, n);
                            let Valor::Op(mut x) = p.en(t) else { return };
                            let antes = x;
                            ComboBox::from_id_salt((f.id, *id)).selected_text(crate::idioma::t(o.get(x as usize).copied().unwrap_or(""))).show_ui(ui, |ui| {
                                for (i, s) in o.iter().enumerate() {
                                    ui.selectable_value(&mut x, i as u32, crate::idioma::t(s));
                                }
                            });
                            if x != antes {
                                p.poner(t, Valor::Op(x));
                            }
                        });
                    }
                }
                // Las tres ruedas van juntas, con su brillo abajo.
                crate::vpropios::Ctl::R("sombras", _) => ruedas(ui, f, t),
                crate::vpropios::Ctl::R(..) => {}
                crate::vpropios::Ctl::C(id, n, _) => {
                    if let Some(p) = f.p.get_mut(*id) {
                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            etiqueta(ui, n);
                            if let Valor::Color(mut x) = p.en(t) {
                                if color(ui, &mut x) {
                                    p.poner(t, Valor::Color(x));
                                }
                            }
                        });
                    }
                }
            }
        }
        return;
    }
    let Some(s) = ec::find(&f.tipo) else {
        ui.label(crate::idioma::t("Este efecto no está disponible"));
        return;
    };
    for d in &s.params {
        let Some(p) = f.p.get_mut(d.id) else { continue };
        let n = nombres::parametro(d.name);
        match (&d.ui, p.en(t)) {
            (ParamUi::Hidden | ParamUi::Mask | ParamUi::Layer | ParamUi::Path | ParamUi::Gradient, _) => {}
            (ParamUi::Slider { slider_min, slider_max, .. }, Valor::Num(_)) => {
                let def = d.default.as_f64();
                numero(ui, n, p, t, *slider_min, *slider_max, def, "")
            }
            (ParamUi::Percent, Valor::Num(_)) => numero(ui, n, p, t, 0.0, 100.0, d.default.as_f64(), " %"),
            (ParamUi::Angle, Valor::Num(_)) => numero(ui, n, p, t, -360.0, 360.0, d.default.as_f64(), "°"),
            (_, Valor::Num(v)) => {
                let k = (v.abs() * 2.0).max(100.0);
                numero(ui, n, p, t, -k, k, d.default.as_f64(), "")
            }
            (ParamUi::File { filters }, v) => {
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    etiqueta(ui, n);
                    let Valor::Texto(r) = v else { return };
                    let corto = if r.is_empty() { "Elegir…".into() } else { nombre_de(Path::new(&r)) };
                    if ui.button(corto).clicked() {
                        let ex: Vec<&str> = filters.iter().map(|s| s.as_str()).collect();
                        if let Some(r) = rfd::FileDialog::new().add_filter(crate::idioma::t("Archivo"), &ex).pick_file() {
                            p.poner(t, Valor::Texto(r.to_string_lossy().into()));
                        }
                    }
                });
            }
            (_, v) => {
                if fuera(ui) {
                    continue;
                }
                ui.horizontal(|ui| {
                    let animable = matches!(v, Valor::Punto(_) | Valor::Color(_));
                    if animable {
                        if rombo(ui, !p.claves.is_empty(), p.clave_en(t).is_some()).clicked() {
                            p.alternar_clave(t);
                        }
                    } else {
                        ui.add_space(20.0);
                    }
                    etiqueta(ui, n);
                    match v {
                        Valor::Punto(mut q) => {
                            // Los puntos van en fracción del cuadro; se muestran en por ciento.
                            let k = if matches!(d.ui, ParamUi::Point) { 100.0 } else { 1.0 };
                            let mut m = false;
                            for x in &mut q {
                                let mut y = *x * k;
                                if ui.add(DragValue::new(&mut y).speed(0.3).max_decimals(1)).changed() {
                                    *x = y / k;
                                    m = true;
                                }
                            }
                            if m {
                                p.poner(t, Valor::Punto(q));
                            }
                        }
                        Valor::Color(mut q) => {
                            if color(ui, &mut q) {
                                p.poner(t, Valor::Color(q));
                            }
                        }
                        Valor::Si(mut b) => {
                            if ui.checkbox(&mut b, "").changed() {
                                p.poner(t, Valor::Si(b));
                            }
                        }
                        Valor::Op(mut i) => {
                            if let ParamUi::Popup { options } = &d.ui {
                                let antes = i;
                                ComboBox::from_id_salt((f.id, d.id))
                                    .selected_text(nombres::opcion(options.get(i as usize).map_or("", |s| s.as_str())))
                                    .show_ui(ui, |ui| {
                                        for (j, o) in options.iter().enumerate() {
                                            ui.selectable_value(&mut i, j as u32, nombres::opcion(o));
                                        }
                                    });
                                if i != antes {
                                    p.poner(t, Valor::Op(i));
                                }
                            }
                        }
                        Valor::Texto(mut s) => {
                            if ui.text_edit_singleline(&mut s).changed() {
                                p.poner(t, Valor::Texto(s));
                            }
                        }
                        Valor::Num(_) => {}
                    }
                });
            }
        }
    }
}

/// Controles de un efecto de FilmCraft.
fn fc_ui(ui: &mut Ui, f: &mut FxVideo, d: &ct::EffectDef, t: f64) {
    for q in &d.params {
        let Some(p) = f.p.get_mut(q.id) else { continue };
        let n = nombres::parametro(q.label);
        match (&q.kind, p.en(t)) {
            (ct::ParamKind::Float { soft_min, soft_max, unit, .. }, Valor::Num(_)) => numero(ui, n, p, t, *soft_min, *soft_max, q.default.as_f64().unwrap_or(0.0), unit),
            (ct::ParamKind::Angle, Valor::Num(_)) => numero(ui, n, p, t, -360.0, 360.0, q.default.as_f64().unwrap_or(0.0), "°"),
            (k, v) => {
                if fuera(ui) {
                    continue;
                }
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    etiqueta(ui, n);
                    match v {
                        Valor::Punto(mut x) => {
                            let centro = matches!(&q.default, ct::ParamValue::Vec2(v) if v.x.is_nan());
                            let e = if centro { 100.0 } else { 1.0 };
                            let mut m = false;
                            for y in &mut x {
                                let mut z = *y * e;
                                if ui.add(DragValue::new(&mut z).speed(0.5).max_decimals(1)).changed() {
                                    *y = z / e;
                                    m = true;
                                }
                            }
                            if m {
                                p.poner(t, Valor::Punto(x));
                            }
                        }
                        Valor::Color(mut x) => {
                            if color(ui, &mut x) {
                                p.poner(t, Valor::Color(x));
                            }
                        }
                        Valor::Si(mut x) => {
                            if ui.checkbox(&mut x, "").changed() {
                                p.poner(t, Valor::Si(x));
                            }
                        }
                        Valor::Op(mut x) => {
                            if let ct::ParamKind::Choice(o) = k {
                                let antes = x;
                                ComboBox::from_id_salt((f.id, q.id)).selected_text(nombres::opcion(o.get(x as usize).copied().unwrap_or(""))).show_ui(ui, |ui| {
                                    for (i, s) in o.iter().enumerate() {
                                        ui.selectable_value(&mut x, i as u32, nombres::opcion(s));
                                    }
                                });
                                if x != antes {
                                    p.poner(t, Valor::Op(x));
                                }
                            }
                        }
                        _ => {}
                    }
                });
            }
        }
    }
}

/// Ruedas de color de sombras, medios y luces.
fn ruedas(ui: &mut Ui, f: &mut FxVideo, t: f64) {
    let lado = ((ui.available_width() - 28.0) / 3.0).clamp(64.0, 150.0);
    ui.horizontal(|ui| {
        for (k, n) in [("sombras", "Sombras"), ("medios", "Medios"), ("luces", "Luces")] {
            ui.allocate_ui_with_layout(vec2(lado, lado + 62.0), Layout::top_down(Align::Center), |ui| {
                ui.set_width(lado);
                ui.spacing_mut().item_spacing.y = 3.0;
                if let Some(p) = f.p.get_mut(k) {
                    ui.horizontal(|ui| {
                        if rombo(ui, !p.claves.is_empty(), p.clave_en(t).is_some()).clicked() {
                            p.alternar_clave(t);
                        }
                        ui.label(RichText::new(crate::idioma::t(n)).size(13.0).color(TEXT_DIM));
                    });
                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(lado - 4.0), Sense::click_and_drag());
                    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, crate::idioma::t(n)));
                    let rad = r.width() / 2.0 - 3.0;
                    rueda_color(ui.painter(), r.center(), rad);
                    let Valor::Punto(v) = p.en(t) else { return };
                    let mut v = v;
                    if resp.dragged() {
                        // Fino: medio punto por pixel, con Shift menos.
                        let k = if ui.input(|i| i.modifiers.shift) { 0.2 } else { 0.7 } / rad as f64;
                        v[0] += resp.drag_delta().x as f64 * k;
                        v[1] -= resp.drag_delta().y as f64 * k;
                        let m = v[0].hypot(v[1]).max(1.0);
                        p.poner(t, Valor::Punto([v[0] / m, v[1] / m]));
                    } else if resp.double_clicked() {
                        p.poner(t, Valor::Punto([0.0; 2]));
                    }
                    let o = r.center() + vec2(v[0] as f32, -v[1] as f32) * rad;
                    ui.painter().line_segment([r.center(), o], Stroke::new(1.0, Color32::from_white_alpha(120)));
                    ui.painter().circle(o, 5.0, Color32::WHITE, Stroke::new(1.5, Color32::BLACK));
                    resp.on_hover_text(crate::idioma::t("Arrastra hacia un color; Shift va más fino; doble clic lo quita"));
                }
                if let Some(p) = f.p.get_mut(&format!("{k}_l")) {
                    let mut v = p.en(t).num();
                    ui.spacing_mut().slider_width = lado - 8.0;
                    if ui.add(Slider::new(&mut v, -100.0..=100.0).show_value(false)).on_hover_text(crate::idioma::t("Brillo")).changed() {
                        p.poner(t, Valor::Num(v));
                    }
                }
            });
        }
    });
}

fn rueda_color(p: &Painter, o: Pos2, r: f32) {
    let mut m = Mesh::default();
    let n = 48;
    m.colored_vertex(o, c(0x6a6a6a));
    for i in 0..=n {
        let g = std::f32::consts::TAU * i as f32 / n as f32;
        // Igual que la rueda: rojo a la derecha, verde arriba a la izquierda, azul abajo a la izquierda.
        let e = |d: f32| ((g - d.to_radians()).cos() * 0.5 + 0.5).powf(0.8);
        let col = Color32::from_rgb((e(0.0) * 220.0) as u8 + 20, (e(120.0) * 220.0) as u8 + 20, (e(240.0) * 220.0) as u8 + 20);
        m.colored_vertex(o + vec2(g.cos(), -g.sin()) * r, col);
        if i > 0 {
            m.add_triangle(0, i, i + 1);
        }
    }
    p.add(Shape::mesh(m));
    p.circle_stroke(o, r, Stroke::new(2.0, c(0x1c1c1c)));
    p.line_segment([o - vec2(r, 0.0), o + vec2(r, 0.0)], Stroke::new(0.5, Color32::from_black_alpha(90)));
    p.line_segment([o - vec2(0.0, r), o + vec2(0.0, r)], Stroke::new(0.5, Color32::from_black_alpha(90)));
}
