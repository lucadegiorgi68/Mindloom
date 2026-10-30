use mindmap_core::*;

fn sample() -> MindMap {
    let mut m = MindMap::new("Root");
    let a = m.add_child(1, "A").unwrap();
    let b = m.add_child(1, "B").unwrap();
    m.add_child(a, "A1").unwrap();
    m.add_child(a, "A2").unwrap();
    m.add_child(b, "B1").unwrap();
    m
}

#[test]
fn root_children_balance_between_sides() {
    let mut m = MindMap::new("Root");
    for i in 0..6 {
        m.add_child(1, &format!("n{i}")).unwrap();
    }
    let right = m
        .nodes
        .values()
        .filter(|n| n.parent == Some(1) && n.side == Side::Right)
        .count();
    assert_eq!(right, 3);
}

#[test]
fn main_branches_get_distinct_colors() {
    let m = sample();
    let ids: Vec<_> = m.nodes[&1].children.clone();
    assert_ne!(m.color_of(ids[0]), m.color_of(ids[1]));
}

#[test]
fn grandchildren_inherit_branch_color() {
    let m = sample();
    let a = m.nodes[&1].children[0];
    let a1 = m.nodes[&a].children[0];
    assert_eq!(m.color_of(a), m.color_of(a1));
}

#[test]
fn delete_removes_subtree_and_protects_root() {
    let mut m = sample();
    let a = m.nodes[&1].children[0];
    assert!(m.delete(a));
    assert_eq!(m.nodes.len(), 3); // Root, B, B1
    assert!(!m.delete(1));
}

#[test]
fn reparent_refuses_cycles() {
    let mut m = sample();
    let a = m.nodes[&1].children[0];
    let a1 = m.nodes[&a].children[0];
    assert!(!m.reparent(a, a1));
    assert!(!m.reparent(a, a));
    assert!(!m.reparent(1, a));
    let b = m.nodes[&1].children[1];
    assert!(m.reparent(a1, b));
    assert_eq!(m.nodes[&a1].parent, Some(b));
    assert!(!m.nodes[&a].children.contains(&a1));
}

#[test]
fn reparent_moves_subtree_to_new_side() {
    let mut m = MindMap::new("Root");
    let a = m.add_child(1, "A").unwrap(); // right
    let b = m.add_child(1, "B").unwrap(); // left
    let a1 = m.add_child(a, "A1").unwrap();
    assert_eq!(m.nodes[&a1].side, Side::Right);
    assert!(m.reparent(a1, b));
    assert_eq!(m.nodes[&a1].side, Side::Left);
}

#[test]
fn sibling_is_inserted_right_after() {
    let mut m = sample();
    let a = m.nodes[&1].children[0];
    let a1 = m.nodes[&a].children[0];
    let s = m.add_sibling(a1, "mid").unwrap();
    assert_eq!(m.nodes[&a].children[1], s);
    assert!(m.add_sibling(1, "x").is_none());
}

#[test]
fn undo_redo_roundtrip() {
    let mut d = Document::new(MindMap::new("Root"));
    d.edit(|m| m.add_child(1, "A"));
    d.edit(|m| m.add_child(1, "B"));
    assert_eq!(d.map.nodes.len(), 3);
    assert!(d.undo());
    assert_eq!(d.map.nodes.len(), 2);
    assert!(d.redo());
    assert_eq!(d.map.nodes.len(), 3);
    // A new edit clears redo.
    d.undo();
    d.edit(|m| m.add_child(1, "C"));
    assert!(!d.can_redo());
}

#[test]
fn failed_edit_leaves_no_history() {
    let mut d = Document::new(MindMap::new("Root"));
    assert!(d.edit(|m| m.delete(1).then_some(())).is_none());
    assert!(!d.can_undo());
}

#[test]
fn layout_has_no_sibling_overlap() {
    let m = sample();
    let rects = layout::layout(&m);
    let a = m.nodes[&1].children[0];
    let kids = &m.nodes[&a].children;
    let (r0, r1) = (rects[&kids[0]], rects[&kids[1]]);
    assert!(r0.bottom() <= r1.top(), "siblings overlap: {r0:?} {r1:?}");
}

#[test]
fn layout_puts_sides_on_opposite_sides_of_root() {
    let m = sample();
    let rects = layout::layout(&m);
    let kids = &m.nodes[&1].children;
    assert!(rects[&kids[0]].cx > 0.0);
    assert!(rects[&kids[1]].cx < 0.0);
}

#[test]
fn manual_offset_moves_whole_subtree() {
    let mut m = sample();
    let a = m.nodes[&1].children[0];
    let a1 = m.nodes[&a].children[0];
    let before = layout::layout(&m)[&a1];
    m.nodes.get_mut(&a).unwrap().dy = 100.0;
    let after = layout::layout(&m)[&a1];
    assert!((after.cy - before.cy - 100.0).abs() < 0.01);
}

#[test]
fn collapsed_node_hides_descendants_in_scene() {
    let mut m = sample();
    let a = m.nodes[&1].children[0];
    let full = build_scene(&m).nodes.len();
    m.nodes.get_mut(&a).unwrap().collapsed = true;
    assert_eq!(build_scene(&m).nodes.len(), full - 2);
}

#[test]
fn branch_outline_tapers_and_is_deterministic() {
    use organic::branch;
    let (a, b) = (Pt::new(0.0, 0.0), Pt::new(200.0, 40.0));
    let g1 = branch(a, b, 16.0, 3.0, 7, 0.7, 50.0, 1.0);
    let g2 = branch(a, b, 16.0, 3.0, 7, 0.7, 50.0, 1.0);
    assert_eq!(g1.outline.len(), g2.outline.len());
    assert!(g1.outline.iter().zip(&g2.outline).all(|(p, q)| p == q));
    // The stroke must be thicker at the start than at the end.
    let n = g1.centerline.len();
    let half = g1.outline.len() / 2;
    let w = |i: usize| {
        let (l, r) = (g1.outline[i], g1.outline[g1.outline.len() - 1 - i]);
        ((l.x - r.x).powi(2) + (l.y - r.y).powi(2)).sqrt()
    };
    assert!(n > 10 && half > 5);
    assert!(
        w(0) > w(half - 4) * 1.5,
        "no taper: {} vs {}",
        w(0),
        w(half - 4)
    );
}

#[test]
fn scene_json_roundtrips() {
    let s = build_scene(&sample());
    let j = serde_json::to_string(&s).unwrap();
    let back: Scene = serde_json::from_str(&j).unwrap();
    assert_eq!(back.nodes.len(), s.nodes.len());
}
