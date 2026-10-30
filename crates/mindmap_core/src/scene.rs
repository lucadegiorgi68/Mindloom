//! A Scene is everything the renderer needs, already laid out and shaped.
//! The frontend (or the SVG exporter) only draws it.
use crate::geom::{Pt, Rect};
use crate::layout::layout;
use crate::model::{BranchStyle, MindMap, NodeId, Shape, Side};
use crate::organic;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneNode {
    pub id: NodeId,
    pub text: String,
    pub rect: Rect,
    pub shape: Shape,
    pub color: String,
    pub font_size: f32,
    pub bold: bool,
    pub italic: bool,
    pub collapsed: bool,
    pub has_children: bool,
    /// Smooth SVG path of the marker frame, relative to the node centre.
    pub frame: String,
    /// Baseline of the last text line, absolute y.
    pub text_baseline: f32,
    /// SVG path data relative to the node centre (Cloud shape).
    pub cloud: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneBranch {
    pub from: NodeId,
    pub to: NodeId,
    pub color: String,
    pub style: BranchStyle,
    pub outline: Vec<Pt>,
    pub centerline: Vec<Pt>,
    pub width: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scene {
    pub nodes: Vec<SceneNode>,
    pub branches: Vec<SceneBranch>,
    pub bounds: Rect,
    pub dark_mode: bool,
}

fn branch_width(depth: usize) -> f32 {
    match depth {
        0 | 1 => 16.0,
        2 => 9.0,
        3 => 6.0,
        _ => 4.0,
    }
}

pub fn build_scene(map: &MindMap) -> Scene {
    let rects = layout(map);
    let mut nodes = vec![];
    let mut branches = vec![];

    fn walk(
        map: &MindMap,
        id: NodeId,
        rects: &std::collections::BTreeMap<NodeId, Rect>,
        nodes: &mut Vec<SceneNode>,
        branches: &mut Vec<SceneBranch>,
    ) {
        let n = &map.nodes[&id];
        let r = rects[&id];
        let color = map.color_of(id);
        let seed = id.wrapping_mul(0x2545_F491_4F6C_DD1D);
        nodes.push(SceneNode {
            id,
            text: n.text.clone(),
            rect: r,
            shape: n.shape,
            color: color.clone(),
            font_size: n.font_size,
            bold: n.bold,
            italic: n.italic,
            collapsed: n.collapsed,
            has_children: !n.children.is_empty(),
            frame: if n.shape == Shape::Marker {
                organic::smooth_path(&organic::marker_frame(r.w, r.h, seed), 0.0, 0.0, false)
            } else {
                String::new()
            },
            text_baseline: if n.shape == Shape::None {
                r.bottom() - 12.0
            } else {
                let extra = (n.text.lines().count().max(1) as f32 - 1.0) * n.font_size * 1.25 / 2.0;
                r.cy + n.font_size * 0.35 + extra
            },
            cloud: if n.shape == Shape::Cloud {
                Some(organic::cloud_path(r.w, r.h, seed))
            } else {
                None
            },
        });
        if n.collapsed {
            return;
        }
        for c in &n.children {
            let cr = rects[c];
            let cn = &map.nodes[c];
            let dir = if cn.side == Side::Right { 1.0 } else { -1.0 };
            let depth = map.depth(*c);
            // The branch leaves the parent's tip. Unframed labels sit on an
            // underline that continues the branch; framed ones stop at the frame.
            let parent_framed = n.shape != Shape::None;
            let a = if n.parent.is_none() || parent_framed {
                Pt::new(
                    if dir > 0.0 { r.right() } else { r.left() } - dir * 4.0,
                    r.cy,
                )
            } else {
                Pt::new(
                    if dir > 0.0 { r.right() } else { r.left() },
                    r.bottom() - 2.0,
                )
            };
            let framed = cn.shape != Shape::None;
            let near_x = if dir > 0.0 { cr.left() } else { cr.right() };
            let (b, tail) = if framed {
                (Pt::new(near_x + dir * 2.0, cr.cy), 0.0)
            } else {
                (Pt::new(near_x, cr.bottom() - 2.0), cr.w)
            };
            let w0 = branch_width(depth);
            let w1 = if cn.children.is_empty() {
                2.5
            } else {
                branch_width(depth + 1).min(w0)
            };
            let cseed = c.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xA5A5;
            let wob = if cn.branch_style == BranchStyle::Straight {
                0.0
            } else {
                0.7
            };
            let geom = organic::branch(a, b, w0, w1.min(w0), cseed, wob, tail, dir);
            let outline = geom.outline;
            branches.push(SceneBranch {
                from: id,
                to: *c,
                color: map.color_of(*c),
                style: cn.branch_style,
                outline,
                centerline: geom.centerline,
                width: w0,
            });
            walk(map, *c, rects, nodes, branches);
        }
    }
    walk(map, map.root, &rects, &mut nodes, &mut branches);

    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for n in &nodes {
        x0 = x0.min(n.rect.left());
        y0 = y0.min(n.rect.top());
        x1 = x1.max(n.rect.right());
        y1 = y1.max(n.rect.bottom());
    }
    let bounds = Rect {
        cx: (x0 + x1) / 2.0,
        cy: (y0 + y1) / 2.0,
        w: x1 - x0,
        h: y1 - y0,
    };
    Scene {
        nodes,
        branches,
        bounds,
        dark_mode: map.dark_mode,
    }
}
