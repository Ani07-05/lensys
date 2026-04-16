use reqwest::Client;
use serde_json::{json, Value};

const COLLECTION: &str = "cluddy_memories";
const VECTOR_SIZE: usize = 256;

/// Simple deterministic embedding: bag-of-chars n-grams hashed into a fixed-size float vector.
/// Not semantic but consistent — good enough for cosine similarity on short text chunks.
pub fn embed(text: &str) -> Vec<f32> {
    let mut vec = vec![0.0f32; VECTOR_SIZE];
    let bytes = text.as_bytes();

    for window in bytes.windows(3) {
        let h = window.iter().fold(2166136261u32, |acc, &b| {
            acc.wrapping_mul(16777619).wrapping_add(b as u32)
        });
        let idx = (h as usize) % VECTOR_SIZE;
        vec[idx] += 1.0;
    }

    // L2-normalize
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-8 {
        vec.iter_mut().for_each(|x| *x /= norm);
    }
    vec
}

pub struct QdrantClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl QdrantClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        }
    }

    fn auth(&self) -> (&'static str, String) {
        ("api-key", self.api_key.clone())
    }

    pub async fn ensure_collection(&self) -> Result<(), String> {
        let url = format!("{}/collections/{}", self.base_url, COLLECTION);

        // Check if it exists
        let res = self
            .client
            .get(&url)
            .header(self.auth().0, self.auth().1)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if res.status().as_u16() == 404 {
            // Create it
            let create_res = self
                .client
                .put(&url)
                .header(self.auth().0, self.auth().1)
                .json(&json!({
                    "vectors": {
                        "size": VECTOR_SIZE,
                        "distance": "Cosine"
                    }
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
        let vector = embed(text);
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
                    "payload": {
                        "text": text,
                        "type": point_type,
                        "timestamp": ts
                    }
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
        let vector = embed(query);
        let url = format!("{}/collections/{}/points/search", self.base_url, COLLECTION);

        let res = self
            .client
            .post(&url)
            .header(self.auth().0, self.auth().1)
            .json(&json!({
                "vector": vector,
                "limit": limit,
                "with_payload": true,
                "score_threshold": 0.3
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
            .filter_map(|item| {
                item["payload"]["text"].as_str().map(|s| s.to_string())
            })
            .collect();

        Ok(results)
    }
}
