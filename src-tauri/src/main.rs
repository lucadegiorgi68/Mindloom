// Hides the extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use mindmap_api::Session;
use std::sync::Mutex;
use tauri::State;

struct App(Mutex<Session>);

/// Single entry point for every editing command. All behaviour lives in
/// `mindmap_api`, so this shell stays a thin wrapper.
#[tauri::command]
fn run_command(
    state: State<'_, App>,
    name: String,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    state
        .0
        .lock()
        .map_err(|_| "internal state is unavailable".to_string())?
        .invoke(&name, &args)
}

#[tauri::command]
fn write_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents).map_err(|e| format!("could not write {path}: {e}"))
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("could not read {path}: {e}"))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(App(Mutex::new(Session::new("Central idea"))))
        .invoke_handler(tauri::generate_handler![run_command, write_file, read_file])
        .run(tauri::generate_context!())
        .expect("error while running Mindloom");
}
