//! El asistente: servidores de mentira hacen de IA y de cliente MCP.
use beethoven_daw_demo::ia::{self, Clase, Evento, Herramienta, Platica, Proveedor, Puerta};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn herramientas() -> Vec<Herramienta> {
    vec![Herramienta { nombre: "sumar", descripcion: "sum", esquema: json!({ "type": "object" }) }]
}

/// Un programa que sabe sumar.
fn programa() -> Puerta {
    let (p, llamadas) = Puerta::nueva(Arc::new(|| {}));
    std::thread::spawn(move || {
        for l in llamadas {
            let r = match (l.datos["a"].as_i64(), l.datos["b"].as_i64()) {
                (Some(a), Some(b)) => Ok((a + b).to_string()),
                _ => Err("faltan números".into()),
            };
            l.responder(r);
        }
    });
    p
}

type Pedidos = Arc<Mutex<Vec<(String, Value)>>>;

/// Contesta en orden lo que se le da; guarda lo que le piden.
fn servidor(respuestas: Vec<(u16, Value)>) -> (String, Pedidos) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    let vistos: Pedidos = Default::default();
    let v = vistos.clone();
    std::thread::spawn(move || {
        for (estado, r) in respuestas {
            let (mut s, _) = l.accept().unwrap();
            let (cabeza, cuerpo) = leer(&mut s);
            v.lock().unwrap().push((cabeza, serde_json::from_str(&cuerpo).unwrap_or(Value::Null)));
            let r = r.to_string();
            let _ = write!(s, "HTTP/1.1 {estado} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{r}", r.len());
        }
    });
    (url, vistos)
}

fn leer(s: &mut TcpStream) -> (String, String) {
    let (mut b, mut x) = (vec![], [0u8; 4096]);
    loop {
        let n = s.read(&mut x).unwrap();
        b.extend_from_slice(&x[..n]);
        if let Some(i) = b.windows(4).position(|w| w == b"\r\n\r\n") {
            let cabeza = String::from_utf8_lossy(&b[..i]).to_lowercase();
            let largo: usize = cabeza.lines().find_map(|l| l.strip_prefix("content-length:")).map_or(0, |x| x.trim().parse().unwrap());
            while b.len() < i + 4 + largo {
                let n = s.read(&mut x).unwrap();
                b.extend_from_slice(&x[..n]);
            }
            return (cabeza, String::from_utf8_lossy(&b[i + 4..]).into_owned());
        }
        if n == 0 {
            return Default::default();
        }
    }
}

fn proveedor(clase: Clase, url: &str, modelo: &str, clave: &str) -> Proveedor {
    Proveedor {
        nombre: "x".into(),
        clase,
        url: url.into(),
        modelo: modelo.into(),
        clave: clave.into(),
        args: vec![],
        modelos: vec![],
        hallado: false,
    }
}

fn hablar(p: &mut Platica, q: &Proveedor, s: &str) -> Vec<Evento> {
    p.decir(q, s, None);
    let mut v = vec![];
    loop {
        let e = p.eventos.recv_timeout(Duration::from_secs(10)).expect("contesta");
        if e == Evento::Fin {
            return v;
        }
        v.push(e);
    }
}

#[test]
fn la_api_de_anthropic_usa_herramientas_y_recuerda() {
    // El bloque de pensamiento debe volver tal cual.
    let piensa = json!({ "type": "thinking", "thinking": "", "signature": "firma" });
    let (url, vistos) = servidor(vec![
        (
            200,
            json!({ "stop_reason": "tool_use", "content": [piensa, { "type": "text", "text": "Sumo" }, { "type": "tool_use", "id": "t1", "name": "sumar", "input": { "a": 2, "b": 3 } }, { "type": "tool_use", "id": "t2", "name": "sumar", "input": {} }] }),
        ),
        (200, json!({ "stop_reason": "end_turn", "content": [{ "type": "text", "text": "Son 5" }] })),
        (200, json!({ "stop_reason": "end_turn", "content": [{ "type": "text", "text": "Sí" }] })),
        (401, json!({ "error": { "message": "invalid x-api-key" } })),
        (200, json!({ "stop_reason": "refusal", "content": [] })),
    ]);
    let mut p = Platica::nueva("sistema".into(), herramientas(), programa());
    let q = proveedor(Clase::Anthropic, &url, "claude-opus-5-5", "llave-secreta");
    let e = hablar(&mut p, &q, "suma 2 y 3");
    assert_eq!(e, [Evento::Texto("Sumo".into()), Evento::Uso("sumar".into()), Evento::Uso("sumar".into()), Evento::Texto("Son 5".into())]);
    assert_eq!(hablar(&mut p, &q, "¿seguro?"), [Evento::Texto("Sí".into())]);
    let v = vistos.lock().unwrap().clone();
    assert!(v[0].0.contains("post /v1/messages") && v[0].0.contains("x-api-key: llave-secreta") && v[0].0.contains("anthropic-version: 2026-10-21"));
    assert_eq!((&v[0].1["model"], &v[0].1["tools"][0]["name"]), (&json!("claude-opus-5-5"), &json!("sumar")));
    assert!(v[0].1.get("temperature").is_none());
    let m = &v[1].1["messages"];
    assert_eq!((&m[1]["content"][0], &m[2]["content"][0]["content"]), (&piensa, &json!("5")));
    // Los dos resultados van juntos, y el que falló lo dice.
    assert_eq!((m[2]["content"].as_array().unwrap().len(), &m[2]["content"][1]["is_error"]), (2, &json!(true)));
    // La segunda pregunta lleva toda la plática.
    assert_eq!(v[2].1["messages"].as_array().unwrap().len(), 5);
    // Un error no deja la plática rota ni enseña la clave.
    let e = hablar(&mut p, &q, "otra");
    assert!(matches!(&e[..], [Evento::Error(s)] if s.contains("invalid x-api-key") && !s.contains("llave-secreta")), "{e:?}");
    assert!(matches!(&hablar(&mut p, &q, "mala")[..], [Evento::Error(_)]));
    assert_eq!(vistos.lock().unwrap()[4].1["messages"].as_array().unwrap().len(), 7);
}

#[test]
fn un_servidor_compatible_usa_herramientas() {
    let (url, vistos) = servidor(vec![
        (
            200,
            json!({ "choices": [{ "message": { "content": "", "tool_calls": [{ "id": "c1", "type": "function", "function": { "name": "sumar", "arguments": "{\"a\": 4, \"b\": 5}" } }, { "id": "c2", "type": "function", "function": { "name": "borrar_disco", "arguments": "{}" } }] } }] }),
        ),
        (200, json!({ "choices": [{ "message": { "role": "assistant", "content": "9" } }] })),
        (200, json!({ "data": [{ "id": "qwen3" }, { "id": "models/otro" }] })),
    ]);
    let mut p = Platica::nueva("sistema".into(), herramientas(), programa());
    let q = proveedor(Clase::Compatible, &format!("{url}/v1/"), "qwen3", "");
    let e = hablar(&mut p, &q, "4 más 5");
    assert_eq!(e.last(), Some(&Evento::Texto("9".into())));
    let v = vistos.lock().unwrap().clone();
    assert!(v[0].0.contains("post /v1/chat/completions") && !v[0].0.contains("authorization"));
    let m = v[1].1["messages"].as_array().unwrap();
    assert_eq!((&m[0]["role"], &m[3]["content"], &m[3]["tool_call_id"]), (&json!("system"), &json!("9"), &json!("c1")));
    assert!(m[4]["content"].as_str().unwrap().starts_with("ERROR"));
    assert_eq!(ia::modelos(&q).unwrap(), ["qwen3", "otro"]);
    // Nadie escucha: lo dice y termina.
    let q = proveedor(Clase::Compatible, "http://127.0.0.1:9/v1", "x", "");
    assert!(matches!(&hablar(&mut p, &q, "hola")[..], [Evento::Error(_)]));
}

fn pedir(puerto: u16, cabezas: &str, cuerpo: &str) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", puerto)).unwrap();
    write!(s, "POST /mcp HTTP/1.1\r\n{cabezas}Content-Length: {}\r\n\r\n{cuerpo}", cuerpo.len()).unwrap();
    let mut t = String::new();
    s.read_to_string(&mut t).unwrap();
    let estado = t.split(' ').nth(1).unwrap().parse().unwrap();
    (estado, t.split_once("\r\n\r\n").unwrap().1.to_string())
}

#[test]
fn el_servidor_mcp_contesta_y_se_cuida() {
    let s = ia::mcp::abrir(0, "llave-de-prueba-larga", Arc::new(herramientas()), programa()).unwrap();
    assert!(ia::mcp::abrir(0, "", Arc::new(herramientas()), programa()).is_err());
    let host = format!("Host: 127.0.0.1:{}\r\n", s.puerto);
    let bien = format!("{host}Authorization: Bearer llave-de-prueba-larga\r\n");
    let rpc = |m: &str, p: Value| json!({ "jsonrpc": "2.0", "id": 1, "method": m, "params": p }).to_string();
    let r = |x: (u16, String)| -> Value {
        assert_eq!(x.0, 200);
        serde_json::from_str(&x.1).unwrap()
    };
    let v = r(pedir(s.puerto, &bien, &rpc("initialize", json!({ "protocolVersion": "2025-06-18" }))));
    assert_eq!(v["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(pedir(s.puerto, &bien, &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string()).0, 202);
    assert_eq!(r(pedir(s.puerto, &bien, &rpc("tools/list", json!({}))))["result"]["tools"][0]["name"], "sumar");
    let v = r(pedir(s.puerto, &bien, &rpc("tools/call", json!({ "name": "sumar", "arguments": { "a": 20, "b": 22 } }))));
    assert_eq!((&v["result"]["content"][0]["text"], &v["result"]["isError"]), (&json!("42"), &json!(false)));
    let v = r(pedir(s.puerto, &bien, &rpc("tools/call", json!({ "name": "otra" }))));
    assert_eq!(v["result"]["isError"], true);
    assert_eq!(r(pedir(s.puerto, &bien, &rpc("nada", json!({}))))["error"]["code"], -32601);
    // Sin llave, con otra, desde una página web o con otro nombre de servidor: no.
    let m = rpc("tools/list", json!({}));
    assert_eq!(pedir(s.puerto, &host, &m).0, 401);
    assert_eq!(pedir(s.puerto, &format!("{host}Authorization: Bearer llave-de-prueba-largo\r\n"), &m).0, 401);
    assert_eq!(pedir(s.puerto, &format!("{bien}Origin: https://malo.com\r\n"), &m).0, 403);
    assert_eq!(pedir(s.puerto, "Host: malo.com\r\nAuthorization: Bearer llave-de-prueba-larga\r\n", &m).0, 403);
    assert_eq!(pedir(s.puerto, &bien, "{").0, 400);
    let puerto = s.puerto;
    drop(s);
    std::thread::sleep(Duration::from_millis(300));
    assert!(TcpStream::connect(("127.0.0.1", puerto)).is_err());
}

#[test]
fn las_claves_se_guardan_en_privado() {
    let r = std::env::temp_dir().join(format!("bdd-ia-{}.json", std::process::id()));
    let mut a = ia::Ajustes::leer(&r);
    assert_eq!(a.token.len(), 48);
    a.propios.push(proveedor(Clase::Anthropic, "https://api.anthropic.com", "claude-opus-5-5", "secreta"));
    a.guardar(&r).unwrap();
    #[cfg(unix)]
    assert_eq!(std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(&r).unwrap().permissions()) & 0o777, 0o600);
    let b = ia::Ajustes::leer(&r);
    assert_eq!((b.token, &b.propios[0].clave), (a.token, &a.propios[0].clave));
    std::fs::remove_file(r).unwrap();
}

/// Contesta con eventos, como la API de verdad.
fn servidor_sse(respuestas: Vec<String>) -> (String, Pedidos) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    let vistos: Pedidos = Default::default();
    let v = vistos.clone();
    std::thread::spawn(move || {
        for r in respuestas {
            let (mut s, _) = l.accept().unwrap();
            let (cabeza, cuerpo) = leer(&mut s);
            v.lock().unwrap().push((cabeza, serde_json::from_str(&cuerpo).unwrap_or(Value::Null)));
            let _ = write!(s, "HTTP/1.1 200 X\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n{r}");
        }
    });
    (url, vistos)
}

#[test]
fn la_respuesta_llega_en_vivo() {
    let ev = |v: Value| format!("event: x\ndata: {v}\n\n");
    let bloque = |i: u32, b: Value, d: Vec<Value>| {
        let mut s = ev(json!({ "type": "content_block_start", "index": i, "content_block": b }));
        d.into_iter().for_each(|d| s += &ev(json!({ "type": "content_block_delta", "index": i, "delta": d })));
        s + &ev(json!({ "type": "content_block_stop", "index": i }))
    };
    let fin = |r: &str| ev(json!({ "type": "message_delta", "delta": { "stop_reason": r } })) + &ev(json!({ "type": "message_stop" }));
    let uso = |datos: &str, i: u32| {
        bloque(
            i,
            json!({ "type": "tool_use", "id": format!("t{i}"), "name": "sumar", "input": {} }),
            datos.as_bytes().chunks(5).map(|c| json!({ "type": "input_json_delta", "partial_json": String::from_utf8_lossy(c) })).collect(),
        )
    };
    let primera = bloque(
        0,
        json!({ "type": "thinking", "thinking": "", "signature": "" }),
        vec![json!({ "type": "signature_delta", "signature": "fir" }), json!({ "type": "signature_delta", "signature": "ma" })],
    ) + &bloque(
        1,
        json!({ "type": "text", "text": "" }),
        vec![json!({ "type": "text_delta", "text": "Va" }), json!({ "type": "text_delta", "text": "mos" })],
    ) + &uso("{\"a\": 7, \"b\": 8}", 2)
        + &uso("{\"a\": 7, \"b", 3)
        + &fin("tool_use");
    let segunda = bloque(0, json!({ "type": "text", "text": "" }), vec![json!({ "type": "text_delta", "text": "15" })]) + &fin("end_turn");
    let cortada = bloque(0, json!({ "type": "text", "text": "" }), vec![json!({ "type": "text_delta", "text": "a medias" })]);
    let (url, vistos) = servidor_sse(vec![primera, segunda, cortada]);
    let mut p = Platica::nueva("sistema".into(), herramientas(), programa());
    let q = proveedor(Clase::Anthropic, &url, "claude-opus-5-5", "k");
    let e = hablar(&mut p, &q, "7 y 8");
    use Evento::*;
    let parte = |s: &str| Parte(s.into());
    // El que llegó mal escrito no se usa.
    assert_eq!(e, [parte("Va"), parte("mos"), Cierra, Uso("sumar".into()), parte("15"), Cierra]);
    let v = vistos.lock().unwrap().clone();
    assert_eq!((&v[0].1["stream"], &v[0].1["tools"][0]["eager_input_streaming"]), (&json!(true), &json!(true)));
    let m = &v[1].1["messages"];
    assert_eq!(
        (&m[1]["content"][0]["signature"], &m[1]["content"][1]["text"], &m[1]["content"][2]["input"]),
        (&json!("firma"), &json!("Vamos"), &json!({ "a": 7, "b": 8 }))
    );
    assert_eq!(
        (&m[2]["content"][0]["content"], &m[2]["content"][1]["content"], &m[2]["content"][1]["is_error"]),
        (&json!("15"), &json!("INVALID_JSON"), &json!(true))
    );
    // Cortada a la mitad: avisa.
    assert!(matches!(hablar(&mut p, &q, "x").last(), Some(Error(_))));
}

/// Un agente ACP de mentira: pide permiso para lo suyo y para una terminal, y apunta lo que se le contesta.
#[cfg(unix)]
#[test]
fn un_agente_acp_solo_usa_las_herramientas() {
    let d = std::env::temp_dir().join(format!("beethoven-acp-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let guion = r#"cd "$(dirname "$0")"; : > dicho
di() { printf '%s\n' "$1"; }
oye() { IFS= read -r l; printf '%s\n' "$l" >> dicho; }
permiso() { di '{"jsonrpc":"2.0","id":"'"$1"'","method":"session/request_permission","params":{"sessionId":"s","toolCall":'"$2"',"options":[{"optionId":"si","kind":"allow_once"},{"optionId":"no","kind":"reject_once"}]}}'; oye; }
oye; di 'ruido que no es json'; di '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"agentCapabilities":{"mcpCapabilities":{"http":true}}}}'
oye; di '{"jsonrpc":"2.0","id":2,"result":{"sessionId":"s","models":{"currentModelId":"uno","availableModels":[{"modelId":"uno"},{"modelId":"dos"}]}}}'
oye; di '{"jsonrpc":"2.0","id":3,"result":{}}'
oye
di '{"jsonrpc":"2.0","method":"session/update","params":{"update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"ho"}}}}'
di '{"jsonrpc":"2.0","method":"session/update","params":{"update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"la"}}}}'
di '{"jsonrpc":"2.0","method":"session/update","params":{"update":{"sessionUpdate":"tool_call","title":"daw_sumar"}}}'
permiso a '{"title":"daw_sumar","kind":"other"}'
permiso b '{"title":"sumar","kind":"execute"}'
permiso c '{"title":"bash","kind":"other","rawInput":{"command":"ls"}}'
permiso d '{"title":"daw_sumar","kind":"other","rawInput":{"command":"rm -rf x"}}'
permiso e '{"title":"echo daw_sumar; sumar","kind":"other"}'
di '{"jsonrpc":"2.0","id":"f","method":"fs/read_text_file","params":{"path":"/etc/passwd"}}'; oye
di '{"jsonrpc":"2.0","id":4,"result":{"stopReason":"end_turn"}}'
oye
"#;
    std::fs::write(d.join("agente.sh"), guion).unwrap();
    let p = Proveedor { args: vec![d.join("agente.sh").to_string_lossy().into()], ..proveedor(Clase::Acp, "/bin/sh", "dos", "") };
    let mut c = Platica::nueva("instrucciones".into(), herramientas(), programa());
    c.decir(&p, "hola", Some(ia::Enlace { url: "http://127.0.0.1:9/mcp".into(), token: "x".repeat(40), carpeta: d.join("c") }));
    let mut v = vec![];
    loop {
        match c.eventos.recv_timeout(Duration::from_secs(20)).unwrap() {
            Evento::Fin => break,
            e => v.push(e),
        }
    }
    let texto: String = v.iter().filter_map(|e| if let Evento::Parte(s) = e { Some(s.as_str()) } else { None }).collect();
    assert_eq!(texto, "hola", "{v:?}");
    assert!(v.contains(&Evento::Uso("sumar".into())) && !v.iter().any(|e| matches!(e, Evento::Error(_))), "{v:?}");
    let dicho: Vec<Value> = std::fs::read_to_string(d.join("dicho")).unwrap().lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    assert_eq!(dicho[0]["params"]["clientCapabilities"]["terminal"], false);
    assert_eq!(dicho[1]["params"]["mcpServers"][0]["headers"][0]["value"], format!("Bearer {}", "x".repeat(40)));
    assert_eq!((dicho[2]["method"].as_str(), dicho[2]["params"]["modelId"].as_str()), (Some("session/set_model"), Some("dos")));
    assert!(dicho[3]["params"]["prompt"][0]["text"].as_str().unwrap().starts_with("instrucciones"));
    let r: Vec<&str> = (4..9).map(|i| dicho[i]["result"]["outcome"]["optionId"].as_str().unwrap()).collect();
    assert_eq!(r, ["si", "no", "no", "no", "no"]);
    assert!(dicho[9]["error"].is_object(), "no se le prestan archivos");
    assert!(std::fs::read_to_string(d.join("c/agentes/opencode.json")).unwrap().contains("ask"));
    // Un programa que no es agente da un error que se entiende, sin quedarse esperando.
    let mut c = Platica::nueva("".into(), vec![], programa());
    c.decir(&proveedor(Clase::Acp, "/usr/bin/true", "", ""), "hola", Some(ia::Enlace { url: "".into(), token: "".into(), carpeta: d.join("c") }));
    assert!(matches!(c.eventos.recv_timeout(Duration::from_secs(20)).unwrap(), Evento::Error(_)));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn se_acomoda_lo_que_mandan_las_ia_chicas() {
    let h = vec![
        Herramienta {
            nombre: "poner",
            descripcion: "",
            esquema: json!({ "type": "object", "properties": {
                "n": { "type": "integer" }, "x": { "type": "number" }, "b": { "type": "boolean" }, "s": { "type": "string" },
                "l": { "type": "array", "items": { "type": "integer" } }, "c": { "type": "object", "additionalProperties": { "type": "number" } } } }),
        },
        Herramienta {
            nombre: "varias",
            descripcion: "",
            esquema: json!({ "type": "object", "properties": { "acciones": { "type": "array", "items": { "type": "object", "properties": { "datos": { "type": "object" } } } } } }),
        },
    ];
    let mal = json!({ "n": 3.0, "x": "1.5", "b": "true", "s": "12", "l": "[1, \"2\", 3.0]", "c": { "tono": "0.5" }, "otro": 1 });
    let bien = json!({ "n": 3, "x": 1.5, "b": true, "s": "12", "l": [1, 2, 3], "c": { "tono": 0.5 }, "otro": 1 });
    let mut d = mal.clone();
    ia::acomodar_llamada(&h, "poner", &mut d);
    assert_eq!(d, bien);
    let mut d = json!({ "acciones": json!([{ "herramienta": "poner", "datos": mal }]).to_string() });
    ia::acomodar_llamada(&h, "varias", &mut d);
    assert_eq!(d["acciones"][0]["datos"], bien);
    let mut d = json!({ "l": 4 });
    ia::acomodar_llamada(&h, "poner", &mut d);
    assert_eq!(d, json!({ "l": [4] }));
    for d in [json!(null), json!("x"), json!([1]), json!({ "n": null })] {
        ia::acomodar_llamada(&h, "poner", &mut d.clone());
    }
}
