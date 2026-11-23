// Copyright (C) 2026 mixter-ha99
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Beethoven DAW.

//! The tab strip and the pages that are tabs of their own: Score, Ideas and Export.

use os_ui::{Color, Font, Rect, tabler};

use super::FrameInfo;
use crate::app::{Action, State, Ui};
use crate::ideas::{self, Make};
use crate::page::{Hit, Scope};
use crate::score::{self, Clef, Elem};
use crate::tabs::{self, Tab};

fn icon_of(t: Tab) -> &'static tabler::TablerIcon {
    match t {
        Tab::Arrange => &tabler::TIMELINE,
        Tab::Rhythm => &tabler::GRID_DOTS,
        Tab::Piano => &tabler::PIANO,
        Tab::Mixer => &tabler::ADJUSTMENTS,
        Tab::Score => &tabler::MUSIC,
        Tab::Ideas => &tabler::STAR,
        Tab::Export => &tabler::FILE_EXPORT,
    }
}

/// The strip, at the workspace's top.
pub fn strip(ui: &mut Ui, s: &State, r: Rect) {
    let t = ui.t;
    ui.c.fill_rect(r, t.overview.mix(t.workspace, 0.92));
    ui.c.fill_rect(Rect::new(r.x, r.bottom() - 1.0, r.w, 1.0), t.window_border);
    // The library's switch, where a browser has its side panel's.
    let lib = Rect::new(r.x + 8.0, r.y + 6.0, 30.0, 26.0);
    let a = Action::Page(Hit::Library);
    let hov = ui.hit(lib, a.clone(), "Library: show or hide it");
    if s.show_browser || hov {
        ui.c.fill_rrect(lib, 8.0, if s.show_browser { t.accent.with_alpha(45) } else { t.button });
    }
    ui.c.icon(&tabler::FOLDERS, Rect::new(lib.x + 7.0, lib.y + 5.0, 16.0, 16.0), if s.show_browser { t.accent } else { t.glyph });
    let n = s.tabs.open.len().max(1) as f32;
    let w = ((r.w - 110.0) / n).clamp(74.0, 168.0);
    let mut x = r.x + 46.0;
    for &tab in &s.tabs.open {
        let on = s.tabs.cur == tab;
        let b = Rect::new(x, r.y + 5.0, w - 4.0, r.h - if on { 5.0 } else { 9.0 });
        let hov = ui.hit(b, Action::Tab(tab), hint_of(tab));
        if on {
            ui.c.fill_rrect(Rect::new(b.x, b.y, b.w, b.h + 8.0), 10.0, t.window_title);
            ui.c.fill_rrect(Rect::new(b.x + 12.0, b.y + 1.0, b.w - 24.0, 2.5), 1.0, t.accent);
        } else {
            ui.c.fill_rrect(b, 9.0, if hov { t.button } else { t.toolbar.mix(t.workspace, 0.4) });
        }
        let ink = if on { t.text } else { t.text_dim };
        ui.c.icon(icon_of(tab), Rect::new(b.x + 11.0, b.cy() - 8.0, 16.0, 16.0), if on { t.accent } else { ink });
        ui.c.push_clip(Rect::new(b.x, b.y, b.w - if on { 24.0 } else { 6.0 }, b.h));
        ui.c.text_mid(Font::new(16), b.x + 34.0, b.cy(), ink, tab.name());
        ui.c.pop_clip();
        if s.tabs.open.len() > 1 && (on || hov) {
            let c = Rect::new(b.right() - 24.0, b.cy() - 8.0, 16.0, 16.0);
            let hc = ui.hit(c.outset(2.0), Action::TabClose(tab), "Close tab");
            if hc {
                ui.c.fill_circle(c.cx(), c.cy(), 9.0, t.button_hi);
            }
            ui.c.icon(&tabler::X, c.inset(2.5), if hc { t.text } else { t.text_dim });
        }
        x += w;
    }
    if s.tabs.open.len() < Tab::ALL.len() {
        let b = Rect::new(x + 4.0, r.y + 7.0, 28.0, 24.0);
        let hov = ui.hit(b, Action::TabNew, "Open the next closed tab");
        ui.c.fill_rrect(b, 8.0, if hov { t.button_hi } else { t.button });
        ui.c.icon(&tabler::PLUS, Rect::new(b.cx() - 7.0, b.cy() - 7.0, 14.0, 14.0), t.glyph);
    }
}

fn hint_of(t: Tab) -> &'static str {
    match t {
        Tab::Arrange => "Arrange: the song's parts in time (F5)",
        Tab::Rhythm => "Rhythm: instruments and their steps (F6)",
        Tab::Piano => "Notes: the piano roll of the selected instrument (F7)",
        Tab::Mixer => "Mix: levels, effects and routing (F9)",
        Tab::Score => "Score: what you composed as sheet music",
        Tab::Ideas => "Ideas: chords, progressions, bass lines and melodies",
        Tab::Export => "Export: audio, MIDI and sheet music",
    }
}

fn button(ui: &mut Ui, r: Rect, label: &str, hit: Hit, hint: &'static str, on: bool) {
    let t = ui.t;
    let a = Action::Page(hit);
    let hov = ui.hit(r, a.clone(), hint);
    let down = ui.down(r, &a);
    let face = if on { t.accent } else if down { t.button_hi.shade(0.1) } else if hov { t.button_hi } else { t.button };
    ui.c.fill_rrect(r, 8.0, face);
    let ink = if on { t.on_glyph } else { t.text };
    let f = ui.c.fit_font(Font::new(16), label, r.w - 14.0);
    ui.c.text_center(f, r, ink, label);
}

pub fn paint(ui: &mut Ui, s: &State, f: &FrameInfo) {
    let r = s.work_rect();
    ui.hit(r, Action::Swallow, "");
    ui.c.push_clip(r);
    match s.tabs.cur {
        Tab::Ideas => ideas_page(ui, s, r),
        Tab::Export => export_page(ui, s, r),
        _ => score_page(ui, s, r, f),
    }
    ui.c.pop_clip();
}

fn head(ui: &mut Ui, r: Rect, title: &str, sub: &str) {
    let t = ui.t;
    ui.c.fill_rect(Rect::new(r.x, r.y, r.w, 58.0), t.window_title);
    ui.c.text(Font::new(24), r.x + 22.0, r.y + 7.0, t.text, title);
    ui.c.text(Font::new(15), r.x + 22.0, r.y + 34.0, t.text_dim, sub);
}

// ------------------------------------------------------------------------------------------------ Score

const PAPER: Color = Color::rgb(0xf6efe0);
const INK: Color = Color::rgb(0x241a1c);

fn score_page(ui: &mut Ui, s: &State, r: Rect, f: &FrameInfo) {
    let t = ui.t;
    ui.c.fill_rect(r, t.workspace);
    let sc = crate::page::score(s);
    let k = s.page.zoom;
    // Paper
    let w = (r.w - 60.0).min(1180.0 * k.max(0.8)).max(300.0);
    let paper = Rect::new(r.x + ((r.w - w) / 2.0).round(), r.y + 74.0 - s.page.scroll, w, 4000.0);
    let body = Rect::new(r.x, r.y + 58.0, r.w, r.h - 58.0);
    ui.c.push_clip(body);
    ui.c.fill_rect(Rect::new(paper.x, body.y, paper.w, body.h), PAPER);
    let d = Draw { x0: paper.x + 46.0 * k, x1: paper.right() - 40.0 * k, sp: 9.0 * k, k };
    let mut y = paper.y + 26.0 * k;
    let tf = Font::new((30.0 * k) as u16);
    ui.c.text_center(tf, Rect::new(paper.x, y, paper.w, 38.0 * k), INK, &sc.title);
    y += 46.0 * k;
    ui.c.text(Font::new((15.0 * k) as u16), d.x0, y, INK, &format!("♩ = {:.0}", sc.tempo));
    let kn = format!("{} {}", ideas::NAMES[usize::from(tonic_of(&sc))], if sc.minor { "minor" } else { "major" });
    ui.c.text_right(Font::new((15.0 * k) as u16), d.x1, y, INK.with_alpha(180), &kn);
    y += 40.0 * k;
    if sc.parts.is_empty() {
        ui.c.text_center(Font::new(20), Rect::new(paper.x, body.y + 170.0, paper.w, 30.0), INK.with_alpha(150), "Write some notes and they will appear here as sheet music");
    }
    let bottom = if sc.parts.is_empty() { y } else { d.systems(ui, &sc, y, body, playhead(s, &sc, f)) };
    s.page.height.set(bottom - paper.y + 120.0);
    ui.c.pop_clip();
    head(ui, r, "Score", "What you compose, written down as you go");
    let mut x = r.right() - 20.0;
    let mut right = |ui: &mut Ui, w: f32, label: &str, h: Hit, hint: &'static str, on: bool| {
        x -= w + 8.0;
        button(ui, Rect::new(x, r.y + 14.0, w, 30.0), label, h, hint, on);
    };
    right(ui, 170.0, "Export MusicXML", Hit::Xml, "A file that MuseScore, Sibelius, Finale or Dorico open", false);
    right(ui, 34.0, "+", Hit::Zoom(1), "Larger", false);
    right(ui, 34.0, "-", Hit::Zoom(-1), "Smaller", false);
    right(ui, 84.0, "Song", Hit::Scope(Scope::Song), "The whole song, as arranged", s.page.scope == Scope::Song);
    right(ui, 92.0, "Pattern", Hit::Scope(Scope::Pattern), "Every instrument of the pattern", s.page.scope == Scope::Pattern);
    right(ui, 110.0, "Instrument", Hit::Scope(Scope::Channel), "Only the selected instrument", s.page.scope == Scope::Channel);
}

fn tonic_of(sc: &score::Score) -> u8 {
    let major = (sc.fifths * 7).rem_euclid(12) as u8;
    if sc.minor { (major + 9) % 12 } else { major }
}

/// Where playback is, in units, when what plays is what is shown.
fn playhead(s: &State, sc: &score::Score, f: &FrameInfo) -> Option<f32> {
    let same = f.song == (s.page.scope == Scope::Song);
    let unit = (u32::from(s.session.project.ppq.max(4)) / score::Q).max(1);
    let at = f.tick as f32 / unit as f32;
    (f.playing && same && at < (sc.measures() as u32 * sc.measure()) as f32).then_some(at)
}

struct Draw {
    x0: f32,
    x1: f32,
    /// Between staff lines.
    sp: f32,
    k: f32,
}

impl Draw {
    /// All the systems; returns where they end.
    fn systems(&self, ui: &mut Ui, sc: &score::Score, mut y: f32, view: Rect, play: Option<f32>) -> f32 {
        let sp = self.sp;
        let many = sc.parts.len() > 1;
        let label = if many { 86.0 * self.k } else { 0.0 };
        let n = sc.measures();
        // Width of each measure: by its busiest staff.
        let need: Vec<f32> = (0..n)
            .map(|m| {
                let c = sc.parts.iter().flat_map(|p| &p.staves).map(|s| s.measures.get(m).map_or(1, Vec::len)).max().unwrap_or(1);
                (c as f32 * 3.4 + 2.6).max(9.0) * sp
            })
            .collect();
        let lead = (6.6 + sc.fifths.unsigned_abs() as f32 * 1.1) * sp;
        let mut m = 0;
        while m < n {
            let first = m == 0;
            let start = self.x0 + label + lead + if first { 3.0 * sp } else { 0.0 };
            let room = self.x1 - start;
            let mut used = 0.0;
            let mut end = m;
            while end < n && (end == m || used + need[end] <= room) {
                used += need[end];
                end += 1;
            }
            // Stretched to the margin, but a short last line stays short.
            let grow = if end == n && used < room * 0.72 { 1.0 } else { room / used };
            let staves: usize = sc.parts.iter().map(|p| p.staves.len()).sum();
            let pitch = 10.5 * sp;
            let height = staves as f32 * pitch;
            if y + height > view.y && y - 4.0 * sp < view.bottom() {
                self.system(ui, sc, m, end, y, label, start, grow, &need, first, play);
            }
            y += height + 3.0 * sp;
            m = end;
        }
        y
    }

    #[allow(clippy::too_many_arguments)]
    fn system(&self, ui: &mut Ui, sc: &score::Score, from: usize, to: usize, y: f32, label: f32, start: f32, grow: f32, need: &[f32], first: bool, play: Option<f32>) {
        let sp = self.sp;
        let x0 = self.x0 + label;
        let lw = (sp * 0.12).max(1.0);
        let end: f32 = start + need[from..to].iter().sum::<f32>() * grow;
        let mut sy = y;
        let top = y;
        for p in &sc.parts {
            let ptop = sy;
            for st in &p.staves {
                for l in 0..5 {
                    ui.c.fill_rect(Rect::new(x0, (sy + l as f32 * sp).round(), end - x0, lw), INK.with_alpha(170));
                }
                let mut x = x0 + sp * 0.8;
                self.clef(ui, st.clef, x, sy);
                x += sp * 4.0;
                for i in 0..sc.fifths.unsigned_abs() {
                    let sharp = sc.fifths > 0;
                    let pos = key_pos(i as usize, sharp, st.clef);
                    self.accidental(ui, if sharp { 1 } else { -1 }, x, sy + (8 - pos) as f32 * sp / 2.0);
                    x += sp * 1.1;
                }
                if first {
                    let tf = Font::new((sp * 2.5) as u16);
                    let tx = start - 3.0 * sp;
                    for (i, v) in [sc.beats, sc.beat_type].iter().enumerate() {
                        ui.c.text_center(tf, Rect::new(tx, sy + i as f32 * 2.0 * sp, 2.2 * sp, 2.0 * sp), INK, &v.to_string());
                    }
                }
                let mut mx = start;
                for m in from..to {
                    let mw = need[m] * grow;
                    if let Some(es) = st.measures.get(m) {
                        self.measure(ui, es, sc, st.clef, mx + sp * 1.0, mw - sp * 1.6, sy);
                    }
                    mx += mw;
                }
                sy += 10.5 * sp;
            }
            let pbot = sy - 10.5 * sp + 4.0 * sp;
            if !p.name.is_empty() && label > 0.0 {
                let f = ui.c.fit_font(Font::new((14.0 * self.k) as u16), &p.name, label - 12.0);
                let lh = ui.c.line_height(f);
                ui.c.text(f, self.x0, (ptop + pbot) / 2.0 - lh / 2.0, INK, &p.name);
            }
            // Bar lines through the part's staves.
            let mut mx = start;
            for m in from..to {
                mx += need[m] * grow;
                let last = m + 1 == sc.measures();
                ui.c.fill_rect(Rect::new(mx - lw, ptop, if last { sp * 0.4 } else { lw }, pbot - ptop), INK.with_alpha(210));
            }
            if p.staves.len() > 1 {
                ui.c.fill_rect(Rect::new(x0 - sp * 0.9, ptop, sp * 0.35, pbot - ptop), INK);
            }
        }
        let bot = sy - 10.5 * sp + 4.0 * sp;
        ui.c.fill_rect(Rect::new(x0, top, lw, bot - top), INK.with_alpha(210));
        ui.c.text(Font::new((11.0 * self.k) as u16), x0, top - 2.4 * sp, INK.with_alpha(130), &(from + 1).to_string());
        if let Some(at) = play {
            let m = (at / sc.measure() as f32) as usize;
            if (from..to).contains(&m) {
                let x = start + need[from..m].iter().sum::<f32>() * grow + sp + (need[m] * grow - sp * 1.6) * (at / sc.measure() as f32).fract();
                ui.c.fill_rect(Rect::new(x, top - sp, 2.0, bot - top + 2.0 * sp), Color::rgb(0xb5323d));
            }
        }
    }

    fn measure(&self, ui: &mut Ui, es: &[Elem], sc: &score::Score, clef: Clef, x: f32, w: f32, sy: f32) {
        let total = sc.measure() as f32;
        // Accidentals last until the bar line.
        let mut shown: std::collections::HashMap<(u8, i32), i32> = std::collections::HashMap::new();
        for e in es {
            let ex = x + w * e.at as f32 / total;
            if e.keys.is_empty() {
                let whole = e.dur == sc.measure();
                self.rest(ui, if whole { 16 } else { e.dur }, if whole { x + w / 2.0 - self.sp } else { ex }, sy);
                continue;
            }
            let spelled: Vec<(u8, i32, i32)> = e.keys.iter().map(|k| score::spell(*k, sc.fifths)).collect();
            let pos: Vec<i32> = spelled.iter().map(|p| staff_pos(p.0, p.2, clef)).collect();
            for (p, &l) in spelled.iter().zip(&pos) {
                let now = shown.get(&(p.0, p.2)).copied().unwrap_or(score::signature_alt(p.0, sc.fifths));
                if now != p.1 && !e.tie_in {
                    self.accidental(ui, p.1, ex - self.sp * 1.5, sy + (8 - l) as f32 * self.sp / 2.0);
                    shown.insert((p.0, p.2), p.1);
                }
            }
            self.notes(ui, e, &pos, ex, sy, w * e.dur as f32 / total);
        }
    }

    fn notes(&self, ui: &mut Ui, e: &Elem, pos: &[i32], x: f32, sy: f32, room: f32) {
        let sp = self.sp;
        let yof = |l: i32| sy + (8 - l) as f32 * sp / 2.0;
        let (lo, hi) = (pos.iter().copied().min().unwrap_or(4), pos.iter().copied().max().unwrap_or(4));
        let up = (lo + hi) / 2 < 4;
        let (_, dots) = score::value(e.dur);
        let open = e.dur >= 8;
        let rx = sp * 0.66;
        let cx = x + rx;
        // Ledger lines
        let mut l = -2;
        while l >= lo {
            ui.c.fill_rect(Rect::new(cx - rx * 1.7, yof(l), rx * 3.4, (sp * 0.12).max(1.0)), INK.with_alpha(190));
            l -= 2;
        }
        let mut l = 10;
        while l <= hi {
            ui.c.fill_rect(Rect::new(cx - rx * 1.7, yof(l), rx * 3.4, (sp * 0.12).max(1.0)), INK.with_alpha(190));
            l += 2;
        }
        for (i, &l) in pos.iter().enumerate() {
            // A second beside another head sits on the other side of the stem.
            let off = if i > 0 && l - pos[i - 1] == 1 { rx * 2.0 } else { 0.0 };
            let (hx, hy) = (cx + off, yof(l));
            let ring: Vec<(f32, f32)> = (0..20)
                .map(|j| {
                    let a = std::f32::consts::TAU * j as f32 / 20.0;
                    let (u, v) = (a.cos() * rx * 1.05, a.sin() * sp * 0.43);
                    // Tilted, as written heads are.
                    (hx + u * 0.94 + v * 0.34, hy - u * 0.34 + v * 0.94)
                })
                .collect();
            ui.c.fill_polygon(&ring, INK);
            if open {
                let hole: Vec<(f32, f32)> = ring.iter().map(|p| (hx + (p.0 - hx) * 0.72, hy + (p.1 - hy) * if e.dur >= 16 { 0.6 } else { 0.5 })).collect();
                ui.c.fill_polygon(&hole, PAPER);
            }
            for d in 0..dots {
                ui.c.fill_circle(hx + rx * 2.0 + d as f32 * sp * 0.6, hy - if l % 2 == 0 { sp * 0.25 } else { 0.0 }, sp * 0.17, INK);
            }
            if e.tie_out {
                // The tie curves away from the stem, to the next note.
                let dir = if up { 1.0 } else { -1.0 };
                let (a, b) = (hx + rx * 0.9, hx + (room - rx).max(sp * 2.0));
                let pts: Vec<(f32, f32)> = (0..=14)
                    .map(|j| {
                        let u = j as f32 / 14.0;
                        (a + (b - a) * u, hy + dir * (sp * 0.55 + sp * 0.6 * (std::f32::consts::PI * u).sin()))
                    })
                    .collect();
                ui.c.polyline(&pts, (sp * 0.15).max(1.0), INK);
            }
        }
        if e.dur >= 16 {
            return;
        }
        // Stem
        let sx = if up { cx + rx * 0.95 } else { cx - rx * 0.95 };
        let (y0, y1) = if up { (yof(lo), yof(hi) - sp * 3.4) } else { (yof(hi), yof(lo) + sp * 3.4) };
        ui.c.line(sx, y0, sx, y1, (sp * 0.14).max(1.0), INK);
        let flags = match e.dur {
            1 => 2,
            2 | 3 => 1,
            _ => 0,
        };
        for i in 0..flags {
            let dir = if up { 1.0 } else { -1.0 };
            let fy = y1 + dir * i as f32 * sp * 0.85;
            let pts: Vec<(f32, f32)> = (0..=10)
                .map(|j| {
                    let u = j as f32 / 10.0;
                    (sx + sp * 1.15 * (u * 1.35).sin().max(0.0) * (1.0 - u * 0.25), fy + dir * sp * 2.1 * u)
                })
                .collect();
            ui.c.polyline(&pts, (sp * 0.24).max(1.2), INK);
        }
    }

    fn rest(&self, ui: &mut Ui, dur: u32, x: f32, sy: f32) {
        let sp = self.sp;
        let (_, dots) = score::value(dur);
        match dur {
            12.. => ui.c.fill_rect(Rect::new(x, sy + if dur >= 16 { sp } else { sp * 1.5 }, sp * 1.3, sp * 0.5), INK),
            8 => ui.c.fill_rect(Rect::new(x, sy + sp * 1.5, sp * 1.3, sp * 0.5), INK),
            4..=7 => {
                let p = [(0.2, 0.5), (0.95, 1.35), (0.45, 1.95), (1.05, 2.75), (0.35, 2.6), (0.75, 3.35)];
                let pts: Vec<(f32, f32)> = p.iter().map(|q| (x + q.0 * sp, sy + q.1 * sp)).collect();
                ui.c.polyline(&pts, sp * 0.26, INK);
            }
            _ => {
                let n = if dur == 1 { 2 } else { 1 };
                ui.c.line(x + sp * 1.0, sy + sp * 1.2, x + sp * 0.45, sy + sp * (2.2 + n as f32 * 0.8), sp * 0.15, INK);
                for i in 0..n {
                    let yy = sy + sp * (1.45 + i as f32 * 0.9);
                    ui.c.fill_circle(x + sp * (0.35 - i as f32 * 0.1), yy, sp * 0.27, INK);
                    ui.c.line(x + sp * (0.35 - i as f32 * 0.1), yy + sp * 0.2, x + sp * (0.98 - i as f32 * 0.22), yy - sp * 0.15, sp * 0.13, INK);
                }
            }
        }
        for d in 0..dots {
            ui.c.fill_circle(x + sp * (1.8 + d as f32 * 0.5), sy + sp * 1.75, sp * 0.17, INK);
        }
    }

    fn accidental(&self, ui: &mut Ui, alt: i32, x: f32, y: f32) {
        let sp = self.sp;
        let w = (sp * 0.14).max(1.0);
        match alt {
            1 => {
                for dx in [0.3, 0.7] {
                    ui.c.line(x + sp * dx, y - sp * 1.3, x + sp * dx, y + sp * 1.3, w, INK);
                }
                for dy in [-0.4, 0.45] {
                    ui.c.line(x, y + sp * (dy + 0.14), x + sp, y + sp * (dy - 0.14), w * 2.2, INK);
                }
            }
            -1 => {
                ui.c.line(x + sp * 0.2, y - sp * 1.9, x + sp * 0.2, y + sp * 0.6, w, INK);
                let pts: Vec<(f32, f32)> = (0..=10)
                    .map(|j| {
                        let u = j as f32 / 10.0;
                        (x + sp * (0.2 + 0.62 * (std::f32::consts::PI * u).sin()), y - sp * 0.45 + sp * 1.05 * u)
                    })
                    .collect();
                ui.c.polyline(&pts, w * 1.7, INK);
            }
            _ => {
                ui.c.line(x + sp * 0.25, y - sp * 1.5, x + sp * 0.25, y + sp * 0.6, w, INK);
                ui.c.line(x + sp * 0.75, y - sp * 0.6, x + sp * 0.75, y + sp * 1.5, w, INK);
                for dy in [-0.45, 0.5] {
                    ui.c.line(x + sp * 0.25, y + sp * (dy + 0.1), x + sp * 0.75, y + sp * (dy - 0.1), w * 2.0, INK);
                }
            }
        }
    }

    fn clef(&self, ui: &mut Ui, c: Clef, x: f32, sy: f32) {
        let sp = self.sp;
        let w = (sp * 0.26).max(1.2);
        let at = |p: &[(f32, f32)]| -> Vec<(f32, f32)> { smooth(p).into_iter().map(|q| (x + q.0 * sp, sy + q.1 * sp)).collect() };
        match c {
            Clef::Treble => {
                // One stroke: the curl on the G line, the loop above, the tail below.
                let p = [(1.45, 3.35), (0.95, 3.05), (1.05, 2.4), (1.75, 2.15), (2.35, 2.75), (2.15, 3.6), (1.35, 3.95), (0.35, 3.5), (0.2, 2.5), (0.85, 1.35), (1.75, 0.4), (1.95, -0.7), (1.5, -1.35), (1.05, -0.5), (1.3, 1.5), (1.75, 4.6), (1.5, 5.5), (0.85, 5.4), (0.75, 4.9)];
                ui.c.polyline(&at(&p), w, INK);
                ui.c.fill_circle(x + 0.95 * sp, sy + 5.0 * sp, sp * 0.3, INK);
            }
            Clef::Bass => {
                let p = [(0.35, 1.15), (0.5, 0.35), (1.35, 0.05), (2.1, 0.75), (1.95, 2.0), (1.1, 3.0), (0.1, 3.6)];
                ui.c.polyline(&at(&p), w * 1.15, INK);
                ui.c.fill_circle(x + 0.5 * sp, sy + 1.1 * sp, sp * 0.38, INK);
                for dy in [0.5, 1.5] {
                    ui.c.fill_circle(x + 2.75 * sp, sy + dy * sp, sp * 0.22, INK);
                }
            }
        }
    }
}

/// A curve through the points (Catmull-Rom).
fn smooth(p: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut v = vec![];
    let g = |i: isize| p[i.clamp(0, p.len() as isize - 1) as usize];
    for i in 0..p.len() as isize - 1 {
        let (a, b, c, d) = (g(i - 1), g(i), g(i + 1), g(i + 2));
        for j in 0..8 {
            let t = j as f32 / 8.0;
            let f = |a: f32, b: f32, c: f32, d: f32| 0.5 * (2.0 * b + (c - a) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t * t + (3.0 * b - a - 3.0 * c + d) * t * t * t);
            v.push((f(a.0, b.0, c.0, d.0), f(a.1, b.1, c.1, d.1)));
        }
    }
    v.push(p[p.len() - 1]);
    v
}

/// Staff place of a note: 0 = bottom line, 1 = the space above, ...
pub fn staff_pos(step: u8, octave: i32, clef: Clef) -> i32 {
    let d = octave * 7 + i32::from(step);
    // Bottom lines: E4 in treble, G2 in bass.
    d - match clef {
        Clef::Treble => 4 * 7 + 2,
        Clef::Bass => 2 * 7 + 4,
    }
}

/// Place of the `i`th sharp or flat of a key signature.
fn key_pos(i: usize, sharp: bool, clef: Clef) -> i32 {
    let v: [i32; 7] = if sharp { [8, 5, 9, 6, 3, 7, 4] } else { [4, 7, 3, 6, 2, 5, 1] };
    v[i % 7] - if clef == Clef::Bass { 2 } else { 0 }
}

// ------------------------------------------------------------------------------------------------ Ideas

fn ideas_page(ui: &mut Ui, s: &State, r: Rect) {
    let t = ui.t;
    ui.c.fill_rect(r, t.workspace);
    let key = s.page.key;
    let notes = crate::edit::notes::notes(&s.session);
    let bar = s.session.project.bar_ticks();
    let ch = crate::piano::channel(s).unwrap_or(0);
    let chords = ideas::chords(notes, ch, bar);
    let last = chords.last().and_then(|c| ideas::degree(&key, &c.1));
    let x = r.x + 24.0;
    let w = (r.w - 48.0).min(1280.0);
    let mut y = r.y + 78.0 - s.page.scroll;
    let title = |ui: &mut Ui, y: f32, a: &str, b: &str| {
        let aw = ui.c.text_width(Font::new(19), a);
        ui.c.text(Font::new(19), x, y, t.text, a);
        ui.c.text(Font::new(14), x + aw + 12.0, y + 4.0, t.text_dim, b);
    };
    // Key
    title(ui, y, "Key", "");
    y += 30.0;
    button(ui, Rect::new(x, y, 34.0, 34.0), "<", Hit::Tonic(-1), "The key below", false);
    let kb = Rect::new(x + 40.0, y, 150.0, 34.0);
    ui.c.fill_rrect(kb, 8.0, t.sunken);
    ui.c.text_center(Font::new(19), kb, t.accent, &format!("{} {}", ideas::NAMES[usize::from(key.tonic)], if key.minor { "minor" } else { "major" }));
    button(ui, Rect::new(x + 196.0, y, 34.0, 34.0), ">", Hit::Tonic(1), "The key above", false);
    button(ui, Rect::new(x + 242.0, y, 130.0, 34.0), "Major / minor", Hit::Minor, "Change between major and minor", false);
    button(ui, Rect::new(x + 380.0, y, 180.0, 34.0), "Find it in the pattern", Hit::Detect, "Listens to the pattern's notes and finds their key", false);
    if let Some(c) = key.sure {
        ui.c.text_mid(Font::new(14), x + 574.0, y + 17.0, t.text_dim, &format!("{:.0}% sure", (c.clamp(0.0, 1.0)) * 100.0));
    }
    y += 56.0;
    // Chords
    title(ui, y, "Chords", "Each one is written as the next bar of the selected instrument");
    y += 30.0;
    let cw = ((w - 6.0 * 10.0) / 7.0).max(70.0);
    for (i, d) in key.degrees().iter().enumerate() {
        let b = Rect::new(x + i as f32 * (cw + 10.0), y, cw, 78.0);
        let a = Action::Page(Hit::Chord(i));
        let hov = ui.hit(b, a, "Write this chord as the next bar");
        let likely = last.and_then(|l| key.next(l).iter().find(|n| n.0 == i)).map(|n| n.1);
        let face = if hov { t.button_hi } else { t.button };
        ui.c.fill_rrect(b, 10.0, if likely.is_some() { face.mix(t.accent, 0.12) } else { face });
        let root = usize::from((key.tonic + key.scale()[i]) % 12);
        let q = if d.ends_with('o') { "dim" } else if d.chars().next().is_some_and(char::is_lowercase) { "m" } else { "" };
        ui.c.text_center(Font::new(26), Rect::new(b.x, b.y + 6.0, b.w, 34.0), t.text, &format!("{}{q}", ideas::NAMES[root]));
        ui.c.text_center(Font::new(14), Rect::new(b.x, b.y + 38.0, b.w, 18.0), t.text_dim, d);
        if let Some(p) = likely {
            let m = Rect::new(b.x + 12.0, b.bottom() - 14.0, b.w - 24.0, 5.0);
            ui.c.fill_rrect(m, 2.5, t.sunken);
            ui.c.fill_rrect(Rect::new(m.x, m.y, m.w * p as f32 / 70.0_f32.max(p as f32), m.h), 2.5, t.accent);
        }
    }
    y += 88.0;
    for (i, c) in key.colours().iter().enumerate() {
        button(ui, Rect::new(x + i as f32 * 98.0, y, 90.0, 30.0), c, Hit::Colour(i), "A chord with more colour", false);
    }
    let tip = match last {
        Some(_) => "The bars show where the last chord usually goes",
        None => "Write a first chord and the next ones light up",
    };
    ui.c.text_mid(Font::new(14), x + 500.0, y + 15.0, t.text_dim, tip);
    y += 52.0;
    // Progressions
    title(ui, y, "Progressions", "Four bars at once, by mood");
    y += 30.0;
    let n = (w / 190.0).floor().max(2.0);
    let pw = (w - (n - 1.0) * 10.0) / n;
    for (i, p) in ideas::PROGRESSIONS.iter().enumerate() {
        let b = Rect::new(x + (i as f32 % n) * (pw + 10.0), y + (i as f32 / n).floor() * 56.0, pw, 48.0);
        let hov = ui.hit(b, Action::Page(Hit::Progression(i)), "Write this progression in the pattern");
        ui.c.fill_rrect(b, 9.0, if hov { t.button_hi } else { t.button });
        ui.c.text(Font::new(16), b.x + 12.0, b.y + 5.0, t.text, p.0);
        ui.c.text(Font::new(14), b.x + 12.0, b.y + 25.0, t.accent, if key.minor { p.2 } else { p.1 });
    }
    y += (ideas::PROGRESSIONS.len() as f32 / n).ceil() * 56.0 + 14.0;
    // From your chords
    title(ui, y, "From your chords", "Written on the same instrument; undo takes it back");
    y += 30.0;
    let n = (w / 250.0).floor().max(2.0);
    let mw = (w - (n - 1.0) * 10.0) / n;
    for (i, (m, name, hint)) in Make::ALL.iter().enumerate() {
        let b = Rect::new(x + (i as f32 % n) * (mw + 10.0), y + (i as f32 / n).floor() * 44.0, mw, 36.0);
        button(ui, b, name, Hit::Make(*m), hint, false);
    }
    y += (Make::ALL.len() as f32 / n).ceil() * 44.0 + 20.0;
    s.page.height.set(y + s.page.scroll - r.y);
    head(ui, r, "Ideas", "For when the page is blank: everything becomes notes you can change");
}

// ------------------------------------------------------------------------------------------------ Export

fn export_page(ui: &mut Ui, s: &State, r: Rect) {
    use super::menus as m;
    let t = ui.t;
    ui.c.fill_rect(r, t.workspace);
    let cards: [(&str, &str, &tabler::TablerIcon, Hit); 6] = [
        ("Audio", "WAV, FLAC, OGG or MP3, with stems and quality settings", &tabler::WAVE_SINE, Hit::File(m::FILE_EXPORT)),
        ("MP3", "Ready to send or upload", &tabler::FILE_MUSIC, Hit::File(m::FILE_EXPORT_MP3)),
        ("MIDI", "The notes, for another program or keyboard", &tabler::PIANO, Hit::File(m::FILE_EXPORT_MIDI)),
        ("Sheet music", "MusicXML for MuseScore, Sibelius, Finale or Dorico", &tabler::MUSIC, Hit::Xml),
        ("Save", "The project, as it is", &tabler::DEVICE_FLOPPY, Hit::File(m::FILE_SAVE)),
        ("New version", "A numbered copy, so you can go back", &tabler::FILE_STAR, Hit::File(m::FILE_SAVE_VERSION)),
    ];
    let w = (r.w - 48.0).min(1180.0);
    let n = (w / 330.0).floor().max(1.0);
    let cw = (w - (n - 1.0) * 14.0) / n;
    let y0 = r.y + 84.0 - s.page.scroll;
    for (i, (name, what, icon, hit)) in cards.iter().enumerate() {
        let b = Rect::new(r.x + 24.0 + (i as f32 % n) * (cw + 14.0), y0 + (i as f32 / n).floor() * 108.0, cw, 94.0);
        let hov = ui.hit(b, Action::Page(*hit), *what);
        ui.c.fill_rrect(b, 12.0, if hov { t.button_hi } else { t.button });
        let ic = Rect::new(b.x + 18.0, b.y + 21.0, 52.0, 52.0);
        ui.c.fill_rrect(ic, 14.0, t.accent.with_alpha(40));
        ui.c.icon(icon, ic.inset(13.0), t.accent);
        ui.c.text(Font::new(21), b.x + 86.0, b.y + 20.0, t.text, name);
        ui.c.push_clip(b.inset(4.0));
        let f = ui.c.fit_font(Font::new(14), what, b.w - 100.0);
        ui.c.text(f, b.x + 86.0, b.y + 50.0, t.text_dim, what);
        ui.c.pop_clip();
    }
    s.page.height.set((cards.len() as f32 / n).ceil() * 108.0 + 100.0);
    head(ui, r, "Export", "Take your song out of here");
    let _ = tabs::on;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_on_the_staff() {
        // E4 on treble's bottom line, F5 on its top; G2 and A3 in bass; middle C one ledger line away from both.
        assert_eq!((staff_pos(2, 4, Clef::Treble), staff_pos(3, 5, Clef::Treble)), (0, 8));
        assert_eq!((staff_pos(4, 2, Clef::Bass), staff_pos(5, 3, Clef::Bass)), (0, 8));
        assert_eq!((staff_pos(0, 4, Clef::Treble), staff_pos(0, 4, Clef::Bass)), (-2, 10));
        // F sharp on treble's top line, B flat on its middle one.
        assert_eq!((key_pos(0, true, Clef::Treble), key_pos(0, false, Clef::Treble)), (8, 4));
        assert_eq!(staff_pos(3, 5, Clef::Treble), key_pos(0, true, Clef::Treble));
        assert_eq!(staff_pos(3, 3, Clef::Bass), key_pos(0, true, Clef::Bass));
    }
}
