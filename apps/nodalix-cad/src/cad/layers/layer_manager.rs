use crate::cad::document::{CADDocument, Layer};

#[derive(Clone, Debug, Default)]
pub struct LayerManager;

impl LayerManager {
    pub fn create_layer(document: &mut CADDocument, name: &str) -> String {
        let id = format!("layer-{}", slugify(name));
        if document.layer_by_id(&id).is_some() {
            return id;
        }
        document.layers.push(Layer::new(id.clone(), name));
        document.modified = true;
        id
    }

    pub fn active_layer_id(document: &CADDocument) -> &str {
        &document.active_layer_id
    }

    pub fn set_active_layer(document: &mut CADDocument, layer_id: &str) -> bool {
        if document.layer_by_id(layer_id).is_none() {
            return false;
        }
        document.active_layer_id = layer_id.to_string();
        document.modified = true;
        true
    }

    pub fn find_by_id<'a>(document: &'a CADDocument, layer_id: &str) -> Option<&'a Layer> {
        document.layer_by_id(layer_id)
    }

    pub fn find_by_name<'a>(document: &'a CADDocument, name: &str) -> Option<&'a Layer> {
        document.layer_by_name(name)
    }

    pub fn set_visible(document: &mut CADDocument, layer_id: &str, visible: bool) -> bool {
        let Some(layer) = document.layers.iter_mut().find(|l| l.id == layer_id) else {
            return false;
        };
        layer.visible = visible;
        document.modified = true;
        true
    }

    pub fn set_locked(document: &mut CADDocument, layer_id: &str, locked: bool) -> bool {
        let Some(layer) = document.layers.iter_mut().find(|l| l.id == layer_id) else {
            return false;
        };
        layer.locked = locked;
        document.modified = true;
        true
    }

    pub fn is_entity_editable(document: &CADDocument, layer_id: &str) -> bool {
        document
            .layer_by_id(layer_id)
            .map(|layer| layer.visible && !layer.locked)
            .unwrap_or(true)
    }
}

fn slugify(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::document::CADDocument;

    #[test]
    fn create_and_activate_layer() {
        let mut doc = CADDocument::new_empty();
        let id = LayerManager::create_layer(&mut doc, "Walls");
        assert!(LayerManager::set_active_layer(&mut doc, &id));
        assert_eq!(LayerManager::active_layer_id(&doc), id);
    }

    #[test]
    fn locked_layer_not_editable() {
        let mut doc = CADDocument::new_empty();
        let id = doc.active_layer_id.clone();
        LayerManager::set_locked(&mut doc, &id, true);
        assert!(!LayerManager::is_entity_editable(&doc, &id));
    }
}
