use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type NodeId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shape {
    /// Plain text sitting on the branch, no frame.
    None,
    RoundedBox,
    Cloud,
    Bubble,
    /// Hand drawn marker frame, used for the central idea.
    Marker,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchStyle {
    /// Thick to thin tapered organic stroke.
    Organic,
    Dotted,
    Straight,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub text: String,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub side: Side,
    pub shape: Shape,
    pub branch_style: BranchStyle,
    /// Hex colour such as "#ff6b6b". None means inherit from parent.
    pub color: Option<String>,
    /// Manual drag offset applied on top of the automatic layout.
    pub dx: f32,
    pub dy: f32,
    pub collapsed: bool,
    pub bold: bool,
    pub italic: bool,
    pub font_size: f32,
    /// Font name from the catalogue in `fonts`. Older files lack it.
    #[serde(default = "default_font")]
    pub font: String,
}

fn default_font() -> String {
    crate::fonts::DEFAULT_FONT.to_string()
}

impl Node {
    fn new(id: NodeId, text: &str, parent: Option<NodeId>, side: Side) -> Self {
        Self {
            id,
            text: text.to_string(),
            parent,
            children: vec![],
            side,
            shape: Shape::None,
            branch_style: BranchStyle::Organic,
            color: None,
            dx: 0.0,
            dy: 0.0,
            collapsed: false,
            bold: false,
            italic: false,
            font_size: 20.0,
            font: default_font(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MindMap {
    pub nodes: BTreeMap<NodeId, Node>,
    pub root: NodeId,
    pub next_id: NodeId,
    pub dark_mode: bool,
    pub auto_layout: bool,
}

pub const PALETTE: [&str; 8] = [
    "#ff6b6b", "#ffa500", "#f5c400", "#7bd44a", "#19c3c3", "#2f9bff", "#9b6bff", "#ee6fd0",
];

impl MindMap {
    pub fn new(title: &str) -> Self {
        let mut root = Node::new(1, title, None, Side::Right);
        root.shape = Shape::Marker;
        root.font_size = 30.0;
        root.color = Some("#00b4f0".into());
        let mut nodes = BTreeMap::new();
        nodes.insert(1, root);
        Self {
            nodes,
            root: 1,
            next_id: 2,
            dark_mode: false,
            auto_layout: true,
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn depth(&self, mut id: NodeId) -> usize {
        let mut d = 0;
        while let Some(p) = self.nodes.get(&id).and_then(|n| n.parent) {
            d += 1;
            id = p;
        }
        d
    }

    /// Effective colour: own colour, else nearest ancestor's.
    pub fn color_of(&self, mut id: NodeId) -> String {
        loop {
            let n = &self.nodes[&id];
            if let Some(c) = &n.color {
                return c.clone();
            }
            match n.parent {
                Some(p) => id = p,
                None => return "#2f9bff".into(),
            }
        }
    }

    pub fn add_child(&mut self, parent: NodeId, text: &str) -> Option<NodeId> {
        let p = self.nodes.get(&parent)?;
        let is_root = p.parent.is_none();
        let side = if is_root {
            // Balance the two sides of the central idea.
            let right = p
                .children
                .iter()
                .filter(|c| self.nodes[c].side == Side::Right)
                .count();
            let left = p.children.len() - right;
            if right <= left {
                Side::Right
            } else {
                Side::Left
            }
        } else {
            p.side
        };
        let id = self.next_id;
        self.next_id += 1;
        let mut n = Node::new(id, text, Some(parent), side);
        // New branches inherit their parent's typeface.
        n.font = self.nodes[&parent].font.clone();
        if is_root {
            // Each main branch gets its own colour from the palette.
            n.color = Some(PALETTE[(p.children.len()) % PALETTE.len()].to_string());
        }
        self.nodes.insert(id, n);
        self.nodes.get_mut(&parent)?.children.push(id);
        Some(id)
    }

    pub fn add_sibling(&mut self, id: NodeId, text: &str) -> Option<NodeId> {
        let parent = self.nodes.get(&id)?.parent?;
        let new = self.add_child(parent, text)?;
        // Insert right after `id`, and keep on the same side.
        let side = self.nodes[&id].side;
        let font = self.nodes[&id].font.clone();
        let node = self.nodes.get_mut(&new)?;
        node.side = side;
        node.font = font;
        let kids = &mut self.nodes.get_mut(&parent)?.children;
        kids.pop();
        let pos = kids.iter().position(|c| *c == id)? + 1;
        kids.insert(pos, new);
        Some(new)
    }

    pub fn set_text(&mut self, id: NodeId, text: &str) -> bool {
        self.nodes
            .get_mut(&id)
            .map(|n| n.text = text.to_string())
            .is_some()
    }

    /// Deletes a node and its whole subtree. The root cannot be deleted.
    pub fn delete(&mut self, id: NodeId) -> bool {
        if id == self.root || !self.nodes.contains_key(&id) {
            return false;
        }
        if let Some(p) = self.nodes[&id].parent {
            self.nodes
                .get_mut(&p)
                .unwrap()
                .children
                .retain(|c| *c != id);
        }
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            if let Some(node) = self.nodes.remove(&n) {
                stack.extend(node.children);
            }
        }
        true
    }

    pub fn is_descendant(&self, id: NodeId, ancestor: NodeId) -> bool {
        let mut cur = Some(id);
        while let Some(c) = cur {
            if c == ancestor {
                return true;
            }
            cur = self.nodes.get(&c).and_then(|n| n.parent);
        }
        false
    }

    /// Moves `id` under `new_parent`. Refuses cycles and moving the root.
    pub fn reparent(&mut self, id: NodeId, new_parent: NodeId) -> bool {
        if id == self.root
            || !self.nodes.contains_key(&new_parent)
            || self.is_descendant(new_parent, id)
        {
            return false;
        }
        let old = self.nodes[&id].parent.unwrap();
        self.nodes
            .get_mut(&old)
            .unwrap()
            .children
            .retain(|c| *c != id);
        self.nodes.get_mut(&new_parent).unwrap().children.push(id);
        let new_side = if new_parent == self.root {
            let right = self.nodes[&new_parent]
                .children
                .iter()
                .filter(|c| **c != id && self.nodes[c].side == Side::Right)
                .count();
            let left = self.nodes[&new_parent].children.len() - 1 - right;
            if right <= left {
                Side::Right
            } else {
                Side::Left
            }
        } else {
            self.nodes[&new_parent].side
        };
        self.set_side_recursive(id, new_side);
        let n = self.nodes.get_mut(&id).unwrap();
        n.parent = Some(new_parent);
        n.dx = 0.0;
        n.dy = 0.0;
        // The moved branch adopts the colour of its new home. Under a main
        // branch it simply inherits; directly under the central idea it
        // becomes a main branch with the next free palette colour.
        self.clear_colors(id);
        if new_parent == self.root {
            let slot = self.nodes[&new_parent].children.len() - 1;
            self.nodes.get_mut(&id).unwrap().color =
                Some(PALETTE[slot % PALETTE.len()].to_string());
        }
        true
    }

    /// Removes explicit colours from a node and its whole subtree so they
    /// inherit from the ancestor they now hang under.
    fn clear_colors(&mut self, id: NodeId) {
        let kids = {
            let n = self.nodes.get_mut(&id).unwrap();
            n.color = None;
            n.children.clone()
        };
        for k in kids {
            self.clear_colors(k);
        }
    }

    fn set_side_recursive(&mut self, id: NodeId, side: Side) {
        let kids = {
            let n = self.nodes.get_mut(&id).unwrap();
            n.side = side;
            n.children.clone()
        };
        for k in kids {
            self.set_side_recursive(k, side);
        }
    }

    /// Plain text outline, depth first, used by export and tests.
    pub fn visit_depth_first(&self, mut f: impl FnMut(&Node, usize)) {
        fn go(m: &MindMap, id: NodeId, d: usize, f: &mut dyn FnMut(&Node, usize)) {
            let n = &m.nodes[&id];
            f(n, d);
            for c in &n.children {
                go(m, *c, d + 1, f);
            }
        }
        go(self, self.root, 0, &mut f);
    }
}
