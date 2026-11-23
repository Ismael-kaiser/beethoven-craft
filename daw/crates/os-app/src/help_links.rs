// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Beethoven DAW, based on Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! The pages Beethoven DAW opens in the default web browser (Beethoven DAW's design): HELP's entries, the toolbar's Help
//! button, the About page's buttons and a plugin's Help. They go to the website's wiki, with what the website leaves
//! out on GitHub: the releases (the site has no changelog of its own and links to them too), bug reports and the
//! source code. Diagnostics writes a report instead (`crate::diagnostics`).

pub const WEBSITE_URL: &str = "";
pub const HELP_INDEX_URL: &str = "";
pub const SHORTCUTS_URL: &str = "";
pub const TROUBLESHOOTING_URL: &str = "";
pub const WHATS_NEW_URL: &str = "";
pub const REPORT_BUG_URL: &str = "";
pub const SOURCE_CODE_URL: &str = "";

/// HELP ▸ Tutorials ▸, as (entry, page): the wiki's pages for starting out, in the order it gives them.
pub const TUTORIALS: [(&str, &str); 5] = [
    ("Start here", ""),
    ("Set up your audio", ""),
    ("A tour of the window", ""),
    ("Your first song", ""),
    ("Command-line rendering", ""),
];

/// The wiki's overview of Beethoven DAW's own plugins.
pub const PLUGINS_URL: &str = "";

/// The wiki's page on each of Beethoven DAW's own plugins that can sit in a mixer slot, as (plugin, page under
/// `PLUGINS_URL`).
const PLUGIN_PAGES: [(&str, &str); 29] = [
    ("Beethoven Sampler", "instruments/beethoven-sampler"),
    ("Beethoven Trio", "instruments/beethoven-trio"),
    ("Citrus", "instruments/citrus"),
    ("Smith", "instruments/smith"),
    ("Beethoven Slicer", "instruments/beethoven-slicer"),
    ("Beethoven Drum Sampler", "instruments/beethoven-drum-sampler"),
    ("Beethoven SoundFont Player", "instruments/beethoven-soundfont-player"),
    ("Beethoven Parametric EQ", "effects/beethoven-parametric-eq"),
    ("Pulp Filter", "effects/pulp-filter"),
    ("Press", "effects/press"),
    ("Beethoven Limiter", "effects/beethoven-limiter"),
    ("Beethoven Reverb", "effects/beethoven-reverb"),
    ("Beethoven Delay", "effects/beethoven-delay"),
    ("Beethoven Soft Clipper", "effects/beethoven-soft-clipper"),
    ("Beethoven Fast Distortion", "effects/beethoven-fast-distortion"),
    ("Beethoven Balance", "effects/beethoven-balance"),
    ("Beethoven Stereo Enhancer", "effects/beethoven-stereo-enhancer"),
    ("Beethoven Transient Processor", "effects/beethoven-transient-processor"),
    ("Beethoven Chorus", "effects/beethoven-chorus"),
    ("Beethoven Stereo Shaper", "effects/beethoven-stereo-shaper"),
    ("Beethoven Filter", "effects/beethoven-filter"),
    ("Beethoven Flanger", "effects/beethoven-flanger"),
    ("Beethoven Phaser", "effects/beethoven-phaser"),
    ("Beethoven Vocoder", "effects/beethoven-vocoder"),
    ("Beethoven Drop", "effects/beethoven-drop"),
    ("Beethoven Peak Controller", "controllers/beethoven-peak-controller"),
    ("Beethoven Formula Controller", "controllers/beethoven-formula-controller"),
    ("Tesla", "tools/tesla"),
    ("Beethoven Container", "tools/beethoven-container"),
];

/// A plugin menu's Help for Beethoven DAW's own plugin `name`: its page in the wiki, or the wiki's overview of them if
/// it has none.
pub fn plugin_page(name: &str) -> String {
    // Beethoven Craft: no online help yet.
    if PLUGINS_URL.is_empty() {
        return String::new();
    }
    match PLUGIN_PAGES.iter().find(|&&(plugin, _)| plugin == name) {
        Some((_, page)) => format!("{PLUGINS_URL}{page}/"),
        None => PLUGINS_URL.to_owned(),
    }
}

/// What HELP's entries say while there is no online help (Beethoven Craft).
pub const NO_HELP: &str = "Online help isn't available yet";

/// HELP ▸ Tutorials ▸'s entries.
pub fn tutorial_items() -> Vec<crate::playlist::PItem> {
    TUTORIALS.iter().map(|&(label, url)| crate::playlist::PItem::new(label, crate::playlist::Cmd::Web(url))).collect()
}

/// Opens `url` in the default web browser (tests: nothing).
pub fn open(url: &str) {
    if cfg!(test) {
        return;
    }
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(url).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

impl crate::app::App {
    /// Opens `url` in the default web browser, the hint bar saying so.
    pub fn open_web(&mut self, url: &str) {
        // Beethoven Craft: there is no online help yet.
        if url.is_empty() {
            self.state.message = Some(NO_HELP.into());
            return;
        }
        open(url);
        self.state.message = Some(format!("Opening {url}"));
    }
}

#[cfg(test)]
mod tests {
    use crate::views::menus::{item_at, works, HELP_ABOUT, HELP_MENU};

    /// Beethoven Craft: HELP's online entries say there is no online help yet.
    #[test]
    fn help_entries_say_there_is_no_online_help() {
        let mut app = crate::app::App::new(None);
        let at = |label: &str| (0..20).find(|&i| item_at(HELP_MENU, i).is_some_and(|it| it.label == label)).unwrap();
        for label in ["Help index", "Keyboard shortcuts", "Troubleshooting", "Report a bug", "What's new"] {
            assert!(works(HELP_MENU, item_at(HELP_MENU, at(label)).unwrap()));
            app.menu_item(HELP_MENU, at(label));
            assert_eq!(app.state.message.as_deref(), Some(super::NO_HELP));
        }
        assert_eq!(super::plugin_page("Beethoven Balance"), "");
    }

    /// About... opens the settings window on its About page, however it's reached.
    #[test]
    fn about_opens_the_about_page() {
        use crate::views::settings::{ABOUT, TABS};
        let mut app = crate::app::App::new(None);
        assert_eq!(TABS[ABOUT].0, "About");
        assert!(works(HELP_MENU, item_at(HELP_MENU, HELP_ABOUT).unwrap()));
        app.menu_item(HELP_MENU, HELP_ABOUT);
        assert!(app.state.settings.open);
        assert_eq!(app.state.settings.tab, ABOUT);
    }
}
