use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use quick_xml::events::{BytesText, Event};

use crate::error::{AppError, AppResult};

/// Clean OOXML packages (.docx/.xlsx/.pptx) by clearing docProps metadata XML.
pub fn clean(input: &Path, output: &Path) -> AppResult<Vec<String>> {
    let src = File::open(input)?;
    let mut archive = zip::ZipArchive::new(src)?;
    let out_file = File::create(output)?;
    let mut writer = zip::ZipWriter::new(out_file);

    let mut touched = 0usize;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(entry.compression())
            .last_modified_time(entry.last_modified().unwrap_or_default());

        if entry.is_dir() {
            writer.add_directory(name, options)?;
            continue;
        }

        if name == "docProps/core.xml" || name == "docProps/app.xml" {
            let mut data = Vec::new();
            entry.read_to_end(&mut data)?;
            let cleared = clear_xml_text(&data)?;
            writer.start_file(name, options)?;
            writer.write_all(&cleared)?;
            touched += 1;
        } else {
            writer.start_file(name, options)?;
            std::io::copy(&mut entry, &mut writer)?;
        }
    }

    writer.finish()?;
    if touched == 0 {
        return Err(AppError::msg(
            "未找到 docProps 元数据（文件可能不是有效的 Office 文档）",
        ));
    }
    Ok(vec![format!("已清理 {touched} 个元数据 XML")])
}

/// Rewrite XML keeping the structure/attributes but clearing all text content.
fn clear_xml_text(data: &[u8]) -> AppResult<Vec<u8>> {
    let mut reader = quick_xml::Reader::from_reader(data);
    reader.config_mut().trim_text(false);
    let mut writer = quick_xml::Writer::new(Vec::new());

    loop {
        match reader.read_event()? {
            Event::Eof => break,
            Event::Text(_) | Event::CData(_) => {
                writer.write_event(Event::Text(BytesText::new("")))?;
            }
            ev => {
                writer.write_event(ev)?;
            }
        }
    }
    Ok(writer.into_inner())
}
