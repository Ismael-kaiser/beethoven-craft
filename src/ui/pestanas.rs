//! Tira de pestañas como de navegador: abrir, cerrar, ordenar y sacar a su ventana.
use super::estilo::*;
use super::*;

pub const TODAS: [(Espacio, &str, &tabler::TablerIcon); 5] = [
    (Espacio::Playlist, "Playlist", &tabler::PLAYLIST),
    (Espacio::Video, "Video", &tabler::LAYOUT_ROWS),
    (Espacio::Audio, "Audio", &tabler::ADJUSTMENTS),
    (Espacio::Efectos, "Efectos", &tabler::FLASK_2),
    (Espacio::Animacion, "Animación", &tabler::KEYFRAME),
];

fn datos(e: Espacio) -> (&'static str, &'static tabler::TablerIcon) {
    TODAS.iter().find(|t| t.0 == e).map_or(("", &tabler::FILE), |t| (crate::idioma::t(t.1), t.2))
}

impl App {
    pub fn tira_pestanas(&mut self, ui: &mut Ui) {
        let alto = 34.0;
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), alto), Sense::hover());
        ui.painter().rect_filled(r, 0, c(0x1f1f1f));
        let mut x = r.left() + 8.0;
        let ancho = ((r.width() - 220.0) / self.pestanas.len().max(1) as f32).clamp(96.0, 190.0);
        let mut cerrar = None;
        let mut mover = None;
        for (i, e) in self.pestanas.clone().into_iter().enumerate() {
            let (nombre, ic) = datos(e);
            let activa = self.espacio == e;
            let t = Rect::from_min_max(pos2(x, r.top() + 5.0), pos2(x + ancho - 3.0, r.bottom() + if activa { 0.0 } else { -3.0 }));
            let resp = ui.interact(t, Id::new(("pestana", e as u8)), Sense::click_and_drag());
            resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, activa, nombre));
            let p = ui.painter();
            let fondo = if activa { TOOLBAR } else if resp.hovered() { c(0x333333) } else { c(0x282828) };
            let rad = if activa { CornerRadius { nw: 9, ne: 9, sw: 0, se: 0 } } else { CornerRadius::same(8) };
            p.rect_filled(t, rad, fondo);
            if activa {
                p.rect_filled(Rect::from_min_size(t.min + vec2(10.0, 0.0), vec2(t.width() - 20.0, 2.5)), 1, MAIN);
            }
            let tinta = if activa { TEXT } else { TEXT_DIM };
            icono(p, ic, Rect::from_center_size(pos2(t.left() + 18.0, t.center().y), Vec2::splat(15.0)), if activa { MAIN } else { TEXT_DIM });
            ui.painter_at(t.shrink2(vec2(26.0, 0.0))).text(pos2(t.left() + 32.0, t.center().y), Align2::LEFT_CENTER, nombre, fuente(15.5), tinta);
            let x_r = Rect::from_center_size(pos2(t.right() - 15.0, t.center().y), Vec2::splat(18.0));
            // Siempre queda al menos una.
            if self.pestanas.len() > 1 {
                let rx = ui.interact(x_r, Id::new(("cerrar-pestana", e as u8)), Sense::click());
                rx.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, crate::idioma::tf("Cerrar {}", &[&nombre])));
                if rx.hovered() {
                    ui.painter().circle_filled(x_r.center(), 9.0, BUTTON_HI);
                }
                if activa || resp.hovered() || rx.hovered() {
                    icono(ui.painter(), &tabler::X, x_r.shrink(4.0), if rx.hovered() { TEXT } else { TEXT_FAINT });
                }
                if rx.clicked() || resp.middle_clicked() {
                    cerrar = Some(i);
                }
            }
            if resp.clicked() || resp.drag_started() {
                self.espacio = e;
            }
            if let (true, Some(m)) = (resp.dragged(), resp.interact_pointer_pos()) {
                let j = (((m.x - r.left() - 8.0) / ancho) as usize).min(self.pestanas.len() - 1);
                if j != i {
                    mover = Some((i, j));
                }
            }
            resp.context_menu(|ui| {
                if let Some(v) = e.ventana() {
                    let t = if self.ventanas[v] { "Regresar de su ventana" } else { "Sacar a su propia ventana" };
                    if ui.button(t).clicked() {
                        self.ventanas[v] = !self.ventanas[v];
                    }
                }
                if self.pestanas.len() > 1 && ui.button(crate::idioma::t("Cerrar pestaña")).clicked() {
                    cerrar = Some(i);
                }
            });
            x += ancho;
        }
        if let Some((a, b)) = mover {
            let e = self.pestanas.remove(a);
            self.pestanas.insert(b, e);
        }
        if let Some(i) = cerrar {
            let e = self.pestanas.remove(i);
            if self.espacio == e {
                self.espacio = self.pestanas[i.min(self.pestanas.len() - 1)];
            }
        }
        // Nueva pestaña
        let mut u = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(pos2(x + 4.0, r.top() + 5.0), vec2(40.0, 26.0))));
        u.menu_button(RichText::new("+").size(18.0), |ui| {
            ui.set_min_width(190.0);
            let cerradas: Vec<_> = TODAS.iter().filter(|t| !self.pestanas.contains(&t.0)).collect();
            if cerradas.is_empty() {
                ui.label(RichText::new(crate::idioma::t("Todas están abiertas")).color(TEXT_FAINT));
            }
            for (e, n, _) in cerradas {
                if ui.button(*n).clicked() {
                    self.pestanas.push(*e);
                    self.espacio = *e;
                }
            }
        });
        // Nombre del proyecto a la derecha
        let n = self.ruta.as_ref().map_or(crate::idioma::t("Proyecto sin guardar").into(), |r| nombre_de(r));
        let n = if self.p != self.guardado { format!("{n} •") } else { n };
        ui.painter().text(r.right_center() - vec2(12.0, 0.0), Align2::RIGHT_CENTER, n, fuente(14.0), TEXT_FAINT);
    }
}
