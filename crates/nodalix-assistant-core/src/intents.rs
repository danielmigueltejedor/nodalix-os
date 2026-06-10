use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntentManifest {
    pub app: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub intents: Vec<IntentDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntentDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub input_schema: serde_json::Value,
    #[serde(default)]
    pub output_schema: serde_json::Value,
    #[serde(default)]
    pub requires_confirmation: bool,
    #[serde(default)]
    pub background_capable: bool,
    #[serde(default)]
    pub modifies_user_data: bool,
}

#[derive(Debug, Clone, Default)]
pub struct IntentRegistry {
    manifests: Vec<IntentManifest>,
}

impl IntentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_manifest(mut self, manifest: IntentManifest) -> Self {
        self.manifests.push(manifest);
        self
    }

    pub fn load_from_paths(paths: &[PathBuf]) -> Self {
        let mut registry = Self::new();
        for path in paths {
            if path.is_file() {
                if let Some(manifest) = read_manifest(path) {
                    registry.manifests.push(manifest);
                }
                continue;
            }
            let Ok(entries) = fs::read_dir(path) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|name| name.ends_with(".intent.json"))
                    .unwrap_or(false)
                {
                    if let Some(manifest) = read_manifest(&path) {
                        registry.manifests.push(manifest);
                    }
                }
            }
        }
        registry
    }

    pub fn default_paths() -> Vec<PathBuf> {
        let root = find_repo_root().unwrap_or_else(|| PathBuf::from("."));
        vec![root.join("config/nodalix/intents"), root.join("apps")]
    }

    pub fn load_default() -> Self {
        Self::load_from_paths(&Self::default_paths())
    }

    pub fn intents(&self) -> Vec<(&str, &IntentDefinition)> {
        self.manifests
            .iter()
            .flat_map(|manifest| {
                manifest
                    .intents
                    .iter()
                    .map(move |intent| (manifest.app.as_str(), intent))
            })
            .collect()
    }

    pub fn find(&self, id: &str) -> Option<(&str, &IntentDefinition)> {
        self.intents()
            .into_iter()
            .find(|(_, intent)| intent.id == id)
    }

    pub fn search(&self, query: &str) -> Vec<(&str, &IntentDefinition)> {
        let query = query.to_lowercase();
        self.intents()
            .into_iter()
            .filter(|(_, intent)| {
                let hay = format!(
                    "{} {} {} {}",
                    intent.id,
                    intent.name,
                    intent.description,
                    intent.examples.join(" ")
                )
                .to_lowercase();
                hay.contains(&query)
            })
            .collect()
    }
}

fn find_repo_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    cwd.ancestors()
        .find(|path| path.join("NODALIX_CONTEXT.md").is_file())
        .map(PathBuf::from)
}

fn read_manifest(path: &PathBuf) -> Option<IntentManifest> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<IntentManifest>(&raw).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_finds_intent() {
        let manifest = IntentManifest {
            app: "nodalix-contacts".to_string(),
            description: String::new(),
            intents: vec![IntentDefinition {
                id: "contacts.findPerson".to_string(),
                name: "Find person".to_string(),
                description: "Find a contact".to_string(),
                permissions: vec!["contacts.read".to_string()],
                examples: vec!["dónde vive José".to_string()],
                input_schema: serde_json::Value::Null,
                output_schema: serde_json::Value::Null,
                requires_confirmation: false,
                background_capable: true,
                modifies_user_data: false,
            }],
        };
        let registry = IntentRegistry::new().with_manifest(manifest);
        assert!(registry.find("contacts.findPerson").is_some());
        assert_eq!(registry.search("José").len(), 1);
    }
}
