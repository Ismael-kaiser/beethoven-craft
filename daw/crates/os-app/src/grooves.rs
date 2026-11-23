// Copyright (C) 2026 mixter-ha99
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Beethoven DAW.

//! Grooves: the rhythm of many genres, piece by piece, written as steps on the selected instrument; and Euclidean
//! rhythms (so many hits spread as evenly as they fit), which most of the world's rhythms are.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece {
    Kick,
    Snare,
    Hat,
    Perc,
}

impl Piece {
    pub const ALL: [(Piece, &'static str); 4] = [(Piece::Kick, "Kick"), (Piece::Snare, "Snare"), (Piece::Hat, "Hi-hat"), (Piece::Perc, "Percussion")];
}

/// Name and the sixteen steps of kick, snare, hi-hat and percussion.
pub const GROOVES: [(&str, [&str; 4]); 16] = [
    ("Rock", ["x-------x-x-----", "----x-------x---", "x-x-x-x-x-x-x-x-", "----------------"]),
    ("Pop", ["x-----x-x-------", "----x-------x---", "x-x-x-x-x-x-x-xx", "----x-------x--x"]),
    ("Cumbia", ["x---x---x---x---", "------x-------x-", "x-xxx-xxx-xxx-xx", "x--x--x-x--x--x-"]),
    ("Reggaeton", ["x---x---x---x---", "---x--x----x--x-", "x-x-x-x-x-x-x-x-", "------------x-x-"]),
    ("Salsa", ["------x-----x---", "x--x--x---x-x---", "x--x-xx-x--x-xx-", "x-x-xx-x-x-xx-x-"]),
    ("Bolero", ["x-------x-------", "------x-x-----x-", "x-xxx-x-x-x-x-x-", "x--x--x-----x---"]),
    ("Banda", ["x-------x-------", "----x-------x---", "--x---x---x---x-", "x--xx--xx--xx--x"]),
    ("Corrido tumbado", ["x-----x---x-----", "----x--x----x---", "x--x--x-x--x--x-", "--x---x---x---x-"]),
    ("Trap", ["x------x--x-----", "--------x-------", "x-x-xxx-x-x-x-xx", "------------x---"]),
    ("Hip hop", ["x--x---x--x-----", "----x-------x---", "x-x-x-x-x-x-x-x-", "-------x--------"]),
    ("House", ["x---x---x---x---", "----x-------x---", "--x---x---x---x-", "---x--x----x--x-"]),
    ("Drum and bass", ["x---------x-----", "----x--x-x--x---", "x-x-x-x-x-x-x-x-", "--x---x---x---x-"]),
    ("Funk", ["x-x----x--x--x--", "----x--x-x--x---", "xxxxxxxxxxxxxxxx", "--x--x----x--x--"]),
    ("Bossa nova", ["x--xx--xx--xx--x", "x--x--x---x--x--", "x-x-x-x-x-x-x-x-", "--x---x---x---x-"]),
    ("Samba", ["x--xx--xx--xx--x", "--x-xx-x--x-xx-x", "x-xxx-xxx-xxx-xx", "x-x-xx-x-x-xx-x-"]),
    ("Afrobeat", ["x--x--x---x-x---", "----x--x----x--x", "x-xxx-x-x-xxx-x-", "x--x--x---x--x--"]),
];

/// `hits` spread as evenly as they fit over `steps`, turned by `rotate`.
pub fn euclid(hits: u32, steps: u32, rotate: u32) -> Vec<bool> {
    let steps = steps.max(1);
    let hits = hits.min(steps);
    let v: Vec<bool> = (0..steps).map(|i| (i * hits) % steps < hits).collect();
    (0..steps).map(|i| v[((i + rotate) % steps) as usize]).collect()
}

pub fn steps(groove: usize, piece: Piece) -> Vec<bool> {
    let i = Piece::ALL.iter().position(|p| p.0 == piece).unwrap_or(0);
    GROOVES[groove % GROOVES.len()].1[i].bytes().map(|b| b == b'x').collect()
}

/// Writes `on` on row `r`, repeated to the pattern's length.
pub fn write(s: &mut crate::app::State, r: usize, on: &[bool]) {
    let len = crate::rack::pattern_steps(&s.session).max(on.len() as u32);
    for k in 0..len {
        crate::rack::set_step(s, r, k, on[k as usize % on.len()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grooves_are_whole() {
        for (n, p) in GROOVES {
            for l in p {
                assert!(l.len() == 16 && l.bytes().all(|b| b == b'x' || b == b'-'), "{n}");
            }
        }
    }

    #[test]
    fn euclid_known() {
        let t = |h, n, r| euclid(h, n, r).iter().map(|b| if *b { 'x' } else { '-' }).collect::<String>();
        assert_eq!(t(3, 8, 0), "x--x--x-", "tresillo");
        assert_eq!(t(4, 16, 0), "x---x---x---x---");
        assert_eq!(t(0, 5, 0), "-----");
        assert_eq!(t(9, 4, 0), "xxxx");
        for h in 0..=16 {
            for r in 0..16 {
                assert_eq!(euclid(h, 16, r).iter().filter(|b| **b).count(), h as usize);
            }
        }
    }
}
