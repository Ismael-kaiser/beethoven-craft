// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio
// Copied into Toma Uno on 2026-11-23 from commit 349c6ba (modified: the test module was removed).

//! The stages around the bands when they run at twice the project's rate: HQ Legacy's half-band up- and downsampler
//! (the Waveshaper's stage) and HQ Enhanced's decimation filter. One channel each.
//!
// spec: plugins/fruity-parametric-eq-2.md ("Order of processing", "HQ Enhanced"), plugins/fruity-waveshaper.md
// ("Oversampling")

/// Values below this are cleared at block ends: 2^-24.
pub const TINY: f64 = 5.960464477539063e-8;

const CA: [f64; 3] = [0.1473771136010466, 0.48246854276970014, 0.8830050257693731];
const CB: [f64; 3] = [0.039151597734460045, 0.3026468483284934, 0.6746159185469639];

/// A first-order all-pass `y = x' + c * (x - y')` with its own previous input and output.
#[derive(Clone, Copy, Debug, Default)]
struct AllPass {
    xp: f64,
    yp: f64,
}

fn path(p: &mut [AllPass; 3], c: &[f64; 3], x: f64) -> f64 {
    let mut x = x;
    for (ap, &c) in p.iter_mut().zip(c) {
        let y = ap.xp + c * (x - ap.yp);
        ap.xp = x;
        ap.yp = y;
        x = y;
    }
    x
}

/// HQ Legacy's half-band stage: two paths of three all-passes.
#[derive(Clone, Copy, Debug, Default)]
pub struct HalfBand {
    a: [AllPass; 3],
    b: [AllPass; 3],
}

impl HalfBand {
    /// Up: both paths on `x`, path B's output first.
    pub fn up(&mut self, x: f32) -> [f32; 2] {
        let x = f64::from(x);
        let b = path(&mut self.b, &CB, x);
        let a = path(&mut self.a, &CA, x);
        [b as f32, a as f32]
    }

    /// Down: the first of a pair through path A, the second through path B.
    pub fn down(&mut self, first: f32, second: f32) -> f32 {
        let a = path(&mut self.a, &CA, f64::from(first));
        let b = path(&mut self.b, &CB, f64::from(second));
        ((a + b) * 0.5) as f32
    }

    /// Block end: the previous outputs below 2^-24 are cleared (the previous inputs are not touched).
    pub fn tidy(&mut self) {
        for ap in self.a.iter_mut().chain(self.b.iter_mut()) {
            if ap.yp.abs() < TINY {
                ap.yp = 0.0;
            }
        }
    }
}

pub const TAPS: usize = 176;

/// HQ Enhanced's taps: a half-band sinc centred on tap 89 under a sine window.
pub fn decimator_taps() -> [f32; TAPS] {
    std::array::from_fn(|i| {
        let d = (i as f64 - 89.0) * std::f64::consts::FRAC_PI_2;
        let c = if i == 89 { 1.0 } else { d.sin() / d };
        ((i as f64 * 0.017849958259032916).sin() * f64::from((0.5 * c) as f32)) as f32
    })
}

/// HQ Enhanced's way down: one output for every two samples at the high rate, summed in FL's order.
#[derive(Clone, Debug)]
pub struct Decimator {
    /// The last 176 samples, newest first from `pos`, written twice so they always lie in one piece.
    buf: [f32; 2 * TAPS],
    pos: usize,
    /// Outputs produced so far (never reset while the effect lives).
    count: u64,
}

impl Default for Decimator {
    fn default() -> Self {
        Decimator { buf: [0.0; 2 * TAPS], pos: 0, count: 0 }
    }
}

/// One piece of the sum: four running sums over whole groups of four, joined to `t`, then the products left over.
fn piece(h: &[f32], u: &[f32], t: f32) -> f32 {
    let mut s = [0f32; 4];
    let whole = h.len() / 4 * 4;
    for (hg, ug) in h[..whole].chunks_exact(4).zip(u[..whole].chunks_exact(4)) {
        for k in 0..4 {
            s[k] += hg[k] * ug[k];
        }
    }
    let mut t = (((s[0] + t) + s[1]) + s[2]) + s[3];
    for (&h, &u) in h[whole..].iter().zip(&u[whole..]) {
        t += h * u;
    }
    t
}

impl Decimator {
    fn push(&mut self, x: f32) {
        self.pos = if self.pos == 0 { TAPS - 1 } else { self.pos - 1 };
        self.buf[self.pos] = x;
        self.buf[self.pos + TAPS] = x;
    }

    /// Takes the samples `2n` and `2n + 1` and gives output `n`: `sum h[j] * u[2n + 1 - j]`.
    pub fn down(&mut self, h: &[f32; TAPS], first: f32, second: f32) -> f32 {
        self.push(first);
        self.push(second);
        let u = &self.buf[self.pos..self.pos + TAPS];
        let p = (self.count % 128) as usize;
        self.count += 1;
        let n1 = (2 * p + 1).min(TAPS);
        let t = piece(&h[..n1], &u[..n1], 0.0);
        if n1 < TAPS { piece(&h[n1..], &u[n1..], t) } else { t }
    }
}

