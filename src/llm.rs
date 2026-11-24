//! Chat com um LLM local. Fala com o Ollama (API própria em `/api/chat`) ou
//! qualquer runtime HTTP compatível trocando `METRO_LLM_URL`.
//!
//! VRAM: o padrão é `num_gpu = 0`, ou seja, inferência em CPU e 0 MB de GPU.
//! Offload parcial só por escolha explícita em `METRO_LLM_GPU_LAYERS`.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde_json::{json, Value};

pub const SYSTEM: &str = "Você é o assistente do Metro Notch, um app de lembretes na borda da tela. \
Responda em português do Brasil, direto, no máximo 4 frases. \
Quando o conteúdo da tela for fornecido, use-o como fonte da verdade e diga o que está visível; \
se a tela não bastar, diga o que falta em vez de inventar.";

fn base() -> String {
    std::env::var("METRO_LLM_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:11434".into())
        .trim_end_matches('/')
        .to_string()
}

fn model() -> String {
    std::env::var("METRO_LLM_MODEL").unwrap_or_else(|_| "lfm3:1.7b".into())
}

fn ctx() -> u64 {
    std::env::var("METRO_LLM_CTX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4096)
}

fn gpu_layers() -> u64 {
    std::env::var("METRO_LLM_GPU_LAYERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

pub fn status() -> Value {
    match http("GET", "/api/tags", None, Duration::from_secs(3)) {
        Ok((_, body)) => {
            let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            let installed: Vec<String> = v["models"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|m| m["name"].as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            json!({
                "ok": installed.iter().any(|m| m == &model()),
                "runtime": base(),
                "model": model(),
                "installed": installed,
                "ctx": ctx(),
                "vram_mb": if gpu_layers() == 0 { json!(0) } else { Value::Null },
            })
        }
        Err(e) => json!({
            "ok": false,
            "runtime": base(),
            "model": model(),
            "installed": [],
            "error": e,
        }),
    }
}

pub fn chat(messages: Value) -> Result<String, String> {
    let body = json!({
        "model": model(),
        "messages": messages,
        "stream": false,
        "think": false,
        "options": { "num_ctx": ctx(), "num_gpu": gpu_layers(), "temperature": 0.4 },
    });
    let (code, text) = http(
        "POST",
        "/api/chat",
        Some(&body.to_string()),
        Duration::from_secs(180),
    )?;
    if code != 200 {
        return Err(format!("LLM respondeu {}: {}", code, head(&text, 300)));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let reply = v["message"]["content"].as_str().unwrap_or("").trim().to_string();
    if reply.is_empty() {
        return Err(format!("resposta vazia: {}", head(&text, 300)));
    }
    Ok(reply)
}

fn head(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// Cliente HTTP mínimo para `http://host:port` (o runtime é sempre local).
/// No release o processo não tem console, então nada de log aqui.
fn http(method: &str, path: &str, body: Option<&str>, timeout: Duration) -> Result<(u16, String), String> {
    let base = base();
    let rest = base
        .strip_prefix("http://")
        .ok_or("METRO_LLM_URL precisa começar com http://")?;
    let (host, port) = match rest.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().map_err(|_| "porta inválida")?),
        None => (rest, 80u16),
    };
    let mut sock = TcpStream::connect((host, port)).map_err(|e| format!("conectar {}: {}", base, e))?;
    let _ = sock.set_write_timeout(Some(Duration::from_secs(5)));
    let _ = sock.set_read_timeout(Some(timeout));
    let body = body.unwrap_or("");
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    sock.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    sock.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    let raw = String::from_utf8_lossy(&buf).to_string();
    let (headers, body) = raw.split_once("\r\n\r\n").ok_or("resposta HTTP inválida")?;
    let code = headers
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    Ok((code, body.to_string()))
}
