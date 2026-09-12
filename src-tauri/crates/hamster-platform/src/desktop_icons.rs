//! 桌面图标显示控制（注册表 HideIcons + SHChangeNotify 广播）

use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const ADVANCED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const VALUE: &str = "HideIcons";

/// 读取当前 HideIcons 值（不存在 = 图标可见，返回 None）
pub fn get_hide_icons() -> Option<u32> {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(ADVANCED)
        .ok()?
        .get_value(VALUE)
        .ok()
}

/// 写入 HideIcons 并广播刷新，返回旧值
pub fn set_hide_icons(v: u32) -> Result<Option<u32>, String> {
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(
            ADVANCED,
            winreg::enums::KEY_SET_VALUE | winreg::enums::KEY_QUERY_VALUE,
        )
        .map_err(|e| format!("打开注册表失败: {e}"))?;
    let old: Option<u32> = key.get_value(VALUE).ok();
    key.set_value(VALUE, &v)
        .map_err(|e| format!("写入注册表失败: {e}"))?;
    refresh_desktop();
    Ok(old)
}

/// 通知 shell 刷新（重新读取桌面设置）
pub fn refresh_desktop() {
    unsafe {
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_hide_icons_does_not_panic() {
        // 只读不写，验证注册表路径可用
        let _ = get_hide_icons();
    }
}
