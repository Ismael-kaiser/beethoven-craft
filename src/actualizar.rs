//! Actualizaciones: la app lee `latest.json` de la última versión publicada en GitHub (no manda nada tuyo, solo pide
//! ese archivo), compara con su versión y, si la persona quiere, baja el paquete de su sistema, revisa su huella
//! SHA-256 y lo instala: en Mac cambia la app por la nueva, en Windows corre el instalador en silencio y en Linux
//! reemplaza los archivos de su carpeta. Después se vuelve a abrir sola.
use crate::medios;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

/// Dónde se publican las versiones.
pub const REPO: &str = "Ismael-kaiser/beethoven-craft";

/// La versión de esta app (la de Cargo.toml).
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Una versión nueva para este sistema.
#[derive(Clone, Debug, PartialEq)]
pub struct Nueva {
    pub version: String,
    pub notas: String,
    pub url: String,
    pub sha256: String,
    pub peso: u64,
}

/// La clave de este sistema en `latest.json`.
pub fn sistema() -> &'static str {
    if cfg!(target_os = "macos") {
        "mac"
    } else if cfg!(windows) {
        "windows"
    } else {
        "linux"
    }
}

/// «0.11.2» → [0, 11, 2]. Lo que no sea número cuenta como 0.
fn partes(v: &str) -> Vec<u64> {
    v.trim().trim_start_matches('v').split(['.', '-']).map(|x| x.parse().unwrap_or(0)).collect()
}

/// ¿`a` es más nueva que `b`?
pub fn mas_nueva(a: &str, b: &str) -> bool {
    let (x, y) = (partes(a), partes(b));
    for i in 0..x.len().max(y.len()) {
        let (p, q) = (x.get(i).copied().unwrap_or(0), y.get(i).copied().unwrap_or(0));
        if p != q {
            return p > q;
        }
    }
    false
}

/// Lee `latest.json` (su texto) y da la versión para este sistema si es más nueva que `actual`.
pub fn leer(json: &str, actual: &str) -> Result<Option<Nueva>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|_| crate::idioma::t("La lista de versiones no se pudo leer").to_string())?;
    let version = v["version"].as_str().unwrap_or("").to_string();
    if version.is_empty() || !mas_nueva(&version, actual) {
        return Ok(None);
    }
    let s = &v[sistema()];
    let (Some(url), Some(sha)) = (s["url"].as_str(), s["sha256"].as_str()) else {
        return Ok(None);
    };
    Ok(Some(Nueva { version, notas: v["notas"].as_str().unwrap_or("").to_string(), url: url.to_string(), sha256: sha.to_lowercase(), peso: s["peso"].as_u64().unwrap_or(0) }))
}

/// Pregunta a GitHub si hay una versión nueva. Tarda un momento: va en segundo plano.
pub fn buscar() -> Result<Option<Nueva>, String> {
    let url = format!("https://github.com/{REPO}/releases/latest/download/latest.json");
    let s = medios::comando("curl").args(["-fsSL", "--max-time", "15"]).arg(&url).output().map_err(|e| e.to_string())?;
    if !s.status.success() {
        return Err(crate::idioma::t("No pude ver si hay versiones nuevas (¿sin internet?)").into());
    }
    leer(&String::from_utf8_lossy(&s.stdout), version())
}

/// Huella SHA-256 de un archivo con la herramienta del sistema.
pub fn huella(r: &Path) -> Option<String> {
    let intentos: [(&str, Vec<&str>); 3] = [("shasum", vec!["-a", "256"]), ("sha256sum", vec![]), ("certutil", vec!["-hashfile"])];
    for (prog, args) in intentos {
        let mut c = medios::comando(prog);
        c.args(&args).arg(r);
        if prog == "certutil" {
            c.arg("SHA256");
        }
        let Ok(s) = c.stderr(Stdio::null()).output() else { continue };
        if !s.status.success() {
            continue;
        }
        // La primera palabra de 64 cifras hexadecimales.
        let texto = String::from_utf8_lossy(&s.stdout);
        if let Some(h) = texto.split(|c: char| !c.is_ascii_hexdigit()).find(|w| w.len() == 64) {
            return Some(h.to_lowercase());
        }
    }
    None
}

/// Baja el paquete a la carpeta temporal y revisa que llegó completo y sin cambios. `avance` de 0 a 1000.
pub fn descargar(n: &Nueva, avance: &AtomicU32, cancelar: &AtomicBool) -> Result<PathBuf, String> {
    let nombre = n.url.rsplit('/').next().filter(|x| !x.is_empty()).unwrap_or("actualizacion");
    let dir = std::env::temp_dir().join("beethoven-craft-actualizacion");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let archivo = dir.join(nombre);
    let mut hijo = medios::comando("curl").args(["-fsSL", "--retry", "2", "-o"]).arg(&archivo).arg(&n.url).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?;
    loop {
        if cancelar.load(Relaxed) {
            let _ = hijo.kill();
            return Err(crate::idioma::t("Descarga cancelada").into());
        }
        if let Ok(Some(s)) = hijo.try_wait() {
            if !s.success() {
                return Err(crate::idioma::t("No se pudo bajar la actualización").into());
            }
            break;
        }
        if n.peso > 0 {
            let ya = std::fs::metadata(&archivo).map_or(0, |m| m.len());
            avance.store((ya * 990 / n.peso).min(990) as u32, Relaxed);
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
    match huella(&archivo) {
        Some(h) if h == n.sha256 => {
            avance.store(1000, Relaxed);
            Ok(archivo)
        }
        Some(_) => Err(crate::idioma::t("La descarga llegó dañada o cambiada: no se instala").into()),
        None => Err(crate::idioma::t("No pude revisar la descarga").into()),
    }
}

/// La carpeta de la app que está corriendo: la `.app` en Mac, la carpeta del programa en Windows y Linux.
fn instalada() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if cfg!(target_os = "macos") {
        // …/Beethoven Craft.app/Contents/MacOS/beethoven-craft
        return exe.ancestors().nth(3).filter(|p| p.extension().is_some_and(|e| e == "app")).map(Path::to_path_buf).ok_or_else(|| crate::idioma::t("Solo se actualiza desde la app instalada").into());
    }
    exe.parent().map(Path::to_path_buf).ok_or_else(|| "?".into())
}

/// Instala el paquete bajado y vuelve a abrir la app. Si todo sale bien, esta app se cierra (no regresa).
pub fn instalar(archivo: &Path) -> Result<(), String> {
    let destino = instalada()?;
    let tmp = archivo.parent().unwrap_or(Path::new(".")).join("nueva");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let corre = |c: &mut std::process::Command| c.status().is_ok_and(|s| s.success());
    if cfg!(target_os = "macos") {
        if !corre(medios::comando("ditto").args(["-x", "-k"]).arg(archivo).arg(&tmp)) {
            return Err(crate::idioma::t("No pude abrir el paquete").into());
        }
        let nueva = std::fs::read_dir(&tmp).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|e| e == "app")).ok_or(crate::idioma::t("El paquete no trae la app"))?;
        // Se cambia de golpe: la vieja a un lado, la nueva en su lugar.
        let vieja = destino.with_extension("app-vieja");
        let _ = std::fs::remove_dir_all(&vieja);
        std::fs::rename(&destino, &vieja).map_err(|e| e.to_string())?;
        if !corre(medios::comando("ditto").arg(&nueva).arg(&destino)) {
            let _ = std::fs::rename(&vieja, &destino);
            return Err(crate::idioma::t("No pude poner la app nueva en su lugar").into());
        }
        let _ = std::fs::remove_dir_all(&vieja);
        medios::comando("open").arg("-n").arg(&destino).spawn().map_err(|e| e.to_string())?;
    } else if cfg!(windows) {
        // El instalador cierra esta app, instala sin preguntar y la vuelve a abrir.
        medios::comando(archivo).args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/CLOSEAPPLICATIONS"]).spawn().map_err(|e| e.to_string())?;
    } else {
        if !corre(medios::comando("tar").arg("-xzf").arg(archivo).arg("-C").arg(&tmp)) {
            return Err(crate::idioma::t("No pude abrir el paquete").into());
        }
        let nueva = tmp.join("beethoven-craft");
        // Encima de la carpeta de siempre (un programa que corre se puede reemplazar en Linux).
        if !corre(medios::comando("cp").arg("-rf").arg(format!("{}/.", nueva.display())).arg(&destino)) {
            return Err(crate::idioma::t("No pude poner la app nueva en su lugar").into());
        }
        medios::comando(destino.join("beethoven-craft")).spawn().map_err(|e| e.to_string())?;
    }
    crate::recuperacion::cerrada_bien(&crate::ui::carpeta());
    std::process::exit(0);
}

fn preferencia(dir: &Path) -> PathBuf {
    dir.join("Actualizaciones.json")
}

/// ¿Buscar versiones nuevas al abrir? Sí, a menos que se haya apagado.
pub fn buscar_al_abrir(dir: &Path) -> bool {
    std::fs::read(preferencia(dir)).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()).and_then(|v| v["buscar"].as_bool()).unwrap_or(true)
}

pub fn poner_buscar_al_abrir(dir: &Path, si: bool) {
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(preferencia(dir), serde_json::json!({ "buscar": si }).to_string());
}
