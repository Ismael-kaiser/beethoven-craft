//! Que nada se quede sin su inglés: revisa los textos del código y de los guiones de fábrica.
use beethoven_daw_demo::idioma::tabla;
use std::path::Path;

fn archivos(d: &str, ext: &str, v: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(d)).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            archivos(&format!("{d}/{}", e.file_name().to_string_lossy()), ext, v);
        } else if p.extension().is_some_and(|x| x == ext) {
            v.push(p);
        }
    }
}

/// Los textos entre comillas de un renglón.
fn comillas(l: &str) -> Vec<String> {
    let (mut v, mut actual, mut dentro, mut c) = (vec![], String::new(), false, l.chars());
    while let Some(x) = c.next() {
        match x {
            '\\' if dentro => {
                actual.push(x);
                actual.extend(c.next());
            }
            '"' if dentro => {
                v.push(std::mem::take(&mut actual));
                dentro = false;
            }
            '"' => dentro = true,
            '/' if !dentro && c.clone().next() == Some('/') => break,
            x if dentro => actual.push(x),
            _ => {}
        }
    }
    v
}

#[test]
fn todo_tiene_ingles() {
    let mut falta = std::collections::BTreeSet::new();
    // No se muestran, o son iguales en los dos idiomas.
    let libres = [
        "Beethoven DAW Demo",
        "Beethoven DAW Demo / FLP",
        "los guiones de fábrica sirven",
        "las imágenes de fábrica sirven",
        "los efectos de fábrica sirven",
        "{:?} {} {modelo}",
        "Bearer {clave}",
        "400 Bad Request",
        "MAL: {s}",
        "(image: this server cannot show it)",
        "## El asistente",
        "## Hacer un instrumento",
        "[What the user is looking at, filled in by the program: {v}]",
        "modelo de claude",
        "{limpio} {i}.{ext}",
        "{ruta}: {x}",
        "canal {}",
        "return 0",
        "son cuatro",
        "efecto {i} {k}",
        "notas {}",
        "fft: 2 a 4096, potencia de dos",
        "forma de la reserva",
        "local e = ... return e.izq, e.der",
        "{nombre} · {autor}",
        "Audio, MIDI, SF2, SFZ, Lua, zip",
        "propias:{}:{clave}",
        "pads:{}",
        "el sistema da azar",
        "Beethoven DAW Demo: clips",
        "Beethoven DAW Demo: audio",
        "Claude API",
        "Claude Code",
        "LM Studio",
        " (daw MCP Server)",
        "Copilot CLI",
        "Gemini CLI",
        "Kilo Code",
        "Qwen Code",
        "modelo de la ia",
        "not available",
        "OpenAI / Ollama / LM Studio",
        "method not found",
        "plugin: {}",
        "sin salida",
        "[clave, nombre, min, max, valor]",
        "Haz",
        "en.json válido",
        "Lua solo llama esto con un sonido",
        "Audio Module Class",
        "guion:",
        "Español",
        "Info.plist",
        "VST3 / CLAP",
        "Audio, SF2, SF3, SFZ",
        "sin fábrica",
        "órgano",
        "WAVEfmt ",
        "{nombre}: {}",
        "fx {fx}",
        "{i:02} {limpio}.wav",
        "{db:+.1} dB",
        "Bossa nova",
        "Corrido tumbado",
        "Drum and bass",
        "Hip hop",
        "Reggaetón",
        "runtime error: ",
        "syntax error: ",
        "msr fpcr, {}",
        "mrs {}, fpcr",
        "stmxcsr [{}]",
        "ldmxcsr [{}]",
    ];
    let mut rs = vec![];
    archivos("src", "rs", &mut rs);
    for f in rs {
        let mut s = std::fs::read_to_string(&f).unwrap();
        // Lo que se le explica a la IA va en inglés y no se muestra.
        if let (Some(a), Some(b)) = (s.find("const SISTEMA"), s.find("type R<T")) {
            s.replace_range(a..b, "");
        }
        for l in s.lines().filter(|l| {
            !l.trim_start().starts_with("//")
                && !l.contains("eprintln!")
                && !l.contains("contestar(")
                && !l.contains("        h(\"")
                && !l.contains("\"description\"")
                && !l.contains("\"Bearer ")
                && !l.contains("Bearer {}")
                && !l.contains("println!")
        }) {
            for x in comillas(l) {
                // `t("…")` y parecidos, no `set("…")`.
                let directo = ["t", "tf", "tr", "ayuda"].iter().any(|f| {
                    let q = format!("{f}(\"{x}\"");
                    l.match_indices(&q).any(|(i, _)| !l[..i].ends_with(|c: char| c.is_alphanumeric() || c == '_'))
                });
                let letras = x.chars().filter(|c| c.is_alphabetic()).count();
                let frase = letras > 3 && (x.contains(' ') || x.chars().any(|c| "áéíóúñ¿«»".contains(c)));
                let lua = l.contains("\\\"") || l.contains("= {");
                if (directo
                    || (frase
                        && !lua
                        && x.chars().filter(|c| c.is_alphabetic()).count() > 3
                        && !x.contains(":.1")
                        && !x.contains("{e}")
                        && !x.contains("{clase}")
                        && !x.contains("{tipo}")))
                    && !x.contains('\\')
                    // El nombre de cada instrumento dentro del banco (src/banco.rs) no se muestra.
                    && !l.contains(&format!("preset: \"{x}\""))
                    && !tabla().contains_key(&x)
                    && !libres.contains(&x.as_str())
                {
                    falta.insert(x);
                }
            }
        }
    }
    let mut lua = vec![];
    ["instrumentos", "efectos", "herramientas"].iter().for_each(|d| archivos(d, "lua", &mut lua));
    for f in lua {
        let s = std::fs::read_to_string(&f).unwrap();
        for l in s.lines().filter(|l| {
            !l.contains("en = \"") && !l.contains("es and") && !l.contains("idioma ==") && !l.contains("in ipairs") && !l.contains("en and")
        }) {
            let v = comillas(l);
            let de_nombre = l.contains("nombre =");
            // Los controles son { clave, nombre, … }: cuenta cada segundo texto.
            for (i, x) in v.iter().enumerate() {
                if (de_nombre && !["efecto", "herramienta"].contains(&x.as_str()) || l.contains("{ \"") && i % 2 == 1)
                    && !x.starts_with('#')
                    && x != "VU"
                    && x.chars().next().is_some_and(|c| c.is_uppercase())
                    && x.chars().any(|c| c.is_alphabetic())
                    && !tabla().contains_key(x)
                {
                    falta.insert(x.clone());
                }
            }
        }
    }
    // Los que se traducen por medio de una lista.
    let sueltos = "Ventana Biblioteca Asistente Voz Agrandar Cerrar Solo Mudo Playlist Mixer Instrumento Nuevo Patrón Canción Arreglo Ritmo Notas Mezcla Instrumentos Efectos Mayor Menor Blues efecto instrumento Otros Do Do# Re Re# Mi Fa Fa# Sol Sol# La La# Si Bajo Campana Subir Bajar";
    falta.extend(sueltos.split(' ').filter(|x| !tabla().contains_key(*x)).map(String::from));
    for (k, v) in tabla() {
        assert_eq!(k.matches("{}").count(), v.matches("{}").count(), "{k}");
    }
    assert!(falta.is_empty(), "sin inglés: {}", serde_json::to_string_pretty(&falta).unwrap());
}
