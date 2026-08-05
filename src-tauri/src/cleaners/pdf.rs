use std::path::Path;

use lopdf::Object;

use crate::error::{AppError, AppResult};

pub fn clean(input: &Path, output: &Path) -> AppResult<Vec<String>> {
    let mut doc = lopdf::Document::load(input)?;
    if doc.is_encrypted() {
        return Err(AppError::msg("加密的 PDF 暂不支持清理"));
    }

    // Clear all Info dictionary entries.
    let empty_info = Object::Dictionary(lopdf::Dictionary::new());
    doc.trailer.set(b"Info", empty_info);

    // Remove XMP metadata from the document catalog.
    if let Ok(Object::Reference(id)) = doc.trailer.get(b"Root") {
        if let Some(obj) = doc.objects.get_mut(&id) {
            if let Object::Dictionary(dict) = obj {
                dict.remove(b"Metadata");
            }
        }
    }

    doc.save(output)?;
    Ok(vec!["已清空 PDF 文档信息与 XMP 元数据".to_string()])
}
