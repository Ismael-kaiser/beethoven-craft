// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio
// Copied into Toma Uno on 2026-11-23 from commit 349c6ba (modified: the test module was removed).

//! Levels: the shared level curve, knob positions, the constant-power pan split.
//!
// spec: audio/conventions.md ("Numbers", "Level curve", "Knob positions are products, not quotients", "Pan split")

/// The single nearest `1 / 12800` (volumes, mix levels, sends).
pub const R12800: f32 = 1.0 / 12800.0;
/// The single nearest `1 / 6400` (pans).
pub const R6400: f32 = 1.0 / 6400.0;
/// The single nearest `1 / 65536` (frequency and width positions; exact).
pub const R65536: f32 = 1.0 / 65536.0;

/// A stored value as a knob position: the value as a single times the single nearest the reciprocal of its full
/// scale (`R12800`, `R6400`, ...), one rounding. For many values this is one unit in the last place away from the
/// quotient.
pub fn knob(value: i32, recip: f32) -> f32 {
    value as f32 * recip
}

/// The level curve `(11^x - 1) / 10` in the host's precision: `x * ln 11` rounded to single, the exponential and
/// the subtraction in doubles, times the single nearest 0.1, rounded once. `curve(1)` is exactly 1; values above 1
/// are allowed (`curve(1.25) = 1.9033...`).
pub fn curve(x: f32) -> f32 {
    if x == 1.0 {
        return 1.0;
    }
    let p = x * (11f64.ln() as f32);
    ((f64::from(p).exp() - 1.0) * f64::from(0.1f32)) as f32
}

/// The constant-power pan split of `level` at `pan` (-1 = left, 1 = right, clamped): the angle `(pan + 1) * pi / 4`
/// in doubles, each side `level * cos` / `level * sin` as a double product rounded to single. Centred, each side is
/// `level * 0.7071...`; a caller that wants unity at centre passes `level * sqrt(2)`.
pub fn pan_split(level: f32, pan: f32) -> (f32, f32) {
    let angle = (f64::from(pan.clamp(-1.0, 1.0)) + 1.0) * std::f64::consts::PI / 4.0;
    let l = f64::from(level);
    ((l * angle.cos()) as f32, (l * angle.sin()) as f32)
}

/// The project's panning law (event 23; spec: audio/gain-laws.md "Panning law: Triangular"). Circular (0, the
/// default) is the constant-power split; Triangular (2) keeps the louder side at the centre level and lowers the
/// quieter side in a straight line. The centre is the same under both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanLaw {
    #[default]
    Circular,
    Triangular,
}

impl PanLaw {
    /// The law event 23 holds: 2 is Triangular; 0, no event and values the spec does not name are Circular.
    pub fn from_event(v: Option<u8>) -> PanLaw {
        if v == Some(2) { PanLaw::Triangular } else { PanLaw::Circular }
    }
}

/// [`pan_split`] under the panning law `law`. Circular is `pan_split` itself. Triangular takes each side's centre
/// level (the products `pan_split` makes at centre, so the centre is the same bit for bit) times `min(1, 1 - pan)` on
/// the left and `min(1, 1 + pan)` on the right, rounded to single once (spec: audio/gain-laws.md "Panning law:
/// Triangular").
pub fn pan_split_law(level: f32, pan: f32, law: PanLaw) -> (f32, f32) {
    match law {
        PanLaw::Circular => pan_split(level, pan),
        PanLaw::Triangular => {
            let p = f64::from(pan.clamp(-1.0, 1.0));
            let centre = std::f64::consts::PI / 4.0;
            let l = f64::from(level);
            ((l * centre.cos() * (1.0 - p).min(1.0)) as f32, (l * centre.sin() * (1.0 + p).min(1.0)) as f32)
        }
    }
}

/// `level * sqrt(2)` as a single (one rounding): the level that comes out of [`pan_split`] as itself at centre.
pub fn unity_at_centre(level: f32) -> f32 {
    (f64::from(level) * std::f64::consts::SQRT_2) as f32
}

/// The own volume of an audio clip's voice before its clip gain: `single(sqrt(2)) / curve(10000 / 12800)` in singles,
/// so that a clip on a channel at the default volume, centred, plays at unity (spec: audio/audio-clips.md "Level").
pub fn audio_clip_level() -> f32 {
    std::f32::consts::SQRT_2 / curve(knob(10000, R12800))
}

