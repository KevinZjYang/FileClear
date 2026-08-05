use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

use crate::cleaners::propset;
use crate::error::{AppError, AppResult};

/// Clean legacy .doc/.xls/.ppt files by overwriting the OLE summary
/// information streams with empty PropertySets.
pub fn clean(input: &Path, output: &Path) -> AppResult<Vec<String>> {
    std::fs::copy(input, output)?;

    let mut comp = cfb::open_rw(output)?;
    let mut touched = 0usize;
    for name in ["\u{5}SummaryInformation", "\u{5}DocumentSummaryInformation"] {
        let stream_path = format!("/{name}");
        if comp.open_stream(&stream_path).is_err() {
            continue;
        }
        let fmtid = if name.starts_with('\u{5}') && name.ends_with("SummaryInformation") {
            propset::FMTID_SUMMARY_INFORMATION
        } else {
            propset::FMTID_DOCUMENT_SUMMARY_INFORMATION
        };
        let data = propset::empty_propset(fmtid);
        {
            let mut stream = comp.open_stream(&stream_path)?;
            stream.seek(SeekFrom::Start(0))?;
            stream.write_all(&data)?;
            stream.set_len(data.len() as u64)?;
        }
        touched += 1;
    }
    comp.flush()?;

    if touched == 0 {
        return Err(AppError::msg(
            "未找到 OLE 摘要信息流（文件可能不是有效的 .doc/.xls/.ppt）",
        ));
    }
    Ok(vec![format!("已清理 {touched} 个摘要信息流")])
}
