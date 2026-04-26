use super::code_context::CodeContext;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub struct ClaudeClient {
    pub http_client: reqwest::Client,
    pub groq_api_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WikiDiff {
    pub page: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeActionProposal {
    pub summary: String,
    pub confidence: f32,
    pub target_file: Option<String>,
    pub old_text: String,
    pub replacement: String,
    pub needs_confirmation: bool,
    pub risk_notes: Vec<String>,
}

impl ClaudeClient {
    pub fn new(http_client: reqwest::Client, groq_api_key: String) -> Self {
        Self {
            http_client,
            groq_api_key,
        }
    }

    pub async fn explain_code(&self, ctx: &CodeContext, question: &str) -> Result<String, String> {
        if self.groq_api_key.is_empty() {
            return Err("Groq API key not configured".to_string());
        }

        let code_block = if let Some(content) = &ctx.content {
            format!(
                "\n\nFile: {} ({})\n```{}\n{}\n```",
                ctx.file_name.as_deref().unwrap_or("unknown"),
                ctx.language.as_deref().unwrap_or("text"),
                ctx.language.as_deref().unwrap_or("").to_lowercase(),
                content,
            )
        } else {
            String::new()
        };

        let prompt = format!(
            "Answer concisely. For non-coding questions, ignore any code context and use at most 2 short sentences. For coding questions, use the selected code only if it is relevant, and prefer direct actionable guidance over reciting code.\n\nQuestion: {question}{code_block}"
        );

        self.call_groq("llama-3.3-70b-versatile", 512, &prompt)
            .await
    }

    pub async fn propose_code_action(
        &self,
        ctx: &CodeContext,
        instruction: &str,
    ) -> Result<CodeActionProposal, String> {
        if self.groq_api_key.is_empty() {
            return Err("Groq API key not configured".to_string());
        }

        let selected = ctx.content.as_deref().unwrap_or("").trim();
        if selected.is_empty() {
            return Err("No selected code or active context found".to_string());
        }

        let prompt = format!(
            r#"You are an agentic coding assistant. The user selected code and expects you to infer the most useful edit unless they gave a specific instruction.

Return ONLY valid JSON with this exact shape:
{{
  "summary": "short human-readable edit summary",
  "confidence": 0.0,
  "target_file": "absolute or null",
  "old_text": "the exact selected text to replace",
  "replacement": "the complete replacement text",
  "needs_confirmation": true,
  "risk_notes": ["short risk notes, or empty array"]
}}

Rules:
- Prefer making a concrete improvement over explaining.
- If the instruction is blank, infer a conservative useful edit from the selected code.
- Preserve the selected code's language and surrounding style.
- old_text must be exactly the selected text provided below.
- replacement must contain only the replacement for old_text, not markdown fences.
- Set needs_confirmation true for behavior changes, file writes, broad refactors, or confidence under 0.75.
- If no useful edit is obvious, return the same old_text as replacement and explain the uncertainty in risk_notes.

Active file: {}
Language: {}
Instruction: {}

Selected code:
```{}
{}
```"#,
            ctx.file_path
                .as_deref()
                .or(ctx.file_name.as_deref())
                .unwrap_or("unknown"),
            ctx.language.as_deref().unwrap_or("text"),
            if instruction.trim().is_empty() {
                "(infer the best edit)"
            } else {
                instruction.trim()
            },
            ctx.language.as_deref().unwrap_or("").to_lowercase(),
            selected
        );

        let text = self
            .call_groq("llama-3.3-70b-versatile", 2048, &prompt)
            .await?;
        let start = text.find('{').unwrap_or(0);
        let end = text.rfind('}').map(|i| i + 1).unwrap_or(text.len());
        let mut proposal: CodeActionProposal = serde_json::from_str(&text[start..end])
            .map_err(|e| format!("Action JSON parse: {e}"))?;

        proposal.old_text = selected.to_string();
        if proposal.target_file.is_none() {
            proposal.target_file = ctx.file_path.clone();
        }
        if proposal.summary.trim().is_empty() {
            proposal.summary = "Suggested code change".to_string();
        }
        proposal.confidence = proposal.confidence.clamp(0.0, 1.0);

        Ok(proposal)
    }

    pub async fn synthesize_wiki_update(
        &self,
        turn: &str,
        wiki_index: &str,
    ) -> Result<Vec<WikiDiff>, String> {
        if self.groq_api_key.is_empty() {
            return Ok(vec![]);
        }

        let prompt = format!(
            "Given this developer conversation and wiki index, output a JSON array of wiki pages to create or update. Only include pages directly relevant to what was discussed.\n\nWiki Index:\n{wiki_index}\n\nConversation:\n{turn}\n\nOutput ONLY a JSON array like:\n[{{\"page\": \"entities/rust-lifetimes\", \"content\": \"# Rust Lifetimes\\nOne-line summary.\\n\"}}]\n\nIf nothing worth recording, output: []"
        );

        let text = self
            .call_groq("llama-3.1-8b-instant", 2048, &prompt)
            .await?;

        let start = text.find('[').unwrap_or(0);
        let end = text.rfind(']').map(|i| i + 1).unwrap_or(text.len());

        serde_json::from_str::<Vec<WikiDiff>>(&text[start..end])
            .map_err(|e| format!("Wiki JSON parse: {e}"))
    }

    async fn call_groq(
        &self,
        model: &str,
        max_tokens: u32,
        prompt: &str,
    ) -> Result<String, String> {
        let body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": [{ "role": "user", "content": prompt }]
        });

        let resp = self
            .http_client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.groq_api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Groq request failed: {e}"))?;

        let status = resp.status();
        let json: Value = resp
            .json()
            .await
            .map_err(|e| format!("Groq parse error: {e}"))?;

        if !status.is_success() {
            return Err(format!(
                "Groq API {}: {}",
                status,
                json["error"]["message"].as_str().unwrap_or("unknown")
            ));
        }

        Ok(json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }
}
