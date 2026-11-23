// From FilmCraft (https://github.com/storytold/filmcraft, commit 17221b5), MIT OR Apache-2.0.
// Copied into Beethoven Craft on 2026-11-23; kept only the effects used.
//! Light, glow and motion effects.
use crate::effects::{dec, enc};
use crate::image::Image;
use crate::{Affine, EffectInstance, ParamValue, Tick, Vec2};
use rayon::prelude::*;

mod distort;
mod lights;

/// What an effect may know about its clip.
pub trait FxEnv: Sync {
    fn clip_seconds(&self) -> f64;
    fn frame_rate(&self) -> f64;
}

pub struct FxCtx<'a> {
    pub t: Tick,
    /// Working-image pixels per full-resolution pixel.
    pub px_scale: f32,
    /// Seconds since the clip start.
    pub seconds: f64,
    pub env: Option<&'a dyn FxEnv>,
    pub frame_rate: f64,
}

pub fn auto_point(effect: &str, param: &str) -> Option<(f64, f64)> {
    Some(match (effect, param) {
        ("corner_pin", "upper_left") => (0.0, 0.0),
        ("corner_pin", "upper_right") => (1.0, 0.0),
        ("corner_pin", "lower_left") => (0.0, 1.0),
        ("corner_pin", "lower_right") => (1.0, 1.0),
        ("gradient", "start") => (0.5, 0.0),
        ("gradient", "end") => (0.5, 1.0),
        ("lightning", "start") => (0.1, 0.1),
        ("lightning", "end") => (0.9, 0.9),
        ("checkerboard", "anchor") => (0.0, 0.0),
        ("write_on", "brush") => (0.5, 0.5),
        ("volumetric_rays", "center") => (0.5, 0.25),
        ("four_color_gradient", "p1") => (0.25, 0.25),
        ("four_color_gradient", "p2") => (0.75, 0.25),
        ("four_color_gradient", "p3") => (0.25, 0.75),
        ("four_color_gradient", "p4") => (0.75, 0.75),
        _ => return None,
    })
}

// ------------------------------------------------------------------ parameters

pub(crate) fn def_value(e: &EffectInstance, id: &str) -> Option<ParamValue> {
    e.def().and_then(|d| d.param(id)).map(|p| p.default.clone())
}
pub(crate) fn fv(e: &EffectInstance, id: &str, cx: &FxCtx) -> f32 {
    e.f64_at(id, cx.t) as f32
}
pub(crate) fn bv(e: &EffectInstance, id: &str) -> bool {
    e.param(id).and_then(|p| p.value.as_bool()).or_else(|| def_value(e, id).and_then(|v| v.as_bool())).unwrap_or(false)
}
pub(crate) fn chv(e: &EffectInstance, id: &str) -> u32 {
    match e.param(id).map(|p| p.value.clone()).or_else(|| def_value(e, id)) {
        Some(ParamValue::Choice(c)) => c,
        _ => 0,
    }
}
pub(crate) fn cv(e: &EffectInstance, id: &str, cx: &FxCtx) -> [f32; 4] {
    e.param(id).and_then(|p| p.value_at(cx.t).as_color()).or_else(|| def_value(e, id).and_then(|v| v.as_color())).unwrap_or([1.0; 4])
}
/// A point parameter in working pixels. Auto (NaN) components use the effect's auto position
/// (`filmcraft_project::effect::auto_point`) or the layer centre.
pub(crate) fn pv(e: &EffectInstance, id: &str, cx: &FxCtx, img: &Image) -> Vec2 {
    let v = e.param(id).map(|p| p.vec2_at(cx.t)).or_else(|| def_value(e, id).and_then(|v| v.as_vec2())).unwrap_or(Vec2::new(f64::NAN, f64::NAN));
    let (fx, fy) = auto_point(&e.effect, id).unwrap_or((0.5, 0.5));
    Vec2::new(if v.x.is_nan() { img.w as f64 * fx } else { v.x * cx.px_scale as f64 }, if v.y.is_nan() { img.h as f64 * fy } else { v.y * cx.px_scale as f64 })
}
/// A point offset parameter (no auto position) in working pixels.
pub(crate) fn offv(e: &EffectInstance, id: &str, cx: &FxCtx) -> Vec2 {
    let v = e.param(id).map(|p| p.vec2_at(cx.t)).or_else(|| def_value(e, id).and_then(|v| v.as_vec2())).unwrap_or_default();
    let k = cx.px_scale as f64;
    Vec2::new(if v.x.is_nan() { 0.0 } else { v.x * k }, if v.y.is_nan() { 0.0 } else { v.y * k })
}

// ------------------------------------------------------------------ colour helpers


#[inline]
pub(crate) fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}
#[inline]
pub(crate) fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}
#[inline]
pub(crate) fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
/// sRGB-encoded straight colour param → linear RGB.
#[inline]
pub(crate) fn lin(c: [f32; 4]) -> [f32; 3] {
    dec([c[0], c[1], c[2]])
}

/// Blend `src` (straight linear RGB) onto `base` (straight linear RGB) with one of
/// [`filmcraft_project::effect::SIMPLE_BLEND`] (Normal, Add, Screen, Multiply, Overlay).
#[inline]
pub(crate) fn blend_simple(mode: u32, base: [f32; 3], src: [f32; 3]) -> [f32; 3] {
    let mut o = [0.0; 3];
    for k in 0..3 {
        let (b, s) = (base[k], src[k]);
        o[k] = match mode {
            1 => b + s,
            2 => 1.0 - (1.0 - b.min(1.0)) * (1.0 - s.min(1.0)),
            3 => b * s,
            4 => {
                if b < 0.5 {
                    2.0 * b * s
                } else {
                    1.0 - 2.0 * (1.0 - b.min(1.0)) * (1.0 - s.min(1.0))
                }
            }
            _ => s,
        };
    }
    o
}

/// Premultiplied `src` over premultiplied `dst`.
#[inline]
pub(crate) fn over(dst: &mut [f32], src: [f32; 4]) {
    let k = 1.0 - src[3];
    for c in 0..4 {
        dst[c] = src[c] + dst[c] * k;
    }
}

/// Add light `add` (linear RGB) to a premultiplied pixel, keeping it opaque where light lands on
/// transparency (glows extend past alpha).
#[inline]
pub(crate) fn add_light(p: &mut [f32], add: [f32; 3]) {
    let m = add[0].max(add[1]).max(add[2]).clamp(0.0, 1.0);
    for k in 0..3 {
        p[k] += add[k];
    }
    p[3] = p[3] + (1.0 - p[3]) * m;
}

// ------------------------------------------------------------------ noise

#[inline]
pub(crate) fn hash_u(x: i64, y: i64, z: i64, seed: u64) -> u64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ (z as u64).wrapping_mul(0x1656_67B1_9E37_79F9)
        ^ seed.wrapping_mul(0x27D4_EB2F_1656_67C5);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 32)
}
/// Uniform 0..1 from integer coordinates.
#[inline]
pub(crate) fn hash01(x: i64, y: i64, z: i64, seed: u64) -> f32 {
    (hash_u(x, y, z, seed) >> 40) as f32 / (1u64 << 24) as f32
}

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}
#[inline]
fn grad3(h: u64, x: f32, y: f32, z: f32) -> f32 {
    // 12 edge directions of a cube (Perlin 2002)
    match h % 12 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 => -y + z,
        10 => y - z,
        _ => -y - z,
    }
}

/// 3-D gradient noise in about −1..1 (our own hash; Perlin's interpolant and gradient set).
pub(crate) fn noise3(x: f32, y: f32, z: f32, seed: u64) -> f32 {
    let (xi, yi, zi) = (x.floor(), y.floor(), z.floor());
    let (xf, yf, zf) = (x - xi, y - yi, z - zi);
    let (xi, yi, zi) = (xi as i64, yi as i64, zi as i64);
    let (u, v, w) = (fade(xf), fade(yf), fade(zf));
    let g = |dx: i64, dy: i64, dz: i64| grad3(hash_u(xi + dx, yi + dy, zi + dz, seed), xf - dx as f32, yf - dy as f32, zf - dz as f32);
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let x00 = l(g(0, 0, 0), g(1, 0, 0), u);
    let x10 = l(g(0, 1, 0), g(1, 1, 0), u);
    let x01 = l(g(0, 0, 1), g(1, 0, 1), u);
    let x11 = l(g(0, 1, 1), g(1, 1, 1), u);
    l(l(x00, x10, v), l(x01, x11, v), w) * 0.9
}

/// Fractal (fBm) noise with `octaves` (fractional octaves fade in), about −1..1.
pub(crate) fn fbm(x: f32, y: f32, z: f32, octaves: f32, seed: u64) -> f32 {
    let n = octaves.clamp(1.0, 10.0);
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    let mut i = 0;
    while (i as f32) < n {
        let k = (n - i as f32).min(1.0);
        sum += noise3(x * freq, y * freq, z * freq + i as f32 * 17.13, seed.wrapping_add(i as u64)) * amp * k;
        norm += amp * k;
        amp *= 0.5;
        freq *= 2.0;
        i += 1;
    }
    sum / norm.max(1e-6)
}

/// Smooth 1-D noise over time (−1..1): for wiggle/shake channels.
pub(crate) fn noise1(t: f64, channel: u64, seed: u64) -> f32 {
    noise3(t as f32, channel as f32 * 7.31 + 0.5, 0.37, seed)
}

// ------------------------------------------------------------------ image helpers

/// The alpha channel as its own buffer.
pub(crate) fn alpha_of(img: &Image) -> Vec<f32> {
    img.px.par_chunks(4).map(|p| p[3]).collect()
}

/// Gaussian-blur a single-channel buffer (via a temporary image).
pub(crate) fn blur_plane(plane: &[f32], w: usize, h: usize, sigma: f32) -> Vec<f32> {
    if sigma <= 0.3 {
        return plane.to_vec();
    }
    let mut im = Image { w, h, px: plane.iter().flat_map(|&v| [v, 0.0, 0.0, 0.0]).collect() };
    crate::effects::gaussian(&mut im, sigma, sigma, false);
    im.px.as_chunks::<4>().0.iter().map(|p| p[0]).collect()
}

/// Bright parts of an image above `threshold` (display luma), premultiplied linear, for glows.
pub(crate) fn highlights(img: &Image, threshold: f32, soft: f32) -> Image {
    let mut out = img.clone();
    out.px.par_chunks_mut(4).for_each(|p| {
        let a = p[3];
        if a <= 1e-6 {
            p.fill(0.0);
            return;
        }
        let l = enc([p[0] / a, p[1] / a, p[2] / a]);
        let k = smoothstep(threshold, threshold + soft.max(1e-3), luma(l));
        for v in p.iter_mut() {
            *v *= k;
        }
    });
    out
}

/// Fill an image from a per-pixel premultiplied colour function (x, y at pixel centres).
pub(crate) fn generate(img: &mut Image, f: impl Fn(f32, f32, [f32; 4]) -> [f32; 4] + Sync) {
    let w = img.w;
    img.px.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let p = &mut row[x * 4..x * 4 + 4];
            let o = f(x as f32 + 0.5, y as f32 + 0.5, [p[0], p[1], p[2], p[3]]);
            p.copy_from_slice(&o);
        }
    });
}

/// Inverse-map warp with an affine (destination → source sampling).
pub(crate) fn affine_warp(img: &mut Image, m: &Affine) {
    if m.is_identity() {
        return;
    }
    *img = img.transformed(img.w, img.h, m);
}

/// Shift/rotate/scale a layer about `c`: the transform shared by the animated Transform presets.
pub(crate) fn place(img: &mut Image, c: Vec2, offset: Vec2, scale: f64, rot_deg: f64) {
    let m = Affine::translate(c.x + offset.x, c.y + offset.y)
        .then_apply(&Affine::rotate_deg(rot_deg))
        .then_apply(&Affine::scale(scale, scale))
        .then_apply(&Affine::translate(-c.x, -c.y));
    affine_warp(img, &m);
}

/// Easing curves for [`filmcraft_project::effect::EASINGS`].
pub(crate) fn ease(kind: u32, t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    match kind {
        1 => t * t,
        2 => 1.0 - (1.0 - t) * (1.0 - t),
        3 => t * t * (3.0 - 2.0 * t),
        _ => t,
    }
}

/// Progress through the clip (0 at its first frame, 1 at its last) when an environment is
/// available, else 0.
pub(crate) fn clip_progress(cx: &FxCtx) -> f64 {
    match cx.env {
        Some(env) => {
            let d = env.clip_seconds();
            if d <= 1e-9 { 0.0 } else { (cx.seconds / d).clamp(0.0, 1.0) }
        }
        None => 0.0,
    }
}

// ------------------------------------------------------------------ dispatch

/// Apply `e` if it is one of the effects here.
pub fn apply(img: &mut Image, e: &EffectInstance, cx: &FxCtx) -> bool {
    match e.effect.as_str() {
        "rotate_3d" => distort::rotate_3d(img, e, cx),
        "grow" | "shrink" => distort::grow(img, e, cx),
        "move" => distort::move_fx(img, e, cx),
        "spin" => distort::spin(img, e, cx),
        "wiggle" => distort::wiggle(img, e, cx),
        "camera_shake" => distort::camera_shake(img, e, cx),
        "rounded_crop" => distort::rounded_crop(img, e, cx),
        "clone" => distort::clone_fx(img, e, cx),
        "echo_glow" => lights::echo_glow(img, e, cx),
        "edge_glow" => lights::edge_glow(img, e, cx),
        "glint" => lights::glint(img, e, cx),
        "light_leaks" => lights::light_leaks(img, e, cx),
        "rgb_split" => lights::rgb_split(img, e, cx),
        "volumetric_rays" => lights::volumetric_rays(img, e, cx),
        "wonder_glow" => lights::wonder_glow(img, e, cx),
        "alpha_glow" => lights::alpha_glow(img, e, cx),
        _ => return false,
    }
    true
}
