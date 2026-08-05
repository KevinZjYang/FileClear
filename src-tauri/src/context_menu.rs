//! Windows Explorer context menu registration (per-user, no admin required).
//! Writes to HKCU\Software\Classes\*\shell\FileClear, which Windows merges
//! into the shell's file context menu.

const MENU_KEY: &str = "FileClear";
const MENU_LABEL: &str = "用 FileClear 清理元数据";

fn key_path() -> String {
    format!("Software\\Classes\\*\\shell\\{MENU_KEY}")
}

#[cfg(target_os = "windows")]
pub fn register() -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = exe.to_string_lossy().to_string();
    // 注意：HKCU 下资源管理器不会展开 %*，必须用带引号的 %1；
    // MultiSelectModel=Single 让多选时逐文件调用（否则只取第一个文件）。
    let command = format!("\"{exe_str}\" --quick-clean \"%1\"");

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (shell_key, _) = hkcu
        .create_subkey(&key_path())
        .map_err(|e| e.to_string())?;
    shell_key.set_value("", &MENU_LABEL).map_err(|e| e.to_string())?;
    shell_key
        .set_value("Icon", &exe_str)
        .map_err(|e| e.to_string())?;
    shell_key
        .set_value("MultiSelectModel", &"Single")
        .map_err(|e| e.to_string())?;
    let (command_key, _) = shell_key
        .create_subkey("command")
        .map_err(|e| e.to_string())?;
    command_key
        .set_value("", &command)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn register() -> Result<(), String> {
    Err("右键菜单仅在 Windows 上可用".to_string())
}

#[cfg(target_os = "windows")]
pub fn unregister() -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.delete_subkey_all(&key_path()) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn unregister() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn is_registered() -> bool {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(&key_path()).is_ok()
}

#[cfg(not(target_os = "windows"))]
pub fn is_registered() -> bool {
    false
}
