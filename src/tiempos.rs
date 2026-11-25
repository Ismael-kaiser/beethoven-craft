//! Marcas de tiempo del arranque y de la carga, para medir: solo se escriben con `BC_TIEMPOS=1`.
use std::sync::OnceLock;
use std::time::Instant;

fn inicio() -> Instant {
    static I: OnceLock<Instant> = OnceLock::new();
    *I.get_or_init(Instant::now)
}

pub fn marca(que: &str) {
    static SI: OnceLock<bool> = OnceLock::new();
    if *SI.get_or_init(|| std::env::var_os("BC_TIEMPOS").is_some()) {
        eprintln!("[{:7.1} ms] {que}", inicio().elapsed().as_secs_f64() * 1e3);
    }
}

/// Marca cuánto tardó cada uno de los primeros cuadros de la ventana.
pub struct Cuadro(Instant);

impl Cuadro {
    pub fn nuevo() -> Cuadro {
        Cuadro(Instant::now())
    }
}

impl Drop for Cuadro {
    fn drop(&mut self) {
        use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Relaxed);
        if n < 30 {
            marca(&format!("cuadro {n}: {:.1} ms", self.0.elapsed().as_secs_f64() * 1e3));
        }
    }
}
