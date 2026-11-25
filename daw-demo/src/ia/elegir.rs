// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! El menú para elegir modelo. La lista se le pide a la IA hasta que se abre el menú.
use super::{Ajustes, Clase, Proveedor};
use crate::idioma::t;
use crossbeam_channel::Receiver;
use eframe::egui::{Button, ComboBox, RichText, ScrollArea, TextEdit, Ui};
use std::collections::HashMap;

type Lista = Result<Vec<String>, String>;

#[derive(Default)]
pub struct Modelos {
    pide: Option<(String, Receiver<Lista>)>,
    vistos: HashMap<String, Lista>,
    filtro: String,
}

impl Modelos {
    /// Devuelve verdadero si cambió: hay que guardar los ajustes.
    pub fn ver(&mut self, ui: &mut Ui, p: &Proveedor, a: &mut Ajustes) -> bool {
        if let Some((n, r)) = &self.pide
            && let Ok(v) = r.try_recv()
        {
            self.vistos.insert(n.clone(), v);
            self.pide = None;
        }
        let nombre = |m: &str| if m.is_empty() { t("Modelo de siempre").to_string() } else { m.to_string() };
        let mut elige = None;
        ComboBox::from_id_salt("modelo de la ia").selected_text(nombre(&p.modelo)).width(120.0).show_ui(ui, |ui| {
            let claude = ["", "opus", "sonnet", "haiku"].map(String::from).to_vec();
            let lista = match p.clase {
                Clase::ClaudeCode => Ok(claude),
                _ if !p.modelos.is_empty() => Ok(p.modelos.clone()),
                _ => match self.vistos.get(&p.nombre) {
                    Some(v) => v.clone(),
                    None => {
                        if self.pide.as_ref().is_none_or(|x| x.0 != p.nombre) {
                            let (manda, recibe) = crossbeam_channel::bounded(1);
                            let (q, ctx) = (p.clone(), ui.ctx().clone());
                            std::thread::spawn(move || {
                                let _ = manda.send(super::modelos(&q));
                                ctx.request_repaint();
                            });
                            self.pide = Some((p.nombre.clone(), recibe));
                        }
                        ui.spinner();
                        return;
                    }
                },
            };
            let mut lista = match lista {
                Ok(v) => v,
                Err(e) => {
                    ui.label(RichText::new(e).size(12.0));
                    if ui.button(t("Buscar otra vez")).clicked() {
                        self.vistos.remove(&p.nombre);
                    }
                    return;
                }
            };
            if p.clase == Clase::Acp && !lista.iter().any(String::is_empty) {
                lista.insert(0, String::new());
            }
            // Algunos traen cientos.
            if lista.len() > 12 {
                ui.add(TextEdit::singleline(&mut self.filtro).hint_text(t("Buscar")).desired_width(260.0));
                let q = self.filtro.to_lowercase();
                lista.retain(|m| m.to_lowercase().contains(&q));
            }
            ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                for m in lista {
                    if ui.add(Button::selectable(m == p.modelo, nombre(&m))).clicked() {
                        elige = Some(m);
                        ui.close();
                    }
                }
            });
        });
        match elige {
            Some(m) => a.modelos.insert(p.nombre.clone(), m.clone()) != Some(m),
            None => false,
        }
    }
}
