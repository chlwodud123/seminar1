use crate::canvas::{Layer, SelectionMask};

const MAX_HISTORY_ENTRIES: usize = 40;

#[derive(Clone)]
pub struct DocumentSnapshot {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<Layer>,
    pub active_layer: usize,
    pub selection: Option<SelectionMask>,
}

#[derive(Clone)]
pub struct HistoryItem {
    pub label: String,
    pub snapshot: DocumentSnapshot,
}

#[derive(Clone)]
pub struct HistoryStack {
    pub items: Vec<HistoryItem>,
    pub current: usize,
}

impl HistoryStack {
    pub fn new(initial_label: impl Into<String>, snapshot: DocumentSnapshot) -> Self {
        Self {
            items: vec![HistoryItem {
                label: initial_label.into(),
                snapshot,
            }],
            current: 0,
        }
    }

    pub fn push(&mut self, label: impl Into<String>, snapshot: DocumentSnapshot) {
        if self.current + 1 < self.items.len() {
            self.items.truncate(self.current + 1);
        }
        self.items.push(HistoryItem {
            label: label.into(),
            snapshot,
        });
        if self.items.len() > MAX_HISTORY_ENTRIES {
            let excess = self.items.len() - MAX_HISTORY_ENTRIES;
            self.items.drain(0..excess);
        }
        self.current = self.items.len() - 1;
    }

    pub fn can_undo(&self) -> bool {
        self.current > 0
    }

    pub fn can_redo(&self) -> bool {
        self.current + 1 < self.items.len()
    }

    pub fn undo(&mut self) -> Option<&DocumentSnapshot> {
        if self.can_undo() {
            self.current -= 1;
            Some(&self.items[self.current].snapshot)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<&DocumentSnapshot> {
        if self.can_redo() {
            self.current += 1;
            Some(&self.items[self.current].snapshot)
        } else {
            None
        }
    }

    pub fn jump_to(&mut self, idx: usize) -> Option<&DocumentSnapshot> {
        if idx < self.items.len() && idx != self.current {
            self.current = idx;
            Some(&self.items[self.current].snapshot)
        } else {
            None
        }
    }
}
