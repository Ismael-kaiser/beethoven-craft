//! Ventana de exportación completa: ajustes listos a la izquierda, la imagen y el resumen al centro, y todos los ajustes
//! (archivo, video, tramo) a la derecha.
use super::*;
use crate::exportar::{Codec, Opciones};

/// Lo que se está eligiendo en la ventana.
#[derive(Clone)]
pub struct Panel {
    pub op: Opciones,
    /// Nombre del archivo, sin extensión.
    pub nombre: String,
    pub carpeta: PathBuf,
}

impl Panel {
    pub fn nuevo(proyecto: Option<&Path>) -> Panel {
        let nombre = proyecto.and_then(|r| r.file_stem()).map_or_else(|| crate::idioma::t("Mi video").to_string(), |s| s.to_string_lossy().into_owned());
        let carpeta = carpeta().parent().map(Path::to_path_buf).unwrap_or_else(carpeta);
        Panel { op: Opciones::default(), nombre, carpeta }
    }
    fn ext(&self) -> &'static str {
        if self.op.codec == Codec::ProRes { "mov" } else { "mp4" }
    }
    fn ruta(&self) -> PathBuf {
        let limpio: String = self.nombre.trim().chars().map(|c| if "/\\:*?\"<>|".contains(c) { '-' } else { c }).collect();
        let limpio = if limpio.is_empty() { crate::idioma::t("Mi video").to_string() } else { limpio };
        self.carpeta.join(format!("{limpio}.{}", self.ext()))
    }
}

/// Un ajuste listo: nombre, para qué sirve y qué opciones pone.
struct Listo {
    nombre: &'static str,
    para: &'static str,
    tam: Tam,
    codec: Codec,
    calidad: usize,
}

#[derive(Clone, Copy)]
enum Tam {
    Proyecto,
    Alto(u32),
    Fijo(u32, u32),
}

const LISTOS: [Listo; 6] = [
    Listo { nombre: "Como el proyecto", para: "Mismo tamaño y cuadros que tu proyecto.", tam: Tam::Proyecto, codec: Codec::H264, calidad: 1 },
    Listo { nombre: "YouTube 1080p", para: "Full HD con buena calidad. Lo más común.", tam: Tam::Alto(1080), codec: Codec::H264, calidad: 2 },
    Listo { nombre: "YouTube 4K", para: "La máxima nitidez. Tarda más y pesa más.", tam: Tam::Alto(2160), codec: Codec::H264, calidad: 2 },
    Listo { nombre: "TikTok / Reels / Shorts", para: "Vertical 1080×1920 para el celular.", tam: Tam::Fijo(1080, 1920), codec: Codec::H264, calidad: 1 },
    Listo { nombre: "WhatsApp (ligero)", para: "720p y archivo chico para mandarlo rápido.", tam: Tam::Alto(720), codec: Codec::H264, calidad: 0 },
    Listo { nombre: "Para seguir editando", para: "ProRes: calidad máxima para otro programa.", tam: Tam::Proyecto, codec: Codec::ProRes, calidad: 1 },
];

/// El tamaño con alto `a` y la misma forma que el proyecto (siempre par).
fn con_alto(p: &Proyecto, a: u32) -> (u32, u32) {
    ((p.ancho as f64 * a as f64 / p.alto.max(1) as f64 / 2.0).round() as u32 * 2, a)
}

fn tam_de(p: &Proyecto, t: Tam) -> Option<(u32, u32)> {
    match t {
        Tam::Proyecto => None,
        Tam::Alto(a) => Some(con_alto(p, a)),
        Tam::Fijo(w, h) => Some((w, h)),
    }
}

fn nombre_codec(c: Codec) -> &'static str {
    match c {
        Codec::H264 => "H.264 (MP4)",
        Codec::H265 => "H.265 (MP4)",
        Codec::ProRes => "ProRes (MOV)",
    }
}

/// Cuánto pesará, en bytes (aproximado: depende mucho de cuánto se mueva la imagen).
fn peso_aprox(w: u32, h: u32, fps: f64, dur: f64, op: &Opciones) -> u64 {
    let q = op.calidad.min(2);
    // Bits por píxel de cada cuadro; H.264 medido con el codificador de la Mac, ProRes según Apple.
    let bpp = match op.codec {
        Codec::H264 => [0.03, 0.06, 0.12][q],
        Codec::H265 => [0.018, 0.036, 0.072][q],
        Codec::ProRes => [1.64, 2.36, 3.54][q],
    };
    let audio = if op.codec == Codec::ProRes { 2_304_000.0 } else { 256_000.0 };
    ((w as f64 * h as f64 * fps * bpp + audio) * dur / 8.0) as u64
}

impl App {
    /// La ventana completa de exportación de video.
    pub(super) fn dialogo_exportar(&mut self, ctx: &Context) {
        let Some(mut v) = self.exportar_op.clone() else { return };
        let (mut cerrar, mut ir) = (false, false);
        let pantalla = ctx.content_rect();
        let (an, al) = ((pantalla.width() - 60.0).clamp(760.0, 1240.0), (pantalla.height() - 60.0).clamp(520.0, 780.0));
        let p = &self.p;
        let marcado = match p.tramo {
            (Some(a), Some(b)) if b > a => Some((a, b)),
            _ => None,
        };
        Modal::new(Id::new("exportar")).frame(Frame::new().fill(WINDOW_BODY).corner_radius(12).inner_margin(0.0)).show(ctx, |ui| {
            ui.set_width(an);
            ui.set_height(al);
            ui.spacing_mut().item_spacing.y = 0.0;
            // Encabezado.
            Frame::new().fill(WINDOW_TITLE).inner_margin(Margin::symmetric(18, 12)).corner_radius(CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 }).show(ui, |ui| {
                ui.set_width(an - 36.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(crate::idioma::t("Exportar video")).size(22.0).strong().color(TEXT));
                    ui.label(RichText::new(crate::idioma::t("con tu audio")).size(15.0).color(TEXT_FAINT));
                });
            });
            let alto_cuerpo = al - 54.0 - 64.0;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                // Ajustes listos.
                Frame::new().fill(WORKSPACE).inner_margin(12.0).show(ui, |ui| {
                    ui.set_width(220.0);
                    ui.set_height(alto_cuerpo - 24.0);
                    ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
                    ui.label(RichText::new(crate::idioma::t("AJUSTES LISTOS")).size(11.5).color(TEXT_FAINT).strong());
                    ui.add_space(4.0);
                    ScrollArea::vertical().id_salt("listos").show(ui, |ui| {
                        for l in &LISTOS {
                            let tam = tam_de(p, l.tam);
                            let on = v.op.tam == tam && v.op.codec == l.codec && v.op.calidad == l.calidad && v.op.fps.is_none();
                            if tarjeta(ui, l.nombre, l.para, on).clicked() {
                                (v.op.tam, v.op.codec, v.op.calidad, v.op.fps) = (tam, l.codec, l.calidad, None);
                            }
                        }
                        let propio = !LISTOS.iter().any(|l| v.op.tam == tam_de(p, l.tam) && v.op.codec == l.codec && v.op.calidad == l.calidad && v.op.fps.is_none());
                        if propio {
                            tarjeta(ui, "Personalizado", "Tus propios ajustes, a la derecha.", true);
                        }
                    });
                    });
                });
                // Imagen y resumen.
                // Lo que queda entre las dos columnas (cada una con sus márgenes) y los márgenes del centro.
                let ancho_centro = an - (220.0 + 24.0) - (340.0 + 28.0) - 32.0;
                Frame::new().inner_margin(16.0).show(ui, |ui| {
                    ui.set_width(ancho_centro);
                    ui.set_height(alto_cuerpo - 32.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
                        let (w, h) = v.op.tam.unwrap_or((p.ancho, p.alto));
                        let caja = vec2(ancho_centro, (alto_cuerpo - 32.0 - 150.0).max(160.0));
                        let (r, _) = ui.allocate_exact_size(caja, Sense::hover());
                        ui.painter().rect_filled(r, 8, Color32::BLACK);
                        let k = (r.width() / w.max(1) as f32).min(r.height() / h.max(1) as f32);
                        let img = Rect::from_center_size(r.center(), vec2(w as f32 * k, h as f32 * k));
                        match &self.textura {
                            Some(t) => {
                                // La imagen del proyecto, recortada al centro si la salida tiene otra forma.
                                let (pw, ph) = (p.ancho.max(1) as f32, p.alto.max(1) as f32);
                                let kk = (img.width() / pw).max(img.height() / ph);
                                let (u, vv) = ((img.width() / (pw * kk)).min(1.0), (img.height() / (ph * kk)).min(1.0));
                                let uv = Rect::from_center_size(pos2(0.5, 0.5), vec2(u, vv));
                                ui.painter().image(t.id(), img, uv, Color32::WHITE);
                            }
                            None => {
                                ui.painter().rect_filled(img, 0, c(0x141414));
                                ui.painter().text(img.center(), Align2::CENTER_CENTER, crate::idioma::t("Vista previa"), fuente(15.0), TEXT_FAINT);
                            }
                        }
                        ui.painter().rect_stroke(img, 0, Stroke::new(1.0, c(0x555555)), StrokeKind::Outside);
                        ui.add_space(12.0);
                        let fps = v.op.fps.unwrap_or(p.fps);
                        let dur = v.op.tramo.map_or(p.fin(), |(a, b)| b - a);
                        let chip = cfg!(target_os = "macos");
                        let datos = [
                            ("Resolución", format!("{w}×{h}")),
                            ("Cuadros", proyecto::nombre_fps(fps)),
                            ("Duración", tiempo(dur)),
                            ("Formato", nombre_codec(v.op.codec).to_string()),
                            ("Peso aprox.", peso(peso_aprox(w, h, fps, dur, &v.op))),
                            ("Codifica", crate::idioma::t(if chip { "El chip de video" } else { "El procesador" }).to_string()),
                        ];
                        Grid::new("resumen-exp").num_columns(3).spacing([10.0, 10.0]).show(ui, |ui| {
                            for (i, (n, x)) in datos.iter().enumerate() {
                                dato(ui, n, x, (ancho_centro - 20.0) / 3.0);
                                if i % 3 == 2 {
                                    ui.end_row();
                                }
                            }
                        });
                    });
                });
                // Ajustes.
                Frame::new().fill(WINDOW_TITLE).inner_margin(14.0).show(ui, |ui| {
                    ui.set_width(340.0);
                    ui.set_height(alto_cuerpo - 28.0);
                    ScrollArea::vertical().id_salt("ajustes-exp").show(ui, |ui| {
                        ui.set_width(312.0);
                        ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
                        seccion(ui, "ARCHIVO");
                        campo(ui, "Nombre");
                        ui.horizontal(|ui| {
                            ui.add(TextEdit::singleline(&mut v.nombre).desired_width(250.0));
                            ui.label(RichText::new(format!(".{}", v.ext())).color(TEXT_FAINT));
                        });
                        campo(ui, "Carpeta");
                        ui.horizontal(|ui| {
                            let n = v.carpeta.file_name().map_or_else(|| v.carpeta.display().to_string(), |n| n.to_string_lossy().into_owned());
                            Frame::new().fill(SUNKEN).corner_radius(6).inner_margin(Margin::symmetric(8, 4)).show(ui, |ui| {
                                ui.set_width(200.0);
                                ui.add(Label::new(RichText::new(n).color(TEXT)).truncate()).on_hover_text(v.carpeta.display().to_string());
                            });
                            if ui.button(crate::idioma::t("Cambiar…")).clicked() {
                                if let Some(d) = rfd::FileDialog::new().set_directory(&v.carpeta).pick_folder() {
                                    v.carpeta = d;
                                }
                            }
                        });
                        seccion(ui, "VIDEO");
                        campo(ui, "Formato");
                        ui.horizontal_wrapped(|ui| {
                            for c in [Codec::H264, Codec::H265, Codec::ProRes] {
                                if biblioteca::pastilla(ui, nombre_codec(c), v.op.codec == c).clicked() {
                                    v.op.codec = c;
                                }
                            }
                        });
                        let que_es = match v.op.codec {
                            Codec::H264 => "Compatible con casi todo (YouTube, WhatsApp, redes). Archivo mediano.",
                            Codec::H265 => "Igual calidad con archivo más chico. Algunos equipos viejos no lo reproducen.",
                            Codec::ProRes => "Para seguir editando en otro programa. Calidad máxima, archivo muy grande.",
                        };
                        ui.label(RichText::new(crate::idioma::t(que_es)).color(TEXT_FAINT).size(12.5));
                        campo(ui, "Calidad");
                        ui.horizontal(|ui| {
                            for (i, n) in ["Ligera", "Normal", "Alta"].into_iter().enumerate() {
                                if biblioteca::pastilla(ui, n, v.op.calidad == i).clicked() {
                                    v.op.calidad = i;
                                }
                            }
                        });
                        campo(ui, "Tamaño");
                        let tams = [
                            ("Original", None),
                            ("480p", Some(con_alto(p, 480))),
                            ("720p", Some(con_alto(p, 720))),
                            ("1080p", Some(con_alto(p, 1080))),
                            ("1440p", Some(con_alto(p, 1440))),
                            ("4K", Some(con_alto(p, 2160))),
                            ("Vertical 1080×1920", Some((1080, 1920))),
                            ("Cuadrado 1080", Some((1080, 1080))),
                        ];
                        let n = tams.iter().find(|t| t.1 == v.op.tam).map_or("Original", |t| t.0);
                        ComboBox::from_id_salt("tam").width(200.0).selected_text(crate::idioma::t(n)).show_ui(ui, |ui| {
                            for (n, t) in tams {
                                ui.selectable_value(&mut v.op.tam, t, crate::idioma::t(n));
                            }
                        });
                        campo(ui, "Cuadros por segundo");
                        let propios = crate::idioma::tf("Los del proyecto ({})", &[&proyecto::nombre_fps(p.fps)]);
                        ComboBox::from_id_salt("fps-exp").width(200.0).selected_text(v.op.fps.map_or(propios.clone(), proyecto::nombre_fps)).show_ui(ui, |ui| {
                            ui.selectable_value(&mut v.op.fps, None, propios);
                            for f in proyecto::FPS {
                                ui.selectable_value(&mut v.op.fps, Some(f), proyecto::nombre_fps(f));
                            }
                        });
                        seccion(ui, "TRAMO");
                        ui.horizontal_wrapped(|ui| {
                            if biblioteca::pastilla(ui, "Todo el video", v.op.tramo.is_none()).clicked() {
                                v.op.tramo = None;
                            }
                            if let Some(t) = marcado {
                                if biblioteca::pastilla(ui, &format!("{} – {}", tiempo(t.0), tiempo(t.1)), v.op.tramo.is_some()).clicked() {
                                    v.op.tramo = Some(t);
                                }
                            }
                        });
                        if marcado.is_none() {
                            ui.label(RichText::new(crate::idioma::t("marca entrada (I) y salida (O) para exportar un pedazo")).color(TEXT_FAINT).size(12.5));
                        }
                        });
                    });
                });
            });
            // Pie.
            Frame::new().fill(WINDOW_TITLE).inner_margin(Margin::symmetric(18, 12)).corner_radius(CornerRadius { nw: 0, ne: 0, sw: 12, se: 12 }).show(ui, |ui| {
                ui.set_width(an - 36.0);
                ui.horizontal(|ui| {
                    let ruta = v.ruta();
                    let (txt, col) = if ruta.exists() {
                        (crate::idioma::t("Ya existe un archivo con ese nombre: se va a reemplazar.").to_string(), c(0xffb74d))
                    } else {
                        (ruta.display().to_string(), TEXT_FAINT)
                    };
                    ui.add(Label::new(RichText::new(txt).color(col).size(12.5)).truncate());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (r, resp) = ui.allocate_exact_size(vec2(160.0, 38.0), Sense::click());
                        let a = if resp.hovered() { HIGHLIGHT } else { mezcla(HIGHLIGHT, MAIN, 0.45) };
                        degradado(ui.painter(), r, 8, a, MAIN);
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, crate::idioma::t("Exportar"), fuente(17.0), COLORED_DARK);
                        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, crate::idioma::t("Exportar")));
                        if resp.clicked() {
                            ir = true;
                        }
                        ui.add_space(8.0);
                        if ui.add_sized([110.0, 38.0], Button::new(crate::idioma::t("Cancelar"))).clicked() {
                            cerrar = true;
                        }
                    });
                });
            });
        });
        self.exportar_op = (!cerrar && !ir).then(|| v.clone());
        if ir {
            self.exportar_video_a(v.ruta(), v.op);
        }
    }
}

/// El nombre de un ajuste, arriba de su control.
fn campo(ui: &mut Ui, n: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(crate::idioma::t(n)).size(13.0).color(TEXT_DIM));
}

fn seccion(ui: &mut Ui, n: &str) {
    ui.add_space(12.0);
    ui.separator();
    ui.label(RichText::new(crate::idioma::t(n)).size(11.5).color(TEXT_FAINT).strong());
    ui.add_space(2.0);
}

/// Un ajuste listo en la columna izquierda.
fn tarjeta(ui: &mut Ui, nombre: &str, para: &str, on: bool) -> Response {
    let (nombre, para) = (crate::idioma::t(nombre), crate::idioma::t(para));
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 54.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, nombre));
    let fondo = if on { COLORED_DARK } else if resp.hovered() { BUTTON } else { SUNKEN };
    ui.painter().rect_filled(r, 8, fondo);
    if on {
        ui.painter().rect_stroke(r, 8, Stroke::new(1.5, MAIN), StrokeKind::Inside);
    }
    let x = r.left() + 10.0;
    ui.painter().text(pos2(x, r.top() + 9.0), Align2::LEFT_TOP, nombre, fuente(14.5), if on { MAIN } else { TEXT });
    let g = ui.painter().layout(para.to_string(), fuente(11.5), TEXT_DIM, r.width() - 20.0);
    ui.painter().galley(pos2(x, r.top() + 29.0), g, TEXT_DIM);
    ui.add_space(4.0);
    resp
}

/// Un dato del resumen: título chico arriba, valor grande abajo.
fn dato(ui: &mut Ui, n: &str, x: &str, ancho: f32) {
    Frame::new().fill(SUNKEN).corner_radius(8).inner_margin(Margin::symmetric(10, 8)).show(ui, |ui| {
        ui.set_width(ancho - 20.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new(crate::idioma::t(n)).size(11.5).color(TEXT_FAINT));
            ui.add(Label::new(RichText::new(x).size(16.0).color(TEXT)).truncate());
        });
    });
}
