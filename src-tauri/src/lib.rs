mod commands;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{Emitter, Manager, State};

pub struct AppState {
    pub is_expanded: AtomicBool,
    pub groq_api_key: String,
    pub tavily_api_key: String,
    pub vapi_public_key: String,
    pub vapi_assistant_id: String,
    pub qdrant_url: String,
    pub qdrant_api_key: String,
    pub http_client: reqwest::Client,
    pub wiki_path: std::path::PathBuf,
    // Screen-diff cache
    pub last_screenshot_hash: Mutex<Option<commands::ScreenHash>>,
    pub last_analysis: Mutex<String>,
    pub last_analysis_time: Mutex<std::time::Instant>,
    // Code context cache
    pub last_code_context: Mutex<Option<commands::CodeContext>>,
    pub cursor_pos: Mutex<(i32, i32)>,
}

#[derive(serde::Serialize)]
pub struct AnalysisResult {
    analysis: String,
    memories: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct EnvKeys {
    vapi_public_key: String,
    vapi_assistant_id: String,
    has_claude: bool,
    has_search: bool,
    has_groq: bool,
}

fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_env_keys(state: State<'_, Arc<AppState>>) -> Result<EnvKeys, String> {
    Ok(EnvKeys {
        vapi_public_key: state.vapi_public_key.clone(),
        vapi_assistant_id: state.vapi_assistant_id.clone(),
        has_claude: !state.groq_api_key.is_empty() && !state.groq_api_key.starts_with("your_"),
        has_search: !state.tavily_api_key.is_empty() && !state.tavily_api_key.starts_with("your_"),
        has_groq: !state.groq_api_key.is_empty() && !state.groq_api_key.starts_with("your_"),
    })
}

/// Primary developer context command — replaces capture_and_analyze for IDE users.
#[tauri::command]
async fn get_code_context(
    window: tauri::WebviewWindow,
    state: State<'_, Arc<AppState>>,
) -> Result<commands::CodeContext, String> {
    let ctx = commands::get_active_code_context().await?;

    // Cache for ask_claude to use
    *lock_or_recover(&state.last_code_context) = Some(ctx.clone());

    // Emit so frontend can react immediately
    let _ = window.emit("cluddy:code_context", &ctx);

    // Async: search Qdrant with code context summary
    if !state.qdrant_url.is_empty() {
        let summary = format!(
            "{} {}",
            ctx.file_name.as_deref().unwrap_or(""),
            ctx.symbols
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        );
        let client = commands::QdrantClient::new(
            &state.http_client,
            &state.qdrant_url,
            &state.qdrant_api_key,
            &state.groq_api_key,
        );
        let _ = client.upsert(&summary, "code_context").await;
    }

    Ok(ctx)
}

/// Reads the current clipboard as an explicit code context.
#[tauri::command]
async fn get_clipboard_context(
    window: tauri::WebviewWindow,
    state: State<'_, Arc<AppState>>,
) -> Result<commands::CodeContext, String> {
    let ctx = commands::get_clipboard_code_context().await?;

    *lock_or_recover(&state.last_code_context) = Some(ctx.clone());
    let _ = window.emit("cluddy:code_context", &ctx);

    Ok(ctx)
}

/// Text-mode: ask Claude directly about the current code context.
#[tauri::command]
async fn ask_claude(question: String, state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let claude = commands::ClaudeClient::new(state.http_client.clone(), state.groq_api_key.clone());
    let ctx = lock_or_recover(&state.last_code_context)
        .clone()
        .unwrap_or_default();
    claude.explain_code(&ctx, &question).await
}

/// Web search via Tavily.
#[tauri::command]
async fn web_search(
    query: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<commands::SearchResult>, String> {
    commands::search_web(&state.http_client, &state.tavily_api_key, &query).await
}

// ── Wiki commands ─────────────────────────────────────────────────────────────

#[tauri::command]
async fn wiki_read(page: String, state: State<'_, Arc<AppState>>) -> Result<String, String> {
    commands::WikiManager::new(state.wiki_path.clone())
        .read_page(&page)
        .await
}

#[tauri::command]
async fn wiki_write(
    page: String,
    content: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let wiki = commands::WikiManager::new(state.wiki_path.clone());
    wiki.write_page(&page, &content).await?;
    wiki.update_index().await
}

#[tauri::command]
async fn wiki_list(state: State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
    commands::WikiManager::new(state.wiki_path.clone())
        .list_pages()
        .await
}

#[tauri::command]
async fn wiki_search(
    query: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    commands::WikiManager::new(state.wiki_path.clone())
        .search(&query)
        .await
}

/// Called from frontend after a conversation turn ends — async wiki update.
#[tauri::command]
async fn wiki_update_from_turn(
    turn: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let wiki = commands::WikiManager::new(state.wiki_path.clone());
    let index = wiki.read_index().await.unwrap_or_default();

    let claude = commands::ClaudeClient::new(state.http_client.clone(), state.groq_api_key.clone());

    let diffs = claude.synthesize_wiki_update(&turn, &index).await?;

    for diff in &diffs {
        wiki.write_page(&diff.page, &diff.content).await?;
    }

    if !diffs.is_empty() {
        wiki.update_index().await?;
        let date = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        let entry = format!("## [{date}] session | {} pages updated", diffs.len());
        wiki.append_log(&entry).await?;
    }

    Ok(())
}

// ── Legacy screenshot + analysis (kept as fallback for non-IDE contexts) ──────

#[tauri::command]
async fn capture_and_analyze(
    window: tauri::WebviewWindow,
    state: State<'_, Arc<AppState>>,
) -> Result<AnalysisResult, String> {
    let cursor_pos = *lock_or_recover(&state.cursor_pos);
    let (b64, hash) = commands::capture_screen_at_cursor(cursor_pos)?;

    let has_groq = !state.groq_api_key.is_empty() && !state.groq_api_key.starts_with("your_");
    let has_qdrant = !state.qdrant_url.is_empty() && !state.qdrant_api_key.is_empty();

    let screen_changed = {
        let mut last_hash = lock_or_recover(&state.last_screenshot_hash);
        let last_time = lock_or_recover(&state.last_analysis_time);
        let cache_expired = last_time.elapsed().as_secs() >= 2;
        let hash_changed = last_hash.map_or(true, |h| commands::screens_differ(&h, &hash));
        let changed = hash_changed || cache_expired;
        if changed {
            *last_hash = Some(hash);
        }
        changed
    };

    let analysis = if !screen_changed {
        let cached = lock_or_recover(&state.last_analysis).clone();
        let _ = window.emit("cluddy:analysis", &cached);
        cached
    } else {
        let groq_fut = async {
            if has_groq {
                commands::analyze_screenshot(&state.http_client, &b64, &state.groq_api_key)
                    .await
                    .unwrap_or_else(|e| format!("Vision unavailable: {e}"))
            } else {
                "No Groq API key configured.".to_string()
            }
        };
        let qdrant_setup_fut = async {
            if has_qdrant {
                let client = commands::QdrantClient::new(
                    &state.http_client,
                    &state.qdrant_url,
                    &state.qdrant_api_key,
                    &state.groq_api_key,
                );
                let _ = client.ensure_collection().await;
            }
        };
        let (a, _) = tokio::join!(groq_fut, qdrant_setup_fut);
        *lock_or_recover(&state.last_analysis) = a.clone();
        *lock_or_recover(&state.last_analysis_time) = std::time::Instant::now();
        let _ = window.emit("cluddy:analysis", &a);
        a
    };

    let memories = if has_qdrant {
        let client = commands::QdrantClient::new(
            &state.http_client,
            &state.qdrant_url,
            &state.qdrant_api_key,
            &state.groq_api_key,
        );
        if !screen_changed {
            let _ = client.ensure_collection().await;
        }
        let (past, _) = tokio::join!(
            client.search(&analysis, 5),
            client.upsert(&analysis, "screen_analysis")
        );
        past.unwrap_or_default()
    } else {
        vec![]
    };

    Ok(AnalysisResult { analysis, memories })
}

#[tauri::command]
async fn get_memories(
    query: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    if state.qdrant_url.is_empty() || state.qdrant_api_key.is_empty() {
        return Ok(vec![]);
    }
    let client = commands::QdrantClient::new(
        &state.http_client,
        &state.qdrant_url,
        &state.qdrant_api_key,
        &state.groq_api_key,
    );
    client.search(&query, 5).await
}

#[tauri::command]
async fn set_window_mode(
    mode: String,
    window: tauri::WebviewWindow,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    match mode.as_str() {
        "expanded" => {
            state.is_expanded.store(true, Ordering::Relaxed);
            window
                .set_size(tauri::Size::Physical(tauri::PhysicalSize {
                    width: 420,
                    height: 560,
                }))
                .map_err(|e| e.to_string())?;
            window.set_resizable(false).ok();
            if let Ok(Some(monitor)) = window.current_monitor() {
                let screen = monitor.size();
                let x = (screen.width as i32 / 2) - 210;
                let y = (screen.height as i32 / 2) - 280;
                window
                    .set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
                    .ok();
            }
            window.show().ok();
            window.set_focus().ok();
        }
        _ => {
            state.is_expanded.store(false, Ordering::Relaxed);
            window
                .set_size(tauri::Size::Physical(tauri::PhysicalSize {
                    width: 280,
                    height: 44,
                }))
                .map_err(|e| e.to_string())?;
            window.set_resizable(false).ok();
        }
    }
    Ok(())
}

// ── Platform helpers ──────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn get_cursor_pos_native() -> (i32, i32) {
    use winapi::shared::windef::POINT;
    use winapi::um::winuser::GetCursorPos;
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        (pt.x, pt.y)
    }
}

#[cfg(target_os = "windows")]
fn copy_selection_to_clipboard_native() {
    use winapi::um::winuser::{keybd_event, KEYEVENTF_KEYUP, VK_CONTROL};

    const C_KEY: u8 = b'C';

    unsafe {
        keybd_event(VK_CONTROL as u8, 0, 0, 0);
        keybd_event(C_KEY, 0, 0, 0);
        keybd_event(C_KEY, 0, KEYEVENTF_KEYUP, 0);
        keybd_event(VK_CONTROL as u8, 0, KEYEVENTF_KEYUP, 0);
    }
}

#[cfg(not(target_os = "windows"))]
fn get_cursor_pos_native() -> (i32, i32) {
    (100, 100)
}

#[cfg(not(target_os = "windows"))]
fn copy_selection_to_clipboard_native() {}

// ── App entry point ───────────────────────────────────────────────────────────

pub fn run() {
    dotenvy::dotenv().ok();

    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let wiki_path = std::path::PathBuf::from(home).join(".cluddy").join("wiki");

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .pool_max_idle_per_host(4)
        .build()
        .expect("failed to build HTTP client");

    let state = Arc::new(AppState {
        is_expanded: AtomicBool::new(false),
        groq_api_key: std::env::var("GROQ_API_KEY").unwrap_or_default(),
        tavily_api_key: std::env::var("TAVILY_API_KEY").unwrap_or_default(),
        vapi_public_key: std::env::var("VAPI_PUBLIC_KEY").unwrap_or_default(),
        vapi_assistant_id: std::env::var("VAPI_ASSISTANT_ID").unwrap_or_default(),
        qdrant_url: std::env::var("QDRANT_URL").unwrap_or_default(),
        qdrant_api_key: std::env::var("QDRANT_API_KEY").unwrap_or_default(),
        http_client,
        wiki_path: wiki_path.clone(),
        last_screenshot_hash: Mutex::new(None),
        last_analysis: Mutex::new(String::new()),
        last_analysis_time: Mutex::new(std::time::Instant::now()),
        last_code_context: Mutex::new(None),
        cursor_pos: Mutex::new((0, 0)),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(state)
        .setup(|app| {
            use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

            let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
            let win = app
                .get_webview_window("main")
                .expect("main window not found");

            // Initialize wiki directory once Tauri's async runtime is available.
            let wiki_init_path = state.wiki_path.clone();
            tauri::async_runtime::spawn(async move {
                let wiki = commands::WikiManager::new(wiki_init_path);
                let _ = wiki.ensure_initialized().await;
            });

            // Cursor-following thread
            let win_tracker = win.clone();
            let tracker_state = Arc::clone(&state);
            std::thread::spawn(move || {
                let mut last_x = -9999i32;
                let mut last_y = -9999i32;
                loop {
                    let (cx, cy) = get_cursor_pos_native();
                    *lock_or_recover(&tracker_state.cursor_pos) = (cx, cy);
                    if !tracker_state.is_expanded.load(Ordering::Relaxed) {
                        let dx = (cx - last_x).abs();
                        let dy = (cy - last_y).abs();
                        if dx > 4 || dy > 4 {
                            let _ = win_tracker.set_position(tauri::Position::Physical(
                                tauri::PhysicalPosition {
                                    x: cx + 16,
                                    y: cy - 52,
                                },
                            ));
                            last_x = cx;
                            last_y = cy;
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            });

            // Ctrl+Shift+A — start/stop call
            let win_a = win.clone();
            app.global_shortcut()
                .on_shortcut("Ctrl+Shift+A", move |_app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        win_a.emit("cluddy:hotkey", ()).ok();
                    }
                })?;

            // Ctrl+Shift+S — toggle panel
            let win_s = win.clone();
            app.global_shortcut()
                .on_shortcut("Ctrl+Shift+S", move |_app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        win_s.emit("cluddy:panel", ()).ok();
                    }
                })?;

            // Ctrl+Shift+T — toggle text input mode
            let win_t = win.clone();
            app.global_shortcut()
                .on_shortcut("Ctrl+Shift+T", move |_app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let win_t = win_t.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(120));
                            copy_selection_to_clipboard_native();
                            std::thread::sleep(std::time::Duration::from_millis(90));
                            win_t.emit("cluddy:text_mode", ()).ok();
                        });
                    }
                })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_env_keys,
            get_code_context,
            get_clipboard_context,
            ask_claude,
            web_search,
            wiki_read,
            wiki_write,
            wiki_list,
            wiki_search,
            wiki_update_from_turn,
            capture_and_analyze,
            get_memories,
            set_window_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Cluddy");
}
