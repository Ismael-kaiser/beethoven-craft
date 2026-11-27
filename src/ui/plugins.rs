// Beethoven Craft. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Las ventanas de los plugins de Beethoven DAW puestos en Movie: la misma que en el DAW, con su onda en vivo.
use super::App;
use crate::modelo::Fx;
use crate::plugins::{self, Igual};
use beethoven_daw_demo::instancia::Instancia;
use beethoven_daw_demo::ui::externa::Externa;
use eframe::egui::*;
use std::sync::Arc;

/// Un plugin que suena en el motor y su ventana, si se abrió.
pub struct Abierto {
    pub n: Arc<Instancia>,
    igual: Igual,
    ventana: Option<Externa>,
}

impl Abierto {
    pub fn nuevo(n: Arc<Instancia>) -> Abierto {
        Abierto { n, igual: Igual::default(), ventana: None }
    }
}

impl App {
    fn fx_por_id(&mut self, id: u64) -> Option<&mut Fx> {
        let p = &mut self.p;
        p.pistas.iter_mut().flat_map(|t| &mut t.fx).chain(p.buses.iter_mut().flat_map(|b| &mut b.fx)).chain(&mut p.fx_maestro).find(|f| f.id == id)
    }

    /// Cada cuadro: iguala cada plugin con su efecto y dibuja las ventanas abiertas.
    pub(super) fn plugins_ui(&mut self, ctx: &Context) {
        let ids: Vec<u64> = self.plugins.keys().copied().collect();
        for id in ids {
            let mut a = self.plugins.remove(&id).unwrap();
            let Some(f) = self.fx_por_id(id) else {
                self.ventanas_plugin.remove(&id);
                continue;
            };
            plugins::igualar(f, &a.n, &mut a.igual);
            let nombre = f.nombre();
            let mut abierta = self.ventanas_plugin.contains(&id);
            if abierta {
                let n = a.n.clone();
                let v = a.ventana.get_or_insert_with(|| Externa::dentro(&n));
                let tam = v.tam(&n);
                Window::new(nombre).id(Id::new(("plugin", id))).open(&mut abierta).resizable(false).collapsible(false).show(ctx, |ui| {
                    let (r, _) = ui.allocate_exact_size(tam, Sense::hover());
                    let mut h = ui.new_child(UiBuilder::new().max_rect(r));
                    v.pintar(&mut h, &n);
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(33));
                if !abierta {
                    self.ventanas_plugin.remove(&id);
                }
            }
            self.plugins.insert(id, a);
        }
    }

    /// Lo que abrió el motor al preparar: lo toma la ventana. Si era otro plugin con el mismo efecto, su ventana se vuelve a armar.
    pub(super) fn tomar_plugins(&mut self, v: Vec<(u64, Arc<Instancia>)>) {
        for (id, n) in v {
            self.plugins.insert(id, Abierto::nuevo(n));
        }
    }
}
