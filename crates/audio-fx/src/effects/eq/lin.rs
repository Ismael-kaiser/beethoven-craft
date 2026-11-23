// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio
// Copied into Toma Uno on 2026-11-23 from commit 349c6ba (modified: the test module was removed).

//! LIN: a linear-phase filter whose magnitude is the bands'. A table of gains from the bands' coefficients becomes a
//! kernel of `N / 2` taps, convolved by FFT in blocks of `N / 2`; a new kernel fades in over the crossfade time while
//! the old one fades out. The transforms are in doubles, rounded to single wherever the page holds singles.
//!
// spec: plugins/fruity-parametric-eq-2.md ("LIN")

use std::f64::consts::{LN_2, TAU};

use super::design::BandCoefs;
use super::fft::Fft;

/// The crossfade time in seconds for the crossfade settings 0..3 (other values: the first).
pub fn fade_time(setting: i32) -> f32 {
    match setting {
        1 => 0.25,
        2 => 0.01,
        3 => 0.02,
        _ => 0.1,
    }
}

/// The transform length `N` for a size setting at the project's `rate`.
pub fn transform_len(size: i32, rate: f64) -> usize {
    let n0 = 512usize << size.clamp(0, 7);
    if rate < 44100.0 {
        return n0 / 2;
    }
    let up = (((rate * (1.0 / 44100.0)).ln() / LN_2 + 0.5) as f32).trunc() as u32;
    (n0 << up.min(8)).min(65536)
}

/// The delay LIN reports: `3 N / 4`.
pub fn delay(n: usize) -> usize {
    3 * n / 4
}

/// What the table needs of one band.
pub struct TableBand {
    /// The band's frequency in Hz (a single, as the design has it).
    pub freq: f32,
    /// Width value below 8192 and band pass, notch or peaking.
    pub narrow: bool,
    /// Its sections, when it is enabled and its type is not 0.
    pub sections: Option<BandCoefs>,
}

/// `p2(x)` for a single `x > 0`: a cubic for `2^x` that reads 0.999862 at whole powers of two.
fn p2(x: f32) -> f32 {
    let k = x.trunc() as i32;
    let fr = f64::from(x - k as f32);
    (2f64.powi(k) * (fr * (fr * (fr * 0.079064658211 + 0.224328091473) + 0.69659850257) + 0.999862219597)) as f32
}

/// A table entry from the bands' squared magnitude: `2^(dB / 6)`.
fn gain_of(mag: f32) -> f32 {
    let v = if mag == 0.0 { -1000.0 } else { ((f64::from(mag).log10() * 0.5555555555555556) as f32).clamp(-1000.0, 1000.0) };
    if v.is_nan() {
        return 1.0;
    }
    let t = v * 18.0;
    let x = t * (1.0f64 / 6.0) as f32;
    if t < -100.0 {
        0.0
    } else if t > 100.0 {
        2f64.powf(f64::from(x)) as f32
    } else if x == 0.0 {
        1.0
    } else if x > 0.0 {
        p2(x)
    } else {
        1.0 / p2(-x)
    }
}

/// The table of `N / 2 + 1` gains: entry `i` for the frequency `i * rate / N` (`rate` the project's), the bands'
/// coefficients evaluated at `band_rate`.
pub fn table(n: usize, rate: f64, band_rate: f64, bands: &[TableBand]) -> Vec<f32> {
    let step = (rate / n as f64) as f32;
    let angle = (1.0 / band_rate) * TAU;
    let mut out = vec![0f32; n / 2 + 1];
    let mut prev = 0f32;
    for (i, e) in out.iter_mut().enumerate() {
        let mut f = i as f32 * step;
        if f < 0.1 {
            f = 0.1;
        }
        if let Some(b) = bands.iter().find(|b| b.narrow && prev < b.freq && b.freq <= f) {
            f = b.freq;
        }
        prev = f;
        let w = (f64::from(f) * angle) as f32;
        let (s1, c1) = f64::from(w).sin_cos();
        let (s2, c2) = f64::from(w * 2.0).sin_cos();
        let mut mag = 1f32;
        for secs in bands.iter().filter_map(|b| b.sections.as_ref()) {
            let mut gb = 1f32;
            for c in secs.as_slice() {
                let re = (c.b0 * c2 + c.b1 * c1) + c.b2;
                let im = c.b1 * s1 + c.b0 * s2;
                let rd = (c2 + c.a1 * c1) + c.a2;
                let id = c.a1 * s1 + s2;
                gb = ((f64::from(gb) * (re * re + im * im)) / (rd * rd + id * id)) as f32;
            }
            mag *= gb;
        }
        *e = gain_of(mag);
    }
    out[n / 2] = out[n / 2 - 1];
    out
}

/// A kernel's transform, `N` bins, held as singles.
pub struct Kernel {
    re: Vec<f32>,
    im: Vec<f32>,
}

/// The zero-phase response of a table, `N` singles with its centre at `N / 2`.
fn response(t: &[f32], fft: &Fft, n: usize) -> Vec<f32> {
    let (mut re, mut im) = (vec![0f64; n], vec![0f64; n]);
    for k in 0..=n / 2 {
        let v = f64::from(if k % 2 == 1 { -t[k] } else { t[k] });
        re[k] = v;
        if k > 0 && k < n / 2 {
            re[n - k] = v;
        }
    }
    fft.run(&mut re, &mut im, true);
    let scale = 1.0f32 / n as f32;
    re.iter().map(|&v| (v as f32) * scale).collect()
}

/// The `N / 2` taps: the response's values around its centre (from `N / 4`) under a sine window.
fn taps(t: &[f32], fft: &Fft, n: usize) -> Vec<f32> {
    let z = response(t, fft, n);
    let h = n / 2;
    let step = 1.0f32 / (h - 1) as f32;
    (0..h).map(|i| z[n / 4 + i] * (f64::from((i as f32 * std::f32::consts::PI) * step).sin() as f32)).collect()
}

/// The kernel of a table: its taps transformed with `N / 2` zeros after them.
pub fn kernel(t: &[f32], fft: &Fft, n: usize) -> Kernel {
    let (mut re, mut im) = (vec![0f64; n], vec![0f64; n]);
    for (r, &v) in re.iter_mut().zip(&taps(t, fft, n)) {
        *r = f64::from(v);
    }
    fft.run(&mut re, &mut im, false);
    Kernel { re: re.iter().map(|&v| v as f32).collect(), im: im.iter().map(|&v| v as f32).collect() }
}

/// One convolver: a kernel (none = silence), per channel an input block of `H` and a result of `N`, and the position.
struct Convolver {
    kernel: Option<Kernel>,
    input: [Vec<f32>; 2],
    result: [Vec<f32>; 2],
    pos: usize,
}

impl Convolver {
    fn new(n: usize) -> Convolver {
        Convolver { kernel: None, input: [vec![0.0; n / 2], vec![0.0; n / 2]], result: [vec![0.0; n], vec![0.0; n]], pos: 0 }
    }

    fn empty(&mut self) {
        for c in 0..2 {
            self.input[c].fill(0.0);
            self.result[c].fill(0.0);
        }
        self.pos = 0;
    }

    fn frame(&mut self, x: [f32; 2], fft: &Fft, re: &mut [f64], im: &mut [f64]) -> [f32; 2] {
        let Some(k) = &self.kernel else { return [0.0; 2] };
        let out = [self.result[0][self.pos], self.result[1][self.pos]];
        self.input[0][self.pos] = x[0];
        self.input[1][self.pos] = x[1];
        self.pos += 1;
        let h = self.input[0].len();
        if self.pos == h {
            self.pos = 0;
            let n = 2 * h;
            let scale = 1.0f32 / n as f32;
            for c in 0..2 {
                for (r, &x) in re.iter_mut().zip(&self.input[c]) {
                    *r = f64::from(x);
                }
                re[h..].fill(0.0);
                im.fill(0.0);
                fft.run(re, im, false);
                for i in 0..n {
                    let (xr, xi) = (re[i] as f32, im[i] as f32);
                    let (kr, ki) = (k.re[i], k.im[i]);
                    re[i] = f64::from(xr * kr - xi * ki);
                    im[i] = f64::from(xr * ki + xi * kr);
                }
                fft.run(re, im, true);
                let res = &mut self.result[c];
                res.copy_within(h.., 0);
                res[h..].fill(0.0);
                for (r, &y) in res.iter_mut().zip(re.iter()) {
                    *r = scale * (y as f32) + *r;
                }
            }
        }
        out
    }
}

/// The two convolvers, the last `N` input frames and the crossfade between them.
pub struct Lin {
    n: usize,
    rate: f64,
    fft: Fft,
    conv: [Convolver; 2],
    history: Vec<[f32; 2]>,
    /// Where the next frame goes in `history` (the oldest kept one).
    hpos: usize,
    cnt: i64,
    len: i64,
    /// The convolver whose turn it is next, and the one that got the last table.
    next: usize,
    newest: usize,
    pending: Option<(Kernel, f32)>,
    re: Vec<f64>,
    im: Vec<f64>,
}

impl Lin {
    /// Nothing handed over yet: silence until the first table.
    pub fn new(n: usize, rate: f64) -> Lin {
        Lin {
            n,
            rate,
            fft: Fft::new(n),
            conv: [Convolver::new(n), Convolver::new(n)],
            history: vec![[0.0; 2]; n],
            hpos: 0,
            cnt: 0,
            len: 0,
            next: 0,
            newest: 0,
            pending: None,
            re: vec![0.0; n],
            im: vec![0.0; n],
        }
    }

    pub fn n(&self) -> usize {
        self.n
    }

    /// True while a crossfade is under way.
    pub fn fading(&self) -> bool {
        self.cnt < self.len
    }

    /// Hands over a new table (`N / 2 + 1` gains) with its crossfade time in seconds.
    pub fn hand_over(&mut self, table: &[f32], time: f32) {
        self.pending = Some((kernel(table, &self.fft, self.n), time));
    }

    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        for (l, r) in l.iter_mut().zip(r.iter_mut()) {
            [*l, *r] = self.frame([*l, *r]);
        }
    }

    fn frame(&mut self, x: [f32; 2]) -> [f32; 2] {
        let (fft, re, im) = (&self.fft, &mut self.re, &mut self.im);
        if self.cnt >= self.len
            && let Some((k, time)) = self.pending.take()
        {
            self.cnt = 0;
            self.len = ((self.rate as f32) * time).trunc() as i64;
            let c = self.next;
            self.next ^= 1;
            self.newest = c;
            let conv = &mut self.conv[c];
            conv.empty();
            conv.kernel = Some(k);
            for i in 0..self.n {
                conv.frame(self.history[(self.hpos + i) % self.n], fft, re, im);
            }
        }
        self.history[self.hpos] = x;
        self.hpos = (self.hpos + 1) % self.n;
        if self.cnt >= self.len {
            return self.conv[self.newest].frame(x, fft, re, im);
        }
        let a = self.conv[0].frame(x, fft, re, im);
        let b = self.conv[1].frame(x, fft, re, im);
        let mut f = self.cnt as f32 / self.len as f32;
        if self.newest == 1 {
            f = 1.0 - f;
        }
        self.cnt += 1;
        [b[0] * (1.0 - f) + a[0] * f, b[1] * (1.0 - f) + a[1] * f]
    }
}

