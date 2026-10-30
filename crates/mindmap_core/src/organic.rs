//! The organic look: tapered, gently wobbling branches and hand drawn frames.
//! Everything is deterministic from a seed so maps look the same every time.
use crate::geom::{Pt, Rng};

pub struct BranchGeom {
    /// Closed polygon outline of the tapered stroke.
    pub outline: Vec<Pt>,
    pub centerline: Vec<Pt>,
}

fn cubic(p0: Pt, p1: Pt, p2: Pt, p3: Pt, t: f32) -> Pt {
    let u = 1.0 - t;
    let (b0, b1, b2, b3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Pt::new(
        b0 * p0.x + b1 * p1.x + b2 * p2.x + b3 * p3.x,
        b0 * p0.y + b1 * p1.y + b2 * p2.y + b3 * p3.y,
    )
}

/// Tapered organic branch from `a` to `b`, then a straight underline of
/// length `tail` continuing in `dir` (+1 right, -1 left) so a label can sit
/// on it. `w0` is the width at the parent end, `w1` at the tip. `wobble`
/// (0..1) adds hand drawn irregularity.
#[allow(clippy::too_many_arguments)]
pub fn branch(
    a: Pt,
    b: Pt,
    w0: f32,
    w1: f32,
    seed: u64,
    wobble: f32,
    tail: f32,
    dir: f32,
) -> BranchGeom {
    let mut rng = Rng(seed);
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    // Leave and arrive horizontally, like a growing branch, with a seeded
    // sideways drift so no two branches are identical.
    let drift = wobble * len * 0.10;
    let p1 = Pt::new(a.x + dx * 0.55, a.y + rng.signed() * drift);
    let p2 = Pt::new(b.x - dx * 0.40, b.y + rng.signed() * drift * 0.4);
    let steps = ((len / 8.0) as usize).clamp(14, 56);
    let phase = rng.signed() * std::f32::consts::PI;

    // Centreline: bezier part followed by the straight tail.
    let mut center: Vec<Pt> = (0..=steps)
        .map(|i| cubic(a, p1, p2, b, i as f32 / steps as f32))
        .collect();
    let tail_steps = (tail / 10.0).ceil() as usize;
    for i in 1..=tail_steps {
        let t = i as f32 / tail_steps as f32;
        // Tiny hand wobble in the underline so it is not ruler straight.
        let y = b.y + wobble * 1.2 * (t * 5.0 + phase).sin();
        center.push(Pt::new(b.x + dir * tail * t, y));
    }
    // Arc length parametrisation for a smooth taper over the whole stroke.
    let mut acc = vec![0.0f32; center.len()];
    for i in 1..center.len() {
        let (p, q) = (center[i - 1], center[i]);
        acc[i] = acc[i - 1] + ((q.x - p.x).powi(2) + (q.y - p.y).powi(2)).sqrt();
    }
    let total = acc.last().copied().unwrap_or(1.0).max(1.0);

    let mut left = Vec::with_capacity(center.len());
    let mut right = Vec::with_capacity(center.len());
    let mut last_w = w1;
    for i in 0..center.len() {
        let t = acc[i] / total;
        let prev = center[i.saturating_sub(1)];
        let next = center[(i + 1).min(center.len() - 1)];
        let (tx, ty) = (next.x - prev.x, next.y - prev.y);
        let tl = (tx * tx + ty * ty).sqrt().max(1e-3);
        let (nx, ny) = (-ty / tl, tx / tl);
        let ease = t * t * (3.0 - 2.0 * t);
        let ripple = 1.0 + wobble * 0.10 * (t * 6.0 + phase).sin();
        let w = ((w0 + (w1 - w0) * ease) * ripple).max(0.8);
        last_w = w;
        let c = center[i];
        left.push(Pt::new(c.x + nx * w / 2.0, c.y + ny * w / 2.0));
        right.push(Pt::new(c.x - nx * w / 2.0, c.y - ny * w / 2.0));
    }
    // Round tip cap.
    let tip = *center.last().unwrap();
    let prev = center[center.len().saturating_sub(2)];
    let (ux, uy) = (tip.x - prev.x, tip.y - prev.y);
    let ul = (ux * ux + uy * uy).sqrt().max(1e-3);
    let (ux, uy) = (ux / ul, uy / ul);
    let r = last_w / 2.0;
    let mut outline = left;
    for k in 1..=3 {
        let ang = std::f32::consts::FRAC_PI_2 - k as f32 * std::f32::consts::FRAC_PI_2 / 2.0;
        // From the left side (normal) around to the right side.
        let (nx, ny) = (-uy, ux);
        outline.push(Pt::new(
            tip.x + nx * r * ang.sin() + ux * r * ang.cos(),
            tip.y + ny * r * ang.sin() + uy * r * ang.cos(),
        ));
    }
    right.reverse();
    outline.extend(right);
    BranchGeom {
        outline,
        centerline: center,
    }
}

/// Smooth closed or open curve through `pts` as SVG path data, using
/// Catmull Rom to cubic bezier conversion. Offset by (ox, oy).
pub fn smooth_path(pts: &[Pt], ox: f32, oy: f32, closed: bool) -> String {
    let n = pts.len();
    if n < 3 {
        return String::new();
    }
    let at = |i: isize| -> Pt {
        if closed {
            pts[i.rem_euclid(n as isize) as usize]
        } else {
            pts[i.clamp(0, n as isize - 1) as usize]
        }
    };
    let mut d = format!("M{:.1} {:.1}", pts[0].x + ox, pts[0].y + oy);
    let segs = if closed { n } else { n - 1 };
    for i in 0..segs as isize {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let c1 = Pt::new(p1.x + (p2.x - p0.x) / 6.0, p1.y + (p2.y - p0.y) / 6.0);
        let c2 = Pt::new(p2.x - (p3.x - p1.x) / 6.0, p2.y - (p3.y - p1.y) / 6.0);
        d.push_str(&format!(
            " C{:.1} {:.1} {:.1} {:.1} {:.1} {:.1}",
            c1.x + ox,
            c1.y + oy,
            c2.x + ox,
            c2.y + oy,
            p2.x + ox,
            p2.y + oy
        ));
    }
    if closed {
        d.push('Z');
    }
    d
}

/// Points along a hand drawn marker frame (rounded rectangle). The stroke
/// overshoots its start and slips inward, like a real marker loop.
pub fn marker_frame(w: f32, h: f32, seed: u64) -> Vec<Pt> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let mut rng = Rng(seed);
    let (hw, hh) = (w / 2.0, h / 2.0);
    let r = (hh.min(hw) * 0.42).clamp(8.0, 30.0);
    let jit = 1.1;
    let mut out: Vec<Pt> = Vec::new();
    let mut p = |x: f32, y: f32, rng: &mut Rng| {
        out.push(Pt::new(x + rng.signed() * jit, y + rng.signed() * jit));
    };
    let edge = |i: usize, n: usize| i as f32 / n as f32;
    for i in 0..=8 {
        p(-hw + r + (w - 2.0 * r) * edge(i, 8), -hh, &mut rng);
    }
    for i in 0..=6 {
        let a = -FRAC_PI_2 + FRAC_PI_2 * edge(i, 6);
        p(hw - r + r * a.cos(), -hh + r + r * a.sin(), &mut rng);
    }
    for i in 0..=5 {
        p(hw, -hh + r + (h - 2.0 * r) * edge(i, 5), &mut rng);
    }
    for i in 0..=6 {
        let a = FRAC_PI_2 * edge(i, 6);
        p(hw - r + r * a.cos(), hh - r + r * a.sin(), &mut rng);
    }
    for i in 0..=8 {
        p(hw - r - (w - 2.0 * r) * edge(i, 8), hh, &mut rng);
    }
    for i in 0..=6 {
        let a = FRAC_PI_2 + FRAC_PI_2 * edge(i, 6);
        p(-hw + r + r * a.cos(), hh - r + r * a.sin(), &mut rng);
    }
    for i in 0..=5 {
        p(-hw, hh - r - (h - 2.0 * r) * edge(i, 5), &mut rng);
    }
    for i in 0..=6 {
        let a = PI + FRAC_PI_2 * edge(i, 6);
        p(-hw + r + r * a.cos(), -hh + r + r * a.sin(), &mut rng);
    }
    // Overshoot: a second pass along the top edge, shifted up and inward.
    for i in 1..=5 {
        let t = 0.1 + 0.5 * edge(i, 5);
        p(
            -hw + r + (w - 2.0 * r) * t,
            -hh - 5.0 + 4.0 * edge(i, 5),
            &mut rng,
        );
    }
    out
}

/// Cloud outline made of scalloped arcs, as SVG path data relative to centre.
pub fn cloud_path(w: f32, h: f32, seed: u64) -> String {
    let mut rng = Rng(seed);
    let (hw, hh) = (w / 2.0, h / 2.0);
    let n = ((w + h) / 26.0).round().max(8.0) as usize;
    // Sample an ellipse perimeter and bulge each arc outward.
    let pt = |i: usize| {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        Pt::new(hw * a.cos(), hh * a.sin())
    };
    let mut d = String::new();
    let start = pt(0);
    d.push_str(&format!("M{:.1} {:.1}", start.x, start.y));
    for i in 0..n {
        let a = pt(i);
        let b = pt(i + 1);
        let mid = a.lerp(b, 0.5);
        let k = 1.42 + rng.signed() * 0.05;
        let ctrl = Pt::new(mid.x * k, mid.y * k);
        d.push_str(&format!(
            " Q{:.1} {:.1} {:.1} {:.1}",
            ctrl.x, ctrl.y, b.x, b.y
        ));
    }
    d.push('Z');
    d
}
