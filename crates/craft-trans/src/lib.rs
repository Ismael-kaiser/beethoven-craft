//! Transiciones de video de FilmCraft (MIT OR Apache-2.0) para Toma Uno. Este archivo es el pegamento.
#![allow(dead_code, unused_imports)]
extern crate self as filmcraft_project;
extern crate self as filmcraft_geom;
extern crate self as filmcraft_color;
extern crate self as filmcraft_time;

pub mod effect;
pub mod effects;
pub mod geom;
pub mod image;
pub mod transitions;
pub mod vdefs;
pub mod vfx;
pub mod vtransition;

pub use effect::{EffectInstance, ParamValue};
pub use geom::*;

#[derive(Clone, Copy)]
pub struct Tick;
impl Tick {
    pub const ZERO: Tick = Tick;
}

#[inline]
pub fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}
#[inline]
pub fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}
