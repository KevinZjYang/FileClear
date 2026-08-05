mod cleaners;
mod commands;
mod context_menu;
mod error;
mod metadata;
mod quick_clean;
mod settings;
mod types;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::Manager;

/// Parse process arguments: detect `--quick-clean` and collect the file paths
/// that follow it (Explorer passes the selected file via `%1`).
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
    // 记录单实例转发来的待处理清理任务数，headless 退出前会等待它们完成。
    let pending = Arc::new(AtomicUsize::new(0));
    // The first instance decides the feedback mode (headless vs. window).
    // Forwarded quick-clean tasks must use the same mode, otherwise a
    // multi-file selection could show a message box for one file and a
    // system notification for the others.
    let headless = Arc::new(AtomicBool::new(quick_clean));

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init({
            let pending = pending.clone();
            let headless = headless.clone();
            move |app, args, _cwd| {
                let (_, paths) = parse_args(&args);
                if !paths.is_empty() {
                    pending.fetch_add(1, Ordering::SeqCst);
                    let pending = pending.clone();
                    let app = app.clone();
                    let is_headless = headless.load(Ordering::SeqCst);
                    tauri::async_runtime::spawn(async move {
                        quick_clean::run(app, paths, is_headless).await;
                        pending.fetch_sub(1, Ordering::SeqCst);
                    });
                }
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
                    quick_clean::run(handle.clone(), paths.clone(), true).await;
                    // 多选时 Explorer 会逐文件启动进程，其余实例把文件转发给本实例；
                    // 等转发来的清理任务完成后（最多 30 秒）再退出。
                    let handle = handle.clone();
                    let pending = pending.clone();
                    let _ = tauri::async_runtime::spawn_blocking(move || {
                        for _ in 0..300 {
                            if pending.load(Ordering::SeqCst) == 0 {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        handle.exit(0);
                    });
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