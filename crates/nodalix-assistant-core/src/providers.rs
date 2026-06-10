use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderMessage {
    pub role: String,
    pub content: String,
}

pub trait ModelProvider {
    fn chat(&self, messages: &[ProviderMessage]) -> Result<String, String>;
    fn summarize(&self, text: &str) -> Result<String, String>;
    fn classify_intent(&self, text: &str) -> Result<String, String>;
}

#[derive(Debug, Clone, Default)]
pub struct MockProvider;

impl ModelProvider for MockProvider {
    fn chat(&self, messages: &[ProviderMessage]) -> Result<String, String> {
        let last = messages
            .last()
            .map(|m| m.content.as_str())
            .unwrap_or_default();
        Ok(format!("Respuesta local de desarrollo para: {last}"))
    }

    fn summarize(&self, text: &str) -> Result<String, String> {
        Ok(text
            .split_whitespace()
            .take(24)
            .collect::<Vec<_>>()
            .join(" "))
    }

    fn classify_intent(&self, text: &str) -> Result<String, String> {
        let lower = text.to_lowercase();
        if lower.contains("vive") || lower.contains("donde") || lower.contains("dónde") {
            return Ok("contacts.findPerson".to_string());
        }
        if lower.contains("correo") || lower.contains("email") {
            return Ok("mail.search".to_string());
        }
        Ok("assistant.answer".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_classifies_person_query() {
        assert_eq!(
            MockProvider.classify_intent("¿Dónde vive José?").unwrap(),
            "contacts.findPerson"
        );
    }
}
