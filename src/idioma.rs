//! Idioma de la interfaz: español (el del código) o inglés. Los textos se escriben en español y `t` da su versión en
//! inglés desde `recursos/idioma/en.json`; lo que falte en la tabla se ve en español.
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
    /// Código corto: lo que se guarda y lo que recibe Beethoven DAW.
    pub fn codigo(self) -> &'static str {
        match self {
            Idioma::Es => "es",
            Idioma::En => "en",
        }
    }
    /// Su nombre en su propio idioma.
    pub fn nombre(self) -> &'static str {
        match self {
            Idioma::Es => "Español",
            Idioma::En => "English",
        }
    }
    pub fn de_codigo(c: &str) -> Option<Idioma> {
        Idioma::TODOS.into_iter().find(|i| c.trim().eq_ignore_ascii_case(i.codigo()))
    }
}

static INGLES: AtomicBool = AtomicBool::new(false);

pub fn actual() -> Idioma {
    if INGLES.load(Relaxed) { Idioma::En } else { Idioma::Es }
}

pub fn poner(i: Idioma) {
    INGLES.store(i == Idioma::En, Relaxed);
}

fn tabla() -> &'static HashMap<String, String> {
    static T: OnceLock<HashMap<String, String>> = OnceLock::new();
    T.get_or_init(|| serde_json::from_str(include_str!("../recursos/idioma/en.json")).expect("en.json válido"))
}

/// El texto en el idioma actual.
pub fn t(s: &str) -> &str {
    if !INGLES.load(Relaxed) {
        return s;
    }
    tabla().get(s).map_or(s, |x| x.as_str())
}

/// Como `t` para una plantilla con `{}`: cada `{}` se cambia, en orden, por los valores.
pub fn tf(plantilla: &str, valores: &[&dyn std::fmt::Display]) -> String {
    let mut s = String::new();
    let mut v = valores.iter();
    let mut partes = t(plantilla).split("{}");
    if let Some(p) = partes.next() {
        s.push_str(p);
    }
    for p in partes {
        if let Some(x) = v.next() {
            s.push_str(&x.to_string());
        }
        s.push_str(p);
    }
    s
}
