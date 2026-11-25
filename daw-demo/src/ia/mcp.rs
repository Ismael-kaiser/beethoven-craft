// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Servidor MCP local: deja que Claude Code (u otro programa con MCP) use las herramientas. Solo escucha en esta máquina
//! y pide una llave; rechaza lo que venga de una página web.
use super::{Herramienta, Puerta};
use crate::idioma::tf;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::time::Duration;

const MAX_CABEZA: usize = 16 << 10;
const MAX_CUERPO: usize = 4 << 20;
const MAX_CONEXIONES: usize = 16;

pub struct Servidor {
    pub puerto: u16,
    pub url: String,
    apagar: Arc<AtomicBool>,
}

impl Drop for Servidor {
    fn drop(&mut self) {
        self.apagar.store(true, Relaxed);
        // Despierta al que espera conexiones.
        let _ = TcpStream::connect_timeout(&([127, 0, 0, 1], self.puerto).into(), Duration::from_millis(200));
    }
}

pub fn abrir(puerto: u16, token: &str, h: Arc<Vec<Herramienta>>, puerta: Puerta) -> Result<Servidor, String> {
    if token.len() < 16 {
        return Err(crate::idioma::t("el servidor necesita una llave").into());
    }
    let l = TcpListener::bind(("127.0.0.1", puerto)).map_err(|e| tf("No se pudo abrir el puerto {}: {}", &[&puerto, &e]))?;
    let puerto = l.local_addr().map_err(|e| e.to_string())?.port();
    let apagar = Arc::new(AtomicBool::new(false));
    let (fin, token) = (apagar.clone(), token.to_string());
    let abiertas = Arc::new(AtomicUsize::new(0));
    std::thread::spawn(move || {
        for s in l.incoming() {
            if fin.load(Relaxed) {
                break;
            }
            let Ok(s) = s else { continue };
            if abiertas.fetch_add(1, Relaxed) >= MAX_CONEXIONES {
                abiertas.fetch_sub(1, Relaxed);
                continue;
            }
            let (h, p, token, a) = (h.clone(), puerta.clone(), token.clone(), abiertas.clone());
            std::thread::spawn(move || {
                let _ = atender(s, puerto, &token, &h, &p);
                a.fetch_sub(1, Relaxed);
            });
        }
    });
    Ok(Servidor { puerto, url: format!("http://127.0.0.1:{puerto}/mcp"), apagar })
}

/// Compara sin decir por el tiempo en qué letra falló.
fn igual(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0, |x, (a, b)| x | (a ^ b)) == 0
}

fn contestar(s: &mut TcpStream, estado: &str, cuerpo: &str) -> std::io::Result<()> {
    write!(s, "HTTP/1.1 {estado}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cuerpo}", cuerpo.len())
}

fn atender(mut s: TcpStream, puerto: u16, token: &str, h: &[Herramienta], p: &Puerta) -> std::io::Result<()> {
    s.set_read_timeout(Some(Duration::from_secs(10)))?;
    s.set_write_timeout(Some(Duration::from_secs(10)))?;
    let mut b = vec![];
    let mut x = [0u8; 4096];
    let corte = loop {
        if let Some(i) = b.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        if b.len() > MAX_CABEZA {
            return contestar(&mut s, "431 Request Header Fields Too Large", "");
        }
        match s.read(&mut x)? {
            0 => return Ok(()),
            n => b.extend_from_slice(&x[..n]),
        }
    };
    let cabeza = String::from_utf8_lossy(&b[..corte]).into_owned();
    let mut l = cabeza.lines();
    let primera = l.next().unwrap_or("");
    let campo = |n: &str| {
        cabeza.lines().skip(1).find_map(|l| l.split_once(':').filter(|(k, _)| k.trim().eq_ignore_ascii_case(n)).map(|x| x.1.trim().to_string()))
    };
    let mut partes = primera.split(' ');
    let (metodo, ruta) = (partes.next().unwrap_or(""), partes.next().unwrap_or(""));
    let propio = |x: &str| [format!("127.0.0.1:{puerto}"), format!("localhost:{puerto}")].iter().any(|y| x == y);
    // Una página web no puede usarlo, aunque apunte su nombre a esta máquina.
    let origen = campo("origin").is_none_or(|o| propio(o.trim_start_matches("http://")));
    if !origen || !campo("host").is_some_and(|x| propio(&x)) {
        return contestar(&mut s, "403 Forbidden", "");
    }
    if !campo("authorization").is_some_and(|a| a.strip_prefix("Bearer ").is_some_and(|x| igual(x, token))) {
        return contestar(&mut s, "401 Unauthorized", "");
    }
    if ruta.split('?').next() != Some("/mcp") {
        return contestar(&mut s, "404 Not Found", "");
    }
    if metodo != "POST" {
        return contestar(&mut s, "405 Method Not Allowed", "");
    }
    let largo: usize = campo("content-length").and_then(|x| x.parse().ok()).unwrap_or(0);
    if largo > MAX_CUERPO {
        return contestar(&mut s, "413 Payload Too Large", "");
    }
    let mut cuerpo = b[corte + 4..].to_vec();
    while cuerpo.len() < largo {
        match s.read(&mut x)? {
            0 => return Ok(()),
            n => cuerpo.extend_from_slice(&x[..n]),
        }
    }
    let Ok(v) = serde_json::from_slice::<Value>(&cuerpo[..largo]) else {
        return contestar(
            &mut s,
            "400 Bad Request",
            &json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "JSON" } }).to_string(),
        );
    };
    let r = match &v {
        Value::Array(a) => {
            let r: Vec<Value> = a.iter().filter_map(|v| mensaje(v, h, p)).collect();
            (!r.is_empty()).then_some(Value::Array(r))
        }
        v => mensaje(v, h, p),
    };
    match r {
        Some(r) => contestar(&mut s, "200 OK", &r.to_string()),
        None => contestar(&mut s, "202 Accepted", ""),
    }
}

/// Contesta un mensaje JSON-RPC; las notificaciones no llevan respuesta.
pub fn mensaje(v: &Value, h: &[Herramienta], p: &Puerta) -> Option<Value> {
    let id = v.get("id").filter(|x| !x.is_null())?;
    let bien = |r: Value| Some(json!({ "jsonrpc": "2.0", "id": id, "result": r }));
    match v["method"].as_str().unwrap_or("") {
        "initialize" => bien(json!({
            "protocolVersion": v["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": env!("CARGO_PKG_NAME"), "version": env!("CARGO_PKG_VERSION") },
        })),
        "ping" => bien(json!({})),
        "tools/list" => {
            let l: Vec<Value> = h.iter().map(|h| json!({ "name": h.nombre, "description": h.descripcion, "inputSchema": h.esquema })).collect();
            bien(json!({ "tools": l }))
        }
        "tools/call" => {
            let n = v["params"]["name"].as_str().unwrap_or("");
            let r = if h.iter().any(|h| h.nombre == n) {
                p.pedir(n, v["params"]["arguments"].clone())
            } else {
                Err(tf("no existe la herramienta «{}»", &[&n]))
            };
            let (s, mal) = match r {
                Ok(s) => (s, false),
                Err(s) => (s, true),
            };
            let parte = match s.strip_prefix(super::IMAGEN) {
                Some(x) => json!({ "type": "image", "data": x, "mimeType": "image/png" }),
                None => json!({ "type": "text", "text": s }),
            };
            bien(json!({ "content": [parte], "isError": mal }))
        }
        _ => Some(json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": "method not found" } })),
    }
}

/// Con este argumento el programa hace de puente: lee MCP por su entrada y lo pasa al servidor que ya está abierto.
pub const PUENTE: &str = "--mcp-puente";

/// Si se abrió como puente, atiende y termina. Se llama al empezar `main`.
pub fn puente() -> bool {
    if !std::env::args().any(|a| a == PUENTE) {
        return false;
    }
    let (Ok(url), Ok(llave)) = (std::env::var("BEETHOVEN_MCP_URL"), std::env::var("BEETHOVEN_MCP_LLAVE")) else { return true };
    // Solo a esta máquina.
    if !url.starts_with("http://127.0.0.1:") {
        return true;
    }
    let a: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(600))).http_status_as_error(false).build().into();
    use std::io::BufRead;
    for l in std::io::stdin().lock().lines().map_while(Result::ok).filter(|l| !l.trim().is_empty()) {
        let r = a.post(&url).header("Authorization", format!("Bearer {llave}")).header("content-type", "application/json").send(l);
        let s = r.ok().and_then(|mut r| r.body_mut().read_to_string().ok()).unwrap_or_default();
        if !s.trim().is_empty() {
            println!("{}", s.trim());
            let _ = std::io::stdout().flush();
        }
    }
    true
}
