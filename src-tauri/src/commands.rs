use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Emitter};

use crate::cleaners;
use crate::types::{CleanFileResult, FileEntry, MetadataInfo, ProgressEvent, Settings};

#[tauri::command]
pub fn add_paths(paths: Vec<String>) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    for p in paths {
        collect_entries(PathBuf::from(p), &mut entries, &mut seen);
    }
    entries
}

fn collect_entries(path: PathBuf, out: &mut Vec<FileEntry>, seen: &mut HashSet<String>) {
    if path.is_dir() {
        let Ok(read_dir) = std::fs::read_dir(&path) else {
            return;
        };
        let mut children: Vec<PathBuf> = read_dir
            .flatten()
            .map(|e| e.path())
            .filter(|p| !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')))
            .collect();
        children.sort();
        for child in children {
            collect_entries(child, out, seen);
        }
    } else if path.is_file() {
        let key = path.to_string_lossy().to_string();
        if seen.insert(key.clone()) {
            out.push(make_entry(&path));
        }
    }
}

fn make_entry(path: &Path) -> FileEntry {
    let metadata = std::fs::metadata(path).ok();
    FileEntry {
        path: path.to_string_lossy().to_string(),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
        file_type: cleaners::file_type_name(path),
        modified: metadata
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0),
        supported: cleaners::is_supported(path),
    }
}

#[tauri::command]
pub fn read_metadata(path: String) -> Result<MetadataInfo, String> {
    crate::metadata::read_metadata(Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clean_files(app: AppHandle, paths: Vec<String>) -> Result<Vec<CleanFileResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let total = paths.len();
        let mut results = Vec::with_capacity(total);
        for (i, p) in paths.iter().enumerate() {
            let name = Path::new(p)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| p.clone());
            let _ = app.emit(
                "clean-progress",
                ProgressEvent {
                    current: i + 1,
                    total,
                    name,
                },
            );
            results.push(clean_one_file(p));
        }
        results
    })
    .await
    .map_err(|e| e.to_string())
}

pub fn clean_one_file(path: &str) -> CleanFileResult {
    let original_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    match cleaners::clean_in_place(Path::new(path)) {
        Ok(output) => CleanFileResult {
            path: path.to_string(),
            success: true,
            error: None,
            warnings: output.warnings,
            original_size,
            cleaned_size: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        },
        Err(e) => CleanFileResult {
            path: path.to_string(),
            success: false,
            error: Some(e.to_string()),
            warnings: Vec::new(),
            original_size,
            cleaned_size: 0,
        },
    }
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<Settings, String> {
    crate::settings::load(&app)
}

#[tauri::command]
pub fn set_context_menu_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    crate::settings::set_context_menu_enabled(&app, enabled)
}

#[tauri::command]
pub fn is_context_menu_registered() -> bool {
    crate::context_menu::is_registered()
}

#[tauri::command]
pub fn open_in_explorer(app: AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| e.to_string())
}
