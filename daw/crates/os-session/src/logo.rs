// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Beethoven DAW, based on Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Beethoven DAW's mark: an eighth note drawn for Beethoven Craft. Absolute M/L/C/Z
//! path data in viewBox units.

pub const VIEW_W: f32 = 982.257;
pub const VIEW_H: f32 = 977.451;

pub const NOTE_HEAD: &str = "M 565 760 C 565 829 487 885 390 885 C 293 885 215 829 215 760 C 215 691 293 635 390 635 C 487 635 565 691 565 760 Z";
pub const NOTE_STEM: &str = "M 510 700 L 510 110 L 565 110 L 565 700 Z";
pub const NOTE_FLAG: &str = "M 565 110 C 600 240 770 300 770 460 C 770 530 745 580 712 622 C 735 552 738 482 703 432 C 662 372 602 342 565 330 Z";
pub const PATHS: &[&str] = &[NOTE_HEAD, NOTE_STEM, NOTE_FLAG];
