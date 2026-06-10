use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRef {
    pub connector: String,
    pub title: String,
    pub uri: Option<String>,
    pub excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextDocument {
    pub id: String,
    pub connector: String,
    pub title: String,
    pub body: String,
    pub entities: Vec<String>,
    pub source: SourceRef,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextSearchResult {
    pub document: ContextDocument,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct ContextIndex {
    path: PathBuf,
}

impl ContextIndex {
    pub fn default_path() -> PathBuf {
        let data_home = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".local/share")
            });
        data_home.join("nodalix/assistant/context-index.json")
    }

    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default() -> Self {
        Self::new(Self::default_path())
    }

    pub fn load_documents(&self) -> Vec<ContextDocument> {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Vec<ContextDocument>>(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save_documents(&self, documents: &[ContextDocument]) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("No se pudo crear {}: {e}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(documents)
            .map_err(|e| format!("No se pudo serializar el índice: {e}"))?;
        fs::write(&self.path, raw)
            .map_err(|e| format!("No se pudo escribir {}: {e}", self.path.display()))
    }

    pub fn rebuild(&self, documents: Vec<ContextDocument>) -> Result<(), String> {
        self.save_documents(&documents)
    }

    pub fn clear(&self) -> Result<(), String> {
        self.save_documents(&[])
    }

    pub fn add_or_replace(&self, document: ContextDocument) -> Result<(), String> {
        let mut documents = self.load_documents();
        documents.retain(|existing| existing.id != document.id);
        documents.push(document);
        self.save_documents(&documents)
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<ContextSearchResult> {
        let query = normalize(query);
        if query.is_empty() {
            return Vec::new();
        }

        let mut results = self
            .load_documents()
            .into_iter()
            .filter_map(|document| {
                let hay = normalize(&format!(
                    "{} {} {}",
                    document.title,
                    document.body,
                    document.entities.join(" ")
                ));
                let score = score_match(&query, &hay, &document);
                (score > 0.0).then_some(ContextSearchResult { document, score })
            })
            .collect::<Vec<_>>();

        results.sort_by_key(|result| {
            (
                Reverse((result.score * 1000.0) as i64),
                Reverse(result.document.timestamp.clone().unwrap_or_default()),
            )
        });
        results.truncate(limit);
        results
    }
}

fn score_match(query: &str, hay: &str, document: &ContextDocument) -> f32 {
    if hay.contains(query) {
        let title = normalize(&document.title);
        if title == query {
            return 1.0;
        }
        if title.contains(query) {
            return 0.9;
        }
        return 0.72;
    }

    let query_tokens = query.split_whitespace().collect::<Vec<_>>();
    if query_tokens.is_empty() {
        return 0.0;
    }
    let hits = query_tokens
        .iter()
        .filter(|token| hay.contains(**token))
        .count();
    if hits > 0 {
        return 0.35 + (hits as f32 / query_tokens.len() as f32) * 0.25;
    }
    if fuzzy_subsequence(query, hay) {
        return 0.24;
    }
    0.0
}

pub fn normalize(value: &str) -> String {
    let folded = value
        .trim()
        .to_lowercase()
        .replace('á', "a")
        .replace('é', "e")
        .replace('í', "i")
        .replace('ó', "o")
        .replace('ú', "u")
        .replace('ñ', "n");
    folded
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn fuzzy_subsequence(needle: &str, haystack: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let mut chars = needle.chars();
    let mut current = chars.next();
    for c in haystack.chars() {
        if Some(c) == current {
            current = chars.next();
            if current.is_none() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_returns_sources() {
        let path =
            std::env::temp_dir().join(format!("nodalix-context-test-{}.json", std::process::id()));
        let index = ContextIndex::new(&path);
        index
            .rebuild(vec![ContextDocument {
                id: "contact-jose".to_string(),
                connector: "contacts".to_string(),
                title: "José García".to_string(),
                body: "Vive en Calle Luna 4".to_string(),
                entities: vec!["jose".to_string(), "direccion".to_string()],
                source: SourceRef {
                    connector: "contacts".to_string(),
                    title: "José García".to_string(),
                    uri: Some("contacts://jose".to_string()),
                    excerpt: Some("Calle Luna 4".to_string()),
                },
                timestamp: Some("2026-05-12".to_string()),
            }])
            .unwrap();

        let results = index.search("donde vive jose", 3);
        assert_eq!(results[0].document.title, "José García");
        let _ = std::fs::remove_file(path);
    }
}
