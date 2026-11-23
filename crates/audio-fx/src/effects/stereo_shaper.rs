// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Beethoven DAW, based on Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio
// Copied into Beethoven Craft on 2026-11-23 from its `daw/` (modified: the test module was removed).

//! Beethoven DAW's stereo shaper (a recreation of FL's Fruity Stereo Shaper mixer effect), configured from its saved
//! state.
//!
//! A 2 x 2 matrix mixes the two channels with four gliding gains, each with a sign. A delay of whole samples and a
//! second-order all-pass ("phase") act on one channel, before the matrix or after it. The input minus the output can be
//! added to another track (the difference output, which the engine carries: [`Effect::extra`]).
//!
// spec: plugins/fruity-stereo-shaper.md, plugins/beethoven-stereo-shaper.md

use os_dsp::glide::{plugin_limit, Glide};

use super::stereo_enhancer::rate_factor;
use super::Effect;

/// The settings, in the saved state's order (after the version field).
pub const PARAMS: usize = 8;
pub const RIGHT_INTO_LEFT: usize = 0;
pub const LEFT: usize = 1;
pub const RIGHT: usize = 2;
pub const LEFT_INTO_RIGHT: usize = 3;
pub const DELAY: usize = 4;
pub const PHASE: usize = 5;
pub const DIFFERENCE: usize = 6;
pub const POSITION: usize = 7;

/// The settings' ranges (spec "Settings"; stored values are used as they are). The difference output's range is the
/// editor's (FL lists the routes of the track; 126 is the most a track can have).
pub const RANGES: [(i32, i32); PARAMS] =
    [(-25600, 25600), (-25600, 25600), (-25600, 25600), (-25600, 25600), (-4096, 4096), (-4096, 4096), (0, 126), (-1, 1)];

/// FL's "Default" preset (what a new Stereo Shaper holds): the levels at 100 %, nothing else, the effects "post".
pub const DEFAULTS: [i32; PARAMS] = [0, 12800, 12800, 0, 0, 0, 0, 1];

/// Effects positions. The editor's Off (-1, and any value below 0) plays as Pre: FL 2026's exports of such states put
/// the delay and the phase before the matrix (spec "FL 2026 null test").
pub const OFF: i32 = -1;
pub const PRE: i32 = 0;
pub const POST: i32 = 1;

/// The saved state: the version (0, but any is read) and the eight values.
pub const STATE_LEN: usize = 36;
pub const VERSION: i32 = 0;

/// The level that is unity.
pub const UNITY: i32 = 12800;

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
    /// A saved state: the eight values after the version field, whatever the version. `None` when it is too short.
    pub fn from_state(b: &[u8]) -> Option<Settings> {
        super::field(b, 0)?;
        Some(Settings { v: super::ints(b, 1)? })
    }

    /// The saved state (36 bytes).
    pub fn to_state(&self) -> Vec<u8> {
        let mut b = super::state_of(&[VERSION]);
        b.extend(super::state_of(&self.v));
        b
    }
}

/// A level slider's gain: `(3^|x| - 1) / 2` with the sign of `x = single(value / 12800)` (spec "Matrix"): exactly 1 at
/// 100 %, 4 at 200 %, 0 at 0.
pub fn level_gain(v: i32) -> f32 {
    let x = (f64::from(v) / 12800.0) as f32;
    if x == 0.0 {
        return 0.0;
    }
    let g = (((f64::from(x.abs() * (3f64.ln() as f32))).exp() - 1.0) * 0.5) as f32;
    if x < 0.0 {
        -g
    } else {
        g
    }
}

/// The gains' targets `[L, R, RL, LR]`.
pub fn gains(v: &[i32; PARAMS]) -> [f32; 4] {
    [LEFT, RIGHT, RIGHT_INTO_LEFT, LEFT_INTO_RIGHT].map(|k| level_gain(v[k]))
}

/// The delay in samples for setting `k` and the rate factor `c = single(rate / 100000)`: `(1001^(|k| / 4096) - 1) / 2`
/// ms, rounded (spec "Delay").
pub fn delay_samples(k: i32, c: f32) -> usize {
    let x = (f64::from(k.unsigned_abs()) / 4096.0) as f32;
    let e = ((f64::from(x) * 1001f64.ln()).exp() - 1.0) as f32;
    let t = e * 50.0;
    (t * c).round_ties_even().max(0.0) as usize
}

/// The channel a delay or phase setting acts on: the right for a positive one, the left for a negative one.
fn side(k: i32) -> usize {
    usize::from(k >= 0)
}

/// The all-pass's frequency for setting `k` (not 0) before the rate's limit: `single(single(exp(u * ln 5001) - 1) *
/// 4.398 + 10)` with `u = single(1 - |k| / 4096)`, 22 kHz just off the centre to 10 Hz at either end.
pub fn phase_hz(k: i32) -> f32 {
    let u = (1.0 - f64::from(k.unsigned_abs()) / 4096.0) as f32;
    let e = ((f64::from(u) * 5001f64.ln()).exp() - 1.0) as f32;
    (f64::from(e) * 4.398 + 10.0) as f32
}

/// The all-pass's frequency and coefficients (spec "Phase").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AllPass {
    pub hz: f32,
    pub a1: f64,
    pub a2: f64,
}

impl AllPass {
    /// `None` for a setting of 0 (off).
    pub fn new(k: i32, rate: u32) -> Option<AllPass> {
        if k == 0 {
            return None;
        }
        let hz = phase_hz(k).min((f64::from(rate) * 0.4999) as f32);
        let w = f64::from(hz) * ((2.0 * std::f64::consts::PI) / f64::from(rate));
        let k = 1.0 / (w.sin() + 1.0);
        Some(AllPass { hz, a1: -2.0 * k * w.cos(), a2: k * (1.0 - w.sin()) })
    }
}

/// A delay line of exactly `d` samples (empty: zeros).
#[derive(Clone, Debug, Default)]
struct Line {
    buf: Vec<f32>,
    pos: usize,
}

impl Line {
    fn new(d: usize) -> Line {
        Line { buf: vec![0.0; d], pos: 0 }
    }

    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        if self.buf.is_empty() {
            return x;
        }
        let y = std::mem::replace(&mut self.buf[self.pos], x);
        self.pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
        }
        y
    }
}

pub struct StereoShaper {
    applied: [i32; PARAMS],
    target: [f32; 4],
    gains: [Glide; 4],
    limit: f32,
    rate: u32,
    /// The rate factor in force when the delay is set.
    c: f32,
    line: Line,
    /// The delayed channel, and the channel the all-pass acts on: 0 left, 1 right.
    delayed: usize,
    phased: usize,
    allpass: Option<AllPass>,
    /// The all-pass's one memory: the last two inputs and outputs `[x1, x2, y1, y2]` (doubles).
    mem: [f64; 4],
    /// The slot has started: its gains were put at their targets (on at the song's start) or it was switched on.
    primed: bool,
    /// The difference output of the block processed last (interleaved).
    diff: Vec<f32>,
}

impl StereoShaper {
    /// A new instance, the rate known: the editor defaults, the matrix at zero (it has not rendered anything yet).
    pub fn new(rate: u32) -> StereoShaper {
        StereoShaper {
            applied: DEFAULTS,
            target: gains(&DEFAULTS),
            gains: [Glide::new(0.0); 4],
            limit: plugin_limit(rate),
            rate,
            c: rate_factor(rate),
            line: Line::default(),
            delayed: 1,
            phased: 1,
            allpass: None,
            mem: [0.0; 4],
            primed: false,
            diff: Vec::new(),
        }
    }

    /// A new instance with a state applied (its gains still at zero: see [`StereoShaper::flush`]).
    pub fn with_settings(rate: u32, s: &Settings) -> StereoShaper {
        let mut e = StereoShaper::new(rate);
        for k in 0..PARAMS {
            e.apply(k, s.v[k]);
        }
        e
    }

    pub fn from_state(state: &[u8], rate: u32) -> Option<StereoShaper> {
        Some(StereoShaper::with_settings(rate, &Settings::from_state(state)?))
    }

    /// The stored values applied last.
    pub fn settings(&self) -> Settings {
        Settings { v: self.applied }
    }

    /// Applies the settings that differ from the ones applied last, in index order.
    pub fn set(&mut self, s: &Settings) {
        for k in 0..PARAMS {
            if s.v[k] != self.applied[k] {
                self.apply(k, s.v[k]);
            }
        }
    }

    /// Applies setting `k`. A delay replaces the line by an empty one (also one of the same length); a phase changes
    /// the coefficients at once and keeps the memory, which 0 clears; a level gives its gain a new target.
    pub fn apply(&mut self, k: usize, v: i32) {
        self.applied[k] = v;
        match k {
            DELAY => {
                self.line = Line::new(delay_samples(v, self.c));
                self.delayed = side(v);
            }
            PHASE => {
                self.allpass = AllPass::new(v, self.rate);
                self.phased = side(v);
                if v == 0 {
                    self.mem = [0.0; 4];
                }
            }
            _ => self.target = gains(&self.applied),
        }
    }

    /// A new sample rate: the glide's limit, and the delay and the all-pass worked out again for it (the line empty).
    pub fn set_sample_rate(&mut self, rate: u32) {
        self.rate = rate;
        self.limit = plugin_limit(rate);
        self.c = rate_factor(rate);
        self.apply(DELAY, self.applied[DELAY]);
        self.allpass = AllPass::new(self.applied[PHASE], rate);
    }

    /// Every gain at its target at once.
    pub fn flush(&mut self) {
        self.gains = self.target.map(Glide::new);
    }

    /// The delay line and the all-pass memory emptied.
    pub fn clear(&mut self) {
        self.line = Line::new(self.line.buf.len());
        self.mem = [0.0; 4];
    }

    /// The delay in samples (tests and the editor).
    pub fn delay(&self) -> usize {
        self.line.buf.len()
    }

    /// The all-pass in force (tests and the editor).
    pub fn allpass(&self) -> Option<AllPass> {
        self.allpass
    }

    /// The gains `[L, R, RL, LR]` as they stand (tests).
    pub fn current(&self) -> [f32; 4] {
        self.gains.map(|g| g.current)
    }

    /// The delay and the all-pass act before the matrix (0 and below), or after it (above 0).
    fn pre(&self) -> bool {
        self.applied[POSITION] <= PRE
    }

    /// The delay, then the all-pass, on one frame.
    #[inline]
    fn effects(&mut self, x: &mut [f32; 2]) {
        let d = self.delayed;
        x[d] = self.line.run(x[d]);
        if let Some(ap) = self.allpass {
            let [x1, x2, y1, y2] = self.mem;
            let s = f64::from(x[self.phased]);
            let y = s * ap.a2 + (((x1 * ap.a1 - y2 * ap.a2) + x2) - y1 * ap.a1);
            self.mem = [s, x1, y, y1];
            x[self.phased] = y as f32;
        }
    }

    /// The end of a block: an all-pass memory value under 2^-24 in magnitude is set to 0.
    fn end_block(&mut self) {
        for m in &mut self.mem {
            if m.abs() < 1.0 / 16_777_216.0 {
                *m = 0.0;
            }
        }
    }

    /// One block in place (the plugin; no difference output).
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        self.block(left.iter_mut().zip(right.iter_mut()), n, None);
    }

    fn block<'a>(&mut self, frames: impl Iterator<Item = (&'a mut f32, &'a mut f32)>, n: usize, mut diff: Option<&mut [f32]>) {
        let [mut l_, mut r_, mut rl_, mut lr_] = [0, 1, 2, 3].map(|k| self.gains[k].block(self.target[k], n, self.limit));
        // With no gain moving and no cross level, the cross terms are left out (which keeps a -0).
        let straight = [l_, r_, rl_, lr_].iter().all(|r| r.is_flat()) && rl_.first() == 0.0 && lr_.first() == 0.0;
        let pre = self.pre();
        for (i, (l, r)) in frames.enumerate() {
            let input = [*l, *r];
            let mut x = input;
            if pre {
                self.effects(&mut x);
            }
            let (gl, gr, grl, glr) = (l_.next(), r_.next(), rl_.next(), lr_.next());
            let mut y = if straight { [gl * x[0], gr * x[1]] } else { [gl * x[0] + grl * x[1], gr * x[1] + glr * x[0]] };
            if !pre {
                self.effects(&mut y);
            }
            (*l, *r) = (y[0], y[1]);
            if let Some(d) = diff.as_deref_mut() {
                d[2 * i] = input[0] - y[0];
                d[2 * i + 1] = input[1] - y[1];
            }
        }
        self.end_block();
    }
}

impl Effect for StereoShaper {
    /// A slot that is on when the song starts has been running: its matrix is at the setting.
    fn process(&mut self, buf: &mut [f32]) {
        if !self.primed {
            self.flush();
            self.primed = true;
        }
        let n = buf.len() / 2;
        let mut diff = std::mem::take(&mut self.diff);
        diff.resize(buf.len(), 0.0);
        self.block(
            buf.chunks_exact_mut(2).map(|f| {
                let (l, r) = f.split_at_mut(1);
                (&mut l[0], &mut r[0])
            }),
            n,
            Some(&mut diff[..]),
        );
        self.diff = diff;
    }

    /// Switched on during the song: the matrix stays on the gains it last worked out (zero if it never ran, so it
    /// glides from there), the all-pass memory is cleared, the delay line keeps what is in it.
    fn reset(&mut self) {
        self.mem = [0.0; 4];
        self.primed = true;
    }

    fn extra(&self) -> Option<(usize, &[f32])> {
        usize::try_from(self.applied[DIFFERENCE]).ok().filter(|&n| n > 0).map(|n| (n, &self.diff[..]))
    }
}

