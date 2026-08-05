use std::path::Path;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::types::ProgressEvent;

/// Multiple headless quick-clean tasks (Explorer launches one process per
/// selected file and the extras are forwarded here) can finish around the same
/// time; serialize the native message boxes so they do not overlap on screen.
static MESSAGE_BOX_LOCK: Mutex<()> = Mutex::new(());

/// Process a batch of files from the Explorer context menu ("快捷清理").
/// Cleans in place, emits progress + finished events, and reports the result:
/// - headless（主窗口未打开）: 弹原生消息框，确保用户一定看到处理结果；
/// - 窗口模式：前端 ElMessage + 系统通知。
pub async fn run(app: AppHandle, paths: Vec<String>, headless: bool) {
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

        // headless 时弹原生消息框已足够（未安装/开发环境系统通知可能不显示），
        // 避免通知与消息框重复；窗口模式保留系统通知 + 前端提示。
        if !headless {
            let _ = handle
                .notification()
                .builder()
                .title("FileClear 快捷清理完成")
                .body(body.clone())
                .show();
        }

        if headless {
            // 系统通知在未安装/开发环境下可能不显示，原生消息框兜底；
            // 消息框关闭前进程保持存活，也避免 toast 因立即退出而丢失。
            let description = if fail > 0 && !first_error.is_empty() {
                format!("{body}\n\n{first_error}")
            } else {
                body
            };
            let level = if fail == 0 {
                rfd::MessageLevel::Info
            } else if ok == 0 {
                rfd::MessageLevel::Error
            } else {
                rfd::MessageLevel::Warning
            };
            let _box_guard = MESSAGE_BOX_LOCK.lock().unwrap();
            rfd::MessageDialog::new()
                .set_title("FileClear 快捷清理")
                .set_description(description)
                .set_level(level)
                .show();
        }
    })
    .await;
}