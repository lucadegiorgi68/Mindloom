//! A Scene is everything the renderer needs, already laid out and shaped.
//! The frontend (or the SVG exporter) only draws it.
use crate::geom::{Pt, Rect};
use crate::layout::layout;
use crate::links::link_geometry;
use crate::model::{BranchStyle, LinkArrow, LinkStyle, MindMap, NodeId, Shape, Side, LINK_GREY};
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
    /// CSS font-family stack for this node.
    pub font_stack: String,
    pub font: String,
    pub bold: bool,
    pub italic: bool,
    pub collapsed: bool,
    pub has_children: bool,
    /// Smooth SVG path of the marker frame, relative to the node centre.
    pub frame: String,
    /// Baseline of the last text line, absolute y.
    pub text_baseline: f32,
    /// SVG path data relative to the node centre (Cloud and Bubble shapes).
    pub shape_path: Option<String>,
    /// True for every central idea (a node with no parent).
    pub central: bool,
    /// Which side of its central idea the node hangs on.
    pub side: Side,
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
pub struct SceneLink {
    pub id: u64,
    pub from: NodeId,
    pub to: NodeId,
    pub style: LinkStyle,
    pub arrow: LinkArrow,
    pub color: String,
    pub width: f32,
    /// Visible part of the curve as SVG path data (absolute coordinates).
    pub path: String,
    /// Arrowhead polygons, empty when absent.
    pub arrow_end: Vec<Pt>,
    pub arrow_start: Vec<Pt>,
    /// Current bow of the curve (fraction of its length), for the bend handle.
    pub bend: f32,
    /// Midpoint of the curve, where the bend handle sits.
    pub mid: Pt,
    /// Centres of the two visible nodes the link attaches to.
    pub ca: Pt,
    pub cb: Pt,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scene {
    pub nodes: Vec<SceneNode>,
    pub branches: Vec<SceneBranch>,
    pub links: Vec<SceneLink>,
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
            font_stack: crate::fonts::resolve(&n.font).stack.to_string(),
            font: crate::fonts::resolve(&n.font).name.to_string(),
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
            shape_path: match n.shape {
                Shape::Cloud => Some(organic::cloud_path(r.w, r.h, seed)),
                Shape::Bubble => Some(organic::bubble_path(r.w, r.h)),
                _ => None,
            },
            central: n.parent.is_none(),
            side: n.side,
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
    for root in map.roots() {
        walk(map, root, &rects, &mut nodes, &mut branches);
    }

    let links = build_links(map, &rects);

    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    let link_pts = links.iter().flat_map(|l| {
        l.arrow_end
            .iter()
            .chain(l.arrow_start.iter())
            .copied()
            .chain(path_points(&l.path))
    });
    for p in link_pts {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
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
        links,
        bounds,
        dark_mode: map.dark_mode,
    }
}

/// A hidden node (inside a collapsed branch) is represented by its nearest
/// visible ancestor, so links never dangle when a branch is folded.
fn visible_anchor(
    map: &MindMap,
    mut id: NodeId,
    rects: &std::collections::BTreeMap<NodeId, Rect>,
) -> Option<NodeId> {
    loop {
        if rects.contains_key(&id) {
            return Some(id);
        }
        id = map.nodes.get(&id)?.parent?;
    }
}

fn build_links(map: &MindMap, rects: &std::collections::BTreeMap<NodeId, Rect>) -> Vec<SceneLink> {
    let mut out = vec![];
    for l in &map.links {
        let (Some(a), Some(b)) = (
            visible_anchor(map, l.from, rects),
            visible_anchor(map, l.to, rects),
        ) else {
            continue;
        };
        if a == b {
            continue; // both ends sit inside the same folded branch
        }
        let width = 3.0;
        let seed = l.id.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x51;
        let Some(g) = link_geometry(&rects[&a], &rects[&b], l.bend, seed, width, l.arrow) else {
            continue;
        };
        let path = g
            .points
            .iter()
            .enumerate()
            .map(|(i, p)| format!("{}{:.1} {:.1}", if i == 0 { "M" } else { "L" }, p.x, p.y))
            .collect::<Vec<_>>()
            .join(" ");
        out.push(SceneLink {
            id: l.id,
            from: l.from,
            to: l.to,
            style: l.style,
            arrow: l.arrow,
            color: l.color.clone().unwrap_or_else(|| LINK_GREY.to_string()),
            width,
            path,
            arrow_end: g.arrow_end,
            arrow_start: g.arrow_start,
            bend: l.bend,
            mid: g.mid,
            ca: Pt::new(rects[&a].cx, rects[&a].cy),
            cb: Pt::new(rects[&b].cx, rects[&b].cy),
        });
    }
    out
}

/// Reads the points back out of the simple "M x y L x y ..." path we emit.
fn path_points(path: &str) -> Vec<Pt> {
    let nums: Vec<f32> = path
        .split(|c: char| c == 'M' || c == 'L' || c.is_whitespace())
        .filter_map(|t| t.parse().ok())
        .collect();
    nums.chunks_exact(2).map(|c| Pt::new(c[0], c[1])).collect()
}
