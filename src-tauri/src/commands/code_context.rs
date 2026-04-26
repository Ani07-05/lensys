use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CodeContext {
    pub file_path: Option<String>,
    pub file_name: Option<String>,
    pub language: Option<String>,
    pub content: Option<String>,
    pub window_title: String,
    pub active_app: String,
    pub is_ide: bool,
    pub symbols: Vec<Symbol>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Symbol {
    pub kind: String,
    pub name: String,
    pub line: u32,
}

pub async fn get_active_code_context() -> Result<CodeContext, String> {
    let (window_title, active_app) = tokio::task::spawn_blocking(get_foreground_window_info)
        .await
        .map_err(|e| e.to_string())?;

    let is_ide = is_ide_window(&active_app, &window_title);

    let mut ctx = CodeContext {
        window_title: window_title.clone(),
        active_app: active_app.clone(),
        is_ide,
        ..Default::default()
    };

    if is_ide {
        if let Some((filename, workspace)) = parse_vscode_title(&window_title) {
            ctx.file_name = Some(filename.clone());
            ctx.language = detect_language(&filename).map(String::from);

            if let Ok(path) = find_file_on_disk(&filename, workspace.as_deref()).await {
                ctx.file_path = Some(path.to_string_lossy().to_string());
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    let lang = ctx.language.as_deref().unwrap_or("");
                    ctx.symbols = extract_symbols(&content, lang);
                    ctx.content = Some(truncate_content(&content, 150));
                }
            }
        }
    }

    // Clipboard fallback if no content found
    if ctx.content.is_none() {
        if let Ok(Ok(text)) = tokio::task::spawn_blocking(read_clipboard).await {
            if text.len() > 30 {
                let lang = detect_language_from_content(&text);
                ctx.language = lang.map(String::from);
                let l = ctx.language.as_deref().unwrap_or("");
                ctx.symbols = extract_symbols(&text, l);
                ctx.content = Some(truncate_content(&text, 100));
            }
        }
    }

    Ok(ctx)
}

pub async fn get_clipboard_code_context() -> Result<CodeContext, String> {
    let text = tokio::task::spawn_blocking(read_clipboard)
        .await
        .map_err(|e| e.to_string())??;
    let text = text.trim().to_string();

    if text.is_empty() {
        return Err("Clipboard is empty".to_string());
    }

    let language = detect_language_from_content(&text).map(String::from);
    let symbols = extract_symbols(&text, language.as_deref().unwrap_or(""));
    let (window_title, active_app) = tokio::task::spawn_blocking(get_foreground_window_info)
        .await
        .map_err(|e| e.to_string())?;
    let mut file_path = None;
    let mut file_name = Some("selection".to_string());
    let mut resolved_language = language;

    if is_ide_window(&active_app, &window_title) {
        if let Some((filename, workspace)) = parse_vscode_title(&window_title) {
            file_name = Some(filename.clone());
            if resolved_language.is_none() {
                resolved_language = detect_language(&filename).map(String::from);
            }
            if let Ok(path) = find_file_on_disk(&filename, workspace.as_deref()).await {
                file_path = Some(path.to_string_lossy().to_string());
            }
        }
    }

    Ok(CodeContext {
        file_path,
        file_name,
        language: resolved_language,
        content: Some(truncate_content(&text, 200)),
        window_title: if window_title.is_empty() {
            "Clipboard".to_string()
        } else {
            window_title
        },
        active_app: if active_app.is_empty() {
            "clipboard".to_string()
        } else {
            active_app
        },
        is_ide: true,
        symbols,
    })
}

fn read_clipboard() -> Result<String, String> {
    arboard::Clipboard::new()
        .map_err(|e| e.to_string())?
        .get_text()
        .map_err(|e| e.to_string())
}

// ── Window detection ──────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn get_foreground_window_info() -> (String, String) {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return (String::new(), String::new());
        }
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        let title = if len > 0 {
            OsString::from_wide(&buf[..len as usize])
                .to_string_lossy()
                .to_string()
        } else {
            String::new()
        };
        // App name: extract from title or use empty string
        let app = if title.contains("Visual Studio Code") {
            "vscode".to_string()
        } else if title.contains("JetBrains")
            || title.contains("IntelliJ")
            || title.contains("PyCharm")
            || title.contains("WebStorm")
            || title.contains("CLion")
            || title.contains("RustRover")
        {
            "jetbrains".to_string()
        } else {
            String::new()
        };
        (title, app)
    }
}

#[cfg(not(target_os = "windows"))]
fn get_foreground_window_info() -> (String, String) {
    (String::new(), String::new())
}

fn is_ide_window(app: &str, title: &str) -> bool {
    title.contains("Visual Studio Code")
        || title.contains("JetBrains")
        || title.contains("IntelliJ")
        || title.contains("PyCharm")
        || title.contains("WebStorm")
        || title.contains("CLion")
        || title.contains("RustRover")
        || app.contains("vscode")
        || app.contains("jetbrains")
        || title.ends_with(" - vim")
        || title.ends_with(" - nvim")
        || title.contains("Neovim")
        || title.contains("Emacs")
}

/// Parse `"● filename.ext - workspace - Visual Studio Code"` → (filename, Some(workspace))
fn parse_vscode_title(title: &str) -> Option<(String, Option<String>)> {
    let title = title.trim_start_matches('●').trim();
    if !title.ends_with("Visual Studio Code") {
        return None;
    }
    let parts: Vec<&str> = title.split(" - ").collect();
    if parts.len() < 2 {
        return None;
    }
    let filename = parts[0].trim().to_string();
    if filename.starts_with("Untitled") || filename.is_empty() {
        return None;
    }
    let workspace = if parts.len() >= 3 {
        Some(parts[1].trim().to_string())
    } else {
        None
    };
    Some((filename, workspace))
}

// ── File search ───────────────────────────────────────────────────────────────

async fn find_file_on_disk(filename: &str, workspace: Option<&str>) -> Result<PathBuf, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| "C:\\Users".to_string());
    let home = PathBuf::from(home);

    let search_roots = vec![
        home.clone(),
        home.join("Documents"),
        home.join("Desktop"),
        home.join("source"),
        home.join("src"),
        home.join("code"),
        home.join("dev"),
        home.join("projects"),
        home.join("work"),
        home.join("repos"),
        home.join("github"),
        PathBuf::from("C:\\code"),
        PathBuf::from("C:\\projects"),
        PathBuf::from("C:\\src"),
    ];

    let filename = filename.to_string();
    let workspace = workspace.map(|s| s.to_string());

    tokio::task::spawn_blocking(move || {
        let mut candidates: Vec<PathBuf> = Vec::new();

        for root in &search_roots {
            if !root.exists() {
                continue;
            }
            search_dir(root, &filename, 0, 5, &mut candidates);
            if candidates.len() >= 10 {
                break;
            }
        }

        if candidates.is_empty() {
            return Err("File not found on disk".to_string());
        }

        // Prefer files whose path contains the workspace name
        if let Some(ws) = &workspace {
            let ws_lower = ws.to_lowercase();
            if let Some(best) = candidates
                .iter()
                .find(|p| p.to_string_lossy().to_lowercase().contains(&ws_lower))
            {
                return Ok(best.clone());
            }
        }

        // Fall back to most recently modified
        candidates.sort_by(|a, b| {
            let at = a.metadata().and_then(|m| m.modified()).ok();
            let bt = b.metadata().and_then(|m| m.modified()).ok();
            bt.cmp(&at)
        });

        Ok(candidates.into_iter().next().unwrap())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn search_dir(
    dir: &Path,
    filename: &str,
    depth: usize,
    max_depth: usize,
    results: &mut Vec<PathBuf>,
) {
    if depth > max_depth || results.len() >= 10 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(
                name,
                "node_modules"
                    | ".git"
                    | "target"
                    | ".cargo"
                    | "__pycache__"
                    | ".venv"
                    | "dist"
                    | "build"
                    | ".next"
                    | "out"
            ) {
                continue;
            }
            search_dir(&path, filename, depth + 1, max_depth, results);
        } else if path.file_name().and_then(|n| n.to_str()) == Some(filename) {
            results.push(path);
        }
    }
}

// ── Language detection ────────────────────────────────────────────────────────

pub fn detect_language(filename: &str) -> Option<&'static str> {
    let ext = filename.rsplit('.').next()?;
    match ext {
        "rs" => Some("Rust"),
        "ts" | "tsx" => Some("TypeScript"),
        "js" | "jsx" | "mjs" => Some("JavaScript"),
        "py" => Some("Python"),
        "go" => Some("Go"),
        "cpp" | "cc" | "cxx" => Some("C++"),
        "c" | "h" => Some("C"),
        "java" => Some("Java"),
        "cs" => Some("C#"),
        "rb" => Some("Ruby"),
        "swift" => Some("Swift"),
        "kt" => Some("Kotlin"),
        "toml" => Some("TOML"),
        "json" => Some("JSON"),
        "yaml" | "yml" => Some("YAML"),
        "md" => Some("Markdown"),
        "html" | "htm" => Some("HTML"),
        "css" | "scss" | "sass" => Some("CSS"),
        "sh" | "bash" | "zsh" => Some("Shell"),
        _ => None,
    }
}

fn detect_language_from_content(s: &str) -> Option<&'static str> {
    if s.contains("fn main()")
        || s.contains("use std::")
        || (s.contains("impl ") && s.contains("->"))
    {
        Some("Rust")
    } else if s.contains("import React") || s.contains(": React.FC") || s.contains("useState") {
        Some("TypeScript")
    } else if s.contains("def ") && s.contains("self") {
        Some("Python")
    } else if s.contains("package main") || s.contains("func main()") {
        Some("Go")
    } else if s.contains("function ") && (s.contains("const ") || s.contains("let ")) {
        Some("JavaScript")
    } else {
        None
    }
}

// ── Symbol extraction ─────────────────────────────────────────────────────────

fn truncate_content(content: &str, max_lines: usize) -> String {
    content
        .lines()
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn extract_symbols(content: &str, language: &str) -> Vec<Symbol> {
    let mut symbols: Vec<Symbol> = content
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let line_num = (i + 1) as u32;
            let trimmed = line.trim();
            match language {
                "Rust" => rust_symbol(trimmed, line_num),
                "TypeScript" | "JavaScript" => js_ts_symbol(trimmed, line_num),
                "Python" => py_symbol(trimmed, line_num),
                "Go" => go_symbol(trimmed, line_num),
                _ => None,
            }
        })
        .collect();

    symbols.truncate(20);
    symbols
}

fn rust_symbol(line: &str, n: u32) -> Option<Symbol> {
    let s = line
        .trim_start_matches("pub ")
        .trim_start_matches("pub(crate) ")
        .trim_start_matches("async ")
        .trim_start_matches("pub async ");

    if s.starts_with("fn ") {
        let name = s[3..].split('(').next()?.trim().to_string();
        if !name.is_empty() {
            return Some(Symbol {
                kind: "fn".into(),
                name,
                line: n,
            });
        }
    } else if s.starts_with("struct ") {
        let name = s[7..]
            .split(|c| c == '{' || c == '<')
            .next()?
            .trim()
            .to_string();
        if !name.is_empty() {
            return Some(Symbol {
                kind: "struct".into(),
                name,
                line: n,
            });
        }
    } else if s.starts_with("enum ") {
        let name = s[5..]
            .split(|c| c == '{' || c == '<')
            .next()?
            .trim()
            .to_string();
        if !name.is_empty() {
            return Some(Symbol {
                kind: "enum".into(),
                name,
                line: n,
            });
        }
    } else if s.starts_with("trait ") {
        let name = s[6..]
            .split(|c| c == '{' || c == '<')
            .next()?
            .trim()
            .to_string();
        if !name.is_empty() {
            return Some(Symbol {
                kind: "trait".into(),
                name,
                line: n,
            });
        }
    } else if s.starts_with("impl ") {
        let name = s[5..].split('{').next()?.trim().to_string();
        if !name.is_empty() {
            return Some(Symbol {
                kind: "impl".into(),
                name,
                line: n,
            });
        }
    }
    None
}

fn js_ts_symbol(line: &str, n: u32) -> Option<Symbol> {
    let s = line
        .trim_start_matches("export ")
        .trim_start_matches("default ")
        .trim_start_matches("async ");

    if s.starts_with("function ") {
        let name = s[9..].split('(').next()?.trim().to_string();
        if !name.is_empty() {
            return Some(Symbol {
                kind: "function".into(),
                name,
                line: n,
            });
        }
    } else if s.starts_with("class ") {
        let name = s[6..]
            .split(|c| c == '{' || c == ' ')
            .next()?
            .trim()
            .to_string();
        if !name.is_empty() {
            return Some(Symbol {
                kind: "class".into(),
                name,
                line: n,
            });
        }
    } else if s.starts_with("interface ") {
        let name = s[10..]
            .split(|c| c == '{' || c == '<')
            .next()?
            .trim()
            .to_string();
        if !name.is_empty() {
            return Some(Symbol {
                kind: "interface".into(),
                name,
                line: n,
            });
        }
    } else if s.starts_with("type ") {
        let name = s[5..]
            .split(|c| c == '=' || c == '<')
            .next()?
            .trim()
            .to_string();
        if !name.is_empty() {
            return Some(Symbol {
                kind: "type".into(),
                name,
                line: n,
            });
        }
    } else if let Some(after) = s.strip_prefix("const ").or_else(|| s.strip_prefix("let ")) {
        if let Some((name, rest)) = after.split_once('=') {
            let name = name.trim().to_string();
            let rest = rest.trim();
            if rest.starts_with('(') || rest.starts_with("async") || rest.starts_with("function") {
                if !name.is_empty() {
                    return Some(Symbol {
                        kind: "function".into(),
                        name,
                        line: n,
                    });
                }
            }
        }
    }
    None
}

fn py_symbol(line: &str, n: u32) -> Option<Symbol> {
    if line.starts_with("def ") || line.starts_with("async def ") {
        let after = if line.starts_with("async ") {
            &line[10..]
        } else {
            &line[4..]
        };
        let name = after.split('(').next()?.trim().to_string();
        if !name.is_empty() {
            return Some(Symbol {
                kind: "def".into(),
                name,
                line: n,
            });
        }
    } else if line.starts_with("class ") {
        let name = line[6..]
            .split(|c| c == '(' || c == ':')
            .next()?
            .trim()
            .to_string();
        if !name.is_empty() {
            return Some(Symbol {
                kind: "class".into(),
                name,
                line: n,
            });
        }
    }
    None
}

fn go_symbol(line: &str, n: u32) -> Option<Symbol> {
    if let Some(after) = line.strip_prefix("func ") {
        let name = if after.starts_with('(') {
            after
                .split(')')
                .nth(1)?
                .trim()
                .split('(')
                .next()?
                .trim()
                .to_string()
        } else {
            after.split('(').next()?.trim().to_string()
        };
        if !name.is_empty() {
            return Some(Symbol {
                kind: "func".into(),
                name,
                line: n,
            });
        }
    } else if let Some(after) = line.strip_prefix("type ") {
        let name = after.split_whitespace().next()?.to_string();
        if !name.is_empty() {
            return Some(Symbol {
                kind: "type".into(),
                name,
                line: n,
            });
        }
    }
    None
}
