//! Command layer shared by the desktop shell and the dev server.
//! Every UI action is a named command with JSON arguments, so the shell
//! stays a thin wrapper and all behaviour is testable here.
use mindmap_core::{build_scene, Document, LinkArrow, LinkStyle, MindMap, Shape};
use serde_json::{json, Value};

pub struct Session {
    pub doc: Document,
}

fn id(args: &Value, key: &str) -> Result<u64, String> {
    args.get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing '{key}'"))
}

fn is_hex(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|h| h.is_ascii_hexdigit())
}

fn text<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing '{key}'"))
}

/// Largest picture accepted, as base64 text (about 9 MB of image data).
const MAX_IMAGE_B64: usize = 12_000_000;

fn is_base64(s: &str) -> bool {
    s.len() & 3 == 0
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
}

fn num(args: &Value, key: &str) -> Option<f32> {
    args.get(key).and_then(Value::as_f64).map(|v| v as f32)
}

impl Session {
    pub fn new(title: &str) -> Self {
        Self {
            doc: Document::new(MindMap::new(title)),
        }
    }

    fn state(&self) -> Value {
        json!({
            "scene": build_scene(&self.doc.map),
            "can_undo": self.doc.can_undo(),
            "can_redo": self.doc.can_redo(),
            "root": self.doc.map.root,
            "roots": self.doc.map.roots(),
            "title": self.doc.map.nodes[&self.doc.map.root].text,
            "nodes": self.doc.map.nodes.values().map(|n| json!({
                "id": n.id, "bold": n.bold, "italic": n.italic,
                "font_size": n.font_size, "shape": n.shape,
                "font": mindmap_core::fonts::resolve(&n.font).name,
                "image_width": n.image.as_ref().map(|i| i.width),
            })).collect::<Vec<_>>(),
            "fonts": mindmap_core::fonts::FONTS.iter().map(|f| json!({
                "name": f.name, "stack": f.stack,
                "files": f.files.iter().map(|(w, file)| json!({"weight": w, "file": file})).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })
    }

    /// Runs one command. Editing commands return the new state; export
    /// commands return `{ "data": "..." }`.
    pub fn invoke(&mut self, cmd: &str, a: &Value) -> Result<Value, String> {
        match cmd {
            "state" => {}
            "add_child" => {
                let p = id(a, "parent")?;
                let new = self
                    .doc
                    .edit(|m| m.add_child(p, text(a, "text").unwrap_or("")))
                    .ok_or("cannot add child")?;
                let mut s = self.state();
                s["new_id"] = json!(new);
                return Ok(s);
            }
            "add_central_idea" => {
                let new = self
                    .doc
                    .edit(|m| Some(m.add_central_idea(text(a, "text").unwrap_or("Central idea"))))
                    .ok_or("cannot add a central idea")?;
                let mut s = self.state();
                s["new_id"] = json!(new);
                return Ok(s);
            }
            "add_sibling" => {
                let i = id(a, "id")?;
                let new = self
                    .doc
                    .edit(|m| m.add_sibling(i, text(a, "text").unwrap_or("")))
                    .ok_or("the central idea has no siblings")?;
                let mut s = self.state();
                s["new_id"] = json!(new);
                return Ok(s);
            }
            "set_text" => {
                let (i, t) = (id(a, "id")?, text(a, "text")?.to_string());
                self.doc
                    .edit(|m| m.set_text(i, &t).then_some(()))
                    .ok_or("no such node")?;
            }
            "delete" => {
                let i = id(a, "id")?;
                self.doc
                    .edit(|m| m.delete(i).then_some(()))
                    .ok_or("cannot delete this node")?;
            }
            "reparent" => {
                let (i, p) = (id(a, "id")?, id(a, "parent")?);
                self.doc
                    .edit(|m| m.reparent(i, p).then_some(()))
                    .ok_or("cannot move there")?;
            }
            "move_node" => {
                let i = id(a, "id")?;
                let dx = a.get("dx").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                let dy = a.get("dy").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                self.doc
                    .edit(|m| {
                        let n = m.nodes.get_mut(&i)?;
                        n.dx += dx;
                        n.dy += dy;
                        Some(())
                    })
                    .ok_or("no such node")?;
            }
            "set_color" => {
                let (i, c) = (id(a, "id")?, text(a, "color")?.to_string());
                if !(c.len() == 7
                    && c.starts_with('#')
                    && c[1..].chars().all(|h| h.is_ascii_hexdigit()))
                {
                    return Err("colour must look like #rrggbb".into());
                }
                self.doc
                    .edit(|m| {
                        m.nodes.get_mut(&i)?.color = Some(c);
                        Some(())
                    })
                    .ok_or("no such node")?;
            }
            "set_shape" => {
                let i = id(a, "id")?;
                let shape: Shape =
                    serde_json::from_value(a.get("shape").cloned().unwrap_or(Value::Null))
                        .map_err(|_| "unknown shape")?;
                self.doc
                    .edit(|m| {
                        m.nodes.get_mut(&i)?.shape = shape;
                        Some(())
                    })
                    .ok_or("no such node")?;
            }
            "set_format" => {
                let i = id(a, "id")?;
                let font = a.get("font").and_then(Value::as_str);
                if let Some(f) = font {
                    if mindmap_core::fonts::find(f).is_none() {
                        return Err(format!("unknown font '{f}'"));
                    }
                }
                self.doc
                    .edit(|m| {
                        let n = m.nodes.get_mut(&i)?;
                        if let Some(f) = font {
                            n.font = f.to_string();
                        }
                        if let Some(b) = a.get("bold").and_then(Value::as_bool) {
                            n.bold = b;
                        }
                        if let Some(b) = a.get("italic").and_then(Value::as_bool) {
                            n.italic = b;
                        }
                        if let Some(f) = a.get("font_size").and_then(Value::as_f64) {
                            n.font_size = (f as f32).clamp(8.0, 80.0);
                        }
                        Some(())
                    })
                    .ok_or("no such node")?;
            }
            "add_link" => {
                let (f, t) = (id(a, "from")?, id(a, "to")?);
                let new = self
                    .doc
                    .edit(|m| m.add_link(f, t))
                    .ok_or("cannot link these nodes (same node, or already linked that way)")?;
                let mut st = self.state();
                st["new_link"] = json!(new);
                return Ok(st);
            }
            "delete_link" => {
                let i = id(a, "id")?;
                self.doc
                    .edit(|m| m.delete_link(i).then_some(()))
                    .ok_or("no such link")?;
            }
            "set_link" => {
                let i = id(a, "id")?;
                let style: Option<LinkStyle> = match a.get("style") {
                    Some(v) => {
                        Some(serde_json::from_value(v.clone()).map_err(|_| "unknown link style")?)
                    }
                    None => None,
                };
                let arrow: Option<LinkArrow> = match a.get("arrow") {
                    Some(v) => {
                        Some(serde_json::from_value(v.clone()).map_err(|_| "unknown arrow")?)
                    }
                    None => None,
                };
                let color = match a.get("color").and_then(Value::as_str) {
                    Some(c) if is_hex(c) => Some(c.to_string()),
                    Some(_) => return Err("colour must look like #rrggbb".into()),
                    None => None,
                };
                let bend = a.get("bend").and_then(Value::as_f64).map(|b| b as f32);
                let ends = match (a.get("from"), a.get("to")) {
                    (None, None) => None,
                    _ => Some((id(a, "from")?, id(a, "to")?)),
                };
                if let Some((f, t)) = ends {
                    self.doc
                        .edit(|m| m.reconnect_link(i, f, t).then_some(()))
                        .ok_or(
                            "cannot move the link there (same node, or already linked that way)",
                        )?;
                }
                if let Some(b) = bend {
                    self.doc
                        .edit(|m| m.set_link_bend(i, b).then_some(()))
                        .ok_or("no such link")?;
                }
                if style.is_none() && arrow.is_none() && color.is_none() {
                    return Ok(self.state());
                }
                self.doc
                    .edit(|m| {
                        let l = m.links.iter_mut().find(|l| l.id == i)?;
                        if let Some(s) = style {
                            l.style = s;
                        }
                        if let Some(r) = arrow {
                            l.arrow = r;
                        }
                        if let Some(c) = color {
                            l.color = Some(c);
                        }
                        Some(())
                    })
                    .ok_or("no such link")?;
            }
            "add_image" => {
                // mode "node": new child of `parent` showing the picture.
                // mode "attach": give existing node `id` the picture.
                // mode "free": place it on the canvas at (x, y).
                let mime = text(a, "mime")?.to_string();
                let data = text(a, "data")?.to_string();
                if data.len() > MAX_IMAGE_B64 {
                    return Err("image is too large (limit about 9 MB)".into());
                }
                if !is_base64(&data) {
                    return Err("image data is not valid base64".into());
                }
                let pw = a.get("pixel_width").and_then(Value::as_u64).unwrap_or(0) as u32;
                let ph = a.get("pixel_height").and_then(Value::as_u64).unwrap_or(0) as u32;
                let width = num(a, "width");
                let mode = text(a, "mode")?.to_string();
                let (px, py) = (num(a, "x").unwrap_or(0.0), num(a, "y").unwrap_or(0.0));
                let target = a
                    .get("id")
                    .or_else(|| a.get("parent"))
                    .and_then(Value::as_u64);
                let new = self
                    .doc
                    .edit(|m| {
                        let asset = m.add_asset(&mime, &data, pw, ph)?;
                        match mode.as_str() {
                            "node" => m
                                .add_image_child(target.unwrap_or(m.root), asset, width)
                                .map(|n| ("new_id", n)),
                            "attach" => m
                                .set_node_image(target?, asset, width)
                                .then(|| ("new_id", target.unwrap())),
                            "free" => m
                                .add_free_image(asset, px, py, width)
                                .map(|f| ("new_free", f)),
                            _ => None,
                        }
                    })
                    .ok_or("cannot add this image (unknown format, empty, or no such node)")?;
                let mut st = self.state();
                st[new.0] = json!(new.1);
                return Ok(st);
            }
            "remove_node_image" => {
                let i = id(a, "id")?;
                self.doc
                    .edit(|m| m.remove_node_image(i).then_some(()))
                    .ok_or("this node has no image")?;
            }
            "set_image_width" => {
                let i = id(a, "id")?;
                let w = num(a, "width").ok_or("missing 'width'")?;
                let centre = num(a, "x").zip(num(a, "y"));
                let free = match a.get("kind").and_then(Value::as_str) {
                    Some("free") => true,
                    Some("node") => false,
                    _ => return Err("kind must be 'node' or 'free'".into()),
                };
                self.doc
                    .edit(|m| {
                        if free {
                            m.set_free_image_width(i, w, centre)
                        } else {
                            m.set_node_image_width(i, w)
                        }
                        .then_some(())
                    })
                    .ok_or("no such image")?;
            }
            "move_free_image" => {
                let i = id(a, "id")?;
                let (dx, dy) = (num(a, "dx").unwrap_or(0.0), num(a, "dy").unwrap_or(0.0));
                self.doc
                    .edit(|m| m.move_free_image(i, dx, dy).then_some(()))
                    .ok_or("no such image")?;
            }
            "delete_free_image" => {
                let i = id(a, "id")?;
                self.doc
                    .edit(|m| m.delete_free_image(i).then_some(()))
                    .ok_or("no such image")?;
            }
            "get_asset" => {
                let i = id(a, "id")?;
                let asset = self.doc.map.assets.get(&i).ok_or("no such asset")?;
                return Ok(json!({ "mime": asset.mime, "data": &*asset.data }));
            }
            "toggle_collapse" => {
                let i = id(a, "id")?;
                self.doc
                    .edit(|m| {
                        let n = m.nodes.get_mut(&i)?;
                        if n.children.is_empty() {
                            return None;
                        }
                        n.collapsed = !n.collapsed;
                        Some(())
                    })
                    .ok_or("nothing to collapse")?;
            }
            "clean_up" => {
                // Drop all manual offsets and return to the automatic layout.
                self.doc.edit(|m| {
                    let any = m.nodes.values().any(|n| n.dx != 0.0 || n.dy != 0.0);
                    for n in m.nodes.values_mut() {
                        n.dx = 0.0;
                        n.dy = 0.0;
                    }
                    any.then_some(())
                });
            }
            "set_dark" => {
                let on = a.get("on").and_then(Value::as_bool).unwrap_or(false);
                self.doc.edit(|m| {
                    m.dark_mode = on;
                    Some(())
                });
            }
            "undo" => {
                self.doc.undo();
            }
            "redo" => {
                self.doc.redo();
            }
            "new" => {
                *self = Session::new(text(a, "title").unwrap_or("Central idea"));
            }
            "load_json" => {
                let map = mindmap_io::from_json(text(a, "json")?)?;
                self.doc = Document::new(map);
            }
            "import_opml" => {
                let map = mindmap_io::from_opml(text(a, "xml")?)?;
                self.doc = Document::new(map);
            }
            "save_json" => return Ok(json!({ "data": mindmap_io::to_json(&self.doc.map) })),
            "export" => {
                let data = match text(a, "format")? {
                    "svg" => mindmap_io::to_svg(&self.doc.map),
                    "markdown" => mindmap_io::to_markdown(&self.doc.map),
                    "opml" => mindmap_io::to_opml(&self.doc.map),
                    other => return Err(format!("unknown export format '{other}'")),
                };
                return Ok(json!({ "data": data }));
            }
            other => return Err(format!("unknown command '{other}'")),
        }
        Ok(self.state())
    }
}
