// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Deja abierto el servidor MCP con las herramientas del asistente, para probarlo con otro programa: `cargo run --example mcp 18766 llave-de-16-letras`
use beethoven_daw_demo::ia::{Puerta, mcp};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (p, llamadas) = Puerta::nueva(std::sync::Arc::new(|| {}));
    let h = beethoven_daw_demo::ui::asistente::herramientas();
    let s = mcp::abrir(a[1].parse().unwrap(), &a[2], std::sync::Arc::new(h), p).unwrap();
    println!("{}", s.url);
    for l in llamadas {
        println!("{} {}", l.nombre, l.datos);
        l.responder(Ok("bpm 77".into()));
    }
}
