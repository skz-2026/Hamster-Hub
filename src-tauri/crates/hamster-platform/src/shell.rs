//! Shell 启动：打开 .lnk/exe/文件、资源管理器定位、已运行应用激活

use std::path::{Path, PathBuf};

use windows::core::PCWSTR;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 用系统关联方式打开（lnk/exe/文档/URL）
pub fn shell_open(path: &Path) -> bool {
    let p = to_wide(&path.to_string_lossy());
    let verb = to_wide("open");
    unsafe {
        let r = ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(p.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        );
        // ShellExecuteW 返回 HINSTANCE，>32 视为成功
        r.0 as usize > 32
    }
}

/// 从候选路径里取第一个 .exe 目标的文件名（小写，如 "chrome.exe"）
fn exe_name_from(paths: impl IntoIterator<Item = String>) -> Option<String> {
    paths.into_iter().find_map(|c| {
        // icon_location 可能带资源索引后缀（"...chrome.exe,0"），去掉纯数字尾段
        let c = match c.rsplit_once(',') {
            Some((head, idx)) if idx.len() <= 3 && idx.bytes().all(|b| b.is_ascii_digit()) => {
                head.to_string()
            }
            _ => c,
        };
        let p = PathBuf::from(&c);
        let is_exe = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);
        p.file_name()
            .and_then(|n| n.to_str())
            .filter(|_| is_exe)
            .map(|n| n.to_ascii_lowercase())
    })
}

/// 从 .lnk 解析目标 exe 的文件名（小写，如 "chrome.exe"）。
/// 绝对路径（link_info）→ 相对路径 → icon_location 逐级兜底；
/// 解析不出 .exe 目标（UWP shell: 目标/文档/URL）返回 None。
fn lnk_exe_file_name(path: &Path) -> Option<String> {
    let parsed = lnk::ShellLink::open(path).ok()?;
    let link_info = parsed.link_info().as_ref();
    exe_name_from(
        [
            link_info.and_then(|li| li.local_base_path_unicode().clone()),
            link_info.and_then(|li| li.local_base_path().clone()),
            parsed.relative_path().clone(),
            parsed.icon_location().clone(),
        ]
        .into_iter()
        .flatten(),
    )
}

/// 激活 target（.lnk）对应应用已运行的可见窗口，替代重复启动
/// （浏览器等单进程多窗应用，再次 shell_open 会新开一扇窗而非复用）。
/// 返回 true = 已激活，调用方不必再 shell_open；false = 未在运行，走正常启动。
pub fn activate_running(target: &Path) -> bool {
    let Some(exe) = cached_exe_file_name(target) else {
        return false;
    };
    let Some(id) = crate::tray::find_by_process(std::process::id(), &exe) else {
        return false;
    };
    crate::tray::activate(id).is_ok()
}

/// target(.lnk) → exe 名解析缓存：dock 运行态每 2.5s 轮询一批，
/// 不缓存则每轮都要重新打开解析全部 .lnk（文件 IO + lnk 反序列化）。
/// 键 = target 路径，值 = (mtime, 解析结果)；mtime 变化才重新解析。
type ExeNameCache = std::collections::HashMap<String, (std::time::SystemTime, Option<String>)>;
static EXE_NAME_CACHE: std::sync::OnceLock<std::sync::Mutex<ExeNameCache>> =
    std::sync::OnceLock::new();

/// 解析 target 的 exe 文件名（小写；非 exe 目标/UWP/folder 返回 None），带 mtime 缓存
pub fn cached_exe_file_name(target: &Path) -> Option<String> {
    let mtime = std::fs::metadata(target).and_then(|m| m.modified()).ok()?;
    let key = target.to_string_lossy().into_owned();
    let cache = EXE_NAME_CACHE.get_or_init(Default::default);
    if let Ok(guard) = cache.lock() {
        if let Some((seen_at, hit)) = guard.get(&key) {
            if *seen_at == mtime {
                return hit.clone();
            }
        }
    }
    let resolved = lnk_exe_file_name(target);
    if let Ok(mut guard) = cache.lock() {
        guard.insert(key, (mtime, resolved.clone()));
    }
    resolved
}

/// 在资源管理器中定位文件
pub fn reveal_in_explorer(path: &Path) -> bool {
    let arg = format!("/select,{}", path.to_string_lossy());
    let sel = to_wide(&arg);
    let exe = to_wide("explorer.exe");
    unsafe {
        let r = ShellExecuteW(
            None,
            None,
            PCWSTR(exe.as_ptr()),
            PCWSTR(sel.as_ptr()),
            None,
            SW_SHOWNORMAL,
        );
        r.0 as usize > 32
    }
}

/// 唤出系统真实「开始」菜单：注入 Ctrl+Esc 系统热键。
/// 接管模式隐藏了真任务栏，但开始菜单（StartMenuExperienceHost）是独立
/// 系统界面，仍可召出；Win 键注入会被 UIPI 过滤，Ctrl+Esc 不受限。
pub fn open_start_menu() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        keybd_event, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_CONTROL, VK_ESCAPE,
    };
    unsafe {
        keybd_event(VK_CONTROL.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_ESCAPE.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_ESCAPE.0 as u8, 0, KEYEVENTF_KEYUP, 0);
        keybd_event(VK_CONTROL.0 as u8, 0, KEYEVENTF_KEYUP, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_name_from_candidates() {
        // 绝对路径
        let got = exe_name_from([r"C:\Program Files\Google\Chrome\Application\chrome.exe".into()]);
        assert_eq!(got.as_deref(), Some("chrome.exe"));
        // 相对路径取最后一段
        let got = exe_name_from([r"..\..\Application\msedge.exe".into()]);
        assert_eq!(got.as_deref(), Some("msedge.exe"));
        // icon_location 带资源索引后缀
        let got = exe_name_from([r"C:\bin\app.exe,0".into()]);
        assert_eq!(got.as_deref(), Some("app.exe"));
        // 非 exe 目标 / 空候选
        assert_eq!(exe_name_from([r"C:\Users\a\readme.md".into()]), None);
        assert_eq!(exe_name_from(Vec::new()), None);
    }

    /// 本机验证：真实开始菜单 .lnk 至少能解析出一些 exe 文件名
    /// （依赖机器安装状态，cargo test -- --ignored 手动跑）
    #[test]
    #[ignore]
    fn lnk_real_start_menu_sample() {
        let mut dirs = Vec::new();
        if let Some(pd) = std::env::var_os("ProgramData") {
            dirs.push(std::path::PathBuf::from(pd).join(r"Microsoft\Windows\Start Menu\Programs"));
        }
        if let Some(ad) = std::env::var_os("APPDATA") {
            dirs.push(std::path::PathBuf::from(ad).join(r"Microsoft\Windows\Start Menu\Programs"));
        }
        let mut total = 0;
        let mut resolved = 0;
        for dir in dirs {
            for entry in walkdir::WalkDir::new(&dir)
                .max_depth(6)
                .into_iter()
                .flatten()
            {
                if entry
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| !e.eq_ignore_ascii_case("lnk"))
                    .unwrap_or(true)
                {
                    continue;
                }
                total += 1;
                if lnk_exe_file_name(entry.path()).is_some() {
                    resolved += 1;
                }
            }
        }
        println!("lnk 总数 {total}，解析出 exe 目标 {resolved}");
        assert!(total > 0, "本机开始菜单没有 .lnk");
        assert!(resolved > 0, "没有任何 .lnk 解析出 exe 目标");
    }

    /// 端到端：按进程名找记事本窗口并激活。已有记事本窗口则直接复用
    /// （等同一次 dock 点击，不动用户的窗）；没有才拉起新记事本，并在
    /// 结束时只关自己拉起的那个（空白文档静默关闭）。
    /// 依赖桌面会话，cargo test -- --ignored 手动跑。
    #[test]
    #[ignore]
    fn find_and_activate_notepad() {
        use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

        let mut spawned: Option<std::process::Child> = None;
        let mut found = crate::tray::find_by_process(std::process::id(), "notepad.exe");
        if found.is_none() {
            // find 为 None 说明当前没有任何可见记事本窗口，之后找到的必然是本次拉起的
            spawned = Some(
                std::process::Command::new("notepad.exe")
                    .spawn()
                    .expect("spawn notepad"),
            );
            for _ in 0..25 {
                found = crate::tray::find_by_process(std::process::id(), "notepad.exe");
                if found.is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
        let id = found.expect("notepad 窗口未被 find_by_process 找到");
        crate::tray::activate(id).unwrap();

        if let Some(mut child) = spawned {
            std::thread::sleep(std::time::Duration::from_millis(300));
            unsafe {
                let _ = PostMessageW(Some(HWND(id as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
            let _ = child.kill();
        }
    }
}
