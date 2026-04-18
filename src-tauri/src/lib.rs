mod commands;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{Emitter, Manager, State};

pub struct AppState {
    pub is_expanded: AtomicBool,
    pub groq_api_key: String,
    pub vapi_public_key: String,
    pub vapi_assistant_id: String,
    pub qdrant_url: String,
    pub qdrant_api_key: String,
    pub http_client: reqwest::Client,
    // Screen-diff: skip Groq when the screen hasn't changed meaningfully
    pub last_screenshot_hash: Mutex<Option<commands::ScreenHash>>,
    pub last_analysis: Mutex<String>,
    pub last_analysis_time: Mutex<std::time::Instant>,
    // Share cursor position with the capture command so it can pick the right monitor
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
}

/// Recover from a poisoned mutex instead of crashing.
fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

#[tauri::command]
async fn get_env_keys(state: State<'_, Arc<AppState>>) -> Result<EnvKeys, String> {
    Ok(EnvKeys {
        vapi_public_key: state.vapi_public_key.clone(),
        vapi_assistant_id: state.vapi_assistant_id.clone(),
    })
}

#[tauri::command]
async fn capture_and_analyze(
    window: tauri::WebviewWindow,
    state: State<'_, Arc<AppState>>,
) -> Result<AnalysisResult, String> {
    let cursor_pos = *lock_or_recover(&state.cursor_pos);
    let (b64, hash) = commands::capture_screen_at_cursor(cursor_pos)?;

    let has_groq = !state.groq_api_key.is_empty()
        && state.groq_api_key != "your_groq_api_key_here";
    let has_qdrant = !state.qdrant_url.is_empty() && !state.qdrant_api_key.is_empty();

    // Screen diff: skip Groq entirely if nothing on screen changed
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
        // Return cached result and emit so the frontend still gets notified
        let cached = lock_or_recover(&state.last_analysis).clone();
        let _ = window.emit("cluddy:analysis", &cached);
        cached
    } else {
        // Groq vision + Qdrant collection setup run in parallel
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
                let client =
                    commands::QdrantClient::new(&state.http_client, &state.qdrant_url, &state.qdrant_api_key, &state.groq_api_key);
                let _ = client.ensure_collection().await;
            }
        };

        let (a, _) = tokio::join!(groq_fut, qdrant_setup_fut);

        // Cache and push to frontend immediately — don't wait for Qdrant search/upsert
        *lock_or_recover(&state.last_analysis) = a.clone();
        *lock_or_recover(&state.last_analysis_time) = std::time::Instant::now();
        let _ = window.emit("cluddy:analysis", &a);
        a
    };

    // Qdrant search + upsert run in parallel (both only need the analysis text)
    let memories = if has_qdrant {
        let client = commands::QdrantClient::new(&state.http_client, &state.qdrant_url, &state.qdrant_api_key, &state.groq_api_key);
        // ensure_collection was already called in the parallel block above for changed screens;
        // call it here for cache-hit turns so search/upsert don't fail on a missing collection.
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
    let client = commands::QdrantClient::new(&state.http_client, &state.qdrant_url, &state.qdrant_api_key, &state.groq_api_key);
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
                    width: 380,
                    height: 500,
                }))
                .map_err(|e| e.to_string())?;
            window.set_resizable(false).ok();
            // Center on screen
            if let Ok(Some(monitor)) = window.current_monitor() {
                let screen = monitor.size();
                let x = (screen.width as i32 / 2) - 190;
                let y = (screen.height as i32 / 2) - 250;
                window
                    .set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
                    .ok();
            }
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

#[cfg(not(target_os = "windows"))]
fn get_cursor_pos_native() -> (i32, i32) {
    (100, 100)
}

pub fn run() {
    dotenvy::dotenv().ok();

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .pool_max_idle_per_host(4)
        .build()
        .expect("failed to build HTTP client");

    let state = Arc::new(AppState {
        is_expanded: AtomicBool::new(false),
        groq_api_key: std::env::var("GROQ_API_KEY").unwrap_or_default(),
        vapi_public_key: std::env::var("VAPI_PUBLIC_KEY").unwrap_or_default(),
        vapi_assistant_id: std::env::var("VAPI_ASSISTANT_ID").unwrap_or_default(),
        qdrant_url: std::env::var("QDRANT_URL").unwrap_or_default(),
        qdrant_api_key: std::env::var("QDRANT_API_KEY").unwrap_or_default(),
        http_client,
        last_screenshot_hash: Mutex::new(None),
        last_analysis: Mutex::new(String::new()),
        last_analysis_time: Mutex::new(std::time::Instant::now()),
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

            // Cursor-following background thread — only moves window when cursor actually moved
            let win_tracker = win.clone();
            let tracker_state = Arc::clone(&state);
            std::thread::spawn(move || {
                let mut last_x = -9999i32;
                let mut last_y = -9999i32;
                loop {
                    let (cx, cy) = get_cursor_pos_native();

                    // Always update cursor pos so capture_and_analyze can pick the right monitor
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
                    // ~30fps for smooth cursor following
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            });

            // Ctrl+Shift+A — start/stop call
            let win_shortcut = win.clone();
            app.global_shortcut()
                .on_shortcut("Ctrl+Shift+A", move |_app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        win_shortcut.emit("cluddy:hotkey", ()).ok();
                    }
                })?;

            // Ctrl+Shift+S — toggle full panel
            let win_panel = win.clone();
            app.global_shortcut()
                .on_shortcut("Ctrl+Shift+S", move |_app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        win_panel.emit("cluddy:panel", ()).ok();
                    }
                })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_env_keys,
            capture_and_analyze,
            get_memories,
            set_window_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Cluddy");
}
