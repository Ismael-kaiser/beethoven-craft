// SPDX-License-Identifier: GPL-3.0-or-later
//! Subconjunto de los efectos de Lemon Studio (https://github.com/ItsAlphaNeon/Lemon-Studio, commit 349c6ba).
//! Los archivos en `effects/` y `os_dsp/` son suyos; este archivo y `effects/mod.rs` son el pegamento.
#![allow(dead_code)]
extern crate self as os_dsp;
pub mod effects;
#[path = "os_dsp/level.rs"]
pub mod level;
#[path = "os_dsp/glide.rs"]
pub mod glide;
#[path = "os_dsp/num.rs"]
pub mod num;
pub mod trio;
