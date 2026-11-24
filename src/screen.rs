//! Leitura de tela pela árvore de UI Automation — a mesma fonte que o Narrator
//! usa. Sem screenshot e sem OCR: custo de GPU zero.

use std::sync::atomic::{AtomicIsize, Ordering};

use uiautomation::patterns::UITextPattern;
use uiautomation::types::{ControlType, Handle};
use uiautomation::{UIAutomation, UIElement, UITreeWalker};

#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> isize;
    fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
    fn IsWindow(hwnd: isize) -> i32;
}

/// Quando o painel do app está em foco o primeiro plano é o próprio Metro
/// Notch; nesse caso lemos a última janela externa que teve foco.
static LAST_FOREIGN: AtomicIsize = AtomicIsize::new(0);

pub fn read(max_chars: usize) -> Result<String, String> {
    let hwnd = foreground();
    if hwnd == 0 {
        return Err("nenhuma janela em primeiro plano".into());
    }
    let auto = UIAutomation::new().map_err(|e| e.to_string())?;
    let win = auto
        .element_from_handle(Handle::from(hwnd))
        .map_err(|e| e.to_string())?;
    let mut out = String::new();
    if let Ok(name) = win.get_name() {
        let name = clean(&name);
        if !name.is_empty() {
            out.push_str(&format!("Janela: {name}\n"));
        }
    }
    let walker = auto.get_control_view_walker().map_err(|e| e.to_string())?;
    let mut budget = max_chars as i64;
    collect(&walker, &win, 0, &mut out, &mut budget);
    Ok(trim(&out, max_chars))
}

fn foreground() -> isize {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd == 0 {
        return 0;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    if pid == std::process::id() {
        let last = LAST_FOREIGN.load(Ordering::Relaxed);
        return if last != 0 && unsafe { IsWindow(last) } != 0 { last } else { 0 };
    }
    LAST_FOREIGN.store(hwnd, Ordering::Relaxed);
    hwnd
}

fn collect(walker: &UITreeWalker, el: &UIElement, depth: usize, out: &mut String, budget: &mut i64) {
    if *budget <= 0 || depth > 12 {
        return;
    }
    if el.is_password().unwrap_or(false) {
        return;
    }
    // Documento/edição: TextPattern devolve o conteúdo inteiro em uma chamada,
    // muito mais barato que percorrer cada nó de texto.
    let ctype = el.get_control_type().ok();
    if matches!(ctype, Some(ControlType::Document) | Some(ControlType::Edit)) {
        if let Ok(p) = el.get_pattern::<UITextPattern>() {
            if let Ok(r) = p.get_document_range() {
                if let Ok(t) = r.get_text(*budget as i32) {
                    let t = clean(&t);
                    if t.chars().count() > 40 {
                        push(out, &t, budget);
                        return;
                    }
                }
            }
        }
    }
    if let Ok(name) = el.get_name() {
        let name = clean(&name);
        if !name.is_empty() {
            push(out, &name, budget);
        }
    }
    if let Ok(mut cur) = walker.get_first_child(el) {
        loop {
            collect(walker, &cur, depth + 1, out, budget);
            if *budget <= 0 {
                break;
            }
            match walker.get_next_sibling(&cur) {
                Ok(next) => cur = next,
                Err(_) => break,
            }
        }
    }
}

fn push(out: &mut String, s: &str, budget: &mut i64) {
    if *budget <= 0 || s.is_empty() {
        return;
    }
    // containers repetem o nome do filho; linha idêntica seguida é ruído
    if out.lines().last() == Some(s) {
        return;
    }
    out.push_str(s);
    out.push('\n');
    *budget -= s.chars().count() as i64 + 1;
}

fn clean(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn trim(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}
