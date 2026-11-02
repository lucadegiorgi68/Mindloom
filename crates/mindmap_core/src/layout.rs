use crate::geom::Rect;
use crate::model::{MindMap, NodeId, Side, IMAGE_PAD_FRAMED, IMAGE_PAD_PLAIN};
use std::collections::BTreeMap;

const H_GAP: f32 = 70.0;
const V_GAP: f32 = 14.0;
/// Empty space between the trees of two central ideas.
const TREE_GAP: f32 = 140.0;

/// Room a caption needs under a picture: its lines plus a small gap.
pub fn caption_height(text: &str, font_size: f32) -> f32 {
    if text.is_empty() {
        0.0
    } else {
        text.lines().count().max(1) as f32 * font_size * 1.3 + 6.0
    }
}

/// Padding around the picture of an image node.
pub fn image_pad(framed: bool) -> f32 {
    if framed {
        IMAGE_PAD_FRAMED
    } else {
        IMAGE_PAD_PLAIN
    }
}

/// Rough text measurement. The frontend can replace this later with real
/// font metrics through `layout_with`.
pub fn estimate_size(text: &str, font: &str, font_size: f32, framed: bool) -> (f32, f32) {
    let lines: Vec<&str> = if text.is_empty() {
        vec![""]
    } else {
        text.lines().collect()
    };
    let longest = lines
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0)
        .max(2);
    let pad = if framed { 36.0 } else { 14.0 };
    let w = longest as f32 * font_size * crate::fonts::resolve(font).width + pad;
    let h = lines.len() as f32 * font_size * 1.3 + pad * 0.8;
    (w, h)
}

/// Measures a label: (text, font name, font size, framed) to (width, height).
pub type Measure<'a> = &'a dyn Fn(&str, &str, f32, bool) -> (f32, f32);

pub fn layout(map: &MindMap) -> BTreeMap<NodeId, Rect> {
    layout_with(map, &|text, font, size, framed| {
        estimate_size(text, font, size, framed)
    })
}

pub fn layout_with(map: &MindMap, measure: Measure) -> BTreeMap<NodeId, Rect> {
    let mut sizes = BTreeMap::new();
    for (id, n) in &map.nodes {
        let framed = n.shape != crate::model::Shape::None;
        let size = match map.node_image_size(*id) {
            // A picture node is the picture with its text, if any, as a caption.
            Some((iw, ih)) => {
                let pad = image_pad(framed);
                let tw = if n.text.is_empty() {
                    0.0
                } else {
                    measure(&n.text, &n.font, n.font_size, false).0
                };
                (
                    iw.max(tw) + pad * 2.0,
                    ih + caption_height(&n.text, n.font_size) + pad * 2.0,
                )
            }
            None => measure(&n.text, &n.font, n.font_size, framed),
        };
        sizes.insert(*id, size);
    }
    let mut out = BTreeMap::new();
    // Where the next central idea's tree may start: the bottom of the trees
    // above it, measured without their own manual offset, so dragging one
    // central idea never shoves the others.
    let mut cursor = 0.0_f32;
    for (i, root) in map.roots().into_iter().enumerate() {
        let mut tree = BTreeMap::new();
        let (rw, rh) = sizes[&root];
        tree.insert(
            root,
            Rect {
                cx: 0.0,
                cy: 0.0,
                w: rw,
                h: rh,
            },
        );
        for side in [Side::Right, Side::Left] {
            place_children(map, root, side, &sizes, &mut tree);
        }
        // Manual offsets are applied top down so they carry the whole subtree.
        apply_offsets(map, root, 0.0, 0.0, &mut tree);
        let rdy = map.nodes[&root].dy;
        let top = tree.values().map(|r| r.top()).fold(f32::MAX, f32::min) - rdy;
        let bottom = tree.values().map(|r| r.bottom()).fold(f32::MIN, f32::max) - rdy;
        // The first tree stays centred on the origin; later ones stack below.
        let ty = if i == 0 { 0.0 } else { cursor + TREE_GAP - top };
        for r in tree.values_mut() {
            r.cy += ty;
        }
        cursor = bottom + ty;
        out.extend(tree);
    }
    out
}

fn visible_children(map: &MindMap, id: NodeId, side: Option<Side>) -> Vec<NodeId> {
    let n = &map.nodes[&id];
    if n.collapsed {
        return vec![];
    }
    n.children
        .iter()
        .copied()
        .filter(|c| side.is_none_or(|s| map.nodes[c].side == s))
        .collect()
}

/// Vertical space a node and its visible descendants need.
fn subtree_height(map: &MindMap, id: NodeId, sizes: &BTreeMap<NodeId, (f32, f32)>) -> f32 {
    let own = sizes[&id].1;
    let kids = visible_children(map, id, None);
    if kids.is_empty() {
        return own;
    }
    let sum: f32 = kids
        .iter()
        .map(|k| subtree_height(map, *k, sizes))
        .sum::<f32>()
        + V_GAP * (kids.len() as f32 - 1.0);
    sum.max(own)
}

fn place_children(
    map: &MindMap,
    parent: NodeId,
    side: Side,
    sizes: &BTreeMap<NodeId, (f32, f32)>,
    out: &mut BTreeMap<NodeId, Rect>,
) {
    let is_root = map.nodes[&parent].parent.is_none();
    let kids = visible_children(map, parent, if is_root { Some(side) } else { None });
    if kids.is_empty() {
        return;
    }
    let hs: Vec<f32> = kids
        .iter()
        .map(|k| subtree_height(map, *k, sizes))
        .collect();
    let total: f32 = hs.iter().sum::<f32>() + V_GAP * (kids.len() as f32 - 1.0);
    let p = out[&parent];
    let dir = if side == Side::Right { 1.0 } else { -1.0 };
    let mut y = p.cy - total / 2.0;
    for (k, h) in kids.iter().zip(hs) {
        let (w, nh) = sizes[k];
        let edge = if dir > 0.0 { p.right() } else { p.left() };
        let cx = edge + dir * (H_GAP + w / 2.0);
        out.insert(
            *k,
            Rect {
                cx,
                cy: y + h / 2.0,
                w,
                h: nh,
            },
        );
        place_children(map, *k, side, sizes, out);
        y += h + V_GAP;
    }
}

fn apply_offsets(map: &MindMap, id: NodeId, ax: f32, ay: f32, out: &mut BTreeMap<NodeId, Rect>) {
    let n = &map.nodes[&id];
    let (tx, ty) = (ax + n.dx, ay + n.dy);
    if let Some(r) = out.get_mut(&id) {
        r.cx += tx;
        r.cy += ty;
    }
    for c in visible_children(map, id, None) {
        apply_offsets(map, c, tx, ty, out);
    }
}
