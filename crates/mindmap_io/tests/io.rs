use mindmap_core::MindMap;
use mindmap_io::*;

fn sample() -> MindMap {
    let mut m = MindMap::new("Atom & <Co>");
    let a = m.add_child(1, "Nucleus").unwrap();
    m.add_child(a, "Protons").unwrap();
    m.add_child(1, "Electrons").unwrap();
    m
}

#[test]
fn json_roundtrip() {
    let m = sample();
    let back = from_json(&to_json(&m)).unwrap();
    assert_eq!(back.nodes.len(), m.nodes.len());
    assert_eq!(back.nodes[&1].text, "Atom & <Co>");
}

#[test]
fn json_rejects_foreign_and_newer_files() {
    assert!(from_json("{\"hello\":1}").is_err());
    let newer = to_json(&sample()).replace("\"version\": 1", "\"version\": 99");
    assert!(from_json(&newer).unwrap_err().contains("newer"));
}

#[test]
fn svg_escapes_text_and_contains_every_label() {
    let svg = to_svg(&sample());
    assert!(svg.contains("Atom &amp; &lt;Co&gt;"));
    for t in ["Nucleus", "Protons", "Electrons"] {
        assert!(svg.contains(t));
    }
    assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
}

#[test]
fn markdown_outline() {
    let md = to_markdown(&sample());
    assert!(md.starts_with("# Atom & <Co>"));
    assert!(md.contains("* Nucleus\n  * Protons\n* Electrons"));
}

#[test]
fn opml_roundtrip_preserves_structure() {
    let m = sample();
    let back = from_opml(&to_opml(&m)).unwrap();
    assert_eq!(to_markdown(&back), to_markdown(&m));
}

#[test]
fn opml_rejects_garbage() {
    assert!(from_opml("hello").is_err());
}

#[test]
fn svg_embeds_only_the_fonts_the_map_uses() {
    let mut m = MindMap::new("Root");
    let a = m.add_child(1, "Hello").unwrap();
    m.nodes.get_mut(&a).unwrap().font = "Caveat".into();
    let svg = to_svg(&m);
    assert!(svg.contains("font-family:'Caveat'"));
    assert!(svg.contains("font-family:'Open Sans'"));
    assert!(!svg.contains("font-family:'Pacifico'"));
    assert!(svg.contains("data:font/woff2;base64,"));
    assert!(svg.contains("font-family=\"'Caveat', 'Segoe Print', cursive\""));
}

#[test]
fn svg_embeds_bold_weight_only_when_used() {
    let mut m = MindMap::new("Root");
    assert!(!to_svg(&m).contains("font-weight:700"));
    m.nodes.get_mut(&1).unwrap().bold = true;
    assert!(to_svg(&m).contains("font-weight:700"));
}

#[test]
fn svg_draws_links_with_dashes_and_arrowheads() {
    let mut m = MindMap::new("Root");
    let a = m.add_child(1, "A").unwrap();
    let b = m.add_child(1, "B").unwrap();
    let lid = m.add_link(a, b).unwrap();
    let solid = to_svg(&m);
    assert!(solid.contains("<polygon"), "end arrow by default");
    assert!(!solid.contains("stroke-dasharray=\"9.0 7.2\""));
    let l = m.links.iter_mut().find(|l| l.id == lid).unwrap();
    l.style = mindmap_core::LinkStyle::Dashed;
    l.arrow = mindmap_core::LinkArrow::None;
    l.color = Some("#ff0000".into());
    let dashed = to_svg(&m);
    assert!(dashed.contains("stroke-dasharray=\"9.0 7.2\""));
    assert!(dashed.contains("stroke=\"#ff0000\""));
    assert!(!dashed.contains("<polygon"));
}

#[test]
fn json_roundtrip_keeps_links() {
    let mut m = MindMap::new("Root");
    let a = m.add_child(1, "A").unwrap();
    m.add_link(a, 1).unwrap();
    assert_eq!(from_json(&to_json(&m)).unwrap().links.len(), 1);
}
