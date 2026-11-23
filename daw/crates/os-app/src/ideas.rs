// Copyright (C) 2026 mixter-ha99
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Beethoven DAW.

//! Ideas: help when the page is blank. Chords of the key with what usually follows, progressions by mood, and
//! accompaniments and melodies made from the chords already in the pattern. Everything is written as ordinary notes in
//! the current pattern on the selected channel, one undo step each.

use os_model::Note;

use crate::app::State;

pub const NAMES: [&str; 12] = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    pub tonic: u8,
    pub minor: bool,
    /// How sure the detection was; `None` when chosen by hand.
    pub sure: Option<f32>,
}

impl Default for Key {
    fn default() -> Self {
        Key { tonic: 0, minor: false, sure: None }
    }
}

impl Key {
    pub fn name(&self) -> String {
        format!("{}{}", NAMES[usize::from(self.tonic % 12)], if self.minor { "m" } else { "" })
    }
    pub fn scale(&self) -> [u8; 7] {
        if self.minor { [0, 2, 3, 5, 7, 8, 10] } else { [0, 2, 4, 5, 7, 9, 11] }
    }
    /// The chords on its degrees, as the chord progression tool reads them.
    pub fn degrees(&self) -> [&'static str; 7] {
        if self.minor { ["i", "iio", "III", "iv", "v", "VI", "VII"] } else { ["I", "ii", "iii", "IV", "V", "vi", "viio"] }
    }
    /// Chords from outside that work well.
    pub fn colours(&self) -> [&'static str; 5] {
        if self.minor { ["V7", "bII", "IV", "i7", "iv7"] } else { ["V7", "iv", "bVII", "Imaj7", "ii7"] }
    }
    /// What degree `d` (0..7) tends to go to, with how often (percent).
    pub fn next(&self, d: usize) -> &'static [(usize, u32)] {
        // Common-practice tendencies, rounded.
        const MAJOR: [&[(usize, u32)]; 7] = [
            &[(3, 34), (4, 30), (5, 20), (1, 12)],
            &[(4, 62), (6, 12), (3, 10)],
            &[(5, 46), (3, 34), (1, 10)],
            &[(4, 40), (0, 30), (1, 16)],
            &[(0, 60), (5, 22), (3, 12)],
            &[(3, 36), (1, 28), (4, 24)],
            &[(0, 70), (5, 16)],
        ];
        const MINOR: [&[(usize, u32)]; 7] = [
            &[(3, 28), (5, 28), (6, 24), (4, 14)],
            &[(4, 66), (6, 12)],
            &[(5, 40), (6, 26), (3, 20)],
            &[(4, 38), (0, 26), (6, 20)],
            &[(0, 64), (5, 22)],
            &[(6, 36), (2, 26), (3, 18), (4, 14)],
            &[(0, 36), (2, 34), (5, 18)],
        ];
        if self.minor { MINOR[d % 7] } else { MAJOR[d % 7] }
    }
}

/// Progressions by mood; degrees for major and for minor.
pub const PROGRESSIONS: [(&str, &str, &str); 12] = [
    ("Luminous", "I V vi IV", "VI VII i i"),
    ("Nostalgic", "vi IV I V", "i VI III VII"),
    ("Heroic", "I IV V I", "i iv V7 i"),
    ("Ballad", "I vi IV V", "i VI iv V7"),
    ("Dreamy", "Imaj7 iii IV iv", "i7 III VI iv7"),
    ("Tension", "ii7 V7 vi IV", "iv7 V7 VI V7"),
    ("Jazz turn", "Imaj7 vi ii7 V7", "i7 VI iio V7"),
    ("Ranchera", "I I V7 I", "i i V7 i"),
    ("Bolero", "I vi ii V7", "i iv V7 i"),
    ("Cumbia", "I IV V7 V7", "i iv V7 i"),
    ("Epic", "vi vi IV V", "i VI VII VII"),
    ("Farewell", "IV V iii vi", "VI VII v i"),
];

/// A small generator, so "Another" always gives something new but the same seed the same thing.
struct Rng(u32);
impl Rng {
    fn below(&mut self, n: u32) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 % n.max(1)
    }
}

/// The chords of `ch` in the pattern, by bar: (bar start, keys low to high). A chord is what sounds at the bar's start.
pub fn chords(notes: &[Note], ch: u16, bar: u32) -> Vec<(u32, Vec<u16>)> {
    let end = notes.iter().filter(|n| n.channel == ch).map(|n| n.position + n.length.max(1)).max().unwrap_or(0);
    (0..end.div_ceil(bar))
        .filter_map(|b| {
            let t = b * bar;
            let mut k: Vec<u16> = notes.iter().filter(|n| n.channel == ch && n.position <= t + bar / 16 && n.position + n.length > t + bar / 8).map(|n| n.key).collect();
            k.sort_unstable();
            k.dedup();
            (k.len() >= 2).then_some((t, k))
        })
        .collect()
}

/// The degree of the key a chord is on, by its lowest note.
pub fn degree(key: &Key, chord: &[u16]) -> Option<usize> {
    let root = (u32::from(*chord.first()?) + 12 - u32::from(key.tonic)) % 12;
    // The bass may be an octave below the root; the fifth above it tells.
    key.scale().iter().position(|s| u32::from(*s) == root)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Make {
    Bass,
    Arpeggio,
    Broken,
    Strum,
    Melody,
    Harmony,
}

impl Make {
    pub const ALL: [(Make, &'static str, &'static str); 6] = [
        (Make::Bass, "Bass line", "A bass from the chords in the pattern, an octave below"),
        (Make::Arpeggio, "Arpeggio", "The chords note by note, going up and down"),
        (Make::Broken, "Broken chords", "Low, high, middle, high: the classical accompaniment"),
        (Make::Strum, "Rhythm", "The chords repeated with a syncopated rhythm"),
        (Make::Melody, "Melody", "A singable melody over the chords; press again for another"),
        (Make::Harmony, "Second voice", "A third above every note of the channel, within the key"),
    ];
    /// The undo step's name.
    pub fn step(self) -> &'static str {
        match self {
            Make::Bass => "Bass line",
            Make::Arpeggio => "Arpeggio",
            Make::Broken => "Broken chords",
            Make::Strum => "Rhythm",
            Make::Melody => "Melody",
            Make::Harmony => "Second voice",
        }
    }
}

/// The notes `what` makes of `notes` of channel `ch`; `seed` varies the melody.
pub fn make(what: Make, notes: &[Note], ch: u16, bar: u32, key: &Key, seed: u32) -> Vec<Note> {
    let note = |at: u32, len: u32, k: i32| Note { key: k.clamp(0, 131) as u16, length: len.max(1), ..Note::step(ch, at) };
    let cs = chords(notes, ch, bar);
    let mut v = vec![];
    let beat = (bar / 4).max(1);
    match what {
        Make::Harmony => {
            let sc = key.scale();
            for n in notes.iter().filter(|n| n.channel == ch) {
                let rel = (i32::from(n.key) - i32::from(key.tonic)).rem_euclid(12);
                // Two steps of the scale up from the nearest degree at or below.
                let d = sc.iter().rposition(|s| i32::from(*s) <= rel).unwrap_or(0);
                let up = i32::from(sc[(d + 2) % 7]) + if d + 2 >= 7 { 12 } else { 0 } - i32::from(sc[d]);
                let k = i32::from(n.key) + up;
                if !notes.iter().any(|o| o.channel == ch && o.position == n.position && i32::from(o.key) == k) {
                    v.push(Note { key: k.clamp(0, 131) as u16, id: Default::default(), ..*n });
                }
            }
        }
        Make::Bass => {
            for (t, c) in &cs {
                let r = i32::from(c[0]);
                let low = r - 12 * ((r - 36) / 12).max(1).min(r / 12);
                let low = if low < 28 { low + 12 } else { low };
                v.push(note(*t, beat * 3 / 2, low));
                v.push(note(t + beat * 3 / 2, beat / 2, low + 12));
                v.push(note(t + beat * 2, beat, low + 7));
                v.push(note(t + beat * 3, beat, low));
            }
        }
        Make::Arpeggio => {
            for (t, c) in &cs {
                let up: Vec<i32> = c.iter().map(|k| i32::from(*k) + 12).chain([i32::from(c[0]) + 24]).collect();
                let wave: Vec<i32> = up.iter().chain(up.iter().rev().skip(1).take(up.len().saturating_sub(2))).copied().collect();
                for i in 0..8 {
                    v.push(note(t + i * bar / 8, bar / 8, wave[i as usize % wave.len()]));
                }
            }
        }
        Make::Broken => {
            for (t, c) in &cs {
                let (lo, hi, mid) = (i32::from(c[0]), i32::from(c[c.len() - 1]), i32::from(c[c.len() / 2]));
                for i in 0..8 {
                    v.push(note(t + i * bar / 8, bar / 8, [lo, hi, mid, hi][i as usize % 4] + 12));
                }
            }
        }
        Make::Strum => {
            for (t, c) in &cs {
                // Sixteenths hit: 1, the "and" of 2, 3 and the last eighth.
                for (at, len) in [(0, 3), (3, 3), (6, 4), (10, 2), (12, 3), (15, 1)] {
                    for k in c {
                        v.push(note(t + at * bar / 16, (len * bar / 16).max(1), i32::from(*k) + 12));
                    }
                }
            }
        }
        Make::Melody => {
            let mut r = Rng(seed.wrapping_mul(2_654_435_761) | 1);
            // A rhythm for two bars, kept through the phrase so it can be remembered.
            const RHYTHMS: [&[u32]; 6] = [&[4, 4, 2, 2, 4], &[6, 2, 4, 4], &[2, 2, 4, 2, 2, 4], &[3, 3, 2, 4, 4], &[4, 2, 2, 8], &[2, 4, 2, 4, 4]];
            let rhy = [RHYTHMS[r.below(6) as usize], RHYTHMS[r.below(6) as usize]];
            let sc = key.scale();
            let inside = |k: i32| sc.contains(&((k - i32::from(key.tonic)).rem_euclid(12) as u8));
            let mut cur = 72 + i32::from(key.tonic) - if key.tonic > 6 { 12 } else { 0 };
            let total = cs.len();
            for (i, (t, c)) in cs.iter().enumerate() {
                let tones: Vec<i32> = c.iter().map(|k| i32::from(*k) % 12).collect();
                let mut at = 0;
                for (j, len) in rhy[i % 2].iter().enumerate() {
                    let strong = j == 0 || at % 8 == 0;
                    let last = i + 1 == total && at + len >= 16;
                    // Strong beats land on the chord; between them it steps through the scale.
                    let mut best = (i32::MAX, cur);
                    for k in cur - 7..=cur + 7 {
                        let fits = if last { (k - i32::from(key.tonic)).rem_euclid(12) == 0 } else if strong { tones.contains(&k.rem_euclid(12)) } else { inside(k) };
                        if !fits || !(57..=84).contains(&k) {
                            continue;
                        }
                        let jump = (k - cur).abs();
                        let cost = jump * 3 + if jump == 0 { 5 } else { 0 } + r.below(9) as i32 + ((k - 70).abs() / 4);
                        if cost < best.0 {
                            best = (cost, k);
                        }
                    }
                    cur = best.1;
                    v.push(note(t + at * bar / 16, (len * bar / 16).max(1), cur));
                    at += len;
                }
            }
        }
    }
    v
}

/// Finds the key of the current pattern's notes (all channels).
pub fn detect(s: &State) -> Option<Key> {
    let v: Vec<crate::score::Note> = crate::edit::notes::notes(&s.session)
        .iter()
        .filter(|n| n.length > 0)
        .map(|n| crate::score::Note { start: n.position, dur: n.length, key: n.key.min(127) as u8 })
        .collect();
    crate::score::find_key(v.iter()).map(|(tonic, minor, sure)| Key { tonic, minor, sure: Some(sure) })
}

/// Writes what `what` makes into the pattern. The message to show.
pub fn run(s: &mut State, what: Make) -> String {
    let Some(ch) = crate::piano::channel(s) else { return "Ideas: select an instrument first".into() };
    let bar = s.session.project.bar_ticks();
    let notes = crate::edit::notes::notes(&s.session).to_vec();
    s.page.seed = s.page.seed.wrapping_add(1);
    let new = make(what, &notes, ch, bar, &s.page.key, s.page.seed);
    if new.is_empty() {
        return "Ideas: write some chords in the pattern first".into();
    }
    crate::edit::notes::snapshot_as(&mut s.session, what.step());
    let p = crate::edit::notes::pattern_mut(&mut s.session);
    // Another melody takes the place of the last one made.
    if what == Make::Melody {
        let old = std::mem::take(&mut s.page.melody);
        p.notes.retain(|n| !old.iter().any(|o| (o.channel, o.position, o.key, o.length) == (n.channel, n.position, n.key, n.length)));
        s.page.melody = new.clone();
    }
    let first = p.notes.len();
    p.notes.extend(new.iter().copied());
    let last = p.notes.len();
    s.piano.select(s.pattern(), ch, (first..last).collect());
    crate::piano::finish_edit(s);
    format!("{}: {} notes written", what.step(), last - first)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(key: &Key, text: &str, bar: u32) -> Vec<Note> {
        let c = crate::chord_tool::parse(&format!("{}: {text}", key.name())).unwrap();
        crate::chord_tool::notes(&c, 2, 0, bar)
    }

    /// Every progression and every chord offered reads in every key.
    #[test]
    fn everything_offered_is_readable() {
        for tonic in 0..12 {
            for minor in [false, true] {
                let k = Key { tonic, minor, sure: None };
                for d in k.degrees().iter().chain(&k.colours()) {
                    assert!(crate::chord_tool::parse(&format!("{}: {d}", k.name())).is_ok(), "{} {d}", k.name());
                }
                for (n, maj, min) in PROGRESSIONS {
                    let t = if minor { min } else { maj };
                    let c = crate::chord_tool::parse(&format!("{}: {t}", k.name())).unwrap();
                    assert!(c.len() == 4 && c.iter().all(Option::is_some), "{n} in {}", k.name());
                }
                for d in 0..7 {
                    assert!(k.next(d).iter().map(|x| x.1).sum::<u32>() <= 100);
                }
            }
        }
    }

    #[test]
    fn accompaniments_stay_in_the_bars_and_the_key() {
        let bar = 384;
        for (tonic, minor) in [(0, false), (9, true), (3, false), (6, true)] {
            let k = Key { tonic, minor, sure: None };
            let src = block(&k, if minor { "i VI III VII" } else { "I V vi IV" }, bar);
            assert_eq!(chords(&src, 2, bar).len(), 4);
            assert_eq!(degree(&k, &chords(&src, 2, bar)[0].1), Some(0));
            for (m, n, _) in Make::ALL {
                let v = make(m, &src, 2, bar, &k, 5);
                assert!(!v.is_empty(), "{n}");
                for x in &v {
                    assert!(x.channel == 2 && x.length > 0 && x.position + x.length <= 4 * bar, "{n} {x:?}");
                    let pc = (i32::from(x.key) - i32::from(tonic)).rem_euclid(12) as u8;
                    assert!(k.scale().contains(&pc), "{n}: {} is not in {}", x.key, k.name());
                }
            }
            // A melody is one voice, without big leaps, ends on the tonic and changes with the seed.
            let m = make(Make::Melody, &src, 2, bar, &k, 5);
            assert!(m.windows(2).all(|w| w[0].position + w[0].length <= w[1].position && w[0].key.abs_diff(w[1].key) <= 7));
            assert_eq!((i32::from(m.last().unwrap().key) - i32::from(tonic)).rem_euclid(12), 0);
            assert_ne!(m, make(Make::Melody, &src, 2, bar, &k, 6));
            assert_eq!(m, make(Make::Melody, &src, 2, bar, &k, 5));
        }
        assert!(make(Make::Bass, &[], 2, bar, &Key::default(), 1).is_empty());
    }
}
