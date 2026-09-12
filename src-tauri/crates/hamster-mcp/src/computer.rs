//! V2 computer-use：桌面截图（xcap）+ 鼠标键盘（enigo）。
//! 覆盖浏览器工具到不了的场景：原生弹窗、桌面应用窗口、登录 UI 等。

use std::path::Path;

use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};
use hamster_core::{HamsterError, Result};

use crate::browser::parse_key;

pub struct Computer {
    enigo: Enigo,
}

impl Computer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            enigo: Enigo::new(&Settings::default())
                .map_err(|e| HamsterError::Other(format!("computer-use 初始化失败：{e}")))?,
        })
    }

    /// 全屏截图保存到指定路径，返回 (路径, "宽x高")。
    pub fn screenshot(&self, path: &Path) -> Result<(std::path::PathBuf, String)> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| HamsterError::Other(format!("截图目录创建失败：{e}")))?;
        }
        let monitor = xcap::Monitor::all()
            .map_err(|e| HamsterError::Other(format!("显示器枚举失败：{e}")))?
            .into_iter()
            .next()
            .ok_or_else(|| HamsterError::Other("未找到可用显示器".into()))?;
        let img = monitor
            .capture_image()
            .map_err(|e| HamsterError::Other(format!("桌面截图失败：{e}")))?;
        let size = format!("{}x{}", img.width(), img.height());
        img.save(path)
            .map_err(|e| HamsterError::Other(format!("截图写入失败：{e}")))?;
        Ok((path.to_path_buf(), size))
    }

    /// 移动并左键/右键点击绝对屏幕坐标。
    pub fn click(&mut self, x: i32, y: i32, right: bool) -> Result<()> {
        self.enigo
            .move_mouse(x, y, Coordinate::Abs)
            .map_err(|e| HamsterError::Other(format!("鼠标移动失败：{e}")))?;
        let btn = if right { Button::Right } else { Button::Left };
        self.enigo
            .button(btn, Direction::Click)
            .map_err(|e| HamsterError::Other(format!("点击失败：{e}")))?;
        Ok(())
    }

    /// 键入文本（当前焦点处）。
    pub fn type_text(&mut self, text: &str) -> Result<()> {
        use enigo::Keyboard as _;
        self.enigo
            .text(text)
            .map_err(|e| HamsterError::Other(format!("输入失败：{e}")))?;
        Ok(())
    }

    /// 按键（键表同 browser，支持 ctrl+c 等组合）。
    pub fn press_key(&mut self, key: &str) -> Result<()> {
        use enigo::Keyboard as _;
        let press = parse_key(key)?;
        let single_char = press.key.chars().count() == 1;
        // 单字符键按原始字符注入：enigo 的 Unicode 注入自带大小写，
        // 无需（也不应）按住 Shift，否则 "a" 会被打成 "A"
        let base = if single_char {
            let t = press
                .text
                .clone()
                .ok_or_else(|| HamsterError::config_invalid(format!("不支持的按键：{key}")))?;
            let c = t
                .chars()
                .next()
                .ok_or_else(|| HamsterError::config_invalid(format!("不支持的按键：{key}")))?;
            enigo::Key::Unicode(c)
        } else {
            key_name_to_enigo(&press.key)?
        };
        // 需按住的修饰键：单字符键跳过 Shift（见上），命名键全部按住
        let held: Vec<enigo::Key> = press
            .modifiers
            .iter()
            .filter(|(name, _, _)| !(single_char && name == "Shift"))
            .map(|(name, _, _)| key_name_to_enigo(name))
            .collect::<Result<_>>()?;
        for k in &held {
            self.enigo
                .key(*k, Direction::Press)
                .map_err(|e| HamsterError::Other(format!("按住修饰键失败：{e}")))?;
        }
        self.enigo
            .key(base, Direction::Click)
            .map_err(|e| HamsterError::Other(format!("按键失败：{e}")))?;
        for k in held.iter().rev() {
            self.enigo
                .key(*k, Direction::Release)
                .map_err(|e| HamsterError::Other(format!("释放修饰键失败：{e}")))?;
        }
        Ok(())
    }
}

fn key_name_to_enigo(name: &str) -> Result<enigo::Key> {
    use enigo::Key;
    Ok(match name {
        "Enter" => Key::Return,
        "Tab" => Key::Tab,
        "Escape" => Key::Escape,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "ArrowUp" => Key::UpArrow,
        "ArrowDown" => Key::DownArrow,
        "ArrowLeft" => Key::LeftArrow,
        "ArrowRight" => Key::RightArrow,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "Control" => Key::Control,
        "Shift" => Key::Shift,
        "Alt" => Key::Alt,
        "Meta" => Key::Meta,
        other => {
            let mut chars = other.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Key::Unicode(c),
                _ => {
                    return Err(HamsterError::config_invalid(format!(
                        "不支持的按键：{other}"
                    )))
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combo_maps_named_modifier_keys() {
        // 修饰键名必须能映射到 enigo，组合才会真正生效
        for name in ["Control", "Shift", "Alt", "Meta"] {
            assert!(key_name_to_enigo(name).is_ok(), "{name} 应可映射");
        }
    }

    #[test]
    fn parse_key_shared_with_browser() {
        // 与 browser 共用键表：组合与小写保持行为一致
        let p = parse_key("ctrl+c").unwrap();
        assert_eq!(p.modifiers[0].0, "Control");
        let p = parse_key("a").unwrap();
        assert_eq!(p.text.as_deref(), Some("a"), "小写字符不得被转成大写");
    }
}
