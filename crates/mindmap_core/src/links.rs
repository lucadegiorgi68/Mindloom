//! Geometry of links: a gently bowed curve between two nodes, trimmed so it
//! meets the nodes cleanly, with optional dart shaped arrowheads.
use crate::geom::{Pt, Rect, Rng};
use crate::model::LinkArrow;

pub struct LinkGeom {
    /// Polyline along the visible part of the curve.
    pub points: Vec<Pt>,
    /// Arrowhead polygon (tip, wing, notch, wing) at the target end.
    pub arrow_end: Vec<Pt>,
    /// Arrowhead polygon at the source end.
    pub arrow_start: Vec<Pt>,
    /// Point of the curve halfway along, where the bend handle sits.
    pub mid: Pt,
}

fn cubic(p0: Pt, p1: Pt, p2: Pt, p3: Pt, t: f32) -> Pt {
    let u = 1.0 - t;
    let (b0, b1, b2, b3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Pt::new(
        b0 * p0.x + b1 * p1.x + b2 * p2.x + b3 * p3.x,
        b0 * p0.y + b1 * p1.y + b2 * p2.y + b3 * p3.y,
    )
}

/// Where a ray from the centre of `r` toward `toward` leaves the rectangle
/// grown by `pad`.
fn exit_point(r: &Rect, toward: Pt, pad: f32) -> Pt {
    let (dx, dy) = (toward.x - r.cx, toward.y - r.cy);
    let (hw, hh) = (r.w / 2.0 + pad, r.h / 2.0 + pad);
    let tx = if dx.abs() < 1e-4 {
        f32::INFINITY
    } else {
        hw / dx.abs()
    };
    let ty = if dy.abs() < 1e-4 {
        f32::INFINITY
    } else {
        hh / dy.abs()
    };
    let t = tx.min(ty);
    if !t.is_finite() {
        return Pt::new(r.cx, r.cy);
    }
    Pt::new(r.cx + dx * t, r.cy + dy * t)
}

fn arrow_polygon(tip: Pt, dir: Pt, size: f32) -> Vec<Pt> {
    let (nx, ny) = (-dir.y, dir.x);
    let base = Pt::new(tip.x - dir.x * size, tip.y - dir.y * size);
    let w = size * 0.5;
    vec![
        tip,
        Pt::new(base.x + nx * w, base.y + ny * w),
        // A shallow notch gives the head a hand drawn dart shape.
        Pt::new(base.x + dir.x * size * 0.28, base.y + dir.y * size * 0.28),
        Pt::new(base.x - nx * w, base.y - ny * w),
    ]
}

/// Cuts `pts` so `cut_start` and `cut_end` of arc length are removed.
fn trim(pts: &[Pt], cut_start: f32, cut_end: f32) -> Vec<Pt> {
    let mut acc = vec![0.0f32];
    for i in 1..pts.len() {
        let (p, q) = (pts[i - 1], pts[i]);
        acc.push(acc[i - 1] + ((q.x - p.x).powi(2) + (q.y - p.y).powi(2)).sqrt());
    }
    let total = *acc.last().unwrap_or(&0.0);
    if total <= cut_start + cut_end + 2.0 {
        return pts.to_vec();
    }
    let (lo, hi) = (cut_start, total - cut_end);
    let at = |d: f32| -> Pt {
        let i = acc.partition_point(|a| *a < d).clamp(1, pts.len() - 1);
        let span = (acc[i] - acc[i - 1]).max(1e-4);
        pts[i - 1].lerp(pts[i], (d - acc[i - 1]) / span)
    };
    let mut out = vec![at(lo)];
    out.extend(
        pts.iter()
            .zip(&acc)
            .filter(|(_, a)| **a > lo && **a < hi)
            .map(|(p, _)| *p),
    );
    out.push(at(hi));
    out
}

/// Returns None when the two nodes sit on top of each other.
pub fn link_geometry(
    a: &Rect,
    b: &Rect,
    bend: f32,
    seed: u64,
    width: f32,
    arrow: LinkArrow,
) -> Option<LinkGeom> {
    let (ca, cb) = (Pt::new(a.cx, a.cy), Pt::new(b.cx, b.cy));
    let (dx, dy) = (cb.x - ca.x, cb.y - ca.y);
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 12.0 {
        return None;
    }
    let (ux, uy) = (dx / dist, dy / dist);
    let (nx, ny) = (-uy, ux);
    let mut rng = Rng(seed);
    let wob = dist * 0.02;
    let off = bend * dist;
    let c1 = Pt::new(
        ca.x + dx * 0.25 + nx * off + rng.signed() * wob,
        ca.y + dy * 0.25 + ny * off + rng.signed() * wob,
    );
    let c2 = Pt::new(
        ca.x + dx * 0.75 + nx * off + rng.signed() * wob,
        ca.y + dy * 0.75 + ny * off + rng.signed() * wob,
    );
    let p0 = exit_point(a, c1, 5.0);
    let p3 = exit_point(b, c2, 5.0);
    let steps = 48;
    let pts: Vec<Pt> = (0..=steps)
        .map(|i| cubic(p0, c1, c2, p3, i as f32 / steps as f32))
        .collect();

    let size = 10.0 + width * 2.4;
    let dir_at = |from: Pt, to: Pt| {
        let (x, y) = (to.x - from.x, to.y - from.y);
        let l = (x * x + y * y).sqrt().max(1e-4);
        Pt::new(x / l, y / l)
    };
    let near = |i: usize| pts[i];
    // Tangent estimated over the arrow length, steadier than the last segment.
    let back = (steps / 8).max(2);
    let (mut arrow_start, mut arrow_end) = (vec![], vec![]);
    let (mut cut_s, mut cut_e) = (0.0, 0.0);
    if matches!(arrow, LinkArrow::End | LinkArrow::Both) {
        arrow_end = arrow_polygon(p3, dir_at(near(steps - back), p3), size);
        cut_e = size * 0.7;
    }
    if matches!(arrow, LinkArrow::Start | LinkArrow::Both) {
        arrow_start = arrow_polygon(p0, dir_at(near(back), p0), size);
        cut_s = size * 0.7;
    }
    Some(LinkGeom {
        points: trim(&pts, cut_s, cut_e),
        arrow_end,
        arrow_start,
        mid: cubic(p0, c1, c2, p3, 0.5),
    })
}
