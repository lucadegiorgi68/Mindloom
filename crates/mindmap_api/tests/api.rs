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
    let a = s.invoke("add_child", &json!({"parent": 1, "text": "A"})).unwrap()["new_id"].as_u64().unwrap_or(2);
    let b = s.invoke("add_child", &json!({"parent": 1, "text": "B"})).unwrap()["new_id"].as_u64().unwrap_or(3);
    let c = s.invoke("add_child", &json!({"parent": 1, "text": "C"})).unwrap()["new_id"].as_u64().unwrap_or(4);
    let l = s.invoke("add_link", &json!({"from": a, "to": b})).unwrap()["new_link"].as_u64().unwrap();
    let st = s.invoke("set_link", &json!({"id": l, "bend": -0.6})).unwrap();
    assert!((st["scene"]["links"][0]["bend"].as_f64().unwrap() + 0.6).abs() < 1e-6);
    let st = s.invoke("set_link", &json!({"id": l, "from": a, "to": c})).unwrap();
    assert_eq!(st["scene"]["links"][0]["to"].as_u64().unwrap(), c);
    assert!(s.invoke("set_link", &json!({"id": l, "from": a, "to": a})).is_err());
    assert!(s.invoke("set_link", &json!({"id": l, "from": a})).is_err(), "both ends are required");
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
