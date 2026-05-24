use crate::manifest::BundleManifest;

pub fn summary(manifest: &BundleManifest) -> Vec<(&'static str, String)> {
    vec![
        ("Bundle name", manifest.display_name.clone()),
        ("Internal name", manifest.name.clone()),
        ("Type", manifest.bundle_type.clone()),
        ("Version", manifest.version.clone()),
        ("Target", manifest.target.clone()),
        ("Author", manifest.author.clone()),
        ("Description", manifest.description.clone()),
    ]
}
