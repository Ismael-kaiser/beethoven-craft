// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Beethoven DAW, based on Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Beethoven Fast Distortion: Beethoven DAW's fast distortion as a VST3 / CLAP plugin module. The plugin, its parameters and its editor are
//! `beethoven_small_fx::fast_distortion` (shared with the other small effects); this crate gives it its own module file.
//!
// spec: plugins/beethoven-fast-distortion.md, plugins/fruity-fast-dist.md
#![allow(semicolon_in_expressions_from_non_local_macros)]

#[cfg(not(feature = "linked"))]
use nih_plug::prelude::*;

pub use beethoven_small_fx::fast_distortion::BeethovenFastDistortion;

// Built as a plugin module: the C entry points hosts look for. Linked into Beethoven DAW (`linked` feature): only the
// symbol-free table, so several Beethoven plugins fit in one program (spec: plugins/plugin-formats.md).
#[cfg(not(feature = "linked"))]
nih_export_clap!(BeethovenFastDistortion);
#[cfg(not(feature = "linked"))]
nih_export_vst3!(BeethovenFastDistortion);
os_plugkit::linked_clap_entry!(BeethovenFastDistortion);
