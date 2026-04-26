use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplyCodeActionRequest {
    pub target_file: String,
    pub old_text: String,
    pub replacement: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplyCodeActionResult {
    pub target_file: String,
    pub changed: bool,
    pub message: String,
}

pub async fn apply_code_action(
    req: ApplyCodeActionRequest,
) -> Result<ApplyCodeActionResult, String> {
    if req.target_file.trim().is_empty() {
        return Err("No target file for this action".to_string());
    }
    if req.old_text.is_empty() {
        return Err("Cannot apply an empty selection".to_string());
    }

    let target = PathBuf::from(&req.target_file);
    let workspace = workspace_root()?;
    let target = target
        .canonicalize()
        .map_err(|e| format!("Target file not found: {e}"))?;

    if !target.starts_with(&workspace) {
        return Err("Refusing to edit a file outside this workspace".to_string());
    }

    let original = tokio::fs::read_to_string(&target)
        .await
        .map_err(|e| format!("Read failed: {e}"))?;

    if !original.contains(&req.old_text) {
        return Err("Selected text no longer matches the file".to_string());
    }

    if req.old_text == req.replacement {
        return Ok(ApplyCodeActionResult {
            target_file: target.to_string_lossy().to_string(),
            changed: false,
            message: "No change needed".to_string(),
        });
    }

    let updated = original.replacen(&req.old_text, &req.replacement, 1);
    tokio::fs::write(&target, updated)
        .await
        .map_err(|e| format!("Write failed: {e}"))?;

    Ok(ApplyCodeActionResult {
        target_file: target.to_string_lossy().to_string(),
        changed: true,
        message: "Applied change".to_string(),
    })
}

fn workspace_root() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let root = if cwd.file_name().and_then(|name| name.to_str()) == Some("src-tauri") {
        cwd.parent().unwrap_or(cwd.as_path()).to_path_buf()
    } else {
        cwd
    };

    root.canonicalize().map_err(|e| e.to_string())
}
