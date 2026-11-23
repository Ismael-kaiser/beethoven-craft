// Copyright (C) 2026 mixter-ha99
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Beethoven DAW.

//! The top bar of the tabbed workspace: menus, what plays in the middle, switches and levels on the right. Every
//! control is one of the app's own actions.

use os_ui::{Canvas, Font, Rect, tabler};

use super::{menus, FrameInfo};
use crate::app::{Action, Live, State, Ui, Util};
use crate::knobs::KnobId;

pub const H: f32 = 56.0;
/// Room for the window's own buttons.
const LEAD: f32 = if cfg!(target_os = "macos") { 82.0 } else { 12.0 };

const MENU_FONT: Font = Font::new(16);

/// The menus' titles.
pub fn menu_rects(c: &mut Canvas) -> Vec<Rect> {
    let mut x = LEAD;
    menus::MENUS
        .iter()
        .map(|m| {
            let t = title(m.title);
            let w = c.text_width(MENU_FONT, &t) + 16.0;
            let r = Rect::new(x, 14.0, w, 28.0);
            x += w;
            r
        })
        .collect()
}

/// "FILE" as "File", in the app's language.
fn title(t: &str) -> String {
    let t = os_ui::i18n::t(t);
    let mut cs = t.chars();
    cs.next().map(|f| f.to_uppercase().collect::<String>() + &cs.as_str().to_lowercase()).unwrap_or_default()
}

/// The pattern's box, on the tab strip.
pub fn pattern_rect(w: f32) -> Rect {
    Rect::new(w - 318.0, H + 6.0, 186.0, 26.0)
}

fn snap_rect(w: f32) -> Rect {
    Rect::new(w - 122.0, H + 6.0, 112.0, 26.0)
}

fn round(ui: &mut Ui, cx: f32, cy: f32, k: f32, lit: bool, hov: bool) -> os_ui::Color {
    let t = ui.t;
    ui.c.fill_circle(cx, cy, k, if lit { t.accent } else if hov { t.button_hi } else { t.button });
    if lit { t.on_glyph } else { t.glyph }
}

fn small(ui: &mut Ui, x: f32, icon: &tabler::TablerIcon, on: bool, a: Action, hint: impl Into<std::borrow::Cow<'static, str>>) {
    let t = ui.t;
    let r = Rect::new(x, 13.0, 32.0, 30.0);
    let hov = ui.hit(r, a.clone(), hint);
    let down = ui.down(r, &a);
    if on {
        ui.c.fill_rrect(r, 9.0, t.accent.with_alpha(48));
    } else if hov || down {
        ui.c.fill_rrect(r, 9.0, t.button);
    }
    ui.c.icon(icon, Rect::new(r.cx() - 9.0, r.cy() - 9.0, 18.0, 18.0), if on { t.accent } else if hov { t.text } else { t.glyph });
}

pub fn paint(ui: &mut Ui, s: &State, f: &FrameInfo, w: f32) {
    let t = ui.t;
    ui.c.fill_rect(Rect::new(0.0, 0.0, w, H), t.toolbar);
    let mut end = LEAD;
    for (i, r) in menu_rects(ui.c).into_iter().enumerate() {
        let m = &menus::MENUS[i];
        let hov = ui.hit(r, Action::Menu(i), m.title);
        let open = s.menu_open == Some(i);
        if hov || open {
            ui.c.fill_rrect(r, 8.0, if open { t.button_hi } else { t.button });
        }
        ui.c.text_center(MENU_FONT, r, if hov || open { t.text } else { t.text_dim }, &title(m.title));
        end = r.right();
    }
    let edge = center(ui, s, f, w, end);
    right(ui, s, w, edge + 10.0);
}

/// What plays, the transport, the clock and the tempo; returns where they end.
fn center(ui: &mut Ui, s: &State, f: &FrameInfo, w: f32, end: f32) -> f32 {
    let t = ui.t;
    // Between the menus and the volume in a narrow window.
    // (what the smallest ones take off the switch, the clock and the tempo)
    let tight = if w < 1130.0 { 22.0 } else { 0.0 };
    let x0 = ((w - 470.0) / 2.0).min(w - 800.0).max(end + 14.0).min(w - 552.0 + 3.0 * tight).round();
    let cy = H / 2.0;
    // What plays
    let seg = Rect::new(x0, 14.0, 132.0 - tight, 28.0);
    ui.c.fill_rrect(seg, 9.0, t.sunken);
    for (i, (label, song, hint)) in [("Pattern", false, "Pattern: play the current pattern"), ("Song", true, "Song: play the arrangement")].into_iter().enumerate() {
        let r = Rect::new(seg.x + seg.w / 2.0 * i as f32, seg.y, seg.w / 2.0, seg.h);
        let hov = ui.hit(r, Action::PatSong(song), hint);
        let on = s.song_mode() == song;
        if on {
            ui.c.fill_rrect(r.inset(2.5), 7.0, t.button_hi);
        }
        ui.c.text_center(Font::new(14), r, if on { t.text } else if hov { t.glyph } else { t.text_dim }, label);
    }
    // Transport
    let sx = x0 + 160.0 - tight;
    let stop = Rect::new(sx - 15.0, cy - 15.0, 30.0, 30.0);
    let hov = ui.hit(stop, Action::Stop, "Stop (Space)");
    let ink = round(ui, sx, cy, 15.0, false, hov);
    ui.c.fill_rrect(Rect::new(sx - 5.0, cy - 5.0, 10.0, 10.0), 2.0, ink);
    let px = sx + 44.0;
    let play = Rect::new(px - 20.0, cy - 20.0, 40.0, 40.0);
    let hov = ui.hit(play, Action::Play, "Play / pause (Space plays / stops)");
    let on = f.playing && !f.paused;
    ui.c.fill_circle(px, cy, 20.0, if hov { t.accent.shade(0.12) } else { t.accent });
    let ink = t.on_glyph;
    if on {
        for dx in [-6.5, 2.5] {
            ui.c.fill_rrect(Rect::new(px + dx, cy - 7.5, 4.0, 15.0), 1.5, ink);
        }
    } else {
        ui.c.fill_polygon(&[(px - 5.5, cy - 8.5), (px + 9.0, cy), (px - 5.5, cy + 8.5)], ink);
    }
    let rx = px + 44.0;
    let rec = Rect::new(rx - 15.0, cy - 15.0, 30.0, 30.0);
    let hov = ui.hit(rec, Action::Record, crate::record::hint(&crate::settings_file::store()));
    let armed = s.session.rec.armed;
    ui.c.fill_circle(rx, cy, 15.0, if armed { t.record } else if hov { t.button_hi } else { t.button });
    ui.c.fill_circle(rx, cy, 6.0, if armed { t.text } else { t.record });
    // Clock
    let clock = Rect::new(rx + 30.0, 10.0, 116.0 - tight, 36.0);
    ui.c.fill_rrect(clock, 10.0, t.time_panel);
    ui.hit(clock, Action::Clock, s.clock.hint());
    ui.live(Live::Clock, clock);
    let ppq = f64::from(s.session.project.ppq.max(1));
    let sig = s.session.project.signature().unwrap_or((s.session.project.bar().0, 4));
    let secs = f.tick / ppq * 60.0 / s.session.project.tick_tempo(s.tempo().max(1.0));
    let ticks96 = f.tick * 96.0 / ppq;
    let start = crate::playlist::start_marker(&s.session).filter(|_| f.song && s.clock.mode == crate::clock::Mode::Bar);
    let (bar, step, tick) = match start {
        Some(z) => s.clock.bars_from(ticks96, (f64::from(z) * 96.0 / ppq / 384.0).floor() * 384.0, sig),
        None => {
            let (b, st, t) = s.clock.reading(secs, ticks96, sig);
            (i64::from(b), st, t)
        }
    };
    let text = match s.session.rec.count.filter(|c| c.left() > 0) {
        Some(c) => format!("REC {}", c.left()),
        None => format!("{bar}:{step:02}:{tick:02}"),
    };
    ui.c.text_center(Font::new(24), clock, t.text, &text);
    // Tempo
    let tb = Rect::new(clock.right() + 8.0, 10.0, 92.0 - tight, 36.0);
    let hov = ui.hit(tb, Action::Tempo, "Tempo (drag up/down: the whole number, or the decimal place the drag starts on)");
    ui.c.fill_rrect(tb, 10.0, if hov { t.button } else { t.time_panel });
    let whole = format!("{}", f.tempo.trunc() as i64);
    let frac = format!(".{:03}", ((f.tempo.fract()) * 1000.0).round() as i64 % 1000);
    let (bf, sf) = (Font::new(22), Font::new(13));
    let (ww, fw) = (ui.c.text_width(bf, &whole), ui.c.text_width(sf, &frac));
    let x = tb.x + ((tb.w - ww - fw) / 2.0).round();
    let mut digit = |n: usize| x + ww + ui.c.text_width(sf, &frac[..n]);
    s.tempo_digits.set([digit(1), digit(2), digit(3)]);
    ui.c.text_mid(bf, x, cy, t.text, &whole);
    ui.c.text_mid(sf, x + ww, cy + 3.0, t.text_dim, &frac);
    tb.right()
}

/// The volume, then what fits before `edge`.
fn right(ui: &mut Ui, s: &State, w: f32, edge: f32) {
    let t = ui.t;
    let mut x = w - 14.0;
    // Volume
    x -= 34.0;
    let (cx, cy) = (x + 17.0, H / 2.0);
    ui.hit(Rect::new(x, 10.0, 34.0, 36.0), Action::Knob(KnobId::MasterVol), s.knob_text(KnobId::MasterVol));
    let r = 16.0;
    let shown = s.knob_in(KnobId::MasterVol, Rect::new((cx - r) * ui.k, (cy - r) * ui.k, 2.0 * r * ui.k, 2.0 * r * ui.k));
    super::knob(ui, cx, cy, 14.0, shown, false, t.glyph, t.button_hi);
    x -= 12.0;
    let save: [(&tabler::TablerIcon, bool, Action, std::borrow::Cow<'static, str>); 6] = [
        (&tabler::DEVICE_FLOPPY, false, Action::Util(Util::Save), "Save as... / Save new version (Shift+Ctrl+S)".into()),
        (&tabler::ARROW_BACK_UP, false, Action::Util(Util::Undo), "Undo / Undo history (Ctrl+Z)".into()),
        (&tabler::METRONOME, crate::metronome::on(), Action::Metronome, "Metronome (Ctrl+M) - right-click for the sound and volume".into()),
        (&tabler::REPEAT, crate::record::Opt::Loop.on(&crate::settings_file::store()), Action::RecOpt(crate::record::Opt::Loop), crate::record::option_hint(crate::record::Opt::Loop).into()),
        (&tabler::MICROPHONE, s.session.rec.audio, Action::OneClick, "Quick record".into()),
        (&tabler::KEYBOARD, s.toggles.contains(&"typing"), Action::Typing, "Typing keyboard to piano keyboard (Ctrl+T)".into()),
    ];
    for (icon, on, a, hint) in save {
        x -= 34.0;
        if x < edge {
            return;
        }
        small(ui, x, icon, on, a, hint);
    }
    // The meters, where there is room left.
    x -= 12.0;
    // Level
    x -= 84.0;
    if x < edge {
        return;
    }
    let m = Rect::new(x, 19.0, 72.0, 18.0);
    ui.hit(m, Action::Swallow, "Output level (peak L / R)");
    for i in 0..2 {
        let b = Rect::new(m.x, m.y + 10.0 * i as f32, m.w, 7.0);
        ui.c.fill_rrect(b, 3.5, t.button);
        let k = s.monitor.meter_pos(s.monitor.meter_db[i]).clamp(0.0, 1.0);
        if k > 0.02 {
            let c = if s.monitor.meter_db[i] > 0.0 { t.meter_clip } else { t.meter_low.mix(t.meter_high, k) };
            ui.c.fill_rrect(Rect::new(b.x, b.y, (b.w * k).max(7.0), b.h), 3.5, c);
        }
    }
    // CPU
    x -= 62.0;
    if x < edge {
        return;
    }
    let c = Rect::new(x, 13.0, 52.0, 30.0);
    ui.hit(c, Action::CpuPanel, "");
    ui.c.text_center(Font::new(15), Rect::new(c.x, c.y - 4.0, c.w, 20.0), if s.cpu.now.load > 85 { t.meter_clip } else { t.text_dim }, &format!("{}%", s.cpu.now.load));
    ui.c.text_center(Font::new(11), Rect::new(c.x, c.y + 13.0, c.w, 16.0), t.text_faint, "CPU");
}

/// What the tab strip holds at its right end: the pattern and the snap.
pub fn strip_tools(ui: &mut Ui, s: &State, w: f32) {
    let t = ui.t;
    let p = pattern_rect(w);
    ui.c.fill_rrect(p, 8.0, t.sunken);
    let prev = Rect::new(p.x, p.y, 24.0, p.h);
    let hov = ui.hit(prev, Action::PatternPrev, "Pattern options");
    ui.c.icon(&tabler::CARET_RIGHT, prev.inset(6.0), if hov { t.text } else { t.glyph_dim });
    let name = crate::patterns::name_of(&s.session, s.pattern().max(1));
    let mid = Rect::new(prev.right(), p.y, p.w - 50.0, p.h);
    let hov = ui.hit(mid, Action::PatternNext, name.clone());
    ui.c.push_clip(mid);
    ui.c.text_mid(Font::new(15), mid.x + 2.0, mid.cy(), if hov { t.text } else { t.glyph }, &name);
    ui.c.pop_clip();
    let add = Rect::new(p.right() - 26.0, p.y, 26.0, p.h);
    let hov = ui.hit(add, Action::PatternAdd, "New pattern (find first / next empty)");
    ui.c.icon(&tabler::PLUS, add.inset(7.0), if hov { t.accent } else { t.glyph });
    let n = snap_rect(w);
    ui.c.fill_rrect(n, 8.0, t.sunken);
    let global = s.global_snap.now;
    let on = global != crate::playlist::Snap::None;
    let m = Rect::new(n.x, n.y, 28.0, n.h);
    ui.hit(m, Action::GlobalSnapToggle, crate::snap::MAGNET_HINT);
    ui.c.icon(&tabler::MAGNET, Rect::new(m.cx() - 8.0, m.cy() - 8.0, 16.0, 16.0), if on { t.accent } else { t.text_faint });
    let r = Rect::new(m.right(), n.y, n.w - 28.0, n.h);
    let hov = ui.hit(r, Action::GlobalSnap, crate::snap::HINT);
    let f = ui.c.fit_font(Font::new(15), global.label(), r.w - 10.0);
    ui.c.text_mid(f, r.x + 2.0, r.cy(), if hov { t.text } else { t.glyph }, global.label());
}
