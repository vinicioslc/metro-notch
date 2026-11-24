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
    fn IsWindowVisible(hwnd: isize) -> i32;
    fn GetWindow(hwnd: isize, cmd: u32) -> isize;
    fn GetClassNameW(hwnd: isize, buf: *mut u16, max: i32) -> i32;
}

/// Fallback quando o app está em foco e a varredura Z falha.
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
    let mut seen = std::collections::HashSet::new();
    collect(&walker, &win, 0, &mut out, &mut budget, &mut seen);
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
        let external = top_external();
        if external != 0 {
            LAST_FOREIGN.store(external, Ordering::Relaxed);
            return external;
        }
        let last = LAST_FOREIGN.load(Ordering::Relaxed);
        return if last != 0 && unsafe { IsWindow(last) } != 0 { last } else { 0 };
    }
    LAST_FOREIGN.store(hwnd, Ordering::Relaxed);
    hwnd
}

/// Janela visível mais alta que não é do nosso processo nem infraestrutura
/// do sistema. Mais robusto que lembrar a última externa, porque menus,
/// overlays e toasts podem roubar o foreground entre uma e outra leitura.
fn top_external() -> isize {
    let me = std::process::id();
    let mut cur = unsafe { GetWindow(GetForegroundWindow(), 2) }; // GW_HWNDNEXT
    let mut hops = 0;
    while cur != 0 && hops < 200 {
        hops += 1;
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(cur, &mut pid) };
        if pid != me && unsafe { IsWindowVisible(cur) } != 0 && !is_system_shell(cur) {
            return cur;
        }
        cur = unsafe { GetWindow(cur, 2) };
    }
    0
}

fn is_system_shell(hwnd: isize) -> bool {
    let mut buf = [0u16; 128];
    let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 0 {
        return false;
    }
    let cls = String::from_utf16_lossy(&buf[..n as usize]);
    matches!(
        cls.as_str(),
        "Shell_TrayWnd"
            | "Shell_SecondaryTrayWnd"
            | "Progman"
            | "WorkerW"
            | "DV2ControlHost"
            | "MsgrIMEWindowClass"
            | "SysShadow"
    )
}

fn collect(
    walker: &UITreeWalker,
    el: &UIElement,
    depth: usize,
    out: &mut String,
    budget: &mut i64,
    seen: &mut std::collections::HashSet<String>,
) {
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
                        push(out, &t, budget, seen);
                        return;
                    }
                }
            }
        }
    }
    // barras de menu/abas/ferramentas só repetem rótulos; o conteúdo real está
    // nos filhos, então não descartamos a subárvore, apenas o nome.
    if !noisy(&ctype) {
        if let Ok(name) = el.get_name() {
            let name = clean(&name);
            if !name.is_empty() {
                push(out, &name, budget, seen);
            }
        }
    }
    if let Ok(mut cur) = walker.get_first_child(el) {
        loop {
            collect(walker, &cur, depth + 1, out, budget, seen);
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

fn noisy(t: &Option<ControlType>) -> bool {
    matches!(
        t,
        Some(ControlType::MenuBar)
            | Some(ControlType::Menu)
            | Some(ControlType::MenuItem)
            | Some(ControlType::ToolBar)
            | Some(ControlType::Tab)
            | Some(ControlType::TabItem)
            | Some(ControlType::TitleBar)
            | Some(ControlType::ScrollBar)
            | Some(ControlType::StatusBar)
            | Some(ControlType::Separator)
            | Some(ControlType::Thumb)
            | Some(ControlType::Image)
    )
}

fn push(out: &mut String, s: &str, budget: &mut i64, seen: &mut std::collections::HashSet<String>) {
    if *budget <= 0 || s.is_empty() {
        return;
    }
    // ícones e rótulos repetidos aparecem uma vez só
    if !seen.insert(s.to_string()) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_deduplica_e_respeita_o_orcamento() {
        let mut out = String::new();
        let mut budget = 10i64;
        let mut seen = std::collections::HashSet::new();
        push(&mut out, "abc", &mut budget, &mut seen);
        push(&mut out, "abc", &mut budget, &mut seen);
        push(&mut out, "def", &mut budget, &mut seen);
        assert_eq!(out, "abc\ndef\n");
        assert_eq!(budget, 2);
    }
}
