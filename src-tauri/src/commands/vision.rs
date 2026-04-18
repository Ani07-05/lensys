use reqwest::Client;
use serde_json::{json, Value};

pub async fn analyze_screenshot(client: &Client, base64_image: &str, api_key: &str) -> Result<String, String> {
    let body = json!({
        "model": "meta-llama/llama-4-scout-17b-16e-instruct",
        "max_tokens": 200,
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:image/jpeg;base64,{}", base64_image)
                    }
                },
                {
                    "type": "text",
                    "text": "One sentence: what is the user doing right now? Include app/tool name, file or URL if visible, and any error message. Cover all activity types (coding, browsing, terminal, reading docs, video, design, etc.)."
                }
            ]
        }]
    });

    let res = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = res.status();
    let json: Value = res.json().await.map_err(|e| format!("Parse error: {e}"))?;

    if !status.is_success() {
        return Err(format!(
            "Groq API error {}: {}",
            status,
            json["error"]["message"].as_str().unwrap_or("unknown")
        ));
    }

    let text = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "No content in response".to_string())?
        .to_string();

    Ok(text)
}
