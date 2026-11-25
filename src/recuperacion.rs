//! Si la app se cierra de golpe (un error, se va la luz, se fuerza a salir), el trabajo no se pierde: mientras corre
//! deja una marca de «sesión abierta» y el autoguardado se hace a los pocos segundos de cada cambio. Al salir bien la
//! marca se borra. Si al arrancar la marca sigue ahí con cambios sin guardar, el autoguardado se aparta a la carpeta
//! «Recuperación» (para que la sesión nueva no lo pise) y se ofrece abrirlo.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Lo que se sabe de la sesión en curso.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Sesion {
    /// El archivo del proyecto, si ya tiene.
    pub ruta: Option<PathBuf>,
    /// Hay cambios que solo están en el autoguardado.
    pub sin_guardar: bool,
}

/// Lo que quedó de una sesión que se cerró de golpe.
#[derive(Clone, Debug, PartialEq)]
pub struct Pendiente {
    /// La copia apartada del autoguardado.
    pub archivo: PathBuf,
    /// El proyecto original, si tenía archivo.
    pub ruta: Option<PathBuf>,
    /// Segundos desde 1970 en que se guardó por última vez.
    pub cuando: u64,
}

fn marca(dir: &Path) -> PathBuf {
    dir.join("Sesión abierta.json")
}

/// Anota cómo va la sesión. Se llama al arrancar, en cada autoguardado y al guardar.
pub fn anotar(dir: &Path, s: &Sesion) {
    let _ = std::fs::create_dir_all(dir);
    if let Ok(b) = serde_json::to_vec(s) {
        let _ = std::fs::write(marca(dir), b);
    }
}

/// La app se cierra bien: ya no hay nada que recuperar.
pub fn cerrada_bien(dir: &Path) {
    let _ = std::fs::remove_file(marca(dir));
}

/// Al arrancar, antes de tocar el autoguardado: si la sesión anterior no se cerró bien y tenía cambios sin guardar,
/// aparta su autoguardado y lo da. La marca vieja se quita.
pub fn pendiente(dir: &Path) -> Option<Pendiente> {
    let m = marca(dir);
    let s: Sesion = serde_json::from_slice(&std::fs::read(&m).ok()?).ok()?;
    let _ = std::fs::remove_file(&m);
    let auto = dir.join("Autoguardado.bcraft");
    if !s.sin_guardar || !auto.is_file() {
        return None;
    }
    let cuando = std::fs::metadata(&auto).and_then(|x| x.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    let lugar = dir.join("Recuperación");
    std::fs::create_dir_all(&lugar).ok()?;
    let nombre = s.ruta.as_ref().and_then(|r| r.file_stem()).map_or_else(|| "Sin nombre".to_string(), |n| n.to_string_lossy().into_owned());
    let archivo = lugar.join(format!("{nombre} ({cuando}).bcraft"));
    std::fs::copy(&auto, &archivo).ok()?;
    Some(Pendiente { archivo, ruta: s.ruta, cuando })
}
