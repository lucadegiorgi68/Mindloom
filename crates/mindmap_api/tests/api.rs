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
