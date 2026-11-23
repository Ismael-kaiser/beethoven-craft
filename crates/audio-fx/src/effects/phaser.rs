// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio
// Copied into Toma Uno on 2026-11-23 from commit 349c6ba (modified: the test module was removed).

//! Lemon Studio's phaser (a recreation of FL's Fruity Phaser mixer effect), configured from its saved state.
//!
//! Per side a chain of 3 to 23 first-order all-pass sections whose common coefficient a sine LFO sweeps, with feedback
//! from the chain's end to its input, mixed with the dry signal. The right LFO runs up to half a cycle ahead of the
//! left one. FL's LFO runs from the plugin's creation, through the mixer's idle blocks before a song; Lemon Studio
//! starts a song with it as after [`IDLE`] such frames.
//!
// spec: plugins/fruity-phaser.md, plugins/lemon-phaser.md

use std::f64::consts::TAU;
use std::sync::OnceLock;

use super::Effect;

/// The settings, in the saved state's order (after the version field).
pub const PARAMS: usize = 9;
pub const SWEEP: usize = 0;
pub const MIN_DEPTH: usize = 1;
pub const MAX_DEPTH: usize = 2;
pub const RANGE: usize = 3;
pub const STEREO: usize = 4;
pub const STAGES: usize = 5;
pub const FEEDBACK: usize = 6;
pub const DRY_WET: usize = 7;
pub const GAIN: usize = 8;

/// The knobs' ranges (spec "Settings").
pub const RANGES: [(i32, i32); PARAMS] = [(0, 5000), (0, 1000), (0, 1000), (0, 1024), (0, 1024), (0, 22), (0, 1000), (0, 1024), (0, 5000)];

/// What a new Fruity Phaser holds (spec "Settings", the Default column).
pub const DEFAULTS: [i32; PARAMS] = [2500, 100, 800, 0, 512, 7, 400, 512, 4000];

/// The saved state: the version (1) and the nine knob values.
pub const STATE_LEN: usize = 40;
pub const VERSION: i32 = 1;

/// The most all-pass sections a side has.
pub const MAX_STAGES: usize = 23;

/// The LFO table's length.
pub const TABLE: usize = 4096;

/// The frames the LFO has run when a song starts (Lemon Studio choice). FL's runs from the plugin's creation through
/// the mixer's idle blocks, 512 or 1024 frames in FL 2026's exports, varying from run to run (spec "The LFO",
/// `plugins/lemon-phaser.md` §5); 512 is one of the starts FL makes, and a fixed one makes renders repeat.
pub const IDLE: u32 = 512;

/// Everything the saved state says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub v: [i32; PARAMS],
}

impl Default for Settings {
    fn default() -> Self {
        Settings { v: DEFAULTS }
    }
}

impl Settings {
    /// A saved state: the nine values after the version field. A version of 2 or more is not read: the instance keeps
    /// a new one's settings. `None` when the state is too short.
    pub fn from_state(b: &[u8]) -> Option<Settings> {
        let version = super::field(b, 0)?;
        let v: [i32; PARAMS] = super::ints(b, 1)?;
        Some(if version > VERSION { Settings::default() } else { Settings { v } })
    }

    /// The saved state (40 bytes).
    pub fn to_state(&self) -> Vec<u8> {
        let mut b = super::state_of(&[VERSION]);
        b.extend(super::state_of(&self.v));
        b
    }
}

// ---------------------------------------------------------------------------------------------------------------
// From a knob to its working value (spec "Settings", "Parameter laws")

/// Knob `k`'s position: `single(clamp(value * (1 / max), 0, 1))` in doubles.
pub fn position(k: usize, v: i32) -> f32 {
    (f64::from(v) * (1.0 / f64::from(RANGES[k].1))).clamp(0.0, 1.0) as f32
}

/// Round half to even, to an integer.
fn round(x: f64) -> i64 {
    x.round_ties_even() as i64
}

/// The frequency range is 10 Hz above half the knob, 2 Hz otherwise.
pub fn wide(x: f32) -> bool {
    x > 0.5
}

/// The sweep frequency in Hz: `single(((exp(single(x * 4)) - 1) / (exp(4) - 1)) * R)` in doubles.
pub fn speed(x: f32, wide_range: bool) -> f32 {
    let r = if wide_range { 10.0 } else { 2.0 };
    (((f64::from(x * 4.0).exp() - 1.0) / (4f64.exp() - 1.0)) * r) as f32
}

/// The number of sections: the stored value plus 1, limited to 3..23.
pub fn stages(stored: i32) -> usize {
    stored.saturating_add(1).clamp(3, MAX_STAGES as i32) as usize
}

/// The LFO's step per sample (2^24 a cycle): `Round(single(speed * k))`, `k = single((4096 / rate) * 4096)`.
pub fn increment(speed: f32, rate: u32) -> u32 {
    let k = ((4096.0 / f64::from(rate)) * 4096.0) as f32;
    round(f64::from(speed * k)) as u32
}

/// How far the right LFO runs ahead, in phase units: `Round(x * 8388608)` (up to half a cycle).
pub fn offset(x: f32) -> u32 {
    round(f64::from(x) * 8_388_608.0) as u32
}

/// The output gain: `single(x * 2 * sqrt(single(1 - single(f * f))))`, `f` the feedback knob's position.
pub fn gain(x: f32, f: f32) -> f32 {
    (f64::from(x) * 2.0 * f64::from(1.0 - f * f).sqrt()) as f32
}

/// The working values of a set of settings at a sample rate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Working {
    pub inc: u32,
    pub offset: u32,
    /// The depths' positions: the coefficient runs from `-min` (LFO at 0) to `-max` (LFO at 1).
    pub min: f32,
    pub max: f32,
    pub stages: usize,
    pub fb: f32,
    /// The dry share (0: only the chain's output).
    pub dry: f32,
    pub gain: f32,
}

impl Working {
    pub fn new(v: &[i32; PARAMS], rate: u32) -> Working {
        let x = |k: usize| position(k, v[k]);
        Working {
            inc: increment(speed(x(SWEEP), wide(x(RANGE))), rate),
            offset: offset(x(STEREO)),
            min: x(MIN_DEPTH),
            max: x(MAX_DEPTH),
            stages: stages(v[STAGES]),
            fb: (f64::from(x(FEEDBACK)) * 0.99) as f32,
            dry: x(DRY_WET),
            gain: gain(x(GAIN), x(FEEDBACK)),
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The LFO (spec "The LFO")

/// The wave (0..1) and its slopes.
pub fn table() -> &'static ([f32; TABLE], [f32; TABLE]) {
    static T: OnceLock<Box<([f32; TABLE], [f32; TABLE])>> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = Box::new(([0.0f32; TABLE], [0.0f32; TABLE]));
        // The true 2 pi (the original's is a hair off it, so its exports differ slightly).
        for i in 0..TABLE {
            t.0[i] = ((((i as f64 / 4096.0) * TAU).sin() + 1.0) * 0.5) as f32;
        }
        for i in 0..TABLE {
            t.1[i] = t.0[(i + 1) % TABLE] - t.0[i];
        }
        t
    })
}

/// The LFO's value at `phase` (2^24 a cycle; the top 8 bits are ignored), 0..1.
#[inline]
pub fn lfo(phase: u32) -> f32 {
    let (w, d) = table();
    let hi = ((phase >> 12) & (TABLE as u32 - 1)) as usize;
    let f = (phase & 4095) as f32 / 4096.0;
    w[hi] + f * d[hi]
}

// ---------------------------------------------------------------------------------------------------------------
// The effect

/// One side's chain: the sections' last outputs, the previous input and the feedback value.
#[derive(Clone, Copy, Debug, Default)]
struct Chain {
    y: [f32; MAX_STAGES],
    before: f32,
    m: f32,
}

pub struct Phaser {
    applied: [i32; PARAMS],
    rate: u32,
    w: Working,
    phase: [u32; 2],
    chain: [Chain; 2],
}

impl Phaser {
    /// A new instance started as at a render: the filters empty, the LFO as after [`IDLE`] frames from phase 0.
    pub fn with_settings(rate: u32, s: &Settings) -> Phaser {
        let mut p = Phaser { applied: s.v, rate, w: Working::new(&s.v, rate), phase: [0; 2], chain: [Chain::default(); 2] };
        p.start(IDLE);
        p
    }

    pub fn from_state(state: &[u8], rate: u32) -> Option<Phaser> {
        Some(Phaser::with_settings(rate, &Settings::from_state(state)?))
    }

    /// The filters emptied and the left LFO put where `idle` frames from phase 0 (at the current increment) take it.
    pub fn start(&mut self, idle: u32) {
        self.chain = [Chain::default(); 2];
        self.phase[0] = idle.wrapping_mul(self.w.inc);
        self.phase[1] = self.phase[0].wrapping_add(self.w.offset);
    }

    /// The LFOs' phases (tests).
    pub fn phases(&self) -> [u32; 2] {
        self.phase
    }

    /// The stored values applied last.
    pub fn settings(&self) -> Settings {
        Settings { v: self.applied }
    }

    /// New settings: they act from the next block (nothing glides).
    pub fn set(&mut self, s: &Settings) {
        if s.v != self.applied {
            self.applied = s.v;
            self.w = Working::new(&s.v, self.rate);
        }
    }

    pub fn set_sample_rate(&mut self, rate: u32) {
        self.rate = rate;
        self.w = Working::new(&self.applied, rate);
    }

    /// The working values (tests and the editor).
    pub fn working(&self) -> &Working {
        &self.w
    }

    /// One side's sample (spec "Per sample"): the coefficient from the LFO, the chain, the mix.
    #[inline]
    fn side(w: &Working, chain: &mut Chain, phase: &mut u32, s: f32) -> f32 {
        let v = lfo(*phase);
        *phase = phase.wrapping_add(w.inc);
        let a = (-w.min) - v * (w.max - w.min);
        let input = s + chain.m * w.fb;
        let (mut x, mut x_before) = (input, chain.before);
        for y in &mut chain.y[..w.stages] {
            let out = a * (x - *y) + x_before;
            x_before = *y;
            *y = out;
            x = out;
        }
        chain.before = input;
        chain.m = x;
        (w.dry * (s - x) + x) * w.gain
    }

    /// The start of a block: the right LFO at the left one plus the stereo offset.
    fn begin_block(&mut self) {
        self.phase[1] = self.phase[0].wrapping_add(self.w.offset);
    }

    /// One block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.begin_block();
        let w = self.w;
        let [pl, pr] = &mut self.phase;
        let [cl, cr] = &mut self.chain;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            *l = Phaser::side(&w, cl, pl, *l);
            *r = Phaser::side(&w, cr, pr, *r);
        }
    }
}

impl Effect for Phaser {
    fn process(&mut self, buf: &mut [f32]) {
        self.begin_block();
        let w = self.w;
        let [pl, pr] = &mut self.phase;
        let [cl, cr] = &mut self.chain;
        for f in buf.chunks_exact_mut(2) {
            f[0] = Phaser::side(&w, cl, pl, f[0]);
            f[1] = Phaser::side(&w, cr, pr, f[1]);
        }
    }

    // A slot switched on during the song: FL's plugin ignores the enable message, so nothing is cleared and the LFO
    // runs on (the default `reset` does nothing).
}

