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
    pub qdrant: Mutex<Option<commands::QdrantClient>>,
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

#[tauri::command]
async fn get_env_keys(state: State<'_, Arc<AppState>>) -> Result<EnvKeys, String> {
    Ok(EnvKeys {
        vapi_public_key: state.vapi_public_key.clone(),
        vapi_assistant_id: state.vapi_assistant_id.clone(),
    })
}

#[tauri::command]
async fn capture_and_analyze(state: State<'_, Arc<AppState>>) -> Result<AnalysisResult, String> {
    let b64 = commands::capture_primary_screen()?;

    // Analyze with Claude Vision
    let analysis = if !state.groq_api_key.is_empty()
        && state.groq_api_key != "your_groq_api_key_here"
    {
        commands::analyze_screenshot(&b64, &state.groq_api_key)
            .await
            .unwrap_or_else(|e| format!("Vision unavailable: {e}"))
    } else {
        "No Groq API key configured.".to_string()
    };

    // Store in Qdrant & fetch related memories
    let memories = if !state.qdrant_url.is_empty() && !state.qdrant_api_key.is_empty() {
        let client = commands::QdrantClient::new(&state.qdrant_url, &state.qdrant_api_key);

        // Ensure collection exists (idempotent)
        let _ = client.ensure_collection().await;

        // Fetch relevant past memories before storing the new one
        let past = client
            .search(&analysis, 5)
            .await
            .unwrap_or_default();

        // Store current analysis
        let _ = client.upsert(&analysis, "screen_analysis").await;

        past
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
    let client = commands::QdrantClient::new(&state.qdrant_url, &state.qdrant_api_key);
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
        "orb" | _ => {
            state.is_expanded.store(false, Ordering::Relaxed);
            window
                .set_size(tauri::Size::Physical(tauri::PhysicalSize {
                    width: 80,
                    height: 80,
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

    let state = Arc::new(AppState {
        is_expanded: AtomicBool::new(false),
        groq_api_key: std::env::var("GROQ_API_KEY").unwrap_or_default(),
        vapi_public_key: std::env::var("VAPI_PUBLIC_KEY").unwrap_or_default(),
        vapi_assistant_id: std::env::var("VAPI_ASSISTANT_ID").unwrap_or_default(),
        qdrant_url: std::env::var("QDRANT_URL").unwrap_or_default(),
        qdrant_api_key: std::env::var("QDRANT_API_KEY").unwrap_or_default(),
        qdrant: Mutex::new(None),
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
            let is_expanded = Arc::clone(&state);
            std::thread::spawn(move || {
                let mut last_x = -9999i32;
                let mut last_y = -9999i32;
                loop {
                    if !is_expanded.is_expanded.load(Ordering::Relaxed) {
                        let (cx, cy) = get_cursor_pos_native();
                        let dx = (cx - last_x).abs();
                        let dy = (cy - last_y).abs();
                        if dx > 4 || dy > 4 {
                            let _ = win_tracker.set_position(tauri::Position::Physical(
                                tauri::PhysicalPosition {
                                    x: cx + 20,
                                    y: cy - 90,
                                },
                            ));
                            last_x = cx;
                            last_y = cy;
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
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
