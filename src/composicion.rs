//! Compone un cuadro de video: capas, movimiento, efectos (EffectCraft), transiciones (FilmCraft) y títulos.
use crate::modelo::*;
use crate::titulos;
use craft_trans::effect as ct;
use effectcraft_color::BlendMode;
use effectcraft_effects as ec;
use effectcraft_keyframe::Value;
use effectcraft_raster::Image;
use rayon::prelude::*;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock};

/// Un cuadro decodificado.
#[derive(Default)]
pub struct Cuadro {
    pub t: f64,
    pub ancho: usize,
    pub alto: usize,
    /// RGBA de 8 bits; vacío si viene en `yuv`.
    pub rgba: Vec<u8>,
    /// Planos Y, U y V a 4:2:0 (BT.709, rango limitado), como sale de casi todo video: pesa menos de la mitad.
    pub yuv: Vec<u8>,
}

impl Cuadro {
    pub fn en_rgba(&self) -> std::borrow::Cow<'_, [u8]> {
        if self.yuv.is_empty() { (&self.rgba[..]).into() } else { a_rgba(&self.yuv, self.ancho, self.alto).into() }
    }
    pub fn peso(&self) -> usize {
        self.rgba.len() + self.yuv.len()
    }
}

/// Planos 4:2:0 a RGBA, con el color suavizado entre muestras.
pub fn a_rgba(yuv: &[u8], w: usize, h: usize) -> Vec<u8> {
    let (cw, ch) = (w / 2, h / 2);
    let (y, c) = yuv.split_at(w * h);
    let (u, v) = c.split_at(cw * ch);
    let mut o = vec![255u8; w * h * 4];
    o.par_chunks_mut(w * 4).enumerate().for_each(|(j, fila)| {
        // Color: centrado a lo alto y pegado a la izquierda a lo ancho.
        let (a, b) = (j / 2, if j % 2 == 0 { (j / 2).saturating_sub(1) } else { (j / 2 + 1).min(ch - 1) });
        let mezcla = |p: &[u8], i: usize| p[a * cw + i] as i32 * 3 + p[b * cw + i] as i32;
        for (i, (px, l)) in fila.as_chunks_mut::<4>().0.iter_mut().zip(&y[j * w..]).enumerate() {
            let (p, q) = (i / 2, if i % 2 == 0 { i / 2 } else { (i / 2 + 1).min(cw - 1) });
            let uu = (mezcla(u, p) + mezcla(u, q) - 1024) as f32 * (1.0 / 8.0);
            let vv = (mezcla(v, p) + mezcla(v, q) - 1024) as f32 * (1.0 / 8.0);
            let l = (*l as f32 - 16.0) * (255.0 / 219.0);
            const K: f32 = 255.0 / 224.0;
            px[0] = (l + 1.5748 * K * vv + 0.5) as u8;
            px[1] = (l - 0.187324 * K * uu - 0.468124 * K * vv + 0.5) as u8;
            px[2] = (l + 1.8556 * K * uu + 0.5) as u8;
        }
    });
    o
}

/// RGBA a planos 4:2:0 (BT.709, rango limitado).
pub fn a_yuv(rgba: &[u8], w: usize, h: usize) -> Vec<u8> {
    let (mut y, mut u, mut v) = (vec![], vec![], vec![]);
    effectcraft_raster::yuv::rgba_to_yuv420(rgba, w, h, &mut y, &mut u, &mut v);
    y.append(&mut u);
    y.append(&mut v);
    y
}

pub const FUSIONES: [(&str, BlendMode); 12] = [
    ("Normal", BlendMode::Normal),
    ("Multiplicar", BlendMode::Multiply),
    ("Pantalla", BlendMode::Screen),
    ("Superponer", BlendMode::Overlay),
    ("Luz suave", BlendMode::SoftLight),
    ("Luz dura", BlendMode::HardLight),
    ("Sumar", BlendMode::Add),
    ("Aclarar", BlendMode::Lighten),
    ("Oscurecer", BlendMode::Darken),
    ("Diferencia", BlendMode::Difference),
    ("Tono", BlendMode::Hue),
    ("Color", BlendMode::Color),
];

/// Tamaño al que se decodifica un clip para un lienzo de `w` × `h`: cabe completo.
pub fn ajuste(c: &ClipVideo, w: usize, h: usize) -> (usize, usize) {
    let k = (w as f64 / c.ancho.max(1) as f64).min(h as f64 / c.alto.max(1) as f64);
    let par = |v: u32| ((v as f64 * k / 2.0).round() as usize * 2).max(2);
    (par(c.ancho), par(c.alto))
}

// ---- efectos

pub fn valor(v: &Value) -> Option<Valor> {
    Some(match v {
        Value::Scalar(x) => Valor::Num(*x),
        Value::Vec2(p) => Valor::Punto(*p),
        Value::Color(c) => Valor::Color(c.map(|x| x as f32)),
        Value::Bool(b) => Valor::Si(*b),
        Value::Enum(i) => Valor::Op(*i),
        Value::Str(s) => Valor::Texto(s.clone()),
        _ => return None,
    })
}

/// Valores iniciales de un efecto. Los puntos quedan en fracción del cuadro.
pub fn fx_nuevo(id: u64, tipo: &str) -> Option<FxVideo> {
    if let Some(d) = tipo.strip_prefix("fc:").and_then(ct::find_video) {
        let p = d
            .params
            .iter()
            .map(|p| {
                let v = match &p.default {
                    craft_trans::ParamValue::Float(x) => Valor::Num(*x),
                    // Sin valor propio, un punto es el centro.
                    craft_trans::ParamValue::Vec2(v) => Valor::Punto([if v.x.is_nan() { 0.5 } else { v.x }, if v.y.is_nan() { 0.5 } else { v.y }]),
                    craft_trans::ParamValue::Color(c) => Valor::Color(*c),
                    craft_trans::ParamValue::Bool(b) => Valor::Si(*b),
                    craft_trans::ParamValue::Choice(i) => Valor::Op(*i),
                };
                (p.id.to_string(), Prop { v, claves: vec![], curvas: vec![], libre: false })
            })
            .collect();
        let mut f = FxVideo { id, tipo: tipo.into(), activo: true, p };
        for (k, v) in arranque(tipo) {
            if let Some(p) = f.p.get_mut(*k) {
                p.v = v.clone();
            }
        }
        return Some(f);
    }
    if let Some(d) = crate::vpropios::buscar(tipo) {
        return Some(crate::vpropios::nuevo(id, d));
    }
    let s = ec::find(tipo)?;
    let p = s
        .params
        .iter()
        .filter_map(|p| Some((p.id.to_string(), Prop { v: valor(&p.default)?, claves: vec![], curvas: vec![], libre: false })))
        .collect();
    let mut f = FxVideo { id, tipo: tipo.into(), activo: true, p };
    for (k, v) in arranque(tipo) {
        if let Some(p) = f.p.get_mut(*k) {
            p.v = v.clone();
        }
    }
    // Algunos traen una vista de diagnóstico: aquí siempre interesa el resultado.
    for d in &s.params {
        if let effectcraft_project::ParamUi::Popup { options } = &d.ui {
            if let (Some(i), Some(p)) = (options.iter().position(|o| o.starts_with("Final")), f.p.get_mut(d.id)) {
                p.v = Valor::Op(i as u32);
            }
        }
    }
    Some(f)
}

/// Valores con los que se pone cada efecto que de fábrica llega en neutro, para que se note desde el principio.
#[rustfmt::skip]
fn arranque(tipo: &str) -> &'static [(&'static str, Valor)] {
    match tipo {
        "ec.blur.bilateral" => &[("radius", Valor::Num(20.0)), ("threshold", Valor::Num(40.0))],
        "ec.blur.ccvector" => &[("amount", Valor::Num(100.0))],
        "ec.blur.channel" => &[("redBlurriness", Valor::Num(60.0)), ("blueBlurriness", Valor::Num(60.0))],
        "ec.blur.directional" => &[("length", Valor::Num(60.0))],
        "ec.blur.gaussianlegacy" => &[("blurriness", Valor::Num(25.0))],
        "ec.blur.reduceflicker" => &[("softness", Valor::Num(10.0))],
        "ec.blur.gaussian" => &[("blurriness", Valor::Num(20.0))],
        "ec.blur.fastbox" => &[("radius", Valor::Num(12.0))],
        "ec.blur.smart" => &[("radius", Valor::Num(20.0)), ("threshold", Valor::Num(80.0))],
        "ec.blur.unsharp" => &[("amount", Valor::Num(160.0)), ("radius", Valor::Num(5.0))],
        "ec.channel.arithmetic" => &[("greenValue", Valor::Num(63.75))],
        "ec.channel.calculations" => &[("inputChannel", Valor::Op(3))],
        "ec.channel.cccomposite" => &[("compositeOriginal", Valor::Op(6))],
        "ec.channel.minimax" => &[("radius", Valor::Num(6.0))],
        "ec.channel.setchannels" => &[("setRedTo", Valor::Op(2))],
        "ec.channel.shiftchannels" => &[("takeRedFrom", Valor::Op(3))],
        "ec.channel.solidcomposite" => &[("blendingMode", Valor::Op(11))],
        "ec.color.brightnesscontrast" => &[("brightness", Valor::Num(15.0)), ("contrast", Valor::Num(35.0))],
        "ec.color.cccolorneutralizer" => &[("brights", Valor::Num(100.0))],
        "ec.color.cccoloroffset" => &[("greenPhase", Valor::Num(30.0))],
        "ec.color.cckernel" => &[("scale", Valor::Num(0.75))],
        "ec.color.changecolor" => &[("hueTransform", Valor::Num(90.0)), ("matchingTolerance", Valor::Num(50.0))],
        "ec.color.channelmixer" => &[("gb", Valor::Num(50.0))],
        "ec.color.colorbalance" => &[("midRed", Valor::Num(45.0)), ("shadowBlue", Valor::Num(30.0))],
        "ec.color.colorbalancehls" => &[("saturation", Valor::Num(40.0))],
        "ec.color.exposure" => &[("master/exposure", Valor::Num(1.0))],
        "ec.color.gammapedestalgain" => &[("blackStretch", Valor::Num(4.0))],
        "ec.color.huesaturation" => &[("hue", Valor::Num(40.0)), ("saturation", Valor::Num(25.0))],
        "ec.color.leavecolor" => &[("amount", Valor::Num(50.0))],
        "ec.color.levels" => &[("inBlack", Valor::Num(0.05)), ("inWhite", Valor::Num(0.75)), ("gamma", Valor::Num(1.3))],
        "ec.color.levelsic" => &[("green/greenOutBlack", Valor::Num(0.25))],
        "ec.color.lumetri" => &[("basicCorrection/tone/contrast", Valor::Num(40.0)), ("basicCorrection/tone/exposure", Valor::Num(0.5)), ("basicCorrection/saturation", Valor::Num(130.0))],
        "ec.color.ociocdl" => &[("saturation", Valor::Num(1.75))],
        "ec.color.ociolook" => &[("look", Valor::Op(1))],
        "ec.color.psarbitrarymap" => &[("phase", Valor::Num(63.75))],
        "ec.color.selectivecolor" => &[("details/cyans/cyansBlack", Valor::Num(-100.0))],
        "ec.color.vibrance" => &[("vibrance", Valor::Num(100.0))],
        "ec.distort.bezierwarp" => &[("topLeftTangent", Valor::Punto([0.3, 0.3]))],
        "ec.distort.bulge" => &[("hradius", Valor::Num(400.0)), ("vradius", Valor::Num(400.0))],
        "ec.distort.ccbender" => &[("amount", Valor::Num(-25.0))],
        "ec.distort.ccbendit" => &[("bend", Valor::Num(30.0))],
        "ec.distort.ccgriddler" => &[("horizontalScale", Valor::Num(75.25))],
        "ec.distort.ccpowerpin" => &[("expandBottom", Valor::Num(25.0))],
        "ec.distort.ccripplepulse" => &[("pulseLevel", Valor::Num(60.0)), ("amplitude", Valor::Num(250.0))],
        "ec.distort.ccslant" => &[("height", Valor::Num(125.0))],
        "ec.distort.ccsmear" => &[("radius", Valor::Num(400.0))],
        "ec.distort.cornerpin" => &[("ul", Valor::Punto([0.08, 0.12])), ("lr", Valor::Punto([0.92, 0.88]))],
        "ec.distort.magnify" => &[("size", Valor::Num(550.0))],
        "ec.distort.offset" => &[("shift", Valor::Punto([0.7, 0.5]))],
        "ec.distort.opticscompensation" => &[("fieldOfView", Valor::Num(45.0))],
        "ec.distort.polar" => &[("interpolation", Valor::Num(25.0))],
        "ec.distort.ripple" => &[("waveHeight", Valor::Num(100.0))],
        "ec.distort.spherize" => &[("radius", Valor::Num(625.0))],
        "ec.distort.transform" => &[("scaleHeight", Valor::Num(85.0)), ("scaleWidth", Valor::Num(85.0)), ("rotation", Valor::Num(5.0))],
        "ec.distort.twirl" => &[("angle", Valor::Num(90.0))],
        "ec.distort.twirllegacy" => &[("angle", Valor::Num(90.0)), ("radius", Valor::Num(60.0))],
        "ec.distort.upscale" => &[("scale", Valor::Num(130.0))],
        "ec.distort.warp" => &[("horizontalDistortion", Valor::Num(25.0))],
        "ec.generate.beam" => &[("length", Valor::Num(90.0)), ("time", Valor::Num(50.0)), ("startThickness", Valor::Num(30.0)), ("endThickness", Valor::Num(30.0))],
        "ec.generate.radiowaves" => &[("waveMotion/lifespan", Valor::Num(10.0))],
        "ec.generate.vegas" => &[("width", Valor::Num(20.0))],
        "ec.key.colorkey" => &[("colorTolerance", Valor::Num(127.5))],
        "ec.key.extract" => &[("blackPoint", Valor::Num(63.75))],
        "ec.key.linearcolor" => &[("keyColor", Valor::Color([0.05, 0.1, 0.15, 1.0]))],
        "ec.key.luma" => &[("threshold", Valor::Num(63.75))],
        "fc:rotate_3d" => &[("rot_y", Valor::Num(35.0))],
        "fc:rounded_crop" => &[("left", Valor::Num(6.0)), ("right", Valor::Num(6.0)), ("top", Valor::Num(6.0)), ("bottom", Valor::Num(6.0)), ("radius", Valor::Num(80.0))],
        "ec.key.unmult" => &[("blackLevel", Valor::Num(25.0))],
        "ec.matte.simplechoker" => &[("chokeMatte", Valor::Num(8.0)), ("choke", Valor::Num(8.0)), ("amount", Valor::Num(8.0))],
        "ec.noise.noisealpha" => &[("amount", Valor::Num(60.0))],
        "ec.noise.dustscratches" => &[("radius", Valor::Num(8.0))],
        "ec.noise.median" => &[("radius", Valor::Num(8.0))],
        "ec.noise.noise" => &[("amount", Valor::Num(25.0))],
        "ec.noise.noisehls" => &[("saturation", Valor::Num(50.0))],
        "ec.noise.noisehlsauto" => &[("saturation", Valor::Num(50.0))],
        "ec.noise.removegrain" => &[("noiseReductionSettings/noiseReduction", Valor::Num(4.0))],
        "ec.obsolete.basic3d" => &[("distanceToImage", Valor::Num(50.0))],
        "ec.obsolete.basictext" => &[("size", Valor::Num(200.0))],
        "ec.obsolete.gaussianlegacy" => &[("blurriness", Valor::Num(50.0))],
        "ec.obsolete.lightning" => &[("width", Valor::Num(30.0)), ("amplitude", Valor::Num(20.0)), ("branching", Valor::Num(0.8))],
        "ec.obsolete.pathtext" => &[("character/size", Valor::Num(160.0))],
        "ec.perspective.dropshadow" => &[("distance", Valor::Num(30.0)), ("softness", Valor::Num(40.0)), ("opacity", Valor::Num(75.0))],
        "ec.perspective.radialshadow" => &[("opacity", Valor::Num(75.0))],
        "ec.sim.carddance" => &[("yRotation/yRotOffset", Valor::Num(25.0)), ("xRotation/xRotOffset", Valor::Num(15.0))],
        "ec.sim.ccrainfall" => &[("size", Valor::Num(10.0))],
        "ec.sim.ccscatterize" => &[("leftTwist", Valor::Num(30.0))],
        "ec.sim.ccsnowfall" => &[("size", Valor::Num(11.5))],
        "ec.stylize.ccburnfilm" => &[("burn", Valor::Num(25.0))],
        "ec.stylize.ccrepetile" => &[("expandLeft", Valor::Num(125.0))],
        "ec.stylize.motiontile" => &[("tileWidth", Valor::Num(50.0)), ("tileHeight", Valor::Num(50.0))],
        "ec.stylize.scatter" => &[("amount", Valor::Num(31.75))],
        "ec.stylize.strobe" => &[("strobeDuration", Valor::Num(0.3)), ("blendWithOriginal", Valor::Num(40.0))],
        "ec.text.numbers" => &[("size", Valor::Num(220.0)), ("format/value", Valor::Num(100.0))],
        "ec.text.timecode" => &[("textSize", Valor::Num(120.0))],
        "ec.transition.blockdissolve" => &[("completion", Valor::Num(25.0))],
        "ec.transition.cardwipe" => &[("rotationJitter/yRotJitterAmount", Valor::Num(22.5))],
        "ec.transition.ccglasswipe" => &[("completion", Valor::Num(25.0))],
        "ec.transition.ccgridwipe" => &[("completion", Valor::Num(25.0))],
        "ec.transition.ccimagewipe" => &[("completion", Valor::Num(25.0))],
        "ec.transition.ccjaws" => &[("completion", Valor::Num(25.0))],
        "ec.transition.cclightwipe" => &[("completion", Valor::Num(25.0))],
        "ec.transition.cclinesweep" => &[("completion", Valor::Num(25.0))],
        "ec.transition.ccradialscalewipe" => &[("completion", Valor::Num(25.0))],
        "ec.transition.ccscalewipe" => &[("stretch", Valor::Num(12.5))],
        "ec.transition.cctwister" => &[("completion", Valor::Num(25.0))],
        "ec.transition.ccwarpomatic" => &[("completion", Valor::Num(25.0))],
        "ec.transition.gradientwipe" => &[("completion", Valor::Num(25.0))],
        "ec.transition.iriswipe" => &[("outerRadius", Valor::Num(500.0))],
        "ec.transition.linearwipe" => &[("completion", Valor::Num(25.0))],
        "ec.transition.radialwipe" => &[("completion", Valor::Num(25.0))],
        "ec.transition.venetian" => &[("completion", Valor::Num(25.0))],
        "ec.utility.colorprofileconverter" => &[("outputProfile", Valor::Op(3))],
        "ec.utility.hdrcompander" => &[("gamma", Valor::Num(0.775))],
        "ec.vr.denoise" => &[("noiseLevel", Valor::Num(100.0))],
        "ec.vr.rotatesphere" => &[("tilt", Valor::Num(30.0))],
        "ec.vr.sharpen" => &[("sharpenAmount", Valor::Num(100.0))],
        _ => &[],
    }
}

/// Punto automático: se guarda como fracción solo si el de fábrica era «centro».
fn fc_es_centro(d: &ct::ParamDef) -> bool {
    matches!(&d.default, craft_trans::ParamValue::Vec2(v) if v.x.is_nan())
}

pub struct Entorno {
    pub dur: f64,
    pub fps: f64,
}

impl craft_trans::vfx::FxEnv for Entorno {
    fn clip_seconds(&self) -> f64 {
        self.dur
    }
    fn frame_rate(&self) -> f64 {
        self.fps
    }
}

/// El efecto con sus valores en el segundo `t`.
pub fn valores_fc(f: &FxVideo, d: &ct::EffectDef, t: f64, tam: [f64; 2]) -> ct::EffectInstance {
    let mut e = d.instance();
    for p in &d.params {
        let Some(v) = f.p.get(p.id).map(|x| x.en(t)) else { continue };
        let q = match v {
            Valor::Num(x) => craft_trans::ParamValue::Float(x),
            Valor::Color(c) => craft_trans::ParamValue::Color(c),
            Valor::Si(b) => craft_trans::ParamValue::Bool(b),
            Valor::Op(i) => craft_trans::ParamValue::Choice(i),
            Valor::Punto(q) => {
                let q = if fc_es_centro(p) { [q[0] * tam[0], q[1] * tam[1]] } else { q };
                craft_trans::ParamValue::Vec2(craft_trans::Vec2::new(q[0], q[1]))
            }
            Valor::Texto(_) => continue,
        };
        e.params.insert(p.id.to_string(), ct::Param { value: q });
    }
    e
}

/// Efecto de FilmCraft: trabaja en luz lineal sobre la capa completa.
fn aplicar_fc(f: &FxVideo, d: &ct::EffectDef, t: f64, dur: f64, fps: f64, buf: ec::Buf, tam: [f64; 2]) -> ec::Buf {
    let e = valores_fc(f, d, t, tam);
    let mut img = a_lineal(&buf.img);
    let env = Entorno { dur, fps };
    let cx = craft_trans::vfx::FxCtx { t: craft_trans::Tick, px_scale: buf.scale as f32, seconds: t, env: Some(&env), frame_rate: fps };
    craft_trans::vfx::apply(&mut img, &e, &cx);
    ec::Buf { img: de_lineal(&img), ..buf }
}

/// Cuadros del pasado de una capa, para los efectos de tiempo.
pub struct Pasado {
    /// (segundo de la capa, imagen)
    v: Vec<(f64, ec::Buf)>,
}

impl ec::EffectHost for Pasado {
    fn layer(&self, _: u64, _: bool) -> Option<ec::LayerPixels> {
        None
    }
    fn audio(&self, _: u64, _: f64, _: usize, _: u32) -> Option<Vec<f32>> {
        None
    }
    fn self_at(&self, t: f64, _: usize) -> Option<ec::Buf> {
        self.v.iter().min_by(|a, b| (a.0 - t).abs().total_cmp(&(b.0 - t).abs())).map(|x| x.1.clone())
    }
}

/// Efectos que miran otros momentos del clip.
pub const DE_TIEMPO: [&str; 6] = ["ec.time.echo", "ec.time.posterizetime", "ec.time.timedifference", "ec.time.ccwidetime", "ec.time.ccforcemotionblur", "ec.time.pixelmotionblur"];

/// Los valores de un efecto de EffectCraft en el segundo `t`.
pub fn valores_ec(f: &FxVideo, s: &ec::EffectSpec, t: f64, tam: [f64; 2]) -> ec::Params {
    let values = s
        .params
        .iter()
        .map(|p| {
            let d = ec::default_value(p, tam);
            let v = match (f.p.get(p.id).map(|x| x.en(t)), &d) {
                (Some(Valor::Num(x)), Value::Scalar(_)) => Value::Scalar(x),
                // Solo los puntos de tipo posición van en fracción del cuadro.
                (Some(Valor::Punto(q)), Value::Vec2(_)) => match p.ui {
                    effectcraft_project::ParamUi::Point => Value::Vec2([q[0] * tam[0], q[1] * tam[1]]),
                    _ => Value::Vec2(q),
                },
                (Some(Valor::Color(c)), Value::Color(_)) => Value::Color(c.map(f64::from)),
                (Some(Valor::Si(b)), Value::Bool(_)) => Value::Bool(b),
                (Some(Valor::Op(i)), Value::Enum(_)) => Value::Enum(i),
                (Some(Valor::Texto(s)), Value::Str(_)) => Value::Str(s),
                _ => d,
            };
            (p.id.to_string(), v)
        })
        .collect();
    ec::Params { values }
}

#[allow(clippy::too_many_arguments)]
pub fn aplicar_fx(f: &FxVideo, t: f64, dur: f64, fps: f64, buf: ec::Buf, tam: [f64; 2], pasado: Option<&Pasado>) -> ec::Buf {
    if let Some(d) = f.tipo.strip_prefix("fc:").and_then(ct::find_video) {
        let respaldo = buf.clone();
        return std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| aplicar_fc(f, d, t, dur, fps, buf, tam))).unwrap_or(respaldo);
    }
    if f.tipo.starts_with("bc:") {
        let img = crate::vpropios::aplicar(f, t, &buf.img, buf.scale);
        return ec::Buf { img, ..buf };
    }
    let Some(s) = ec::find(&f.tipo) else { return buf };
    let params = valores_ec(f, s, t, tam);
    let respaldo = buf.clone();
    // Un efecto que truene no debe tirar la app: se salta.
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let env = ec::EffectEnv { comp_time: t, frame_rate: fps, host: pasado.map(|p| p as &dyn ec::EffectHost), ..Default::default() };
        let ctx = ec::EffectCtx { params: &params, time: t, layer_size: tam, seed: f.id as u32, adjustment: false, env };
        ec::apply(s, &ctx, buf)
    }));
    match r {
        Ok(mut b) if b.img.width > 0 && b.img.height > 0 => {
            b.img.data.par_iter_mut().for_each(|p| {
                if !p.iter().all(|v| v.is_finite()) {
                    *p = [0.0; 4];
                }
            });
            b
        }
        _ => respaldo,
    }
}

// ---- transiciones

fn tablas() -> &'static [f32; 4097] {
    static T: OnceLock<Box<[f32; 4097]>> = OnceLock::new();
    T.get_or_init(|| Box::new(std::array::from_fn(|i| craft_trans::srgb_to_linear(i as f32 / 4096.0))))
}

fn a_lineal(i: &Image) -> craft_trans::image::Image {
    let t = tablas();
    let mut o = craft_trans::image::Image::new(i.width as usize, i.height as usize);
    o.px.par_chunks_mut(4).zip(i.data.par_iter()).for_each(|(o, p)| {
        let a = p[3].clamp(0.0, 1.0);
        for k in 0..3 {
            let s = if a > 0.0 { (p[k] / a).clamp(0.0, 1.0) } else { 0.0 };
            o[k] = t[(s * 4096.0) as usize] * a;
        }
        o[3] = a;
    });
    o
}

fn de_lineal(i: &craft_trans::image::Image) -> Image {
    let mut o = Image::new(i.w as u32, i.h as u32);
    o.data.par_iter_mut().zip(i.px.par_chunks(4)).for_each(|(o, p)| {
        let a = p[3].clamp(0.0, 1.0);
        for k in 0..3 {
            o[k] = if a > 0.0 { craft_trans::linear_to_srgb((p[k] / a).clamp(0.0, 1.0)) * a } else { 0.0 };
        }
        o[3] = a;
    });
    o
}

pub fn transicion(t: &Transicion, a: &Image, b: &Image, p: f32, escala: f32) -> Image {
    let Some(d) = ct::find_transition(&t.tipo) else { return if p < 0.5 { a.clone() } else { b.clone() } };
    let mut e = d.instance();
    for (k, v) in &t.p {
        if let Some(q) = e.params.get_mut(k) {
            q.value = match v {
                Valor::Num(x) => craft_trans::ParamValue::Float(*x),
                Valor::Color(c) => craft_trans::ParamValue::Color(*c),
                Valor::Si(b) => craft_trans::ParamValue::Bool(*b),
                Valor::Op(i) => craft_trans::ParamValue::Choice(*i),
                _ => continue,
            };
        }
    }
    let (la, lb) = (a_lineal(a), a_lineal(b));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        craft_trans::transitions::apply_scaled(&e, &la, &lb, p, escala)
    }));
    r.map(|i| de_lineal(&i)).unwrap_or_else(|_| if p < 0.5 { a.clone() } else { b.clone() })
}

pub fn transicion_nueva(tipo: &str) -> Transicion {
    Transicion { tipo: tipo.into(), dur: 1.0, p: BTreeMap::new() }
}

// ---- capas

/// De dónde salen los cuadros decodificados: clip, segundo del archivo, tamaño.
pub trait Fuente {
    fn cuadro(&mut self, c: &ClipVideo, t: f64, ancho: usize, alto: usize) -> Option<Arc<Cuadro>>;
    /// Igual, pero quien lo pide sabe cambiarle el tamaño: puede llegar más grande o más chico.
    fn cuadro_libre(&mut self, c: &ClipVideo, t: f64, ancho: usize, alto: usize) -> Option<Arc<Cuadro>> {
        self.cuadro(c, t, ancho, alto)
    }
    /// Los últimos cuadros que se vieron de ese clip.
    fn recientes(&self, _: &ClipVideo) -> Vec<Arc<Cuadro>> {
        vec![]
    }
}

/// Lo que se le pide a la fuente para dibujar `c` en `t` sobre un lienzo de `w` × `h`: segundo del archivo y tamaño.
/// `None`: no sale de un archivo (título, color, ajuste).
pub fn pedido(c: &ClipVideo, t: f64, w: usize, h: usize) -> Option<(f64, usize, usize)> {
    let (aw, ah) = ajuste(c, w, h);
    match c.contenido {
        Contenido::Archivo => Some((c.fuente(t).clamp(0.0, (c.total - 0.02).max(0.0)), aw, ah)),
        Contenido::Imagen => Some((0.0, aw, ah)),
        _ => None,
    }
}

/// El clip pegado a la izquierda de `c` en su capa: el que sale en la transición de entrada.
pub fn previo<'a>(p: &'a Proyecto, c: &ClipVideo) -> Option<&'a ClipVideo> {
    p.video.iter().find(|q| q.capa == c.capa && q.id != c.id && (q.fin() - c.inicio).abs() < 0.05)
}

/// Algún clip tiene un efecto que mira cuadros anteriores (`Fuente::recientes`).
pub fn usa_pasado(p: &Proyecto) -> bool {
    p.video.iter().flat_map(|c| &c.fx).any(|x| x.activo && DE_TIEMPO.contains(&x.tipo.as_str()))
}

/// Todo lo que `componer` le pedirá a la fuente para el segundo `t`, en orden: (clip, segundo del archivo, ancho, alto).
pub fn pedidos(p: &Proyecto, t: f64, w: usize, h: usize) -> Vec<(Cow<'_, ClipVideo>, f64, usize, usize)> {
    let mut v = vec![];
    for (i, capa) in p.capas.iter().enumerate() {
        if !capa.visible {
            continue;
        }
        let Some(c) = p.video_en(i, t) else { continue };
        if atajo(p, i, c, t, w, h) {
            v.extend(pedido(c, t, w, h).map(|(ts, aw, ah)| (Cow::Borrowed(c), ts, aw, ah)));
            return v;
        }
        let propio = pedido(c, t, w, h);
        let (cw, ch) = propio.map_or((w, h), |x| (x.1, x.2));
        v.extend(propio.map(|(ts, aw, ah)| (Cow::Borrowed(c), ts, aw, ah)));
        v.extend(crate::nodos::pedidos(c, t, cw, ch).into_iter().map(|(m, ts, a, b)| (Cow::Owned(m), ts, a, b)));
        if c.entrada.as_ref().is_some_and(|x| t < c.inicio + x.dur) {
            if let Some(q) = previo(p, c) {
                let propio = pedido(q, t, w, h);
                let (cw, ch) = propio.map_or((w, h), |x| (x.1, x.2));
                v.extend(propio.map(|(ts, aw, ah)| (Cow::Borrowed(q), ts, aw, ah)));
                v.extend(crate::nodos::pedidos(q, t, cw, ch).into_iter().map(|(m, ts, a, b)| (Cow::Owned(m), ts, a, b)));
            }
        }
    }
    v
}

/// Camino corto: un clip limpio que llena el cuadro y tapa lo de abajo.
fn atajo(p: &Proyecto, i: usize, c: &ClipVideo, t: f64, w: usize, h: usize) -> bool {
    c.simple() && ajuste(c, w, h) == (w, h) && p.capas.iter().enumerate().skip(i + 1).all(|(j, x)| !x.visible || p.video_en(j, t).is_none())
}

/// Fuente con los cuadros ya decodificados (`pedidos`), uno por clip: para componer en otro hilo.
pub struct Prestado {
    pub cuadros: HashMap<u64, Arc<Cuadro>>,
    /// Los últimos cuadros que vio la fuente de cada clip con efectos de tiempo, en orden.
    pub recientes: HashMap<u64, Vec<Arc<Cuadro>>>,
}

impl Fuente for Prestado {
    fn cuadro(&mut self, c: &ClipVideo, _: f64, ancho: usize, alto: usize) -> Option<Arc<Cuadro>> {
        self.cuadros.get(&c.id).filter(|q| (q.ancho, q.alto) == (ancho, alto)).cloned()
    }
    fn cuadro_libre(&mut self, c: &ClipVideo, _: f64, _: usize, _: usize) -> Option<Arc<Cuadro>> {
        self.cuadros.get(&c.id).cloned()
    }
    fn recientes(&self, c: &ClipVideo) -> Vec<Arc<Cuadro>> {
        self.recientes.get(&c.id).cloned().unwrap_or_default()
    }
}

/// Si el clip tiene algún efecto que mira otros momentos (ver `DE_TIEMPO`).
pub fn de_tiempo(c: &ClipVideo) -> bool {
    c.fx.iter().any(|x| x.activo && DE_TIEMPO.contains(&x.tipo.as_str()))
}

pub fn imagen_de(c: &ClipVideo, t: f64, p: &Proyecto, w: usize, h: usize, f: &mut dyn Fuente) -> Option<ec::Buf> {
    let k = h as f64 / p.alto.max(1) as f64;
    let img = match &c.contenido {
        Contenido::Titulo(x) => titulos::dibujar_en(x, w, h, t - c.inicio, c.dur),
        Contenido::Forma(x) => crate::formas::dibujar(x, w, h),
        Contenido::Color(q) => Image::filled(w as u32, h as u32, [q[0] * q[3], q[1] * q[3], q[2] * q[3], q[3]]),
        Contenido::Ajuste => return None,
        Contenido::Archivo | Contenido::Imagen => {
            let (ts, aw, ah) = pedido(c, t, w, h)?;
            let q = f.cuadro(c, ts, aw, ah)?;
            Image::from_rgba8(q.ancho as u32, q.alto as u32, &q.en_rgba())
        }
    };
    Some(ec::Buf { img, offset: [0.0; 2], scale: k })
}

/// Los cuadros anteriores, si algún efecto del clip los mira.
pub fn pasado_de(c: &ClipVideo, f: &dyn Fuente, escala: f64) -> Option<Pasado> {
    de_tiempo(c).then(|| Pasado {
        v: f.recientes(c)
            .iter()
            .map(|q| {
                let t = (q.t - c.desde) / c.mov.velocidad.max(0.01);
                (t, ec::Buf { img: Image::from_rgba8(q.ancho as u32, q.alto as u32, &q.en_rgba()), offset: [0.0; 2], scale: escala })
            })
            .collect(),
    })
}

/// El clip ya con efectos y movimiento, en un lienzo transparente de `w` × `h`.
/// Efectos que son del cuadro (la pantalla), no de la imagen: van después de mover y hacer zoom, para que las barras
/// de cine o un marco se queden quietos aunque el video crezca.
pub fn del_cuadro(tipo: &str) -> bool {
    matches!(tipo, "bc:barras" | "bc:marco" | "bc:visor" | "bc:rollo" | "bc:redondo" | "bc:apertura" | "bc:persianas" | "ec.stylize.ccvignette")
}

/// Los efectos del cuadro de un clip, sobre su imagen ya movida (del tamaño del lienzo).
fn efectos_del_cuadro(c: &ClipVideo, tl: f64, p: &Proyecto, img: Image, w: usize, h: usize) -> Image {
    let tam = [p.ancho as f64, p.alto as f64];
    let mut b = ec::Buf { img, offset: [0.0; 2], scale: h as f64 / p.alto.max(1) as f64 };
    for x in c.fx.iter().filter(|x| x.activo && del_cuadro(&x.tipo)) {
        b = aplicar_fx(x, tl, c.dur, p.fps, b, tam, None);
    }
    // Si alguno lo movió o lo cambió de tamaño, de vuelta al lienzo.
    if b.offset == [0.0; 2] && (b.img.width as usize, b.img.height as usize) == (w, h) {
        return b.img;
    }
    let mut o = Image::new(w as u32, h as u32);
    let (dx, dy) = (-b.offset[0].round() as i64, -b.offset[1].round() as i64);
    o.data.par_chunks_mut(w).enumerate().for_each(|(y, fila)| {
        for (x, q) in fila.iter_mut().enumerate() {
            *q = b.img.get(x as i64 + dx, y as i64 + dy);
        }
    });
    o
}

fn dibujar_clip(c: &ClipVideo, t: f64, p: &Proyecto, w: usize, h: usize, f: &mut dyn Fuente) -> Option<Image> {
    let tl = t - c.inicio;
    let c = &*c.en_su_momento(tl);
    let mut b = imagen_de(c, t, p, w, h, f)?;
    let (bw, bh) = (b.img.width as f64, b.img.height as f64);
    let r = c.mov.recorte;
    if r.iter().any(|v| *v > 0.0) {
        let (iw, ih) = (b.img.width as usize, b.img.height as usize);
        let x0 = (iw as f32 * r[0] / 100.0) as usize;
        let y0 = (ih as f32 * r[1] / 100.0) as usize;
        let x1 = iw - ((iw as f32 * r[2] / 100.0) as usize).min(iw);
        let y1 = ih - ((ih as f32 * r[3] / 100.0) as usize).min(ih);
        b.img.data.par_chunks_mut(iw).enumerate().for_each(|(y, fila)| {
            for (x, q) in fila.iter_mut().enumerate() {
                if x < x0 || x >= x1 || y < y0 || y >= y1 {
                    *q = [0.0; 4];
                }
            }
        });
    }
    let tam = [bw / b.scale, bh / b.scale];
    let pasado = pasado_de(c, f, b.scale);
    for x in c.fx.iter().filter(|x| x.activo && !del_cuadro(&x.tipo)) {
        b = aplicar_fx(x, tl, c.dur, p.fps, b, tam, pasado.as_ref());
    }
    if let Some(g) = c.nodos.as_ref().filter(|g| g.activo) {
        let medios = crate::nodos::medios(c, t, p, &b, f);
        b = crate::nodos::evaluar(g, b, tl, &medios, &|f, b| aplicar_fx(f, tl, c.dur, p.fps, b, tam, pasado.as_ref()));
    }
    let de_cuadro = c.fx.iter().any(|x| x.activo && del_cuadro(&x.tipo));
    let m = &c.mov;
    let esc = (m.escala.en(tl).num() / 100.0).max(0.0001);
    let giro = m.giro.en(tl).num().to_radians();
    let cx = w as f64 / 2.0 + m.x.en(tl).num() / 100.0 * w as f64;
    let cy = h as f64 / 2.0 + m.y.en(tl).num() / 100.0 * h as f64;
    // Centro de la capa dentro del buffer (los efectos pueden haberlo agrandado).
    // El punto de apoyo se queda quieto al girar y escalar.
    let (ax, ay) = (m.ancla[0] / 100.0 * bw, m.ancla[1] / 100.0 * bh);
    let (ox, oy) = (bw / 2.0 + b.offset[0] + ax, bh / 2.0 + b.offset[1] + ay);
    let (cx, cy) = (cx + ax, cy + ay);
    let (ex, ey) = (esc * (m.ancho.en(tl).num() / 100.0), esc * (m.alto.en(tl).num() / 100.0));
    // Una escala negativa voltea; cero no se dibuja.
    let firme = |v: f64| if v.abs() < 0.0001 { 0.0001 } else { v };
    let (ex, ey) = (firme(ex), firme(ey));
    let op = (m.opacidad.en(tl).num() / 100.0).clamp(0.0, 1.0) as f32;
    // Sin mover ni estirar, la imagen sirve tal cual: no se copia.
    if ex == 1.0 && ey == 1.0 && giro == 0.0 && ox == cx && oy == cy && (b.img.width as usize, b.img.height as usize) == (w, h) {
        let mut img = if de_cuadro { efectos_del_cuadro(c, tl, p, b.img, w, h) } else { b.img };
        if op < 1.0 {
            img.data.par_iter_mut().for_each(|q| *q = q.map(|v| v * op));
        }
        return Some(img);
    }
    let mut o = Image::new(w as u32, h as u32);
    if ex == 1.0 && ey == 1.0 && giro == 0.0 && cx == cx.round() && cy == cy.round() && (ox - cx).fract() == 0.0 && (oy - cy).fract() == 0.0 {
        let (dx, dy) = ((ox - cx) as i64, (oy - cy) as i64);
        o.data.par_chunks_mut(w).enumerate().for_each(|(y, fila)| {
            for (x, q) in fila.iter_mut().enumerate() {
                *q = b.img.get(x as i64 + dx, y as i64 + dy);
            }
        });
    } else {
        // Al achicar mucho se promedian varias muestras para que no brille.
        let n = ((1.0 / ex.abs().min(ey.abs())).ceil() as i32).clamp(1, 4);
        let (s, cs) = giro.sin_cos();
        o.data.par_chunks_mut(w).enumerate().for_each(|(y, fila)| {
            for (x, q) in fila.iter_mut().enumerate() {
                let mut acc = [0f32; 4];
                for j in 0..n {
                    for i in 0..n {
                        let px = x as f64 + (i as f64 + 0.5) / n as f64 - cx;
                        let py = y as f64 + (j as f64 + 0.5) / n as f64 - cy;
                        let u = (px * cs + py * s) / ex + ox - 0.5;
                        let v = (-px * s + py * cs) / ey + oy - 0.5;
                        let z = b.img.sample_bilinear(u, v);
                        (0..4).for_each(|k| acc[k] += z[k]);
                    }
                }
                *q = acc.map(|v| v / (n * n) as f32);
            }
        });
    }
    if de_cuadro {
        o = efectos_del_cuadro(c, tl, p, o, w, h);
    }
    if op < 1.0 {
        o.data.par_iter_mut().for_each(|q| *q = q.map(|v| v * op));
    }
    Some(o)
}

/// Un texto o figura que se ve igual todo el tiempo.
pub fn quieto(c: &ClipVideo, t: f64) -> bool {
    let fijo = match &c.contenido {
        Contenido::Titulo(x) => x.anim.is_none(),
        Contenido::Forma(_) => true,
        _ => false,
    };
    fijo && c.fx.iter().all(|f| !f.activo)
        && c.nodos.as_ref().is_none_or(|g| !g.activo)
        && c.mov.props().iter().all(|p| p.claves.is_empty())
        && c.extra.is_empty()
        && c.entrada.as_ref().is_none_or(|x| t >= c.inicio + x.dur)
        && c.salida.as_ref().is_none_or(|x| t <= c.fin() - x.dur)
}

/// Un recuadro de imagen: (x, y) y lo que va ahí.
pub struct Recuadro {
    pub x: usize,
    pub y: usize,
    pub img: Arc<Image>,
}

/// El dibujo de un clip quieto, recortado a lo que tiene, guardado para el siguiente cuadro.
pub fn guardado(c: &ClipVideo, t: f64, p: &Proyecto, w: usize, h: usize, f: &mut dyn Fuente) -> Arc<Recuadro> {
    use std::hash::{DefaultHasher, Hash, Hasher};
    static G: OnceLock<std::sync::Mutex<Vec<(u64, Arc<Recuadro>)>>> = OnceLock::new();
    let mut k = DefaultHasher::new();
    (w, h, p.ancho, p.alto, serde_json::to_string(&(&c.contenido, &c.mov)).unwrap_or_default()).hash(&mut k);
    let k = k.finish();
    let g = G.get_or_init(Default::default);
    if let Some(q) = g.lock().unwrap().iter().find(|x| x.0 == k) {
        return q.1.clone();
    }
    let img = dibujar_clip(c, t, p, w, h, f).unwrap_or_else(|| Image::new(w as u32, h as u32));
    let q = Arc::new(match img.alpha_bounds() {
        Some((x0, y0, x1, y1)) => {
            let mut r = Image::new(x1 - x0, y1 - y0);
            let rw = r.width as usize;
            for (y, fila) in r.data.chunks_mut(rw).enumerate() {
                let i = (y0 as usize + y) * w + x0 as usize;
                fila.copy_from_slice(&img.data[i..i + rw]);
            }
            Recuadro { x: x0 as usize, y: y0 as usize, img: Arc::new(r) }
        }
        None => Recuadro { x: 0, y: 0, img: Arc::new(Image::new(0, 0)) },
    });
    let mut g = g.lock().unwrap();
    if g.len() >= 24 {
        g.remove(0);
    }
    g.push((k, q.clone()));
    q
}

/// Encima, en modo normal, solo donde hay algo.
fn encimar(l: &mut Image, q: &Recuadro) {
    let (w, rw) = (l.width as usize, q.img.width as usize);
    if rw == 0 {
        return;
    }
    let alto = q.img.height as usize;
    l.data[q.y * w..(q.y + alto) * w].par_chunks_mut(w).zip(q.img.data.par_chunks(rw)).for_each(|(d, s)| {
        for (d, s) in d[q.x..q.x + rw].iter_mut().zip(s) {
            let k = 1.0 - s[3];
            *d = [s[0] + d[0] * k, s[1] + d[1] * k, s[2] + d[2] * k, s[3] + d[3] * k];
        }
    });
}

/// Compone el cuadro del segundo `t`, en la tarjeta gráfica si se puede. Opaco, RGBA de 8 bits.
pub fn componer(p: &Proyecto, t: f64, w: usize, h: usize, f: &mut dyn Fuente) -> Vec<u8> {
    let mut hay = p.capas.iter().enumerate().filter(|x| x.1.visible).filter_map(|(i, _)| p.video_en(i, t).map(|c| (i, c))).peekable();
    let corto = hay.peek().is_none() || hay.any(|(i, c)| atajo(p, i, c, t, w, h));
    if !corto {
        if let Some(v) = crate::vgpu::componer(p, t, w, h, f, crate::vgpu::Salida::Rgba) {
            return v;
        }
    }
    componer_cpu(p, t, w, h, f)
}

/// Lo mismo en planos 4:2:0 (tamaño par), como lo quiere el codificador.
pub fn componer_yuv(p: &Proyecto, t: f64, w: usize, h: usize, f: &mut dyn Fuente) -> Vec<u8> {
    let hay = p.capas.iter().enumerate().filter(|x| x.1.visible).filter_map(|(i, _)| p.video_en(i, t).map(|c| (i, c)));
    match hay.into_iter().find(|(i, c)| atajo(p, *i, c, t, w, h)) {
        // Sin cambios: el cuadro pasa tal cual.
        Some((_, c)) => {
            if let Some(q) = pedido(c, t, w, h).and_then(|(ts, aw, ah)| f.cuadro(c, ts, aw, ah)).filter(|q| !q.yuv.is_empty()) {
                return q.yuv.clone();
            }
        }
        None => {
            if let Some(v) = crate::vgpu::componer(p, t, w, h, f, crate::vgpu::Salida::I420) {
                return v;
            }
        }
    }
    a_yuv(&componer(p, t, w, h, f), w, h)
}

/// Lo mismo, en el procesador.
pub fn componer_cpu(p: &Proyecto, t: f64, w: usize, h: usize, f: &mut dyn Fuente) -> Vec<u8> {
    let negro = [0.0, 0.0, 0.0, 1.0];
    let mut lienzo: Option<Image> = None;
    let escala = h as f32 / p.alto.max(1) as f32;
    for (i, capa) in p.capas.iter().enumerate() {
        if !capa.visible {
            continue;
        }
        let Some(c) = p.video_en(i, t) else { continue };
        if atajo(p, i, c, t, w, h) {
            if let Some(q) = pedido(c, t, w, h).and_then(|(ts, aw, ah)| f.cuadro(c, ts, aw, ah)) {
                return q.en_rgba().into_owned();
            }
        }
        if c.contenido == Contenido::Ajuste {
            if let Some(l) = lienzo.as_mut() {
                let tl = t - c.inicio;
                let tam = [p.ancho as f64, p.alto as f64];
                let mut b = ec::Buf { img: l.clone(), offset: [0.0; 2], scale: escala as f64 };
                for x in c.fx.iter().filter(|x| x.activo) {
                    b = aplicar_fx(x, tl, c.dur, p.fps, b, tam, None);
                }
                if let Some(g) = c.nodos.as_ref().filter(|g| g.activo) {
                    let medios = crate::nodos::medios(c, t, p, &b, f);
                    b = crate::nodos::evaluar(g, b, tl, &medios, &|f, b| aplicar_fx(f, tl, c.dur, p.fps, b, tam, None));
                }
                // Los efectos pueden agrandar el buffer: se recorta al lienzo.
                let (dx, dy) = (b.offset[0].round() as i64, b.offset[1].round() as i64);
                let mut o = Image::new(w as u32, h as u32);
                o.data.par_chunks_mut(w).enumerate().for_each(|(y, fila)| {
                    fila.iter_mut().enumerate().for_each(|(x, q)| *q = b.img.get(x as i64 + dx, y as i64 + dy));
                });
                // Con recorte, los efectos solo se ven dentro del recuadro.
                let r = c.en_su_momento(tl).mov.recorte;
                if r.iter().any(|v| *v > 0.0) {
                    let (x0, y0) = ((w as f32 * r[0] / 100.0) as usize, (h as f32 * r[1] / 100.0) as usize);
                    let (x1, y1) = ((w as f32 * (1.0 - r[2] / 100.0)) as usize, (h as f32 * (1.0 - r[3] / 100.0)) as usize);
                    o.data.par_chunks_mut(w).enumerate().for_each(|(y, fila)| {
                        fila.iter_mut().enumerate().filter(|(x, _)| *x < x0 || *x >= x1 || y < y0 || y >= y1).for_each(|(_, q)| *q = [0.0; 4]);
                    });
                }
                let op = (c.mov.opacidad.en(tl).num() / 100.0).clamp(0.0, 1.0) as f32;
                let modo = FUSIONES.get(c.mov.fusion as usize).map_or(BlendMode::Normal, |x| x.1);
                l.blend_from(&o, modo, op, 0);
            }
            continue;
        }
        let modo = FUSIONES.get(c.mov.fusion as usize).map_or(BlendMode::Normal, |x| x.1);
        // Textos y figuras quietos se dibujan una vez y se encima solo su recuadro.
        if let (Some(l), BlendMode::Normal, true) = (lienzo.as_mut(), modo, quieto(c, t)) {
            let q = guardado(c, t, p, w, h, f);
            encimar(l, &q);
            continue;
        }
        let vacio = || Image::new(w as u32, h as u32);
        let mut img = dibujar_clip(c, t, p, w, h, f).unwrap_or_else(vacio);
        if let Some(tr) = c.entrada.as_ref().filter(|x| t < c.inicio + x.dur) {
            // La saliente es el clip pegado a la izquierda, que sigue corriendo más allá de su corte.
            let a = previo(p, c).and_then(|q| dibujar_clip(q, t, p, w, h, f)).unwrap_or_else(vacio);
            img = transicion(tr, &a, &img, ((t - c.inicio) / tr.dur.max(0.01)) as f32, escala);
        }
        if let Some(tr) = c.salida.as_ref().filter(|x| t > c.fin() - x.dur) {
            img = transicion(tr, &img, &vacio(), ((t - (c.fin() - tr.dur)) / tr.dur.max(0.01)) as f32, escala);
        }
        match lienzo.as_mut() {
            Some(l) => l.blend_from(&img, modo, 1.0, 0),
            None if modo == BlendMode::Normal => {
                // Lo de hasta abajo va sobre negro: basta con hacerlo opaco.
                img.data.par_iter_mut().for_each(|q| q[3] = 1.0);
                lienzo = Some(img);
            }
            None => lienzo.get_or_insert_with(|| Image::filled(w as u32, h as u32, negro)).blend_from(&img, modo, 1.0, 0),
        }
    }
    match lienzo {
        Some(i) => i.to_rgba8_over([0.0; 3]),
        None => [0u8, 0, 0, 255].repeat(w * h),
    }
}
