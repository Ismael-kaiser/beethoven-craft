//! Atajos de teclado de Beethoven Movie: la tabla de acciones, las teclas de cada una (las de fábrica o las que el
//! usuario puso) y su texto. Se guardan junto al idioma, en `Ajustes.json`.
use eframe::egui::{Event, Key, KeyboardShortcut, ModifierNames, Modifiers};
use std::collections::BTreeMap;
use std::sync::{LazyLock, RwLock};

pub struct Accion {
    pub id: &'static str,
    pub grupo: &'static str,
    pub nombre: &'static str,
    /// Teclas de fábrica, separadas por espacios.
    pub fabrica: &'static str,
}

const fn a(id: &'static str, grupo: &'static str, nombre: &'static str, fabrica: &'static str) -> Accion {
    Accion { id, grupo, nombre, fabrica }
}

pub const ACCIONES: &[Accion] = &[
    a("guardar", "Proyecto", "Guardar", "Cmd+S"),
    a("guardar_como", "Proyecto", "Guardar como…", "Cmd+Shift+S"),
    a("abrir", "Proyecto", "Abrir proyecto…", "Cmd+O"),
    a("importar", "Proyecto", "Importar video o audio…", "Cmd+I"),
    a("exportar", "Proyecto", "Exportar video con mi audio…", "Cmd+M"),
    a("menu", "Proyecto", "Menú principal", ""),
    a("atajos", "Proyecto", "Atajos de teclado…", "Cmd+Alt+K"),
    a("tocar", "Reproducción", "Reproducir / pausa", "Space"),
    a("pausa", "Reproducción", "Pausa", "K"),
    a("seguir", "Reproducción", "Reproducir", "L"),
    a("regresar5", "Reproducción", "Regresar 5 segundos", "J"),
    a("detener", "Reproducción", "Detener y volver al inicio", "Shift+Space"),
    a("grabar", "Reproducción", "Grabar", "R"),
    a("cuadro_atras", "Reproducción", "Un cuadro atrás", "Left"),
    a("cuadro_adelante", "Reproducción", "Un cuadro adelante", "Right"),
    a("cinco_atras", "Reproducción", "Cinco cuadros atrás", "Shift+Left"),
    a("cinco_adelante", "Reproducción", "Cinco cuadros adelante", "Shift+Right"),
    a("corte_antes", "Reproducción", "Corte anterior", "Up"),
    a("corte_despues", "Reproducción", "Corte siguiente", "Down"),
    a("inicio", "Reproducción", "Ir al inicio", "Home"),
    a("fin", "Reproducción", "Ir al final", "End"),
    a("deshacer", "Edición", "Deshacer", "Cmd+Z"),
    a("rehacer", "Edición", "Rehacer", "Cmd+Shift+Z"),
    a("dividir", "Edición", "Dividir en el cursor", "S Cmd+K"),
    a("borrar", "Edición", "Borrar clip", "Backspace Delete"),
    a("borrar_cerrar", "Edición", "Borrar y cerrar hueco", "Shift+Backspace Shift+Delete"),
    a("todo", "Edición", "Elegir todo", "Cmd+A"),
    a("elegir", "Edición", "Elegir el clip del cursor", "D"),
    a("recortar_izq", "Edición", "Recortar el principio al cursor", "Q"),
    a("recortar_der", "Edición", "Recortar el final al cursor", "W"),
    a("transicion", "Edición", "Transición rápida", "Cmd+D"),
    a("marca", "Edición", "Poner marca", "M"),
    a("entrada", "Edición", "Principio del tramo", "I"),
    a("salida", "Edición", "Final del tramo", "O"),
    a("sin_tramo", "Edición", "Quitar el tramo", "Alt+X"),
    a("levantar", "Edición", "Levantar el tramo", "Semicolon"),
    a("extraer", "Edición", "Extraer el tramo", "Quote"),
    a("vacio", "Edición", "Clip vacío (efectos para todo)", "Cmd+Shift+A"),
    a("titulo", "Edición", "Poner un texto", "Cmd+T"),
    a("h0", "Herramientas", "Selección", "V"),
    a("h1", "Herramientas", "Selección hacia adelante", "A"),
    a("h2", "Herramientas", "Recorte ondulado", "B"),
    a("h3", "Herramientas", "Rodar", "N"),
    a("h4", "Herramientas", "Estirar", "X"),
    a("h5", "Herramientas", "Cuchilla", "C"),
    a("h6", "Herramientas", "Deslizar", "Y"),
    a("h7", "Herramientas", "Correr", "U"),
    a("h8", "Herramientas", "Mano", "H"),
    a("h9", "Herramientas", "Lupa", "Z"),
    a("h10", "Herramientas", "Texto", "T"),
    a("acercar", "Vista", "Acercar", "Plus Equals"),
    a("alejar", "Vista", "Alejar", "Minus"),
    a("ver_todo", "Vista", "Ver todo el proyecto", "Backslash"),
    a("iman", "Vista", "Imán", "Shift+S"),
    a("p_playlist", "Vista", "Pestaña Playlist", "Cmd+1"),
    a("p_video", "Vista", "Pestaña Video", "Cmd+2"),
    a("p_audio", "Vista", "Pestaña Audio", "Cmd+3"),
    a("p_efectos", "Vista", "Pestaña Efectos", "Cmd+4"),
    a("p_animacion", "Vista", "Pestaña Animación", "Cmd+5"),
    a("p_nodos", "Vista", "Pestaña Nodos", "Cmd+6"),
    a("animar", "Animación", "Animar el clip elegido", "Shift+A"),
    a("clave", "Animación", "Poner cuadro clave", "Shift+K"),
    a("clave_antes", "Animación", "Cuadro clave anterior", "Shift+J"),
    a("clave_despues", "Animación", "Cuadro clave siguiente", "Shift+L"),
    a("auto", "Animación", "Grabar cuadros clave", "Shift+R"),
    a("copiar_claves", "Animación", "Copiar cuadros clave", "Cmd+C"),
    a("pegar_claves", "Animación", "Pegar cuadros clave", "Cmd+V"),
];

pub const GRUPOS: [&str; 6] = ["Proyecto", "Reproducción", "Edición", "Herramientas", "Vista", "Animación"];

/// Lee «Cmd+Shift+K».
pub fn leer(s: &str) -> Option<KeyboardShortcut> {
    let mut m = Modifiers::NONE;
    let mut tecla = None;
    for p in s.split('+') {
        match p {
            "Cmd" => m |= Modifiers::COMMAND,
            "Shift" => m |= Modifiers::SHIFT,
            "Alt" => m |= Modifiers::ALT,
            "Ctrl" => m |= Modifiers::CTRL,
            p => tecla = Some(Key::ALL.iter().copied().find(|k| format!("{k:?}") == p || k.name() == p || alias(p) == Some(*k))?),
        }
    }
    Some(KeyboardShortcut::new(m, tecla?))
}

fn alias(s: &str) -> Option<Key> {
    Some(match s {
        "Left" => Key::ArrowLeft,
        "Right" => Key::ArrowRight,
        "Up" => Key::ArrowUp,
        "Down" => Key::ArrowDown,
        _ => return None,
    })
}

pub fn escribir(k: &KeyboardShortcut) -> String {
    let m = k.modifiers;
    let mut s = String::new();
    for (on, n) in [(m.command || m.mac_cmd, "Cmd+"), (m.ctrl && !m.command, "Ctrl+"), (m.alt, "Alt+"), (m.shift, "Shift+")] {
        if on {
            s.push_str(n);
        }
    }
    s + &format!("{:?}", k.logical_key)
}

/// Como se ve en los menús: ⇧⌘K.
pub fn texto(k: &KeyboardShortcut) -> String {
    let t = crate::idioma::t;
    let tecla = match k.logical_key {
        Key::Space => t("espacio").into(),
        Key::Backspace => "⌫".to_string(),
        Key::Delete => "⌦".into(),
        Key::ArrowLeft => "←".into(),
        Key::ArrowRight => "→".into(),
        Key::ArrowUp => "↑".into(),
        Key::ArrowDown => "↓".into(),
        k => k.symbol_or_name().to_string(),
    };
    let mac = cfg!(target_os = "macos");
    let n = if mac { ModifierNames::SYMBOLS } else { ModifierNames::NAMES };
    let mut s = n.format(&k.modifiers, mac);
    if !s.is_empty() && !mac {
        s.push('+');
    }
    s + &tecla
}

type Tabla = BTreeMap<String, Vec<KeyboardShortcut>>;

pub fn de_fabrica(id: &str) -> Vec<KeyboardShortcut> {
    ACCIONES.iter().find(|a| a.id == id).map_or(vec![], |a| a.fabrica.split_whitespace().filter_map(leer).collect())
}

/// Solo lo que el usuario cambió.
static PUESTOS: LazyLock<RwLock<Tabla>> = LazyLock::new(Default::default);

pub fn de(id: &str) -> Vec<KeyboardShortcut> {
    PUESTOS.read().unwrap().get(id).cloned().unwrap_or_else(|| de_fabrica(id))
}

/// El primero, para los menús.
pub fn menu(id: &str) -> String {
    de(id).first().map(texto).unwrap_or_default()
}

pub fn cambiado(id: &str) -> bool {
    PUESTOS.read().unwrap().contains_key(id)
}

pub fn poner(id: &str, v: Vec<KeyboardShortcut>) {
    let mut p = PUESTOS.write().unwrap();
    if v == de_fabrica(id) {
        p.remove(id);
    } else {
        p.insert(id.into(), v);
    }
}

pub fn restablecer(id: Option<&str>) {
    let mut p = PUESTOS.write().unwrap();
    match id {
        Some(id) => drop(p.remove(id)),
        None => p.clear(),
    }
}

/// Qué otra acción usa esas teclas.
pub fn dueno(k: &KeyboardShortcut, menos: &str) -> Option<&'static Accion> {
    ACCIONES.iter().find(|a| a.id != menos && de(a.id).iter().any(|x| igual(x, k)))
}

fn igual(a: &KeyboardShortcut, b: &KeyboardShortcut) -> bool {
    a.logical_key == b.logical_key && norma(a.modifiers) == norma(b.modifiers)
}

/// (⌘ o Ctrl según el sistema, Control de Mac, Alt, Shift)
fn norma(m: Modifiers) -> (bool, bool, bool, bool) {
    let cmd = m.command || m.mac_cmd;
    (cmd, m.ctrl && !cmd, m.alt, m.shift)
}

/// Se guarda como texto: {"dividir": ["Cmd+K"]}.
pub fn a_json() -> serde_json::Value {
    let p = PUESTOS.read().unwrap();
    serde_json::Value::Object(p.iter().map(|(k, v)| (k.clone(), v.iter().map(|x| escribir(x).into()).collect::<Vec<serde_json::Value>>().into())).collect())
}

pub fn de_json(v: &serde_json::Value) {
    let Some(o) = v.as_object() else { return };
    let mut p = PUESTOS.write().unwrap();
    p.clear();
    for (k, x) in o.iter().filter(|(k, _)| ACCIONES.iter().any(|a| a.id == *k)) {
        p.insert(k.clone(), x.as_array().into_iter().flatten().filter_map(|s| leer(s.as_str()?)).collect());
    }
}

/// Signos que en algunos teclados salen con Shift: ahí no cuenta.
fn signo(k: Key) -> bool {
    matches!(k, Key::Plus | Key::Minus | Key::Equals | Key::Semicolon | Key::Quote | Key::Backslash | Key::Slash | Key::Comma | Key::Period | Key::OpenBracket | Key::CloseBracket | Key::Backtick | Key::Colon | Key::Pipe | Key::Questionmark | Key::Exclamationmark)
}

/// Saca de la lista de eventos las teclas con atajo y dice qué acciones tocan.
pub fn atender(eventos: &mut Vec<Event>) -> Vec<&'static str> {
    let mut r = vec![];
    eventos.retain(|e| {
        // ⌘C y ⌘V llegan ya como copiar y pegar.
        let (k, m) = match e {
            Event::Key { key, pressed: true, modifiers, .. } => (*key, *modifiers),
            Event::Copy => (Key::C, Modifiers::COMMAND),
            Event::Paste(_) => (Key::V, Modifiers::COMMAND),
            _ => return true,
        };
        let n = norma(m);
        let va = |x: &KeyboardShortcut| {
            let q = norma(x.modifiers);
            x.logical_key == k && (q == n || (signo(k) && (q.0, q.1, q.2) == (n.0, n.1, n.2) && !q.3))
        };
        match ACCIONES.iter().find(|a| de(a.id).iter().any(va)) {
            Some(a) => {
                r.push(a.id);
                false
            }
            None => true,
        }
    });
    r
}

/// La combinación que se acaba de apretar (para ponerla a una acción); None si todavía no hay.
pub fn capturar(eventos: &[Event]) -> Option<KeyboardShortcut> {
    eventos.iter().find_map(|e| match e {
        Event::Key { key, pressed: true, repeat: false, modifiers, .. } => Some(KeyboardShortcut::new(limpio(*modifiers), *key)),
        Event::Copy => Some(KeyboardShortcut::new(Modifiers::COMMAND, Key::C)),
        Event::Paste(_) => Some(KeyboardShortcut::new(Modifiers::COMMAND, Key::V)),
        Event::Cut => Some(KeyboardShortcut::new(Modifiers::COMMAND, Key::X)),
        _ => None,
    })
}

fn limpio(m: Modifiers) -> Modifiers {
    let (cmd, ctrl, alt, shift) = norma(m);
    let mut r = Modifiers { alt, shift, ..Modifiers::NONE };
    if cmd {
        r |= Modifiers::COMMAND;
    }
    if ctrl {
        r |= Modifiers::CTRL;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn de_fabrica_sin_choques() {
        for a in ACCIONES {
            assert!(GRUPOS.contains(&a.grupo), "{}", a.id);
            assert_eq!(de_fabrica(a.id).len(), a.fabrica.split_whitespace().count(), "{}: {}", a.id, a.fabrica);
            for k in de_fabrica(a.id) {
                assert_eq!(leer(&escribir(&k)).map(|x| igual(&x, &k)), Some(true), "{}", escribir(&k));
                for b in ACCIONES.iter().filter(|b| b.id != a.id) {
                    assert!(!de_fabrica(b.id).iter().any(|x| igual(x, &k)), "{} y {} usan {}", a.id, b.id, escribir(&k));
                }
            }
        }
    }

    #[test]
    fn se_guardan_y_vuelven() {
        let k = leer("Cmd+Shift+F9").unwrap();
        poner("grabar", vec![k]);
        let j = a_json();
        restablecer(None);
        assert!(!cambiado("grabar"));
        de_json(&j);
        assert!(igual(&de("grabar")[0], &k));
        let tecla = |key, modifiers| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers };
        // ⌘ llega con las dos marcas en Mac; Shift de más no cuenta en los signos.
        let mut ev = vec![tecla(Key::F9, Modifiers { command: true, mac_cmd: true, shift: true, ..Modifiers::NONE }), tecla(Key::R, Modifiers::NONE), tecla(Key::Plus, Modifiers::SHIFT)];
        assert_eq!(atender(&mut ev), ["grabar", "acercar"]);
        assert_eq!(ev.len(), 1, "R ya no hace nada");
        restablecer(None);
    }
}
