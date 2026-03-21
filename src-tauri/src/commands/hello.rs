#[tauri::command]
pub fn hello() -> serde_json::Value {
    serde_json::json!({ "message": "Hello from SamWise" })
}
