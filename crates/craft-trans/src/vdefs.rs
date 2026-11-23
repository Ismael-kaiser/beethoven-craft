// From FilmCraft (https://github.com/storytold/filmcraft, commit 17221b5), MIT OR Apache-2.0.
// Copied into Beethoven Craft on 2026-11-23; kept only the effects used.
//! Definitions of the effects in `vfx`.
#![allow(unused_imports)]
use crate::effect::{EffectDef, EffectKind, ParamDef, ParamKind, ParamValue, ang, b, ch, col, f, fs, pt};

pub const EASINGS: &[&str] = &["Linear", "Ease In", "Ease Out", "Ease In and Out"];
const IMAGE_CONTROL: &[&str] = &["Video Effects", "Image Control"];
pub(super) const LEGACY: &[&str] = &["Legacy", "Video Effects"];
pub(super) const LIGHTS: &[&str] = &["Video Effects", "Lights & Glows"];
const TRANSFORM: &[&str] = &["Video Effects", "Transform"];
pub(super) const UTILITY: &[&str] = &["Video Effects", "Utility"];

fn video(id: &'static str, name: &'static str, cat: &'static [&'static str], params: Vec<ParamDef>) -> EffectDef {
    EffectDef { id, name, kind: EffectKind::Video, category: cat, params, intrinsic: false, accelerated: false, float32: true, yuv: false }
}

fn grp(mut p: ParamDef, g: &'static str) -> ParamDef {
    p.group = Some(g);
    p
}

fn blend_orig() -> ParamDef {
    pct("blend", "Blend With Original", 0.0)
}

fn pct(id: &'static str, label: &'static str, def: f64) -> ParamDef {
    f(id, label, def, 0.0, 100.0, "%")
}

fn px(id: &'static str, label: &'static str, def: f64, max: f64, soft: f64) -> ParamDef {
    fs(id, label, def, (0.0, max), (0.0, soft), "", 1)
}

fn seed() -> ParamDef {
    fs("seed", "Random Seed", 0.0, (0.0, 10000.0), (0.0, 1000.0), "", 0)
}


pub fn video_defs() -> Vec<EffectDef> {
    vec![
        video(
            "rotate_3d",
            "3D Rotate",
            TRANSFORM,
            vec![
                ang("rot_x", "X Rotation", 0.0),
                ang("rot_y", "Y Rotation", 0.0),
                ang("rot_z", "Z Rotation", 0.0),
                fs("perspective", "Perspective", 50.0, (0.0, 100.0), (0.0, 100.0), "", 0),
                fs("z", "Z Position", 0.0, (-10000.0, 10000.0), (-2000.0, 2000.0), "", 0),
                b("hide_back", "Hide Back Face", false),
            ],
        ),
        video(
            "grow",
            "Grow",
            TRANSFORM,
            vec![
                fs("from", "Scale From", 100.0, (1.0, 1000.0), (50.0, 200.0), "%", 1),
                fs("to", "Scale To", 120.0, (1.0, 1000.0), (50.0, 200.0), "%", 1),
                pt("center", "Center", f64::NAN, f64::NAN),
                ch("easing", "Easing", EASINGS, 0),
            ],
        ),
        video(
            "shrink",
            "Shrink",
            TRANSFORM,
            vec![
                fs("from", "Scale From", 120.0, (1.0, 1000.0), (50.0, 200.0), "%", 1),
                fs("to", "Scale To", 100.0, (1.0, 1000.0), (50.0, 200.0), "%", 1),
                pt("center", "Center", f64::NAN, f64::NAN),
                ch("easing", "Easing", EASINGS, 0),
            ],
        ),
        video(
            "move",
            "Move",
            TRANSFORM,
            vec![
                pt("from", "Offset From", -100.0, 0.0),
                pt("to", "Offset To", 0.0, 0.0),
                ch("easing", "Easing", EASINGS, 3),
                pct("motion_blur", "Motion Blur", 0.0),
            ],
        ),
        video(
            "spin",
            "Spin",
            TRANSFORM,
            vec![
                fs("amount", "Rotation Amount", 360.0, (-36000.0, 36000.0), (-720.0, 720.0), "°", 1),
                pt("center", "Center", f64::NAN, f64::NAN),
                ch("easing", "Easing", EASINGS, 3),
                fs("scale", "Scale", 100.0, (1.0, 1000.0), (50.0, 200.0), "%", 1),
            ],
        ),
        video(
            "wiggle",
            "Wiggle",
            TRANSFORM,
            vec![
                fs("frequency", "Frequency", 2.0, (0.0, 60.0), (0.0, 10.0), "Hz", 2),
                px("amount", "Position Amount", 20.0, 2000.0, 200.0),
                fs("rotation", "Rotation Amount", 0.0, (0.0, 180.0), (0.0, 45.0), "°", 1),
                pct("scale", "Scale Amount", 0.0),
                ch("dimensions", "Dimensions", &["Horizontal and Vertical", "Horizontal", "Vertical"], 0),
                seed(),
            ],
        ),
        video(
            "camera_shake",
            "Camera Shake",
            TRANSFORM,
            vec![
                px("amount", "Position Amount", 20.0, 1000.0, 200.0),
                fs("rotation", "Rotation Amount", 1.0, (0.0, 45.0), (0.0, 10.0), "°", 2),
                pct("zoom", "Zoom Amount", 5.0),
                fs("frequency", "Frequency", 4.0, (0.1, 60.0), (0.1, 20.0), "Hz", 2),
                fs("complexity", "Complexity", 3.0, (1.0, 6.0), (1.0, 6.0), "", 0),
                pct("motion_blur", "Motion Blur", 0.0),
                seed(),
            ],
        ),
        video(
            "rounded_crop",
            "Rounded Crop",
            IMAGE_CONTROL,
            vec![
                pct("left", "Left", 0.0),
                pct("top", "Top", 0.0),
                pct("right", "Right", 0.0),
                pct("bottom", "Bottom", 0.0),
                px("radius", "Roundness", 40.0, 2000.0, 400.0),
                px("feather", "Feather", 0.0, 500.0, 100.0),
                px("border", "Border Width", 0.0, 500.0, 100.0),
                col("border_color", "Border Color", [1.0, 1.0, 1.0, 1.0]),
            ],
        ),
        video(
            "clone",
            "Clone",
            UTILITY,
            vec![
                fs("columns", "Columns", 2.0, (1.0, 16.0), (1.0, 16.0), "", 0),
                fs("rows", "Rows", 1.0, (1.0, 16.0), (1.0, 16.0), "", 0),
                px("gap", "Gap", 0.0, 1000.0, 100.0),
                b("mirror", "Mirror Alternate", false),
            ],
        ),
        video(
            "echo_glow",
            "Echo Glow",
            LIGHTS,
            vec![
                pct("threshold", "Threshold", 60.0),
                fs("echoes", "Echoes", 4.0, (1.0, 12.0), (1.0, 12.0), "", 0),
                px("radius", "Radius", 12.0, 500.0, 100.0),
                fs("spread", "Spread", 1.8, (1.0, 4.0), (1.0, 4.0), "", 2),
                fs("intensity", "Intensity", 100.0, (0.0, 1000.0), (0.0, 400.0), "%", 0),
                pct("decay", "Decay", 50.0),
                col("color", "Glow Color", [1.0, 0.85, 0.6, 1.0]),
            ],
        ),
        video(
            "edge_glow",
            "Edge Glow",
            LIGHTS,
            vec![
                pct("threshold", "Edge Threshold", 20.0),
                px("width", "Edge Width", 2.0, 50.0, 10.0),
                px("radius", "Glow Radius", 10.0, 500.0, 100.0),
                fs("intensity", "Intensity", 150.0, (0.0, 1000.0), (0.0, 400.0), "%", 0),
                col("color", "Glow Color", [0.3, 0.8, 1.0, 1.0]),
                b("only", "Glow Only", false),
            ],
        ),
        video(
            "glint",
            "Glint",
            LIGHTS,
            vec![
                pct("threshold", "Threshold", 75.0),
                ch("rays", "Rays", &["2", "4", "6", "8"], 1),
                px("length", "Length", 60.0, 2000.0, 400.0),
                ang("rotation", "Rotation", 45.0),
                fs("intensity", "Intensity", 100.0, (0.0, 1000.0), (0.0, 400.0), "%", 0),
                col("color", "Color", [1.0, 1.0, 1.0, 1.0]),
                pct("colorize", "Colorize", 0.0),
            ],
        ),
        video(
            "light_leaks",
            "Light Leaks",
            LIGHTS,
            vec![
                col("c1", "Color 1", [1.0, 0.45, 0.1, 1.0]),
                col("c2", "Color 2", [1.0, 0.15, 0.35, 1.0]),
                pct("intensity", "Intensity", 70.0),
                fs("scale", "Scale", 100.0, (10.0, 1000.0), (20.0, 400.0), "%", 0),
                ang("direction", "Direction", 30.0),
                fs("speed", "Speed", 1.0, (0.0, 20.0), (0.0, 5.0), "", 2),
                seed(),
                ch("mode", "Blending Mode", &["Screen", "Add", "Overlay"], 0),
            ],
        ),
        video(
            "rgb_split",
            "RGB Split",
            LIGHTS,
            vec![
                ch("mode", "Mode", &["Linear", "Radial"], 0),
                px("amount", "Amount", 10.0, 1000.0, 100.0),
                ang("angle", "Angle", 0.0),
                pt("center", "Center", f64::NAN, f64::NAN),
                blend_orig(),
            ],
        ),
        video(
            "volumetric_rays",
            "Volumetric Rays",
            LIGHTS,
            vec![
                pt("center", "Source Point", f64::NAN, f64::NAN),
                pct("threshold", "Threshold", 60.0),
                fs("length", "Ray Length", 50.0, (0.0, 100.0), (0.0, 100.0), "%", 0),
                fs("intensity", "Intensity", 100.0, (0.0, 1000.0), (0.0, 400.0), "%", 0),
                col("color", "Ray Color", [1.0, 0.95, 0.8, 1.0]),
                b("only", "Rays Only", false),
            ],
        ),
        video(
            "wonder_glow",
            "Wonder Glow",
            LIGHTS,
            vec![
                pct("threshold", "Threshold", 60.0),
                px("radius", "Radius", 25.0, 1000.0, 200.0),
                fs("intensity", "Intensity", 100.0, (0.0, 1000.0), (0.0, 400.0), "%", 0),
                fs("saturation", "Saturation", 100.0, (0.0, 400.0), (0.0, 200.0), "%", 0),
                b("use_color", "Use Glow Color", false),
                col("color", "Glow Color", [1.0, 0.8, 0.5, 1.0]),
                ch("mode", "Blending Mode", &["Add", "Screen"], 0),
            ],
        ),
        video(
            "alpha_glow",
            "Alpha Glow",
            LEGACY,
            vec![
                fs("glow", "Glow", 30.0, (0.0, 100.0), (0.0, 100.0), "", 0),
                fs("brightness", "Brightness", 252.0, (0.0, 255.0), (0.0, 255.0), "", 0),
                col("start_color", "Start Color", [1.0, 1.0, 1.0, 1.0]),
                col("end_color", "End Color", [1.0, 1.0, 1.0, 1.0]),
                b("use_end", "Use End Color", false),
                b("fade_out", "Fade Out", true),
            ],
        ),
    ]
}
