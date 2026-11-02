use mindmap_api::Session;
use serde_json::json;

#[test]
fn full_editing_flow_with_undo() {
    let mut s = Session::new("Root");
    let r = s
        .invoke("add_child", &json!({"parent": 1, "text": "A"}))
        .unwrap();
    let a = r["new_id"].as_u64().unwrap();
    s.invoke("add_sibling", &json!({"id": a, "text": "B"}))
        .unwrap();
    assert_eq!(
        s.invoke("state", &json!({})).unwrap()["scene"]["nodes"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    s.invoke("set_text", &json!({"id": a, "text": "Alpha"}))
        .unwrap();
    s.invoke("undo", &json!({})).unwrap();
    let st = s.invoke("state", &json!({})).unwrap();
    assert!(st["scene"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["text"] == "A"));
    assert_eq!(st["can_redo"], true);
}

#[test]
fn errors_are_reported_not_panics() {
    let mut s = Session::new("Root");
    assert!(s.invoke("delete", &json!({"id": 1})).is_err());
    assert!(s.invoke("add_sibling", &json!({"id": 1})).is_err());
    assert!(s
        .invoke("set_color", &json!({"id": 1, "color": "red"}))
        .is_err());
    assert!(s
        .invoke("set_shape", &json!({"id": 1, "shape": "Nope"}))
        .is_err());
    assert!(s.invoke("export", &json!({"format": "exe"})).is_err());
    assert!(s.invoke("bogus", &json!({})).is_err());
    assert!(s.invoke("add_child", &json!({})).is_err());
}

#[test]
fn save_and_load_roundtrip() {
    let mut s = Session::new("Root");
    s.invoke("add_child", &json!({"parent": 1, "text": "A"}))
        .unwrap();
    let saved = s.invoke("save_json", &json!({})).unwrap()["data"]
        .as_str()
        .unwrap()
        .to_string();
    let mut t = Session::new("Other");
    let st = t.invoke("load_json", &json!({"json": saved})).unwrap();
    assert_eq!(st["title"], "Root");
    assert_eq!(st["can_undo"], false);
    assert!(t.invoke("load_json", &json!({"json": "garbage"})).is_err());
}

#[test]
fn clean_up_resets_manual_moves() {
    let mut s = Session::new("Root");
    let a = s
        .invoke("add_child", &json!({"parent": 1, "text": "A"}))
        .unwrap()["new_id"]
        .as_u64()
        .unwrap();
    let before = s.invoke("state", &json!({})).unwrap()["scene"]["nodes"][1]["rect"]["cy"]
        .as_f64()
        .unwrap();
    s.invoke("move_node", &json!({"id": a, "dx": 40, "dy": 90}))
        .unwrap();
    s.invoke("clean_up", &json!({})).unwrap();
    let after = s.invoke("state", &json!({})).unwrap()["scene"]["nodes"][1]["rect"]["cy"]
        .as_f64()
        .unwrap();
    assert_eq!(before, after);
}

#[test]
fn font_changes_are_validated_and_listed() {
    let mut s = Session::new("Root");
    let st = s.invoke("state", &json!({})).unwrap();
    assert_eq!(st["fonts"].as_array().unwrap().len(), 7);
    assert_eq!(st["fonts"][0]["name"], "Open Sans");
    let st = s
        .invoke("set_format", &json!({"id": 1, "font": "Caveat"}))
        .unwrap();
    assert_eq!(st["scene"]["nodes"][0]["font"], "Caveat");
    assert_eq!(st["nodes"][0]["font"], "Caveat");
    assert!(s
        .invoke("set_format", &json!({"id": 1, "font": "Wingdings"}))
        .is_err());
    // A rejected change must not touch the document or the history.
    assert_eq!(
        s.invoke("state", &json!({})).unwrap()["scene"]["nodes"][0]["font"],
        "Caveat"
    );
    s.invoke("undo", &json!({})).unwrap();
    assert_eq!(
        s.invoke("state", &json!({})).unwrap()["scene"]["nodes"][0]["font"],
        "Open Sans"
    );
}

#[test]
fn link_commands_create_edit_delete_and_undo() {
    let mut s = Session::new("Root");
    let a = s
        .invoke("add_child", &json!({"parent": 1, "text": "A"}))
        .unwrap()["new_id"]
        .as_u64()
        .unwrap();
    let b = s
        .invoke("add_child", &json!({"parent": 1, "text": "B"}))
        .unwrap()["new_id"]
        .as_u64()
        .unwrap();
    let st = s.invoke("add_link", &json!({"from": a, "to": b})).unwrap();
    let lid = st["new_link"].as_u64().unwrap();
    assert_eq!(st["scene"]["links"].as_array().unwrap().len(), 1);
    assert_eq!(st["scene"]["links"][0]["arrow"], "End");
    let st = s
        .invoke(
            "set_link",
            &json!({"id": lid, "style": "Dashed", "arrow": "Both", "color": "#00aa55"}),
        )
        .unwrap();
    let l = &st["scene"]["links"][0];
    assert_eq!(
        (l["style"].clone(), l["arrow"].clone(), l["color"].clone()),
        (json!("Dashed"), json!("Both"), json!("#00aa55"))
    );
    assert_eq!(l["arrow_start"].as_array().unwrap().len(), 4);
    // Partial updates leave the other fields alone.
    let st = s
        .invoke("set_link", &json!({"id": lid, "arrow": "None"}))
        .unwrap();
    assert_eq!(st["scene"]["links"][0]["style"], "Dashed");
    s.invoke("undo", &json!({})).unwrap();
    assert_eq!(
        s.invoke("state", &json!({})).unwrap()["scene"]["links"][0]["arrow"],
        "Both"
    );
    let st = s.invoke("delete_link", &json!({"id": lid})).unwrap();
    assert!(st["scene"]["links"].as_array().unwrap().is_empty());
    s.invoke("undo", &json!({})).unwrap();
    assert_eq!(
        s.invoke("state", &json!({})).unwrap()["scene"]["links"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn bad_link_commands_are_errors_and_leave_no_history() {
    let mut s = Session::new("Root");
    let a = s
        .invoke("add_child", &json!({"parent": 1, "text": "A"}))
        .unwrap()["new_id"]
        .as_u64()
        .unwrap();
    assert!(s.invoke("add_link", &json!({"from": a, "to": a})).is_err());
    assert!(s.invoke("add_link", &json!({"from": a, "to": 99})).is_err());
    assert!(s.invoke("add_link", &json!({"from": a})).is_err());
    let lid = s.invoke("add_link", &json!({"from": a, "to": 1})).unwrap()["new_link"]
        .as_u64()
        .unwrap();
    assert!(s.invoke("add_link", &json!({"from": a, "to": 1})).is_err());
    assert!(s
        .invoke("set_link", &json!({"id": lid, "style": "Wavy"}))
        .is_err());
    assert!(s
        .invoke("set_link", &json!({"id": lid, "arrow": "Sideways"}))
        .is_err());
    assert!(s
        .invoke("set_link", &json!({"id": lid, "color": "blue"}))
        .is_err());
    assert!(s
        .invoke("set_link", &json!({"id": 4242, "style": "Solid"}))
        .is_err());
    assert!(s.invoke("delete_link", &json!({"id": 4242})).is_err());
    // Still just the one link, still undoable back to zero in one step.
    s.invoke("undo", &json!({})).unwrap();
    assert!(s.invoke("state", &json!({})).unwrap()["scene"]["links"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn set_link_bends_and_reconnects() {
    let mut s = Session::new("Root");
    let a = s
        .invoke("add_child", &json!({"parent": 1, "text": "A"}))
        .unwrap()["new_id"]
        .as_u64()
        .unwrap_or(2);
    let b = s
        .invoke("add_child", &json!({"parent": 1, "text": "B"}))
        .unwrap()["new_id"]
        .as_u64()
        .unwrap_or(3);
    let c = s
        .invoke("add_child", &json!({"parent": 1, "text": "C"}))
        .unwrap()["new_id"]
        .as_u64()
        .unwrap_or(4);
    let l = s.invoke("add_link", &json!({"from": a, "to": b})).unwrap()["new_link"]
        .as_u64()
        .unwrap();
    let st = s
        .invoke("set_link", &json!({"id": l, "bend": -0.6}))
        .unwrap();
    assert!((st["scene"]["links"][0]["bend"].as_f64().unwrap() + 0.6).abs() < 1e-6);
    let st = s
        .invoke("set_link", &json!({"id": l, "from": a, "to": c}))
        .unwrap();
    assert_eq!(st["scene"]["links"][0]["to"].as_u64().unwrap(), c);
    assert!(s
        .invoke("set_link", &json!({"id": l, "from": a, "to": a}))
        .is_err());
    assert!(
        s.invoke("set_link", &json!({"id": l, "from": a})).is_err(),
        "both ends are required"
    );
    s.invoke("undo", &json!({})).unwrap();
    let st = s.invoke("state", &json!({})).unwrap();
    assert_eq!(st["scene"]["links"][0]["to"].as_u64().unwrap(), b);
}

#[test]
fn add_central_idea_command_with_undo_and_delete() {
    let mut s = Session::new("Root");
    let r = s.invoke("add_central_idea", &json!({})).unwrap();
    let c = r["new_id"].as_u64().unwrap();
    assert_eq!(r["roots"], json!([1, c]));
    let n = r["scene"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == c)
        .unwrap()
        .clone();
    assert_eq!(n["text"], "Central idea");
    assert_eq!(n["central"], true);
    // Children can hang off it.
    let k = s
        .invoke("add_child", &json!({"parent": c, "text": "A"}))
        .unwrap();
    assert_eq!(k["scene"]["nodes"].as_array().unwrap().len(), 3);
    // The title still comes from the first central idea.
    assert_eq!(k["title"], "Root");
    // The first one is protected, the second one is not.
    assert!(s.invoke("delete", &json!({"id": 1})).is_err());
    let d = s.invoke("delete", &json!({"id": c})).unwrap();
    assert_eq!(d["roots"], json!([1]));
    let u = s.invoke("undo", &json!({})).unwrap();
    assert_eq!(u["roots"], json!([1, c]));
    // It survives a save and open round trip.
    let saved = s.invoke("save_json", &json!({})).unwrap();
    let mut t = Session::new("Other");
    let l = t
        .invoke("load_json", &json!({"json": saved["data"]}))
        .unwrap();
    assert_eq!(l["roots"], json!([1, c]));
}

// ---------- Images ----------

fn png(mode: &str, extra: serde_json::Value) -> serde_json::Value {
    let mut v = json!({
        "mode": mode, "mime": "image/png", "data": "iVBORw0KGgo=",
        "pixel_width": 200, "pixel_height": 100,
    });
    for (k, x) in extra.as_object().unwrap() {
        v[k] = x.clone();
    }
    v
}

#[test]
fn image_as_a_node_then_resize_then_undo() {
    let mut s = Session::new("Root");
    let st = s
        .invoke(
            "add_image",
            &png("node", json!({"parent": 1, "width": 160})),
        )
        .unwrap();
    let id = st["new_id"].as_u64().unwrap();
    let node = st["scene"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap()
        .clone();
    assert_eq!(node["image"]["rect"]["w"], 160.0);
    assert_eq!(node["image"]["rect"]["h"], 80.0);

    let st = s
        .invoke(
            "set_image_width",
            &json!({"kind": "node", "id": id, "width": 320}),
        )
        .unwrap();
    let meta = st["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap()
        .clone();
    assert_eq!(meta["image_width"], 320.0);

    s.invoke("undo", &json!({})).unwrap();
    let st = s.invoke("state", &json!({})).unwrap();
    let meta = st["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap()
        .clone();
    assert_eq!(meta["image_width"], 160.0, "a resize is one undo step");
}

#[test]
fn free_image_moves_resizes_and_deletes() {
    let mut s = Session::new("Root");
    let st = s
        .invoke("add_image", &png("free", json!({"x": 400, "y": 50})))
        .unwrap();
    let f = st["new_free"].as_u64().unwrap();
    assert_eq!(st["scene"]["images"][0]["rect"]["cx"], 400.0);
    let st = s
        .invoke("move_free_image", &json!({"id": f, "dx": 5, "dy": -5}))
        .unwrap();
    assert_eq!(st["scene"]["images"][0]["rect"]["cx"], 405.0);
    let st = s
        .invoke(
            "set_image_width",
            &json!({"kind": "free", "id": f, "width": 100, "x": 0, "y": 0}),
        )
        .unwrap();
    assert_eq!(st["scene"]["images"][0]["rect"]["w"], 100.0);
    assert_eq!(st["scene"]["images"][0]["rect"]["cx"], 0.0);
    let st = s.invoke("delete_free_image", &json!({"id": f})).unwrap();
    assert!(st["scene"]["images"].as_array().unwrap().is_empty());
    let st = s.invoke("undo", &json!({})).unwrap();
    assert_eq!(
        st["scene"]["images"].as_array().unwrap().len(),
        1,
        "undo brings it back"
    );
}

#[test]
fn attach_replace_and_remove_a_node_picture() {
    let mut s = Session::new("Root");
    let a = s
        .invoke("add_child", &json!({"parent": 1, "text": "A"}))
        .unwrap()["new_id"]
        .as_u64()
        .unwrap();
    s.invoke("add_image", &png("attach", json!({"id": a})))
        .unwrap();
    let st = s.invoke("state", &json!({})).unwrap();
    let n = st["scene"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == a)
        .unwrap()
        .clone();
    assert!(n["image"].is_object());
    assert_eq!(n["text"], "A");
    let st = s.invoke("remove_node_image", &json!({"id": a})).unwrap();
    let n = st["scene"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == a)
        .unwrap()
        .clone();
    assert!(n["image"].is_null());
    assert!(s.invoke("remove_node_image", &json!({"id": a})).is_err());
}

#[test]
fn bad_images_are_refused_and_change_nothing() {
    let mut s = Session::new("Root");
    let before = s.invoke("state", &json!({})).unwrap();
    assert!(s
        .invoke("add_image", &png("free", json!({"mime": "text/html"})))
        .is_err());
    assert!(s
        .invoke("add_image", &png("free", json!({"data": "not base64!"})))
        .is_err());
    assert!(s
        .invoke("add_image", &png("free", json!({"pixel_width": 0})))
        .is_err());
    assert!(s.invoke("add_image", &png("sideways", json!({}))).is_err());
    assert!(s
        .invoke("add_image", &png("attach", json!({"id": 999})))
        .is_err());
    assert!(s
        .invoke(
            "set_image_width",
            &json!({"kind": "bogus", "id": 1, "width": 10})
        )
        .is_err());
    let after = s.invoke("state", &json!({})).unwrap();
    assert_eq!(before["can_undo"], after["can_undo"]);
    assert!(after["scene"]["images"].as_array().unwrap().is_empty());
}

#[test]
fn asset_bytes_come_from_get_asset_not_from_the_scene() {
    let mut s = Session::new("Root");
    let st = s.invoke("add_image", &png("free", json!({}))).unwrap();
    assert!(
        !st.to_string().contains("iVBORw0KGgo="),
        "scene stays light"
    );
    let asset = st["scene"]["images"][0]["asset"].as_u64().unwrap();
    let a = s.invoke("get_asset", &json!({"id": asset})).unwrap();
    assert_eq!(a["mime"], "image/png");
    assert_eq!(a["data"], "iVBORw0KGgo=");
    assert!(s.invoke("get_asset", &json!({"id": 999})).is_err());
}

#[test]
fn pictures_survive_save_and_open() {
    let mut s = Session::new("Root");
    s.invoke("add_image", &png("node", json!({"parent": 1})))
        .unwrap();
    s.invoke("add_image", &png("free", json!({"x": 10, "y": 20})))
        .unwrap();
    let saved = s.invoke("save_json", &json!({})).unwrap()["data"]
        .as_str()
        .unwrap()
        .to_string();
    let mut t = Session::new("Other");
    let st = t.invoke("load_json", &json!({"json": saved})).unwrap();
    assert_eq!(st["scene"]["images"].as_array().unwrap().len(), 1);
    assert!(st["scene"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["image"].is_object()));
    let asset = st["scene"]["images"][0]["asset"].as_u64().unwrap();
    assert_eq!(
        t.invoke("get_asset", &json!({"id": asset})).unwrap()["data"],
        "iVBORw0KGgo="
    );
}

#[test]
fn picture_fingerprint_changes_when_an_id_is_reused_after_undo() {
    let mut s = Session::new("Root");
    let one = s.invoke("add_image", &png("free", json!({}))).unwrap();
    let key1 = one["scene"]["images"][0]["key"].as_u64().unwrap();
    s.invoke("undo", &json!({})).unwrap();
    let two = s
        .invoke(
            "add_image",
            &png("free", json!({"data": "R0lGODlhAQABAAAAACw="})),
        )
        .unwrap();
    let img = &two["scene"]["images"][0];
    assert_eq!(
        img["asset"], one["scene"]["images"][0]["asset"],
        "id is reused"
    );
    assert_ne!(
        img["key"].as_u64().unwrap(),
        key1,
        "but the fingerprint tells them apart"
    );
}
