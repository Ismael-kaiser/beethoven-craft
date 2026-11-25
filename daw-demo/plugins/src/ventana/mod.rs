// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! La ventana del plugin dentro de la del otro programa, en cada sistema.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::Ventana;
#[cfg(target_os = "macos")]
pub use mac::Ventana;
#[cfg(target_os = "windows")]
pub use windows::Ventana;

/// Cuánto mide la ventana de ese plugin, como lo pide cada sistema: puntos en Mac, píxeles en los otros.
pub fn medida(n: &beethoven_daw_demo::instancia::Instancia) -> (u32, u32) {
    let (w, h) = crate::lienzo::medida(n);
    #[cfg(target_os = "windows")]
    {
        let e = windows::escala();
        return ((w as f32 * e).round() as u32, (h as f32 * e).round() as u32);
    }
    #[cfg(not(target_os = "windows"))]
    (w, h)
}
