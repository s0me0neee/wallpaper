use std::path::PathBuf;

#[tauri::command]
fn image() ->  {
    let p = dirs::home_dir().ok_or_else()
}
