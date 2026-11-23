// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio
// Copied into Beethoven Craft on 2026-11-23 from commit 349c6ba.

/// One channel envelope record, values as stored (spec: audio/channel-envelopes.md). Times are knob
/// values (100 = none), `sustain` is 0..=128, tensions are -128..=128.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Envelope {
    pub enabled: bool,
    pub delay: i32,
    pub attack: i32,
    pub hold: i32,
    pub decay: i32,
    pub sustain: i32,
    pub release: i32,
    /// Pitch record: cents. Cutoff record: -128..=128.
    pub amount: i32,
    pub attack_tension: i32,
    pub decay_tension: i32,
    pub release_tension: i32,
    /// Flags (int 0): 1 = envelope times follow the tempo, 2 = LFO times follow the tempo, 4 = the envelope's
    /// value is 1 while it is off (spec: audio/envelopes-and-lfos.md).
    pub flags: i32,
    /// The record's LFO (ints 9..13): delay, attack and speed are time knobs, amount -128..=128, shape 0 sine /
    /// 1 triangle / 2 square.
    pub lfo_delay: i32,
    pub lfo_attack: i32,
    pub lfo_amount: i32,
    pub lfo_speed: i32,
    pub lfo_shape: i32,
}

impl Default for Envelope {
    /// A fresh channel's record: off, with the default knob positions.
    fn default() -> Self {
        Envelope {
            enabled: false,
            delay: 100,
            attack: 20000,
            hold: 20000,
            decay: 30000,
            sustain: 50,
            release: 20000,
            amount: 0,
            attack_tension: 0,
            decay_tension: 0,
            release_tension: 0,
            flags: 0,
            lfo_delay: 100,
            lfo_attack: 20000,
            lfo_amount: 0,
            lfo_speed: 32950,
            lfo_shape: 0,
        }
    }
}
