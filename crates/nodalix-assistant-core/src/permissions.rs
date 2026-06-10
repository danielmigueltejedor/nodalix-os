use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionDecision {
    pub granted: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionRequest {
    pub app: String,
    pub intent: String,
    pub permission: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct AuditEntry {
    timestamp: u64,
    app: String,
    intent: String,
    permission: String,
    purpose: String,
    granted: bool,
}

#[derive(Debug, Clone)]
pub struct PermissionBroker {
    permissions_path: PathBuf,
    audit_path: PathBuf,
}

impl PermissionBroker {
    pub fn default() -> Self {
        Self::new(default_permissions_path(), default_audit_path())
    }

    pub fn new(permissions_path: impl Into<PathBuf>, audit_path: impl Into<PathBuf>) -> Self {
        Self {
            permissions_path: permissions_path.into(),
            audit_path: audit_path.into(),
        }
    }

    pub fn is_granted(&self, app: &str, permission: &str) -> bool {
        self.load_permissions()
            .get(app)
            .and_then(|perms| perms.get(permission))
            .copied()
            .unwrap_or(false)
    }

    pub fn request(&self, request: &PermissionRequest) -> PermissionDecision {
        let granted = self.is_granted(&request.app, &request.permission);
        let decision = PermissionDecision {
            granted,
            reason: (!granted).then(|| {
                format!(
                    "{} necesita permiso {} para {}",
                    request.app, request.permission, request.purpose
                )
            }),
        };
        let _ = self.audit(request, granted);
        decision
    }

    pub fn grant(&self, app: &str, permission: &str) -> Result<(), String> {
        let mut permissions = self.load_permissions();
        permissions
            .entry(app.to_string())
            .or_default()
            .insert(permission.to_string(), true);
        self.save_permissions(&permissions)
    }

    pub fn revoke(&self, app: &str, permission: &str) -> Result<(), String> {
        let mut permissions = self.load_permissions();
        if let Some(perms) = permissions.get_mut(app) {
            perms.insert(permission.to_string(), false);
        }
        self.save_permissions(&permissions)
    }

    fn load_permissions(&self) -> BTreeMap<String, BTreeMap<String, bool>> {
        fs::read_to_string(&self.permissions_path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    fn save_permissions(
        &self,
        permissions: &BTreeMap<String, BTreeMap<String, bool>>,
    ) -> Result<(), String> {
        if let Some(parent) = self.permissions_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("No se pudo crear {}: {e}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(permissions)
            .map_err(|e| format!("No se pudieron serializar permisos: {e}"))?;
        fs::write(&self.permissions_path, raw).map_err(|e| {
            format!(
                "No se pudo escribir {}: {e}",
                self.permissions_path.display()
            )
        })
    }

    fn audit(&self, request: &PermissionRequest, granted: bool) -> Result<(), String> {
        if let Some(parent) = self.audit_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("No se pudo crear {}: {e}", parent.display()))?;
        }
        let entry = AuditEntry {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            app: request.app.clone(),
            intent: request.intent.clone(),
            permission: request.permission.clone(),
            purpose: request.purpose.clone(),
            granted,
        };
        let raw = serde_json::to_string(&entry)
            .map_err(|e| format!("No se pudo serializar auditoría: {e}"))?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.audit_path)
            .map_err(|e| format!("No se pudo abrir {}: {e}", self.audit_path.display()))?;
        writeln!(file, "{raw}").map_err(|e| format!("No se pudo escribir auditoría: {e}"))
    }
}

fn default_permissions_path() -> PathBuf {
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            Path::new(&std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".config")
        });
    config_home.join("nodalix/privacy/permissions.json")
}

fn default_audit_path() -> PathBuf {
    let data_home = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            Path::new(&std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                .join(".local/share")
        });
    data_home.join("nodalix/privacy/audit.log")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_are_denied_until_granted() {
        let base = std::env::temp_dir().join(format!("nodalix-perms-{}", std::process::id()));
        let broker = PermissionBroker::new(base.join("permissions.json"), base.join("audit.log"));
        let req = PermissionRequest {
            app: "nodalix-assistant".to_string(),
            intent: "contacts.findPerson".to_string(),
            permission: "contacts.read".to_string(),
            purpose: "buscar una persona".to_string(),
        };
        assert!(!broker.request(&req).granted);
        broker.grant("nodalix-assistant", "contacts.read").unwrap();
        assert!(broker.request(&req).granted);
        let _ = std::fs::remove_dir_all(base);
    }
}
