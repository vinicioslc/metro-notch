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
    std::env::var("METRO_LLM_MODEL").unwrap_or_else(|_| "lfm3-omni:latest".into())
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
        "keep_alive": "30m",
        "options": { "num_ctx": ctx(), "num_gpu": gpu_layers(), "temperature": 0.4, "num_predict": 180 },
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

/// Transcreve áudio com o modelo omni via endpoint OpenAI-compatível do Ollama.
/// `format` é "wav", "webm", "ogg" etc.; webm/ogg são convertidos para wav com ffmpeg.
pub fn transcribe(audio_b64: &str, format: &str) -> Result<String, String> {
    let wav_b64 = normalize_audio(audio_b64, format)?;
    let body = json!({
        "model": model(),
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "Transcreva este áudio fielmente em português. Responda apenas com a transcrição."},
                {"type": "input_audio", "input_audio": {"data": wav_b64, "format": "wav"}}
            ]
        }],
        "stream": false,
        "max_tokens": 300,
        "temperature": 0
    });
    let (code, text) = http(
        "POST",
        "/v1/chat/completions",
        Some(&body.to_string()),
        Duration::from_secs(240),
    )?;
    if code != 200 {
        return Err(format!("LLM respondeu {}: {}", code, head(&text, 300)));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let reply = v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();
    if reply.is_empty() {
        return Err(format!("resposta vazia: {}", head(&text, 300)));
    }
    Ok(reply)
}

fn normalize_audio(audio_b64: &str, format: &str) -> Result<String, String> {
    let fmt = format.to_ascii_lowercase();
    if fmt == "wav" || fmt.is_empty() {
        return Ok(audio_b64.to_string());
    }
    let bytes = base64_decode(audio_b64)?;
    let dir = std::env::temp_dir();
    let stem = format!("metro-audio-{}", std::process::id());
    let src = dir.join(format!("{}.{}", stem, if fmt == "webm" { "webm" } else { "ogg" }));
    let dst = dir.join(format!("{}-out.wav", stem));
    std::fs::write(&src, &bytes).map_err(|e| e.to_string())?;
    let mut cmd = std::process::Command::new("ffmpeg");
    cmd.args(["-y", "-i"])
        .arg(&src)
        .args(["-ac", "1", "-ar", "16000", "-f", "wav"])
        .arg(&dst);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().map_err(|e| format!("ffmpeg: {}", e))?;
    let _ = std::fs::remove_file(&src);
    if !out.status.success() {
        let _ = std::fs::remove_file(&dst);
        return Err(format!("ffmpeg falhou: {}", head(&String::from_utf8_lossy(&out.stderr), 200)));
    }
    let wav = std::fs::read(&dst).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&dst);
    Ok(base64_encode(&wav))
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for b in s.bytes() {
        let val = match b {
            b'A'..=b'Z' => (b - b'A') as u32,
            b'a'..=b'z' => (b - b'a' + 26) as u32,
            b'0'..=b'9' => (b - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\n' | b'\r' | b' ' | b'\t' => continue,
            _ => return Err("base64 inválido".into()),
        };
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xFF) as u8);
        }
    }
    Ok(out)
}

fn base64_encode(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    let mut i = 0;
    while i + 2 < bytes.len() {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8) | (bytes[i + 2] as u32);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push(T[(n & 63) as usize] as char);
        i += 3;
    }
    let rem = bytes.len() - i;
    if rem == 1 {
        let n = (bytes[i] as u32) << 16;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push('=');
    }
    out
}

/// Monta a conversa enviada ao modelo: sistema (+ texto da tela) e depois o
/// histórico. A tela entra como system para não virar pedido do usuário.
pub fn messages(mut msgs: Vec<Value>, screen: bool) -> Value {
    let mut system = SYSTEM.to_string();
    if screen {
        if let Ok(text) = crate::screen::read(4000) {
            if !text.trim().is_empty() {
                system.push_str("\n\n--- conteúdo da tela agora ---\n");
                system.push_str(&text);
            }
        }
    }
    let mut out = vec![json!({"role": "system", "content": system})];
    out.append(&mut msgs);
    Value::Array(out)
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
