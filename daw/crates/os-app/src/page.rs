// Copyright (C) 2026 mixter-ha99
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Beethoven DAW.

//! State and actions of the pages that are tabs of their own: Score, Ideas and Export.

use crate::app::{App, State};
use crate::ideas::{Key, Make};
use crate::score;

/// What the Score tab shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Channel,
    Pattern,
    Song,
}

/// What the pencil writes on the sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pen {
    /// Units: 16 = whole .. 1 = sixteenth.
    pub dur: u32,
    pub dot: bool,
    /// Sharp, flat or natural instead of what the key signature says.
    pub alter: Option<i32>,
    pub erase: bool,
}

impl Pen {
    pub fn units(&self) -> u32 {
        self.dur + if self.dot { self.dur / 2 } else { 0 }
    }
    /// What presses snap to.
    fn grid(&self) -> u32 {
        self.dur.min(score::Q)
    }
}

/// A measure as it was painted: where the mouse is on it.
#[derive(Debug, Clone, Copy)]
pub struct Bar {
    pub clef: score::Clef,
    pub measure: u32,
    /// The notes' part, between bar lines.
    pub x: f32,
    pub w: f32,
    /// The staff's top line and the space between lines.
    pub sy: f32,
    pub sp: f32,
}

/// A place on the sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    pub bar: usize,
    /// Units from the pattern's start, on the pen's grid.
    pub start: u32,
    pub pos: i32,
    pub key: u8,
}

/// Where the mouse is on a measure: units into it (not snapped) and staff place.
impl Bar {
    pub fn at(&self, x: f32, y: f32, total: u32) -> (f32, i32) {
        let t = (x - self.x - self.sp * HEAD) / self.w.max(1.0) * total as f32;
        (t, (8.0 - (y - self.sy) / (self.sp / 2.0)).round() as i32)
    }
}

/// Half a note head's width, in staff spaces.
pub const HEAD: f32 = 0.66;

/// A drag on the sheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stroke {
    Erase,
    /// The note just written: it follows the mouse up and down, and others come after it to the right.
    Paint { start: u32, key: u8 },
    /// A note held: (tick, key) it is at, where it was grabbed and whether it moved.
    Move { tick: u32, key: u8, from: (u32, i32), moved: bool },
}

#[derive(Debug, Clone)]
pub struct Page {
    pub pen: Pen,
    /// The sheet's measures, as painted last.
    pub bars: std::cell::RefCell<Vec<Bar>>,
    /// The place under the mouse, shown as a pale note.
    pub hover: Option<Spot>,
    pub scope: Scope,
    pub zoom: f32,
    pub scroll: f32,
    /// Height of what was painted last, for the scroll's limit.
    pub height: std::cell::Cell<f32>,
    pub key: Key,
    pub seed: u32,
    /// The last melody written, replaced by the next one.
    pub melody: Vec<os_model::Note>,
    /// The Rhythm tab's grooves: which piece, and the even rhythm's hits and turn.
    pub piece: crate::grooves::Piece,
    pub euclid: u32,
    pub rotate: u32,
}

impl Default for Page {
    fn default() -> Self {
        Page { pen: Pen { dur: 4, dot: false, alter: None, erase: false }, bars: Default::default(), hover: None, scope: Scope::Channel, zoom: 1.0, scroll: 0.0, height: std::cell::Cell::new(0.0), key: Key::default(), seed: 1, melody: vec![], piece: crate::grooves::Piece::Kick, euclid: 3, rotate: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    Scope(Scope),
    Zoom(i8),
    Xml,
    Detect,
    Tonic(i8),
    Minor,
    Chord(usize),
    Colour(usize),
    Progression(usize),
    Make(Make),
    /// An entry of the FILE menu.
    File(usize),
    Library,
    Piece(crate::grooves::Piece),
    Groove(usize),
    Euclid(i8),
    Rotate(i8),
    ClearSteps,
    /// The sheet itself.
    Paper,
    Dur(u32),
    Dot,
    Alter(i32),
    Eraser,
    /// The pattern's handle, dragged to Arrange, and its button.
    Carry,
    ToArrange,
}

/// What the Score tab shows: the sheet to write on for one instrument, else the score.
pub fn shown(s: &State) -> score::Score {
    if s.page.scope != Scope::Channel {
        return score(s);
    }
    let (beats, ty) = s.session.project.signature().unwrap_or((4, 4));
    let part = channel_part(s);
    let title = if part.name.is_empty() { title(s) } else { part.name.clone() };
    score::sheet(&title, s.tempo(), u32::from(beats), u32::from(ty), part, (s.page.key.tonic, s.page.key.minor))
}

/// Ticks in a unit.
fn unit(s: &State) -> u32 {
    (u32::from(s.session.project.ppq.max(4)) / score::Q).max(1)
}

/// The selected instrument's notes, straight from the pattern.
fn channel_part(s: &State) -> score::Part {
    let unit = unit(s);
    let grid = |t: u32| (t + unit / 2) / unit;
    let ch = crate::piano::channel(s);
    let mut v: Vec<score::Note> = crate::edit::notes::notes(&s.session)
        .iter()
        .filter(|n| Some(n.channel) == ch)
        .map(|n| score::Note { start: grid(n.position), dur: grid(n.length.max(unit)).max(1), key: n.key.min(127) as u8 })
        .collect();
    v.sort_by_key(|n| (n.start, n.key));
    v.dedup_by(|a, b| a.start == b.start && a.key == b.key);
    let name = if ch.is_some() { crate::edit::rack::channel_name(&s.session, s.selected_channel) } else { String::new() };
    score::Part { name, notes: v }
}

fn title(s: &State) -> String {
    let file = || s.session.path.as_deref().and_then(|p| p.file_stem()).map_or("Untitled".into(), |f| f.to_string_lossy().into_owned());
    s.session.project.info.title.clone().filter(|t| !t.trim().is_empty()).unwrap_or_else(file)
}

/// Units in a measure of the pattern.
fn measure(s: &State) -> u32 {
    let (beats, ty) = s.session.project.signature().unwrap_or((4, 4));
    (u32::from(beats.max(1)) * score::Q * 4 / u32::from(ty.max(1))).max(1)
}

fn fifths(s: &State) -> i32 {
    score::fifths(s.page.key.tonic, s.page.key.minor)
}

/// The sheet's measure under the mouse.
fn bar_at(s: &State, x: f32, y: f32) -> Option<(usize, Bar)> {
    let far = |b: &Bar| (y - (b.sy + 2.0 * b.sp)).abs();
    let bars = s.page.bars.borrow();
    bars.iter().copied().enumerate().filter(|(_, b)| x >= b.x - b.sp && x < b.x + b.w + b.sp * 0.6 && far(b) < 7.0 * b.sp).min_by(|a, b| far(&a.1).total_cmp(&far(&b.1)))
}

/// The place under the mouse.
pub fn spot(s: &State, x: f32, y: f32) -> Option<Spot> {
    let (i, b) = bar_at(s, x, y)?;
    let m = measure(s);
    let (t, pos) = b.at(x, y, m);
    let g = s.page.pen.grid();
    let at = ((t / g as f32).round().max(0.0) as u32 * g).min((m - 1) / g * g);
    let key = score::key_at(pos, b.clef, fifths(s), s.page.pen.alter)?;
    Some(Spot { bar: i, start: b.measure * m + at, pos, key })
}

/// The selected instrument's note whose head is under the mouse: its index in the pattern.
pub fn note_under(s: &State, x: f32, y: f32) -> Option<usize> {
    let (_, b) = bar_at(s, x, y)?;
    let (m, u, ch) = (measure(s), unit(s), crate::piano::channel(s)?);
    let (_, pos) = b.at(x, y, m);
    let f = fifths(s);
    crate::edit::notes::notes(&s.session).iter().position(|n| {
        let start = (n.position + u / 2) / u;
        let key = n.key.min(127) as u8;
        let (step, _, oct) = score::spell(key, f);
        let clef = if key >= 60 { score::Clef::Treble } else { score::Clef::Bass };
        let hx = b.x + b.w * (start % m) as f32 / m as f32 + b.sp * HEAD;
        n.channel == ch && start / m == b.measure && clef == b.clef && crate::views::page::staff_pos(step, oct, clef) == pos && (x - hx).abs() < b.sp * 1.1
    })
}

fn find(s: &State, tick: u32, key: u8) -> Option<usize> {
    let ch = crate::piano::channel(s)?;
    crate::edit::notes::notes(&s.session).iter().position(|n| n.channel == ch && n.position == tick && n.key == u16::from(key))
}

/// Writes the pen's note at `start`; nothing when one is there.
fn write_note(s: &mut State, start: u32, key: u8) {
    let (Some(ch), u) = (crate::piano::channel(s), unit(s)) else { return };
    if find(s, start * u, key).is_some() {
        return;
    }
    let i = crate::piano::add_note(s, ch, start * u, i32::from(key));
    crate::edit::notes::pattern_mut(&mut s.session).notes[i].length = s.page.pen.units() * u;
    crate::piano::finish_edit(s);
}

fn remove_note(s: &mut State, i: usize) {
    crate::edit::notes::pattern_mut(&mut s.session).notes.remove(i);
    let (pattern, ch) = (s.pattern(), crate::piano::channel(s).unwrap_or(0));
    s.piano.select(pattern, ch, vec![]);
}

/// Puts the pattern shown in the arrangement, after its last clip there. The message to show.
pub fn to_arrange(s: &mut State) -> String {
    use os_model::ClipSource;
    let n = s.pattern().max(1);
    let name = crate::patterns::name_of(&s.session, n);
    if crate::edit::notes::notes(&s.session).is_empty() {
        return format!("{name} has no notes yet");
    }
    let src = ClipSource::Pattern(n);
    let len = crate::edit::playlist::source_len(&s.session, src).max(1);
    let last = crate::edit::playlist::clips(&s.session).iter().filter(|c| c.source == src).max_by_key(|c| c.end()).map(|c| (c.track, c.end()));
    let (track, tick) = last.filter(|l| crate::playlist::free_track(s, l.0, l.1, l.1 + len) == l.0).unwrap_or_else(|| {
        let t = crate::playlist::free_track(s, 0, 0, len);
        (t, 0)
    });
    crate::playlist::snapshot(&mut s.session);
    s.playlist.source = Some(src);
    s.playlist.picker = crate::playlist::PickKind::Pattern;
    let i = crate::playlist::place(s, track, tick);
    s.playlist.sel = vec![i];
    let bar = s.session.project.bar_ticks().max(1);
    format!("{name}: track {}, bar {}", track + 1, tick / bar + 1)
}

/// The score of what `scope` shows, in the key detected.
pub fn score(s: &State) -> score::Score {
    let p = &s.session.project;
    let midi = match s.page.scope {
        Scope::Song => crate::midi_file::project_smf(s),
        _ => crate::midi_file::pattern_smf(s),
    };
    let mut parts = score::parts(&midi);
    if s.page.scope == Scope::Channel {
        let part = channel_part(s);
        parts = if part.notes.is_empty() { vec![] } else { vec![part] };
    }
    let (beats, ty) = p.signature().unwrap_or((4, 4));
    score::build(&title(s), s.tempo(), u32::from(beats), u32::from(ty), &parts, None)
}

impl App {
    pub fn page_act(&mut self, h: Hit) {
        let s = &mut self.state;
        match h {
            Hit::Scope(c) => {
                s.page.scope = c;
                s.page.scroll = 0.0;
            }
            Hit::Zoom(d) => s.page.zoom = (s.page.zoom + f32::from(d) * 0.15).clamp(0.55, 2.2),
            Hit::Detect => {
                s.message = Some(match crate::ideas::detect(s) {
                    Some(k) => {
                        s.page.key = k;
                        format!("The pattern sounds in {}", k.name())
                    }
                    None => "There are no notes in the pattern to find the key from".into(),
                });
            }
            Hit::Tonic(d) => {
                s.page.key.tonic = (s.page.key.tonic + (12 + d) as u8) % 12;
                s.page.key.sure = None;
            }
            Hit::Minor => {
                s.page.key.minor = !s.page.key.minor;
                s.page.key.sure = None;
            }
            Hit::Chord(i) => self.write_chords(|k| k.degrees()[i % 7].into()),
            Hit::Colour(i) => self.write_chords(|k| k.colours()[i % 5].into()),
            Hit::Progression(i) => self.write_chords(|k| {
                let p = crate::ideas::PROGRESSIONS[i % 12];
                if k.minor { p.2 } else { p.1 }.into()
            }),
            Hit::Make(m) => {
                let t = crate::ideas::run(s, m);
                s.message = Some(t);
            }
            Hit::Xml => self.export_xml_dialog(),
            Hit::File(i) => return self.act(crate::app::Action::MenuItem(0, i)),
            Hit::Piece(p) => s.page.piece = p,
            Hit::Groove(i) => {
                let v = crate::grooves::steps(i, s.page.piece);
                let r = s.selected_channel;
                crate::grooves::write(s, r, &v);
            }
            Hit::Euclid(d) | Hit::Rotate(d) => {
                if matches!(h, Hit::Euclid(_)) {
                    s.page.euclid = s.page.euclid.saturating_add_signed(i32::from(d)).min(16);
                } else {
                    s.page.rotate = (s.page.rotate + (16 + d) as u32) % 16;
                }
                let v = crate::grooves::euclid(s.page.euclid, 16, s.page.rotate);
                let r = s.selected_channel;
                crate::grooves::write(s, r, &v);
            }
            Hit::ClearSteps => {
                let r = s.selected_channel;
                crate::grooves::write(s, r, &[false; 16]);
            }
            Hit::Paper => return self.score_press(false),
            Hit::Dur(d) => {
                s.page.pen.dur = d;
                s.page.pen.erase = false;
            }
            Hit::Dot => s.page.pen.dot = !s.page.pen.dot,
            Hit::Alter(a) => s.page.pen.alter = (s.page.pen.alter != Some(a)).then_some(a),
            Hit::Eraser => s.page.pen.erase = !s.page.pen.erase,
            Hit::Carry => {
                let n = s.pattern().max(1);
                let label = crate::patterns::name_of(&s.session, n);
                self.drag = Some(crate::app::Drag::Browser { item: crate::views::browser::Item::Pattern(n), label, m0: self.mouse, started: false });
            }
            Hit::ToArrange => {
                let before = crate::edit::playlist::clips(&s.session).len();
                let m = to_arrange(s);
                if crate::edit::playlist::clips(&s.session).len() > before {
                    crate::tabs::show(s, crate::tabs::Tab::Arrange);
                }
                s.message = Some(m);
            }
            Hit::Library => s.show_browser = !s.show_browser,
        }
        self.invalidate();
    }

    /// A press on the sheet; the right button erases.
    pub fn score_press(&mut self, right: bool) {
        use crate::edit::notes::snapshot_as;
        let (x, y) = self.mouse;
        let s = &mut self.state;
        if s.page.scope != Scope::Channel {
            s.message = Some("Choose Instrument to write on the sheet".into());
            return self.invalidate();
        }
        s.page.hover = None;
        let under = note_under(s, x, y);
        let stroke = if right || s.page.pen.erase {
            snapshot_as(&mut s.session, "Erase on the score");
            if let Some(i) = under {
                remove_note(s, i);
            }
            Stroke::Erase
        } else if let Some(i) = under {
            snapshot_as(&mut s.session, "Score edit");
            let n = crate::edit::notes::notes(&s.session)[i];
            let Some(p) = spot(s, x, y) else { return };
            Stroke::Move { tick: n.position, key: n.key.min(127) as u8, from: (p.start, p.pos), moved: false }
        } else if let Some(p) = spot(s, x, y) {
            if crate::piano::channel(s).is_none() {
                s.message = Some("Ideas: select an instrument first".into());
                return self.invalidate();
            }
            snapshot_as(&mut s.session, "Write on the score");
            write_note(s, p.start, p.key);
            Stroke::Paint { start: p.start, key: p.key }
        } else {
            return;
        };
        let heard = match stroke {
            Stroke::Paint { key, .. } | Stroke::Move { key, .. } => Some(i32::from(key)),
            Stroke::Erase => None,
        };
        self.drag = Some(crate::app::Drag::Score(stroke));
        let play = self.audition();
        self.hold_key(heard, play);
        self.invalidate();
    }

    pub fn score_drag(&mut self, k: Stroke) {
        let (x, y) = self.mouse;
        let s = &mut self.state;
        let u = unit(s);
        let (next, heard) = match k {
            Stroke::Erase => {
                let Some(i) = note_under(s, x, y) else { return };
                remove_note(s, i);
                (k, None)
            }
            Stroke::Paint { start, key } => {
                let Some(p) = spot(s, x, y).filter(|p| (p.start, p.key) != (start, key)) else { return };
                if p.start == start {
                    if let Some(i) = find(s, start * u, key) {
                        crate::edit::notes::pattern_mut(&mut s.session).notes[i].key = u16::from(p.key);
                    }
                } else {
                    write_note(s, p.start, p.key);
                }
                (Stroke::Paint { start: p.start, key: p.key }, Some(p.key))
            }
            Stroke::Move { tick, key, from, .. } => {
                let Some(p) = spot(s, x, y).filter(|p| (p.start, p.pos) != from) else { return };
                let at = (i64::from(tick) + (i64::from(p.start) - i64::from(from.0)) * i64::from(u)).max(0) as u32;
                let to = if p.pos == from.1 { key } else { p.key };
                if let Some(i) = find(s, tick, key) {
                    let n = &mut crate::edit::notes::pattern_mut(&mut s.session).notes[i];
                    (n.position, n.key) = (at, u16::from(to));
                    crate::piano::finish_edit(s);
                }
                (Stroke::Move { tick: at, key: to, from: (p.start, p.pos), moved: true }, (to != key).then_some(to))
            }
        };
        self.drag = Some(crate::app::Drag::Score(next));
        if let Some(h) = heard {
            let play = self.audition();
            self.hold_key(Some(i32::from(h)), play);
        }
        self.invalidate();
    }

    /// The button up: a note held and not moved takes the pen's length and accidental.
    pub fn score_release(&mut self, k: Stroke) {
        let s = &mut self.state;
        if let Stroke::Move { tick, key, moved: false, .. } = k {
            let len = s.page.pen.units() * unit(s);
            let alter = s.page.pen.alter;
            let f = fifths(s);
            if let Some(i) = find(s, tick, key) {
                let n = &mut crate::edit::notes::pattern_mut(&mut s.session).notes[i];
                n.length = len;
                if let Some(a) = alter {
                    let (_, was, _) = score::spell(key, f);
                    n.key = (i32::from(n.key) - was + a).clamp(0, 127) as u16;
                }
            }
        }
        self.invalidate();
    }

    /// The mouse moved over the sheet: the pale note follows it.
    pub fn score_hover(&mut self, over: bool) {
        let s = &mut self.state;
        let (x, y) = self.mouse;
        let want = over && s.page.scope == Scope::Channel && !s.page.pen.erase;
        let h = if want && note_under(s, x, y).is_none() { spot(s, x, y) } else { None };
        if h != s.page.hover {
            s.page.hover = h;
            self.invalidate();
        }
    }

    fn write_chords(&mut self, f: impl FnOnce(&Key) -> String) {
        let k = self.state.page.key;
        let t = crate::chord_tool::write(&mut self.state, &format!("{}: {}", k.name(), f(&k)));
        self.state.message = Some(t);
    }

    fn export_xml_dialog(&mut self) {
        use crate::dialog::Then;
        use crate::filedlg::{FileDlg, Mode};
        let s = &self.state;
        if score(s).parts.is_empty() {
            self.state.message = Some("There are no notes to write as a score".into());
            return;
        }
        let name = format!("{}.musicxml", s.session.path.as_deref().and_then(|p| p.file_stem()).map_or("Untitled".into(), |f| f.to_string_lossy().into_owned()));
        let dlg = FileDlg::new(Mode::Save, "Export score", "score", Then::ScoreExport)
            .filter("MusicXML", &["musicxml", "xml"])
            .start(s.session.path.as_deref().and_then(std::path::Path::parent))
            .name(&name);
        self.file_dialog(dlg);
    }

    /// The save dialog's file: written.
    pub fn score_export_chosen(&mut self, path: std::path::PathBuf) {
        let path = if path.extension().is_none() { path.with_extension("musicxml") } else { path };
        self.state.message = Some(match std::fs::write(&path, score::musicxml(&score(&self.state))) {
            Ok(()) => format!("Exported {}", path.display()),
            Err(e) => format!("Could not write {}: {e}", path.display()),
        });
        self.invalidate();
    }

    /// The wheel over a page: scrolls it.
    pub fn page_wheel(&mut self, dy: f32, ctrl: bool) {
        let s = &mut self.state;
        if ctrl {
            s.page.zoom = (s.page.zoom * (1.0 - dy * 0.08)).clamp(0.55, 2.2);
        } else {
            let max = (s.page.height.get() - s.work_rect().h + 40.0).max(0.0);
            s.page.scroll = (s.page.scroll + dy * 48.0).clamp(0.0, max);
        }
        self.invalidate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Drag;

    fn sheet() -> App {
        let app = App::new(None);
        app.state.page.bars.borrow_mut().extend([
            Bar { clef: score::Clef::Treble, measure: 0, x: 100.0, w: 160.0, sy: 50.0, sp: 10.0 },
            Bar { clef: score::Clef::Bass, measure: 0, x: 100.0, w: 160.0, sy: 155.0, sp: 10.0 },
        ]);
        app
    }
    /// The mouse on unit `u`, staff place `pos` of the treble staff.
    fn on(app: &mut App, u: u32, pos: i32) {
        app.mouse = (100.0 + 10.0 * u as f32 + 6.6, 90.0 - 5.0 * pos as f32);
    }
    fn all(app: &App) -> Vec<(u32, u32, u16)> {
        let u = unit(&app.state);
        crate::edit::notes::notes(&app.state.session).iter().map(|n| (n.position / u, n.length / u, n.key)).collect()
    }
    fn up(app: &mut App) {
        if let Some(Drag::Score(k)) = app.drag.take() {
            app.score_release(k);
        }
        app.hold_key(None, false);
    }
    fn drag(app: &mut App) {
        let Some(Drag::Score(k)) = app.drag.clone() else { panic!("no stroke") };
        app.score_drag(k);
    }

    #[test]
    fn writing_on_the_sheet() {
        let mut app = sheet();
        on(&mut app, 4, 0);
        app.score_press(false);
        up(&mut app);
        assert_eq!(all(&app), [(4, 4, 64)], "a quarter E on beat 2");
        // Painted to the right: a note a beat; up and down before that, the note follows.
        on(&mut app, 8, 2);
        app.score_press(false);
        on(&mut app, 8, 3);
        drag(&mut app);
        on(&mut app, 12, 4);
        drag(&mut app);
        up(&mut app);
        assert_eq!(all(&app), [(4, 4, 64), (8, 4, 69), (12, 4, 71)]);
        // A note held and let go takes the pencil's length and sharp; dragged, it moves.
        app.page_act(Hit::Dur(2));
        app.page_act(Hit::Dot);
        app.page_act(Hit::Alter(1));
        on(&mut app, 8, 3);
        app.score_press(false);
        up(&mut app);
        assert_eq!(all(&app)[1], (8, 3, 70));
        app.page_act(Hit::Alter(1));
        on(&mut app, 4, 0);
        app.score_press(false);
        on(&mut app, 0, 5);
        drag(&mut app);
        up(&mut app);
        assert_eq!(all(&app), [(0, 4, 72), (8, 3, 70), (12, 4, 71)]);
        // Erased with the right button and with the eraser; undo brings it back.
        on(&mut app, 0, 5);
        app.score_press(true);
        up(&mut app);
        assert_eq!(all(&app).len(), 2);
        crate::piano::undo(&mut app.state);
        assert_eq!(all(&app).len(), 3);
        app.page_act(Hit::Eraser);
        on(&mut app, 12, 4);
        app.score_press(false);
        up(&mut app);
        assert_eq!(all(&app).len(), 2);
        // The bass staff has its own places: its bottom line is G2.
        app.page_act(Hit::Eraser);
        app.mouse = (106.6, 195.0);
        app.score_press(false);
        up(&mut app);
        assert!(all(&app).contains(&(0, 3, 43)));
        // Another scope only shows.
        app.page_act(Hit::Scope(Scope::Song));
        let n = all(&app).len();
        on(&mut app, 4, 6);
        app.score_press(false);
        assert_eq!((all(&app).len(), app.drag.is_none()), (n, true));
    }

    #[test]
    fn a_pattern_goes_to_the_arrangement() {
        use os_model::ClipSource;
        let mut app = sheet();
        assert!(to_arrange(&mut app.state).contains("no notes"));
        on(&mut app, 0, 0);
        app.score_press(false);
        up(&mut app);
        let clips = |app: &App| crate::edit::playlist::clips(&app.state.session).iter().map(|c| (c.track, c.position, c.length)).collect::<Vec<_>>();
        to_arrange(&mut app.state);
        to_arrange(&mut app.state);
        let v = clips(&app);
        assert_eq!((v.len(), v[0].0, v[1].0, v[1].1), (2, 0, 0, v[0].2), "one after the other, on the same track");
        assert!(crate::edit::playlist::clips(&app.state.session).iter().all(|c| c.source == ClipSource::Pattern(1)));
        // The handle starts the drag that the playlist takes.
        app.page_act(Hit::Carry);
        assert!(matches!(&app.drag, Some(Drag::Browser { item: crate::views::browser::Item::Pattern(1), started: false, .. })));
    }
}
