use std::io::{Read, Write};
use std::net::TcpListener;
// (no direct sync imports; SharedState arrives as Arc<Mutex<_>>)
use std::time::Duration;

use serde_json::{json, Value};

use crate::{SharedState, UserEvent};
use tao::event_loop::EventLoopProxy;

pub fn start(shared: SharedState, proxy: EventLoopProxy<UserEvent>, port: u16) {
    std::thread::spawn(move || {
        let bind = std::env::var("NOTCH_BIND").unwrap_or_else(|_| "127.0.0.1".into());
        let listener = match TcpListener::bind(format!("{}:{}", bind, port)) {
            Ok(l) => l,
            Err(_) => return,
        };
        for stream in listener.incoming() {
            if let Ok(s) = stream {
                let sh = shared.clone();
                let pr = proxy.clone();
                std::thread::spawn(move || handle(s, sh, pr));
            }
        }
    });
}

fn handle(mut stream: std::net::TcpStream, shared: SharedState, proxy: EventLoopProxy<UserEvent>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => break,
        }
        if buf.len() > 1 << 20 {
            return;
        }
    }
    let header_end = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
        .unwrap_or(buf.len());
    let headers_str = String::from_utf8_lossy(&buf[..header_end]);
    let mut lines = headers_str.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").to_string();

    let mut content_length = 0usize;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_length = v.trim().parse().unwrap_or(0);
            }
        }
    }

    let mut body_bytes = buf[header_end..].to_vec();
    while body_bytes.len() < content_length {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => body_bytes.extend_from_slice(&tmp[..n]),
            Err(_) => break,
        }
    }
    let body = String::from_utf8_lossy(&body_bytes[..content_length.min(body_bytes.len())]).to_string();

    if method == "OPTIONS" {
        let resp = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET,POST,PATCH,DELETE,OPTIONS\r\nAccess-Control-Allow-Headers: *\r\nConnection: close\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    let (status, resp_body, ctype) = route(&method, &path, &body, &shared, &proxy);
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        status, ctype, resp_body.len(), resp_body
    );
    let _ = stream.write_all(response.as_bytes());
}

fn route(method: &str, path: &str, body: &str, shared: &SharedState, proxy: &EventLoopProxy<UserEvent>) -> (&'static str, String, &'static str) {
    let (path, query) = match path.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path, ""),
    };
    if path == "/" || path == "/health" {
        return ("200 OK", json!({"ok": true}).to_string(), "application/json");
    }

    if path == "/api/screen" {
        let max = query_param(query, "chars")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(4000)
            .clamp(200, 20000);
        return match crate::screen::read(max) {
            Ok(text) => (
                "200 OK",
                json!({"text": text, "chars": text.chars().count()}).to_string(),
                "application/json",
            ),
            Err(e) => ("200 OK", json!({"text": "", "error": e}).to_string(), "application/json"),
        };
    }

    if path == "/api/llm" {
        return ("200 OK", crate::llm::status().to_string(), "application/json");
    }

    if path == "/api/chat" && method == "POST" {
        let parsed: Value = serde_json::from_str(body).unwrap_or(json!({}));
        let msgs = parsed
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if msgs.is_empty() {
            return ("400 Bad Request", json!({"error":"messages required"}).to_string(), "application/json");
        }
        let screen = parsed.get("screen").and_then(|v| v.as_bool()).unwrap_or(true);
        return match crate::llm::chat(chat_messages(msgs, screen)) {
            Ok(reply) => ("200 OK", json!({"reply": reply}).to_string(), "application/json"),
            Err(e) => ("502 Bad Gateway", json!({"error": e}).to_string(), "application/json"),
        };
    }

    if path == "/api/state" {
        if method == "GET" {
            let v = shared.lock().unwrap().clone();
            return ("200 OK", v.to_string(), "application/json");
        }
        if method == "POST" || method == "PUT" {
            if let Ok(v) = serde_json::from_str::<Value>(body) {
                *shared.lock().unwrap() = v;
                persist(shared);
                push(proxy, shared);
                return ("200 OK", json!({"ok": true}).to_string(), "application/json");
            }
            return ("400 Bad Request", json!({"error":"invalid json"}).to_string(), "application/json");
        }
    }

    if path == "/api/tasks" {
        if method == "GET" {
            let v = shared.lock().unwrap();
            let tasks = v.get("tasks").and_then(|t| t.as_array()).cloned().unwrap_or_default();
            let filter = query_param(query, "filter").unwrap_or_else(|| "all".into());
            let filtered: Vec<Value> = tasks.into_iter().filter(|t| {
                let done = t.get("done").and_then(|d| d.as_bool()).unwrap_or(false);
                match filter.as_str() {
                    "pending" => !done,
                    "done" => done,
                    _ => true,
                }
            }).collect();
            return ("200 OK", Value::Array(filtered).to_string(), "application/json");
        }
        if method == "POST" {
            let parsed: Value = serde_json::from_str(body).unwrap_or(json!({}));
            let name = parsed.get("name").or_else(|| parsed.get("text")).and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if name.is_empty() {
                return ("400 Bad Request", json!({"error":"name required"}).to_string(), "application/json");
            }
            let name = name.chars().take(200).collect::<String>();
            let description = parsed.get("description").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            let description = description.chars().take(250000).collect::<String>();
            let id = now_ms();
            let task = json!({"id": id, "name": name, "description": description, "done": false, "createdAt": id, "updatedAt": id});
            {
                let mut s = shared.lock().unwrap();
                let arr = s.get_mut("tasks").and_then(|v| v.as_array_mut());
                if let Some(a) = arr {
                    a.insert(0, task.clone());
                } else {
                    s["tasks"] = json!([task.clone()]);
                }
            }
            persist(shared);
            push(proxy, shared);
            return ("201 Created", task.to_string(), "application/json");
        }
    }

    if let Some(rest) = path.strip_prefix("/api/tasks/") {
        let id = rest.trim_end_matches('/').to_string();
        if method == "GET" {
            let v = shared.lock().unwrap();
            let task = v.get("tasks").and_then(|t| t.as_array()).and_then(|arr| arr.iter().find(|t| id_of(t) == id)).cloned();
            return match task {
                Some(t) => ("200 OK", t.to_string(), "application/json"),
                None => ("404 Not Found", json!({"error":"not found"}).to_string(), "application/json"),
            };
        }
        if method == "PATCH" {
            let parsed: Value = serde_json::from_str(body).unwrap_or(json!({}));
            let mut found = false;
            {
                let mut s = shared.lock().unwrap();
                if let Some(arr) = s.get_mut("tasks").and_then(|v| v.as_array_mut()) {
                    for t in arr.iter_mut() {
                        if id_of(t) == id {
                            if let Some(d) = parsed.get("done") {
                                t["done"] = d.clone();
                            }
                            if let Some(n) = parsed.get("name").or_else(|| parsed.get("text")).and_then(|v| v.as_str()) {
                                t["name"] = json!(n.chars().take(200).collect::<String>());
                            }
                            if let Some(d) = parsed.get("description").and_then(|v| v.as_str()) {
                                t["description"] = json!(d.chars().take(250000).collect::<String>());
                            }
                            t["updatedAt"] = json!(now_ms());
                            found = true;
                        }
                    }
                }
            }
            if !found {
                return ("404 Not Found", json!({"error":"not found"}).to_string(), "application/json");
            }
            persist(shared);
            push(proxy, shared);
            return ("200 OK", json!({"ok": true}).to_string(), "application/json");
        }
        if method == "DELETE" {
            let mut found = false;
            {
                let mut s = shared.lock().unwrap();
                if let Some(arr) = s.get_mut("tasks").and_then(|v| v.as_array_mut()) {
                    let before = arr.len();
                    arr.retain(|t| id_of(t) != id);
                    found = arr.len() < before;
                }
            }
            if !found {
                return ("404 Not Found", json!({"error":"not found"}).to_string(), "application/json");
            }
            persist(shared);
            push(proxy, shared);
            return ("200 OK", json!({"ok": true}).to_string(), "application/json");
        }
    }

    if path == "/api/settings" {
        if method == "GET" {
            let v = shared.lock().unwrap();
            let out = json!({
                "interval": v.get("interval"),
                "unit": v.get("unit"),
                "theme": v.get("theme"),
                "border": v.get("border"),
                "bg": v.get("bg"),
                "text": v.get("text"),
                "sort": v.get("sort"),
                "alertSeconds": v.get("alertSeconds"),
                "quietStart": v.get("quietStart"),
                "quietEnd": v.get("quietEnd"),
                "autostart": v.get("autostart"),
            });
            return ("200 OK", out.to_string(), "application/json");
        }
        if method == "POST" || method == "PUT" || method == "PATCH" {
            let parsed: Value = serde_json::from_str(body).unwrap_or(json!({}));
            {
                let mut s = shared.lock().unwrap();
                for key in ["interval", "unit", "theme", "border", "bg", "text", "sort", "alertSeconds", "quietStart", "quietEnd", "autostart"] {
                    if let Some(val) = parsed.get(key) {
                        s[key] = val.clone();
                    }
                }
            }
            persist(shared);
            push(proxy, shared);
            return ("200 OK", json!({"ok": true}).to_string(), "application/json");
        }
    }

    if path == "/api/quit" && method == "POST" {
        std::process::exit(0);
    }

    if path == "/mcp" {
        if method == "POST" {
            let resp = handle_mcp(body, shared, proxy);
            return ("200 OK", resp, "application/json");
        }
        // GET info
        let tools = tool_list();
        let port = std::env::var("NOTCH_PORT").ok().and_then(|v| v.parse::<u16>().ok()).unwrap_or(7878);
        let info = json!({
            "endpoint": format!("http://127.0.0.1:{}/mcp", port),
            "transport": "JSON-RPC 2.0 over HTTP (POST)",
            "tools": tools,
        });
        return ("200 OK", info.to_string(), "application/json");
    }

    ("404 Not Found", json!({"error":"not found"}).to_string(), "application/json")
}

fn handle_mcp(body: &str, shared: &SharedState, proxy: &EventLoopProxy<UserEvent>) -> String {
    let payload: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    if payload.is_array() {
        let arr = payload.as_array().cloned().unwrap_or_default();
        let results: Vec<Value> = arr.iter().map(|p| mcp_one(p, shared, proxy)).collect();
        return serde_json::to_string(&results).unwrap_or_else(|_| "[]".into());
    }
    let resp = mcp_one(&payload, shared, proxy);
    serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into())
}

fn mcp_one(p: &Value, shared: &SharedState, proxy: &EventLoopProxy<UserEvent>) -> Value {
    let method = p.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let rid = p.get("id").cloned().unwrap_or(Value::Null);
    let params = p.get("params").cloned().unwrap_or(json!({}));
    match method {
        "initialize" => json!({
            "jsonrpc": "2.0", "id": rid,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "metro-notch", "version": "0.1.0"}
            }
        }),
        "notifications/initialized" => json!({"jsonrpc":"2.0","id":rid,"result":{}}),
        "tools/list" => json!({"jsonrpc":"2.0","id":rid,"result":{"tools": tool_list()}}),
        "tools/call" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let (is_err, text) = call_tool(name, args, shared, proxy);
            json!({"jsonrpc":"2.0","id":rid,"result":{"content":[{"type":"text","text":text}],"isError":is_err}})
        }
        _ => json!({"jsonrpc":"2.0","id":rid,"error":{"code":-32601,"message":format!("method not found: {}", method)}}),
    }
}

fn tool_list() -> Value {
    json!([
        {"name":"get_state","description":"Return the full persisted state (tasks + settings).","inputSchema":{"type":"object","properties":{}}},
        {"name":"set_state","description":"Replace the full persisted state.","inputSchema":{"type":"object","properties":{"state":{"type":"object"}},"required":["state"]}},
        {"name":"list_tasks","description":"List tasks. Optional filter: all|pending|done.","inputSchema":{"type":"object","properties":{"filter":{"type":"string","enum":["all","pending","done"]}}}},
        {"name":"get_task","description":"Get one task by id.","inputSchema":{"type":"object","properties":{"id":{}},"required":["id"]}},
        {"name":"add_task","description":"Add a new task (name <=200 chars, description <=250000 chars).","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"text":{"type":"string"},"description":{"type":"string"}},"required":["name"]}},
        {"name":"update_task","description":"Update name/description/done of a task.","inputSchema":{"type":"object","properties":{"id":{},"name":{"type":"string"},"description":{"type":"string"},"done":{"type":"boolean"}},"required":["id"]}},
        {"name":"complete_task","description":"Set a task done/undone.","inputSchema":{"type":"object","properties":{"id":{},"done":{"type":"boolean"}},"required":["id"]}},
        {"name":"remove_task","description":"Remove a task by id.","inputSchema":{"type":"object","properties":{"id":{}},"required":["id"]}},
        {"name":"reorder_tasks","description":"Reorder tasks by an array of ids.","inputSchema":{"type":"object","properties":{"ids":{"type":"array","items":{}}},"required":["ids"]}},
        {"name":"stats","description":"Task counts (total, pending, done).","inputSchema":{"type":"object","properties":{}}},
        {"name":"get_settings","description":"Return interval/unit/theme/colors.","inputSchema":{"type":"object","properties":{}}},
        {"name":"set_settings","description":"Update settings (interval,unit,theme,border,bg,text,sort,alertSeconds,quietStart,quietEnd,autostart).","inputSchema":{"type":"object","properties":{"interval":{"type":"integer"},"unit":{"type":"string"},"theme":{"type":"string"},"border":{"type":"string"},"bg":{"type":"string"},"text":{"type":"string"},"sort":{"type":"string"},"alertSeconds":{"type":"integer"},"quietStart":{"type":"string"},"quietEnd":{"type":"string"},"autostart":{"type":"boolean"}}}},
        {"name":"trigger_alert","description":"Return the task that would be shown next.","inputSchema":{"type":"object","properties":{}}},
        {"name":"read_screen","description":"Read the text of the foreground window via Windows UI Automation (no OCR, no GPU).","inputSchema":{"type":"object","properties":{"chars":{"type":"integer"}}}},
        {"name":"chat","description":"Chat with the local LLM; the current screen text is attached as context unless screen=false.","inputSchema":{"type":"object","properties":{"message":{"type":"string"},"screen":{"type":"boolean"}},"required":["message"]}},
        {"name":"quit","description":"Quit the application.","inputSchema":{"type":"object","properties":{}}}
    ])
}

fn call_tool(name: &str, args: Value, shared: &SharedState, proxy: &EventLoopProxy<UserEvent>) -> (bool, String) {
    match name {
        "get_state" => {
            let v = shared.lock().unwrap().clone();
            (false, v.to_string())
        }
        "set_state" => {
            if let Some(st) = args.get("state") {
                *shared.lock().unwrap() = st.clone();
                persist(shared);
                push(proxy, shared);
                (false, json!({"ok": true}).to_string())
            } else {
                (true, json!({"error":"state required"}).to_string())
            }
        }
        "list_tasks" => {
            let filter = args.get("filter").and_then(|v| v.as_str()).unwrap_or("all");
            let v = shared.lock().unwrap();
            let tasks = v.get("tasks").and_then(|t| t.as_array()).cloned().unwrap_or_default();
            let filtered: Vec<Value> = tasks.into_iter().filter(|t| {
                let done = t.get("done").and_then(|d| d.as_bool()).unwrap_or(false);
                match filter {
                    "pending" => !done,
                    "done" => done,
                    _ => true,
                }
            }).collect();
            (false, Value::Array(filtered).to_string())
        }
        "get_task" => {
            let id = args.get("id").map(id_to_string).unwrap_or_default();
            let v = shared.lock().unwrap();
            let task = v.get("tasks").and_then(|t| t.as_array()).and_then(|arr| arr.iter().find(|t| id_of(t) == id)).cloned();
            match task {
                Some(t) => (false, t.to_string()),
                None => (true, json!({"error":"not found"}).to_string()),
            }
        }
        "update_task" => {
            let id = args.get("id").map(id_to_string).unwrap_or_default();
            let mut found = false;
            {
                let mut s = shared.lock().unwrap();
                if let Some(arr) = s.get_mut("tasks").and_then(|v| v.as_array_mut()) {
                    for t in arr.iter_mut() {
                        if id_of(t) == id {
                            if let Some(n) = args.get("name").and_then(|v| v.as_str()) {
                                t["name"] = json!(n.chars().take(200).collect::<String>());
                            }
                            if let Some(d) = args.get("description").and_then(|v| v.as_str()) {
                                t["description"] = json!(d.chars().take(250000).collect::<String>());
                            }
                            if let Some(d) = args.get("done") {
                                t["done"] = d.clone();
                            }
                            t["updatedAt"] = json!(now_ms());
                            found = true;
                        }
                    }
                }
            }
            if !found {
                return (true, json!({"error":"not found"}).to_string());
            }
            persist(shared);
            push(proxy, shared);
            (false, json!({"ok": true}).to_string())
        }
        "reorder_tasks" => {
            let ids: Vec<String> = args.get("ids").and_then(|v| v.as_array()).map(|a| a.iter().map(id_to_string).collect()).unwrap_or_default();
            {
                let mut s = shared.lock().unwrap();
                if let Some(arr) = s.get_mut("tasks").and_then(|v| v.as_array_mut()) {
                    let mut ordered = Vec::new();
                    for id in &ids {
                        if let Some(pos) = arr.iter().position(|t| id_of(t) == *id) {
                            ordered.push(arr.remove(pos));
                        }
                    }
                    ordered.extend(arr.drain(..));
                    *arr = ordered;
                }
            }
            persist(shared);
            push(proxy, shared);
            (false, json!({"ok": true}).to_string())
        }
        "stats" => {
            let v = shared.lock().unwrap();
            let tasks = v.get("tasks").and_then(|t| t.as_array()).cloned().unwrap_or_default();
            let done = tasks.iter().filter(|t| t.get("done").and_then(|d| d.as_bool()).unwrap_or(false)).count();
            (false, json!({"total": tasks.len(), "pending": tasks.len() - done, "done": done}).to_string())
        }
        "add_task" => {
            let name = args.get("name").or_else(|| args.get("text")).and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if name.is_empty() {
                return (true, json!({"error":"name required"}).to_string());
            }
            let name = name.chars().take(200).collect::<String>();
            let description = args.get("description").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            let description = description.chars().take(250000).collect::<String>();
            let id = now_ms();
            let task = json!({"id": id, "name": name, "description": description, "done": false, "createdAt": id, "updatedAt": id});
            {
                let mut s = shared.lock().unwrap();
                let arr = s.get_mut("tasks").and_then(|v| v.as_array_mut());
                if let Some(a) = arr {
                    a.insert(0, task.clone());
                } else {
                    s["tasks"] = json!([task.clone()]);
                }
            }
            persist(shared);
            push(proxy, shared);
            (false, task.to_string())
        }
        "complete_task" => {
            let id = args.get("id").map(id_to_string).unwrap_or_default();
            let done = args.get("done").and_then(|v| v.as_bool()).unwrap_or(true);
            let mut found = false;
            {
                let mut s = shared.lock().unwrap();
                if let Some(arr) = s.get_mut("tasks").and_then(|v| v.as_array_mut()) {
                    for t in arr.iter_mut() {
                        if id_of(t) == id {
                            t["done"] = json!(done);
                            t["updatedAt"] = json!(now_ms());
                            found = true;
                        }
                    }
                }
            }
            if !found {
                return (true, json!({"error":"not found"}).to_string());
            }
            persist(shared);
            push(proxy, shared);
            (false, json!({"ok": true}).to_string())
        }
        "remove_task" => {
            let id = args.get("id").map(id_to_string).unwrap_or_default();
            let mut found = false;
            {
                let mut s = shared.lock().unwrap();
                if let Some(arr) = s.get_mut("tasks").and_then(|v| v.as_array_mut()) {
                    let before = arr.len();
                    arr.retain(|t| id_of(t) != id);
                    found = arr.len() < before;
                }
            }
            if !found {
                return (true, json!({"error":"not found"}).to_string());
            }
            persist(shared);
            push(proxy, shared);
            (false, json!({"ok": true}).to_string())
        }
        "get_settings" => {
            let v = shared.lock().unwrap();
            let out = json!({
                "interval": v.get("interval"),
                "unit": v.get("unit"),
                "theme": v.get("theme"),
                "border": v.get("border"),
                "bg": v.get("bg"),
                "text": v.get("text"),
                "sort": v.get("sort"),
                "alertSeconds": v.get("alertSeconds"),
                "quietStart": v.get("quietStart"),
                "quietEnd": v.get("quietEnd"),
                "autostart": v.get("autostart"),
            });
            (false, out.to_string())
        }
        "set_settings" => {
            {
                let mut s = shared.lock().unwrap();
                for key in ["interval", "unit", "theme", "border", "bg", "text", "sort", "alertSeconds", "quietStart", "quietEnd", "autostart"] {
                    if let Some(val) = args.get(key) {
                        s[key] = val.clone();
                    }
                }
            }
            persist(shared);
            push(proxy, shared);
            (false, json!({"ok": true}).to_string())
        }
        "trigger_alert" => {
            let v = shared.lock().unwrap();
            let tasks = v.get("tasks").and_then(|t| t.as_array()).cloned().unwrap_or_default();
            let pending: Vec<&Value> = tasks.iter().filter(|t| !t.get("done").and_then(|d| d.as_bool()).unwrap_or(false)).collect();
            let task = pending.first().copied().cloned().unwrap_or(Value::Null);
            (false, task.to_string())
        }
        "read_screen" => {
            let max = args.get("chars").and_then(|v| v.as_u64()).unwrap_or(4000).clamp(200, 20000) as usize;
            match crate::screen::read(max) {
                Ok(text) => (false, text),
                Err(e) => (true, json!({"error": e}).to_string()),
            }
        }
        "chat" => {
            let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if message.is_empty() {
                return (true, json!({"error":"message required"}).to_string());
            }
            let screen = args.get("screen").and_then(|v| v.as_bool()).unwrap_or(true);
            let msgs = vec![json!({"role": "user", "content": message})];
            match crate::llm::chat(chat_messages(msgs, screen)) {
                Ok(reply) => (false, reply),
                Err(e) => (true, json!({"error": e}).to_string()),
            }
        }
        "quit" => {
            std::process::exit(0);
        }
        _ => (true, json!({"error": format!("unknown tool {}", name)}).to_string()),
    }
}

/// Mensagens enviadas ao LLM: prompt de sistema (+ texto da tela) e depois a
/// conversa. A tela entra como system para não ser confundida com pedido do
/// usuário.
fn chat_messages(mut msgs: Vec<Value>, screen: bool) -> Value {
    let mut system = crate::llm::SYSTEM.to_string();
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

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k == key { Some(v.replace('+', " ")) } else { None }
    })
}

fn id_of(t: &Value) -> String {
    match t.get("id") {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
        None => String::new(),
    }
}

fn id_to_string(v: &Value) -> String {
    match v {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn persist(shared: &SharedState) {
    let data = shared.lock().unwrap().clone();
    let path = crate::state_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, data.to_string());
}

fn push(proxy: &EventLoopProxy<UserEvent>, shared: &SharedState) {
    let json_str = shared.lock().unwrap().to_string();
    let script = format!("window.__load({})", json_str);
    let _ = proxy.send_event(UserEvent::Eval(script));
}