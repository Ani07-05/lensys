use std::path::PathBuf;
use tokio::fs;

pub struct WikiManager {
    pub base_path: PathBuf,
}

impl WikiManager {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub async fn ensure_initialized(&self) -> Result<(), String> {
        fs::create_dir_all(&self.base_path)
            .await
            .map_err(|e| e.to_string())?;
        fs::create_dir_all(self.base_path.join("entities"))
            .await
            .map_err(|e| e.to_string())?;

        let index = self.base_path.join("index.md");
        if !index.exists() {
            fs::write(&index, INDEX_TEMPLATE)
                .await
                .map_err(|e| e.to_string())?;
        }

        let log = self.base_path.join("log.md");
        if !log.exists() {
            fs::write(&log, "# Cluddy Session Log\n\n")
                .await
                .map_err(|e| e.to_string())?;
        }

        let schema = self.base_path.join("SCHEMA.md");
        if !schema.exists() {
            fs::write(&schema, SCHEMA_TEMPLATE)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub async fn read_page(&self, name: &str) -> Result<String, String> {
        fs::read_to_string(self.page_path(name))
            .await
            .map_err(|e| format!("Wiki read error: {e}"))
    }

    pub async fn write_page(&self, name: &str, content: &str) -> Result<(), String> {
        let path = self.page_path(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        fs::write(&path, content)
            .await
            .map_err(|e| format!("Wiki write error: {e}"))
    }

    pub async fn list_pages(&self) -> Result<Vec<String>, String> {
        let mut pages = Vec::new();

        if let Ok(mut entries) = fs::read_dir(&self.base_path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                if name.ends_with(".md")
                    && !matches!(name.as_str(), "index.md" | "log.md" | "SCHEMA.md")
                {
                    pages.push(name.trim_end_matches(".md").to_string());
                }
            }
        }

        let entities_dir = self.base_path.join("entities");
        if let Ok(mut entries) = fs::read_dir(&entities_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                if name.ends_with(".md") {
                    pages.push(format!("entities/{}", name.trim_end_matches(".md")));
                }
            }
        }

        Ok(pages)
    }

    pub async fn search(&self, query: &str) -> Result<Vec<String>, String> {
        let pages = self.list_pages().await?;
        let query_lower = query.to_lowercase();
        let words: Vec<&str> = query_lower.split_whitespace().collect();
        let mut scored: Vec<(String, usize)> = Vec::new();

        for page in &pages {
            let Ok(content) = fs::read_to_string(self.page_path(page)).await else {
                continue;
            };
            let content_lower = content.to_lowercase();
            let score: usize = words.iter().map(|w| content_lower.matches(w).count()).sum();
            if score > 0 {
                let snippet = content
                    .lines()
                    .find(|l| words.iter().any(|w| l.to_lowercase().contains(w)))
                    .unwrap_or("")
                    .chars()
                    .take(120)
                    .collect::<String>();
                scored.push((format!("**{page}**: {snippet}"), score));
            }
        }

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(scored.into_iter().take(3).map(|(s, _)| s).collect())
    }

    pub async fn read_index(&self) -> Result<String, String> {
        fs::read_to_string(self.base_path.join("index.md"))
            .await
            .map_err(|e| format!("Wiki index error: {e}"))
    }

    pub async fn update_index(&self) -> Result<(), String> {
        let pages = self.list_pages().await?;
        let mut lines = vec![
            "# Cluddy Wiki Index\n".to_string(),
            "_Auto-managed by Cluddy. Edit via conversation._\n".to_string(),
            "## Pages\n".to_string(),
        ];
        for p in &pages {
            lines.push(format!("- [{p}]({p}.md)"));
        }
        fs::write(self.base_path.join("index.md"), lines.join("\n"))
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn append_log(&self, entry: &str) -> Result<(), String> {
        let path = self.base_path.join("log.md");
        let existing = fs::read_to_string(&path).await.unwrap_or_default();
        fs::write(&path, format!("{existing}{entry}\n"))
            .await
            .map_err(|e| format!("Wiki log error: {e}"))
    }

    fn page_path(&self, name: &str) -> PathBuf {
        self.base_path.join(format!("{name}.md"))
    }
}

const INDEX_TEMPLATE: &str =
    "# Cluddy Wiki Index\n\n_Auto-managed by Cluddy. Edit via conversation._\n\n## Pages\n\n";

const SCHEMA_TEMPLATE: &str = r#"# Cluddy Wiki Schema

This wiki is maintained automatically by Cluddy during your dev sessions.

## Page Types
- **entities/**: Concepts, tools, languages, frameworks
- **index.md**: Catalog of all pages (auto-updated)
- **log.md**: Append-only session log
- **SCHEMA.md**: This file

## Conventions
- Page names: kebab-case (e.g., `entities/rust-lifetimes`)
- Each page starts with a one-line summary
- Contradictions noted as `> ⚠️ Contradicts: ...`
- Code blocks use fenced syntax with language tags

## Log entry format
```
## [2026-04-18] session | topic summary
```
"#;
