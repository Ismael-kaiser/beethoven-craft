// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Lo que de verdad entra y sale de un efecto o instrumento, para las pantallas de su ventana. El audio solo anota si la
//! ventana está mirando, y nunca espera.
use crate::receta::Buf;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};
use std::time::Instant;

/// Muestras que se guardan.
pub const LARGO: usize = 2048;
/// Puntos de la historia, y muestras que junta cada uno.
pub const HISTORIA: usize = 256;
const JUNTA: u32 = 768;

static INICIO: LazyLock<Instant> = LazyLock::new(Instant::now);

fn ahora() -> u64 {
    INICIO.elapsed().as_micros() as u64
}

fn muchos(n: usize) -> Vec<AtomicU32> {
    (0..n).map(|_| AtomicU32::new(0)).collect()
}

pub struct Ojo {
    /// Cuadros que faltan por anotar; la ventana lo renueva mientras está abierta.
    mira: AtomicU32,
    /// Entra y sale, izquierda y derecha, de corrido.
    onda: [Vec<AtomicU32>; 4],
    pos: AtomicU32,
    /// Lo más alto de cada rato, de los mismos cuatro.
    historia: [Vec<AtomicU32>; 4],
    h_pos: AtomicU32,
    junta: [AtomicU32; 5],
    hora: AtomicU64,
}

impl Default for Ojo {
    fn default() -> Ojo {
        LazyLock::force(&INICIO);
        Ojo {
            mira: AtomicU32::new(0),
            onda: std::array::from_fn(|_| muchos(LARGO)),
            pos: AtomicU32::new(0),
            historia: std::array::from_fn(|_| muchos(HISTORIA)),
            h_pos: AtomicU32::new(0),
            junta: Default::default(),
            hora: AtomicU64::new(0),
        }
    }
}

/// Lo visto, de lo más viejo a lo más nuevo.
#[derive(Clone, Default)]
pub struct Visto {
    /// Entra y sale, los dos lados juntos.
    pub entra: Vec<f32>,
    pub sale: Vec<f32>,
    /// Lo más alto de cada rato: entra, sale a la izquierda y a la derecha.
    pub historia: [Vec<f32>; 3],
    pub suena: bool,
}

impl Ojo {
    pub fn mirando(&self) -> bool {
        self.mira.load(Relaxed) > 0
    }

    /// Hilo de audio.
    pub fn anotar(&self, entra: &[Buf; 2], sale: &[Buf; 2], n: usize) {
        let falta = self.mira.load(Relaxed);
        if falta == 0 {
            return;
        }
        self.mira.store(falta.saturating_sub(n as u32), Relaxed);
        let p = self.pos.load(Relaxed) as usize;
        let lados = [&entra[0], &entra[1], &sale[0], &sale[1]];
        for (k, b) in lados.into_iter().enumerate() {
            let mut alto = f32::from_bits(self.junta[k].load(Relaxed));
            for (j, x) in b[..n].iter().enumerate() {
                self.onda[k][(p + j) % LARGO].store(x.to_bits(), Relaxed);
                alto = alto.max(x.abs());
            }
            self.junta[k].store(alto.to_bits(), Relaxed);
        }
        self.pos.store(((p + n) % LARGO) as u32, Relaxed);
        let van = self.junta[4].load(Relaxed) + n as u32;
        if van >= JUNTA {
            let h = self.h_pos.load(Relaxed) as usize;
            for k in 0..4 {
                self.historia[k][h].store(self.junta[k].swap(0, Relaxed), Relaxed);
            }
            self.h_pos.store(((h + 1) % HISTORIA) as u32, Relaxed);
            self.junta[4].store(0, Relaxed);
        } else {
            self.junta[4].store(van, Relaxed);
        }
        self.hora.store(ahora(), Relaxed);
    }

    /// La ventana: sigue mirando, y esto es lo que hay.
    pub fn ver(&self) -> Visto {
        self.mira.store(48_000, Relaxed);
        // Si hace rato que no llega nada, calla.
        if ahora().saturating_sub(self.hora.load(Relaxed)) > 150_000 {
            return Visto::default();
        }
        let p = self.pos.load(Relaxed) as usize;
        let lee = |v: &[AtomicU32], i: usize| f32::from_bits(v[i].load(Relaxed));
        let par = |a: usize| (0..LARGO).map(|j| (lee(&self.onda[a], (p + j) % LARGO) + lee(&self.onda[a + 1], (p + j) % LARGO)) * 0.5).collect();
        let h = self.h_pos.load(Relaxed) as usize;
        let cinta = |k: usize| -> Vec<f32> { (0..HISTORIA).map(|j| lee(&self.historia[k], (h + j) % HISTORIA)).collect() };
        let entra: Vec<f32> = cinta(0).into_iter().zip(cinta(1)).map(|(a, b)| a.max(b)).collect();
        let historia = [entra, cinta(2), cinta(3)];
        let sale: Vec<f32> = par(2);
        let suena = sale.iter().any(|x| x.abs() > 1e-4) || historia.iter().any(|v| v[HISTORIA - 24..].iter().any(|x| *x > 1e-4));
        Visto { entra: par(0), sale, historia, suena }
    }
}

impl Visto {
    /// Un pedazo corto y quieto de la onda: empieza donde cruza hacia arriba, para que no tiemble. Lo que entra y sale van
    /// con el mismo corte.
    pub fn quieta(&self, puntos: usize) -> [Vec<f32>; 2] {
        const PEDAZO: usize = 1024;
        if self.sale.len() < LARGO {
            return [vec![], vec![]];
        }
        let guia = if self.entra.iter().any(|x| x.abs() > 1e-4) { &self.entra } else { &self.sale };
        let i = (1..LARGO - PEDAZO).rev().find(|i| guia[i - 1] <= 0.0 && guia[*i] > 0.0).unwrap_or(LARGO - PEDAZO);
        let corta = |v: &[f32]| (0..puntos).map(|k| v[i + k * (PEDAZO - 1) / (puntos - 1)]).collect();
        [corta(&self.entra), corta(&self.sale)]
    }
}
