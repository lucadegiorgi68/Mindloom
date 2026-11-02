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

#[test]
fn reparented_branch_takes_target_branch_color() {
    let mut m = MindMap::new("Root");
    let a = m.add_child(1, "A").unwrap();
    let b = m.add_child(1, "B").unwrap();
    let a1 = m.add_child(a, "A1").unwrap();
    let a1x = m.add_child(a1, "A1x").unwrap();
    // A user colour deep in the moved subtree must not survive the move.
    m.nodes.get_mut(&a1x).unwrap().color = Some("#123456".into());
    let target = m.color_of(b);
    assert_ne!(m.color_of(a1), target);
    assert!(m.reparent(a1, b));
    assert_eq!(m.color_of(a1), target);
    assert_eq!(m.color_of(a1x), target);
}

#[test]
fn main_branch_moved_under_another_takes_its_color() {
    let mut m = MindMap::new("Root");
    let a = m.add_child(1, "A").unwrap();
    let b = m.add_child(1, "B").unwrap();
    assert!(m.reparent(a, b));
    assert_eq!(m.color_of(a), m.color_of(b));
}

#[test]
fn branch_promoted_to_main_gets_its_own_palette_color() {
    let mut m = MindMap::new("Root");
    let a = m.add_child(1, "A").unwrap();
    let a1 = m.add_child(a, "A1").unwrap();
    assert!(m.reparent(a1, 1));
    assert!(m.nodes[&a1].color.is_some());
    assert_ne!(m.color_of(a1), m.color_of(a));
}

#[test]
fn bubble_and_cloud_both_emit_a_shape_path() {
    let mut m = MindMap::new("Root");
    let a = m.add_child(1, "A").unwrap();
    let b = m.add_child(1, "B").unwrap();
    m.nodes.get_mut(&a).unwrap().shape = Shape::Cloud;
    m.nodes.get_mut(&b).unwrap().shape = Shape::Bubble;
    let s = build_scene(&m);
    for id in [a, b] {
        let n = s.nodes.iter().find(|n| n.id == id).unwrap();
        assert!(n
            .shape_path
            .as_deref()
            .is_some_and(|p| p.starts_with('M') && p.ends_with('Z')));
    }
}

#[test]
fn files_saved_before_fonts_existed_still_load() {
    let m = sample();
    let mut v = serde_json::to_value(&m).unwrap();
    for n in v["nodes"].as_object_mut().unwrap().values_mut() {
        n.as_object_mut().unwrap().remove("font");
    }
    let back: MindMap = serde_json::from_value(v).unwrap();
    assert!(back.nodes.values().all(|n| n.font == fonts::DEFAULT_FONT));
}

#[test]
fn new_branches_inherit_parent_font_and_siblings_copy_theirs() {
    let mut m = MindMap::new("Root");
    let a = m.add_child(1, "A").unwrap();
    m.nodes.get_mut(&a).unwrap().font = "Caveat".into();
    let a1 = m.add_child(a, "A1").unwrap();
    assert_eq!(m.nodes[&a1].font, "Caveat");
    m.nodes.get_mut(&a1).unwrap().font = "Lora".into();
    let sib = m.add_sibling(a1, "A2").unwrap();
    assert_eq!(m.nodes[&sib].font, "Lora");
}

#[test]
fn every_catalogue_font_resolves_and_unknown_falls_back() {
    for f in fonts::FONTS {
        assert_eq!(fonts::resolve(f.name).name, f.name);
        assert!(!f.files.is_empty() && f.width > 0.2 && f.width < 0.9);
    }
    assert_eq!(fonts::resolve("Comic Nonsense").name, fonts::DEFAULT_FONT);
    let mut m = MindMap::new("Root");
    m.nodes.get_mut(&1).unwrap().font = "Comic Nonsense".into();
    let s = build_scene(&m);
    assert_eq!(s.nodes[0].font, fonts::DEFAULT_FONT);
}

#[test]
fn layout_width_depends_on_the_font() {
    let width = |font: &str| {
        let mut m = MindMap::new("Root");
        let a = m.add_child(1, "Electromagnetic force").unwrap();
        m.nodes.get_mut(&a).unwrap().font = font.into();
        layout::layout(&m)[&a].w
    };
    assert!(width("Caveat") < width("Open Sans"));
    assert!(width("Open Sans") < width("Montserrat"));
}

// ---------- links ----------

fn two_far_nodes() -> (MindMap, u64, u64) {
    let mut m = MindMap::new("Root");
    let a = m.add_child(1, "Alpha").unwrap();
    let b = m.add_child(1, "Beta").unwrap();
    (m, a, b)
}

#[test]
fn add_link_rejects_self_unknown_and_duplicates() {
    let (mut m, a, b) = two_far_nodes();
    assert!(m.add_link(a, a).is_none());
    assert!(m.add_link(a, 999).is_none());
    assert!(m.add_link(999, a).is_none());
    let id = m.add_link(a, b).unwrap();
    assert!(m.add_link(a, b).is_none(), "same direction twice");
    assert!(
        m.add_link(b, a).is_some(),
        "the reverse direction is a different relation"
    );
    assert_ne!(m.add_link(a, 1).unwrap(), id);
}

#[test]
fn new_link_defaults_to_solid_with_end_arrow() {
    let (mut m, a, b) = two_far_nodes();
    m.add_link(a, b).unwrap();
    let l = &m.links[0];
    assert_eq!(l.style, LinkStyle::Solid);
    assert_eq!(l.arrow, LinkArrow::End);
    assert!(l.color.is_none());
}

#[test]
fn deleting_a_node_removes_its_links_only() {
    let mut m = MindMap::new("Root");
    let a = m.add_child(1, "A").unwrap();
    let a1 = m.add_child(a, "A1").unwrap();
    let b = m.add_child(1, "B").unwrap();
    let c = m.add_child(1, "C").unwrap();
    m.add_link(a1, b).unwrap();
    m.add_link(b, c).unwrap();
    assert!(m.delete(a)); // takes A1 with it
    assert_eq!(m.links.len(), 1);
    assert_eq!((m.links[0].from, m.links[0].to), (b, c));
}

#[test]
fn link_ids_never_collide_with_live_links() {
    let (mut m, a, b) = two_far_nodes();
    let l1 = m.add_link(a, b).unwrap();
    let l2 = m.add_link(b, a).unwrap();
    assert!(m.delete_link(l1));
    let l3 = m.add_link(a, 1).unwrap();
    assert!(l3 != l2, "a new link must not reuse a live id");
    assert!(!m.delete_link(12345));
}

#[test]
fn files_saved_before_links_existed_still_load() {
    let m = sample();
    let mut v = serde_json::to_value(&m).unwrap();
    v.as_object_mut().unwrap().remove("links");
    let back: MindMap = serde_json::from_value(v).unwrap();
    assert!(back.links.is_empty());
}

#[test]
fn links_survive_serialisation() {
    let (mut m, a, b) = two_far_nodes();
    let id = m.add_link(a, b).unwrap();
    m.links[0].style = LinkStyle::Dashed;
    m.links[0].arrow = LinkArrow::Both;
    m.links[0].color = Some("#ff0000".into());
    let back: MindMap = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
    let l = back.links.iter().find(|l| l.id == id).unwrap();
    assert_eq!(
        (l.style, l.arrow, l.color.as_deref()),
        (LinkStyle::Dashed, LinkArrow::Both, Some("#ff0000"))
    );
}

fn rect_has(r: &Rect, p: &Pt, slack: f32) -> bool {
    p.x > r.left() - slack
        && p.x < r.right() + slack
        && p.y > r.top() - slack
        && p.y < r.bottom() + slack
}

#[test]
fn link_curve_starts_and_ends_just_outside_the_nodes() {
    let (mut m, a, b) = two_far_nodes();
    m.add_link(a, b).unwrap();
    m.links[0].arrow = LinkArrow::None;
    let s = build_scene(&m);
    let rects = layout::layout(&m);
    let l = &s.links[0];
    assert!(l.arrow_end.is_empty() && l.arrow_start.is_empty());
    let pts = links_points(&l.path);
    let (first, last) = (pts[0], *pts.last().unwrap());
    assert!(!rects[&a].left().is_nan());
    // Endpoints hug the node frames (padding 5) without entering them.
    assert!(rect_has(&rects[&a], &first, 6.5) && !rect_has(&rects[&a], &first, 3.0));
    assert!(rect_has(&rects[&b], &last, 6.5) && !rect_has(&rects[&b], &last, 3.0));
}

fn links_points(path: &str) -> Vec<Pt> {
    let nums: Vec<f32> = path
        .split(|c: char| c == 'M' || c == 'L' || c.is_whitespace())
        .filter_map(|t| t.parse().ok())
        .collect();
    nums.chunks_exact(2).map(|c| Pt::new(c[0], c[1])).collect()
}

#[test]
fn arrow_options_control_which_heads_exist_and_trim_the_line() {
    let (mut m, a, b) = two_far_nodes();
    m.add_link(a, b).unwrap();
    let heads = |arrow: LinkArrow, m: &mut MindMap| {
        m.links[0].arrow = arrow;
        let l = build_scene(m).links.remove(0);
        (
            l.arrow_start.len(),
            l.arrow_end.len(),
            links_points(&l.path),
        )
    };
    let (s0, e0, none_pts) = heads(LinkArrow::None, &mut m);
    let (s1, e1, end_pts) = heads(LinkArrow::End, &mut m);
    let (s2, e2, start_pts) = heads(LinkArrow::Start, &mut m);
    let (s3, e3, both_pts) = heads(LinkArrow::Both, &mut m);
    assert_eq!((s0, e0), (0, 0));
    assert_eq!((s1, e1), (0, 4));
    assert_eq!((s2, e2), (4, 0));
    assert_eq!((s3, e3), (4, 4));
    let len = |p: &[Pt]| {
        p.windows(2)
            .map(|w| ((w[1].x - w[0].x).powi(2) + (w[1].y - w[0].y).powi(2)).sqrt())
            .sum::<f32>()
    };
    // The line stops short under an arrowhead instead of poking through it.
    assert!(len(&end_pts) < len(&none_pts) - 5.0);
    assert!(len(&start_pts) < len(&none_pts) - 5.0);
    assert!(len(&both_pts) < len(&end_pts) - 5.0);
}

#[test]
fn arrowhead_tip_sits_on_the_target_end_of_the_curve() {
    let (mut m, a, b) = two_far_nodes();
    m.add_link(a, b).unwrap();
    let l = build_scene(&m).links.remove(0);
    let rects = layout::layout(&m);
    let tip = l.arrow_end[0];
    assert!(rect_has(&rects[&b], &tip, 6.5) && !rect_has(&rects[&b], &tip, 3.0));
}

#[test]
fn link_to_a_hidden_node_attaches_to_its_visible_ancestor() {
    let mut m = MindMap::new("Root");
    let a = m.add_child(1, "A").unwrap();
    let a1 = m.add_child(a, "A1").unwrap();
    let b = m.add_child(1, "B").unwrap();
    m.add_link(a1, b).unwrap();
    assert_eq!(build_scene(&m).links.len(), 1);
    m.nodes.get_mut(&a).unwrap().collapsed = true;
    let s = build_scene(&m);
    assert_eq!(s.links.len(), 1, "link stays, anchored on A");
    let rects = layout::layout(&m);
    let first = links_points(&s.links[0].path)[0];
    assert!(rect_has(&rects[&a], &first, 7.0));
}

#[test]
fn link_inside_one_folded_branch_is_hidden() {
    let mut m = MindMap::new("Root");
    let a = m.add_child(1, "A").unwrap();
    let a1 = m.add_child(a, "A1").unwrap();
    let a2 = m.add_child(a, "A2").unwrap();
    m.add_link(a1, a2).unwrap();
    assert_eq!(build_scene(&m).links.len(), 1);
    m.nodes.get_mut(&a).unwrap().collapsed = true;
    assert!(build_scene(&m).links.is_empty());
}

#[test]
fn links_follow_moved_nodes_and_count_toward_bounds() {
    let (mut m, a, b) = two_far_nodes();
    m.add_link(a, b).unwrap();
    let before = links_points(&build_scene(&m).links[0].path)[0];
    m.nodes.get_mut(&a).unwrap().dy = 300.0;
    let s = build_scene(&m);
    let after = links_points(&s.links[0].path)[0];
    assert!((after.y - before.y).abs() > 100.0);
    let maxy = s
        .nodes
        .iter()
        .map(|n| n.rect.bottom())
        .fold(f32::MIN, f32::max);
    assert!(s.bounds.bottom() >= maxy - 0.01);
}

#[test]
fn a_pair_linked_both_ways_draws_two_distinct_curves() {
    let (mut m, a, b) = two_far_nodes();
    m.add_link(a, b).unwrap();
    m.add_link(b, a).unwrap();
    let s = build_scene(&m);
    assert_eq!(s.links.len(), 2);
    assert_ne!(s.links[0].path, s.links[1].path);
}

#[test]
fn link_bend_is_clamped_and_unknown_links_refused() {
    let (mut m, a, b) = two_far_nodes();
    let id = m.add_link(a, b).unwrap();
    assert!(m.set_link_bend(id, -0.4));
    assert_eq!(m.links[0].bend, -0.4);
    assert!(m.set_link_bend(id, 50.0));
    assert_eq!(m.links[0].bend, 1.5);
    assert!(!m.set_link_bend(id, f32::NAN));
    assert!(!m.set_link_bend(999, 0.1));
}

#[test]
fn reconnecting_a_link_validates_its_new_ends() {
    let (mut m, a, b) = two_far_nodes();
    let c = m.add_child(1, "Gamma").unwrap();
    let l1 = m.add_link(a, b).unwrap();
    let l2 = m.add_link(a, c).unwrap();
    assert!(!m.reconnect_link(l1, a, a), "self link");
    assert!(!m.reconnect_link(l1, a, 999), "unknown node");
    assert!(
        !m.reconnect_link(l1, a, c),
        "would duplicate the other link"
    );
    assert!(m.reconnect_link(l1, a, b), "keeping its own ends is fine");
    assert!(m.reconnect_link(l1, b, c));
    assert_eq!((m.links[0].from, m.links[0].to), (b, c));
    assert_eq!((m.links[1].id, m.links[1].to), (l2, c));
}

#[test]
fn scene_link_exposes_handle_geometry() {
    let (mut m, a, b) = two_far_nodes();
    let id = m.add_link(a, b).unwrap();
    m.set_link_bend(id, 0.5);
    let s = mindmap_core::build_scene(&m);
    let l = &s.links[0];
    assert_eq!(l.bend, 0.5);
    assert_ne!((l.ca.x, l.ca.y), (l.cb.x, l.cb.y));
    let (mx, my) = ((l.ca.x + l.cb.x) / 2.0, (l.ca.y + l.cb.y) / 2.0);
    let off = ((l.mid.x - mx).powi(2) + (l.mid.y - my).powi(2)).sqrt();
    let dist = ((l.cb.x - l.ca.x).powi(2) + (l.cb.y - l.ca.y).powi(2)).sqrt();
    assert!(
        (off / dist - 0.75 * 0.5).abs() < 0.05,
        "midpoint sits 0.75 * bend away"
    );
}

#[test]
fn second_central_idea_starts_its_own_tree() {
    let mut m = sample();
    let c = m.add_central_idea("Second");
    assert_eq!(m.roots(), vec![1, c]);
    assert!(m.is_central(c) && m.is_central(1));
    let n = &m.nodes[&c];
    assert_eq!(n.parent, None);
    assert_eq!(n.shape, Shape::Marker);
    assert_ne!(m.color_of(c), m.color_of(1));
    // It takes children like the first one, balanced over both sides.
    let k1 = m.add_child(c, "x").unwrap();
    let k2 = m.add_child(c, "y").unwrap();
    assert_ne!(m.nodes[&k1].side, m.nodes[&k2].side);
    assert_ne!(m.color_of(k1), m.color_of(k2));
    // A central idea has no siblings and cannot be moved under another node.
    assert!(m.add_sibling(c, "z").is_none());
    assert!(!m.reparent(c, 1));
    // But branches can move between the two trees and adopt the new colour.
    let a = m.nodes[&1].children[0];
    assert!(m.reparent(a, c));
    assert_eq!(m.nodes[&a].parent, Some(c));
    assert!(m.nodes[&1].children.iter().all(|x| *x != a));
}

#[test]
fn first_central_idea_stays_but_later_ones_can_be_deleted() {
    let mut m = sample();
    let c = m.add_central_idea("Second");
    let k = m.add_child(c, "x").unwrap();
    let total = m.nodes.len();
    m.add_link(k, 1).unwrap();
    assert!(!m.delete(1));
    assert!(m.delete(c));
    assert_eq!(m.roots(), vec![1]);
    assert_eq!(m.nodes.len(), total - 2);
    assert!(m.links.is_empty(), "links to a deleted tree go with it");
}

#[test]
fn central_ideas_never_overlap_and_dragging_one_leaves_the_rest() {
    let mut m = sample();
    let c = m.add_central_idea("Second");
    for i in 0..4 {
        m.add_child(c, &format!("k{i}")).unwrap();
    }
    let rects = layout::layout(&m);
    let in_tree = |root: NodeId| -> Vec<Rect> {
        let mut out = vec![];
        m.visit_depth_first(|n, _| {
            if n.id == root || m.is_descendant(n.id, root) {
                out.push(rects[&n.id]);
            }
        });
        out
    };
    let first = in_tree(1);
    let second = in_tree(c);
    let first_bottom = first.iter().map(|r| r.bottom()).fold(f32::MIN, f32::max);
    let second_top = second.iter().map(|r| r.top()).fold(f32::MAX, f32::min);
    assert!(
        second_top > first_bottom,
        "second tree sits below the first"
    );
    assert_eq!(rects[&1].cx, 0.0);
    assert_eq!(rects[&1].cy, 0.0);
    // Dragging the second central idea moves its tree and nothing else.
    let before = rects.clone();
    m.nodes.get_mut(&c).unwrap().dx = 250.0;
    m.nodes.get_mut(&c).unwrap().dy = -60.0;
    let after = layout::layout(&m);
    for id in m.nodes.keys() {
        let (b, a) = (before[id], after[id]);
        if *id == c || m.is_descendant(*id, c) {
            assert!((a.cx - b.cx - 250.0).abs() < 0.01 && (a.cy - b.cy + 60.0).abs() < 0.01);
        } else {
            assert_eq!((a.cx, a.cy), (b.cx, b.cy));
        }
    }
}

#[test]
fn scene_draws_every_central_idea_with_its_branches() {
    let mut m = sample();
    let c = m.add_central_idea("Second");
    m.add_child(c, "x").unwrap();
    let s = build_scene(&m);
    assert_eq!(s.nodes.len(), 8);
    assert_eq!(s.nodes.iter().filter(|n| n.central).count(), 2);
    assert_eq!(s.branches.len(), 6);
    assert!(s.nodes.iter().any(|n| n.id == c && n.central));
}

#[test]
fn old_files_without_extra_roots_still_load() {
    let json = serde_json::to_string(&sample()).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    v.as_object_mut().unwrap().remove("extra_roots");
    let m: MindMap = serde_json::from_value(v).unwrap();
    assert_eq!(m.roots(), vec![1]);
}

// ---------- Images ----------

fn with_image() -> (MindMap, u64) {
    let mut m = sample();
    // A 200 x 100 picture, so the aspect ratio is 2:1.
    let asset = m.add_asset("image/png", "AAAA", 200, 100).unwrap();
    (m, asset)
}

#[test]
fn assets_reject_bad_input_and_reuse_identical_pictures() {
    let mut m = sample();
    assert!(m.add_asset("text/html", "AAAA", 10, 10).is_none());
    assert!(m.add_asset("image/png", "", 10, 10).is_none());
    assert!(m.add_asset("image/png", "AAAA", 0, 10).is_none());
    let a = m.add_asset("image/png", "AAAA", 10, 10).unwrap();
    assert_eq!(m.add_asset("image/png", "AAAA", 10, 10), Some(a));
    assert_ne!(m.add_asset("image/png", "BBBB", 10, 10), Some(a));
    assert_eq!(m.assets.len(), 2);
}

#[test]
fn image_node_is_a_child_with_aspect_ratio_size() {
    let (mut m, asset) = with_image();
    let id = m.add_image_child(1, asset, Some(160.0)).unwrap();
    assert_eq!(m.nodes[&id].parent, Some(1));
    assert!(m.nodes[&1].children.contains(&id));
    assert_eq!(m.node_image_size(id), Some((160.0, 80.0)));
    // Without a width it starts at its natural width, capped.
    let id2 = m.add_image_child(1, asset, None).unwrap();
    assert_eq!(m.node_image_size(id2), Some((200.0, 100.0)));
    assert!(m.add_image_child(1, 999, None).is_none());
}

#[test]
fn image_width_is_clamped_and_keeps_aspect() {
    let (mut m, asset) = with_image();
    let id = m.add_image_child(1, asset, Some(100.0)).unwrap();
    assert!(m.set_node_image_width(id, 1.0));
    assert_eq!(m.node_image_size(id).unwrap().0, model::IMAGE_MIN_WIDTH);
    assert!(m.set_node_image_width(id, 99999.0));
    assert_eq!(m.node_image_size(id).unwrap().0, model::IMAGE_MAX_WIDTH);
    assert!(!m.set_node_image_width(id, f32::NAN));
    assert!(!m.set_node_image_width(1, 100.0)); // the root has no picture
    let (w, h) = m.node_image_size(id).unwrap();
    assert!((h - w / 2.0).abs() < 0.01);
}

#[test]
fn picture_can_be_attached_to_and_removed_from_a_node() {
    let (mut m, asset) = with_image();
    let a = m.nodes[&1].children[0];
    assert!(m.set_node_image(a, asset, Some(120.0)));
    assert_eq!(m.nodes[&a].text, "A", "the text stays as a caption");
    assert!(m.remove_node_image(a));
    assert!(m.nodes[&a].image.is_none());
    assert!(!m.remove_node_image(a));
}

#[test]
fn image_node_layout_makes_room_for_the_picture_and_caption() {
    let (mut m, asset) = with_image();
    let id = m.add_image_child(1, asset, Some(200.0)).unwrap();
    let plain = layout::layout(&m)[&id];
    assert!(plain.w >= 200.0 && plain.h >= 100.0);
    m.set_text(id, "Caption");
    let captioned = layout::layout(&m)[&id];
    assert!(captioned.h > plain.h, "a caption adds height");
    m.set_node_image_width(id, 400.0);
    assert!(layout::layout(&m)[&id].w >= 400.0);
}

#[test]
fn scene_places_the_picture_inside_the_node_rect() {
    let (mut m, asset) = with_image();
    let id = m.add_image_child(1, asset, Some(200.0)).unwrap();
    m.set_text(id, "Cap");
    let scene = build_scene(&m);
    let n = scene.nodes.iter().find(|n| n.id == id).unwrap();
    let img = n.image.as_ref().expect("scene carries the picture");
    assert_eq!(img.asset, asset);
    assert_eq!((img.rect.w, img.rect.h), (200.0, 100.0));
    assert!(img.rect.top() >= n.rect.top() && img.rect.bottom() <= n.rect.bottom());
    assert!(
        n.text_baseline > img.rect.bottom(),
        "the caption sits below the picture"
    );
    // The branch joins the picture node at its side, not on an underline.
    let br = scene.branches.iter().find(|b| b.to == id).unwrap();
    let end = br.centerline.last().unwrap();
    assert!((end.y - n.rect.cy).abs() < 1.0, "joined at mid height");
}

#[test]
fn free_images_live_outside_the_tree() {
    let (mut m, asset) = with_image();
    let nodes_before = m.nodes.len();
    let f = m.add_free_image(asset, 500.0, -300.0, Some(150.0)).unwrap();
    assert_eq!(m.nodes.len(), nodes_before, "no node is created");
    let scene = build_scene(&m);
    let img = scene.images.iter().find(|i| i.id == f).unwrap();
    assert_eq!((img.rect.cx, img.rect.cy), (500.0, -300.0));
    assert_eq!((img.rect.w, img.rect.h), (150.0, 75.0));
    assert!(
        scene.bounds.right() >= 575.0,
        "bounds include free pictures"
    );
    assert!(m.move_free_image(f, 10.0, 20.0));
    assert_eq!((m.free_images[0].x, m.free_images[0].y), (510.0, -280.0));
    assert!(m.set_free_image_width(f, 300.0, Some((0.0, 0.0))));
    assert_eq!(
        (
            m.free_images[0].width,
            m.free_images[0].x,
            m.free_images[0].y
        ),
        (300.0, 0.0, 0.0)
    );
    assert!(m.delete_free_image(f));
    assert!(!m.delete_free_image(f));
    assert!(m.add_free_image(asset, f32::NAN, 0.0, None).is_none());
}

#[test]
fn unused_pictures_are_pruned_but_used_ones_stay() {
    let (mut m, asset) = with_image();
    let other = m.add_asset("image/png", "BBBB", 10, 10).unwrap();
    m.add_free_image(asset, 0.0, 0.0, None).unwrap();
    m.prune_assets();
    assert!(m.assets.contains_key(&asset));
    assert!(!m.assets.contains_key(&other));
}

#[test]
fn maps_saved_before_images_existed_still_load() {
    let json = serde_json::to_value(sample()).unwrap();
    let mut v = json.clone();
    v.as_object_mut().unwrap().remove("assets");
    v.as_object_mut().unwrap().remove("free_images");
    for n in v["nodes"].as_object_mut().unwrap().values_mut() {
        n.as_object_mut().unwrap().remove("image");
    }
    let m: MindMap = serde_json::from_value(v).unwrap();
    assert!(m.assets.is_empty() && m.free_images.is_empty());
    assert!(m.nodes.values().all(|n| n.image.is_none()));
}
