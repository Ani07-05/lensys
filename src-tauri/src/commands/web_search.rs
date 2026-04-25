use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub content: String,
}

#[derive(Serialize)]
struct TavilyRequest<'a> {
    api_key: &'a str,
    query: &'a str,
    max_results: u8,
    search_depth: &'a str,
}

#[derive(Deserialize)]
struct TavilyResponse {
    results: Vec<TavilyResult>,
}

#[derive(Deserialize)]
struct TavilyResult {
    title: String,
    url: String,
    content: String,
}

pub async fn search_web(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
) -> Result<Vec<SearchResult>, String> {
    if api_key.is_empty() || api_key.starts_with("your_") {
        return Err("Tavily API key not configured — add TAVILY_API_KEY to .env".to_string());
    }

    let req = TavilyRequest {
        api_key,
        query,
        max_results: 3,
        search_depth: "basic",
    };

    let resp = client
        .post("https://api.tavily.com/search")
        .json(&req)
        .send()
        .await
        .map_err(|e| format!("Search request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Search API error: {}", resp.status()));
    }

    let data: TavilyResponse = resp
        .json()
        .await
        .map_err(|e| format!("Search parse error: {e}"))?;

    Ok(data
        .results
        .into_iter()
        .map(|r| SearchResult {
            title: r.title,
            url: r.url,
            content: r.content.chars().take(350).collect(),
        })
        .collect())
}
