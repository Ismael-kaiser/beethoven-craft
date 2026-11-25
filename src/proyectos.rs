//! Los proyectos del menú: videos de Beethoven Movie y canciones de Beethoven DAW. Renombrar, duplicar y una papelera
//! propia de donde se pueden recuperar. Solo toca el archivo del proyecto, nunca los videos, sonidos o imágenes que usa.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Cuántos proyectos recientes se recuerdan.
const TOPE: usize = 60;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tipo {
    Video,
    Cancion,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Ficha {
    pub ruta: PathBuf,
    pub nombre: String,
    pub tipo: Tipo,
    pub fecha: SystemTime,
    pub tam: u64,
}

/// Un proyecto en la papelera.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Borrado {
    /// Nombre dentro de la papelera.
    pub archivo: String,
    pub original: PathBuf,
    /// Segundos desde 1970.
    pub cuando: u64,
}

fn leer<T: serde::de::DeserializeOwned + Default>(r: &Path) -> T {
    std::fs::read(r).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn escribir<T: Serialize>(r: &Path, v: &T) {
    if let Some(c) = r.parent() {
        let _ = std::fs::create_dir_all(c);
    }
    if let Ok(b) = serde_json::to_vec_pretty(v) {
        let _ = std::fs::write(r, b);
    }
}

fn nombre(r: &Path) -> String {
    r.file_stem().map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

pub fn es_cancion(r: &Path) -> bool {
    r.extension().is_some_and(|e| e == "bdaw" || e == "flp")
}

/// La carpeta de Movie (recientes, papelera) donde vive todo esto.
pub struct Libro {
    pub base: PathBuf,
}

impl Libro {
    fn lista(&self) -> PathBuf {
        self.base.join("Recientes.json")
    }
    fn ocultos(&self) -> PathBuf {
        self.base.join("Ocultos.json")
    }
    fn papelera_dir(&self) -> PathBuf {
        self.base.join("Papelera")
    }
    fn papelera_lista(&self) -> PathBuf {
        self.papelera_dir().join("Papelera.json")
    }

    pub fn recientes(&self) -> Vec<PathBuf> {
        leer::<Vec<PathBuf>>(&self.lista()).into_iter().filter(|r| r.exists()).collect()
    }

    /// Pone `ruta` al principio de los recientes.
    pub fn anotar(&self, ruta: &Path) {
        let ruta = ruta.canonicalize().unwrap_or_else(|_| ruta.to_path_buf());
        let mut l = self.recientes();
        l.retain(|r| *r != ruta);
        l.insert(0, ruta.clone());
        l.truncate(TOPE);
        escribir(&self.lista(), &l);
        self.mostrar(&ruta);
    }

    fn mostrar(&self, ruta: &Path) {
        let mut o: Vec<PathBuf> = leer(&self.ocultos());
        if o.iter().any(|x| x == ruta) {
            o.retain(|x| x != ruta);
            escribir(&self.ocultos(), &o);
        }
    }

    /// Lo saca de la lista sin tocar el archivo.
    pub fn quitar(&self, ruta: &Path) {
        let mut l = self.recientes();
        l.retain(|r| r != ruta);
        escribir(&self.lista(), &l);
        let mut o: Vec<PathBuf> = leer(&self.ocultos());
        if !o.iter().any(|x| x == ruta) {
            o.push(ruta.into());
            escribir(&self.ocultos(), &o);
        }
    }

    /// Los recientes, los que haya sueltos en la carpeta de la app y las canciones del DAW, del más nuevo al más viejo.
    pub fn listar(&self, canciones: &[PathBuf]) -> Vec<Ficha> {
        let ocultos: Vec<PathBuf> = leer(&self.ocultos());
        let mut v = self.recientes();
        let sueltos = std::fs::read_dir(&self.base).into_iter().flatten().flatten().map(|d| d.path());
        v.extend(sueltos.filter(|r| r.extension().is_some_and(|e| e == "bcraft") && nombre(r) != "Autoguardado"));
        v.extend(canciones.iter().cloned());
        let mut vistos: Vec<PathBuf> = vec![];
        let mut fichas: Vec<Ficha> = v
            .into_iter()
            .filter_map(|r| {
                let real = r.canonicalize().ok()?;
                if vistos.contains(&real) || ocultos.contains(&r) || ocultos.contains(&real) {
                    return None;
                }
                vistos.push(real);
                let m = std::fs::metadata(&r).ok().filter(|m| m.is_file())?;
                Some(Ficha { nombre: nombre(&r), tipo: if es_cancion(&r) { Tipo::Cancion } else { Tipo::Video }, fecha: m.modified().unwrap_or(SystemTime::UNIX_EPOCH), tam: m.len(), ruta: r })
            })
            .collect();
        fichas.sort_by(|a, b| b.fecha.cmp(&a.fecha));
        fichas
    }

    fn cambiar(&self, de: &Path, a: &Path) {
        let mut l: Vec<PathBuf> = leer(&self.lista());
        for r in l.iter_mut().filter(|r| *r == de) {
            *r = a.into();
        }
        escribir(&self.lista(), &l);
    }

    /// Cambia el nombre del archivo, en su misma carpeta.
    pub fn renombrar(&self, ruta: &Path, nuevo: &str) -> Result<PathBuf, String> {
        let limpio: String = nuevo.trim().chars().map(|c| if "/\\:*?\"<>|".contains(c) || c.is_control() { '-' } else { c }).collect();
        let limpio = limpio.trim_matches(|c: char| c == '.' || c == ' ').to_string();
        if limpio.is_empty() {
            return Err(crate::idioma::t("Escribe un nombre").into());
        }
        let ext = ruta.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
        let a = ruta.with_file_name(format!("{limpio}.{ext}"));
        if a == ruta {
            return Ok(a);
        }
        // En discos que no distinguen mayúsculas, cambiar solo eso es el mismo archivo.
        let mismo = a.canonicalize().ok().zip(ruta.canonicalize().ok()).is_some_and(|(x, y)| x == y);
        if a.exists() && !mismo {
            return Err(crate::idioma::t("Ya hay un proyecto con ese nombre ahí").into());
        }
        std::fs::rename(ruta, &a).map_err(|e| e.to_string())?;
        self.cambiar(ruta, &a);
        Ok(a)
    }

    /// Una copia al lado, «Nombre copia».
    pub fn duplicar(&self, ruta: &Path) -> Result<PathBuf, String> {
        let ext = ruta.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
        let n = nombre(ruta);
        let copia = crate::idioma::t("copia");
        let a = (1..).map(|k| ruta.with_file_name(if k == 1 { format!("{n} {copia}.{ext}") } else { format!("{n} {copia} {k}.{ext}") })).find(|a| !a.exists()).unwrap();
        std::fs::copy(ruta, &a).map_err(|e| e.to_string())?;
        self.anotar(&a);
        Ok(a)
    }

    // ---- papelera

    pub fn papelera(&self) -> Vec<Borrado> {
        let mut v: Vec<Borrado> = leer(&self.papelera_lista());
        v.retain(|b| self.papelera_dir().join(&b.archivo).is_file());
        v.sort_by_key(|b| std::cmp::Reverse(b.cuando));
        v
    }

    pub fn archivo_de(&self, b: &Borrado) -> PathBuf {
        self.papelera_dir().join(&b.archivo)
    }

    /// Manda el archivo del proyecto a la papelera de la app.
    pub fn eliminar(&self, ruta: &Path) -> Result<(), String> {
        let dir = self.papelera_dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let cuando = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let ext = ruta.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
        let archivo = (0..).map(|k| format!("{}-{cuando}-{k}.{ext}", nombre(ruta))).find(|a| !dir.join(a).exists()).unwrap();
        mover(ruta, &dir.join(&archivo))?;
        let mut v = self.papelera();
        v.push(Borrado { archivo, original: ruta.into(), cuando });
        escribir(&self.papelera_lista(), &v);
        let mut l: Vec<PathBuf> = leer(&self.lista());
        l.retain(|r| r != ruta);
        escribir(&self.lista(), &l);
        Ok(())
    }

    /// Lo regresa a donde estaba; si ya hay otro ahí, con otro nombre.
    pub fn restaurar(&self, b: &Borrado) -> Result<PathBuf, String> {
        let ext = b.original.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
        let dir = b.original.parent().filter(|c| c.is_dir()).map_or_else(|| self.base.clone(), Path::to_path_buf);
        let n = nombre(&b.original);
        let a = (1..).map(|k| dir.join(if k == 1 { format!("{n}.{ext}") } else { format!("{n} {k}.{ext}") })).find(|a| !a.exists()).unwrap();
        mover(&self.archivo_de(b), &a)?;
        self.olvidar(b);
        if es_cancion(&a) { self.mostrar(&a) } else { self.anotar(&a) }
        Ok(a)
    }

    /// Lo borra para siempre.
    pub fn borrar(&self, b: &Borrado) -> Result<(), String> {
        std::fs::remove_file(self.archivo_de(b)).map_err(|e| e.to_string())?;
        self.olvidar(b);
        Ok(())
    }

    fn olvidar(&self, b: &Borrado) {
        let mut v: Vec<Borrado> = leer(&self.papelera_lista());
        v.retain(|x| x.archivo != b.archivo);
        escribir(&self.papelera_lista(), &v);
    }
}

/// Mueve un archivo aunque el destino esté en otro disco.
fn mover(de: &Path, a: &Path) -> Result<(), String> {
    if std::fs::rename(de, a).is_ok() {
        return Ok(());
    }
    std::fs::copy(de, a).map_err(|e| e.to_string())?;
    std::fs::remove_file(de).map_err(|e| {
        // Sin poder quitar el de origen, la copia sobra.
        let _ = std::fs::remove_file(a);
        e.to_string()
    })
}

/// Las canciones que Beethoven DAW recuerda, de su archivo de ajustes (solo se lee).
/// Las canciones de Beethoven DAW: los `.bdaw` de su carpeta (`Música/Beethoven DAW Demo`), sin su respaldo.
pub fn canciones_del_daw() -> Vec<PathBuf> {
    let casa = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from).filter(|p| p.is_absolute());
    let Some(dir) = casa.map(|c| c.join("Music").join("Beethoven DAW Demo")) else { return vec![] };
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "bdaw") && p.file_stem().is_some_and(|n| n != "respaldo"))
        .collect();
    v.sort();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn libro(n: &str) -> Libro {
        let base = std::env::temp_dir().join(format!("proyectos-{n}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        Libro { base: base.canonicalize().unwrap() }
    }
    fn hacer(l: &Libro, n: &str, t: &str) -> PathBuf {
        let r = l.base.join(n);
        std::fs::write(&r, t).unwrap();
        r
    }
    fn nombres(l: &Libro) -> Vec<String> {
        let mut v: Vec<String> = l.listar(&[]).into_iter().map(|f| f.nombre).collect();
        v.sort();
        v
    }

    #[test]
    fn lista_y_quita() {
        let l = libro("lista");
        let a = hacer(&l, "boda.bcraft", "a");
        hacer(&l, "Autoguardado.bcraft", "x");
        hacer(&l, "nota.txt", "x");
        let c = hacer(&l, "cumbia.bdaw", "c");
        l.anotar(&a);
        l.anotar(&a);
        let v = l.listar(&[c.clone(), c.clone(), l.base.join("no está.bdaw")]);
        assert_eq!(v.len(), 2, "{v:?}");
        assert_eq!(v.iter().find(|f| f.nombre == "cumbia").unwrap().tipo, Tipo::Cancion);
        l.quitar(&c);
        l.quitar(&a);
        assert!(l.listar(std::slice::from_ref(&c)).is_empty() && a.exists() && c.exists(), "quitar no borra");
        l.anotar(&a);
        assert_eq!(nombres(&l), ["boda"], "abrirlo lo regresa");
        let _ = std::fs::remove_dir_all(&l.base);
    }

    #[test]
    fn renombra_y_duplica() {
        let l = libro("nombres");
        let a = hacer(&l, "boda.bcraft", "contenido");
        l.anotar(&a);
        let d = l.duplicar(&a).unwrap();
        let d2 = l.duplicar(&a).unwrap();
        assert!(d != d2 && std::fs::read_to_string(&d).unwrap() == "contenido");
        assert!(l.renombrar(&a, "  ").is_err());
        assert!(l.renombrar(&a, &nombre(&d)).is_err(), "no encima de otro");
        assert_eq!(std::fs::read_to_string(&d).unwrap(), "contenido");
        let b = l.renombrar(&a, "Boda: Ana/Luis?").unwrap();
        assert_eq!(b.file_name().unwrap(), "Boda- Ana-Luis-.bcraft");
        assert!(!a.exists() && l.recientes().contains(&b));
        // Solo cambiar mayúsculas.
        let e = l.renombrar(&b, "BODA- ana-luis-").unwrap();
        assert!(e.exists());
        // No se sale de la carpeta.
        let f = l.renombrar(&e, "../fuera").unwrap();
        assert_eq!(f.parent(), Some(l.base.as_path()));
        let _ = std::fs::remove_dir_all(&l.base);
    }

    #[test]
    fn papelera_de_ida_y_vuelta() {
        let l = libro("papelera");
        let a = hacer(&l, "boda.bcraft", "uno");
        l.anotar(&a);
        l.eliminar(&a).unwrap();
        assert!(!a.exists() && nombres(&l).is_empty());
        let p = l.papelera();
        assert_eq!((p.len(), p[0].original.clone()), (1, a.clone()));
        // Otro con el mismo nombre también cabe.
        hacer(&l, "boda.bcraft", "dos");
        l.eliminar(&a).unwrap();
        assert_eq!(l.papelera().len(), 2);
        // Regresan sin encimarse.
        let nuevo = hacer(&l, "boda.bcraft", "tres");
        for b in l.papelera() {
            l.restaurar(&b).unwrap();
        }
        assert_eq!(std::fs::read_to_string(&nuevo).unwrap(), "tres");
        assert_eq!(nombres(&l), ["boda", "boda 2", "boda 3"]);
        assert!(l.papelera().is_empty());
        // Para siempre.
        l.eliminar(&nuevo).unwrap();
        let b = l.papelera().remove(0);
        l.borrar(&b).unwrap();
        assert!(l.papelera().is_empty() && !l.archivo_de(&b).exists());
        let _ = std::fs::remove_dir_all(&l.base);
    }

}
