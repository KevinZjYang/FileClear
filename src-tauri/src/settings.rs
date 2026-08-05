use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::types::Settings;

const STORE_FILE: &str = "settings.json";
const KEY_CONTEXT_MENU: &str = "contextMenuEnabled";

pub fn load(app: &AppHandle) -> Result<Settings, String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    let enabled = store
        .get(KEY_CONTEXT_MENU)
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    Ok(Settings {
        context_menu_enabled: enabled,
    })
}

pub fn set_context_menu_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(KEY_CONTEXT_MENU, serde_json::json!(enabled));
    store.save().map_err(|e| e.to_string())?;

    if enabled {
        crate::context_menu::register()?;
    } else {
        crate::context_menu::unregister()?;
    }
    Ok(())
}

/// Called on every startup: ensure the context menu matches the saved setting.
pub fn init(app: &AppHandle) -> Result<(), String> {
    let settings = load(app)?;
    if settings.context_menu_enabled && !crate::context_menu::is_registered() {
        let _ = crate::context_menu::register();
    }
    Ok(())
}
