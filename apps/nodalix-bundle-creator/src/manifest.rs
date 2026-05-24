#[derive(Debug, Clone)]
pub struct BundleManifest {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub bundle_type: String,
    pub target: String,
    pub author: String,
    pub description: String,
}

impl BundleManifest {
    pub fn from_toml_like(input: &str) -> Result<Self, String> {
        let get = |key: &str| -> Result<String, String> {
            input
                .lines()
                .find_map(|line| {
                    let line = line.trim();
                    let (left, right) = line.split_once('=')?;
                    if left.trim() == key {
                        Some(right.trim().trim_matches('"').to_string())
                    } else {
                        None
                    }
                })
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("Missing required field: {key}"))
        };

        Ok(Self {
            name: get("name")?,
            display_name: get("display_name")?,
            version: get("version")?,
            bundle_type: get("bundle_type")?,
            target: get("target")?,
            author: get("author")?,
            description: get("description")?,
        })
    }
}

pub fn example_manifest() -> Result<BundleManifest, String> {
    BundleManifest::from_toml_like(include_str!("../examples/example.nodalix-bundle.toml"))
}
