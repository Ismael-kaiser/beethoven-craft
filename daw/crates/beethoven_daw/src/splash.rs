// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Beethoven DAW, based on Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! The startup splash. Beethoven Craft: removed (Beethoven Craft's menu shows its own loading screen); only the calls
//! `main` makes are left, and they do nothing.

/// Where startup's steps start on the gauge (0..1), kept for `main`'s calls.
pub const SETTINGS: f32 = 0.06;
pub const PROJECT: f32 = 0.2;
pub const MIDI: f32 = 0.32;
pub const WINDOW: f32 = 0.4;
pub const PLUGINS: f32 = 0.5;
pub const LOADING: f32 = 0.62;
pub const LOADED: f32 = 0.96;

/// There is never a splash; this type exists so `main` keeps its shape.
pub enum Splash {}

impl Splash {
    pub fn step(&self, _: f32, _: &str) {
        match *self {}
    }
    pub fn finish(&self) {
        match *self {}
    }
    pub fn gone(&self) -> bool {
        match *self {}
    }
}

pub fn step(_: &Option<Splash>, _: f32, _: &str) {}

pub fn open() -> Option<Splash> {
    None
}

/// The argument that started the splash's own process on macOS; nothing starts it now.
#[cfg(target_os = "macos")]
pub const ARG: &str = "--splash";

#[cfg(target_os = "macos")]
pub fn child_main() -> ! {
    std::process::exit(0)
}
