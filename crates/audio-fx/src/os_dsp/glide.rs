// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio
// Copied into Beethoven Craft on 2026-11-23 from commit 349c6ba (modified: the test module was removed).

//! The gain glide: gains do not jump, they move to a new target by a limited step per sample, planned once per block.
//!
// spec: audio/conventions.md ("Glide"), audio/mixer.md ("Gains ramp")

/// The per-sample limit of the engine's own gains (mixer, voices, channel filter): `single(single(44100 / rate) *
/// 0.001)`; 0.001 at 44100 Hz.
pub fn engine_limit(rate: u32) -> f32 {
    (f64::from((44100.0 / f64::from(rate.max(1))) as f32) * 0.001) as f32
}

/// The per-sample limit plugins use: `single(44100 / rate * 0.001)`, quotient and product in doubles. The same number
/// as [`engine_limit`] at 48000, 88200 and 96000 Hz.
pub fn plugin_limit(rate: u32) -> f32 {
    (44100.0 / f64::from(rate.max(1)) * 0.001) as f32
}

/// A gliding gain between blocks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glide {
    /// The value the next block starts from.
    pub current: f32,
}

/// One block of a glide: the gain as a running sum in single precision. The first sample has the old value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ramp {
    value: f32,
    step: f32,
}

impl Ramp {
    /// The gain for the next sample.
    #[inline]
    pub fn next(&mut self) -> f32 {
        let v = self.value;
        self.value += self.step;
        v
    }

    /// True when the gain does not move in this block.
    pub fn is_flat(&self) -> bool {
        self.step == 0.0
    }

    /// The gain the block starts with.
    pub fn first(&self) -> f32 {
        self.value
    }

    /// The per-sample step of the block (0 when the gain does not move).
    pub fn step(&self) -> f32 {
        self.step
    }
}

impl Glide {
    pub fn new(value: f32) -> Self {
        Glide { current: value }
    }

    /// Plans a block of `n` samples towards `target`, at most `limit` per sample. Unlimited, the gain reaches the
    /// target at the block's end; limited, it ends the block at `current + step * n` (set to exactly 0 when that is
    /// closer to zero than 2^-24) and carries on in the next block. The value carried over is that end value, not the
    /// running sum.
    pub fn block(&mut self, target: f32, n: usize, limit: f32) -> Ramp {
        let start = self.current;
        if start == target {
            return Ramp { value: start, step: 0.0 };
        }
        let nf = n.max(1) as f32;
        let mut step = (target - start) / nf;
        let end = if step.abs() <= limit {
            target
        } else {
            step = limit.copysign(target - start);
            let e = start + step * nf;
            if e.abs() < 2f32.powi(-24) {
                step = -start / nf;
                0.0
            } else {
                e
            }
        };
        self.current = end;
        Ramp { value: start, step }
    }
}

