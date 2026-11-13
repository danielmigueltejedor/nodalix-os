//! Where the user has been: back, forward and up.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct History {
    back: Vec<PathBuf>,
    forward: Vec<PathBuf>,
    current: PathBuf,
}

impl History {
    pub fn new(start: PathBuf) -> Self {
        Self {
            back: Vec::new(),
            forward: Vec::new(),
            current: start,
        }
    }

    pub fn current(&self) -> &Path {
        &self.current
    }

    /// Goes somewhere new: what was ahead is forgotten. Going to the folder already open does
    /// nothing, so that reloading never fills the history.
    pub fn visit(&mut self, path: PathBuf) {
        if path == self.current {
            return;
        }
        self.back.push(std::mem::replace(&mut self.current, path));
        self.forward.clear();
    }

    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    pub fn go_back(&mut self) -> Option<&Path> {
        let previous = self.back.pop()?;
        self.forward
            .push(std::mem::replace(&mut self.current, previous));
        Some(&self.current)
    }

    pub fn go_forward(&mut self) -> Option<&Path> {
        let next = self.forward.pop()?;
        self.back.push(std::mem::replace(&mut self.current, next));
        Some(&self.current)
    }

    /// The folder that contains the current one; there is none at the root.
    pub fn parent(&self) -> Option<PathBuf> {
        self.current.parent().map(Path::to_path_buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn back_and_forward_walk_the_visited_folders() {
        let mut h = History::new(p("/home/a"));
        assert!(!h.can_go_back() && !h.can_go_forward());
        h.visit(p("/home/a/docs"));
        h.visit(p("/home/a/docs/work"));
        assert_eq!(h.go_back(), Some(Path::new("/home/a/docs")));
        assert_eq!(h.go_back(), Some(Path::new("/home/a")));
        assert_eq!(h.go_back(), None);
        assert_eq!(h.go_forward(), Some(Path::new("/home/a/docs")));
        assert!(h.can_go_forward());
    }

    #[test]
    fn visiting_somewhere_new_forgets_what_was_ahead() {
        let mut h = History::new(p("/a"));
        h.visit(p("/b"));
        h.go_back();
        h.visit(p("/c"));
        assert!(!h.can_go_forward());
        assert_eq!(h.go_back(), Some(Path::new("/a")));
    }

    #[test]
    fn reloading_the_open_folder_adds_nothing() {
        let mut h = History::new(p("/a"));
        h.visit(p("/a"));
        assert!(!h.can_go_back());
    }

    #[test]
    fn up_has_nowhere_to_go_at_the_root() {
        assert_eq!(History::new(p("/home/a")).parent(), Some(p("/home")));
        assert_eq!(History::new(p("/")).parent(), None);
    }
}
