// Copyright (C) 2026 mixter-ha99
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Beethoven DAW.

//! Sheet music from what is composed: the notes of a channel, a pattern or the whole song as measures of notes, chords,
//! rests and ties, on a treble or bass staff (or both), spelled in the key found; and MusicXML, which notation programs
//! open. One voice a staff: overlapping notes become chords, held until the next one starts.

use os_smf::{Kind, Smf};

/// Grid units in a quarter note (sixteenths).
pub const Q: u32 = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub start: u32,
    pub dur: u32,
    pub key: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub name: String,
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clef {
    Treble,
    Bass,
}

/// A thing in a measure; no keys = a rest. Lengths are always one note value, dotted or not.
#[derive(Debug, Clone, PartialEq)]
pub struct Elem {
    /// Units from the measure's start.
    pub at: u32,
    pub dur: u32,
    pub keys: Vec<u8>,
    /// Tied from the one before / to the next.
    pub tie_in: bool,
    pub tie_out: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Staff {
    pub clef: Clef,
    pub measures: Vec<Vec<Elem>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Part2 {
    pub name: String,
    pub staves: Vec<Staff>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    pub title: String,
    pub tempo: f64,
    pub beats: u32,
    pub beat_type: u32,
    /// Sharps (positive) or flats (negative) of the key signature.
    pub fifths: i32,
    pub minor: bool,
    pub parts: Vec<Part2>,
}

impl Score {
    /// Units in a measure.
    pub fn measure(&self) -> u32 {
        (self.beats * Q * 4 / self.beat_type.max(1)).max(1)
    }
    pub fn measures(&self) -> usize {
        self.parts.iter().flat_map(|p| &p.staves).map(|s| s.measures.len()).max().unwrap_or(0)
    }
}

/// The parts of a MIDI file, on the sixteenth grid.
pub fn parts(f: &Smf) -> Vec<Part> {
    let unit = (u32::from(f.ppq.max(4)) / Q).max(1);
    let grid = |t: u32| (t + unit / 2) / unit;
    let mut out = Vec::new();
    for t in &f.tracks {
        let mut name = String::new();
        let mut open: Vec<(u8, u8, u32)> = Vec::new();
        let mut notes = Vec::new();
        for e in &t.events {
            match &e.kind {
                Kind::TrackName(n) => name = n.clone(),
                Kind::NoteOn { channel, key, velocity } if *velocity > 0 => open.push((*channel, *key, e.tick)),
                Kind::NoteOff { channel, key, .. } | Kind::NoteOn { channel, key, .. } => {
                    if let Some(i) = open.iter().position(|o| (o.0, o.1) == (*channel, *key)) {
                        let (_, key, at) = open.remove(i);
                        let start = grid(at);
                        notes.push(Note { start, dur: grid(e.tick).saturating_sub(start).max(1), key });
                    }
                }
                _ => {}
            }
        }
        if !notes.is_empty() {
            notes.sort_by_key(|n: &Note| (n.start, n.key));
            notes.dedup_by(|a, b| a.start == b.start && a.key == b.key);
            out.push(Part { name, notes });
        }
    }
    out
}

/// The key a set of notes is most likely in: (tonic pitch class, minor, confidence 0..1). Correlates how long each
/// pitch class sounds with the profiles of major and minor keys (C. Krumhansl and E. Kessler, 1982).
pub fn find_key<'a>(notes: impl Iterator<Item = &'a Note>) -> Option<(u8, bool, f32)> {
    const MAJ: [f32; 12] = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
    const MIN: [f32; 12] = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];
    let mut h = [0f32; 12];
    for n in notes {
        h[usize::from(n.key % 12)] += n.dur as f32;
    }
    if h.iter().sum::<f32>() <= 0.0 {
        return None;
    }
    let corr = |p: &[f32; 12], r: usize| {
        let (ma, mb) = (h.iter().sum::<f32>() / 12.0, p.iter().sum::<f32>() / 12.0);
        let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
        for i in 0..12 {
            let (a, b) = (h[(i + r) % 12] - ma, p[i] - mb);
            ab += a * b;
            aa += a * a;
            bb += b * b;
        }
        ab / (aa * bb).sqrt().max(1e-6)
    };
    let mut all: Vec<(u8, bool, f32)> = (0..12).flat_map(|r| [(r as u8, false, corr(&MAJ, r)), (r as u8, true, corr(&MIN, r))]).collect();
    all.sort_by(|a, b| b.2.total_cmp(&a.2));
    Some(all[0])
}

/// Key signature of a key.
pub fn fifths(tonic: u8, minor: bool) -> i32 {
    let major = (tonic + if minor { 3 } else { 0 }) % 12;
    [0, -5, 2, -3, 4, -1, 6, 1, -4, 3, -2, 5][usize::from(major)]
}

/// How a key is written in a key signature: (step 0 = C .. 6 = B, alteration, octave: 4 holds middle C).
pub fn spell(key: u8, fifths: i32) -> (u8, i32, i32) {
    const SHARP: [(u8, i32); 12] = [(0, 0), (0, 1), (1, 0), (1, 1), (2, 0), (3, 0), (3, 1), (4, 0), (4, 1), (5, 0), (5, 1), (6, 0)];
    const FLAT: [(u8, i32); 12] = [(0, 0), (1, -1), (1, 0), (2, -1), (2, 0), (3, 0), (4, -1), (4, 0), (5, -1), (5, 0), (6, -1), (6, 0)];
    let pc = usize::from(key % 12);
    let octave = i32::from(key / 12) - 1;
    // A pitch the signature already names keeps that name (E sharp in F sharp major, C flat in G flat major).
    for step in 0..7u8 {
        let alt = signature_alt(step, fifths);
        if alt != 0 && (i32::from([0u8, 2, 4, 5, 7, 9, 11][usize::from(step)]) + alt).rem_euclid(12) == pc as i32 {
            let o = if step == 6 && alt > 0 { octave - 1 } else if step == 0 && alt < 0 { octave + 1 } else { octave };
            return (step, alt, o);
        }
    }
    let (s, a) = if fifths < 0 { FLAT[pc] } else { SHARP[pc] };
    (s, a, octave)
}

/// The key a staff place (0 = the bottom line) sounds, with the signature's alteration or `alter`.
pub fn key_at(pos: i32, clef: Clef, fifths: i32, alter: Option<i32>) -> Option<u8> {
    // Bottom lines: E4 in treble, G2 in bass.
    let d = pos + if clef == Clef::Treble { 30 } else { 18 };
    let step = d.rem_euclid(7) as u8;
    let alt = alter.unwrap_or_else(|| signature_alt(step, fifths));
    let key = [0, 2, 4, 5, 7, 9, 11][usize::from(step)] + alt + (d.div_euclid(7) + 1) * 12;
    u8::try_from(key).ok().filter(|k| *k < 128)
}

/// What the key signature does to a step.
pub fn signature_alt(step: u8, fifths: i32) -> i32 {
    const SHARPS: [u8; 7] = [3, 0, 4, 1, 5, 2, 6];
    if fifths > 0 && SHARPS[..fifths.min(7) as usize].contains(&step) {
        1
    } else if fifths < 0 && SHARPS[(7 + fifths.max(-7)) as usize..].contains(&step) {
        -1
    } else {
        0
    }
}

/// Splits `dur` units starting at `at` in a measure into written lengths, long first but never hiding a beat.
fn split(mut at: u32, mut dur: u32, measure: u32) -> Vec<u32> {
    let mut v = Vec::new();
    while dur > 0 {
        let best = [16, 12, 8, 6, 4, 3, 2, 1]
            .into_iter()
            .filter(|&d| d <= dur && at + d <= measure)
            .find(|&d| match d {
                16 | 12 => at % measure == 0 || at.is_multiple_of(4),
                8 => at.is_multiple_of(4),
                6 => at.is_multiple_of(4),
                4 => at.is_multiple_of(2),
                3 => at.is_multiple_of(4),
                2 => at.is_multiple_of(1),
                _ => true,
            })
            .unwrap_or(1);
        v.push(best);
        at += best;
        dur -= best;
    }
    v
}

fn staff(notes: &[&Note], clef: Clef, measure: u32, count: usize) -> Staff {
    // Chords by onset, held until the next onset at most.
    let mut groups: Vec<(u32, u32, Vec<u8>)> = Vec::new();
    for n in notes {
        match groups.last_mut() {
            Some(g) if g.0 == n.start => {
                g.1 = g.1.max(n.dur);
                g.2.push(n.key);
            }
            _ => groups.push((n.start, n.dur, vec![n.key])),
        }
    }
    for i in 0..groups.len() {
        if let Some(next) = groups.get(i + 1).map(|g| g.0) {
            groups[i].1 = groups[i].1.min(next - groups[i].0);
        }
    }
    let mut measures: Vec<Vec<Elem>> = vec![Vec::new(); count];
    let mut put = |start: u32, dur: u32, keys: &[u8]| {
        let (mut t, mut left) = (start, dur);
        let mut pieces = Vec::new();
        while left > 0 {
            let m = (t / measure) as usize;
            let at = t % measure;
            let here = left.min(measure - at);
            let mut a = at;
            for d in split(at, here, measure) {
                pieces.push((m, a, d));
                a += d;
            }
            t += here;
            left -= here;
        }
        let n = pieces.len();
        for (i, (m, at, d)) in pieces.into_iter().enumerate() {
            if let Some(x) = measures.get_mut(m) {
                x.push(Elem { at, dur: d, keys: keys.to_vec(), tie_in: !keys.is_empty() && i > 0, tie_out: !keys.is_empty() && i + 1 < n });
            }
        }
    };
    let mut t = 0;
    for (start, dur, keys) in &groups {
        if *start > t {
            put(t, start - t, &[]);
        }
        put(*start, *dur, keys);
        t = start + dur;
    }
    let end = count as u32 * measure;
    if t < end {
        put(t, end - t, &[]);
    }
    // A measure with nothing is one rest.
    for m in &mut measures {
        if m.iter().all(|e| e.keys.is_empty()) {
            *m = vec![Elem { at: 0, dur: measure, keys: vec![], tie_in: false, tie_out: false }];
        }
    }
    Staff { clef, measures }
}

/// The score of some parts. `key`: (tonic, minor), found when `None`.
pub fn build(title: &str, tempo: f64, beats: u32, beat_type: u32, parts: &[Part], key: Option<(u8, bool)>) -> Score {
    assemble(title, tempo, beats, beat_type, parts, key, false)
}

/// `build` to write on: both staves, and a blank measure after the last one.
pub fn sheet(title: &str, tempo: f64, beats: u32, beat_type: u32, part: Part, key: (u8, bool)) -> Score {
    assemble(title, tempo, beats, beat_type, &[part], Some(key), true)
}

fn assemble(title: &str, tempo: f64, beats: u32, beat_type: u32, parts: &[Part], key: Option<(u8, bool)>, blank: bool) -> Score {
    let found = key.or_else(|| find_key(parts.iter().flat_map(|p| &p.notes)).map(|k| (k.0, k.1))).unwrap_or((0, false));
    let mut s = Score { title: title.into(), tempo, beats: beats.max(1), beat_type: beat_type.max(1), fifths: fifths(found.0, found.1), minor: found.1, parts: vec![] };
    let m = s.measure();
    let end = parts.iter().flat_map(|p| &p.notes).map(|n| n.start + n.dur).max().unwrap_or(0);
    let count = end.div_ceil(m) as usize;
    let count = if blank { (count + 1).max(4) } else { count }.clamp(1, 2000);
    for p in parts {
        let (lo, hi) = p.notes.iter().fold((127, 0), |(a, b), n| (n.key.min(a), n.key.max(b)));
        let mean = p.notes.iter().map(|n| u32::from(n.key)).sum::<u32>() / p.notes.len().max(1) as u32;
        let all: Vec<&Note> = p.notes.iter().collect();
        let staves = if blank || (lo < 53 && hi > 67) {
            let up: Vec<&Note> = all.iter().copied().filter(|n| n.key >= 60).collect();
            let down: Vec<&Note> = all.iter().copied().filter(|n| n.key < 60).collect();
            vec![staff(&up, Clef::Treble, m, count), staff(&down, Clef::Bass, m, count)]
        } else {
            vec![staff(&all, if mean < 57 { Clef::Bass } else { Clef::Treble }, m, count)]
        };
        s.parts.push(Part2 { name: p.name.clone(), staves });
    }
    s
}

/// (note type, dots) of a length in units.
pub fn value(d: u32) -> (&'static str, u32) {
    match d {
        24 => ("whole", 1),
        16 => ("whole", 0),
        12 => ("half", 1),
        8 => ("half", 0),
        6 => ("quarter", 1),
        4 => ("quarter", 0),
        3 => ("eighth", 1),
        2 => ("eighth", 0),
        _ => ("16th", 0),
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// MusicXML 4.0 (partwise).
pub fn musicxml(s: &Score) -> String {
    use std::fmt::Write;
    let mut o = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE score-partwise PUBLIC \"-//Recordare//DTD MusicXML 4.0 Partwise//EN\" \"http://www.musicxml.org/dtds/partwise.dtd\">\n<score-partwise version=\"4.0\">\n");
    let _ = writeln!(o, " <work><work-title>{}</work-title></work>\n <identification><encoding><software>Beethoven DAW</software></encoding></identification>\n <part-list>", esc(&s.title));
    for (i, p) in s.parts.iter().enumerate() {
        let name = if p.name.is_empty() { format!("Part {}", i + 1) } else { p.name.clone() };
        let _ = writeln!(o, "  <score-part id=\"P{}\"><part-name>{}</part-name></score-part>", i + 1, esc(&name));
    }
    o.push_str(" </part-list>\n");
    for (i, p) in s.parts.iter().enumerate() {
        let _ = writeln!(o, " <part id=\"P{}\">", i + 1);
        for m in 0..s.measures() {
            let _ = writeln!(o, "  <measure number=\"{}\">", m + 1);
            if m == 0 {
                let _ = write!(o, "   <attributes><divisions>{Q}</divisions><key><fifths>{}</fifths><mode>{}</mode></key><time><beats>{}</beats><beat-type>{}</beat-type></time>", s.fifths, if s.minor { "minor" } else { "major" }, s.beats, s.beat_type);
                if p.staves.len() > 1 {
                    let _ = write!(o, "<staves>{}</staves>", p.staves.len());
                }
                for (k, st) in p.staves.iter().enumerate() {
                    let n = if p.staves.len() > 1 { format!(" number=\"{}\"", k + 1) } else { String::new() };
                    let (sign, line) = if st.clef == Clef::Treble { ("G", 2) } else { ("F", 4) };
                    let _ = write!(o, "<clef{n}><sign>{sign}</sign><line>{line}</line></clef>");
                }
                let _ = writeln!(o, "</attributes>");
                if i == 0 {
                    let _ = writeln!(o, "   <direction placement=\"above\"><direction-type><metronome><beat-unit>quarter</beat-unit><per-minute>{:.0}</per-minute></metronome></direction-type><sound tempo=\"{:.2}\"/></direction>", s.tempo, s.tempo);
                }
            }
            for (k, st) in p.staves.iter().enumerate() {
                if k > 0 {
                    let _ = writeln!(o, "   <backup><duration>{}</duration></backup>", s.measure());
                }
                let tail = if p.staves.len() > 1 { format!("<staff>{}</staff>", k + 1) } else { String::new() };
                for e in st.measures.get(m).into_iter().flatten() {
                    let (ty, dots) = value(e.dur);
                    let kind = format!("<voice>{}</voice><type>{ty}</type>{}", k + 1, "<dot/>".repeat(dots as usize));
                    if e.keys.is_empty() {
                        let whole = e.dur == s.measure();
                        let _ = writeln!(o, "   <note><rest{}/><duration>{}</duration>{}{tail}</note>", if whole { " measure=\"yes\"" } else { "" }, e.dur, if whole { format!("<voice>{}</voice>", k + 1) } else { kind });
                        continue;
                    }
                    for (j, key) in e.keys.iter().enumerate() {
                        let (step, alter, oct) = spell(*key, s.fifths);
                        let a = if alter != 0 { format!("<alter>{alter}</alter>") } else { String::new() };
                        let ties = [(e.tie_in, "stop"), (e.tie_out, "start")];
                        let tie: String = ties.iter().filter(|t| t.0).map(|t| format!("<tie type=\"{}\"/>", t.1)).collect();
                        let tied: String = ties.iter().filter(|t| t.0).map(|t| format!("<tied type=\"{}\"/>", t.1)).collect();
                        let notations = if tied.is_empty() { String::new() } else { format!("<notations>{tied}</notations>") };
                        let _ = writeln!(
                            o,
                            "   <note>{}<pitch><step>{}</step>{a}<octave>{oct}</octave></pitch><duration>{}</duration>{tie}{kind}{tail}{notations}</note>",
                            if j > 0 { "<chord/>" } else { "" },
                            ['C', 'D', 'E', 'F', 'G', 'A', 'B'][usize::from(step)],
                            e.dur
                        );
                    }
                }
            }
            o.push_str("  </measure>\n");
        }
        o.push_str(" </part>\n");
    }
    o.push_str("</score-partwise>\n");
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(start: u32, dur: u32, key: u8) -> Note {
        Note { start, dur, key }
    }
    fn part(notes: Vec<Note>) -> Vec<Part> {
        vec![Part { name: "Piano & voz".into(), notes }]
    }

    /// Every measure adds up, whatever the notes; ties join what a bar line cuts.
    #[test]
    fn measures_add_up() {
        let mut x = 7u32;
        let mut r = |m: u32| {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12345);
            (x >> 8) % m
        };
        for (beats, ty) in [(4, 4), (3, 4), (6, 8), (5, 4), (7, 8)] {
            let notes = (0..60).map(|_| n(r(200), 1 + r(30), 36 + r(50) as u8)).collect::<Vec<_>>();
            let mut notes = notes;
            notes.sort_by_key(|n| (n.start, n.key));
            notes.dedup_by(|a, b| a.start == b.start && a.key == b.key);
            let s = build("x", 120.0, beats, ty, &part(notes), None);
            for st in &s.parts[0].staves {
                for m in &st.measures {
                    let mut t = 0;
                    for e in m {
                        assert_eq!(e.at, t, "{beats}/{ty} {m:?}");
                        t += e.dur;
                    }
                    assert_eq!(t, s.measure());
                }
                let flat: Vec<&Elem> = st.measures.iter().flatten().collect();
                for w in flat.windows(2) {
                    assert_eq!(w[0].tie_out, w[1].tie_in);
                    assert!(!w[0].tie_out || w[0].keys == w[1].keys);
                }
            }
        }
        let s = build("x", 120.0, 4, 4, &part(vec![n(12, 8, 67)]), Some((0, false)));
        let m = &s.parts[0].staves[0].measures;
        assert_eq!((m[0].last().unwrap().dur, m[0].last().unwrap().tie_out), (4, true));
        assert_eq!((m[1][0].dur, m[1][0].tie_in), (4, true));
    }

    #[test]
    fn keys_and_spelling() {
        let scale = |t: u8, minor: bool| {
            let st: [u8; 7] = if minor { [0, 2, 3, 5, 7, 8, 10] } else { [0, 2, 4, 5, 7, 9, 11] };
            let mut v: Vec<Note> = st.iter().enumerate().map(|(i, k)| n(i as u32 * 4, 4, 60 + t + k)).collect();
            // The tonic chord sounds longest.
            v.extend([0, if minor { 3 } else { 4 }, 7].map(|k| n(30, 16, 48 + t + k)));
            v
        };
        for t in 0..12 {
            for minor in [false, true] {
                let k = find_key(scale(t, minor).iter()).unwrap();
                assert_eq!((k.0, k.1), (t, minor), "{t} {minor}");
            }
        }
        assert_eq!((fifths(7, false), fifths(5, false), fifths(9, true), fifths(6, false), fifths(1, false)), (1, -1, 0, 6, -5));
        // F sharp in G major, B flat in F major, middle C.
        assert_eq!((spell(66, 1), spell(70, -1), spell(60, 0)), ((3, 1, 4), (6, -1, 4), (0, 0, 4)));
        // E sharp in F sharp major is not F; C flat keeps its octave.
        assert_eq!(spell(65, 6), (2, 1, 4));
        assert_eq!(spell(59, -7), (0, -1, 4));
        for f in -7..=7 {
            for k in 21..=108u8 {
                let (s, a, o) = spell(k, f);
                assert_eq!(i32::from([0u8, 2, 4, 5, 7, 9, 11][usize::from(s)]) + a + (o + 1) * 12, i32::from(k), "{k} in {f}");
            }
        }
    }

    /// A place on the staff and the key written there are each other's.
    #[test]
    fn staff_places_and_keys() {
        use crate::views::page::staff_pos;
        for f in -7..=7 {
            for clef in [Clef::Treble, Clef::Bass] {
                for pos in -6..=14 {
                    let k = key_at(pos, clef, f, None).unwrap();
                    let (step, alt, oct) = spell(k, f);
                    assert_eq!((staff_pos(step, oct, clef), alt), (pos, signature_alt(step, f)), "{f} {clef:?} {pos}");
                }
            }
        }
        // E4 and F4 sharp in C; F is sharp in G, and a natural takes it off.
        assert_eq!((key_at(0, Clef::Treble, 0, None), key_at(1, Clef::Treble, 0, Some(1))), (Some(64), Some(66)));
        assert_eq!((key_at(1, Clef::Treble, 1, None), key_at(1, Clef::Treble, 1, Some(0))), (Some(66), Some(65)));
        assert_eq!(key_at(0, Clef::Bass, 0, None), Some(43));
    }

    #[test]
    fn a_blank_sheet_has_both_staves() {
        let s = sheet("x", 120.0, 4, 4, Part { name: String::new(), notes: vec![] }, (0, false));
        assert_eq!((s.parts[0].staves.len(), s.measures()), (2, 4));
        let s = sheet("x", 120.0, 4, 4, Part { name: String::new(), notes: vec![n(64, 16, 60)] }, (0, false));
        assert_eq!(s.measures(), 6);
    }

    #[test]
    fn xml_is_well_formed() {
        let mut v: Vec<Note> = (0..16).map(|i| n(i * 3, 3, 40 + (i * 7 % 45) as u8)).collect();
        v.push(n(0, 20, 84));
        v.sort_by_key(|n| (n.start, n.key));
        let s = build("A < B & \"C\"", 96.0, 4, 4, &part(v), None);
        assert_eq!(s.parts[0].staves.len(), 2, "wide parts get both staves");
        let x = musicxml(&s);
        assert!(x.contains("A &lt; B &amp; &quot;C&quot;") && x.contains("Piano &amp; voz") && x.contains("<staves>2</staves>"));
        // Balanced tags.
        let mut open: Vec<&str> = vec![];
        for t in x.split('<').skip(1) {
            let tag = t.split('>').next().unwrap();
            if tag.starts_with(['?', '!']) || tag.ends_with('/') {
                continue;
            }
            match tag.strip_prefix('/') {
                Some(c) => assert_eq!(open.pop(), Some(c)),
                None => open.push(tag.split_whitespace().next().unwrap()),
            }
        }
        assert!(open.is_empty());
        assert_eq!(x.matches("<measure ").count(), s.measures());
        // Durations of each measure: the staves' sums and the backups cancel.
        for m in x.split("<measure ").skip(1) {
            let d: u32 = m.split("<note>").skip(1).filter(|n| !n.starts_with("<chord/>")).filter_map(|n| n.split("<duration>").nth(1)?.split('<').next()?.parse::<u32>().ok()).sum();
            assert_eq!(d, s.measure() * 2);
        }
    }

    #[test]
    fn from_midi() {
        use os_smf::{Event, Track};
        let on = |tick, key, velocity| Event { tick, kind: Kind::NoteOn { channel: 0, key, velocity } };
        let f = Smf { format: 1, ppq: 96, tracks: vec![Track { events: vec![Event { tick: 0, kind: Kind::TrackName("Bajo".into()) }, on(0, 40, 90), on(47, 40, 0), on(49, 43, 90), on(96, 43, 0)] }, Track::default()] };
        let p = parts(&f);
        assert_eq!(p, vec![Part { name: "Bajo".into(), notes: vec![n(0, 2, 40), n(2, 2, 43)] }]);
        assert_eq!(build("t", 120.0, 4, 4, &p, None).parts[0].staves[0].clef, Clef::Bass);
    }
}
