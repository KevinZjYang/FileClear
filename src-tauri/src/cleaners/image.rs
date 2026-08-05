use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use image::ExtendedColorType;
use image::ImageEncoder;

use crate::error::{AppError, AppResult};

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

pub fn clean(input: &Path, output: &Path) -> AppResult<Vec<String>> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => strip_jpeg_segments(input, output),
        "png" => strip_png_chunks(input, output),
        "gif" => reencode_gif(input, output),
        "webp" => reencode_webp(input, output),
        "tiff" | "tif" => reencode_tiff(input, output),
        "bmp" => reencode_bmp(input, output),
        _ => Err(AppError::msg("不支持的图片类型")),
    }
}

fn read_be16(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([data[offset], data[offset + 1]])
}

fn read_be32(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
}

/// Losslessly strip APP1(EXIF/XMP)、APP2(ICC/XMP)、APP13(Photoshop) and COM segments from JPEG.
fn strip_jpeg_segments(input: &Path, output: &Path) -> AppResult<Vec<String>> {
    let data = std::fs::read(input)?;
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return Err(AppError::msg("不是有效的 JPEG 文件"));
    }

    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(&data[..2]);
    let mut i = 2usize;
    let mut dropped = 0usize;
    let mut saw_eoi = false;

    while i < data.len() {
        if data[i] != 0xFF {
            return Err(AppError::msg("JPEG 解析错误：标记位置异常"));
        }
        let marker = data[i + 1];
        match marker {
            0xD9 => {
                out.extend_from_slice(&data[i..i + 2]);
                i += 2;
                saw_eoi = true;
                break;
            }
            0x01 | 0xD0..=0xD7 => {
                out.extend_from_slice(&data[i..i + 2]);
                i += 2;
            }
            0xDA => {
                if i + 4 > data.len() {
                    return Err(AppError::msg("JPEG 解析错误：SOS 头不完整"));
                }
                let header_len = read_be16(&data, i + 2) as usize;
                if header_len < 2 || i + 2 + header_len > data.len() {
                    return Err(AppError::msg("JPEG 解析错误：SOS 长度无效"));
                }
                out.extend_from_slice(&data[i..i + 2 + header_len]);
                i += 2 + header_len;
                // Copy entropy-coded data until the next non-stuffed marker.
                let mut j = i;
                while j < data.len() {
                    if data[j] == 0xFF {
                        let next = *data.get(j + 1).unwrap_or(&0x00);
                        if next == 0x00 {
                            j += 2;
                        } else if (0xD0..=0xD7).contains(&next) {
                            out.extend_from_slice(&data[i..j + 2]);
                            i = j + 2;
                            j = i;
                        } else {
                            out.extend_from_slice(&data[i..j]);
                            i = j;
                            break;
                        }
                    } else {
                        j += 1;
                    }
                }
            }
            _ => {
                if i + 4 > data.len() {
                    return Err(AppError::msg("JPEG 解析错误：标记头不完整"));
                }
                let seg_len = read_be16(&data, i + 2) as usize;
                if seg_len < 2 || i + 2 + seg_len > data.len() {
                    return Err(AppError::msg("JPEG 解析错误：标记长度无效"));
                }
                let skip = matches!(marker, 0xE1 | 0xE2 | 0xED) || marker == 0xFE;
                if skip {
                    dropped += 2 + seg_len;
                    i += 2 + seg_len;
                } else {
                    out.extend_from_slice(&data[i..i + 2 + seg_len]);
                    i += 2 + seg_len;
                }
            }
        }
    }

    if !saw_eoi {
        return Err(AppError::msg("JPEG 解析错误：缺少 EOI 标记"));
    }
    if i < data.len() {
        out.extend_from_slice(&data[i..]);
    }

    std::fs::write(output, &out)?;
    let warnings = if dropped > 0 {
        vec![format!("已移除 {dropped} 字节元数据")]
    } else {
        Vec::new()
    };
    Ok(warnings)
}

/// Losslessly strip privacy-related ancillary chunks from PNG
/// (tEXt/zTXt/iTXt/eXIf/iCCP/tIME), keeping image + animation data intact.
fn strip_png_chunks(input: &Path, output: &Path) -> AppResult<Vec<String>> {
    let data = std::fs::read(input)?;
    if data.len() < 8 || &data[..8] != PNG_SIGNATURE {
        return Err(AppError::msg("不是有效的 PNG 文件"));
    }

    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(PNG_SIGNATURE);
    let mut i = 8usize;
    let mut dropped = 0usize;
    let mut saw_idat = false;
    let mut saw_iend = false;

    while i + 8 <= data.len() {
        let len = read_be32(&data, i) as usize;
        let ctype = &data[i + 4..i + 8];
        let total = 12usize.checked_add(len).ok_or_else(|| AppError::msg("PNG 块长度溢出"))?;
        if i + total > data.len() {
            return Err(AppError::msg("PNG 解析错误：块长度无效"));
        }
        let is_meta = matches!(ctype, b"tEXt" | b"zTXt" | b"iTXt" | b"eXIf" | b"iCCP" | b"tIME");
        if is_meta {
            dropped += total;
        } else {
            out.extend_from_slice(&data[i..i + total]);
        }
        if ctype == b"IDAT" {
            saw_idat = true;
        }
        if ctype == b"IEND" {
            saw_iend = true;
            i += total;
            break;
        }
        i += total;
    }

    if !saw_idat || !saw_iend {
        return Err(AppError::msg("PNG 解析错误：缺少 IDAT/IEND 块"));
    }
    if i < data.len() {
        out.extend_from_slice(&data[i..]);
    }

    std::fs::write(output, &out)?;
    let warnings = if dropped > 0 {
        vec![format!("已移除 {dropped} 字节元数据")]
    } else {
        Vec::new()
    };
    Ok(warnings)
}

fn reencode_gif(input: &Path, output: &Path) -> AppResult<Vec<String>> {
    use image::AnimationDecoder;

    let decoder = image::codecs::gif::GifDecoder::new(BufReader::new(File::open(input)?))?;
    let frames = decoder.into_frames().collect_frames()?;
    if frames.is_empty() {
        return Err(AppError::msg("GIF 中没有可编码的帧"));
    }
    let mut encoder =
        image::codecs::gif::GifEncoder::new(BufWriter::new(File::create(output)?));
    encoder.encode_frames(frames.into_iter())?;
    Ok(Vec::new())
}

fn reencode_webp(input: &Path, output: &Path) -> AppResult<Vec<String>> {
    use image::AnimationDecoder;

    let decoder = image::codecs::webp::WebPDecoder::new(BufReader::new(File::open(input)?))?;
    let frames = decoder.into_frames().collect_frames()?;
    if frames.is_empty() {
        return Err(AppError::msg("WebP 中没有可编码的帧"));
    }
    let mut warnings = Vec::new();
    let frame = if frames.len() > 1 {
        warnings.push("动画 WebP 仅保留第一帧".to_string());
        &frames[0]
    } else {
        &frames[0]
    };
    let buf = frame.buffer();
    let encoder =
        image::codecs::webp::WebPEncoder::new_lossless(BufWriter::new(File::create(output)?));
    encoder.encode(buf.as_raw(), buf.width(), buf.height(), ExtendedColorType::Rgba8)?;
    Ok(warnings)
}

fn to_8bit_buf(
    img: image::DynamicImage,
    warnings: &mut Vec<String>,
) -> (Vec<u8>, u32, u32, ExtendedColorType) {
    match img.color() {
        image::ColorType::L8 => {
            let i = img.into_luma8();
            let (w, h) = (i.width(), i.height());
            (i.into_raw(), w, h, ExtendedColorType::L8)
        }
        image::ColorType::La8 => {
            let i = img.into_luma_alpha8();
            let (w, h) = (i.width(), i.height());
            (i.into_raw(), w, h, ExtendedColorType::La8)
        }
        image::ColorType::Rgb8 => {
            let i = img.into_rgb8();
            let (w, h) = (i.width(), i.height());
            (i.into_raw(), w, h, ExtendedColorType::Rgb8)
        }
        image::ColorType::Rgba8 => {
            let i = img.into_rgba8();
            let (w, h) = (i.width(), i.height());
            (i.into_raw(), w, h, ExtendedColorType::Rgba8)
        }
        _ => {
            warnings.push("已转换为 8 位 RGBA".to_string());
            let i = img.to_rgba8();
            let (w, h) = (i.width(), i.height());
            (i.into_raw(), w, h, ExtendedColorType::Rgba8)
        }
    }
}

fn reencode_tiff(input: &Path, output: &Path) -> AppResult<Vec<String>> {
    let img = image::ImageReader::open(input)?
        .with_guessed_format()?
        .decode()?;
    let mut warnings = Vec::new();
    let (buf, w, h, color) = to_8bit_buf(img, &mut warnings);
    let encoder = image::codecs::tiff::TiffEncoder::new(BufWriter::new(File::create(output)?));
    encoder.write_image(&buf, w, h, color)?;
    Ok(warnings)
}

fn reencode_bmp(input: &Path, output: &Path) -> AppResult<Vec<String>> {
    let img = image::ImageReader::open(input)?
        .with_guessed_format()?
        .decode()?;
    let warnings = Vec::new();
    let has_alpha = img.color().has_alpha();
    let (buf, w, h, color) = if has_alpha {
        let i = img.to_rgba8();
        let (w, h) = (i.width(), i.height());
        (i.into_raw(), w, h, ExtendedColorType::Rgba8)
    } else {
        let i = img.to_rgb8();
        let (w, h) = (i.width(), i.height());
        (i.into_raw(), w, h, ExtendedColorType::Rgb8)
    };
    let mut writer = BufWriter::new(File::create(output)?);
    let encoder = image::codecs::bmp::BmpEncoder::new(&mut writer);
    encoder.write_image(&buf, w, h, color)?;
    Ok(warnings)
}

/// Read PNG text metadata (tEXt / iTXt / zTXt) for preview purposes.
pub fn read_png_text_chunks(path: &Path) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let Ok(data) = std::fs::read(path) else {
        return result;
    };
    if data.len() < 8 || &data[..8] != PNG_SIGNATURE {
        return result;
    }
    let mut i = 8usize;
    while i + 8 <= data.len() {
        let len = read_be32(&data, i) as usize;
        let ctype = &data[i + 4..i + 8];
        let total = 12usize.checked_add(len).unwrap_or(usize::MAX);
        if i + total > data.len() {
            break;
        }
        let chunk = &data[i + 8..i + 8 + len];
        match ctype {
            b"tEXt" => {
                if let Some(nul) = chunk.iter().position(|b| *b == 0) {
                    let key = decode_utf8_lossy(&chunk[..nul]);
                    let value = decode_utf8_lossy(&chunk[nul + 1..]);
                    result.push((key, value));
                }
            }
            b"iTXt" => {
                if let Some(nul) = chunk.iter().position(|b| *b == 0) {
                    let key = decode_utf8_lossy(&chunk[..nul]);
                    result.push((key, "(iTXt)".to_string()));
                }
            }
            b"zTXt" => {
                if let Some(nul) = chunk.iter().position(|b| *b == 0) {
                    let key = decode_utf8_lossy(&chunk[..nul]);
                    result.push((key, "(zTXt 压缩文本)".to_string()));
                }
            }
            _ => {}
        }
        if ctype == b"IEND" {
            break;
        }
        i += total;
    }
    result
}

fn decode_utf8_lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim_matches('\0').to_string()
}