// From FilmCraft (https://github.com/storytold/filmcraft, commit 17221b5), MIT OR Apache-2.0.
// Copied into Toma Uno on 2026-11-23; tests removed, kept only what the transitions need.
//! Definitions the transitions read: effect schemas and instances.
use crate::geom::Vec2;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectKind {
    Video,
    VideoTransition,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ParamValue {
    Float(f64),
    Vec2(Vec2),
    Color([f32; 4]),
    Bool(bool),
    Choice(u32),
}

impl ParamValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ParamValue::Float(v) => Some(*v),
            ParamValue::Bool(b) => Some(f64::from(u8::from(*b))),
            ParamValue::Choice(c) => Some(f64::from(*c)),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParamValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_color(&self) -> Option<[f32; 4]> {
        match self {
            ParamValue::Color(c) => Some(*c),
            _ => None,
        }
    }
    pub fn as_vec2(&self) -> Option<Vec2> {
        match self {
            ParamValue::Vec2(v) => Some(*v),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParamKind {
    Float { min: f64, max: f64, soft_min: f64, soft_max: f64, unit: &'static str, decimals: u8 },
    Point,
    Color,
    Bool,
    Choice(&'static [&'static str]),
    Angle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParamDef {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: ParamKind,
    pub default: ParamValue,
    pub animatable: bool,
    pub group: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EffectDef {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: EffectKind,
    pub category: &'static [&'static str],
    pub params: Vec<ParamDef>,
    pub intrinsic: bool,
    pub accelerated: bool,
    pub float32: bool,
    pub yuv: bool,
}

impl EffectDef {
    pub fn param(&self, id: &str) -> Option<&ParamDef> {
        self.params.iter().find(|p| p.id == id)
    }
    pub fn instance(&self) -> EffectInstance {
        EffectInstance { effect: self.id.into(), params: self.params.iter().map(|p| (p.id.to_string(), Param { value: p.default.clone() })).collect() }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub value: ParamValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EffectInstance {
    pub effect: String,
    pub params: BTreeMap<String, Param>,
}

impl Param {
    pub fn value_at(&self, _: crate::Tick) -> ParamValue {
        self.value.clone()
    }
    pub fn vec2_at(&self, _: crate::Tick) -> Vec2 {
        self.value.as_vec2().unwrap_or_default()
    }
}

impl EffectInstance {
    pub fn def(&self) -> Option<&'static EffectDef> {
        find_effect(&self.effect)
    }
    pub fn param(&self, id: &str) -> Option<&Param> {
        self.params.get(id)
    }
    pub fn f64_at(&self, id: &str, _: crate::Tick) -> f64 {
        self.params.get(id).and_then(|p| p.value.as_f64()).or_else(|| self.def().and_then(|d| d.param(id)).and_then(|p| p.default.as_f64())).unwrap_or(0.0)
    }
}

pub fn effect_defs() -> &'static [EffectDef] {
    static D: OnceLock<Vec<EffectDef>> = OnceLock::new();
    D.get_or_init(|| {
        let mut v = crate::vtransition::video_transition_defs();
        v.extend(crate::vdefs::video_defs());
        v
    })
}

pub fn find_effect(id: &str) -> Option<&'static EffectDef> {
    effect_defs().iter().find(|d| d.id == id)
}

/// A transition and a video effect may share an id.
pub fn find_video(id: &str) -> Option<&'static EffectDef> {
    effect_defs().iter().find(|d| d.id == id && d.kind == EffectKind::Video)
}

pub fn find_transition(id: &str) -> Option<&'static EffectDef> {
    effect_defs().iter().find(|d| d.id == id && d.kind == EffectKind::VideoTransition)
}

pub(crate) fn fs(
    id: &'static str,
    label: &'static str,
    def: f64,
    (min, max): (f64, f64),
    (smin, smax): (f64, f64),
    unit: &'static str,
    decimals: u8,
) -> ParamDef {
    ParamDef {
        id,
        label,
        kind: ParamKind::Float { min, max, soft_min: smin, soft_max: smax, unit, decimals },
        default: ParamValue::Float(def),
        animatable: true,
        group: None,
    }
}
pub(crate) fn pt(id: &'static str, label: &'static str, x: f64, y: f64) -> ParamDef {
    ParamDef { id, label, kind: ParamKind::Point, default: ParamValue::Vec2(Vec2::new(x, y)), animatable: true, group: None }
}
pub(crate) fn col(id: &'static str, label: &'static str, c: [f32; 4]) -> ParamDef {
    ParamDef { id, label, kind: ParamKind::Color, default: ParamValue::Color(c), animatable: true, group: None }
}
pub(crate) fn b(id: &'static str, label: &'static str, v: bool) -> ParamDef {
    ParamDef { id, label, kind: ParamKind::Bool, default: ParamValue::Bool(v), animatable: false, group: None }
}
pub(crate) fn ch(id: &'static str, label: &'static str, opts: &'static [&'static str], def: u32) -> ParamDef {
    ParamDef { id, label, kind: ParamKind::Choice(opts), default: ParamValue::Choice(def), animatable: false, group: None }
}
pub(crate) fn ang(id: &'static str, label: &'static str, def: f64) -> ParamDef {
    ParamDef { id, label, kind: ParamKind::Angle, default: ParamValue::Float(def), animatable: true, group: None }
}

pub(crate) fn f(id: &'static str, label: &'static str, def: f64, min: f64, max: f64, unit: &'static str) -> ParamDef {
    fs(id, label, def, (min, max), (min, max), unit, 1)
}
