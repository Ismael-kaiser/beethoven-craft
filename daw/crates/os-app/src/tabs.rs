// Copyright (C) 2026 mixter-ha99
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Beethoven DAW.

//! Beethoven DAW's workspace: one thing at a time, in browser-like tabs (Arrange, Rhythm, Piano roll, Mixer, Score,
//! Ideas, Export) instead of floating windows. Each tab fills the workspace; tabs can be closed, reopened and dragged.
//! `sync` keeps the windows following the tabs whatever opened them (keys, menus, double clicks).
//!
//! Off in the unit tests, which check the windows as they were; `set` turns it on for a thread.

use std::cell::Cell;

use crate::app::{State, Win};

thread_local! {
    static ON: Cell<bool> = const { Cell::new(!cfg!(test)) };
}

pub fn on() -> bool {
    ON.with(Cell::get)
}

pub fn set(v: bool) {
    ON.with(|o| o.set(v));
}

/// Height of the strip, below the toolbar.
pub const H: f32 = 38.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Arrange,
    Rhythm,
    Piano,
    Mixer,
    Score,
    Ideas,
    Export,
}

impl Tab {
    pub const ALL: [Tab; 7] = [Tab::Arrange, Tab::Rhythm, Tab::Piano, Tab::Mixer, Tab::Score, Tab::Ideas, Tab::Export];

    pub fn name(self) -> &'static str {
        match self {
            Tab::Arrange => "Arrange",
            Tab::Rhythm => "Rhythm",
            Tab::Piano => "Notes",
            Tab::Mixer => "Mix",
            Tab::Score => "Score",
            Tab::Ideas => "Ideas",
            Tab::Export => "Export",
        }
    }

    pub fn win(self) -> Win {
        match self {
            Tab::Arrange => Win::Playlist,
            Tab::Rhythm => Win::ChannelRack,
            Tab::Piano => Win::PianoRoll,
            Tab::Mixer => Win::Mixer,
            Tab::Score | Tab::Ideas | Tab::Export => Win::Page,
        }
    }

    pub fn page(self) -> bool {
        self.win() == Win::Page
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tabs {
    pub open: Vec<Tab>,
    pub cur: Tab,
    /// The tab before this one, gone back to when this one closes.
    pub last: Tab,
    /// Set at the first `sync`: the windows the app starts with become the Arrange tab.
    pub ready: bool,
}

impl Default for Tabs {
    fn default() -> Self {
        Tabs { open: Tab::ALL.to_vec(), cur: Tab::Arrange, last: Tab::Arrange, ready: false }
    }
}

impl Tabs {
    pub fn go(&mut self, t: Tab) {
        if !self.open.contains(&t) {
            self.open.push(t);
        }
        if self.cur != t {
            self.last = self.cur;
            self.cur = t;
        }
    }

    pub fn close(&mut self, t: Tab) {
        if self.open.len() < 2 {
            return;
        }
        let i = self.open.iter().position(|&o| o == t).unwrap_or(0);
        self.open.retain(|&o| o != t);
        if self.cur == t {
            self.cur = if self.last != t && self.open.contains(&self.last) { self.last } else { self.open[i.min(self.open.len() - 1)] };
        }
    }
}

/// The windows that are a tab.
fn is_tab_win(w: Win) -> bool {
    Tab::ALL.iter().any(|t| t.win() == w)
}

/// Shows tab `t`'s window.
pub fn show(s: &mut State, t: Tab) {
    s.tabs.ready = true;
    s.tabs.go(t);
    let w = t.win();
    s.z.retain(|&o| !is_tab_win(o) || s.detached.iter().any(|d| d.0 == o));
    s.z.insert(0, w);
    fill(s);
}

/// Makes the tab follow the windows: one of them raised is its tab, the front tab's window closed goes back to the one
/// before. The front tab's window fills the workspace, under the tool windows. Whether anything moved.
pub fn sync(s: &mut State) -> bool {
    if !on() {
        return false;
    }
    let before = (s.tabs.cur, s.z.len(), s.rack_pos, s.rack_size);
    if !s.tabs.ready {
        s.tabs.ready = true;
        let t = s.tabs.cur;
        show(s, t);
    }
    let cur = s.tabs.cur.win();
    let live = |s: &State, w: Win| is_tab_win(w) && !s.is_detached(w);
    // Something else was opened or raised.
    let other = s.z.iter().rev().copied().find(|&w| live(s, w) && w != cur);
    if let Some(w) = other {
        let t = Tab::ALL.into_iter().find(|t| t.win() == w).unwrap_or(Tab::Arrange);
        // A window of a page opens the page last used.
        show(s, if t.page() && s.tabs.last.page() { s.tabs.last } else { t });
    } else if !s.z.contains(&cur) && !s.is_detached(cur) {
        let back = if s.tabs.last != s.tabs.cur && s.tabs.open.contains(&s.tabs.last) { s.tabs.last } else { Tab::Arrange };
        show(s, back);
    } else if s.is_detached(cur) {
        let back = if s.tabs.last.win() == cur { Tab::Arrange } else { s.tabs.last };
        if back.win() != cur {
            show(s, back);
        }
    }
    fill(s);
    before != (s.tabs.cur, s.z.len(), s.rack_pos, s.rack_size)
}

fn fill(s: &mut State) {
    let w = s.tabs.cur.win();
    if let Some(i) = w.slot() {
        s.wins[i].maxed = true;
        s.wins[i].mini = false;
    } else if w == Win::ChannelRack && !s.is_detached(w) {
        let r = s.work_rect();
        s.rack_pos = (r.x, r.y);
        // The graph editor hangs below the rack: it keeps its room.
        let graph = crate::views::rack::graph_rect(s).map_or(0.0, |g| (g.bottom() - crate::views::rack::rect(s).bottom()).max(0.0));
        s.rack_size = (r.w.max(crate::views::rack::min_w(s)), (r.h - crate::views::page::GROOVES_H - graph).max(crate::views::rack::MIN_H));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Action, App};

    #[test]
    fn tabs_follow_the_windows() {
        set(true);
        let mut app = App::new(None);
        sync(&mut app.state);
        assert_eq!(app.state.tabs.cur, Tab::Arrange);
        app.state.raise(Win::Mixer);
        sync(&mut app.state);
        assert_eq!(app.state.tabs.cur, Tab::Mixer);
        assert!(app.state.is_max(Win::Mixer) && !app.state.shown(Win::Playlist));
        // The same key again hides it: back to the tab before.
        app.state.hide(Win::Mixer);
        sync(&mut app.state);
        assert_eq!(app.state.tabs.cur, Tab::Arrange);
        // The rack fills the workspace too.
        show(&mut app.state, Tab::Rhythm);
        let (r, w) = (crate::views::rack::rect(&app.state), app.state.work_rect());
        assert_eq!((r.x, r.y, r.w), (w.x, w.y, w.w));
        // A tool window stays over the tab.
        app.state.raise(Win::Tapper);
        sync(&mut app.state);
        assert_eq!((app.state.tabs.cur, app.state.z.last()), (Tab::Rhythm, Some(&Win::Tapper)));
        app.act(Action::TabClose(Tab::Rhythm));
        assert!(!app.state.tabs.open.contains(&Tab::Rhythm) && app.state.tabs.cur != Tab::Rhythm);
        app.act(Action::TabNew);
        assert_eq!(app.state.tabs.cur, Tab::Rhythm);
        set(false);
    }
}
