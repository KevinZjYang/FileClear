mod cleaners;
mod commands;
mod context_menu;
mod error;
mod metadata;
mod quick_clean;
mod settings;
mod types;

use tauri::Manager;

/// Parse process arguments: detect `--quick-clean` and collect the file paths
/// that follow it (Explorer passes selected files via `%*`).
pub fn parse_args(args: &[String]) -> (bool, Vec<String>) {
    let mut quick_clean = false;
    let mut paths = Vec::new();
    let mut after_flag = false;
    for arg in args.iter().skip(1) {
        if arg == "--quick-clean" {
            quick_clean = true;
            after_flag = true;
        } else if after_flag {
            paths.push(arg.clone());
        }
    }
    (quick_clean, paths)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(quick_clean: bool, paths: Vec<String>) {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let (_, paths) = parse_args(&args);
            if !paths.is_empty() {
                tauri::async_runtime::spawn(quick_clean::run(app.clone(), paths));
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(move |app| {
            let _ = settings::init(app.handle());

            if quick_clean {
                // Headless quick-clean: no window, then exit.
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    quick_clean::run(handle.clone(), paths.clone()).await;
                    handle.exit(0);
                });
            } else if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::add_paths,
            commands::read_metadata,
            commands::clean_files,
            commands::get_settings,
            commands::set_context_menu_enabled,
            commands::is_context_menu_registered,
            commands::open_in_explorer,
        ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::parse_args;

    #[test]
    fn parse_args_detects_quick_clean() {
        let (flag, paths) = parse_args(&[
            "fileclear.exe".to_string(),
            "--quick-clean".to_string(),
            r"C:\a.jpg".to_string(),
            r"C:\b.docx".to_string(),
        ]);
        assert!(flag);
        assert_eq!(paths, vec![r"C:\a.jpg".to_string(), r"C:\b.docx".to_string()]);
    }

    #[test]
    fn parse_args_ignores_normal_launch() {
        let (flag, paths) = parse_args(&["fileclear.exe".to_string()]);
        assert!(!flag);
        assert!(paths.is_empty());
    }

    #[test]
    fn parse_args_collects_only_paths_after_flag() {
        let (flag, paths) = parse_args(&[
            "fileclear.exe".to_string(),
            "unrelated.bin".to_string(),
            "--quick-clean".to_string(),
            r"D:\x.png".to_string(),
        ]);
        assert!(flag);
        assert_eq!(paths, vec![r"D:\x.png".to_string()]);
    }
}