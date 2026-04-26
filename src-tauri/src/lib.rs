mod commands;
mod api;

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
    pub transcript: Mutex<Vec<api::TranscriptEntry>>,
    pub cursor_pos: Mutex<(i32, i32)>,
    pub workspace_path: std::path::PathBuf,
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

fn workspace_root() -> std::path::PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let root = if cwd.file_name().and_then(|name| name.to_str()) == Some("src-tauri") {
        cwd.parent().unwrap_or(cwd.as_path()).to_path_buf()
    } else {
        cwd
    };
    root.canonicalize().unwrap_or(root)
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

/// Copies the active selection first, then reads it from the clipboard.
#[tauri::command]
async fn capture_selection_context(
    window: tauri::WebviewWindow,
    state: State<'_, Arc<AppState>>,
) -> Result<commands::CodeContext, String> {
    copy_selection_to_clipboard_native();
    let ctx = read_clipboard_context_with_retry().await?;

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

/// Agentic text-mode: infer or generate a concrete edit for the current context.
#[tauri::command]
async fn propose_code_action(
    instruction: String,
    state: State<'_, Arc<AppState>>,
) -> Result<commands::CodeActionProposal, String> {
    let claude = commands::ClaudeClient::new(state.http_client.clone(), state.groq_api_key.clone());
    let ctx = lock_or_recover(&state.last_code_context)
        .clone()
        .unwrap_or_default();
    claude.propose_code_action(&ctx, &instruction).await
}

/// Applies a previously generated code action after validating the target path and old text.
#[tauri::command]
async fn apply_code_action(
    request: commands::ApplyCodeActionRequest,
) -> Result<commands::ApplyCodeActionResult, String> {
    commands::apply_code_action(request).await
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
            let width = 380.0f64;
            let height = 240.0f64;
            window
                .set_size(tauri::Size::Logical(tauri::LogicalSize { width, height }))
                .map_err(|e| e.to_string())?;
            window.set_resizable(false).ok();
            if let Ok(Some(monitor)) = window.current_monitor() {
                // Derive logical screen width from physical size ÷ scale factor.
                let scale = monitor.scale_factor();
                let phys = monitor.size();
                let screen_w = phys.width as f64 / scale;
                let x = screen_w - width - 12.0;
                // 40 logical px clears the menu bar on both standard and notch Macs.
                let y = 40.0f64;
                window
                    .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }))
                    .ok();
            }
            window.show().ok();
            window.set_focus().ok();
        }
        "calling" => {
            state.is_expanded.store(false, Ordering::Relaxed);
            window
                .set_size(tauri::Size::Logical(tauri::LogicalSize {
                    width: 340.0,
                    height: 118.0,
                }))
                .map_err(|e| e.to_string())?;
            window.set_resizable(false).ok();
        }
        _ => {
            state.is_expanded.store(false, Ordering::Relaxed);
            window
                .set_size(tauri::Size::Logical(tauri::LogicalSize {
                    width: 96.0,
                    height: 76.0,
                }))
                .map_err(|e| e.to_string())?;
            window.set_resizable(false).ok();
        }
    }
    Ok(())
}

#[tauri::command]
async fn resize_panel(height: u32, window: tauri::WebviewWindow) -> Result<(), String> {
    // height comes from scrollHeight (logical CSS px) — use Logical to match.
    let height = (height as f64).clamp(168.0, 460.0);
    window
        .set_size(tauri::Size::Logical(tauri::LogicalSize {
            width: 380.0,
            height,
        }))
        .map_err(|e| e.to_string())
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
pub(crate) fn copy_selection_to_clipboard_native() {
    use winapi::um::winuser::{keybd_event, KEYEVENTF_KEYUP, VK_CONTROL};

    const C_KEY: u8 = b'C';

    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text("");
    }

    unsafe {
        keybd_event(VK_CONTROL as u8, 0, 0, 0);
        keybd_event(C_KEY, 0, 0, 0);
        keybd_event(C_KEY, 0, KEYEVENTF_KEYUP, 0);
        keybd_event(VK_CONTROL as u8, 0, KEYEVENTF_KEYUP, 0);
    }
}

#[cfg(target_os = "macos")]
fn get_cursor_pos_native() -> (i32, i32) {
    use std::ffi::c_void;

    #[repr(C)]
    struct CGPoint { x: f64, y: f64 }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventCreate(source: *const c_void) -> *const c_void;
        fn CGEventGetLocation(event: *const c_void) -> CGPoint;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRelease(cf: *const c_void);
    }

    unsafe {
        let ev = CGEventCreate(std::ptr::null());
        if ev.is_null() { return (0, 0); }
        let pt = CGEventGetLocation(ev);
        CFRelease(ev);
        (pt.x as i32, pt.y as i32)
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn copy_selection_to_clipboard_native() {
    use std::ffi::c_void;

    // Use CGEvent directly — more reliable than enigo for a one-shot Cmd+C.
    // kCGEventSourceStateHIDSystemState = 1, kCGHIDEventTap = 0
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventCreateKeyboardEvent(
            source: *const c_void,
            virtual_key: u16,
            key_down: bool,
        ) -> *const c_void;
        fn CGEventSetFlags(event: *const c_void, flags: u64);
        fn CGEventPost(tap: u32, event: *const c_void);
        fn CFRelease(cf: *const c_void);
    }

    unsafe {
        // Virtual key 8 = 'c', kCGEventFlagMaskCommand = 1 << 20
        const C_KEY: u16 = 8;
        const CMD_FLAG: u64 = 1 << 20;
        const HID_TAP: u32 = 0;

        let down = CGEventCreateKeyboardEvent(std::ptr::null(), C_KEY, true);
        CGEventSetFlags(down, CMD_FLAG);
        CGEventPost(HID_TAP, down);
        CFRelease(down);

        let up = CGEventCreateKeyboardEvent(std::ptr::null(), C_KEY, false);
        CGEventSetFlags(up, CMD_FLAG);
        CGEventPost(HID_TAP, up);
        CFRelease(up);
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn get_cursor_pos_native() -> (i32, i32) {
    (100, 100)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub(crate) fn copy_selection_to_clipboard_native() {}

async fn read_clipboard_context_with_retry() -> Result<commands::CodeContext, String> {
    let mut last_error = "Clipboard is empty".to_string();

    for delay_ms in [80_u64, 140, 220, 320, 480, 650] {
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        match commands::get_clipboard_code_context().await {
            Ok(ctx) => return Ok(ctx),
            Err(err) => last_error = err,
        }
    }

    #[cfg(target_os = "macos")]
    let hint = "Select text in your editor, then press Cmd+Shift+T. \
                If it keeps failing, grant Accessibility in System Settings → Privacy.";
    #[cfg(not(target_os = "macos"))]
    let hint = "Select text in the editor, then press Ctrl+Shift+T without focusing Cluddy first.";

    Err(format!("{last_error}. {hint}"))
}

// ── macOS permission requests ─────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod macos_permissions {
    use std::ffi::{c_char, c_void};

    type CFTypeRef = *const c_void;
    type CFDictionaryRef = *const c_void;
    type CFAllocatorRef = *const c_void;
    type CFStringRef = *const c_void;
    type CFIndex = isize;

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        static kCFBooleanTrue: CFTypeRef;
        // Declared as u8 so we can safely take their address without dereferencing.
        static kCFTypeDictionaryKeyCallBacks: u8;
        static kCFTypeDictionaryValueCallBacks: u8;
        fn CFStringCreateWithCString(
            alloc: CFAllocatorRef,
            c_str: *const c_char,
            encoding: u32,
        ) -> CFStringRef;
        fn CFDictionaryCreate(
            allocator: CFAllocatorRef,
            keys: *const CFTypeRef,
            values: *const CFTypeRef,
            num_values: CFIndex,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> CFDictionaryRef;
        fn CFRelease(cf: CFTypeRef);
    }

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
    }

    /// Triggers the macOS system dialogs for Screen Recording and Accessibility.
    /// Both dialogs are only shown if the permission is not already granted.
    pub fn request() {
        unsafe {
            // ── Screen Recording ──────────────────────────────────────────────
            if !CGPreflightScreenCaptureAccess() {
                // Opens System Settings > Privacy > Screen Recording on first run.
                CGRequestScreenCaptureAccess();
            }

            // ── Accessibility ─────────────────────────────────────────────────
            if !AXIsProcessTrusted() {
                // AXIsProcessTrustedWithOptions({kAXTrustedCheckOptionPrompt: true})
                // shows the system Accessibility permission dialog.
                const UTF8: u32 = 0x08000100;
                let key_cstr = b"AXTrustedCheckOptionPrompt\0";
                let key = CFStringCreateWithCString(
                    std::ptr::null(),
                    key_cstr.as_ptr() as *const c_char,
                    UTF8,
                );
                let keys: [CFTypeRef; 1] = [key];
                let vals: [CFTypeRef; 1] = [kCFBooleanTrue];
                let dict = CFDictionaryCreate(
                    std::ptr::null(),
                    keys.as_ptr(),
                    vals.as_ptr(),
                    1,
                    &kCFTypeDictionaryKeyCallBacks as *const u8 as *const c_void,
                    &kCFTypeDictionaryValueCallBacks as *const u8 as *const c_void,
                );
                AXIsProcessTrustedWithOptions(dict);
                CFRelease(dict);
                CFRelease(key);
            }
        }
    }
}

// ── macOS full-screen overlay ─────────────────────────────────────────────────

/// Sets NSWindowCollectionBehavior so the floating window appears on top of
/// full-screen spaces, not just normal desktops.
#[cfg(target_os = "macos")]
fn setup_window_for_fullscreen(win: &tauri::WebviewWindow) {
    use std::ffi::{c_char, c_void};

    // Alias objc_msgSend with the exact signature we need to avoid variadic UB.
    #[link(name = "objc", kind = "dylib")]
    extern "C" {
        #[link_name = "objc_msgSend"]
        fn msg_send_u64(recv: *mut c_void, sel: *const c_void, val: u64);
    }

    #[link(name = "Foundation", kind = "framework")]
    extern "C" {
        fn sel_registerName(name: *const c_char) -> *const c_void;
    }

    if let Ok(ns_window) = win.ns_window() {
        unsafe {
            let sel = sel_registerName(b"setCollectionBehavior:\0".as_ptr() as *const c_char);
            // NSWindowCollectionBehaviorCanJoinAllSpaces    = 1 << 0 = 1
            // NSWindowCollectionBehaviorFullScreenAuxiliary = 1 << 8 = 256
            let behavior: u64 = (1 << 0) | (1 << 8);
            msg_send_u64(ns_window as *mut c_void, sel, behavior);
        }
    }
}

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
            transcript: Mutex::new(Vec::new()),
        cursor_pos: Mutex::new((0, 0)),
            workspace_path: workspace_root(),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(state)
        .setup(|app| {
            use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

            // Ask for Screen Recording and Accessibility on macOS.
            #[cfg(target_os = "macos")]
            macos_permissions::request();

            let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
            let win = app
                .get_webview_window("main")
                .expect("main window not found");

            // Allow the window to overlay full-screen spaces on macOS.
            #[cfg(target_os = "macos")]
            setup_window_for_fullscreen(&win);

            // Initialize wiki directory once Tauri's async runtime is available.
            let wiki_init_path = state.wiki_path.clone();
            tauri::async_runtime::spawn(async move {
                let wiki = commands::WikiManager::new(wiki_init_path);
                let _ = wiki.ensure_initialized().await;
            });

            let api_state = Arc::clone(&state);
            tauri::async_runtime::spawn(async move {
                if let Err(err) = api::serve(api_state).await {
                    eprintln!("Cluddy API failed: {err}");
                }
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
                            let _ = win_tracker.set_position(tauri::Position::Logical(
                                tauri::LogicalPosition {
                                    x: cx as f64 + 20.0,
                                    y: cy as f64 + 20.0,
                                },
                            ));
                            last_x = cx;
                            last_y = cy;
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            });

            // Use Cmd on macOS, Ctrl on Windows/Linux
            #[cfg(target_os = "macos")]
            let (mod_a, mod_s, mod_t, mod_b) = ("Super+Shift+A", "Super+Shift+S", "Super+Shift+T", "Super+Shift+B");
            #[cfg(not(target_os = "macos"))]
            let (mod_a, mod_s, mod_t, mod_b) = ("Ctrl+Shift+A", "Ctrl+Shift+S", "Ctrl+Shift+T", "Ctrl+Shift+B");

            // Cmd/Ctrl+Shift+A — start/stop call
            let win_a = win.clone();
            app.global_shortcut()
                .on_shortcut(mod_a, move |_app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        win_a.emit("cluddy:hotkey", ()).ok();
                    }
                })?;

            // Cmd/Ctrl+Shift+S — toggle panel
            let win_s = win.clone();
            app.global_shortcut()
                .on_shortcut(mod_s, move |_app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        win_s.emit("cluddy:panel", ()).ok();
                    }
                })?;

            // Cmd/Ctrl+Shift+T — toggle text input mode
            let win_t = win.clone();
            app.global_shortcut()
                .on_shortcut(mod_t, move |_app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        win_t.emit("cluddy:text_mode", ()).ok();
                    }
                })?;

            // Cmd/Ctrl+Shift+B — cycle the visible buddy
            let win_b = win.clone();
            app.global_shortcut()
                .on_shortcut(mod_b, move |_app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        win_b.emit("cluddy:buddy", ()).ok();
                    }
                })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_env_keys,
            get_code_context,
            get_clipboard_context,
            capture_selection_context,
            ask_claude,
            propose_code_action,
            apply_code_action,
            web_search,
            wiki_read,
            wiki_write,
            wiki_list,
            wiki_search,
            wiki_update_from_turn,
            capture_and_analyze,
            get_memories,
            set_window_mode,
            resize_panel,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Cluddy");
}
