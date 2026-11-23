// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio
// Copied into Beethoven Craft on 2026-11-23 from commit 349c6ba (modified: tests removed, paths).

//! Gain, pan and pitch laws.
//!
// spec: audio/gain-laws.md, audio/sampler-playback.md

use std::f64::consts::{FRAC_PI_4, SQRT_2};

pub use os_dsp::level::PanLaw;

/// The shared volume curve: `(11^x − 1) / 10`.
pub fn volume_curve(x: f64) -> f64 {
    (11f64.powf(x.max(0.0)) - 1.0) / 10.0
}

/// Channel volume `v` (0..=1) → amplitude; 1.0 = unity.
pub fn channel_gain(v: f64) -> f64 {
    volume_curve(v)
}

/// Note velocity (0..=128, 100 = unity) → amplitude.
pub fn velocity_gain(v: u8) -> f64 {
    volume_curve(f64::from(v) / 100.0)
}

/// Note pan byte (0..=128, 64 = center) → −1..=1, added to the channel pan.
pub fn note_pan(byte: u8) -> f64 {
    (f64::from(byte) - 64.0) / 64.0
}

/// Mixer fader `f` (0..=1) → amplitude; 0.8 = unity, 1.0 = +5.6 dB.
pub fn fader_gain(f: f64) -> f64 {
    volume_curve(f / 0.8)
}

/// The mixer fader position whose amplitude is `g`: [`fader_gain`]'s inverse, held to the fader's range (a gain over
/// +5.6 dB gives 1.0).
pub fn fader_of_gain(g: f64) -> f64 {
    (0.8 * (10.0 * g.max(0.0) + 1.0).ln() / 11f64.ln()).clamp(0.0, 1.0)
}

/// Channel pan (−1..=1), constant power: center = 0.7071 per side.
pub fn channel_pan(p: f64) -> (f64, f64) {
    let a = (1.0 + p.clamp(-1.0, 1.0)) * FRAC_PI_4;
    (a.cos(), a.sin())
}

/// Channel pan (−1..=1) under the project's panning law (event 23): [`channel_pan`] when Circular. Triangular keeps
/// the louder side at the centre level and lowers the quieter side in a straight line, `L = 0.7071·min(1, 1 − p)`,
/// `R = 0.7071·min(1, 1 + p)`, from the circular centre's own values, so the centre is the same bit for bit (spec:
/// audio/gain-laws.md "Panning law: Triangular").
pub fn channel_pan_law(p: f64, law: PanLaw) -> (f64, f64) {
    match law {
        PanLaw::Circular => channel_pan(p),
        PanLaw::Triangular => {
            let p = p.clamp(-1.0, 1.0);
            let (l, r) = channel_pan(0.0);
            (l * (1.0 - p).min(1.0), r * (1.0 + p).min(1.0))
        }
    }
}

/// Mixer track pan (−1..=1) as a stereo matrix `[[L←L, L←R], [R←L, R←R]]`.
///
/// Mid follows constant power (unity at center); on the louder side the side signal fades with
/// `cos(qπ/2)`; the quieter side keeps only its own input scaled by `√2·cos(t)`.
pub fn mixer_pan(p: f64) -> [[f64; 2]; 2] {
    let p = p.clamp(-1.0, 1.0);
    let q = p.abs();
    let t = (1.0 + q) * FRAC_PI_4;
    let loud_mid = SQRT_2 * t.sin();
    let side = (q * std::f64::consts::FRAC_PI_2).cos();
    let own = (loud_mid + side) / 2.0;
    let cross = (loud_mid - side) / 2.0;
    let quiet = SQRT_2 * t.cos();
    if p <= 0.0 {
        [[own, cross], [0.0, quiet]]
    } else {
        [[quiet, 0.0], [cross, own]]
    }
}

/// Playback rate for a note key (60 = original speed) plus channel pitch in cents.
pub fn playback_rate(key: u16, cents: i32) -> f64 {
    2f64.powf((f64::from(key) - 60.0) / 12.0 + f64::from(cents) / 1200.0)
}

/// Rate of a pitch in cents (0 = original speed).
pub fn cents_rate(cents: f64) -> f64 {
    2f64.powf(cents / 1200.0)
}

/// The channel's pitch in cents from its root note (spec: audio/voice-levels.md "The channel's levels"): the pitch
/// knob, fine tune and root note summed in doubles and rounded to single once, then the main pitch added as a single
/// when the channel follows it (`main` is `None` when its switch is off).
pub fn channel_pitch(knob: f64, fine: i32, root: u8, main: Option<i32>) -> f32 {
    let p = (knob + f64::from(fine) - f64::from(root) * 100.0) as f32;
    match main {
        Some(m) => p + m as f32,
        None => p,
    }
}

/// The channel's pitch knob (cents) for an automation value `v` (0..1): `Round(9600 v) - 4800`, so a clip spans
/// ±48 semitones whatever the channel's pitch range (spec: audio/voice-levels.md "Values from automation").
pub fn auto_pitch_cents(v: f64) -> i32 {
    (9600.0 * v).round_ties_even() as i32 - 4800
}

/// A note's fine pitch byte as cents (spec: audio/voice-levels.md "The note's own levels"): `fine * 10 - 1200`, 120 = 0.
pub fn fine_pitch_cents(fine: u8) -> i32 {
    i32::from(fine) * 10 - 1200
}

/// Length in ticks of a mono or portamento glide for the Misc tab's slide time knob `slide` (slide time / 1024) at
/// `ppq`: `x = single(slide time / 1024)`, `r = Round(single(single(exp(x ln 27) - 1) * 48) / 26)`, `n = r * (PPQ div
/// 4) div 48`; the glide takes `n + 1` steps, one per tick (spec: audio/note-rules.md "The length of a glide"; 500 gives
/// 3 ticks and 1024 gives 24 at 96 PPQ, whatever the tempo).
pub fn glide_ticks(slide: f32, ppq: u16) -> u32 {
    let x = slide.clamp(0.0, 1.0);
    let e = ((f64::from(x) * 27f64.ln()).exp() - 1.0) as f32;
    let r = (f64::from(e * 48.0) * (1.0 / 26.0)).round_ties_even().max(0.0) as u32;
    r * u32::from(ppq / 4) / 48
}

