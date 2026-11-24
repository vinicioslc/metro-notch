#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::time::{Duration, Instant};

mod http;
mod llm;
mod screen;

use tao::{
    dpi::{LogicalSize, PhysicalPosition},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    platform::windows::{WindowBuilderExtWindows, WindowExtWindows},
    window::{Window, WindowBuilder},
};
use wry::{WebView, WebViewBuilder};
const HTML: &str = include_str!("ui.html");

pub type SharedState = std::sync::Arc<std::sync::Mutex<serde_json::Value>>;

const IDLE_W: f64 = 560.0;
const IDLE_H_DEFAULT: f64 = 5.0;

type Hwnd = *mut std::ffi::c_void;

#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(hwnd: Hwnd, attr: u32, value: *const std::ffi::c_void, size: u32) -> i32;
    fn DwmEnableBlurBehindWindow(hwnd: Hwnd, bb: *const BlurBehind) -> i32;
}

#[repr(C)]
struct BlurBehind {
    flags: u32,
    enable: i32,
    region: *mut std::ffi::c_void,
    transition_on_maximized: i32,
}

#[link(name = "user32")]
extern "system" {
    fn GetWindowLongPtrW(hwnd: Hwnd, idx: i32) -> isize;
    fn SetWindowLongPtrW(hwnd: Hwnd, idx: i32, value: isize) -> isize;
    fn SetWindowPos(hwnd: Hwnd, after: Hwnd, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    fn SetWindowRgn(hwnd: Hwnd, rgn: Hwnd, redraw: i32) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryW(name: *const u16) -> Hwnd;
    fn GetProcAddress(lib: Hwnd, name: *const u8) -> *mut std::ffi::c_void;
}

#[repr(C)]
struct AccentPolicy {
    state: u32,
    flags: u32,
    gradient_color: u32,
    animation_id: u32,
}

#[repr(C)]
struct WinCompAttrData {
    attribute: i32,
    data: *mut std::ffi::c_void,
    size_of_data: usize,
}

/// Frosted glass: ACCENT_ENABLE_ACRYLICBLURBEHIND blurs whatever is behind the
/// window; gradient_color is 0xAABBGGRR and tints the blur. The API is exported
/// by user32.dll but missing from the SDK import library, hence the dynamic lookup.
fn enable_glass(hwnd: Hwnd, color: u32) {
    type SetWindowCompositionAttribute = unsafe extern "system" fn(Hwnd, *mut WinCompAttrData) -> i32;
    let func: SetWindowCompositionAttribute = unsafe {
        let lib = LoadLibraryW("user32.dll\0".encode_utf16().collect::<Vec<u16>>().as_ptr());
        if lib.is_null() {
            return;
        }
        let proc = GetProcAddress(lib, c"SetWindowCompositionAttribute".as_ptr() as *const u8);
        if proc.is_null() {
            return;
        }
        std::mem::transmute(proc)
    };
    let mut accent = AccentPolicy {
        state: 4,
        flags: 0,
        gradient_color: color,
        animation_id: 0,
    };
    let mut data = WinCompAttrData {
        attribute: 19,
        data: &mut accent as *mut AccentPolicy as *mut std::ffi::c_void,
        size_of_data: std::mem::size_of::<AccentPolicy>(),
    };
    unsafe { func(hwnd, &mut data) };
}

/// "#rrggbb" (page palette) -> 0xAABBGGRR tint, 40% alpha so the blur shows.
fn tint(bg: Option<&str>) -> u32 {
    let hex = bg.unwrap_or("").trim_start_matches('#');
    let rgb = || -> Option<(u32, u32, u32)> {
        if hex.len() != 6 {
            return None;
        }
        Some((
            u32::from_str_radix(&hex[0..2], 16).ok()?,
            u32::from_str_radix(&hex[2..4], 16).ok()?,
            u32::from_str_radix(&hex[4..6], 16).ok()?,
        ))
    };
    match rgb() {
        Some((r, g, b)) => 0x66_00_00_00 | (b << 16) | (g << 8) | r,
        None => 0x66_40_34_2E,
    }
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateRectRgn(l: i32, t: i32, r: i32, b: i32) -> *mut std::ffi::c_void;
    fn CreateRoundRectRgn(l: i32, t: i32, r: i32, b: i32, w: i32, h: i32) -> *mut std::ffi::c_void;
    fn CombineRgn(dst: *mut std::ffi::c_void, src1: *mut std::ffi::c_void, src2: *mut std::ffi::c_void, mode: i32) -> i32;
    fn DeleteObject(o: *mut std::ffi::c_void) -> i32;
}

const GWL_STYLE: i32 = -16;
const GWL_EXSTYLE: i32 = -20;
const WS_POPUP: isize = 0x8000_0000u32 as isize;
const WS_EX_TOOLWINDOW: isize = 0x0000_0080;
const WS_EX_TOPMOST: isize = 0x0000_0008;
const WS_EX_APPWINDOW: isize = 0x0004_0000;
const WS_EX_WINDOWEDGE: isize = 0x0000_0100;
const SWP_FRAMECHANGED: u32 = 0x0020;
const SWP_NOMOVE: u32 = 0x0002;
const SWP_NOSIZE: u32 = 0x0001;
const SWP_SHOWWINDOW: u32 = 0x0040;
const HWND_TOPMOST: Hwnd = -1isize as Hwnd;

/// WS_DLGFRAME (what a tao undecorated window keeps) makes Windows clamp the
/// height to the minimum tracking size, so a 5px idle bar becomes ~38px.
/// Replacing the style with plain WS_POPUP removes that clamp; the empty
/// blur region keeps per-pixel alpha for the transparent webview.
fn strip_native_frame(window: &Window) {
    let hwnd = window.hwnd() as Hwnd;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        SetWindowLongPtrW(hwnd, GWL_STYLE, WS_POPUP | (style & !0x00CF_0000));
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            (ex & !(WS_EX_APPWINDOW | WS_EX_WINDOWEDGE)) | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
        );
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        );
        // Apply rounded shape (top straight, bottom rounded) so the acrylic
        // blur does not bleed into the square corners.
        shape_window(hwnd, IDLE_W as i32, IDLE_H_DEFAULT as i32, window.scale_factor());

        let region = CreateRectRgn(0, 0, -1, -1);
        let bb = BlurBehind {
            flags: 0x0000_0001 | 0x0000_0002,
            enable: 1,
            region,
            transition_on_maximized: 0,
        };
        DwmEnableBlurBehindWindow(hwnd, &bb);
        DeleteObject(region);

        enable_glass(hwnd, tint(None));

        // DWMWA_WINDOW_CORNER_PREFERENCE -> DWMWCP_DONOTROUND
        let corner: i32 = 1;
        DwmSetWindowAttribute(hwnd, 33, &corner as *const i32 as *const std::ffi::c_void, 4);
        // DWMWA_BORDER_COLOR -> DWMWA_COLOR_NONE
        let border: u32 = 0xFFFF_FFFE;
        DwmSetWindowAttribute(hwnd, 34, &border as *const u32 as *const std::ffi::c_void, 4);
    }
}

pub enum UserEvent {
    Ipc(String),
    Eval(String),
}

pub fn state_path() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    let dir = PathBuf::from(&base).join("metro-notch");
    let path = dir.join("state.json");
    // one-time migration of the pre-rename state file
    let old = PathBuf::from(&base).join("notch-reminder").join("state.json");
    if !path.exists() && old.exists() {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::copy(&old, &path);
    }
    path
}

fn shape_window(hwnd: Hwnd, w: i32, h: i32, scale: f64) {
    let pw = (w as f64 * scale).round() as i32;
    let ph = (h as f64 * scale).round() as i32;
    // macOS-like rounded bottom corners.
    let r = if ph <= 12 { 8 } else { 18 };
    let pr = (r as f64 * scale).round() as i32;
    unsafe {
        let top = CreateRectRgn(0, 0, pw, pr);
        let round = CreateRoundRectRgn(0, 0, pw, ph, pr * 2, pr * 2);
        let combined = CreateRectRgn(0, 0, 0, 0);
        const RGN_OR: i32 = 2;
        CombineRgn(combined, top, round, RGN_OR);
        DeleteObject(top);
        DeleteObject(round);
        SetWindowRgn(hwnd, combined, 1);
    }
}

fn idle_h(shared: &SharedState) -> f64 {
    shared.lock().unwrap().get("idleVisibleHeight").and_then(|v| v.as_f64()).unwrap_or(IDLE_H_DEFAULT).clamp(1.0, 30.0)
}

fn recenter(window: &Window, logical_width: f64) {
    let width = (logical_width * window.scale_factor()).round() as i32;
    if let Some(monitor) = window.current_monitor() {
        let pos = monitor.position();
        let size = monitor.size();
        let x = pos.x + (size.width as i32 - width).max(0) / 2;
        window.set_outer_position(PhysicalPosition::new(x, pos.y));
    }
}

/// Autostart at logon via HKCU Run (a GUI app cannot run as a Windows Service,
/// which lives in session 0 with no desktop).
fn set_autostart(on: bool) {
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut cmd = std::process::Command::new("reg");
    if on {
        cmd.args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "MetroNotch",
            "/t",
            "REG_SZ",
            "/d",
            &format!("\"{}\"", exe),
            "/f",
        ]);
    } else {
        cmd.args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "MetroNotch",
            "/f",
        ]);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let _ = cmd.status();
    // drop the pre-rename autostart entry if present
    let mut legacy = std::process::Command::new("reg");
    legacy.args([
        "delete",
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
        "/v",
        "NotchReminder",
        "/f",
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        legacy.creation_flags(0x0800_0000);
    }
    let _ = legacy.status();
}

fn handle_ipc(
    body: &str,
    window: &Window,
    webview: &WebView,
    shared: &SharedState,
    proxy: &EventLoopProxy<UserEvent>,
) {
    let Ok(msg) = serde_json::from_str::<serde_json::Value>(body) else {
        return;
    };
    match msg["t"].as_str() {
        Some("size") => {
            let w = msg["w"].as_f64().unwrap_or(IDLE_W).clamp(120.0, 900.0);
            let h = msg["h"].as_f64().unwrap_or(IDLE_H_DEFAULT).clamp(1.0, 900.0);
            window.set_inner_size(LogicalSize::new(w, h));
            recenter(window, w);
            shape_window(window.hwnd() as Hwnd, w as i32, h as i32, window.scale_factor());
        }
        Some("load") => {
            let state = shared.lock().unwrap().clone();
            let json = state.to_string();
            let _ = webview.evaluate_script(&format!("window.__load({json})"));
        }
        Some("save") => {
            if let Some(data) = msg.get("data") {
                *shared.lock().unwrap() = data.clone();
                let path = state_path();
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = std::fs::write(path, data.to_string());
                enable_glass(window.hwnd() as Hwnd, tint(data["bg"].as_str()));
            }
        }
        Some("idleHeight") => {
            if let Some(h) = msg["h"].as_f64() {
                let h = h.clamp(1.0, 30.0);
                if let Some(o) = shared.lock().unwrap().as_object_mut() {
                    o.insert("idleVisibleHeight".into(), serde_json::json!(h));
                }
            }
        }
        Some("autostart") => set_autostart(msg["on"].as_bool().unwrap_or(false)),
        Some("chat") => {
            let msgs = msg["messages"].as_array().cloned().unwrap_or_default();
            let screen = msg["screen"].as_bool().unwrap_or(true);
            let proxy = proxy.clone();
            std::thread::spawn(move || {
                let out = match llm::chat(llm::messages(msgs, screen)) {
                    Ok(reply) => serde_json::json!({"reply": reply}),
                    Err(e) => serde_json::json!({"error": e}),
                };
                let _ = proxy.send_event(UserEvent::Eval(format!("window.__chatReply({out});")));
            });
        }
        Some("transcribe") => {
            let audio = msg.get("audio").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let format = msg.get("format").and_then(|v| v.as_str()).unwrap_or("wav").to_string();
            let p = proxy.clone();
            std::thread::spawn(move || {
                let out = if audio.is_empty() {
                    serde_json::json!({"error":"audio vazio"})
                } else {
                    match llm::transcribe(&audio, &format) {
                        Ok(text) => serde_json::json!({"text": text}),
                        Err(e) => serde_json::json!({"error": e}),
                    }
                };
                let _ = p.send_event(UserEvent::Eval(format!("window.__transcribeReply({out});")));
            });
        }
        Some("quit") => std::process::exit(0),
        _ => {}
    }
}

fn main() -> wry::Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Metro Notch")
        .with_inner_size(LogicalSize::new(IDLE_W, IDLE_H_DEFAULT))
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_undecorated_shadow(false)
        .with_always_on_top(true)
        .with_skip_taskbar(true)
        .build(&event_loop)
        .expect("failed to create window");

    let mut initial = std::fs::read_to_string(state_path())
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({
            "interval": 15, "unit": "m", "theme": "macos",
            "border": "#0a84ff", "bg": "#1e1e1e", "text": "#f5f5f7",
            "sort": "last-added", "alertSeconds": 8,
            "quietStart": "", "quietEnd": "", "autostart": false, "idleVisibleHeight": 5,
            "tasks": [
                {"id": 1, "name": "Revisar pull requests", "description": "", "done": false},
                {"id": 2, "name": "Responder e-mails pendentes", "description": "", "done": false},
                {"id": 3, "name": "Planejar as prioridades do dia", "description": "", "done": false}
            ]
        }));
    if let Some(o) = initial.as_object_mut() {
        o.entry("sort").or_insert_with(|| serde_json::json!("last-added"));
        o.entry("alertSeconds").or_insert_with(|| serde_json::json!(8));
        o.entry("quietStart").or_insert_with(|| serde_json::json!(""));
        o.entry("quietEnd").or_insert_with(|| serde_json::json!(""));
        o.entry("autostart").or_insert_with(|| serde_json::json!(false));
        o.entry("idleVisibleHeight").or_insert_with(|| serde_json::json!(5));
    }
    if let Some(tasks) = initial.get_mut("tasks").and_then(|v| v.as_array_mut()) {
        for t in tasks.iter_mut() {
            if let Some(o) = t.as_object_mut() {
                let id = o.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                o.entry("createdAt").or_insert_with(|| serde_json::json!(id));
                o.entry("updatedAt").or_insert_with(|| serde_json::json!(id));
                if !o.contains_key("name") {
                    if let Some(text) = o.get("text").and_then(|v| v.as_str()) {
                        o.insert("name".into(), serde_json::json!(text));
                    }
                }
            }
        }
    }
    let shared: SharedState = std::sync::Arc::new(std::sync::Mutex::new(initial));
    let ih = idle_h(&shared);
    strip_native_frame(&window);
    window.set_inner_size(LogicalSize::new(IDLE_W, ih));
    recenter(&window, IDLE_W);

    let port: u16 = std::env::var("NOTCH_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7878);
    http::start(shared.clone(), proxy.clone(), port);

    let ipc_proxy = proxy.clone();
    let webview = WebViewBuilder::new()
        .with_html(HTML)
        .with_transparent(true)
        .with_ipc_handler(move |req| {
            let _ = ipc_proxy.send_event(UserEvent::Ipc(req.body().clone()));
        })
        .build(&window)
        .expect("failed to create webview");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(_) => {
                window.set_always_on_top(true);
                *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(3));
            }
            Event::UserEvent(UserEvent::Ipc(body)) => handle_ipc(&body, &window, &webview, &shared, &proxy),
            Event::UserEvent(UserEvent::Eval(script)) => {
                let _ = webview.evaluate_script(&script);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}