use crate::context::{normalize, ContextSearchResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersonCandidate {
    pub name: String,
    pub confidence: f32,
    pub source_title: String,
}

pub fn extract_person_hint(query: &str) -> Option<String> {
    let normalized = normalize(query);
    let stop = [
        "donde",
        "vive",
        "esta",
        "está",
        "cual",
        "cuál",
        "es",
        "la",
        "el",
        "de",
        "del",
        "direccion",
        "dirección",
    ];
    normalized
        .split_whitespace()
        .find(|token| token.len() >= 3 && !stop.contains(token))
        .map(str::to_string)
}

pub fn rank_person_candidates(hint: &str, results: &[ContextSearchResult]) -> Vec<PersonCandidate> {
    let hint = normalize(hint);
    let mut candidates = results
        .iter()
        .filter(|result| result.document.connector == "contacts")
        .map(|result| {
            let name = result.document.title.clone();
            let normalized_name = normalize(&name);
            let mut confidence = result.score;
            if normalized_name == hint {
                confidence += 0.3;
            } else if normalized_name.contains(&hint) {
                confidence += 0.2;
            }
            PersonCandidate {
                name,
                confidence: confidence.min(1.0),
                source_title: result.document.source.title.clone(),
            }
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_name_from_address_question() {
        assert_eq!(
            extract_person_hint("¿Dónde vive José?").as_deref(),
            Some("jose")
        );
    }
}
