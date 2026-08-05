use std::path::Path;

use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::types::ProgressEvent;

/// Process a batch of files from the Explorer context menu ("快捷清理").
/// Runs headlessly: cleans in place, emits progress + finished events, and
/// shows a system notification with the result summary.
pub async fn run(app: AppHandle, paths: Vec<String>) {
    let handle = app.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || {
        let total = paths.len();
        let mut ok = 0usize;
        let mut fail = 0usize;
        let mut first_error = String::new();

        for (i, p) in paths.iter().enumerate() {
            let name = Path::new(p)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| p.clone());
            let _ = handle.emit(
                "clean-progress",
                ProgressEvent {
                    current: i + 1,
                    total,
                    name,
                },
            );
            let result = crate::commands::clean_one_file(p);
            if result.success {
                ok += 1;
            } else {
                fail += 1;
                if first_error.is_empty() {
                    first_error = result.error.unwrap_or_default();
                }
            }
        }

        let body = if fail == 0 {
            format!("成功清理 {ok} 个文件")
        } else if ok == 0 {
            format!("清理失败 {fail} 个文件")
        } else {
            format!("成功 {ok} 个，失败 {fail} 个")
        };

        let _ = handle.emit(
            "quick-clean-finished",
            serde_json::json!({
                "success": ok,
                "failed": fail,
                "firstError": first_error,
            }),
        );

        let _ = handle
            .notification()
            .builder()
            .title("FileClear 快捷清理完成")
            .body(body)
            .show();
    })
    .await;
}
