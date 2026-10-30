use crate::model::MindMap;

/// A map plus snapshot based undo and redo. Maps are small, so cloning
/// per edit is simple and robust.
pub struct Document {
    pub map: MindMap,
    undo: Vec<MindMap>,
    redo: Vec<MindMap>,
}

impl Document {
    pub fn new(map: MindMap) -> Self {
        Self {
            map,
            undo: vec![],
            redo: vec![],
        }
    }

    /// Runs an edit. The snapshot is kept only if the edit reports a change.
    pub fn edit<R>(&mut self, f: impl FnOnce(&mut MindMap) -> Option<R>) -> Option<R> {
        let before = self.map.clone();
        let out = f(&mut self.map);
        if out.is_some() {
            self.undo.push(before);
            self.redo.clear();
            if self.undo.len() > 200 {
                self.undo.remove(0);
            }
        } else {
            self.map = before;
        }
        out
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some(prev) => {
                self.redo.push(std::mem::replace(&mut self.map, prev));
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push(std::mem::replace(&mut self.map, next));
                true
            }
            None => false,
        }
    }
}
