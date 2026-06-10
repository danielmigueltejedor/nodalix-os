use crate::{
    context::{ContextIndex, SourceRef},
    entities::{extract_person_hint, rank_person_candidates},
    intents::IntentRegistry,
    permissions::{PermissionBroker, PermissionRequest},
    providers::ModelProvider,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuggestedAction {
    pub title: String,
    pub intent_id: String,
    pub requires_confirmation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantResponse {
    pub answer: String,
    pub confidence: f32,
    pub sources: Vec<SourceRef>,
    pub suggested_actions: Vec<SuggestedAction>,
    pub missing_permissions: Vec<String>,
}

pub struct AssistantCore<P: ModelProvider> {
    pub app_id: String,
    pub provider: P,
    pub intents: IntentRegistry,
    pub permissions: PermissionBroker,
    pub context: ContextIndex,
}

impl<P: ModelProvider> AssistantCore<P> {
    pub fn ask(&self, query: &str) -> AssistantResponse {
        let intent_id = self
            .provider
            .classify_intent(query)
            .unwrap_or_else(|_| "assistant.answer".to_string());

        match intent_id.as_str() {
            "contacts.findPerson" => self.answer_person_query(query),
            "mail.search" => self.permission_gated_placeholder(
                "mail.search",
                "mail.read",
                "buscar correos relevantes para la consulta",
            ),
            _ => AssistantResponse {
                answer: self
                    .provider
                    .chat(&[crate::providers::ProviderMessage {
                        role: "user".to_string(),
                        content: query.to_string(),
                    }])
                    .unwrap_or_else(|e| format!("No pude generar respuesta local: {e}")),
                confidence: 0.25,
                sources: Vec::new(),
                suggested_actions: self.intent_suggestions(query),
                missing_permissions: Vec::new(),
            },
        }
    }

    fn answer_person_query(&self, query: &str) -> AssistantResponse {
        let permission = PermissionRequest {
            app: self.app_id.clone(),
            intent: "contacts.findPerson".to_string(),
            permission: "contacts.read".to_string(),
            purpose: "resolver una persona mencionada por el usuario".to_string(),
        };
        let decision = self.permissions.request(&permission);
        if !decision.granted {
            return AssistantResponse {
                answer: "Necesito permiso para leer contactos antes de resolver esa persona."
                    .to_string(),
                confidence: 0.0,
                sources: Vec::new(),
                suggested_actions: vec![SuggestedAction {
                    title: "Conceder permiso contacts.read en Settings".to_string(),
                    intent_id: "settings.openPanel".to_string(),
                    requires_confirmation: true,
                }],
                missing_permissions: vec!["contacts.read".to_string()],
            };
        }

        let hint = extract_person_hint(query).unwrap_or_else(|| query.to_string());
        let results = self.context.search(&hint, 8);
        let candidates = rank_person_candidates(&hint, &results);

        if candidates.len() > 1 && candidates[0].confidence - candidates[1].confidence < 0.12 {
            let names = candidates
                .iter()
                .take(3)
                .map(|candidate| candidate.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return AssistantResponse {
                answer: format!("Tengo varios candidatos: {names}. ¿Cuál quieres?"),
                confidence: candidates[0].confidence,
                sources: results
                    .iter()
                    .take(3)
                    .map(|result| result.document.source.clone())
                    .collect(),
                suggested_actions: vec![SuggestedAction {
                    title: "Elegir contacto".to_string(),
                    intent_id: "contacts.findPerson".to_string(),
                    requires_confirmation: false,
                }],
                missing_permissions: Vec::new(),
            };
        }

        if let Some(best) = results.first() {
            let excerpt = best
                .document
                .source
                .excerpt
                .clone()
                .unwrap_or_else(|| best.document.body.clone());
            return AssistantResponse {
                answer: format!(
                    "Creo que te refieres a {}. Fuente interna: {}",
                    best.document.title, excerpt
                ),
                confidence: best.score,
                sources: vec![best.document.source.clone()],
                suggested_actions: vec![SuggestedAction {
                    title: "Abrir contacto".to_string(),
                    intent_id: "contacts.findPerson".to_string(),
                    requires_confirmation: false,
                }],
                missing_permissions: Vec::new(),
            };
        }

        AssistantResponse {
            answer: "No encontré una persona relacionada en el índice local.".to_string(),
            confidence: 0.0,
            sources: Vec::new(),
            suggested_actions: self.intent_suggestions(query),
            missing_permissions: Vec::new(),
        }
    }

    fn permission_gated_placeholder(
        &self,
        intent: &str,
        permission: &str,
        purpose: &str,
    ) -> AssistantResponse {
        let request = PermissionRequest {
            app: self.app_id.clone(),
            intent: intent.to_string(),
            permission: permission.to_string(),
            purpose: purpose.to_string(),
        };
        let decision = self.permissions.request(&request);
        if !decision.granted {
            return AssistantResponse {
                answer: format!("Necesito permiso {permission} para continuar."),
                confidence: 0.0,
                sources: Vec::new(),
                suggested_actions: vec![SuggestedAction {
                    title: format!("Revisar permiso {permission}"),
                    intent_id: "settings.openPanel".to_string(),
                    requires_confirmation: true,
                }],
                missing_permissions: vec![permission.to_string()],
            };
        }
        AssistantResponse {
            answer: "El permiso está concedido; falta conectar el conector real.".to_string(),
            confidence: 0.2,
            sources: Vec::new(),
            suggested_actions: self.intent_suggestions(intent),
            missing_permissions: Vec::new(),
        }
    }

    fn intent_suggestions(&self, query: &str) -> Vec<SuggestedAction> {
        self.intents
            .search(query)
            .into_iter()
            .take(3)
            .map(|(_, intent)| SuggestedAction {
                title: intent.name.clone(),
                intent_id: intent.id.clone(),
                requires_confirmation: intent.requires_confirmation,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::{ContextDocument, SourceRef},
        providers::MockProvider,
    };

    #[test]
    fn does_not_read_contacts_without_permission() {
        let base = std::env::temp_dir().join(format!("nodalix-assistant-{}", std::process::id()));
        let core = AssistantCore {
            app_id: "nodalix-assistant".to_string(),
            provider: MockProvider,
            intents: IntentRegistry::new(),
            permissions: PermissionBroker::new(
                base.join("permissions.json"),
                base.join("audit.log"),
            ),
            context: ContextIndex::new(base.join("index.json")),
        };
        let response = core.ask("¿Dónde vive José?");
        assert!(response
            .missing_permissions
            .contains(&"contacts.read".to_string()));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn answers_person_query_with_source_when_allowed() {
        let base =
            std::env::temp_dir().join(format!("nodalix-assistant-ok-{}", std::process::id()));
        let context = ContextIndex::new(base.join("index.json"));
        context
            .rebuild(vec![ContextDocument {
                id: "jose".to_string(),
                connector: "contacts".to_string(),
                title: "José García".to_string(),
                body: "José vive en Calle Luna 4".to_string(),
                entities: vec!["jose".to_string()],
                source: SourceRef {
                    connector: "contacts".to_string(),
                    title: "José García".to_string(),
                    uri: Some("contacts://jose".to_string()),
                    excerpt: Some("Calle Luna 4".to_string()),
                },
                timestamp: Some("2026-05-12".to_string()),
            }])
            .unwrap();
        let permissions =
            PermissionBroker::new(base.join("permissions.json"), base.join("audit.log"));
        permissions
            .grant("nodalix-assistant", "contacts.read")
            .unwrap();
        let core = AssistantCore {
            app_id: "nodalix-assistant".to_string(),
            provider: MockProvider,
            intents: IntentRegistry::new(),
            permissions,
            context,
        };
        let response = core.ask("¿Dónde vive José?");
        assert!(response.answer.contains("José García"));
        assert_eq!(response.sources.len(), 1);
        let _ = std::fs::remove_dir_all(base);
    }
}
