// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! La ventana de un plugin de Beethoven dentro de otro programa: la misma que en el DAW, con sus preajustes arriba.
use super::*;
use crate::instancia::{Guardado, Instancia};
use crate::interfaz::{Interfaz, Momento};

pub const ALTO_BARRA: f32 = 34.0;

pub struct Externa {
    it: Option<Interfaz>,
    /// Es la única ventana del programa: le pone la piel del DAW a todo.
    sola: bool,
}

impl Externa {
    pub fn nueva(ctx: &Context, n: &Instancia) -> Externa {
        vestir(ctx);
        Externa { it: n.interfaz(), sola: true }
    }

    /// Dentro de otro programa hecho con egui (Movie), sin cambiarle la letra ni los colores.
    pub fn dentro(n: &Instancia) -> Externa {
        Externa { it: n.interfaz(), sola: false }
    }

    /// Cuánto mide, en puntos.
    pub fn tam(&self, n: &Instancia) -> Vec2 {
        match &self.it {
            Some(i) => vec2(i.ancho + 16.0, i.alto + 16.0 + ALTO_BARRA),
            None => vec2(420.0, 120.0 + (n.receta.controles.len().div_ceil(6) as f32) * 80.0),
        }
    }

    pub fn pintar(&mut self, ui: &mut Ui, n: &Instancia) {
        let todo = ui.max_rect();
        placa(ui.painter(), todo, 0);
        let antes: Vec<f32> = (0..n.receta.controles.len()).map(|k| n.valor(k)).collect();
        let mut v: BTreeMap<String, f32> =
            n.receta.controles.iter().zip(&antes).filter(|(c, x)| c.valor != **x).map(|(c, x)| (c.clave.clone(), *x)).collect();
        let barra = Rect::from_min_size(todo.min + vec2(8.0, 4.0), vec2(todo.width() - 16.0, ALTO_BARRA - 8.0));
        let mut b = ui.new_child(UiBuilder::new().max_rect(barra).layout(Layout::left_to_right(Align::Center)));
        b.label(RichText::new("BEETHOVEN").size(13.0).color(ACENTO));
        b.label(RichText::new(n.ficha.nombre.ver()).size(14.0).color(TEXTO));
        b.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button(t("Reiniciar")).clicked() {
                v.clear();
            }
            if !n.ficha.presets.is_empty() {
                ui.menu_button(t("Preajustes"), |ui| {
                    for (nombre, x) in &n.ficha.presets {
                        if ui.button(nombre.ver()).clicked() {
                            v = x.clone();
                            ui.close();
                        }
                    }
                });
            }
        });
        let (medidas, huella, cortes, g, visto) = n.para_ver();
        let mut m = Momento {
            tiempo: ui.input(|i| i.time),
            medidas,
            huella,
            cortes,
            archivo: g.muestra.as_ref().map_or(String::new(), |r| Path::new(r).file_name().unwrap_or_default().to_string_lossy().into()),
            ..Default::default()
        };
        m.poner(&visto, n.sr);
        let dentro = Rect::from_min_max(todo.min + vec2(8.0, ALTO_BARRA), todo.max - vec2(8.0, 8.0));
        let mut h = ui.new_child(UiBuilder::new().max_rect(dentro));
        let mut activo = n.activo();
        let mut ent = lienzo::Entorno {
            activo: Some(&mut activo),
            receta: Some(&n.receta),
            sr: n.sr,
            onda: (!m.cruda.is_empty()).then(|| m.cruda.clone()),
            pads: g.pads.clone(),
            datos: Some(g.datos.clone()),
            ..Default::default()
        };
        match &mut self.it {
            Some(it) => lienzo::pintar(&mut h, it, &n.ficha, &mut v, m, &mut ent),
            None => {
                torno::bandeja(h.painter(), dentro, 8.0, Color32::from_rgb(0x2a, 0x2c, 0x30), 1.0);
                let mut h = h.new_child(UiBuilder::new().max_rect(dentro.shrink(8.0)));
                lado::controles(&mut h, &n.ficha, &mut v, ACENTO);
            }
        }
        let toca = std::mem::take(&mut ent.toca);
        let (elegir, quitar, pad) = (ent.elegir, ent.quitar, ent.pad_archivo);
        let datos = ent.datos.take().unwrap_or_default();
        toca.into_iter().for_each(|(t, si)| n.tocar(t, si));
        n.prender(activo);
        for (k, c) in n.receta.controles.iter().enumerate() {
            let x = v.get(&c.clave).copied().unwrap_or(c.valor);
            if x != antes[k] {
                n.poner(k, x);
                n.movidos.fetch_add(1, Relaxed);
            }
        }
        let audio =
            || rfd::FileDialog::new().add_filter("Audio", crate::decodificar::EXTENSIONES).pick_file().map(|r| r.to_string_lossy().to_string());
        let mut otro: Option<Guardado> = None;
        if datos != g.datos {
            otro = Some(Guardado { datos, ..g.clone() });
        }
        if elegir && let Some(r) = audio() {
            otro = Some(Guardado { muestra: Some(r), ..g.clone() });
        }
        if quitar {
            otro = Some(Guardado { muestra: None, ..g.clone() });
        }
        if let Some(i) = pad
            && let Some(r) = audio()
        {
            let mut q = g.clone();
            q.pads.resize(crate::motor::PADS, String::new());
            q.pads[i] = r;
            otro = Some(q);
        }
        if let Some(q) = otro {
            n.cambiar(q);
            n.movidos.fetch_add(1, Relaxed);
        }
        n.poner_al_dia();
        if self.sola {
            metal::metalizar(ui.ctx());
        }
    }
}
