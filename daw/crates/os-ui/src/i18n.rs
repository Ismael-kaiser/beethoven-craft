// Beethoven Craft. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: GPL-3.0-or-later

//! The interface's language. Beethoven DAW's texts are written in English; in Spanish each text is swapped for its
//! translation as it is measured and drawn (`Canvas::text`, `text_width`, `text_vertical`), from `es.tsv` (one line per
//! text: English, a tab, Spanish; `\n` for a line break). A text with a number at the end ("Track 12") is looked up as
//! "Track {}". What the table lacks stays in English.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

static SPANISH: AtomicBool = AtomicBool::new(false);

pub fn set_spanish(on: bool) {
    SPANISH.store(on, Relaxed);
}

pub fn spanish() -> bool {
    SPANISH.load(Relaxed)
}

/// The language Beethoven Craft's menu passes (`BEETHOVEN_IDIOMA`), else the system's (`LANG`, `LC_ALL`).
pub fn from_environment() {
    let code = std::env::var("BEETHOVEN_IDIOMA")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    set_spanish(code.to_ascii_lowercase().starts_with("es"));
}

fn table() -> &'static HashMap<String, String> {
    static T: OnceLock<HashMap<String, String>> = OnceLock::new();
    T.get_or_init(|| {
        include_str!("../es.tsv")
            .lines()
            .filter_map(|l| l.split_once('\t'))
            .map(|(a, b)| (a.replace("\\n", "\n"), b.replace("\\n", "\n")))
            .collect()
    })
}

/// The table's templates with one `{}`: (before it, after it, translation), longest first.
fn templates() -> &'static Vec<(String, String, String)> {
    static T: OnceLock<Vec<(String, String, String)>> = OnceLock::new();
    T.get_or_init(|| {
        let mut v: Vec<(String, String, String)> = table()
            .iter()
            .filter(|(k, _)| k.matches("{}").count() == 1 && k.len() > 2)
            .map(|(k, x)| {
                let (a, b) = k.split_once("{}").unwrap();
                (a.to_string(), b.to_string(), x.clone())
            })
            .collect();
        v.sort_by_key(|(a, b, _)| std::cmp::Reverse(a.len() + b.len()));
        v
    })
}

/// Texts already looked up that needed a template (a frame draws the same ones again and again).
fn memo() -> &'static Mutex<HashMap<String, Option<String>>> {
    static M: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    M.get_or_init(Default::default)
}

/// `s` in the interface's language.
pub fn t(s: &str) -> Cow<'_, str> {
    if !spanish() || s.is_empty() {
        return Cow::Borrowed(s);
    }
    if let Some(x) = table().get(s) {
        return Cow::Borrowed(x.as_str());
    }
    let mut m = memo().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(r) = m.get(s) {
        return r.clone().map_or(Cow::Borrowed(s), Cow::Owned);
    }
    // "Track 12" -> "Track {}", "Channel panning: 30% left" -> "Channel panning: {}".
    let r = templates()
        .iter()
        .find(|(a, b, _)| s.len() > a.len() + b.len() && s.starts_with(a.as_str()) && s.ends_with(b.as_str()))
        .map(|(a, b, x)| {
            // What fills the hole is translated too when the table has it ("Arranger - Arrangement").
            let inner = &s[a.len()..s.len() - b.len()];
            x.replace("{}", table().get(inner).map_or(inner, |y| y.as_str()))
        });
    if m.len() > 20_000 {
        m.clear();
    }
    m.insert(s.to_string(), r.clone());
    r.map_or(Cow::Borrowed(s), Cow::Owned)
}
