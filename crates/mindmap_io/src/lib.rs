//! Import and export for Mindloom maps.
use mindmap_core::{build_scene, scene::Scene, BranchStyle, MindMap, Shape};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

pub const FILE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct FileEnvelope {
    format: String,
    version: u32,
    map: MindMap,
}

pub fn to_json(map: &MindMap) -> String {
    serde_json::to_string_pretty(&FileEnvelope {
        format: "mindloom".into(),
        version: FILE_VERSION,
        map: map.clone(),
    })
    .expect("map serialises")
}

pub fn from_json(s: &str) -> Result<MindMap, String> {
    let env: FileEnvelope = serde_json::from_str(s).map_err(|e| e.to_string())?;
    if env.format != "mindloom" {
        return Err("not a Mindloom file".into());
    }
    if env.version > FILE_VERSION {
        return Err(format!(
            "file version {} is newer than this app",
            env.version
        ));
    }
    Ok(env.map)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn path_d(pts: &[mindmap_core::Pt], ox: f32, oy: f32, close: bool) -> String {
    let mut d = String::new();
    for (i, p) in pts.iter().enumerate() {
        let _ = write!(
            d,
            "{}{:.1} {:.1} ",
            if i == 0 { "M" } else { "L" },
            p.x + ox,
            p.y + oy
        );
    }
    if close {
        d.push('Z');
    }
    d
}

/// Self contained SVG of the whole map.
pub fn to_svg(map: &MindMap) -> String {
    scene_to_svg(&build_scene(map))
}

pub fn scene_to_svg(scene: &Scene) -> String {
    let pad = 60.0;
    let b = scene.bounds;
    let (bg, fg) = if scene.dark_mode {
        ("#1c1f26", "#eceff4")
    } else {
        ("#ffffff", "#333333")
    };
    let mut s = String::new();
    let _ = write!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{:.0} {:.0} {:.0} {:.0}\" width=\"{:.0}\" height=\"{:.0}\" font-family=\"Open Sans, Segoe UI, sans-serif\">",
        b.left() - pad, b.top() - pad, b.w + pad * 2.0, b.h + pad * 2.0, b.w + pad * 2.0, b.h + pad * 2.0
    );
    let _ = write!(
        s,
        "<rect x=\"{:.0}\" y=\"{:.0}\" width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        b.left() - pad,
        b.top() - pad,
        bg
    );
    for br in &scene.branches {
        match br.style {
            BranchStyle::Dotted => {
                let _ = write!(
                    s,
                    "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.1}\" stroke-dasharray=\"1 {:.1}\" stroke-linecap=\"round\"/>",
                    path_d(&br.centerline, 0.0, 0.0, false), br.color, br.width * 0.5, br.width * 1.2
                );
            }
            _ => {
                let _ = write!(
                    s,
                    "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"0.6\" stroke-linejoin=\"round\"/>",
                    path_d(&br.outline, 0.0, 0.0, true), br.color, br.color
                );
            }
        }
    }
    for n in &scene.nodes {
        let r = n.rect;
        match n.shape {
            Shape::Marker => {
                let _ = write!(
                    s,
                    "<path transform=\"translate({:.1} {:.1})\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",
                    r.cx, r.cy, n.frame, n.color
                );
            }
            Shape::RoundedBox => {
                let _ = write!(
                    s,
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"14\" fill=\"none\" stroke=\"{}\" stroke-width=\"3\"/>",
                    r.left(), r.top(), r.w, r.h, n.color
                );
            }
            Shape::Cloud | Shape::Bubble => {
                if let Some(c) = &n.shape_path {
                    let _ = write!(
                        s,
                        "<path transform=\"translate({:.1} {:.1})\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"3\"/>",
                        r.cx, r.cy, c, n.color
                    );
                }
            }
            Shape::None => {}
        }
        let lines: Vec<&str> = n.text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let y = n.text_baseline - (lines.len() - 1 - i) as f32 * n.font_size * 1.25;
            let _ = write!(
                s,
                "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"{:.0}\" fill=\"{}\" font-weight=\"{}\" font-style=\"{}\">{}</text>",
                r.cx, y, n.font_size, fg,
                if n.bold { "700" } else { "400" },
                if n.italic { "italic" } else { "normal" },
                esc(line)
            );
        }
    }
    s.push_str("</svg>");
    s
}

/// Markdown outline: the root becomes a heading, descendants nested bullets.
pub fn to_markdown(map: &MindMap) -> String {
    let mut out = String::new();
    map.visit_depth_first(|n, d| {
        if d == 0 {
            let _ = writeln!(out, "# {}\n", n.text.replace('\n', " "));
        } else {
            let _ = writeln!(out, "{}* {}", "  ".repeat(d - 1), n.text.replace('\n', " "));
        }
    });
    out
}

/// OPML 2.0, readable by most outliners and by FreeMind importers.
pub fn to_opml(map: &MindMap) -> String {
    fn go(map: &MindMap, id: u64, out: &mut String) {
        let n = &map.nodes[&id];
        if n.children.is_empty() {
            let _ = write!(out, "<outline text=\"{}\"/>", esc(&n.text));
        } else {
            let _ = write!(out, "<outline text=\"{}\">", esc(&n.text));
            for c in &n.children {
                go(map, *c, out);
            }
            out.push_str("</outline>");
        }
    }
    let mut body = String::new();
    go(map, map.root, &mut body);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><opml version=\"2.0\"><head><title>{}</title></head><body>{}</body></opml>",
        esc(&map.nodes[&map.root].text),
        body
    )
}

/// Minimal OPML reader: enough for outlines exported by common tools.
pub fn from_opml(xml: &str) -> Result<MindMap, String> {
    fn unesc(s: &str) -> String {
        s.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&")
    }
    let body_start = xml.find("<body>").ok_or("no <body> in OPML")? + 6;
    let body = &xml[body_start..];
    let mut map: Option<MindMap> = None;
    let mut stack: Vec<u64> = vec![];
    let mut i = 0;
    let bytes = body.as_bytes();
    while let Some(off) = body[i..].find('<') {
        i += off;
        let end = i + bytes[i..]
            .iter()
            .position(|b| *b == b'>')
            .ok_or("unterminated tag")?;
        let tag = &body[i + 1..end];
        i = end + 1;
        if tag.starts_with("/outline") {
            stack.pop();
        } else if tag.starts_with("outline") {
            let self_close = tag.ends_with('/');
            let text = tag
                .split("text=\"")
                .nth(1)
                .and_then(|r| r.split('"').next())
                .map(unesc)
                .unwrap_or_default();
            let id = match (&mut map, stack.last()) {
                (None, _) => {
                    let m = MindMap::new(&text);
                    let root = m.root;
                    map = Some(m);
                    root
                }
                (Some(m), Some(p)) => m.add_child(*p, &text).ok_or("bad parent")?,
                (Some(m), None) => m.add_child(m.root, &text).ok_or("bad parent")?,
            };
            if !self_close {
                stack.push(id);
            }
        }
    }
    map.ok_or_else(|| "no outlines found".to_string())
}
