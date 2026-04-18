use reqwest::Client;
use serde_json::{json, Value};

const COLLECTION: &str = "cluddy_memories_v2";
const VECTOR_SIZE: usize = 768;

async fn embed(client: &Client, text: &str, groq_api_key: &str) -> Result<Vec<f32>, String> {
    let res = client
        .post("https://api.groq.com/openai/v1/embeddings")
        .header("Authorization", format!("Bearer {groq_api_key}"))
        .json(&json!({ "model": "nomic-embed-text-v1.5", "input": text }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: Value = res.json().await.map_err(|e| e.to_string())?;
    let embedding: Vec<f32> = body["data"][0]["embedding"]
        .as_array()
        .ok_or_else(|| "no embedding in response".to_string())?
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();

    if embedding.len() != VECTOR_SIZE {
        return Err(format!("expected {VECTOR_SIZE} dims, got {}", embedding.len()));
    }
    Ok(embedding)
}

pub struct QdrantClient<'a> {
    client: &'a Client,
    base_url: String,
    api_key: String,
    groq_api_key: String,
}

impl<'a> QdrantClient<'a> {
    pub fn new(client: &'a Client, base_url: &str, api_key: &str, groq_api_key: &str) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            groq_api_key: groq_api_key.to_string(),
        }
    }

    fn auth(&self) -> (&'static str, String) {
        ("api-key", self.api_key.clone())
    }

    pub async fn ensure_collection(&self) -> Result<(), String> {
        let url = format!("{}/collections/{}", self.base_url, COLLECTION);

        let res = self
            .client
            .get(&url)
            .header(self.auth().0, self.auth().1)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if res.status().as_u16() == 404 {
            let create_res = self
                .client
                .put(&url)
                .header(self.auth().0, self.auth().1)
                .json(&json!({
                    "vectors": { "size": VECTOR_SIZE, "distance": "Cosine" }
                }))
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !create_res.status().is_success() {
                let body: Value = create_res.json().await.unwrap_or_default();
                return Err(format!("Failed to create collection: {body}"));
            }
        }

        Ok(())
    }

    pub async fn upsert(&self, text: &str, point_type: &str) -> Result<(), String> {
        let vector = embed(self.client, text, &self.groq_api_key).await?;
        let id = uuid::Uuid::new_v4().to_string();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let url = format!("{}/collections/{}/points", self.base_url, COLLECTION);

        let res = self
            .client
            .put(&url)
            .header(self.auth().0, self.auth().1)
            .json(&json!({
                "points": [{
                    "id": id,
                    "vector": vector,
                    "payload": { "text": text, "type": point_type, "timestamp": ts }
                }]
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let body: Value = res.json().await.unwrap_or_default();
            return Err(format!("Upsert failed: {body}"));
        }

        Ok(())
    }

    pub async fn search(&self, query: &str, limit: u64) -> Result<Vec<String>, String> {
        let vector = embed(self.client, query, &self.groq_api_key).await?;
        let url = format!("{}/collections/{}/points/search", self.base_url, COLLECTION);

        let res = self
            .client
            .post(&url)
            .header(self.auth().0, self.auth().1)
            .json(&json!({
                "vector": vector,
                "limit": limit,
                "with_payload": true,
                "score_threshold": 0.6
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            return Ok(vec![]);
        }

        let body: Value = res.json().await.map_err(|e| e.to_string())?;
        let results: Vec<String> = body["result"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|item| item["payload"]["text"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(results)
    }
}
