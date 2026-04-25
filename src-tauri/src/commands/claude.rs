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

        self.call_groq(
            "llama-3.3-70b-versatile",
            1024,
            &format!("{question}{code_block}"),
        )
        .await
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
