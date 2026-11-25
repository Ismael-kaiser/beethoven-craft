// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Español (el del código) o inglés. `t` da la versión en inglés desde `recursos/en.json`.
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Idioma {
    Es,
    En,
}

impl Idioma {
    pub const TODOS: [Idioma; 2] = [Idioma::Es, Idioma::En];
    pub fn codigo(self) -> &'static str {
        match self {
            Idioma::Es => "es",
            Idioma::En => "en",
        }
    }
    pub fn nombre(self) -> &'static str {
        match self {
            Idioma::Es => "Español",
            Idioma::En => "English",
        }
    }
    /// De un código como `es`, `es-MX` o `en_US.UTF-8`.
    pub fn de_codigo(c: &str) -> Option<Idioma> {
        Idioma::TODOS.into_iter().find(|i| c.trim().to_lowercase().split(['-', '_', '.']).next() == Some(i.codigo()))
    }
    /// El del sistema; en cualquier otro idioma, inglés.
    pub fn del_sistema() -> Idioma {
        sys_locale::get_locale().map_or(Idioma::Es, |l| Idioma::de_codigo(&l).unwrap_or(Idioma::En))
    }
}

static INGLES: AtomicBool = AtomicBool::new(false);

pub fn actual() -> Idioma {
    if INGLES.load(Relaxed) { Idioma::En } else { Idioma::Es }
}

pub fn poner(i: Idioma) {
    INGLES.store(i == Idioma::En, Relaxed);
}

pub fn tabla() -> &'static HashMap<String, String> {
    static T: OnceLock<HashMap<String, String>> = OnceLock::new();
    T.get_or_init(|| serde_json::from_str(include_str!("../recursos/en.json")).expect("en.json válido"))
}

pub fn t(s: &str) -> &str {
    if !INGLES.load(Relaxed) {
        return s;
    }
    tabla().get(s).map_or(s, |x| x.as_str())
}

/// Como `t`, y cada `{}` se cambia en orden por los valores.
pub fn tf(plantilla: &str, valores: &[&dyn std::fmt::Display]) -> String {
    let mut v = valores.iter();
    let mut s = String::new();
    for (i, p) in t(plantilla).split("{}").enumerate() {
        if let Some(x) = (i > 0).then(|| v.next()).flatten() {
            s.push_str(&x.to_string());
        }
        s.push_str(p);
    }
    s
}

/// Un texto que viene de un guion: puede traer su propia traducción.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Texto {
    pub es: String,
    pub en: Option<String>,
}

impl Texto {
    pub fn ver(&self) -> &str {
        match &self.en {
            Some(e) if actual() == Idioma::En => e,
            _ => t(&self.es),
        }
    }
}
