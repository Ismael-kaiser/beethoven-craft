// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Beethoven DAW, based on Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio
// Copied into Beethoven Craft on 2026-11-23 from its `daw/` (modified: the saved state and the test module were removed).

//! Beethoven DAW's vocoder (Beethoven Vocoder, a recreation of FL's Vocodex mixer effect).
//!
//! A band-split channel vocoder: the modulator (its left and right summed) and the carrier are split by the same bank
//! of band-pass filters; each modulator band is followed by an envelope (peak with hold, or RMS over the hold time,
//! then attack and release), and the carrier's band of the same frequency is played at that envelope. The bands' sum
//! is the wet signal; the modulator and the carrier can also pass through (the modulator high-passed, the carrier
//! low-passed), and noise can be added to the carrier before the split. The carrier keeps its stereo; the modulator's
//! pass-through too.
//!
//! The band spacing, the filter shapes, the envelope laws and the level of the wet signal are Beethoven DAW's own design
//! (spec plugins/beethoven-vocoder.md §4); what the saved state's ints mean is measured (plugins/vocodex.md).
//!
// spec: plugins/vocodex.md, plugins/beethoven-vocoder.md

use std::f64::consts::{LN_2, TAU};


/// The most bands (the original's range is 5..100).
pub const MAX_BANDS: usize = 100;
/// The fewest bands.
pub const MIN_BANDS: usize = 5;
/// The steepest band filters: sections in a row.
pub const MAX_ORDER: usize = 4;

/// The level the bands' sum is played at (Beethoven DAW choice: a modulator around -12 dBFS gives about the carrier's
/// level).
pub const MAKEUP: f64 = 4.0;

/// The vocoder's settings in plain units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// Bands, 5..100.
    pub bands: usize,
    /// Band-pass sections in a row per band, 1..4 (the original's ORDER).
    pub order: usize,
    /// The lowest and the highest band's centre, in Hz.
    pub low_hz: f64,
    pub high_hz: f64,
    /// A band's width as a share of the spacing between band centres (1: neighbours cross at their -3 dB points).
    pub width: f64,
    /// Envelope times in milliseconds.
    pub attack_ms: f64,
    pub release_ms: f64,
    pub hold_ms: f64,
    /// RMS detection (the hold time is the window) instead of peaks held for the hold time.
    pub rms: bool,
    /// Linear gains: the vocoded signal, the modulator's and the carrier's pass-through, the noise added to the carrier.
    pub wet: f64,
    pub mod_level: f64,
    pub car_level: f64,
    pub noise: f64,
    /// The modulator pass-through's high-pass, the carrier pass-through's low-pass and the added noise's high-pass
    /// cutoffs, in Hz (0: no filter).
    pub mod_hp_hz: f64,
    pub car_lp_hz: f64,
    pub noise_hp_hz: f64,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            bands: 32,
            order: 2,
            low_hz: 80.0,
            high_hz: 12000.0,
            width: 1.0,
            attack_ms: 2.0,
            release_ms: 40.0,
            hold_ms: 5.0,
            rms: false,
            wet: 1.0,
            mod_level: 0.0,
            car_level: 0.0,
            noise: 0.0,
            mod_hp_hz: 5000.0,
            car_lp_hz: 500.0,
            noise_hp_hz: 0.0,
        }
    }
}

impl Params {
    /// The band centres: spaced evenly in octaves from `low_hz` to `high_hz`.
    pub fn centres(&self) -> Vec<f64> {
        let n = self.bands.clamp(MIN_BANDS, MAX_BANDS);
        let (lo, hi) = (self.low_hz.max(1.0), self.high_hz.max(self.low_hz.max(1.0) * 1.01));
        (0..n).map(|b| lo * (hi / lo).powf(b as f64 / (n - 1) as f64)).collect()
    }

    /// A band's width in octaves.
    pub fn octaves(&self) -> f64 {
        let n = self.bands.clamp(MIN_BANDS, MAX_BANDS);
        let (lo, hi) = (self.low_hz.max(1.0), self.high_hz.max(self.low_hz.max(1.0) * 1.01));
        ((hi / lo).log2() / (n - 1) as f64 * self.width).clamp(0.01, 4.0)
    }
}

/// A biquad's coefficients, normalised (`a0 = 1`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Coefs {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

impl Coefs {
    /// A band-pass with 0 dB at its centre `hz` and a width of `oct` octaves between its -3 dB points.
    fn band(hz: f64, oct: f64, rate: f64) -> Coefs {
        let w = TAU * hz / rate;
        let (s, c) = w.sin_cos();
        let alpha = s * (LN_2 / 2.0 * oct * w / s).sinh();
        let n = 1.0 / (1.0 + alpha);
        Coefs { b0: alpha * n, b1: 0.0, b2: -alpha * n, a1: -2.0 * c * n, a2: (1.0 - alpha) * n }
    }

    /// A two-pole low-pass (`high`: high-pass) at `hz`, Q 0.707; a frequency of 0 passes everything.
    fn pass(hz: f64, rate: f64, high: bool) -> Coefs {
        if hz <= 0.0 {
            return Coefs { b0: 1.0, ..Coefs::default() };
        }
        let w = TAU * hz.clamp(10.0, rate * 0.45) / rate;
        let (s, c) = w.sin_cos();
        let alpha = s / std::f64::consts::SQRT_2;
        let n = 1.0 / (1.0 + alpha);
        let (b0, b1) = if high { ((1.0 + c) / 2.0, -(1.0 + c)) } else { ((1.0 - c) / 2.0, 1.0 - c) };
        Coefs { b0: b0 * n, b1: b1 * n, b2: b0 * n, a1: -2.0 * c * n, a2: (1.0 - alpha) * n }
    }
}

/// A biquad's state (transposed direct form II).
#[derive(Clone, Copy, Debug, Default)]
struct State {
    s1: f64,
    s2: f64,
}

impl State {
    #[inline]
    fn run(&mut self, c: &Coefs, x: f64) -> f64 {
        let y = c.b0 * x + self.s1;
        self.s1 = c.b1 * x - c.a1 * y + self.s2;
        self.s2 = c.b2 * x - c.a2 * y;
        y
    }

    /// Flushes values too small to matter (no denormals in long silences).
    fn flush(&mut self) {
        if self.s1.abs() < 1e-30 {
            self.s1 = 0.0;
        }
        if self.s2.abs() < 1e-30 {
            self.s2 = 0.0;
        }
    }
}

/// One band: its filter, the filters' states for the modulator and the carrier's two channels, its envelope.
#[derive(Clone, Debug, Default)]
struct Band {
    c: Coefs,
    m: [State; MAX_ORDER],
    car: [[State; MAX_ORDER]; 2],
    /// The detector's level (the held peak, or the mean square), the samples the peak is still held for, the
    /// envelope.
    det: f64,
    held: u32,
    env: f64,
}

/// A gain that glides to its target over about 10 ms (no zipper noise on automation).
#[derive(Clone, Copy, Debug)]
struct Gain {
    v: f64,
    to: f64,
}

impl Gain {
    #[inline]
    fn next(&mut self, k: f64) -> f64 {
        self.v += (self.to - self.v) * k;
        self.v
    }
}

/// The vocoder.
pub struct Vocoder {
    rate: f64,
    p: Params,
    bands: Vec<Band>,
    order: usize,
    hp: Coefs,
    lp: Coefs,
    nhp: Coefs,
    hp_s: [State; 2],
    lp_s: [State; 2],
    nhp_s: State,
    /// One-pole steps per sample: attack, release, RMS window, gain glide; the peak hold in samples.
    ka: f64,
    kr: f64,
    kw: f64,
    kg: f64,
    hold: u32,
    gains: [Gain; 4],
    /// The noise generator (a 32-bit LCG).
    seed: u32,
}

impl Vocoder {
    pub fn new(rate: u32, p: Params) -> Vocoder {
        let mut v = Vocoder {
            rate: f64::from(rate.max(1)),
            p,
            bands: Vec::new(),
            order: 1,
            hp: Coefs::default(),
            lp: Coefs::default(),
            nhp: Coefs::default(),
            hp_s: [State::default(); 2],
            lp_s: [State::default(); 2],
            nhp_s: State::default(),
            ka: 1.0,
            kr: 1.0,
            kw: 1.0,
            kg: 1.0,
            hold: 0,
            gains: [Gain { v: 0.0, to: 0.0 }; 4],
            seed: 0x1234_5678,
        };
        v.configure();
        for g in &mut v.gains {
            g.v = g.to;
        }
        v
    }

    pub fn params(&self) -> &Params {
        &self.p
    }

    /// New settings: the filters change at once (their states are kept), the levels glide.
    pub fn set(&mut self, p: Params) {
        if p != self.p {
            self.p = p;
            self.configure();
        }
    }

    pub fn set_sample_rate(&mut self, rate: u32) {
        let rate = f64::from(rate.max(1));
        if rate != self.rate {
            self.rate = rate;
            self.configure();
            self.reset();
        }
    }

    /// Forgets what the filters and envelopes hold.
    pub fn reset(&mut self) {
        for b in &mut self.bands {
            let c = b.c;
            *b = Band { c, ..Band::default() };
        }
        self.hp_s = [State::default(); 2];
        self.lp_s = [State::default(); 2];
        self.nhp_s = State::default();
        for g in &mut self.gains {
            g.v = g.to;
        }
    }

    fn configure(&mut self) {
        let p = self.p;
        let rate = self.rate;
        let oct = p.octaves();
        let centres: Vec<f64> = p.centres().into_iter().filter(|&f| f < rate * 0.45).collect();
        self.bands.resize_with(centres.len(), Band::default);
        for (b, f) in self.bands.iter_mut().zip(centres) {
            b.c = Coefs::band(f, oct, rate);
        }
        self.order = p.order.clamp(1, MAX_ORDER);
        self.hp = Coefs::pass(p.mod_hp_hz, rate, true);
        self.lp = Coefs::pass(p.car_lp_hz, rate, false);
        self.nhp = Coefs::pass(p.noise_hp_hz, rate, true);
        let step = |ms: f64| if ms <= 0.0 { 1.0 } else { 1.0 - (-1000.0 / (ms * rate)).exp() };
        self.ka = step(p.attack_ms);
        self.kr = step(p.release_ms);
        self.kw = step(p.hold_ms.max(0.5));
        self.kg = step(10.0);
        self.hold = (p.hold_ms.max(0.0) * rate / 1000.0).round() as u32;
        // The bands' sum is louder with steeper filters' narrower skirts made up (each section narrows the band).
        let wet = p.wet * MAKEUP * (self.order as f64).sqrt();
        let to = [wet, p.mod_level, p.car_level, p.noise];
        for (g, t) in self.gains.iter_mut().zip(to) {
            g.to = t;
        }
    }

    #[inline]
    fn noise(&mut self) -> f64 {
        self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(self.seed as i32) / 2_147_483_648.0
    }

    /// Vocodes one block: `modulator` and `carrier` are (left, right) pairs of equal length; the result goes to `out`.
    pub fn process(&mut self, modulator: (&[f32], &[f32]), carrier: (&[f32], &[f32]), out: (&mut [f32], &mut [f32])) {
        let n = out.0.len().min(out.1.len()).min(modulator.0.len()).min(modulator.1.len()).min(carrier.0.len()).min(carrier.1.len());
        let order = self.order;
        let (hold, ka, kr, kw, kg, rms) = (self.hold, self.ka, self.kr, self.kw, self.kg, self.p.rms);
        for i in 0..n {
            let (ml, mr) = (f64::from(modulator.0[i]), f64::from(modulator.1[i]));
            let (cl, cr) = (f64::from(carrier.0[i]), f64::from(carrier.1[i]));
            let wet = self.gains[0].next(kg);
            let mod_level = self.gains[1].next(kg);
            let car_level = self.gains[2].next(kg);
            let noise_level = self.gains[3].next(kg);
            let nz = if noise_level > 0.0 {
                let x = self.noise();
                self.nhp_s.run(&self.nhp, x) * noise_level
            } else {
                0.0
            };
            let m = 0.5 * (ml + mr);
            let (xl, xr) = (cl + nz, cr + nz);
            let (mut yl, mut yr) = (0.0, 0.0);
            for b in &mut self.bands {
                // The modulator's band and its envelope.
                let mut a = m;
                for s in &mut b.m[..order] {
                    a = s.run(&b.c, a);
                }
                let level = if rms {
                    b.det += (a * a - b.det) * kw;
                    b.det.max(0.0).sqrt()
                } else {
                    let x = a.abs();
                    if x >= b.det {
                        b.det = x;
                        b.held = hold;
                    } else if b.held > 0 {
                        b.held -= 1;
                    } else {
                        b.det = x;
                    }
                    b.det
                };
                b.env += (level - b.env) * if level > b.env { ka } else { kr };
                // The carrier's band at that level.
                let (mut l, mut r) = (xl, xr);
                for s in &mut b.car[0][..order] {
                    l = s.run(&b.c, l);
                }
                for s in &mut b.car[1][..order] {
                    r = s.run(&b.c, r);
                }
                yl += l * b.env;
                yr += r * b.env;
            }
            let hl = self.hp_s[0].run(&self.hp, ml);
            let hr = self.hp_s[1].run(&self.hp, mr);
            let ll = self.lp_s[0].run(&self.lp, cl);
            let lr = self.lp_s[1].run(&self.lp, cr);
            out.0[i] = (yl * wet + hl * mod_level + ll * car_level) as f32;
            out.1[i] = (yr * wet + hr * mod_level + lr * car_level) as f32;
        }
        for b in &mut self.bands {
            let [c0, c1] = &mut b.car;
            for s in b.m.iter_mut().chain(c0.iter_mut()).chain(c1.iter_mut()) {
                s.flush();
            }
            if b.env < 1e-30 {
                b.env = 0.0;
            }
            if b.det < 1e-30 {
                b.det = 0.0;
            }
        }
        for s in self.hp_s.iter_mut().chain(self.lp_s.iter_mut()).chain(std::iter::once(&mut self.nhp_s)) {
            s.flush();
        }
    }
}
