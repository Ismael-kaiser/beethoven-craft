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

#[derive(Debug, Clone)]
pub struct Page {
    pub scope: Scope,
    pub zoom: f32,
    pub scroll: f32,
    /// Height of what was painted last, for the scroll's limit.
    pub height: std::cell::Cell<f32>,
    pub key: Key,
    pub seed: u32,
    /// The last melody written, replaced by the next one.
    pub melody: Vec<os_model::Note>,
}

impl Default for Page {
    fn default() -> Self {
        Page { scope: Scope::Pattern, zoom: 1.0, scroll: 0.0, height: std::cell::Cell::new(0.0), key: Key::default(), seed: 1, melody: vec![] }
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
        // Only the selected channel: its notes, straight from the pattern.
        let unit = (u32::from(p.ppq.max(4)) / score::Q).max(1);
        let grid = |t: u32| (t + unit / 2) / unit;
        let ch = crate::piano::channel(s);
        let mut v: Vec<score::Note> = crate::edit::notes::notes(&s.session)
            .iter()
            .filter(|n| Some(n.channel) == ch)
            .map(|n| score::Note { start: grid(n.position), dur: grid(n.length.max(unit)).max(1), key: n.key.min(127) as u8 })
            .collect();
        v.sort_by_key(|n| (n.start, n.key));
        v.dedup_by(|a, b| a.start == b.start && a.key == b.key);
        parts = if v.is_empty() { vec![] } else { vec![score::Part { name: crate::edit::rack::channel_name(&s.session, s.selected_channel), notes: v }] };
    }
    let (beats, ty) = p.signature().unwrap_or((4, 4));
    let file = || s.session.path.as_deref().and_then(|p| p.file_stem()).map_or("Untitled".into(), |f| f.to_string_lossy().into_owned());
    let title = p.info.title.clone().filter(|t| !t.trim().is_empty()).unwrap_or_else(file);
    score::build(&title, s.tempo(), u32::from(beats), u32::from(ty), &parts, None)
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
            Hit::Library => s.show_browser = !s.show_browser,
        }
        self.invalidate();
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
