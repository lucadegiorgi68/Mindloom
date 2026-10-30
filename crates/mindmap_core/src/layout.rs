use crate::geom::Rect;
use crate::model::{MindMap, NodeId, Side};
use std::collections::BTreeMap;

const H_GAP: f32 = 70.0;
const V_GAP: f32 = 14.0;

/// Rough text measurement. The frontend can replace this later with real
/// font metrics through `layout_with`.
pub fn estimate_size(text: &str, font_size: f32, framed: bool) -> (f32, f32) {
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
    let w = longest as f32 * font_size * 0.56 + pad;
    let h = lines.len() as f32 * font_size * 1.3 + pad * 0.8;
    (w, h)
}

pub fn layout(map: &MindMap) -> BTreeMap<NodeId, Rect> {
    layout_with(map, &|text, size, framed| estimate_size(text, size, framed))
}

pub fn layout_with(
    map: &MindMap,
    measure: &dyn Fn(&str, f32, bool) -> (f32, f32),
) -> BTreeMap<NodeId, Rect> {
    let mut sizes = BTreeMap::new();
    for (id, n) in &map.nodes {
        let framed = n.shape != crate::model::Shape::None;
        sizes.insert(*id, measure(&n.text, n.font_size, framed));
    }
    let mut out = BTreeMap::new();
    let (rw, rh) = sizes[&map.root];
    out.insert(
        map.root,
        Rect {
            cx: 0.0,
            cy: 0.0,
            w: rw,
            h: rh,
        },
    );
    for side in [Side::Right, Side::Left] {
        place_children(map, map.root, side, &sizes, &mut out);
    }
    // Manual offsets are applied top down so they carry the whole subtree.
    apply_offsets(map, map.root, 0.0, 0.0, &mut out);
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
