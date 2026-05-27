use std::collections::BTreeSet;

#[derive(Clone, Debug, Default)]
pub struct SelectionManager {
    selected: BTreeSet<String>,
}

impl SelectionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.selected.clear();
    }

    pub fn select_one(&mut self, id: impl Into<String>) {
        self.selected.clear();
        self.selected.insert(id.into());
    }

    pub fn select_many(&mut self, ids: impl IntoIterator<Item = String>) {
        self.selected.clear();
        self.selected.extend(ids);
    }

    pub fn add(&mut self, id: impl Into<String>) {
        self.selected.insert(id.into());
    }

    pub fn remove(&mut self, id: &str) {
        self.selected.remove(id);
    }

    pub fn toggle(&mut self, id: impl Into<String>) {
        let id = id.into();
        if !self.selected.remove(&id) {
            self.selected.insert(id);
        }
    }

    pub fn is_selected(&self, id: &str) -> bool {
        self.selected.contains(id)
    }

    pub fn selected_ids(&self) -> Vec<String> {
        self.selected.iter().cloned().collect()
    }

    pub fn count(&self) -> usize {
        self.selected.len()
    }

    pub fn is_empty(&self) -> bool {
        self.selected.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_toggle_and_clear() {
        let mut sel = SelectionManager::new();
        sel.select_one("entity-1");
        assert!(sel.is_selected("entity-1"));
        sel.toggle("entity-2");
        assert_eq!(sel.count(), 2);
        sel.clear();
        assert!(sel.is_empty());
    }
}
