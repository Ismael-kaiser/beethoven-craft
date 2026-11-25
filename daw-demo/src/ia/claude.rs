// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Claude Code: se usa el que está instalado, con la cuenta de quien lo instaló. Va sin sus herramientas de archivos ni
//! de terminal: solo puede usar las del programa, que le llegan por MCP.
use super::{Enlace, Evento, Proveedor, privado};
use crate::idioma::{t, tf};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

struct Borra<'a>(&'a std::path::Path);

impl Drop for Borra<'_> {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.0);
    }
}

pub fn decir(
    p: &Proveedor,
    sistema: &str,
    mensaje: &str,
    sesion: &mut Option<String>,
    e: &Enlace,
    parar: &AtomicBool,
    di: &dyn Fn(Evento),
) -> Result<(), String> {
    std::fs::create_dir_all(&e.carpeta).map_err(|x| x.to_string())?;
    // La llave del servidor va en un archivo privado y no en la línea de comando, que cualquiera puede ver.
    let conf = e.carpeta.join("mcp.json");
    let v = json!({ "mcpServers": { "daw": { "type": "http", "url": e.url, "headers": { "Authorization": format!("Bearer {}", e.token) } } } });
    privado(&conf, v.to_string().as_bytes())?;
    // La llave no se queda ahí al terminar.
    let _borra = Borra(&conf);
    let mut c = Command::new(&p.url);
    c.args([
        "-p",
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
        "--strict-mcp-config",
        "--tools",
        "",
        "--allowedTools",
        "mcp__daw",
        // Sin las instrucciones ni los ajustes personales de quien lo instaló: no son para este trabajo.
        "--setting-sources",
        "",
        "--effort",
        "high",
    ]);
    c.arg("--mcp-config").arg(&conf).arg("--system-prompt").arg(sistema);
    if !p.modelo.trim().is_empty() {
        c.arg("--model").arg(p.modelo.trim());
    }
    if let Some(s) = sesion.as_ref() {
        c.arg("--resume").arg(s);
    }
    // Aparte de cualquier proyecto, y sin creerse dentro de otra sesión.
    c.current_dir(&e.carpeta).env_remove("CLAUDECODE").env_remove("CLAUDE_CODE_ENTRYPOINT");
    // Que espere al servidor y ponga todas las herramientas a la vista: su buscador va apagado.
    c.env("MCP_CONNECTION_NONBLOCKING", "false").env("ENABLE_TOOL_SEARCH", "false");
    let mut h =
        c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|x| tf("No se pudo abrir Claude Code: {}", &[&x]))?;
    if let Some(mut s) = h.stdin.take() {
        let _ = s.write_all(mensaje.as_bytes());
    }
    let fallas = h.stderr.take().map(|s| {
        std::thread::spawn(move || {
            let mut t = String::new();
            let _ = s.take(8192).read_to_string(&mut t);
            t
        })
    });
    let sale = h.stdout.take().ok_or("sin salida")?;
    let (manda, lineas) = crossbeam_channel::unbounded();
    std::thread::spawn(move || {
        for l in BufReader::new(sale).lines().map_while(Result::ok) {
            if manda.send(l).is_err() {
                break;
            }
        }
    });
    let mut fin: Option<Result<(), String>> = None;
    let mut parcial = false;
    loop {
        if parar.load(Relaxed) {
            let _ = h.kill();
            let _ = h.wait();
            return Err(t("Detenido").into());
        }
        match lineas.recv_timeout(std::time::Duration::from_millis(150)) {
            Ok(l) => {
                let Ok(v) = serde_json::from_str::<Value>(&l) else { continue };
                if let Some(s) = v["session_id"].as_str() {
                    *sesion = Some(s.into());
                }
                match v["type"].as_str() {
                    Some("system") if v["subtype"] == "init" => {
                        let llegaron = v["tools"].as_array().is_some_and(|t| t.iter().any(|x| x.as_str().is_some_and(|x| x.starts_with("mcp__daw"))));
                        if !llegaron {
                            let _ = h.kill();
                            let _ = h.wait();
                            return Err(t("Claude Code no pudo conectarse con las herramientas del programa").into());
                        }
                    }
                    Some("stream_event") => {
                        let e = &v["event"];
                        if e["delta"]["type"] == "text_delta" {
                            parcial = true;
                            di(Evento::Parte(e["delta"]["text"].as_str().unwrap_or("").into()));
                        }
                    }
                    Some("assistant") => {
                        for b in v["message"]["content"].as_array().into_iter().flatten() {
                            match b["type"].as_str() {
                                // Ya se dijo por partes.
                                Some("text") if std::mem::take(&mut parcial) => di(Evento::Cierra),
                                Some("text") => di(Evento::Texto(b["text"].as_str().unwrap_or("").into())),
                                Some("tool_use") => di(Evento::Uso(b["name"].as_str().unwrap_or("").trim_start_matches("mcp__daw__").into())),
                                _ => {}
                            }
                        }
                    }
                    Some("result") => {
                        let mal = v["is_error"].as_bool().unwrap_or(false);
                        let s = v["result"].as_str().unwrap_or("").to_string();
                        fin = Some(if mal { Err(s) } else { Ok(()) });
                    }
                    _ => {}
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(_) => break,
        }
    }
    let estado = h.wait().map_err(|x| x.to_string())?;
    match fin {
        Some(r) => r,
        None if estado.success() => Ok(()),
        None => {
            let f = fallas.and_then(|h| h.join().ok()).unwrap_or_default();
            Err(tf("Claude Code se cerró: {}", &[&f.trim().chars().take(300).collect::<String>()]))
        }
    }
}
