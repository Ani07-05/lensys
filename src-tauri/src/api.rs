use crate::{commands, AppState};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{net::TcpListener, process::Command};

const API_ADDR: &str = "127.0.0.1:17373";

type ApiState = Arc<AppState>;
type ApiResult<T> = Result<Json<T>, ApiError>;

#[derive(Debug)]
struct ApiError(String);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscriptEntry {
    pub role: String,
    pub text: String,
    pub kind: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
struct AskRequest {
    question: String,
}

#[derive(Debug, Deserialize)]
struct WriteRequest {
    instruction: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchRequest {
    query: String,
}

#[derive(Debug, Deserialize)]
struct WikiPageRequest {
    page: String,
}

#[derive(Debug, Deserialize)]
struct ToolReadRequest {
    path: String,
}

#[derive(Debug, Deserialize)]
struct ToolRgRequest {
    query: String,
    path: Option<String>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    ok: bool,
    api: &'static str,
}

#[derive(Debug, Serialize)]
struct WriteResponse {
    proposal: commands::CodeActionProposal,
    result: commands::ApplyCodeActionResult,
}

#[derive(Debug, Serialize)]
struct ReadResponse {
    path: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct RgResponse {
    command: String,
    output: String,
}

pub async fn serve(state: ApiState) -> Result<(), String> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/context", get(context))
        .route("/capture", post(capture))
        .route("/ask", post(ask))
        .route("/write", post(write))
        .route("/web/search", post(web_search))
        .route("/wiki/search", post(wiki_search))
        .route("/wiki/read", post(wiki_read))
        .route("/transcript", get(transcript))
        .route("/transcript/clear", post(clear_transcript))
        .route("/tools/read", post(tool_read))
        .route("/tools/rg", post(tool_rg))
        .with_state(state);

    let addr: SocketAddr = API_ADDR.parse::<SocketAddr>().map_err(|e| e.to_string())?;
    let listener = TcpListener::bind(addr).await.map_err(|e| e.to_string())?;
    println!("Cluddy API listening on http://{API_ADDR}");
    axum::serve(listener, app).await.map_err(|e| e.to_string())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        api: "cluddy",
    })
}

async fn context(State(state): State<ApiState>) -> ApiResult<commands::CodeContext> {
    let ctx = commands::get_active_code_context().await.map_err(ApiError)?;
    *lock(&state.last_code_context) = Some(ctx.clone());
    Ok(Json(ctx))
}

async fn capture(State(state): State<ApiState>) -> ApiResult<commands::CodeContext> {
    crate::copy_selection_to_clipboard_native();
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    let ctx = commands::get_clipboard_code_context().await.map_err(ApiError)?;
    *lock(&state.last_code_context) = Some(ctx.clone());
    Ok(Json(ctx))
}

async fn ask(State(state): State<ApiState>, Json(req): Json<AskRequest>) -> ApiResult<serde_json::Value> {
    let question = req.question.trim();
    if question.is_empty() {
        return Err(ApiError("question is required".to_string()));
    }

    push_transcript(&state, "user", question, "ask");
    let claude = commands::ClaudeClient::new(state.http_client.clone(), state.groq_api_key.clone());
    let ctx = lock(&state.last_code_context).clone().unwrap_or_default();
    let answer = claude.explain_code(&ctx, question).await.map_err(ApiError)?;
    push_transcript(&state, "assistant", &answer, "ask");
    Ok(Json(json!({ "answer": answer })))
}

async fn write(State(state): State<ApiState>, Json(req): Json<WriteRequest>) -> ApiResult<WriteResponse> {
    let instruction = req.instruction.unwrap_or_default();
    let claude = commands::ClaudeClient::new(state.http_client.clone(), state.groq_api_key.clone());
    let ctx = lock(&state.last_code_context).clone().unwrap_or_default();

    push_transcript(
        &state,
        "user",
        if instruction.trim().is_empty() {
            "Infer and apply the best edit."
        } else {
            instruction.trim()
        },
        "write",
    );

    let proposal = claude
        .propose_code_action(&ctx, &instruction)
        .await
        .map_err(ApiError)?;
    let result = commands::apply_code_action(commands::ApplyCodeActionRequest {
        target_file: proposal
            .target_file
            .clone()
            .ok_or_else(|| ApiError("proposal has no target_file".to_string()))?,
        old_text: proposal.old_text.clone(),
        replacement: proposal.replacement.clone(),
    })
    .await
    .map_err(ApiError)?;

    push_transcript(&state, "assistant", &result.message, "write");
    Ok(Json(WriteResponse { proposal, result }))
}

async fn web_search(State(state): State<ApiState>, Json(req): Json<SearchRequest>) -> ApiResult<Vec<commands::SearchResult>> {
    let results = commands::search_web(&state.http_client, &state.tavily_api_key, req.query.trim())
        .await
        .map_err(ApiError)?;
    Ok(Json(results))
}

async fn wiki_search(State(state): State<ApiState>, Json(req): Json<SearchRequest>) -> ApiResult<Vec<String>> {
    let wiki = commands::WikiManager::new(state.wiki_path.clone());
    wiki.search(req.query.trim()).await.map(Json).map_err(ApiError)
}

async fn wiki_read(State(state): State<ApiState>, Json(req): Json<WikiPageRequest>) -> ApiResult<serde_json::Value> {
    let wiki = commands::WikiManager::new(state.wiki_path.clone());
    let content = wiki.read_page(req.page.trim()).await.map_err(ApiError)?;
    Ok(Json(json!({ "page": req.page, "content": content })))
}

async fn transcript(State(state): State<ApiState>) -> ApiResult<Vec<TranscriptEntry>> {
    Ok(Json(lock(&state.transcript).clone()))
}

async fn clear_transcript(State(state): State<ApiState>) -> ApiResult<serde_json::Value> {
    lock(&state.transcript).clear();
    Ok(Json(json!({ "ok": true })))
}

async fn tool_read(State(state): State<ApiState>, Json(req): Json<ToolReadRequest>) -> ApiResult<ReadResponse> {
    let path = safe_path(&state, &req.path)?;
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| ApiError(format!("read failed: {e}")))?;
    Ok(Json(ReadResponse {
        path: path.to_string_lossy().to_string(),
        content,
    }))
}

async fn tool_rg(State(state): State<ApiState>, Json(req): Json<ToolRgRequest>) -> ApiResult<RgResponse> {
    let query = req.query.trim();
    if query.is_empty() {
        return Err(ApiError("query is required".to_string()));
    }

    let search_path = match req.path.as_deref() {
        Some(path) if !path.trim().is_empty() => safe_path(&state, path)?,
        _ => state.workspace_path.clone(),
    };

    let output = Command::new("rg")
        .arg("--line-number")
        .arg("--color")
        .arg("never")
        .arg(query)
        .arg(&search_path)
        .current_dir(&state.workspace_path)
        .output()
        .await
        .map_err(|e| ApiError(format!("rg failed: {e}")))?;

    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.is_empty() {
        text = String::from_utf8_lossy(&output.stderr).to_string();
    }
    if !output.status.success() && text.is_empty() {
        text = "no matches".to_string();
    }

    Ok(Json(RgResponse {
        command: format!("rg --line-number --color never {query} {}", search_path.display()),
        output: text,
    }))
}

fn safe_path(state: &AppState, input: &str) -> Result<PathBuf, ApiError> {
    let raw = Path::new(input);
    let candidate = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        state.workspace_path.join(raw)
    };

    let canonical = candidate
        .canonicalize()
        .map_err(|e| ApiError(format!("path not found: {e}")))?;

    if canonical.starts_with(&state.workspace_path) || canonical.starts_with(&state.wiki_path) {
        return Ok(canonical);
    }

    Err(ApiError("path is outside workspace/wiki".to_string()))
}

fn push_transcript(state: &AppState, role: &str, text: &str, kind: &str) {
    lock(&state.transcript).push(TranscriptEntry {
        role: role.to_string(),
        text: text.to_string(),
        kind: kind.to_string(),
        timestamp: chrono::Local::now().to_rfc3339(),
    });
}

fn lock<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(json!({ "error": self.0 }))).into_response()
    }
}
