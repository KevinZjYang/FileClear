//! 清洗引擎单元测试：现场构造带元数据的 fixture，
//! 验证清洗后元数据消失、文件仍可正常打开、内容保持不变。

use std::io::{BufWriter, Read, Write};
use image::ImageEncoder;
use std::path::{Path, PathBuf};

use crate::cleaners::{self, legacy, ooxml, pdf, propset};
use crate::cleaners::image as image_cleaner;

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("创建临时目录")
}

fn contains(data: &[u8], pattern: &[u8]) -> bool {
    data.windows(pattern.len()).any(|w| w == pattern)
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// 构造最小合法 EXIF APP1 段（Make = ACME）。
fn exif_app1() -> Vec<u8> {
    let mut tiff = Vec::new();
    tiff.extend_from_slice(b"II");
    tiff.extend_from_slice(&0x002Au16.to_le_bytes());
    tiff.extend_from_slice(&8u32.to_le_bytes()); // IFD0 偏移
    tiff.extend_from_slice(&1u16.to_le_bytes()); // 条目数
    tiff.extend_from_slice(&0x010Fu16.to_le_bytes()); // Make
    tiff.extend_from_slice(&2u16.to_le_bytes()); // ASCII
    tiff.extend_from_slice(&4u32.to_le_bytes()); // 长度 4
    tiff.extend_from_slice(b"ACME");
    tiff.extend_from_slice(&0u32.to_le_bytes()); // 下一个 IFD

    let mut payload = Vec::new();
    payload.extend_from_slice(b"Exif\0\0");
    payload.extend_from_slice(&tiff);

    let mut segment = vec![0xFF, 0xE1];
    segment.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    segment.extend_from_slice(&payload);
    segment
}

/// 生成带 EXIF 的 JPEG fixture。
fn jpeg_with_exif(dir: &Path) -> PathBuf {
    let img = image::RgbImage::from_pixel(64, 48, image::Rgb([120u8, 90, 200]));
    let base = dir.join("base.jpg");
    {
        let mut writer = BufWriter::new(std::fs::File::create(&base).unwrap());
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, 95);
        encoder
            .encode(&img.into_raw(), 64, 48, image::ExtendedColorType::Rgb8)
            .unwrap();
    }
    let data = std::fs::read(&base).unwrap();
    let mut out = Vec::with_capacity(data.len() + 64);
    out.extend_from_slice(&data[..2]); // SOI
    out.extend_from_slice(&exif_app1());
    out.extend_from_slice(&data[2..]);
    let path = dir.join("with_exif.jpg");
    std::fs::write(&path, out).unwrap();
    path
}

/// 生成带 tEXt 文本块的 PNG fixture。
fn png_with_text(dir: &Path) -> PathBuf {
    let img = image::RgbaImage::from_pixel(32, 32, image::Rgba([10u8, 20, 30, 255]));
    let base = dir.join("base.png");
    {
        let mut writer = BufWriter::new(std::fs::File::create(&base).unwrap());
        image::codecs::png::PngEncoder::new(&mut writer)
            .write_image(&img.into_raw(), 32, 32, image::ExtendedColorType::Rgba8)
            .unwrap();
    }
    let data = std::fs::read(&base).unwrap();
    let mut i = 8usize;
    while i + 12 <= data.len() {
        let len = u32::from_be_bytes(data[i..i + 4].try_into().unwrap()) as usize;
        if &data[i + 4..i + 8] == b"IDAT" {
            break;
        }
        i += 12 + len;
    }

    let text_data = b"Author\0Alice";
    let mut chunk = Vec::new();
    chunk.extend_from_slice(&(text_data.len() as u32).to_be_bytes());
    chunk.extend_from_slice(b"tEXt");
    chunk.extend_from_slice(text_data);
    chunk.extend_from_slice(&crc32(&chunk[4..]).to_be_bytes());

    let mut out = Vec::with_capacity(data.len() + chunk.len());
    out.extend_from_slice(&data[..i]);
    out.extend_from_slice(&chunk);
    out.extend_from_slice(&data[i..]);
    let path = dir.join("with_text.png");
    std::fs::write(&path, out).unwrap();
    path
}

/// 生成两帧 GIF 动画 fixture。
fn gif_animated(dir: &Path) -> PathBuf {
    let path = dir.join("anim.gif");
    let file = std::fs::File::create(&path).unwrap();
    let mut encoder = image::codecs::gif::GifEncoder::new(BufWriter::new(file));
    let delay = image::Delay::from_numer_denom_ms(100, 1);
    let frame1 = image::Frame::from_parts(
        image::RgbaImage::from_pixel(16, 16, image::Rgba([255u8, 0, 0, 255])),
        0,
        0,
        delay,
    );
    let frame2 = image::Frame::from_parts(
        image::RgbaImage::from_pixel(16, 16, image::Rgba([0u8, 255, 0, 255])),
        0,
        0,
        delay,
    );
    encoder
        .encode_frames(vec![frame1, frame2].into_iter())
        .unwrap();
    path
}

/// 生成带 Info 字典与 XMP 元数据的 PDF fixture。
fn pdf_with_metadata(dir: &Path) -> PathBuf {
    let path = dir.join("meta.pdf");
    let mut doc = lopdf::Document::with_version("1.4");

    let pages_id = doc.new_object_id();
    let page_id = doc.add_object(lopdf::Object::Dictionary(lopdf::Dictionary::from_iter([
        (b"Type".to_vec(), lopdf::Object::Name(b"Page".to_vec())),
        (b"Parent".to_vec(), lopdf::Object::Reference(pages_id)),
        (
            b"MediaBox".to_vec(),
            lopdf::Object::Array(vec![
                lopdf::Object::Integer(0),
                lopdf::Object::Integer(0),
                lopdf::Object::Integer(612),
                lopdf::Object::Integer(792),
            ]),
        ),
    ])));
    doc.objects.insert(
        pages_id,
        lopdf::Object::Dictionary(lopdf::Dictionary::from_iter([
            (b"Type".to_vec(), lopdf::Object::Name(b"Pages".to_vec())),
            (
                b"Kids".to_vec(),
                lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)]),
            ),
            (b"Count".to_vec(), lopdf::Object::Integer(1)),
        ])),
    );

    let xmp_id = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
        lopdf::Dictionary::from_iter([
            (b"Type".to_vec(), lopdf::Object::Name(b"Metadata".to_vec())),
            (b"Subtype".to_vec(), lopdf::Object::Name(b"XML".to_vec())),
        ]),
        b"<x:xmpmeta><rdf:RDF>secret</rdf:RDF></x:xmpmeta>".to_vec(),
    )));
    let root_id = doc.add_object(lopdf::Object::Dictionary(lopdf::Dictionary::from_iter([
        (b"Type".to_vec(), lopdf::Object::Name(b"Catalog".to_vec())),
        (b"Pages".to_vec(), lopdf::Object::Reference(pages_id)),
        (b"Metadata".to_vec(), lopdf::Object::Reference(xmp_id)),
    ])));
    doc.trailer.set(b"Root", lopdf::Object::Reference(root_id));

    let info_id = doc.add_object(lopdf::Object::Dictionary(lopdf::Dictionary::from_iter([
        (b"Title".to_vec(), lopdf::Object::string_literal("Secret Title")),
        (b"Author".to_vec(), lopdf::Object::string_literal("Alice")),
    ])));
    doc.trailer.set(b"Info", lopdf::Object::Reference(info_id));

    doc.save(&path).unwrap();
    path
}

/// 生成带 docProps 元数据的 .docx fixture。
fn docx_with_metadata(dir: &Path) -> PathBuf {
    let path = dir.join("sample.docx");
    let file = std::fs::File::create(&path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
    let core = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Secret Title</dc:title><dc:creator>Alice</dc:creator><dc:subject>Top Secret</dc:subject><cp:lastModifiedBy>Bob</cp:lastModifiedBy></cp:coreProperties>"#;
    let app = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>Word</Application><Company>ACME</Company></Properties>"#;
    let document = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Hello world</w:t></w:r></w:p></w:body></w:document>"#;

    zip.start_file("[Content_Types].xml", options).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("docProps/core.xml", options).unwrap();
    zip.write_all(core.as_bytes()).unwrap();
    zip.start_file("docProps/app.xml", options).unwrap();
    zip.write_all(app.as_bytes()).unwrap();
    zip.start_file("word/document.xml", options).unwrap();
    zip.write_all(document.as_bytes()).unwrap();
    zip.finish().unwrap();
    path
}

/// 构造带“作者=Alice”属性的非空 PropertySet 流。
fn propset_with_author() -> Vec<u8> {
    let mut value = Vec::new();
    value.extend_from_slice(&30u32.to_le_bytes()); // VT_LPSTR
    value.extend_from_slice(b"Alice\0");
    while value.len() % 4 != 0 {
        value.push(0);
    }
    let section_size = 16u32 + value.len() as u32; // 属性条目(16) + 值区

    let mut out = Vec::new();
    out.extend_from_slice(&0xFFFEu16.to_le_bytes()); // 字节序
    out.extend_from_slice(&0x0000u16.to_le_bytes()); // 版本
    out.extend_from_slice(&0x0000_0002u32.to_le_bytes()); // 系统标识
    out.extend_from_slice(&[0u8; 16]); // CLSID
    out.extend_from_slice(&1u32.to_le_bytes()); // 节数
    out.extend_from_slice(&propset::FMTID_SUMMARY_INFORMATION);
    out.extend_from_slice(&48u32.to_le_bytes()); // 节偏移
    out.extend_from_slice(&section_size.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes()); // 属性数
    out.extend_from_slice(&0x03u32.to_le_bytes()); // PID 0x03（作者）
    out.extend_from_slice(&16u32.to_le_bytes()); // 值偏移（相对节起始）
    out.extend_from_slice(&value);
    out
}

/// 生成含 SummaryInformation 流的旧版 .doc fixture。
fn legacy_doc_with_summary(dir: &Path) -> PathBuf {
    let path = dir.join("old.doc");
    let data = propset_with_author();
    let mut comp = cfb::create(&path).unwrap();
    {
        let mut stream = comp.create_stream("/\u{5}SummaryInformation").unwrap();
        stream.write_all(&data).unwrap();
    }
    comp.flush().unwrap();
    path
}

fn resolve_dict<'a>(
    doc: &'a lopdf::Document,
    obj: &'a lopdf::Object,
) -> Option<&'a lopdf::Dictionary> {
    match obj {
        lopdf::Object::Dictionary(dict) => Some(dict),
        lopdf::Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| resolve_dict(doc, o)),
        _ => None,
    }
}

#[test]
fn jpeg_exif_stripped() {
    let dir = temp_dir();
    let src = jpeg_with_exif(dir.path());
    let out = dir.path().join("out.jpg");
    let warnings = image_cleaner::clean(&src, &out).unwrap();
    assert!(
        warnings.iter().any(|w| w.contains("已移除")),
        "应提示已移除元数据：{warnings:?}"
    );
    let data = std::fs::read(&out).unwrap();
    assert!(!contains(&data, &[0xFF, 0xE1]), "APP1 段必须被移除");
    let dims = image::ImageReader::open(&out)
        .unwrap()
        .with_guessed_format()
        .unwrap()
        .into_dimensions()
        .unwrap();
    assert_eq!(dims, (64, 48), "像素尺寸应保持不变");
}

#[test]
fn png_text_chunks_stripped() {
    let dir = temp_dir();
    let src = png_with_text(dir.path());
    let out = dir.path().join("out.png");
    image_cleaner::clean(&src, &out).unwrap();
    let data = std::fs::read(&out).unwrap();
    assert!(!contains(&data, b"tEXt"), "tEXt 块必须被移除");
    let img = image::ImageReader::open(&out)
        .unwrap()
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!((img.width(), img.height()), (32, 32));
}

#[test]
fn gif_animation_preserved() {
    use image::AnimationDecoder;
    let dir = temp_dir();
    let src = gif_animated(dir.path());
    let out = dir.path().join("out.gif");
    image_cleaner::clean(&src, &out).unwrap();
    let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(
        std::fs::File::open(&out).unwrap(),
    ))
    .unwrap();
    let frames = decoder.into_frames().collect_frames().unwrap();
    assert_eq!(frames.len(), 2, "GIF 动画帧数应保持不变");
}

#[test]
fn pdf_info_and_xmp_cleared() {
    let dir = temp_dir();
    let src = pdf_with_metadata(dir.path());
    let out = dir.path().join("out.pdf");
    pdf::clean(&src, &out).unwrap();

    let doc = lopdf::Document::load(&out).unwrap();
    let info = doc.trailer.get(b"Info").unwrap();
    let info_dict = resolve_dict(&doc, info).expect("Info 应为字典");
    assert!(info_dict.is_empty(), "Info 字典应为空");

    let root = doc.trailer.get(b"Root").unwrap();
    let root_dict = resolve_dict(&doc, root).expect("Root 应为字典");
    assert!(root_dict.get(b"Metadata").is_err(), "目录中不得再引用 Metadata");
    assert!(root_dict.get(b"Pages").is_ok(), "页面结构应保留");

    // 不得残留任何 Metadata 类型流对象（含孤立对象），输出文件不含 XMP 字节。
    assert!(
        doc.objects
            .values()
            .all(|o| !o.type_name().is_ok_and(|n| n == b"Metadata")),
        "文件中不应残留 XMP Metadata 流对象"
    );
    let out_bytes = std::fs::read(&out).unwrap();
    assert!(
        !out_bytes.windows(10).any(|w| w == b"<x:xmpmeta"),
        "输出文件不应包含 XMP 内容"
    );
}

#[test]
fn ooxml_docprops_cleared() {
    let dir = temp_dir();
    let src = docx_with_metadata(dir.path());
    let out = dir.path().join("out.docx");
    let warnings = ooxml::clean(&src, &out).unwrap();
    assert_eq!(warnings.len(), 1);

    let mut archive = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    let mut core = String::new();
    archive
        .by_name("docProps/core.xml")
        .unwrap()
        .read_to_string(&mut core)
        .unwrap();
    assert!(!core.contains("Secret Title"));
    assert!(!core.contains("Alice"));
    assert!(core.contains("<dc:title"), "标签结构应保留");
    assert!(core.contains("<dc:creator"));

    let mut app = String::new();
    archive
        .by_name("docProps/app.xml")
        .unwrap()
        .read_to_string(&mut app)
        .unwrap();
    assert!(!app.contains("ACME"));
    assert!(app.contains("<Application"));

    let mut document = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut document)
        .unwrap();
    assert!(document.contains("Hello world"), "正文内容应保持不变");
}

#[test]
fn legacy_summary_stream_emptied() {
    let dir = temp_dir();
    let src = legacy_doc_with_summary(dir.path());

    // 清洗前可解析出作者属性。
    let before = propset_with_author();
    assert!(
        propset::parse(&before).iter().any(|p| p.name == "作者" && p.value == "Alice"),
        "fixture 应能解析出作者属性"
    );

    let out = dir.path().join("out.doc");
    let warnings = legacy::clean(&src, &out).unwrap();
    assert_eq!(warnings.len(), 1);

    let mut comp = cfb::open(&out).unwrap();
    let mut data = Vec::new();
    comp.open_stream("/\u{5}SummaryInformation")
        .unwrap()
        .read_to_end(&mut data)
        .unwrap();
    let parsed = propset::parse(&data);
    assert!(parsed.is_empty(), "清洗后应无任何属性：{parsed:?}");
}

#[test]
fn empty_propset_roundtrip() {
    let data = propset::empty_propset(propset::FMTID_SUMMARY_INFORMATION);
    assert!(propset::parse(&data).is_empty());
}

#[test]
fn clean_in_place_replaces_original() {
    let dir = temp_dir();
    let src = png_with_text(dir.path());
    let before = std::fs::read(&src).unwrap();
    assert!(contains(&before, b"tEXt"));

    let output = cleaners::clean_in_place(&src).unwrap();
    assert!(output.warnings.iter().any(|w| w.contains("已移除")));

    let after = std::fs::read(&src).unwrap();
    assert!(!contains(&after, b"tEXt"), "原路径内容应已被清洗");
    let dims = image::ImageReader::open(&src)
        .unwrap()
        .with_guessed_format()
        .unwrap()
        .into_dimensions()
        .unwrap();
    assert_eq!(dims, (32, 32), "原位替换后文件仍可正常打开");
}

#[test]
fn supported_extensions_dispatched() {
    assert!(cleaners::is_supported(Path::new("a.JPG")));
    assert!(cleaners::is_supported(Path::new("b.DocX")));
    assert!(cleaners::is_supported(Path::new("c.ppt")));
    assert!(!cleaners::is_supported(Path::new("d.txt")));
    assert_eq!(cleaners::extension(Path::new("x.jpeg")), Some(".jpeg"));
}

/// 完整链路：清洗前软件能读到元数据，clean_in_place 后软件预览不再显示。
#[test]
fn cleaned_files_show_no_metadata_in_preview() {
    let dir = temp_dir();

    let jpg = jpeg_with_exif(dir.path());
    let before = crate::metadata::read_metadata(&jpg).unwrap();
    assert!(
        before.fields.iter().any(|f| f.key.starts_with("EXIF")),
        "JPEG fixture 应能被预览读到 EXIF，实际：{before:?}"
    );
    cleaners::clean_in_place(&jpg).unwrap();
    let after = crate::metadata::read_metadata(&jpg).unwrap();
    assert!(
        !after.fields.iter().any(|f| f.key.starts_with("EXIF")),
        "JPEG 清洗后不应再显示 EXIF，实际：{after:?}"
    );

    let png = png_with_text(dir.path());
    let before = crate::metadata::read_metadata(&png).unwrap();
    assert!(
        before.fields.iter().any(|f| f.key.starts_with("PNG")),
        "PNG fixture 应能被预览读到文本元数据，实际：{before:?}"
    );
    cleaners::clean_in_place(&png).unwrap();
    let after = crate::metadata::read_metadata(&png).unwrap();
    assert!(
        !after.fields.iter().any(|f| f.key.starts_with("PNG")),
        "PNG 清洗后不应再显示文本元数据，实际：{after:?}"
    );

    let pdf = pdf_with_metadata(dir.path());
    let before = crate::metadata::read_metadata(&pdf).unwrap();
    assert!(
        before.fields.iter().any(|f| f.key.starts_with("PDF")),
        "PDF fixture 应能被预览读到元数据，实际：{before:?}"
    );
    cleaners::clean_in_place(&pdf).unwrap();
    let after = crate::metadata::read_metadata(&pdf).unwrap();
    assert!(
        !after.fields.iter().any(|f| f.key.starts_with("PDF")),
        "PDF 清洗后不应再显示元数据，实际：{after:?}"
    );

    let docx = docx_with_metadata(dir.path());
    let before = crate::metadata::read_metadata(&docx).unwrap();
    assert!(
        before.fields.iter().any(|f| f.key.starts_with("核心") || f.key.starts_with("应用")),
        "docx fixture 应能被预览读到元数据，实际：{before:?}"
    );
    cleaners::clean_in_place(&docx).unwrap();
    let after = crate::metadata::read_metadata(&docx).unwrap();
    assert!(
        !after.fields.iter().any(|f| f.key.starts_with("核心") || f.key.starts_with("应用")),
        "docx 清洗后不应再显示元数据，实际：{after:?}"
    );

    let doc = legacy_doc_with_summary(dir.path());
    let before = crate::metadata::read_metadata(&doc).unwrap();
    assert!(
        before.fields.iter().any(|f| f.key.starts_with("摘要") || f.key.starts_with("文档摘要")),
        "doc fixture 应能被预览读到元数据，实际：{before:?}"
    );
    cleaners::clean_in_place(&doc).unwrap();
    let after = crate::metadata::read_metadata(&doc).unwrap();
    assert!(
        !after.fields.iter().any(|f| f.key.starts_with("摘要") || f.key.starts_with("文档摘要")),
        "doc 清洗后不应再显示元数据，实际：{after:?}"
    );
}



