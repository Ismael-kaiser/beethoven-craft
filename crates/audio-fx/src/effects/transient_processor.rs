// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio
// Copied into Beethoven Craft on 2026-11-23 from commit 349c6ba (modified: the test module was removed).

//! Lemon Studio's transient processor (a recreation of FL's Transient Processor mixer effect), configured from its
//! saved state.
//!
//! The signal is split by a first-order crossover; the high band's attacks and tails are made louder or softer by one
//! gain per sample from five level followers, then soft-saturated with a drive, mixed back with the low band by the
//! split balance, and an output gain follows. The settings glide every 8th sample; the effect stands still in long
//! silence. All arithmetic is single precision with denormals flushed to zero.
//!
// spec: plugins/transient-processor.md, plugins/lemon-transient-processor.md

use super::Effect;

/// The settings, in the saved state's order (after the version field).
pub const PARAMS: usize = 9;
pub const ATTACK: usize = 0;
pub const DRIVE: usize = 1;
pub const RELEASE: usize = 2;
pub const GAIN: usize = 3;
pub const ATTACK_SHAPE: usize = 4;
pub const RELEASE_SHAPE: usize = 5;
pub const SPLIT: usize = 6;
pub const BALANCE: usize = 7;
pub const ON: usize = 8;

/// The settings' ranges (the editor's). Stored positions outside them are clamped (spec "Not covered"); any value but
/// 0 for processing on counts as on.
pub const RANGES: [(i32, i32); PARAMS] =
    [(0, 160), (0, 160), (0, 160), (0, 160), (0, 2), (0, 2), (0, 160), (0, 160), (0, 1)];

/// A fresh instance's settings, also FL's "Default" preset.
pub const DEFAULTS: [i32; PARAMS] = [80, 0, 80, 80, 1, 1, 80, 80, 1];

/// Shapes: Sharp, Mid, Slow.
pub const SHARP: i32 = 0;
pub const MID: i32 = 1;
pub const SLOW: i32 = 2;

/// The saved state: the version (1), the nine settings, then the display's switches (byte 40), paused flag (41..44)
/// and scroll speed (45..48).
pub const STATE_LEN: usize = 49;
pub const VERSION: i32 = 1;

/// Bytes 40..48 of a new state: display switches 5, not paused, scroll speed 50 (as in all of FL's presets).
pub const DISPLAY: [u8; 9] = [5, 0, 0, 0, 0, 50, 0, 0, 0];

/// The settings that glide (spec "Glide of the settings"), in index order.
const GLIDING: [usize; 6] = [ATTACK, DRIVE, RELEASE, GAIN, SPLIT, BALANCE];

/// Everything the saved state says that reaches the sound.
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
    /// A saved state: the nine settings after the version field. A version of 2 or more is not read: the instance
    /// keeps its fresh settings. `None` when the state is too short.
    pub fn from_state(b: &[u8]) -> Option<Settings> {
        let version = super::field(b, 0)?;
        let v: [i32; PARAMS] = super::ints(b, 1)?;
        Some(if version >= 2 { Settings::default() } else { Settings { v } })
    }

    /// The saved state (49 bytes) with the display bytes of a new instance.
    pub fn to_state(&self) -> Vec<u8> {
        let mut b = super::state_of(&[VERSION]);
        b.extend(super::state_of(&self.v));
        b.extend_from_slice(&DISPLAY);
        b
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Single arithmetic with denormals flushed to zero (spec "Behaviour")

/// A denormal counts as zero (keeping its sign).
#[inline]
fn z(x: f32) -> f32 {
    if x != 0.0 && x.abs() < f32::MIN_POSITIVE {
        0.0f32.copysign(x)
    } else {
        x
    }
}

#[inline]
fn add(a: f32, b: f32) -> f32 {
    z(a + b)
}

#[inline]
fn sub(a: f32, b: f32) -> f32 {
    z(a - b)
}

#[inline]
fn mul(a: f32, b: f32) -> f32 {
    z(a * b)
}

#[inline]
fn div(a: f32, b: f32) -> f32 {
    z(a / b)
}

/// A single power, correctly rounded (spec "Not covered": FL's matched it but for one near-tie).
fn powf(b: f32, e: f32) -> f32 {
    f64::from(b).powf(f64::from(e)) as f32
}

/// A single tangent, correctly rounded.
fn tanf(x: f32) -> f32 {
    f64::from(x).tan() as f32
}

const C_04: f32 = f32::from_bits(0x3ECC_CCCD);
const C_06: f32 = f32::from_bits(0x3F19_999A);
const C_01: f32 = f32::from_bits(0x3DCC_CCCD);
const C_DRIVE_EXP: f32 = f32::from_bits(0x3F8E_38E3);
const C_GAIN_EXP: f32 = f32::from_bits(0x3F66_6667);
const C_SPLIT_SQ: f32 = f32::from_bits(0x4543_5E28);
const C_SPLIT_LIN: f32 = f32::from_bits(0x417B_53D2);
const C_1E4: f32 = f32::from_bits(0x38D1_B717);
const C_1E6: f32 = f32::from_bits(0x3586_37BD);
const C_A1: f32 = f32::from_bits(0x3F75_C290);
const C_A2: f32 = f32::from_bits(0x3B44_9BA6);
const C_A_MAX: f32 = f32::from_bits(0x40D3_3333);
const C_B1: f32 = f32::from_bits(0x3F57_0A3D);
const C_B2: f32 = f32::from_bits(0x3A1D_4952);
/// The drive's copy when the plugin is created.
const DRIVE_AT_CREATION: f32 = 0.01;
/// The silence count after a block with sound: `Round(rate * 1.5) + 88`; and while standing still.
const STILL: i64 = -1_073_741_823;

// ---------------------------------------------------------------------------------------------------------------
// From a setting to its value (spec "From a setting to its value")

/// Setting `k`'s position in 0..1: `single(clamp(value * (1 / range), 0, 1))` in doubles (range 160, or 2 for the
/// shapes).
pub fn position(k: usize, value: i32) -> f32 {
    let range = if k == ATTACK_SHAPE || k == RELEASE_SHAPE { 2.0 } else { 160.0 };
    (f64::from(value) * (1.0 / range)).clamp(0.0, 1.0) as f32
}

/// `s = t * |t|` with `t = ((x - 0.5) + x) - 0.5`, -1..1.
fn signed_square(x: f32) -> f32 {
    let t = sub(add(sub(x, 0.5), x), 0.5);
    let s = mul(t, t);
    if t < 0.0 {
        -s
    } else {
        s
    }
}

/// The attack's weight, -0.6..0.4.
pub fn attack(x: f32) -> f32 {
    let s = signed_square(x);
    if s >= 0.0 {
        mul(C_04, s)
    } else {
        mul(C_06, s)
    }
}

/// The release's weight, -1..0.6.
pub fn release(x: f32) -> f32 {
    let s = signed_square(x);
    if s >= 0.0 {
        mul(s, C_06)
    } else {
        s
    }
}

/// The drive `D`, 0.1..15 (1 at position 0.1).
pub fn drive(x: f32) -> f32 {
    if x < C_01 {
        add(mul(x, 9.0), C_01)
    } else {
        powf(15.0, mul(sub(x, C_01), C_DRIVE_EXP))
    }
}

/// The output gain `G`, -9..+9 dB.
pub fn gain(x: f32) -> f32 {
    powf(10.0, mul(sub(x, 0.5), C_GAIN_EXP))
}

/// A shape's factor: Sharp 0.5, Mid 1, Slow 2 (the position compared as a double).
pub fn shape(x: f32) -> f64 {
    let x = f64::from(x);
    if x < 0.333 {
        0.5
    } else if x < 0.666 {
        1.0
    } else {
        2.0
    }
}

/// The split's filter coefficient `k = tan(pi * (995 x^2 + 5) / rate)` as the spec rounds it.
pub fn split_k(x: f32, sample_rate: u32) -> f32 {
    let inv = (1.0 / f64::from(sample_rate)) as f32;
    let arg = add(mul(mul(mul(x, x), inv), C_SPLIT_SQ), mul(inv, C_SPLIT_LIN));
    tanf(arg)
}

/// The split frequency in Hz, about `995 x^2 + 5` (the editor's text).
pub fn split_hz(x: f32) -> f64 {
    let x = f64::from(x);
    995.0 * x * x + 5.0
}

/// The split balance: the low band's and the processed high band's levels.
pub fn balance(x: f32) -> (f32, f32) {
    let r = sub(1.0, x);
    (add(r, r).min(1.0), add(x, x).min(1.0))
}

/// `0.1^y` in doubles, rounded to a single.
fn p(y: f64) -> f32 {
    0.1f64.powf(y) as f32
}

/// The followers' and the glide's constants for a rate and the two shapes.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Consts {
    kf: f32,
    ka: f32,
    kr: f32,
    kr2: f32,
    k4: f32,
    k3: f32,
    kp: f32,
}

impl Consts {
    fn new(sample_rate: u32, ash: f64, rsh: f64) -> Consts {
        let rate = f64::from(sample_rate);
        let kr = p(1.0 / (rsh * 0.02 * rate));
        Consts {
            kf: p(5.0 / rate),
            ka: 1.0 - p(1.0 / (ash * 0.01 * rate)),
            kr,
            kr2: 1.0 - kr,
            k4: p(1.0 / (rsh * 0.1 * rate)),
            k3: p(1.0 / f64::from(sample_rate as f32 * 4.0)),
            kp: 1.0 - p(200.0 / rate),
        }
    }
}

/// The split's coefficients from `k`: `c = 1 / (k + 1)`, `c * k`, `a = -((k - 1) * c)`.
#[derive(Clone, Copy, Debug, Default)]
struct Split {
    c: f32,
    ck: f32,
    a: f32,
}

impl Split {
    fn new(k: f32) -> Split {
        let c = div(1.0, add(k, 1.0));
        Split { c, ck: mul(c, k), a: -mul(sub(k, 1.0), c) }
    }
}

pub struct TransientProcessor {
    rate: u32,
    applied: [i32; PARAMS],
    /// The gliding settings' positions and targets (indexed by setting).
    pos: [f32; PARAMS],
    target: [f32; PARAMS],
    /// The mapped values of the positions.
    attack: f32,
    release: f32,
    drive: f32,
    out_gain: f32,
    split: Split,
    lo: f32,
    hi: f32,
    /// The copies the gain and the saturator use (a sample behind the values).
    attack_copy: f32,
    release_copy: f32,
    drive_copy: f32,
    consts: Consts,
    on: bool,
    e: [f32; 5],
    /// Per channel: the previous input, low and high band.
    x1: [f32; 2],
    low1: [f32; 2],
    high1: [f32; 2],
    /// Samples processed since creation (the glide's phase).
    count: u64,
    /// The silence count.
    quiet: i64,
}

impl TransientProcessor {
    /// A new instance with these settings in force at once (a project's loaded state: nothing glides in); the
    /// followers, filters and counts at 0, the copies at attack 0, release 0, drive 0.01.
    pub fn with_settings(sample_rate: u32, s: &Settings) -> TransientProcessor {
        let mut t = TransientProcessor {
            rate: sample_rate,
            applied: s.v,
            pos: [0.0; PARAMS],
            target: [0.0; PARAMS],
            attack: 0.0,
            release: 0.0,
            drive: 1.0,
            out_gain: 1.0,
            split: Split::default(),
            lo: 1.0,
            hi: 1.0,
            attack_copy: 0.0,
            release_copy: 0.0,
            drive_copy: DRIVE_AT_CREATION,
            consts: Consts::new(sample_rate, 1.0, 1.0),
            on: true,
            e: [0.0; 5],
            x1: [0.0; 2],
            low1: [0.0; 2],
            high1: [0.0; 2],
            count: 0,
            quiet: 0,
        };
        for k in 0..PARAMS {
            t.target[k] = position(k, s.v[k]);
            t.pos[k] = t.target[k];
        }
        for k in GLIDING {
            t.map(k);
        }
        t.reshape();
        t.on = s.v[ON] != 0;
        t
    }

    pub fn from_state(state: &[u8], sample_rate: u32) -> Option<TransientProcessor> {
        Some(TransientProcessor::with_settings(sample_rate, &Settings::from_state(state)?))
    }

    /// The stored values applied last.
    pub fn settings(&self) -> Settings {
        Settings { v: self.applied }
    }

    /// New settings: a gliding one gets a new target, the shapes and the switch act at once.
    pub fn set(&mut self, s: &Settings) {
        for k in 0..PARAMS {
            if s.v[k] == self.applied[k] {
                continue;
            }
            self.applied[k] = s.v[k];
            self.target[k] = position(k, s.v[k]);
            match k {
                ATTACK_SHAPE | RELEASE_SHAPE => {
                    self.pos[k] = self.target[k];
                    self.reshape();
                }
                ON => self.on = s.v[k] != 0,
                _ => {}
            }
        }
    }

    /// A new sample rate: the constants worked out again, the split frequency at its target at once.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.rate = sample_rate;
        self.pos[SPLIT] = self.target[SPLIT];
        self.map(SPLIT);
        self.reshape();
    }

    /// Every gliding setting at its target at once (a plugin's start; FL has no such step).
    pub fn settle(&mut self) {
        for k in GLIDING {
            self.pos[k] = self.target[k];
            self.map(k);
        }
    }

    /// The followers' and the glide's constants for the rate and the shapes.
    fn reshape(&mut self) {
        let (a, r) = (shape(self.pos[ATTACK_SHAPE]), shape(self.pos[RELEASE_SHAPE]));
        self.consts = Consts::new(self.rate, a, r);
    }

    /// Setting `k`'s position through its mapping.
    fn map(&mut self, k: usize) {
        let x = self.pos[k];
        match k {
            ATTACK => self.attack = attack(x),
            RELEASE => self.release = release(x),
            DRIVE => self.drive = drive(x),
            GAIN => self.out_gain = gain(x),
            SPLIT => self.split = Split::new(split_k(x, self.rate)),
            BALANCE => (self.lo, self.hi) = balance(x),
            _ => {}
        }
    }

    /// Every 8th processed sample: each gliding setting off its target moves by `kp` of the way, onto it within 1e-4.
    fn glide(&mut self) {
        let kp = self.consts.kp;
        for k in GLIDING {
            let (v, t) = (self.pos[k], self.target[k]);
            if v == t {
                continue;
            }
            let mut n = add(v, mul(sub(t, v), kp));
            if sub(n, t).abs() < C_1E4 {
                n = t;
            }
            self.pos[k] = n;
            self.map(k);
        }
    }

    /// One sample pair (spec "The split", "The gain, per sample", "Saturation and output").
    fn frame(&mut self, l: f32, r: f32) -> (f32, f32) {
        if self.count.is_multiple_of(8) {
            self.glide();
        }
        self.count += 1;
        let Split { c, ck, a } = self.split;
        let x = [z(l), z(r)];
        let mut low = [0.0f32; 2];
        let mut high = [0.0f32; 2];
        for ch in 0..2 {
            low[ch] = add(add(mul(ck, x[ch]), mul(ck, self.x1[ch])), mul(a, self.low1[ch]));
            high[ch] = add(add(mul(c, x[ch]), mul(-c, self.x1[ch])), mul(a, self.high1[ch]));
        }
        self.x1 = x;
        self.low1 = low;
        self.high1 = high;

        let k = self.consts;
        let m = mul(add(high[1].abs(), high[0].abs()), 0.5);
        let [e1, e2, e3, e4, e5] = &mut self.e;
        *e1 = if m > *e1 { m } else { mul(*e1, k.kf) };
        *e2 = if m > *e2 { add(mul(sub(m, *e2), k.ka), *e2) } else { mul(*e2, k.kf) };
        *e3 = if m >= *e3 { m } else { mul(*e3, k.k3) };
        *e4 = if m >= *e4 { m } else { mul(*e4, k.k4) };
        let q = if self.release_copy >= 0.0 && *e4 > 0.0 {
            let d = add(*e4, C_1E4);
            div(sub(d, *e5), d)
        } else {
            1.0
        };
        let q = mul(q, 3.0).clamp(0.0, 1.0);
        *e5 = add(mul(add(mul(q, k.kr), k.kr2), sub(*e4, *e5)), *e5);
        let (e1, e2, e3, e5) = (*e1, *e2, *e3, *e5);

        let ra = if e2 > 0.0 { div(e1, e2) } else { 0.0 };
        let att = if e2 > 0.0 && ra >= 1.0 {
            if self.attack_copy < 0.0 {
                ra
            } else if ra <= 10.0 {
                mul(ra, sub(C_A1, mul(mul(ra, C_A2), ra)))
            } else {
                C_A_MAX
            }
        } else {
            1.0
        };
        let rb = if e5 > C_1E6 { div(e3, e5) } else { 0.0 };
        let rel = if e5 > C_1E6 && rb >= 1.0 {
            if self.release_copy < 0.0 {
                rb
            } else if rb <= 20.0 {
                sub(mul(rb, C_B1), mul(mul(rb, rb), mul(rb, C_B2)))
            } else {
                12.0
            }
        } else {
            1.0
        };
        let s = add(mul(sub(rel, 1.0), self.release_copy), mul(sub(att, 1.0), self.attack_copy));
        let g = if s >= 0.0 { add(s, 1.0) } else { div(1.0, sub(1.0, s)) };

        let d = self.drive_copy;
        let mut out = [0.0f32; 2];
        for ch in 0..2 {
            let u = mul(d, mul(high[ch], g));
            let sat = if u < -3.0 {
                -1.0
            } else if u > 3.0 {
                1.0
            } else {
                div(mul(add(mul(u, u), 27.0), u), add(mul(mul(u, 9.0), u), 27.0))
            };
            let y = div(sat, d);
            out[ch] = mul(add(mul(self.lo, low[ch]), mul(self.hi, y)), self.out_gain);
        }

        self.attack_copy = add(sub(self.attack, self.attack_copy), self.attack_copy);
        self.release_copy = add(sub(self.release, self.release_copy), self.release_copy);
        self.drive_copy = add(sub(self.drive, self.drive_copy), self.drive_copy);
        (out[0], out[1])
    }

    /// The silence count for a block of `n` frames: whether the block is processed (spec "State and timing").
    fn runs(&mut self, silent: bool, n: usize) -> bool {
        let n = n as i64;
        if silent {
            self.quiet -= n;
        } else {
            self.quiet = (f64::from(self.rate) * 1.5).round_ties_even() as i64 + 88;
        }
        if self.quiet > -n {
            true
        } else {
            self.quiet = STILL;
            false
        }
    }

    /// One block in place. In long silence, or switched off, the input passes unchanged and the state stands still.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        let silent = left[..n].iter().chain(&right[..n]).all(|x| x.abs() <= C_1E6);
        if self.runs(silent, n) && self.on {
            for (l, r) in left.iter_mut().zip(right.iter_mut()) {
                (*l, *r) = self.frame(*l, *r);
            }
        }
    }
}

impl Effect for TransientProcessor {
    fn process(&mut self, buf: &mut [f32]) {
        let n = buf.len() / 2;
        let silent = buf.iter().all(|x| x.abs() <= C_1E6);
        if self.runs(silent, n) && self.on {
            for f in buf.chunks_exact_mut(2) {
                (f[0], f[1]) = self.frame(f[0], f[1]);
            }
        }
    }
}

