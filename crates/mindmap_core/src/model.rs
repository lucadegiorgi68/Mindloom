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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkStyle {
    Solid,
    Dashed,
}

/// Which ends of a link carry an arrowhead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkArrow {
    None,
    End,
    Start,
    Both,
}

/// A relation between two nodes anywhere in the map, drawn as a curve that
/// is separate from the tree of branches.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Link {
    pub id: u64,
    pub from: NodeId,
    pub to: NodeId,
    pub style: LinkStyle,
    pub arrow: LinkArrow,
    /// Hex colour. None uses the neutral link grey.
    pub color: Option<String>,
    /// How far the curve bows away from a straight line (fraction of length).
    pub bend: f32,
}

pub const LINK_GREY: &str = "#8a93a3";

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
    /// Relations between nodes. Older files lack this.
    #[serde(default)]
    pub links: Vec<Link>,
    /// Further central ideas, each the root of its own tree. `root` stays the
    /// first one (it names the map and cannot be deleted). Older files lack this.
    #[serde(default)]
    pub extra_roots: Vec<NodeId>,
}

pub const PALETTE: [&str; 8] = [
    "#ff6b6b", "#ffa500", "#f5c400", "#7bd44a", "#19c3c3", "#2f9bff", "#9b6bff", "#ee6fd0",
];

/// Frame colours handed out to second, third... central ideas, so each one
/// reads as its own centre next to the first (which is "#00b4f0").
pub const CENTRAL_COLORS: [&str; 5] = ["#8e5cf7", "#1fa971", "#ff7a1a", "#e5389b", "#4a6cf7"];

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
            links: vec![],
            extra_roots: vec![],
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Every central idea, the first one leading.
    pub fn roots(&self) -> Vec<NodeId> {
        std::iter::once(self.root)
            .chain(self.extra_roots.iter().copied())
            .collect()
    }

    /// True for any central idea, the first or a later one.
    pub fn is_central(&self, id: NodeId) -> bool {
        self.nodes.get(&id).is_some_and(|n| n.parent.is_none())
    }

    /// Adds another central idea: a parentless node that starts its own tree,
    /// framed like the first one but in its own colour.
    pub fn add_central_idea(&mut self, text: &str) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        let mut n = Node::new(id, text, None, Side::Right);
        n.shape = Shape::Marker;
        n.font_size = 30.0;
        n.color = Some(CENTRAL_COLORS[self.extra_roots.len() % CENTRAL_COLORS.len()].to_string());
        self.nodes.insert(id, n);
        self.extra_roots.push(id);
        id
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

    /// Deletes a node and its whole subtree. The first central idea cannot be
    /// deleted; any later one can, together with its tree.
    pub fn delete(&mut self, id: NodeId) -> bool {
        if id == self.root || !self.nodes.contains_key(&id) {
            return false;
        }
        match self.nodes[&id].parent {
            Some(p) => self
                .nodes
                .get_mut(&p)
                .unwrap()
                .children
                .retain(|c| *c != id),
            None => self.extra_roots.retain(|r| *r != id),
        }
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            if let Some(node) = self.nodes.remove(&n) {
                stack.extend(node.children);
            }
        }
        // A link cannot outlive either of its ends.
        let nodes = &self.nodes;
        self.links
            .retain(|l| nodes.contains_key(&l.from) && nodes.contains_key(&l.to));
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

    /// Moves `id` under `new_parent`. Refuses cycles and moving any central idea.
    pub fn reparent(&mut self, id: NodeId, new_parent: NodeId) -> bool {
        if !self.nodes.get(&id).is_some_and(|n| n.parent.is_some())
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
        let new_side = if self.is_central(new_parent) {
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
        if self.is_central(new_parent) {
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

    /// Adds a link from `from` to `to`. Refuses self links, unknown nodes and
    /// a second link in the same direction between the same pair.
    pub fn add_link(&mut self, from: NodeId, to: NodeId) -> Option<u64> {
        if from == to
            || !self.nodes.contains_key(&from)
            || !self.nodes.contains_key(&to)
            || self.links.iter().any(|l| l.from == from && l.to == to)
        {
            return None;
        }
        let id = self.links.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        self.links.push(Link {
            id,
            from,
            to,
            style: LinkStyle::Solid,
            arrow: LinkArrow::End,
            color: None,
            bend: 0.22,
        });
        Some(id)
    }

    /// Bows the link by `bend` (fraction of its length), clamped to a sane range.
    pub fn set_link_bend(&mut self, id: u64, bend: f32) -> bool {
        match self.links.iter_mut().find(|l| l.id == id) {
            Some(l) if bend.is_finite() => {
                l.bend = bend.clamp(-1.5, 1.5);
                true
            }
            _ => false,
        }
    }

    /// Moves the ends of a link. Refuses a self link, unknown nodes and a
    /// link that would duplicate another one in the same direction.
    pub fn reconnect_link(&mut self, id: u64, from: NodeId, to: NodeId) -> bool {
        if from == to
            || !self.nodes.contains_key(&from)
            || !self.nodes.contains_key(&to)
            || self
                .links
                .iter()
                .any(|l| l.id != id && l.from == from && l.to == to)
        {
            return false;
        }
        match self.links.iter_mut().find(|l| l.id == id) {
            Some(l) => {
                l.from = from;
                l.to = to;
                true
            }
            None => false,
        }
    }

    pub fn delete_link(&mut self, id: u64) -> bool {
        let before = self.links.len();
        self.links.retain(|l| l.id != id);
        self.links.len() != before
    }

    /// Plain text outline, depth first, used by export and tests. Each central
    /// idea starts a new tree at depth 0.
    pub fn visit_depth_first(&self, mut f: impl FnMut(&Node, usize)) {
        fn go(m: &MindMap, id: NodeId, d: usize, f: &mut dyn FnMut(&Node, usize)) {
            let n = &m.nodes[&id];
            f(n, d);
            for c in &n.children {
                go(m, *c, d + 1, f);
            }
        }
        for r in self.roots() {
            go(self, r, 0, &mut f);
        }
    }
}
