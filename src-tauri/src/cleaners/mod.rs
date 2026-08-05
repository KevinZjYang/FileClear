pub mod image;
pub mod legacy;
pub mod ooxml;
pub mod pdf;
pub mod propset;

use std::path::Path;

use crate::error::{AppError, AppResult};

pub struct CleanOutput {
    pub warnings: Vec<String>,
}

const SUPPORTED: &[&str] = &[
    ".jpg", ".jpeg", ".png", ".gif", ".webp", ".tiff", ".tif", ".bmp", ".pdf", ".docx", ".doc",
    ".xlsx", ".xls", ".pptx", ".ppt",
];

pub fn is_supported(path: &Path) -> bool {
    extension(path).is_some()
}


pub fn extension(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let ext = format!(".{ext}");
    SUPPORTED.iter().find(|e| **e == ext).copied()
}

pub fn file_type_name(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "jpg" | "jpeg" => "JPEG 图片".to_string(),
        "png" => "PNG 图片".to_string(),
        "gif" => "GIF 图片".to_string(),
        "webp" => "WebP 图片".to_string(),
        "tiff" | "tif" => "TIFF 图片".to_string(),
        "bmp" => "BMP 图片".to_string(),
        "pdf" => "PDF 文档".to_string(),
        "docx" => "Word 文档".to_string(),
        "doc" => "Word 文档(旧版)".to_string(),
        "xlsx" => "Excel 表格".to_string(),
        "xls" => "Excel 表格(旧版)".to_string(),
        "pptx" => "PowerPoint 演示".to_string(),
        "ppt" => "PowerPoint 演示(旧版)".to_string(),
        _ => "未知类型".to_string(),
    }
}

pub fn clean(input: &Path, output: &Path) -> AppResult<CleanOutput> {
    let Some(ext) = extension(input) else {
        return Err(AppError::msg("不支持的文件类型"));
    };
    let warnings = match ext {
        ".jpg" | ".jpeg" | ".png" | ".gif" | ".webp" | ".tiff" | ".tif" | ".bmp" => {
            image::clean(input, output)?
        }
        ".pdf" => pdf::clean(input, output)?,
        ".docx" | ".xlsx" | ".pptx" => ooxml::clean(input, output)?,
        ".doc" | ".xls" | ".ppt" => legacy::clean(input, output)?,
        _ => return Err(AppError::msg("不支持的文件类型")),
    };
    Ok(CleanOutput { warnings })
}

/// Clean a file in place: write to a temp file in the same directory,
/// then atomically replace the original.
pub fn clean_in_place(path: &Path) -> AppResult<CleanOutput> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut builder = tempfile::Builder::new();
    let prefix = format!(".{stem}.fileclear-");
    builder.prefix(&prefix);
    if !ext.is_empty() {
        builder.suffix(&ext);
    }
    let temp = builder.tempfile_in(dir)?;
    let temp_path = temp.path().to_path_buf();

    let output = clean(path, &temp_path)?;
    temp.persist(path)?;
    Ok(output)
}


#[cfg(test)]
mod tests;
