// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Agentes con ACP (Agent Client Protocol): OpenCode, Cline, Gemini CLI y los que sigan la misma norma. Se abre el que
//! está instalado, con su propia cuenta y su modelo. Aquí se decide qué permisos tiene: solo las herramientas del
//! programa; nada de archivos ni de terminal.
use super::{Enlace, Evento, Proveedor, ruta_de_programas};
use crate::idioma::{t, tf};
use crossbeam_channel::{Receiver, RecvTimeoutError, unbounded};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::time::{Duration, Instant};

/// Nombre, programa y cómo se le pide ACP.
pub const CONOCIDOS: &[(&str, &str, &[&str])] = &[
    ("OpenCode", "opencode", &["acp"]),
    ("Cline", "cline", &["--acp", "--auto-approve", "false"]),
    ("Gemini CLI", "gemini", &["--acp"]),
    ("Codex", "codex-acp", &[]),
    ("Goose", "goose", &["acp"]),
    ("Qwen Code", "qwen", &["--acp"]),
    ("Kimi", "kimi", &["acp"]),
    ("Copilot CLI", "copilot", &["--acp"]),
    ("Cursor", "cursor-agent", &["acp"]),
    ("Kilo Code", "kilo", &["acp"]),
    ("Auggie", "auggie", &["--acp"]),
];

/// Un agente abierto, con su sesión.
pub struct Vivo {
    h: Child,
    entra: ChildStdin,
    lineas: Receiver<String>,
    fallas: std::sync::Arc<std::sync::Mutex<String>>,
    sesion: String,
    sig: u64,
    /// El modelo se cambia con una opción de la sesión (y no con su propio mensaje).
    por_opcion: Option<String>,
    pub modelos: Vec<String>,
    pub actual: String,
    nuevo: bool,
}

impl Drop for Vivo {
    fn drop(&mut self) {
        let _ = self.h.kill();
        let _ = self.h.wait();
    }
}

/// ¿Se deja? Solo si es una herramienta del programa y no algo de archivos, red o terminal.
pub fn permitir(u: &Value, herramientas: &[&str]) -> bool {
    let c = &u["toolCall"];
    let clase = c["kind"].as_str().unwrap_or("other");
    if !matches!(clase, "other" | "think") {
        return false;
    }
    // Lo que trae un comando es una terminal, se llame como se llame.
    if ["command", "commands", "cmd", "script"].iter().any(|k| !c["rawInput"][k].is_null()) {
        return false;
    }
    // El nombre completo, solo o con el del servidor: nunca «que lo contenga», porque el título puede ser cualquier cosa.
    let es = |s: &str| {
        let s = s.trim();
        herramientas.iter().any(|h| {
            s == *h
                || ["daw_", "daw.", "daw/", "daw:", "daw__", "mcp__daw__", "mcp_daw_", "mcp.daw."].iter().any(|a| s.strip_prefix(a) == Some(*h))
                || [" (daw MCP Server)", " (daw)", " - daw"].iter().any(|b| s.strip_suffix(b) == Some(*h))
        })
    };
    ["name", "title", "toolName"].iter().any(|k| c[k].as_str().is_some_and(es))
}

impl Vivo {
    pub fn abrir(p: &Proveedor, e: Option<&Enlace>, carpeta: &std::path::Path) -> Result<Vivo, String> {
        // Una carpeta solo para esto: algunos agentes leen lo que haya donde trabajan.
        let carpeta = &carpeta.join("agentes");
        std::fs::create_dir_all(carpeta).map_err(|x| x.to_string())?;
        // OpenCode deja todo por omisión: con esto pregunta, y aquí se le contesta.
        let _ = std::fs::write(carpeta.join("opencode.json"), r#"{"$schema":"https://opencode.ai/config.json","permission":"ask"}"#);
        let mut c = Command::new(&p.url);
        c.args(&p.args).current_dir(carpeta).env("PATH", ruta_de_programas()).env_remove("CLAUDECODE").env_remove("CLAUDE_CODE_ENTRYPOINT");
        let mut h = c
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|x| tf("No se pudo abrir {}: {}", &[&p.nombre, &x]))?;
        let (entra, sale, mal) = (h.stdin.take(), h.stdout.take(), h.stderr.take());
        let (Some(entra), Some(sale)) = (entra, sale) else { return Err(tf("No se pudo abrir {}: {}", &[&p.nombre, &""])) };
        let (manda, lineas) = unbounded();
        std::thread::spawn(move || {
            for l in BufReader::new(sale).lines().map_while(Result::ok) {
                if manda.send(l).is_err() {
                    break;
                }
            }
        });
        let fallas: std::sync::Arc<std::sync::Mutex<String>> = Default::default();
        if let Some(mut m) = mal {
            let f = fallas.clone();
            std::thread::spawn(move || {
                let mut b = [0u8; 1024];
                while let Ok(n) = m.read(&mut b) {
                    if n == 0 {
                        break;
                    }
                    let mut s = f.lock().unwrap_or_else(|e| e.into_inner());
                    s.push_str(&String::from_utf8_lossy(&b[..n]));
                    let sobra = s.len().saturating_sub(4000);
                    let corte = (sobra..s.len()).find(|i| s.is_char_boundary(*i)).unwrap_or(0);
                    s.drain(..corte);
                }
            });
        }
        let mut v =
            Vivo { h, entra, lineas, fallas, sesion: String::new(), sig: 0, por_opcion: None, modelos: vec![], actual: String::new(), nuevo: true };
        let parar = AtomicBool::new(false);
        let tope = Duration::from_secs(45);
        let r = v.pedir(
            "initialize",
            json!({ "protocolVersion": 1, "clientCapabilities": { "fs": { "readTextFile": false, "writeTextFile": false }, "terminal": false }, "clientInfo": { "name": env!("CARGO_PKG_NAME"), "version": env!("CARGO_PKG_VERSION") } }),
            tope,
            &parar,
            &mut |_| {},
            &[],
        )?;
        let http = r["agentCapabilities"]["mcpCapabilities"]["http"].as_bool().unwrap_or(false);
        let servidores = match e {
            None => json!([]),
            Some(e) if http => {
                json!([{ "type": "http", "name": "daw", "url": e.url, "headers": [{ "name": "Authorization", "value": format!("Bearer {}", e.token) }] }])
            }
            // El que no sabe MCP por HTTP usa este mismo programa como puente.
            Some(e) => {
                let yo = std::env::var_os("BEETHOVEN_PUENTE").map(Into::into).or(std::env::current_exe().ok()).unwrap_or_default();
                json!([{ "name": "daw", "command": yo.to_string_lossy(), "args": [super::mcp::PUENTE],
                    "env": [{ "name": "BEETHOVEN_MCP_URL", "value": e.url }, { "name": "BEETHOVEN_MCP_LLAVE", "value": e.token }] }])
            }
        };
        let r = v.pedir("session/new", json!({ "cwd": carpeta, "mcpServers": servidores }), Duration::from_secs(90), &parar, &mut |_| {}, &[])?;
        v.sesion = r["sessionId"].as_str().unwrap_or("").into();
        if let Some(m) = r["models"]["availableModels"].as_array() {
            v.modelos = m.iter().filter_map(|x| Some(x["modelId"].as_str()?.to_string())).collect();
            v.actual = r["models"]["currentModelId"].as_str().unwrap_or("").into();
        } else if let Some(o) = r["configOptions"].as_array().into_iter().flatten().find(|o| o["category"] == "model" || o["id"] == "model") {
            // Pueden venir en grupos.
            let mut hojas = vec![];
            for x in o["options"].as_array().into_iter().flatten() {
                match x["options"].as_array() {
                    Some(g) => hojas.extend(g.iter()),
                    None => hojas.push(x),
                }
            }
            v.modelos = hojas.iter().filter_map(|x| Some(x["value"].as_str()?.to_string())).collect();
            v.actual = o["currentValue"].as_str().unwrap_or("").into();
            v.por_opcion = o["id"].as_str().map(String::from);
        }
        Ok(v)
    }

    fn mandar(&mut self, v: Value) -> Result<(), String> {
        writeln!(self.entra, "{v}").and_then(|_| self.entra.flush()).map_err(|_| self.se_cerro())
    }

    fn se_cerro(&self) -> String {
        let f = self.fallas.lock().unwrap_or_else(|e| e.into_inner());
        let f: String = f.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").chars().take(300).collect();
        tf("El programa de la IA se cerró: {}", &[&f])
    }

    /// Manda una petición y atiende lo que el agente diga o pida mientras contesta.
    fn pedir(
        &mut self,
        metodo: &str,
        datos: Value,
        tope: Duration,
        parar: &AtomicBool,
        dice: &mut dyn FnMut(&Value),
        herramientas: &[&str],
    ) -> Result<Value, String> {
        self.sig += 1;
        let id = self.sig;
        self.mandar(json!({ "jsonrpc": "2.0", "id": id, "method": metodo, "params": datos }))?;
        let mut ultimo = Instant::now();
        let mut cancelado = false;
        loop {
            if parar.load(Relaxed) && !cancelado {
                cancelado = true;
                let s = self.sesion.clone();
                self.mandar(json!({ "jsonrpc": "2.0", "method": "session/cancel", "params": { "sessionId": s } }))?;
                ultimo = Instant::now();
            }
            if cancelado && ultimo.elapsed() > Duration::from_secs(5) {
                return Err(t("Detenido").into());
            }
            let l = match self.lineas.recv_timeout(Duration::from_millis(150)) {
                Ok(l) => l,
                Err(RecvTimeoutError::Timeout) if ultimo.elapsed() < tope => continue,
                Err(RecvTimeoutError::Timeout) => return Err(t("La IA tardó demasiado en contestar").into()),
                Err(_) => return Err(self.se_cerro()),
            };
            ultimo = Instant::now();
            let Ok(v) = serde_json::from_str::<Value>(&l) else { continue };
            match (v["method"].as_str(), v.get("id")) {
                (Some("session/update"), _) => dice(&v["params"]["update"]),
                (Some("session/request_permission"), Some(i)) => {
                    let si = permitir(&v["params"], herramientas) && !cancelado;
                    let busca = |k: &str| {
                        v["params"]["options"].as_array().into_iter().flatten().find(|o| o["kind"].as_str() == Some(k)).map(|o| o["optionId"].clone())
                    };
                    let o = if si { busca("allow_once").or(busca("allow_always")) } else { busca("reject_once").or(busca("reject_always")) };
                    let r = match o {
                        Some(o) if !cancelado => json!({ "outcome": { "outcome": "selected", "optionId": o } }),
                        _ => json!({ "outcome": { "outcome": "cancelled" } }),
                    };
                    self.mandar(json!({ "jsonrpc": "2.0", "id": i, "result": r }))?;
                }
                // Archivos, terminales y lo demás: no se prestan.
                (Some(_), Some(i)) => self.mandar(json!({ "jsonrpc": "2.0", "id": i, "error": { "code": -32601, "message": "not available" } }))?,
                (Some(_), None) => {}
                (None, Some(i)) if i.as_u64() == Some(id) => {
                    if v["error"].is_object() {
                        let m = v["error"]["message"].as_str().unwrap_or("").chars().take(300).collect::<String>();
                        return Err(if v["error"]["code"] == -32000 || m.to_lowercase().contains("auth") {
                            tf("Esa IA pide iniciar sesión: ábrela en la terminal, o elige otro modelo ({})", &[&m])
                        } else {
                            m
                        });
                    }
                    return Ok(v["result"].clone());
                }
                _ => {}
            }
        }
    }

    fn poner_modelo(&mut self, m: &str, parar: &AtomicBool) -> Result<(), String> {
        if m.is_empty() || m == self.actual {
            return Ok(());
        }
        let s = self.sesion.clone();
        let (metodo, datos) = match &self.por_opcion {
            Some(o) => ("session/set_config_option", json!({ "sessionId": s, "configId": o, "value": m })),
            None => ("session/set_model", json!({ "sessionId": s, "modelId": m })),
        };
        self.pedir(metodo, datos, Duration::from_secs(30), parar, &mut |_| {}, &[])?;
        self.actual = m.into();
        Ok(())
    }

    pub fn decir(
        &mut self,
        p: &Proveedor,
        sistema: &str,
        mensaje: &str,
        herramientas: &[&str],
        parar: &AtomicBool,
        di: &dyn Fn(Evento),
    ) -> Result<(), String> {
        self.poner_modelo(p.modelo.trim(), parar)?;
        // No hay lugar para instrucciones: van con el primer mensaje.
        let texto = if std::mem::take(&mut self.nuevo) {
            format!("{sistema}\n\nYou only have the tools of the server \"daw\"; do not use files, the terminal or the web.\n\n{mensaje}")
        } else {
            mensaje.to_string()
        };
        let mut hablo = false;
        let mut dice = |u: &Value| match u["sessionUpdate"].as_str() {
            Some("agent_message_chunk") => {
                if let Some(s) = u["content"]["text"].as_str() {
                    hablo = true;
                    di(Evento::Parte(s.into()));
                }
            }
            Some("tool_call") => {
                if std::mem::take(&mut hablo) {
                    di(Evento::Cierra);
                }
                let s = u["name"].as_str().or(u["title"].as_str()).unwrap_or("");
                // Solo el nombre de la herramienta, si es de las nuestras.
                let n = herramientas.iter().filter(|h| s.contains(**h)).max_by_key(|h| h.len()).map_or(s, |h| h);
                di(Evento::Uso(n.chars().take(60).collect()));
            }
            _ => {}
        };
        let s = self.sesion.clone();
        let r = self.pedir(
            "session/prompt",
            json!({ "sessionId": s, "prompt": [{ "type": "text", "text": texto }] }),
            Duration::from_secs(900),
            parar,
            &mut dice,
            herramientas,
        );
        di(Evento::Cierra);
        match r?["stopReason"].as_str() {
            Some("refusal") => Err(t("La IA no quiso hacer eso").into()),
            Some("max_tokens" | "max_turn_requests") => Err(t("La respuesta quedó cortada: pide algo más corto").into()),
            Some("cancelled") => Err(t("Detenido").into()),
            _ => Ok(()),
        }
    }
}
