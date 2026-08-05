use std::io::Read;
use std::path::Path;

use lopdf::Object;

use crate::cleaners::{self, propset};
use crate::error::{AppError, AppResult};
use crate::types::{MetadataField, MetadataInfo};

pub fn read_metadata(path: &Path) -> AppResult<MetadataInfo> {
    let file_type = cleaners::file_type_name(path);
    let mut info = MetadataInfo {
        path: path.to_string_lossy().to_string(),
        file_type,
        fields: Vec::new(),
        warnings: Vec::new(),
    };

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "tiff" | "tif" | "bmp" => {
            read_image_metadata(path, &mut info)
        }
        "pdf" => read_pdf_metadata(path, &mut info)?,
        "docx" | "xlsx" | "pptx" => read_ooxml_metadata(path, &mut info)?,
        "doc" | "xls" | "ppt" => read_legacy_metadata(path, &mut info)?,
        _ => return Err(AppError::msg("不支持的文件类型")),
    }
    Ok(info)
}

fn push(info: &mut MetadataInfo, key: impl Into<String>, value: impl Into<String>) {
    let value = value.into();
    if !value.is_empty() {
        info.fields.push(MetadataField {
            key: key.into(),
            value,
        });
    }
}

fn read_image_metadata(path: &Path, info: &mut MetadataInfo) {
    // Basic dimensions / format
    if let Ok(reader) = image::ImageReader::open(path) {
        if let Ok(reader) = reader.with_guessed_format() {
            if let Ok(dim) = reader.into_dimensions() {
                push(info, "尺寸", format!("{} × {}", dim.0, dim.1));
            }
        }
    }

    // EXIF (jpeg/tiff/png/webp)
    if matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(),
        Some("jpg" | "jpeg" | "tiff" | "tif" | "png" | "webp")
    ) {
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut buf_reader = std::io::BufReader::new(file);
        if let Ok(exif) = kamadak_exif::Reader::new().read_from_container(&mut buf_reader) {
            for field in exif.fields().take(20) {
                push(
                    info,
                    format!("EXIF · {}", field.tag),
                    field.display_value().with_unit(&exif).to_string(),
                );
            }
        }
    }

    // PNG text chunks
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("png"))
    {
        for (key, value) in cleaners::image::read_png_text_chunks(path) {
            push(info, format!("PNG · {key}"), value);
        }
    }
}

fn read_pdf_metadata(path: &Path, info: &mut MetadataInfo) -> AppResult<()> {
    let doc = lopdf::Document::load(path)?;
    push(info, "页数", doc.get_pages().len().to_string());

    let mut info_fields: Vec<(String, String)> = Vec::new();
    if let Ok(Object::Reference(id)) = doc.trailer.get(b"Info") {
        if let Ok(object) = doc.get_object(*id) {
            if let Object::Dictionary(dict) = object {
                for (key, value) in dict.iter() {
                    if let Some(text) = object_text(value) {
                        info_fields.push((
                            String::from_utf8_lossy(key).to_string(),
                            text,
                        ));
                    }
                }
            }
        }
    }
    for (key, value) in info_fields {
        push(info, format!("PDF · {key}"), value);
    }

    // XMP presence
    let has_xmp = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|obj| match obj {
            Object::Reference(id) => doc.get_object(*id).ok(),
            other => Some(other),
        })
        .and_then(|catalog| match catalog {
            Object::Dictionary(dict) => dict.get(b"Metadata").ok().cloned(),
            _ => None,
        })
        .is_some();
    if has_xmp {
        push(info, "PDF · XMP", "存在 XMP 元数据流");
    }
    Ok(())
}

fn object_text(obj: &lopdf::Object) -> Option<String> {
    match obj {
        Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).trim_matches('\0').to_string()),
        Object::Integer(v) => Some(v.to_string()),
        Object::Real(v) => Some(v.to_string()),
        Object::Boolean(v) => Some(v.to_string()),
        Object::Name(v) => Some(String::from_utf8_lossy(v).to_string()),
        _ => None,
    }
}

fn read_ooxml_metadata(path: &Path, info: &mut MetadataInfo) -> AppResult<()> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for target in ["docProps/core.xml", "docProps/app.xml"] {
        let Ok(mut entry) = archive.by_name(target) else {
            continue;
        };
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        let fields = parse_ooxml_fields(&data);
        let prefix = if target.ends_with("core.xml") { "核心" } else { "应用" };
        for (key, value) in fields {
            push(info, format!("{prefix} · {key}"), value);
        }
    }
    Ok(())
}

fn parse_ooxml_fields(data: &[u8]) -> Vec<(String, String)> {
    use quick_xml::events::Event;
    let mut result = Vec::new();
    let mut reader = quick_xml::Reader::from_reader(data);
    reader.config_mut().trim_text(false);
    let mut current: Option<String> = None;
    let mut text = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                current = Some(String::from_utf8_lossy(e.name().as_ref()).to_string());
                text.clear();
            }
            Ok(Event::End(_)) => {
                if let Some(name) = current.take() {
                    let trimmed = text.trim().to_string();
                    if !trimmed.is_empty() {
                        result.push((name, trimmed));
                    }
                }
            }
            Ok(Event::Text(e)) => {
                text.push_str(&String::from_utf8_lossy(e.as_ref()));
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    result
}

fn read_legacy_metadata(path: &Path, info: &mut MetadataInfo) -> AppResult<()> {
    let mut comp = cfb::open(path)?;
    for name in ["\u{5}SummaryInformation", "\u{5}DocumentSummaryInformation"] {
        let stream_path = format!("/{name}");
        let Ok(mut stream) = comp.open_stream(&stream_path) else {
            continue;
        };
        let mut data = Vec::new();
        stream.read_to_end(&mut data)?;
        let prefix = if name.starts_with('\u{5}') && name.ends_with("SummaryInformation") {
            "摘要"
        } else {
            "文档摘要"
        };
        for prop in propset::parse(&data) {
            push(info, format!("{prefix} · {}", prop.name), prop.value);
        }
    }
    Ok(())
}
