// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio
// Copied into Toma Uno on 2026-11-23 from commit 349c6ba (modified: the test module was removed).

//! HQ+'s coefficients: a section whose magnitude matches an analogue prototype's up to the Nyquist frequency. The
//! squared magnitude of a biquad is a ratio of two polynomials in `x = sin(HALFPI * f)^2`,
//! `D(x) = (1 - x) + A1 x + 4 A2 x (1 - x)` over `N(x)` of the same form; each type fits the four numbers to the
//! prototype and the last step turns them into coefficients. In doubles, evaluated as bracketed; `w` is the frequency
//! as a fraction of the Nyquist frequency, `q` the quality, `g` the gain in dB. The mathematical functions are the
//! standard ones (the page's figures for that are under "Checked against FL").
//!
// spec: plugins/fruity-parametric-eq-2.md ("HQ+": "The design")

use super::design::Coefs;
use super::{BAND_PASS, BAND_STOP, HIGH_PASS, HIGH_SHELF, LOW_PASS, LOW_SHELF, PEAKING};

const HALFPI: f64 = std::f64::consts::FRAC_PI_2;
const PI: f64 = std::f64::consts::PI;
/// `2 / pi`, `16 / pi^4`, `4 / pi^2`.
const TWO_OVER_PI: f64 = std::f64::consts::FRAC_2_PI;
const SIXTEEN_OVER_PI4: f64 = 0.16425571607494938;
const FOUR_OVER_PI2: f64 = 0.4052847345693511;

fn r0(v: f64) -> f64 {
    if v > 0.0 { v.sqrt() } else { 0.0 }
}

/// The section of type `kind` for the three arguments.
pub fn section(kind: i32, w: f64, q: f64, g: f64) -> Coefs {
    match kind {
        LOW_PASS => low_pass(w, q),
        HIGH_PASS => high_pass(w, q),
        BAND_PASS => band_pass(w, q),
        BAND_STOP => notch(w, q),
        PEAKING => peaking(w, q, g),
        LOW_SHELF => shelf(w, q, g, true),
        HIGH_SHELF => shelf(w, q, g, false),
        _ => Coefs::UNITY,
    }
}

/// The last step: the coefficients from `D`'s numbers `den1, den2` and `N`'s `num1, num2`.
fn last_step(den1: f64, den2: f64, num1: f64, num2: f64, scale: f64) -> Coefs {
    let v = (r0(den1) + 1.0) * 0.5;
    let w = (r0(num1) + 1.0) * 0.5;
    let r = 1.0 / (r0((v * v) + den2) + v);
    let t = r0((w * w) + num2) + w;
    let rb = r * scale;
    Coefs { b0: rb * t, b1: rb * (1.0 - r0(num1)), b2: -(rb * (num2 / t)), a1: r * (1.0 - r0(den1)), a2: -(r * (r * den2)) }
}

/// The poles of the band pass and the notch, by impulse invariance: `(a1, a2)`.
fn poles(w: f64, q: f64) -> (f64, f64) {
    let r = ((w * -HALFPI) / q).exp();
    let u = 1.0 / ((q * 4.0) * q);
    let c = if q > 0.5 { (((1.0 - u).sqrt() * PI) * w).cos() } else { (((u - 1.0).sqrt() * PI) * w).cosh() };
    ((r * c) * -2.0, r * r)
}

fn notch(w: f64, q: f64) -> Coefs {
    let (a1, a2) = poles(w, q);
    let z = (w * PI).cos() * -2.0;
    let n = ((a1 + 1.0) + a2) / ((1.0 + z) + 1.0);
    Coefs { b0: n, b1: n * z, b2: n, a1, a2 }
}

fn band_pass(w: f64, q: f64) -> Coefs {
    let (a1, a2) = poles(w, q);
    let s = (w * HALFPI).sin();
    let p1 = s * s;
    let p0 = 1.0 - p1;
    let d0 = ((a1 + 1.0) + a2) * ((a1 + 1.0) + a2);
    let d1 = ((1.0 - a1) + a2) * ((1.0 - a1) + a2);
    let d2 = a2 * -4.0;
    let r1 = ((p1 * d1) + (p0 * d0)) + (((p0 * 4.0) * p1) * d2);
    let r2 = (((p0 - p1) * 4.0) * d2) + (d1 - d0);
    let n2 = (r1 - (r2 * p1)) / ((p1 * 4.0) * p1);
    let n1 = (((p1 - p0) * 4.0) * n2) + r2;
    let b1 = n1.sqrt() * -0.5;
    let b0 = (((n1 * 0.25) + n2).sqrt() - b1) * 0.5;
    Coefs { b0, b1, b2: -b0 - b1, a1, a2 }
}

fn low_pass(w: f64, q: f64) -> Coefs {
    let mut iq = 1.0 / (q * q);
    if (1.99..2.0).contains(&iq) {
        iq = 1.99;
    }
    let w2 = w * w;
    let w4 = w2 * w2;
    let c = TWO_OVER_PI / w;
    let s0 = (c * c) * (iq - 2.0);
    let den = ((iq * w) * w) + ((w2 - 1.0) * (w2 - 1.0));
    let hn = w4 / den;
    let k = 1.0 - (iq * 0.5);
    let (den1, den2, num1, num2);
    if iq < 2.0 && (k * w) * w < 1.0 {
        // A resonance peak below the Nyquist frequency: its height, frequency and level tangent.
        let s = ((r0(k) * w) * HALFPI).sin();
        let p = s * s;
        let hr = 1.0 / (iq - ((iq * 0.25) * iq));
        den1 = (((1.0 - p) * (hr - 1.0)) * (1.0 - p)) / (((hr - hn) * s) * (p * s));
        num1 = (den1 / den) * w4;
        let ip = 1.0 / (p * p);
        den2 = ((((den1 - num1) - s0) * 0.25) / (hr - 1.0)) - (ip * 0.25);
        num2 = (((hr - 1.0) * 0.25) * ip) + (den2 * hr);
    } else {
        let fm = w.min(0.5);
        let s = (fm * HALFPI).sin();
        let p = s * s;
        let dm = (((fm * w) * (fm * w)) * iq) + ((w2 - (fm * fm)) * (w2 - (fm * fm)));
        let hm = w4 / dm;
        let kk = hn / (hn - 1.0);
        let km = hm / (hm - 1.0);
        let e = ((((km - kk) * s0) + (1.0 / p)) - 1.0) / (kk - (km * p));
        let x = (e + s0) * kk;
        den1 = (s0 - x) + e;
        num1 = -x;
        den2 = e * -0.25;
        num2 = -0.0;
    }
    last_step(den1, den2, num1, num2, 1.0)
}

fn high_pass(w: f64, q: f64) -> Coefs {
    let iq = 1.0 / (q * q);
    let w2 = w * w;
    let b = SIXTEEN_OVER_PI4 / (w2 * w2);
    let k = 1.0 - (iq * 0.5);
    let (den1, den2);
    if iq < 2.0 && w2 < k {
        let s = ((w / r0(k)) * HALFPI).sin();
        let p = s * s;
        let ih = 1.0 / (1.0 / (iq - ((iq * 0.25) * iq)));
        den2 = (-0.25 / (p * p)) - ((ih * b) * 0.25);
        den1 = ((((b + b) * p) * ih) + 1.0) - ((4.0 - (p * 8.0)) * den2);
    } else {
        let fm = w.min(0.5);
        let s = (fm * HALFPI).sin();
        let p = s * s;
        let e = w2 * iq;
        den1 = (((w2 - 1.0) * (w2 - 1.0)) + e) * b;
        let dm = ((e * fm) * fm) + ((w2 - (fm * fm)) * (w2 - (fm * fm)));
        let big_dm = (dm * (p * b)) / ((fm * fm) * (fm * fm));
        let mut d2 = (((big_dm - den1) * 0.25) / (1.0 - p)) - (0.25 / p);
        if iq < 2.0 {
            let m = (((w2 - 1.0) + (iq * 0.5)) * q).min(1.0);
            let z = -0.25 - (den1 * 0.25);
            d2 = ((d2 - z) * m) + z;
        }
        den2 = d2;
    }
    let v = (r0(den1) + 1.0) * 0.5;
    let r = 1.0 / (r0((v * v) + den2) + v);
    let h = r0(b) * 0.5;
    Coefs { b0: r * h, b1: -(r * r0(b)), b2: r * ((b * 0.25) / h), a1: r * (1.0 - r0(den1)), a2: -(r * (r * den2)) }
}

fn peaking(w: f64, q: f64, g: f64) -> Coefs {
    let gg = 10f64.powf(g * 0.05);
    if (gg - 1.0).abs() < 0.001 {
        return Coefs::UNITY;
    }
    let q2 = q * q;
    let iq = 1.0 / q2;
    let g2 = gg * gg;
    let w2 = w * w;
    let e = r0((q2 + 0.5) + r0(q2 + 0.25));
    let d1 = (w2 - 1.0) * (w2 - 1.0);
    if w < 1.0 {
        let hn = (((w2 * gg) * iq) + d1) / (((w2 * iq) / gg) + d1);
        let s0 = (w * HALFPI).sin();
        let p0 = s0 * s0;
        let s1 = (((w * q) / e) * HALFPI).sin();
        let p1 = s1 * s1;
        let ip = 1.0 / p0;
        let m = (1.0 - g2) * ip;
        let pp = ((((ip * p1) - 2.0) * m) + ((1.0 - gg) / p1)) / (gg - g2);
        let t = (((ip - 2.0) * m) + (1.0 - hn)) / (hn - g2);
        let ee = (t - pp) / (1.0 - p1);
        let fd = pp - (ee * p1);
        let en = ((1.0 - g2) / (p0 * p0)) + (ee * g2);
        let den1 = (fd + 1.0) + ee;
        let den2 = ee * -0.25;
        let num1 = ((((((g2 - 1.0) + g2) - 1.0) * ip) + (fd * g2)) + 1.0) + en;
        let num2 = en * -0.25;
        last_step(den1, den2, num1, num2, 1.0)
    } else {
        // The centre at or above the Nyquist frequency (only a steep band's shifted sections): first degree.
        let f1 = q / e;
        let s1 = (f1 * HALFPI).sin();
        let p1 = s1 * s1;
        let f2 = f1 * f1;
        let dm = (w2 - f2) * (w2 - f2);
        let hn = (((w2 * gg) * iq) + d1) / (((w2 * iq) * (1.0 / gg)) + d1);
        let h1 = ((((f2 * w2) * gg) * iq) + dm) / ((((f2 * w2) * iq) * (1.0 / gg)) + dm);
        let den1 = ((1.0 - p1) * (h1 - 1.0)) / ((hn - h1) * p1);
        last_step(den1, 0.0, hn * den1, 0.0, 1.0)
    }
}

fn shelf(w: f64, q: f64, g: f64, low: bool) -> Coefs {
    let gg = 10f64.powf(g * 0.05);
    if (gg - 1.0).abs() < 0.001 {
        return Coefs::UNITY;
    }
    let q2 = q * q;
    let iq = 1.0 / q2;
    let g2 = r0((1.0 / gg).max(gg));
    let g4 = r0(g2);
    let glo = gg.min(1.0);
    let ghi = gg.max(1.0);
    let (wc, n2, d2, gp);
    if low {
        wc = g4 * w;
        let v = (wc * wc) / (g2 * g2);
        n2 = ghi * v;
        d2 = v / glo;
        gp = 1.0 / (gg * gg);
    } else {
        wc = w / g4;
        let v = wc * wc;
        n2 = v / glo;
        d2 = ghi * v;
        gp = gg * gg;
    }
    // The prototype's squared magnitude at the frequency `sqrt(u)`.
    let h = |n: f64, d: f64, u: f64| (((((n - u) * (n - u)) * q2) + (n * u)) * gp) / ((((d - u) * (d - u)) * q2) + (d * u));
    let hn = h(n2, d2, 1.0);
    let scale = if low { gg } else { 1.0 };

    if q2 <= 0.6 {
        // Low Q: the poles by impulse invariance, the numerator from the values at the Nyquist frequency and at `fm`.
        let fm = if low { (wc / r0(g2)).min(0.2) } else { (wc * r0(g2)).min(0.2) };
        let s = (fm * HALFPI).sin();
        let p = s * s;
        let hm = h(n2, d2, fm * fm);
        let th = r0(d2) * HALFPI;
        let r = ((-1.0 / q) * th).exp();
        let c = if q <= 0.5 { (th * r0(iq - 4.0)).cosh() } else { (th * r0(4.0 - iq)).cos() };
        let al = (r * c) * -2.0;
        let r2 = r * r;
        let pn = ((1.0 - al) + r2) * ((1.0 - al) + r2);
        let pz = ((al + 1.0) + r2) * ((al + 1.0) + r2);
        let u = 1.0 / pz;
        let den1 = pn * u;
        let den2 = (r2 * -4.0) * u;
        let num1 = (hn * pn) * u;
        let num2 = ((((((pn * hm) - (hn * pn)) / (1.0 - p)) + ((hm - 1.0) / (u * p))) * 0.25) + ((r2 * -4.0) * hm)) * u;
        return last_step(den1, den2, num1, num2, scale);
    }

    let k = iq - 2.0;
    let c = -(((g2 * g2) + 1.0) / (k * g2));
    let rho = c - r0((c * c) - 1.0);
    let rg = if low { (1.0 / g2) * rho } else { g2 * rho };
    let fa = wc * r0(rg);

    if fa >= 1.0 {
        // The prototype's slope at 0 Hz, its values at the Nyquist frequency and at `fm`: first-degree numerator.
        let s0 = ((FOUR_OVER_PI2 / d2) - (FOUR_OVER_PI2 / n2)) * k;
        let fm = if low { ((1.0 / g2) * wc).min(0.5) } else { wc.min(0.5) };
        let s = (fm * HALFPI).sin();
        let p = s * s;
        let hm = h(n2, d2, fm * fm);
        let kk = hn / (hn - 1.0);
        let km = hm / (hm - 1.0);
        let e = ((((km - kk) * s0) + (1.0 / p)) - 1.0) / (kk - (km * p));
        let x = (s0 + e) * kk;
        let den1 = ((s0 + (-x - 1.0)) + 1.0) + e;
        let num1 = (-x - 1.0) + 1.0;
        return last_step(den1, e * -0.25, num1, -0.0, scale);
    }

    let sa = (fa * HALFPI).sin();
    let pa = sa * sa;
    let ha = h(n2, d2, fa * fa);
    let da = ha - 1.0;
    let (fd, fnum, ed, en);
    if fa < rho {
        // Both turning points below the Nyquist frequency: their values, with a level tangent at both.
        let fb = fa / rho;
        let sb = (fb * HALFPI).sin();
        let pb = sb * sb;
        let hb = h(n2, d2, fb * fb);
        fd = (((pa * (hb - 1.0)) - (da * pb)) * -2.0) / ((pa * (hb - ha)) * pb);
        fnum = ((da + da) / pa) + (ha * fd);
        ed = (((pa * pa) * (hb - 1.0)) - ((pb * pb) * da)) / (((pb * pa) * (pb * pa)) * (hb - ha));
        en = (ha * ed) - (da / (pa * pa));
    } else {
        // Only the first turning point below it: its value with a level tangent, the value at the Nyquist frequency,
        // and `S` from an interpolation of its own.
        let (nr, dr, sg) = if low { (ghi * rg, rg / glo, gg - 1.0) } else { ((rho / g2) / glo, (rho / g2) * ghi, 1.0 - gg) };
        let h_a = h(nr, dr, rho * rho);
        let h_r = h(nr, dr, 1.0);
        let sr = (rho * HALFPI).sin();
        let ipr = 1.0 / (sr * sr);
        let t = (((2.0 - (ipr + ipr)) * (h_r - 1.0)) * (h_a - 1.0)) / (h_r - h_a);
        let kap = k * FOUR_OVER_PI2;
        let l = (((rho * r0(gg)) * t) / (sg * kap)).ln();
        let y = fa.powf(l / rho.ln());
        let big_s = y * (((n2 - d2) * kap) / (d2 * n2));
        fd = (-2.0 / pa) - (big_s / da);
        fnum = fd - big_s;
        let qq = da / (pa * pa);
        ed = ((((hn + qq) - 1.0) - fnum) + (hn * fd)) / (ha - hn);
        en = (ha * ed) - qq;
    }
    last_step((fd + 1.0) + ed, ed * -0.25, (fnum + 1.0) + en, en * -0.25, scale)
}

