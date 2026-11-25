// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! Por HTTP: la API de Anthropic y los servidores compatibles con OpenAI. Las herramientas corren aquí, en esta máquina.
use super::{Evento, Herramienta, MAX_VUELTAS, Proveedor, Puerta, de_red};
use crate::idioma::{t, tf};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::time::Duration;

pub const ANTHROPIC: &str = "https://api.anthropic.com";
pub const MODELO: &str = "claude-opus-5-5";
const VERSION: &str = "2026-10-21";
const MAX_RESPUESTA: u64 = 32 << 20;

fn agente(tiempo: Duration) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(tiempo)).http_status_as_error(false).build().into()
}

fn limpia(url: &str) -> &str {
    url.trim().trim_end_matches('/')
}

/// Lee la respuesta; si el servidor dijo que no, da su mensaje.
fn leer(r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<Value, String> {
    let mut r = r.map_err(de_red)?;
    let estado = r.status().as_u16();
    let s = r.body_mut().with_config().limit(MAX_RESPUESTA).read_to_string().map_err(de_red)?;
    let v: Value = serde_json::from_str(&s).unwrap_or(Value::Null);
    if (200..300).contains(&estado) {
        return if v.is_null() { Err(t("El servidor contestó algo que no se entiende").into()) } else { Ok(v) };
    }
    let dice = v["error"]["message"].as_str().or(v["error"].as_str()).unwrap_or("").chars().take(300).collect::<String>();
    Err(match estado {
        401 | 403 => tf("La clave no sirve o no tiene permiso ({})", &[&dice]),
        429 => tf("Se acabó el límite por ahora; espera un momento ({})", &[&dice]),
        _ => tf("El servidor dijo {}: {}", &[&estado, &dice]),
    })
}

pub fn modelos(url: &str, clave: &str, tiempo: Duration) -> Result<Vec<String>, String> {
    let mut r = agente(tiempo).get(format!("{}/models", limpia(url)));
    if !clave.is_empty() {
        r = r.header("Authorization", format!("Bearer {clave}"));
    }
    lista(leer(r.call())?)
}

pub fn modelos_anthropic(url: &str, clave: &str) -> Result<Vec<String>, String> {
    let r = agente(Duration::from_secs(10)).get(format!("{}/v1/models", limpia(url))).header("x-api-key", clave).header("anthropic-version", VERSION);
    lista(leer(r.call())?)
}

fn lista(v: Value) -> Result<Vec<String>, String> {
    let a = v["data"].as_array().ok_or_else(|| t("El servidor contestó algo que no se entiende").to_string())?;
    Ok(a.iter().filter_map(|m| Some(m["id"].as_str()?.trim_start_matches("models/").to_string())).collect())
}

pub struct Caso<'a> {
    pub p: &'a Proveedor,
    pub sistema: &'a str,
    pub herramientas: &'a [Herramienta],
    pub puerta: &'a Puerta,
    pub parar: &'a AtomicBool,
    pub di: &'a dyn Fn(Evento),
}

impl Caso<'_> {
    fn usar(&self, nombre: &str, datos: Value) -> Result<String, String> {
        (self.di)(Evento::Uso(nombre.into()));
        if !self.herramientas.iter().any(|h| h.nombre == nombre) {
            return Err(tf("no existe la herramienta «{}»", &[&nombre]));
        }
        self.puerta.pedir(nombre, datos)
    }

    /// La respuesta completa, diciendo el texto conforme llega.
    fn recibir(&self, r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<Value, String> {
        let mut r = r.map_err(de_red)?;
        let es = r.headers().get("content-type").and_then(|x| x.to_str().ok()).is_some_and(|x| x.contains("event-stream"));
        if !es {
            let v = leer(Ok(r))?;
            for b in v["content"].as_array().into_iter().flatten().filter(|b| b["type"] == "text") {
                (self.di)(Evento::Texto(b["text"].as_str().unwrap_or("").into()));
            }
            return Ok(v);
        }
        let mut v = json!({ "content": [], "stop_reason": null });
        let mut escrito: Vec<String> = vec![];
        for l in BufReader::new(r.body_mut().with_config().limit(MAX_RESPUESTA).reader()).lines() {
            if self.parar.load(Relaxed) {
                return Err(t("Detenido").into());
            }
            let l = l.map_err(de_red)?;
            let Some(e) = l.strip_prefix("data:").and_then(|d| serde_json::from_str::<Value>(d.trim()).ok()) else { continue };
            let i = e["index"].as_u64().unwrap_or(0) as usize;
            let bloques = v["content"].as_array_mut().unwrap();
            match e["type"].as_str().unwrap_or("") {
                "content_block_start" => {
                    bloques.push(e["content_block"].clone());
                    escrito.push(String::new());
                }
                "content_block_delta" if i < bloques.len() => {
                    let d = &e["delta"];
                    let pega = |b: &mut Value, k: &str| {
                        let x = format!("{}{}", b[k].as_str().unwrap_or(""), d[k].as_str().unwrap_or(""));
                        b[k] = json!(x);
                    };
                    match d["type"].as_str().unwrap_or("") {
                        "text_delta" => {
                            pega(&mut bloques[i], "text");
                            (self.di)(Evento::Parte(d["text"].as_str().unwrap_or("").into()));
                        }
                        "thinking_delta" => pega(&mut bloques[i], "thinking"),
                        "signature_delta" => pega(&mut bloques[i], "signature"),
                        "input_json_delta" => escrito[i].push_str(d["partial_json"].as_str().unwrap_or("")),
                        _ => {}
                    }
                }
                "content_block_stop" if i < bloques.len() => {
                    if bloques[i]["type"] == "text" {
                        (self.di)(Evento::Cierra);
                    }
                    if bloques[i]["type"] == "tool_use" && !escrito[i].trim().is_empty() {
                        bloques[i]["input"] = serde_json::from_str(&escrito[i]).unwrap_or(Value::Null);
                    }
                }
                "message_delta" => v["stop_reason"] = e["delta"]["stop_reason"].clone(),
                "error" => {
                    return Err(tf(
                        "El servidor dijo {}: {}",
                        &[&e["error"]["type"].as_str().unwrap_or(""), &e["error"]["message"].as_str().unwrap_or("")],
                    ));
                }
                "message_stop" => return Ok(v),
                _ => {}
            }
        }
        Err(t("La respuesta se cortó a la mitad").into())
    }

    /// La plática con la API de mensajes de Anthropic. Lo que contesta se guarda tal cual, con todos sus bloques.
    pub fn anthropic(&self, m: &mut Vec<Value>, mensaje: &str) -> Result<(), String> {
        if self.p.clave.trim().is_empty() {
            return Err(t("Falta la clave de la API").into());
        }
        m.push(json!({ "role": "user", "content": mensaje }));
        let herramientas: Vec<Value> = self
            .herramientas
            .iter()
            .map(|h| json!({ "name": h.nombre, "description": h.descripcion, "input_schema": h.esquema, "eager_input_streaming": true }))
            .collect();
        let base = if limpia(&self.p.url).is_empty() { ANTHROPIC } else { limpia(&self.p.url) };
        let modelo = if self.p.modelo.trim().is_empty() { MODELO } else { self.p.modelo.trim() };
        for _ in 0..MAX_VUELTAS {
            if self.parar.load(Relaxed) {
                return Err(t("Detenido").into());
            }
            let mut cuerpo = json!({
                "model": modelo,
                "max_tokens": 16000,
                "system": [{ "type": "text", "text": self.sistema, "cache_control": { "type": "ephemeral" } }],
                "tools": herramientas,
                "messages": m,
                "stream": true,
            });
            let mut r = agente(Duration::from_secs(600))
                .post(format!("{base}/v1/messages"))
                .header("x-api-key", self.p.clave.trim())
                .header("anthropic-version", VERSION)
                .header("content-type", "application/json");
            // Si un filtro rechaza la petición, otro modelo la atiende en la misma llamada.
            if base == ANTHROPIC && modelo == MODELO {
                cuerpo["fallbacks"] = json!("default");
                r = r.header("anthropic-beta", "server-side-fallback-2026-07-01");
            }
            let v = self.recibir(r.send(cuerpo.to_string()))?;
            let bloques = v["content"].as_array().cloned().unwrap_or_default();
            let alto = v["stop_reason"].as_str().unwrap_or("");
            if alto == "refusal" {
                return Err(t("La IA no quiso hacer eso").into());
            }
            let mut dicho = bloques.clone();
            for b in dicho.iter_mut().filter(|b| b["type"] == "tool_use" && !b["input"].is_object()) {
                b["input"] = json!({});
            }
            m.push(json!({ "role": "assistant", "content": dicho }));
            match alto {
                "tool_use" => {}
                "pause_turn" => continue,
                "max_tokens" => return Err(t("La respuesta quedó cortada: pide algo más corto").into()),
                _ => return Ok(()),
            }
            let hechas: Vec<Value> = bloques
                .iter()
                .filter(|b| b["type"] == "tool_use")
                .map(|b| {
                    // Los datos llegan por partes: si quedaron mal escritos no se usan.
                    let datos = if b["input"].is_object() { Ok(b["input"].clone()) } else { Err("INVALID_JSON".to_string()) };
                    let (texto, mal) = match datos.and_then(|d| self.usar(b["name"].as_str().unwrap_or(""), d)) {
                        Ok(s) => (s, false),
                        Err(s) => (s, true),
                    };
                    let texto = match texto.strip_prefix(super::IMAGEN) {
                        Some(x) => json!([{ "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": x } }]),
                        None => json!(texto),
                    };
                    json!({ "type": "tool_result", "tool_use_id": b["id"], "content": texto, "is_error": mal })
                })
                .collect();
            m.push(json!({ "role": "user", "content": hechas }));
        }
        Err(t("Fueron demasiados pasos para una sola petición").into())
    }

    /// La plática con un servidor que habla como el de OpenAI.
    pub fn compatible(&self, m: &mut Vec<Value>, mensaje: &str) -> Result<(), String> {
        if self.p.modelo.trim().is_empty() {
            return Err(t("Falta elegir el modelo").into());
        }
        if m.is_empty() {
            m.push(json!({ "role": "system", "content": self.sistema }));
        }
        m.push(json!({ "role": "user", "content": mensaje }));
        let herramientas: Vec<Value> = self
            .herramientas
            .iter()
            .map(|h| json!({ "type": "function", "function": { "name": h.nombre, "description": h.descripcion, "parameters": h.esquema } }))
            .collect();
        for _ in 0..MAX_VUELTAS {
            if self.parar.load(Relaxed) {
                return Err(t("Detenido").into());
            }
            let cuerpo = json!({ "model": self.p.modelo.trim(), "messages": m, "tools": herramientas });
            let mut r =
                agente(Duration::from_secs(600)).post(format!("{}/chat/completions", limpia(&self.p.url))).header("content-type", "application/json");
            if !self.p.clave.trim().is_empty() {
                r = r.header("Authorization", format!("Bearer {}", self.p.clave.trim()));
            }
            let v = leer(r.send(cuerpo.to_string()))?;
            let mut dice = v["choices"][0]["message"].clone();
            if !dice.is_object() {
                return Err(t("El servidor contestó algo que no se entiende").into());
            }
            dice["role"] = json!("assistant");
            if let Some(s) = dice["content"].as_str().filter(|s| !s.trim().is_empty()) {
                (self.di)(Evento::Texto(s.into()));
            }
            let usos = dice["tool_calls"].as_array().cloned().unwrap_or_default();
            m.push(dice);
            if usos.is_empty() {
                return Ok(());
            }
            for u in usos {
                let f = &u["function"];
                // Lo normal es texto con JSON; algunos servidores ya mandan el objeto.
                let datos = match &f["arguments"] {
                    Value::String(s) if s.trim().is_empty() => Ok(json!({})),
                    Value::String(s) => serde_json::from_str(s).map_err(|e| e.to_string()),
                    v => Ok(v.clone()),
                };
                let r = datos.and_then(|d| self.usar(f["name"].as_str().unwrap_or(""), d));
                let texto = r.unwrap_or_else(|e| format!("ERROR: {e}"));
                let texto = if texto.starts_with(super::IMAGEN) { "(image: this server cannot show it)".to_string() } else { texto };
                m.push(json!({ "role": "tool", "tool_call_id": u["id"], "content": texto }));
            }
        }
        Err(t("Fueron demasiados pasos para una sola petición").into())
    }
}
