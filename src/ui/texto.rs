//! Apartado «Texto»: todas las letras, cada una escrita con ella misma, y la ventana para agregar una letra nueva
//! (qué formatos sirven → elegir el archivo → vista previa → se guarda en la carpeta Fuentes).
use super::estilo::*;
use super::*;
use crate::titulos;

/// La ventana de agregar una letra, paso por paso.
pub enum FuenteNueva {
    Explicar,
    Vista { ruta: PathBuf, nombre: String, texto: String, tex: Option<(String, TextureHandle)> },
    Error(String),
}

const MUESTRA: &str = "Aa Bb Cc 123";

impl App {
    /// Lista de letras del catálogo. `q`: lo que se busca.
    pub(super) fn catalogo_texto(&mut self, ui: &mut Ui, q: &str) {
        let b = Button::new(RichText::new(crate::idioma::t("+  Agregar fuente")).size(15.0).color(MAIN)).min_size(vec2(ui.available_width(), 30.0));
        if ui.add(b).on_hover_text(crate::idioma::t("Agrega una fuente de tu computadora a Beethoven Craft")).clicked() {
            self.fuente_nueva = Some(FuenteNueva::Explicar);
        }
        ui.add_space(4.0);
        let elegida = self.clip.and_then(|id| self.p.clip_video(id)).and_then(|c| match &c.contenido {
            Contenido::Titulo(t) => Some(t.letra.clone()),
            _ => None,
        });
        let todas: Vec<_> = titulos::letras().into_iter().filter(|l| q.is_empty() || l.0.to_lowercase().contains(&q.to_lowercase())).collect();
        let grupos: [(&str, Vec<&(String, Option<PathBuf>)>); 3] = [
            ("Tus fuentes", todas.iter().filter(|l| l.1.as_deref().is_some_and(titulos::es_tuya)).collect()),
            ("De la app", todas.iter().filter(|l| l.1.is_none()).collect()),
            ("Del sistema", todas.iter().filter(|l| l.1.as_deref().is_some_and(|r| !titulos::es_tuya(r))).collect()),
        ];
        let mut usar = None;
        // Pocas muestras nuevas por cuadro: leer una letra del disco tarda.
        let mut hacer = 4;
        ScrollArea::vertical().id_salt("letras").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            let ancho = ui.available_width();
            for (g, v) in &grupos {
                ui.add_space(8.0);
                ui.label(RichText::new(crate::idioma::t(g).to_uppercase()).size(11.5).color(TEXT_FAINT));
                ui.add_space(2.0);
                if v.is_empty() && *g == "Tus fuentes" && q.is_empty() {
                    ui.label(RichText::new(crate::idioma::t("Todavía no agregas fuentes. Usa «+ Agregar fuente».")).size(12.5).color(TEXT_FAINT));
                }
                for (n, _) in v.iter().map(|x| (&x.0, &x.1)) {
                    // Nombre chico arriba y la muestra abajo: nunca se enciman.
                    let (r, resp) = ui.allocate_exact_size(vec2(ancho, 50.0), Sense::click_and_drag());
                    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, n.as_str()));
                    resp.dnd_set_drag_payload(biblioteca::Soltar::Letra(n.to_string()));
                    if !ui.is_rect_visible(r) {
                        continue;
                    }
                    if !self.letras_tex.contains_key(n.as_str()) && hacer > 0 {
                        hacer -= 1;
                        let t = titulos::muestra(n, MUESTRA, 22.0).map(|(w, h, q)| ui.ctx().load_texture(format!("letra-{n}"), ColorImage::from_rgba_unmultiplied([w, h], &q), TextureOptions::LINEAR));
                        self.letras_tex.insert(n.to_string(), t);
                    }
                    let p = ui.painter_at(r);
                    let on = elegida.as_deref() == Some(n.as_str());
                    let caja = r.shrink2(vec2(0.0, 1.5));
                    if on {
                        p.rect_filled(caja, 6, MAIN.gamma_multiply(0.12));
                        p.rect_stroke(caja, 6, Stroke::new(1.5, MAIN), StrokeKind::Inside);
                    } else if resp.hovered() {
                        p.rect_filled(caja, 6, c(0x333333));
                    }
                    p.text(caja.left_top() + vec2(10.0, 5.0), Align2::LEFT_TOP, n, fuente(12.0), if on { MAIN } else { TEXT_DIM });
                    let zona = Rect::from_min_max(caja.left_top() + vec2(10.0, 20.0), caja.right_bottom() - vec2(10.0, 3.0));
                    let tinta = if on { HIGHLIGHT } else { TEXT };
                    match self.letras_tex.get(n.as_str()) {
                        Some(Some(t)) => {
                            let s = t.size_vec2();
                            let k = (zona.height() / s.y).min(1.0);
                            // Si es más ancha que el renglón, se corta con un desvanecido a la derecha.
                            let img = Rect::from_min_size(pos2(zona.left(), zona.center().y - s.y * k / 2.0), s * k);
                            ui.painter_at(zona).image(t.id(), img, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tinta);
                            if img.right() > zona.right() {
                                let fin = Rect::from_min_max(pos2(zona.right() - 24.0, zona.top()), zona.max);
                                let fondo = if on { c(0x2f2c22) } else if resp.hovered() { c(0x333333) } else { WINDOW_TITLE };
                                let mut m = Mesh::default();
                                m.colored_vertex(fin.left_top(), Color32::TRANSPARENT);
                                m.colored_vertex(fin.right_top(), fondo);
                                m.colored_vertex(fin.right_bottom(), fondo);
                                m.colored_vertex(fin.left_bottom(), Color32::TRANSPARENT);
                                m.add_triangle(0, 1, 2);
                                m.add_triangle(0, 2, 3);
                                p.add(Shape::mesh(m));
                            }
                        }
                        Some(None) => {
                            p.text(zona.left_center(), Align2::LEFT_CENTER, crate::idioma::t("(no se pudo leer)"), fuente(13.0), TEXT_FAINT);
                        }
                        None => {}
                    }
                    if resp.dragged() {
                        if let Some(m) = ui.ctx().pointer_interact_pos() {
                            let capa = ui.ctx().layer_painter(LayerId::new(Order::Tooltip, Id::new("arrastrando-letra")));
                            let g = capa.layout_no_wrap(format!("T  {n}"), fuente(14.0), COLORED_DARK);
                            let caja = Rect::from_min_size(m + vec2(12.0, 6.0), g.size() + vec2(14.0, 8.0));
                            encendido(&capa, caja, 6, MAIN);
                            capa.galley(caja.min + vec2(7.0, 4.0), g, COLORED_DARK);
                        }
                    } else if resp.on_hover_text(crate::idioma::t("Arrástrala a la vista previa o a la línea de tiempo, o doble clic para ponerla en el título elegido")).double_clicked() {
                        usar = Some(n.to_string());
                    }
                }
            }
        });
        if let Some(n) = usar {
            self.usar_letra(&n);
        }
    }

    /// La letra va al título elegido; si no hay, se crea un título con ella en el cursor.
    pub fn usar_letra(&mut self, n: &str) {
        let es_titulo = |a: &App, id: u64| a.p.video.iter().any(|c| c.id == id && matches!(c.contenido, Contenido::Titulo(_)));
        let id = match self.clip.filter(|id| es_titulo(self, *id)) {
            Some(id) => id,
            None => {
                self.poner_titulo(0, self.ahora(), None);
                match self.clip {
                    Some(id) => id,
                    None => return,
                }
            }
        };
        if let Some(Contenido::Titulo(t)) = self.p.clip_video(id).map(|c| &mut c.contenido) {
            t.letra = n.to_string();
        }
        self.avisar(crate::idioma::tf("Letra: {}", &[&n]), false);
    }

    /// Un título nuevo con esa letra en el segundo `t`. `donde`: posición en la imagen (por ciento, 0 al centro); con
    /// ella, queda listo para escribir encima.
    pub fn titulo_con_letra(&mut self, n: &str, t: f64, capa: Option<usize>, donde: Option<(f64, f64)>) {
        self.poner_titulo(0, t, capa);
        let Some(id) = self.clip else { return };
        if let Some(c) = self.p.clip_video(id) {
            if let Contenido::Titulo(x) = &mut c.contenido {
                x.letra = n.to_string();
                x.texto = crate::idioma::t("Tu texto").to_string();
            }
            if let Some((x, y)) = donde {
                c.mov.x = Prop::num(x);
                c.mov.y = Prop::num(y);
            }
        }
        self.seleccion.clear();
        self.seleccion.insert(id);
        // Soltada en la imagen: se escribe ahí mismo.
        if donde.is_some() {
            self.escribiendo = Some((id, true));
        }
    }

    fn elegir_archivo_fuente(&mut self) {
        let Some(r) = rfd::FileDialog::new().add_filter(crate::idioma::t("Fuentes"), &titulos::FORMATOS).pick_file() else { return };
        self.fuente_nueva = Some(match titulos::revisar_fuente(&r) {
            Ok(nombre) => FuenteNueva::Vista { ruta: r, nombre, texto: crate::idioma::t("Tu texto aquí").to_string(), tex: None },
            Err(e) => FuenteNueva::Error(e),
        });
    }

    /// La ventana de agregar una letra.
    pub(super) fn dialogo_fuente(&mut self, ctx: &Context) {
        let Some(paso) = self.fuente_nueva.as_mut() else { return };
        #[derive(PartialEq)]
        enum Hacer {
            Nada,
            Cerrar,
            Elegir,
            Guardar,
        }
        let mut hacer = Hacer::Nada;
        Modal::new(Id::new("fuente-nueva")).show(ctx, |ui| {
            ui.set_width(560.0);
            match paso {
                FuenteNueva::Explicar => {
                    ui.heading(crate::idioma::t("Agregar una fuente"));
                    ui.add_space(6.0);
                    Frame::new().fill(SUNKEN).corner_radius(8).inner_margin(12.0).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(crate::idioma::t("Beethoven Craft acepta:")).strong().color(MAIN));
                        for x in [
                            "•  TrueType (.ttf): la más común.",
                            "•  OpenType (.otf).",
                            "•  Colecciones (.ttc): se usa la primera fuente de la colección.",
                        ] {
                            ui.label(crate::idioma::t(x));
                        }
                        ui.add_space(6.0);
                        ui.label(RichText::new(crate::idioma::t("No sirven:")).strong().color(TEXT_DIM));
                        for x in ["•  .woff y .woff2 (son para páginas web).", "•  Fuentes de mapa de bits (.fon, .fnt, .pcf, .bdf)."] {
                            ui.label(RichText::new(crate::idioma::t(x)).color(TEXT_DIM));
                        }
                    });
                    ui.add_space(6.0);
                    ui.label(RichText::new(crate::idioma::t("La fuente se copia a la carpeta Fuentes de Beethoven Craft; tu archivo no se toca. Revisa que su licencia te deje usarla en tus videos.")).size(13.0).color(TEXT_FAINT));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.add_sized([180.0, 32.0], Button::new(RichText::new(crate::idioma::t("Buscar fuente…")).color(MAIN))).clicked() {
                            hacer = Hacer::Elegir;
                        }
                        if ui.add_sized([110.0, 32.0], Button::new(crate::idioma::t("Cancelar"))).clicked() {
                            hacer = Hacer::Cerrar;
                        }
                    });
                }
                FuenteNueva::Vista { ruta, nombre, texto, tex } => {
                    ui.heading(crate::idioma::t("Así se ve tu fuente"));
                    ui.label(RichText::new(format!("{nombre}  ·  {}", ruta.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned()))).color(TEXT_DIM));
                    ui.add_space(6.0);
                    ui.add(TextEdit::singleline(texto).desired_width(f32::INFINITY).hint_text(crate::idioma::t("Escribe para probarla")));
                    let que = format!("{texto}\n");
                    if tex.as_ref().is_none_or(|t| t.0 != que) {
                        let linea1 = titulos::muestra_de_archivo(ruta, texto, 52.0);
                        let linea2 = titulos::muestra_de_archivo(ruta, "ABCDEFGHIJ abcdefghij 0123456789 ¡¿ñáé!", 24.0);
                        // Las dos líneas, una debajo de la otra.
                        let t = match (linea1, linea2) {
                            (Some(a), Some(b)) => {
                                let (w, h) = (a.0.max(b.0), a.1 + b.1 + 10);
                                let mut v = vec![0u8; w * h * 4];
                                for (img, y0) in [(&a, 0), (&b, a.1 + 10)] {
                                    for y in 0..img.1 {
                                        let o = ((y0 + y) * w) * 4;
                                        v[o..o + img.0 * 4].copy_from_slice(&img.2[y * img.0 * 4..(y + 1) * img.0 * 4]);
                                    }
                                }
                                Some(ui.ctx().load_texture("fuente-vista", ColorImage::from_rgba_unmultiplied([w, h], &v), TextureOptions::LINEAR))
                            }
                            _ => None,
                        };
                        *tex = t.map(|t| (que, t));
                    }
                    ui.add_space(6.0);
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::hover());
                    hueco(ui.painter(), r, 8, c(0x141414));
                    if let Some((_, t)) = tex {
                        let s = t.size_vec2();
                        let k = ((r.width() - 24.0) / s.x).min((r.height() - 20.0) / s.y).min(1.0);
                        let img = Rect::from_center_size(r.center(), s * k);
                        ui.painter().image(t.id(), img, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    ui.add_space(6.0);
                    ui.label(RichText::new(crate::idioma::tf("Se guardará en: {}", &[&titulos::carpeta_fuentes().display()])).size(12.5).color(TEXT_FAINT));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.add_sized([170.0, 32.0], Button::new(RichText::new(crate::idioma::t("Guardar fuente")).color(MAIN))).clicked() {
                            hacer = Hacer::Guardar;
                        }
                        if ui.add_sized([130.0, 32.0], Button::new(crate::idioma::t("Elegir otra…"))).clicked() {
                            hacer = Hacer::Elegir;
                        }
                        if ui.add_sized([110.0, 32.0], Button::new(crate::idioma::t("Cancelar"))).clicked() {
                            hacer = Hacer::Cerrar;
                        }
                    });
                }
                FuenteNueva::Error(e) => {
                    ui.heading(crate::idioma::t("Esa fuente no sirve"));
                    ui.add_space(4.0);
                    ui.colored_label(c(0xff8a80), e.as_str());
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.add_sized([150.0, 32.0], Button::new(RichText::new(crate::idioma::t("Elegir otra…")).color(MAIN))).clicked() {
                            hacer = Hacer::Elegir;
                        }
                        if ui.add_sized([110.0, 32.0], Button::new(crate::idioma::t("Cerrar"))).clicked() {
                            hacer = Hacer::Cerrar;
                        }
                    });
                }
            }
        });
        match hacer {
            Hacer::Nada => {}
            Hacer::Cerrar => self.fuente_nueva = None,
            Hacer::Elegir => self.elegir_archivo_fuente(),
            Hacer::Guardar => {
                let Some(FuenteNueva::Vista { ruta, .. }) = &self.fuente_nueva else { return };
                match titulos::instalar_fuente(ruta) {
                    Ok(n) => {
                        self.fuente_nueva = None;
                        self.lib.seccion = biblioteca::Seccion::Texto;
                        self.avisar(crate::idioma::tf("Fuente «{}» guardada: ya está en Texto y en los títulos", &[&n]), false);
                    }
                    Err(e) => self.fuente_nueva = Some(FuenteNueva::Error(e)),
                }
            }
        }
    }
}
