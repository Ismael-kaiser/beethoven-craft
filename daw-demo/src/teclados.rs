// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Teclados MIDI. Las notas van derecho al motor, sin pasar por la ventana.
use crate::motor::Orden;
use crossbeam_channel::Sender;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

/// Lo que se tocó, para la ventana: canal, tono y fuerza (0 es soltar).
pub type Toque = (u32, u8, f32);

pub struct Teclados {
    _c: Vec<midir::MidiInputConnection<()>>,
    pub nombres: Vec<String>,
}

/// Abre todos los que haya. `canal` es el instrumento que suena.
pub fn abrir(canal: Arc<AtomicU32>, motor: Sender<Orden>, ventana: Sender<Toque>) -> Teclados {
    let mut t = Teclados { _c: vec![], nombres: vec![] };
    let Ok(lista) = midir::MidiInput::new("Beethoven DAW Demo") else { return t };
    for p in lista.ports() {
        let Ok(m) = midir::MidiInput::new("Beethoven DAW Demo") else { continue };
        let nombre = m.port_name(&p).unwrap_or_default();
        let (canal, motor, ventana) = (canal.clone(), motor.clone(), ventana.clone());
        // La tecla se suelta en el canal donde se apretó, aunque ya se haya elegido otro.
        let mut abajo = [0u32; 128];
        let c = m.connect(
            &p,
            "entrada",
            move |_, b, _| {
                let [s, tono, f] = b else { return };
                let (tono, f) = (*tono & 127, *f & 127);
                let o = match (s >> 4, f) {
                    (9, 1..) => {
                        let c = canal.load(Relaxed);
                        abajo[tono as usize] = c;
                        let fuerza = f as f32 / 127.0;
                        let _ = ventana.try_send((c, tono, fuerza));
                        Orden::Nota { canal: c, tono, fuerza }
                    }
                    (8 | 9, _) => {
                        let c = std::mem::take(&mut abajo[tono as usize]);
                        let _ = ventana.try_send((c, tono, 0.0));
                        Orden::Soltar { canal: c, tono }
                    }
                    _ => return,
                };
                let _ = motor.try_send(o);
            },
            (),
        );
        if let Ok(c) = c {
            t._c.push(c);
            t.nombres.push(nombre);
        }
    }
    t
}
