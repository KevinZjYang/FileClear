//! Minimal MS-OLEPS PropertySet (SummaryInformation) parsing and
//! empty-stream generation for legacy .doc/.xls/.ppt files.

use chrono::{DateTime, Utc};

pub const FMTID_SUMMARY_INFORMATION: [u8; 16] = [
    0xE0, 0x85, 0x9F, 0xF2, 0xF9, 0x4F, 0x68, 0x10, 0xAB, 0x91, 0x08, 0x00, 0x2B, 0x27, 0xB3, 0xD9,
];
pub const FMTID_DOCUMENT_SUMMARY_INFORMATION: [u8; 16] = [
    0x02, 0xD5, 0xCD, 0xD5, 0x9C, 0x2E, 0x1B, 0x10, 0x93, 0x97, 0x08, 0x00, 0x2B, 0x2C, 0xF9, 0xAE,
];

const VT_I2: u32 = 2;
const VT_I4: u32 = 3;
const VT_BSTR: u32 = 8;
const VT_BOOL: u32 = 11;
const VT_UI1: u32 = 17;
const VT_UI2: u32 = 18;
const VT_UI4: u32 = 19;
const VT_LPSTR: u32 = 30;
const VT_LPWSTR: u32 = 31;
const VT_FILETIME: u32 = 64;

#[derive(Debug, Clone)]
pub struct PropValue {
    pub name: String,
    pub value: String,
}

fn summary_names() -> &'static [(u32, &'static str)] {
    &[
        (0x01, "标题"),
        (0x02, "主题"),
        (0x03, "作者"),
        (0x04, "关键词"),
        (0x05, "备注"),
        (0x06, "模板"),
        (0x07, "最后保存者"),
        (0x08, "修订号"),
        (0x09, "总编辑时间"),
        (0x0A, "最后打印时间"),
        (0x0B, "创建时间"),
        (0x0C, "最后保存时间"),
        (0x0D, "页数"),
        (0x0E, "字数"),
        (0x0F, "字符数"),
        (0x10, "缩略图"),
        (0x11, "应用程序"),
        (0x12, "安全级别"),
    ]
}

fn doc_summary_names() -> &'static [(u32, &'static str)] {
    &[
        (0x02, "类别"),
        (0x03, "演示目标"),
        (0x04, "字节数"),
        (0x05, "行数"),
        (0x06, "段落数"),
        (0x07, "幻灯片数"),
        (0x08, "备注数"),
        (0x09, "隐藏幻灯片数"),
        (0x0A, "多媒体剪辑数"),
        (0x0E, "经理"),
        (0x0F, "公司"),
        (0x13, "共享文档"),
    ]
}

fn name_for(fmtid: &[u8; 16], id: u32) -> String {
    let table = if fmtid == &FMTID_SUMMARY_INFORMATION {
        summary_names()
    } else {
        doc_summary_names()
    };
    table
        .iter()
        .find(|(pid, _)| *pid == id)
        .map(|(_, n)| n.to_string())
        .unwrap_or_else(|| format!("属性 0x{id:04X}"))
}

/// Generate a minimal, valid empty PropertySet stream (one section, zero properties).
pub fn empty_propset(fmtid: [u8; 16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(48 + 8);
    out.extend_from_slice(&0xFFFEu16.to_le_bytes()); // byte order
    out.extend_from_slice(&0x0000u16.to_le_bytes()); // version
    out.extend_from_slice(&0x0000_0000u32.to_le_bytes()); // system identifier
    out.extend_from_slice(&[0u8; 16]); // CLSID
    out.extend_from_slice(&1u32.to_le_bytes()); // number of sections
    out.extend_from_slice(&fmtid); // section FMTID
    out.extend_from_slice(&48u32.to_le_bytes()); // section offset
    out.extend_from_slice(&8u32.to_le_bytes()); // section size
    out.extend_from_slice(&0u32.to_le_bytes()); // property count
    out
}

/// Parse a PropertySet stream and return human-readable metadata properties.
pub fn parse(data: &[u8]) -> Vec<PropValue> {
    let mut result = Vec::new();
    if data.len() < 28 {
        return result;
    }
    let section_count = u32::from_le_bytes([data[24], data[25], data[26], data[27]]) as usize;
    let mut cursor = 28usize;
    for _ in 0..section_count {
        if cursor + 20 > data.len() {
            break;
        }
        let mut fmtid = [0u8; 16];
        fmtid.copy_from_slice(&data[cursor..cursor + 16]);
        let sec_off = u32::from_le_bytes([
            data[cursor + 16],
            data[cursor + 17],
            data[cursor + 18],
            data[cursor + 19],
        ]) as usize;
        cursor += 20;

        if sec_off + 8 > data.len() {
            continue;
        }
        let prop_count = u32::from_le_bytes([
            data[sec_off + 4],
            data[sec_off + 5],
            data[sec_off + 6],
            data[sec_off + 7],
        ]) as usize;

        let mut p = sec_off + 8;
        for _ in 0..prop_count {
            if p + 8 > data.len() {
                break;
            }
            let id = u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]);
            let rel = u32::from_le_bytes([data[p + 4], data[p + 5], data[p + 6], data[p + 7]]) as usize;
            p += 8;
            let value_off = sec_off + rel;
            if value_off + 4 > data.len() {
                continue;
            }
            let ty = u32::from_le_bytes([
                data[value_off],
                data[value_off + 1],
                data[value_off + 2],
                data[value_off + 3],
            ]);
            let value = read_value(data, value_off + 4, ty);
            if let Some(value) = value {
                result.push(PropValue {
                    name: name_for(&fmtid, id),
                    value,
                });
            }
        }
    }
    result
}

fn read_value(data: &[u8], off: usize, ty: u32) -> Option<String> {
    match ty {
        VT_LPSTR => {
            let end = data[off..].iter().position(|b| *b == 0).map(|i| off + i).unwrap_or(data.len());
            let bytes = &data[off..end];
            Some(String::from_utf8_lossy(bytes).trim_matches('\0').to_string())
        }
        VT_LPWSTR => {
            let bytes = &data[off..];
            let mut chars = Vec::new();
            let mut i = 0;
            while i + 1 < bytes.len() {
                let unit = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
                if unit == 0 {
                    break;
                }
                chars.push(unit);
                i += 2;
            }
            Some(String::from_utf16_lossy(&chars))
        }
        VT_BSTR => {
            if off + 4 > data.len() {
                return None;
            }
            let len = u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]) as usize;
            let start = off + 4;
            if start + len * 2 > data.len() {
                return None;
            }
            let mut chars = Vec::new();
            for i in 0..len {
                let unit = u16::from_le_bytes([data[start + i * 2], data[start + i * 2 + 1]]);
                chars.push(unit);
            }
            Some(String::from_utf16_lossy(&chars))
        }
        VT_FILETIME => {
            if off + 8 > data.len() {
                return None;
            }
            let ft = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            let unix_secs = (ft / 10_000_000) as i64 - 116_444_736_00;
            Some(
                DateTime::<Utc>::from_timestamp(unix_secs, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "无效时间".to_string()),
            )
        }
        VT_I2 => {
            if off + 2 > data.len() {
                return None;
            }
            Some(i16::from_le_bytes([data[off], data[off + 1]]).to_string())
        }
        VT_I4 => {
            if off + 4 > data.len() {
                return None;
            }
            Some(i32::from_le_bytes([
                data[off],
                data[off + 1],
                data[off + 2],
                data[off + 3],
            ]).to_string())
        }
        VT_UI1 => Some(data[off].to_string()),
        VT_UI2 => {
            if off + 2 > data.len() {
                return None;
            }
            Some(u16::from_le_bytes([data[off], data[off + 1]]).to_string())
        }
        VT_UI4 => {
            if off + 4 > data.len() {
                return None;
            }
            Some(u32::from_le_bytes([
                data[off],
                data[off + 1],
                data[off + 2],
                data[off + 3],
            ]).to_string())
        }
        VT_BOOL => {
            if off + 2 > data.len() {
                return None;
            }
            Some(if data[off] == 0xFF { "是".to_string() } else { "否".to_string() })
        }
        _ => None,
    }
}
