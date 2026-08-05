use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub file_type: String,
    pub modified: u64,
    pub supported: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataField {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MetadataInfo {
    pub path: String,
    pub file_type: String,
    pub fields: Vec<MetadataField>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanFileResult {
    pub path: String,
    pub success: bool,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub original_size: u64,
    pub cleaned_size: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub current: usize,
    pub total: usize,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub context_menu_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            context_menu_enabled: true,
        }
    }
}
