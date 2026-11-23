// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio
// Copied into Toma Uno on 2026-11-23 from commit 349c6ba (modified: the test module was removed).

//! The EQ's processing, shared by the engine's own slot effect and the Lemon Parametric EQ plugin: the main level, then
//! LIN, or the bands as plain processing, HQ Legacy, HQ+ or HQ Enhanced; the silence gate, the bands' countdowns and the
//! clean-up at block ends. A block is one call of [`Equalizer::process`].
//!
// spec: plugins/fruity-parametric-eq-2.md ("Which processing a state selects", "Behaviour", "State and timing")

use super::design::{self, BandCoefs, Coefs, Design, Stored};
use super::hq::{self, Decimator, HalfBand, TAPS, TINY};
use super::lin::{self, Lin, TableBand};
use super::{BANDS, BAND_PASS, BAND_STOP, MASK_BYTE, PEAKING};

/// The 36 parameters: `group * 7 + band` (level, frequency, width, type, order), then the main level.
pub const PARAMS: usize = 36;
pub const MAIN: usize = 35;
/// A block is silent when no sample's magnitude is above this (1e-6).
const SILENT: u32 = 0x358637BD;
/// Pre-roll: the buffers of silence FL runs before a render.
const PREROLL_BLOCK: usize = 2048;

/// Each parameter's range.
pub fn range(i: usize) -> (i32, i32) {
    match (i / BANDS, i) {
        (_, MAIN) | (0, _) => (-1800, 1800),
        (1 | 2, _) => (0, 65536),
        (3, _) => (0, 7),
        _ => (-3, 3),
    }
}

/// The processing a state selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Plain,
    HqLegacy,
    HqPlus,
    HqEnhanced,
    Lin,
}

/// The settings of the state that decide the processing (besides the parameters).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// The int at 292: bit 0 = HQ on, bits 1..8 = the HQ mode.
    pub hq: i32,
    /// LIN's size setting (byte 309) and crossfade setting (byte 313).
    pub lin_size: i32,
    pub lin_fade: i32,
    /// Byte 317: 0x01 LIN, 0x02 HQ Enhanced (the other bits do nothing to the sound and are dropped).
    pub flags: u8,
    /// Bit `k` = band `k + 1` enabled.
    pub mask: u8,
}

impl Default for Options {
    /// A state of version 3 or lower with HQ off.
    fn default() -> Self {
        Options { hq: 0, lin_size: 2, lin_fade: 0, flags: 0, mask: 0x7f }
    }
}

fn int(state: &[u8], at: usize) -> Option<i32> {
    state.get(at..at + 4).map(|b| i32::from_le_bytes(b.try_into().unwrap()))
}

impl Options {
    /// From a saved state: the options of versions 4 and up, the defaults before.
    pub fn from_state(state: &[u8]) -> Options {
        Options::read(state, int(state, 0).unwrap_or(0) >= 4)
    }

    /// From a full-size state image whatever its version (the plugin's image, which the editor writes the options
    /// into; an older state is completed with the defaults there).
    pub fn from_image(state: &[u8]) -> Options {
        Options::read(state, true)
    }

    fn read(state: &[u8], with_options: bool) -> Options {
        let mut o = Options { hq: int(state, 292).unwrap_or(0), ..Options::default() };
        if with_options {
            o.lin_size = int(state, 309).unwrap_or(2);
            o.lin_fade = int(state, 313).unwrap_or(0);
            o.flags = state.get(317).copied().unwrap_or(0) & 0x03;
            o.mask = state.get(MASK_BYTE).copied().unwrap_or(0x7f) & 0x7f;
        }
        o
    }

    pub fn mode(&self) -> Mode {
        if self.flags & 0x01 != 0 { Mode::Lin } else { self.band_mode() }
    }

    /// The processing of the rows below LIN (which also decides LIN's coefficients).
    fn band_mode(&self) -> Mode {
        let on = self.hq & 1 != 0;
        match (on, (self.hq >> 1) & 0xff) {
            (true, 0) if self.flags & 0x02 != 0 => Mode::HqEnhanced,
            (true, 0) => Mode::HqLegacy,
            (true, 1) => Mode::HqPlus,
            _ => Mode::Plain,
        }
    }

    /// The bands' coefficients at the project's `rate`: the ordinary ones at twice it with HQ on in mode 0, the matched
    /// design's in HQ+, the ordinary ones at it otherwise.
    pub fn design(&self, rate: f64) -> Design {
        match self.band_mode() {
            Mode::HqLegacy | Mode::HqEnhanced => Design::Ordinary(2.0 * rate),
            Mode::HqPlus => Design::Matched(rate),
            _ => Design::Ordinary(rate),
        }
    }

    /// LIN's transform length.
    pub fn lin_n(&self, rate: f64) -> usize {
        lin::transform_len(self.lin_size, rate)
    }

    /// The delay the processing reports, in frames.
    pub fn latency(&self, rate: f64) -> usize {
        match self.mode() {
            Mode::Lin => lin::delay(self.lin_n(rate)),
            Mode::HqEnhanced => 44,
            _ => 0,
        }
    }
}

/// One section's state, in doubles.
#[derive(Clone, Copy, Debug, Default)]
struct Section {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Section {
    #[inline]
    fn run(&mut self, c: &Coefs, x: f32) -> f32 {
        let x = f64::from(x);
        let y = x * c.b0 + (((self.x1 * c.b1 - self.y2 * c.a2) + self.x2 * c.b2) - self.y1 * c.a1);
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y as f32
    }

    fn tidy(&mut self) {
        for v in [&mut self.x1, &mut self.x2, &mut self.y1, &mut self.y2] {
            if v.abs() < TINY {
                *v = 0.0;
            }
        }
    }
}

/// A band: its stored values, coefficients, the sections' state per channel and the countdown of a band at 0 dB.
#[derive(Clone, Copy, Debug, Default)]
struct Band {
    stored: Stored,
    coefs: BandCoefs,
    state: [[Section; 4]; 2],
    count: i64,
    off: bool,
}

impl Band {
    fn clear(&mut self) {
        self.state = Default::default();
    }

    /// A shelf or peaking band at a level of exactly 0.
    fn at_zero(&self) -> bool {
        self.stored.has_gain() && self.stored.level == 0
    }
}

pub struct Equalizer {
    /// The project's rate.
    rate: f64,
    params: [i32; PARAMS],
    opts: Options,
    design: Design,
    bands: [Band; BANDS],
    gain: f32,
    gate: i64,
    closed: bool,
    up: [HalfBand; 2],
    down: [HalfBand; 2],
    dec: [Decimator; 2],
    taps: [f32; TAPS],
    lin: Option<Lin>,
    /// LIN's "settings changed" mark.
    changed: bool,
    hi: [Vec<f32>; 2],
}

/// `single(exp(double(single(main) * 0.01 * dB)))`, the product in singles.
fn main_gain(main: i32) -> f32 {
    (f64::from(main as f32 * 0.01 * design::db_unit())).exp() as f32
}

impl Equalizer {
    /// A new instance at the project's `rate` with these parameters (clamped to their ranges) and options.
    pub fn new(rate: f64, params: &[i32; PARAMS], opts: Options) -> Equalizer {
        let params: [i32; PARAMS] = std::array::from_fn(|i| {
            let (lo, hi) = range(i);
            params[i].clamp(lo, hi)
        });
        let mut eq = Equalizer {
            rate,
            params,
            opts,
            design: opts.design(rate),
            bands: [Band::default(); BANDS],
            gain: main_gain(params[MAIN]),
            gate: 0,
            closed: false,
            up: Default::default(),
            down: Default::default(),
            dec: Default::default(),
            taps: hq::decimator_taps(),
            lin: None,
            changed: true,
            hi: [Vec::new(), Vec::new()],
        };
        for k in 0..BANDS {
            eq.set_band(k);
        }
        eq
    }

    /// From a saved state (fields 1..36 = the parameters, the options by the state's version).
    pub fn from_state(state: &[u8], rate: f64) -> Option<Equalizer> {
        let mut params = [0; PARAMS];
        for (i, p) in params.iter_mut().enumerate() {
            *p = int(state, 4 * (i + 1))?;
        }
        Some(Equalizer::new(rate, &params, Options::from_state(state)))
    }

    pub fn params(&self) -> &[i32; PARAMS] {
        &self.params
    }

    pub fn options(&self) -> Options {
        self.opts
    }

    pub fn mode(&self) -> Mode {
        self.opts.mode()
    }

    /// The delay the processing reports, in frames.
    pub fn latency(&self) -> usize {
        self.opts.latency(self.rate)
    }

    /// The rate the bands run at.
    fn band_rate(&self) -> f64 {
        self.design.rate()
    }

    /// Band `k`'s settings (re)set: its coefficients anew and its countdown from the start.
    fn set_band(&mut self, k: usize) {
        let p = &self.params;
        let stored = Stored { level: p[k], freq: p[BANDS + k], width: p[2 * BANDS + k], kind: p[3 * BANDS + k], order: p[4 * BANDS + k] };
        let count = (self.band_rate() as i64) / 5;
        let b = &mut self.bands[k];
        b.stored = stored;
        b.coefs = design::band(&stored, self.design);
        b.count = count;
        b.off = false;
    }

    /// An automation value for parameter `i`: from the next block on, without smoothing. A value for a band's type or
    /// order also clears its sections; any value for a band starts its countdown anew.
    pub fn set_param(&mut self, i: usize, v: i32) {
        if i >= PARAMS {
            return;
        }
        let (lo, hi) = range(i);
        self.params[i] = v.clamp(lo, hi);
        if i == MAIN {
            self.gain = main_gain(self.params[MAIN]);
        } else {
            let k = i % BANDS;
            self.set_band(k);
            if i / BANDS >= 3 {
                self.bands[k].clear();
            }
        }
        self.changed = true;
    }

    /// New options (an editor change, or a state loaded into a running instance). A new way of designing the bands
    /// recomputes them all and starts their countdowns anew; LIN switched on starts from silence.
    pub fn set_options(&mut self, o: Options) {
        if o == self.opts {
            return;
        }
        let was_lin = self.opts.mode() == Mode::Lin;
        self.opts = o;
        let design = o.design(self.rate);
        if design != self.design {
            self.design = design;
            for k in 0..BANDS {
                self.set_band(k);
            }
        }
        if o.mode() != Mode::Lin || !was_lin {
            self.lin = None;
        }
        self.changed = true;
    }

    /// Back to a new instance's state (the settings stay).
    pub fn reset(&mut self) {
        for k in 0..BANDS {
            self.set_band(k);
            self.bands[k].clear();
        }
        self.up = Default::default();
        self.down = Default::default();
        self.dec = Default::default();
        self.gate = 0;
        self.closed = false;
        self.lin = None;
        self.changed = true;
    }

    /// The buffers of silence FL runs before a render: four of 2048 frames, five when LIN's `N` is 8192 or more, so
    /// that LIN's first fade is over when the song starts and its blocks fall where FL's do. In the other modes
    /// silence does nothing (the gate closes on the first buffer).
    pub fn preroll(&mut self) {
        let buffers = if self.mode() == Mode::Lin && self.opts.lin_n(self.rate) >= 8192 { 5 } else { 4 };
        let (mut l, mut r) = (vec![0f32; PREROLL_BLOCK], vec![0f32; PREROLL_BLOCK]);
        for _ in 0..buffers {
            l.fill(0.0);
            r.fill(0.0);
            self.process(&mut l, &mut r);
        }
    }

    /// The table for LIN from the bands as they stand.
    fn lin_table(&self, n: usize) -> Vec<f32> {
        let bands: Vec<TableBand> = self
            .bands
            .iter()
            .enumerate()
            .map(|(k, b)| TableBand {
                freq: design::freq_hz(b.stored.freq.clamp(0, 65536)),
                narrow: b.stored.width < 8192 && matches!(b.stored.kind, BAND_PASS | BAND_STOP | PEAKING),
                sections: (self.opts.mask & (1 << k) != 0 && b.stored.kind != 0).then_some(b.coefs),
            })
            .collect();
        lin::table(n, self.rate, self.band_rate(), &bands)
    }

    fn runs(&self, k: usize) -> bool {
        let b = &self.bands[k];
        self.opts.mask & (1 << k) != 0 && b.stored.kind != 0 && !(b.at_zero() && b.off)
    }

    /// One block of both channels, in place.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        if n == 0 {
            return;
        }
        let (l, r) = (&mut l[..n], &mut r[..n]);
        let mode = self.mode();
        if mode == Mode::Lin {
            let len = self.opts.lin_n(self.rate);
            if self.lin.as_ref().is_none_or(|e| e.n() != len) {
                self.lin = Some(Lin::new(len, self.rate));
                self.changed = true;
            }
            if self.changed && !self.lin.as_ref().is_some_and(Lin::fading) {
                self.changed = false;
                let t = self.lin_table(len);
                let time = lin::fade_time(self.opts.lin_fade);
                if let Some(e) = self.lin.as_mut() {
                    e.hand_over(&t, time);
                }
            }
        }

        // The silence gate (in LIN refilled on every block).
        let silent = f32::from_bits(SILENT);
        let open = (self.rate as i64) / 5;
        if mode == Mode::Lin || l.iter().chain(r.iter()).any(|x| x.abs() > silent) {
            self.gate = open;
            self.closed = false;
        } else {
            self.gate = self.gate.saturating_sub(n as i64);
            if self.gate <= -(n as i64) {
                if !self.closed {
                    self.closed = true;
                    self.up = Default::default();
                    self.down = Default::default();
                    for k in 0..BANDS {
                        if self.runs(k) && self.bands[k].at_zero() {
                            self.bands[k].clear();
                        }
                    }
                }
                return;
            }
        }

        if self.gain != 1.0 {
            for x in l.iter_mut().chain(r.iter_mut()) {
                *x *= self.gain;
            }
        }

        match mode {
            Mode::Lin => {
                if let Some(e) = self.lin.as_mut() {
                    e.process(l, r);
                }
            }
            Mode::Plain | Mode::HqPlus => {
                self.run_bands(0, l);
                self.run_bands(1, r);
                self.band_ends(n);
            }
            Mode::HqLegacy | Mode::HqEnhanced => {
                for (ch, x) in [l, r].into_iter().enumerate() {
                    let mut hi = std::mem::take(&mut self.hi[ch]);
                    hi.resize(2 * n, 0.0);
                    if mode == Mode::HqLegacy {
                        for (i, &v) in x.iter().enumerate() {
                            [hi[2 * i], hi[2 * i + 1]] = self.up[ch].up(v);
                        }
                    } else {
                        for (i, &v) in x.iter().enumerate() {
                            hi[2 * i] = v + v;
                            hi[2 * i + 1] = 0.0;
                        }
                    }
                    self.run_bands(ch, &mut hi);
                    if mode == Mode::HqLegacy {
                        for (i, y) in x.iter_mut().enumerate() {
                            *y = self.down[ch].down(hi[2 * i], hi[2 * i + 1]);
                        }
                        self.up[ch].tidy();
                        self.down[ch].tidy();
                    } else {
                        for (i, y) in x.iter_mut().enumerate() {
                            *y = self.dec[ch].down(&self.taps, hi[2 * i], hi[2 * i + 1]);
                        }
                    }
                    self.hi[ch] = hi;
                }
                self.band_ends(2 * n);
            }
        }
    }

    /// The enabled bands in order on channel `ch`.
    fn run_bands(&mut self, ch: usize, x: &mut [f32]) {
        for k in 0..BANDS {
            if !self.runs(k) {
                continue;
            }
            let b = &mut self.bands[k];
            for (c, s) in b.coefs.as_slice().iter().zip(b.state[ch].iter_mut()) {
                for v in x.iter_mut() {
                    *v = s.run(c, *v);
                }
            }
        }
    }

    /// Block end for the bands that ran `samples` samples: the clean-up, and the countdown of those at 0 dB.
    fn band_ends(&mut self, samples: usize) {
        for k in 0..BANDS {
            if !self.runs(k) {
                continue;
            }
            let b = &mut self.bands[k];
            for s in b.state.iter_mut().flatten() {
                s.tidy();
            }
            if b.at_zero() {
                b.count -= samples as i64;
                if b.count <= 0 {
                    b.off = true;
                    b.clear();
                }
            }
        }
    }
}

