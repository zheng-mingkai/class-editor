mod classfile;
mod jar;
mod decompiler;
mod jdk;
mod commands;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_file,
            commands::open_class_in_jar,
            commands::save_class_file,
            commands::save_class_in_jar,
            commands::detect_jdk,
            commands::set_jdk_path,
            commands::decompile_class,
            commands::get_bytecode,
            commands::locate_string_lines,
            commands::search_strings,
            commands::batch_save,
            commands::open_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
