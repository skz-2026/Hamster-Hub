//! V1 browser-use：自托管 Chromium CDP 引擎（chromiumoxide）的最小验证面。
//!
//! 借鉴 nomi-browser-engine 的形态（自管浏览器、页面注册表快照、trusted 输入），
//! 按验证场景裁剪：单标签页 + 顺序操作 + 截图落盘。
//! 发现的本机浏览器优先级：HAMSTER_BROWSER_PATH env → Chrome → Edge。

use std::path::PathBuf;

use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchKeyEventParams, DispatchKeyEventType, InsertTextParams,
};
use chromiumoxide::Page;
use futures_util::StreamExt;

use hamster_core::{HamsterError, Result};

pub struct VerifyBrowser {
    browser: Browser,
    tab: Page,
    /// 最近一次快照的交互元素数量（ref 1..=n 对应页面 window.__上游_els 下标）
    ref_count: u32,
    /// 自管浏览器进程（Edge/Chrome 启动器进程会立刻退出，必须自己持有并 kill）
    child: Option<std::process::Child>,
    user_data_dir: Option<PathBuf>,
}

impl Drop for VerifyBrowser {
    /// 进程兜底清理：MCP server 正常退出（stdin EOF）时保证浏览器进程与
    /// 临时目录不残留。显式 close() 已 take 掉字段，这里自然成为空操作。
    /// 宿主强杀 server 进程时 Drop 不会执行，属已知残留边界（见 verify-agent.md §6）。
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(dir) = self.user_data_dir.take() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// 探测本机 Chromium 系浏览器（env 覆盖 → Chrome → Edge）。
pub fn find_browser_executable() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("HAMSTER_BROWSER_PATH") {
        let p = PathBuf::from(p.trim());
        if p.is_file() {
            return Ok(p);
        }
    }
    let candidates = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        "/usr/bin/google-chrome",
        "/usr/bin/chromium-browser",
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    ];
    for c in candidates {
        let p = PathBuf::from(c);
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(HamsterError::Other(
        "未找到 Chrome/Edge：请设置 HAMSTER_BROWSER_PATH 指向浏览器可执行文件".into(),
    ))
}

impl VerifyBrowser {
    /// 启动浏览器并开第一个标签页（headless 默认 true）。
    ///
    /// 自管进程：Edge/Chrome 的启动器进程会立刻退出（状态 0），chromiumoxide
    /// 的 launch 监听不到真实进程的 WS 地址——所以这里自己 spawn（独立
    /// user-data-dir + 固定调试端口），端口就绪后 `Browser::connect` 接管。
    pub async fn open(url: &str, headless: bool) -> Result<Self> {
        use std::net::TcpListener;
        use std::process::{Command, Stdio};

        let exe = find_browser_executable()?;
        let port = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| HamsterError::Other(format!("取空闲端口失败：{e}")))?
            .local_addr()
            .map_err(|e| HamsterError::Other(format!("读端口失败：{e}")))?
            .port();
        let user_data = std::env::temp_dir().join(format!("hamster-mcp-{}", std::process::id()));
        let mut args = vec![
            format!("--remote-debugging-port={port}"),
            format!("--user-data-dir={}", user_data.display()),
            "--no-first-run".into(),
            "--no-default-browser-check".into(),
            "--disable-background-networking".into(),
        ];
        if headless {
            args.push("--headless=new".into());
        }
        #[cfg(windows)]
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let child = {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                Command::new(&exe)
                    .args(&args)
                    .creation_flags(CREATE_NO_WINDOW)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
            }
            #[cfg(not(windows))]
            {
                Command::new(&exe)
                    .args(&args)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
            }
        }
        .map_err(|e| HamsterError::Launch {
            program: exe.display().to_string(),
            message: format!("浏览器进程拉起失败：{e}"),
        })?;

        // 等调试端口就绪（进程提前退出则立即报错）
        let mut child = child;
        let mut ready = false;
        for _ in 0..100 {
            if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
                ready = true;
                break;
            }
            if let Ok(Some(_)) = child.try_wait() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        if !ready {
            let _ = child.kill();
            let _ = child.wait();
            return Err(HamsterError::Other(format!(
                "浏览器调试端口未就绪（{port}）：{} 进程可能提前退出",
                exe.display()
            )));
        }

        let (browser, mut handler) = Browser::connect(format!("http://127.0.0.1:{port}"))
            .await
            .map_err(|e| HamsterError::Other(format!("连接浏览器失败：{e}")))?;
        // chromiumoxide 要求驱动 handler 的后台循环（官方约定用法）
        tokio::spawn(async move { while handler.next().await.is_some() {} });
        let tab = browser
            .new_page("about:blank")
            .await
            .map_err(|e| HamsterError::Other(format!("打开标签页失败：{e}")))?;
        let this = Self {
            browser,
            tab,
            ref_count: 0,
            child: Some(child),
            user_data_dir: Some(user_data),
        };
        this.tab
            .goto(url)
            .await
            .map_err(|e| HamsterError::Other(format!("导航失败：{e}")))?;
        this.wait_page_ready(6000).await;
        Ok(this)
    }

    /// 导航到 URL，返回页面标题。
    pub async fn navigate(&mut self, url: &str) -> Result<String> {
        self.tab
            .goto(url)
            .await
            .map_err(|e| HamsterError::Other(format!("导航失败：{e}")))?;
        self.wait_page_ready(5000).await;
        self.page_title().await
    }

    async fn page_title(&self) -> Result<String> {
        self.tab
            .evaluate("document.title")
            .await
            .ok()
            .and_then(|r| r.into_value().ok())
            .and_then(|v: serde_json::Value| v.as_str().map(String::from))
            .ok_or_else(|| HamsterError::Other("读取页面标题失败".into()))
    }

    /// 等页面就绪：readyState 进入 interactive/complete 后再稳定一小段
    /// （给 SPA 懒加载 chunk 留时间），上限 max_ms。读取失败按未就绪继续轮询，
    /// 超时则放行（由后续 snapshot/click 自行面对真实页面状态）。
    async fn wait_page_ready(&self, max_ms: u64) {
        let settle = std::time::Duration::from_millis(300);
        let mut ready_since: Option<std::time::Instant> = None;
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_millis(max_ms) {
            if ready_since.is_some_and(|t| t.elapsed() >= settle) {
                return;
            }
            // 必须确认已离开 about:blank：goto 提交瞬间旧空白页的
            // readyState 也是 complete，不判 href 会在旧页面上提前放行
            let state = self
                .tab
                .evaluate("(() => ({ href: location.href, rs: document.readyState }))()")
                .await
                .ok()
                .and_then(|r| r.into_value().ok())
                .map(|v: serde_json::Value| {
                    (
                        v.get("href")
                            .and_then(serde_json::Value::as_str)
                            .map(String::from),
                        v.get("rs")
                            .and_then(serde_json::Value::as_str)
                            .map(String::from),
                    )
                });
            let on_target = state
                .as_ref()
                .and_then(|(href, _)| href.as_deref())
                .is_some_and(|h| h != "about:blank");
            match (on_target, state.and_then(|(_, rs)| rs).as_deref()) {
                (true, Some("interactive")) | (true, Some("complete")) => {
                    if ready_since.is_none() {
                        ready_since = Some(std::time::Instant::now());
                    }
                }
                _ => ready_since = None,
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    /// 页面快照：DOM 交互元素注册表（window.__上游_els）+ 缩进结构文本。
    /// ref 1..=n 与 __上游_els 下标一一对应；Vue/React 合成事件均可响应。
    /// 遍历穿透 Shadow DOM；深度超 10 或元素超 300 截断并在文末标注。
    pub async fn snapshot(&mut self) -> Result<String> {
        const JS: &str = r#"(() => {
            window.__上游_els = [];
            const out = [];
            let truncated = false;
            const interactive = (el) => {
              if (el.closest('[aria-hidden="true"]')) return null;
              const tag = el.tagName.toLowerCase();
              const role = el.getAttribute('role') || '';
              const explicit = ['button','a','input','select','textarea','label'].includes(tag);
              const roleHit = ['button','link','textbox','searchbox','checkbox','radio','combobox','listbox','option','menuitem','tab','slider','switch'].includes(role);
              if (!explicit && !roleHit && !el.onclick) return null;
              const fallbackRole = { button: 'button', a: 'link', input: (el.type === 'checkbox' ? 'checkbox' : 'textbox'), select: 'combobox', textarea: 'textbox', label: 'label' }[tag] || tag;
              const label = el.getAttribute('aria-label') || (el.innerText || '').trim().replace(/\s+/g, ' ').slice(0, 60) || el.value || el.getAttribute('placeholder') || el.getAttribute('title') || el.getAttribute('alt') || tag;
              return { role: role || fallbackRole, label: label };
            };
            const walk = (el, depth) => {
              if (out.length >= 300) { truncated = true; return; }
              if (depth > 10) { truncated = true; return; }
              const info = interactive(el);
              if (info) {
                window.__上游_els.push(el);
                const r = window.__上游_els.length;
                out.push('  '.repeat(depth) + '[' + r + '] ' + info.role + ' "' + info.label + '"');
              }
              const nextDepth = depth + (info ? 1 : 0);
              for (const c of el.children) walk(c, nextDepth);
              if (el.shadowRoot) for (const c of el.shadowRoot.children) walk(c, nextDepth + 1);
            };
            walk(document.body, 0);
            return { text: out.join('\n'), count: window.__上游_els.length, truncated: truncated };
        })()"#;
        let r = self
            .tab
            .evaluate(JS)
            .await
            .map_err(|e| HamsterError::Other(format!("页面快照失败：{e}")))?;
        let v: serde_json::Value = r
            .into_value()
            .map_err(|e| HamsterError::Other(format!("页面快照解析失败：{e}")))?;
        let text = v
            .get("text")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| HamsterError::Other("页面快照返回结构异常".into()))?;
        self.ref_count = v
            .get("count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u32;
        let mut out = text.to_string();
        if v.get("truncated").and_then(serde_json::Value::as_bool) == Some(true) {
            out.push_str("\n…（快照不完整：DOM 深度超 10 或交互元素超 300，仅显示前 300 个）");
        }
        Ok(out)
    }

    /// 点击 ref 对应元素（页面内注册表 + scrollIntoView + 原生 click）。
    pub async fn click(&mut self, r: u32) -> Result<()> {
        if r == 0 || r > self.ref_count {
            return Err(HamsterError::Other(format!(
                "ref {r} 不存在（当前快照共 {} 个交互元素，请先 browser_snapshot）",
                self.ref_count
            )));
        }
        let js = format!(
            "(() => {{ const el = window.__上游_els && window.__上游_els[{r} - 1]; \
               if (!el) return 'missing'; \
               el.scrollIntoView({{ block: 'center' }}); \
               el.click(); return 'ok'; }})()"
        );
        let r2 = self
            .tab
            .evaluate(js)
            .await
            .map_err(|e| HamsterError::Other(format!("点击执行失败：{e}")))?;
        let v: serde_json::Value = r2.into_value().unwrap_or(serde_json::Value::Null);
        if v.as_str() != Some("ok") {
            return Err(HamsterError::Other(format!(
                "点击失败：{}（可重新 snapshot 后重试）",
                v.as_str().unwrap_or("元素已失效")
            )));
        }
        // 点击可能触发路由跳转/懒加载，等页面重新稳定（无跳转时 ~300ms 即返回）
        self.wait_page_ready(1500).await;
        Ok(())
    }

    /// 向当前聚焦元素插入文本（CDP trusted 插入；先点击目标输入框聚焦）。
    /// `clear` = 先全选（ctrl+a）再插入，insertText 会替换选区，等效清空重填。
    pub async fn type_text(&mut self, text: &str, clear: bool) -> Result<()> {
        if clear {
            self.press_key("ctrl+a").await?;
        }
        let params = InsertTextParams::builder()
            .text(text)
            .build()
            .map_err(|e| HamsterError::Other(format!("输入参数错误：{e}")))?;
        self.tab
            .execute(params)
            .await
            .map_err(|e| HamsterError::Other(format!("输入失败：{e}")))?;
        Ok(())
    }

    /// 按键：Enter/Tab/…/Space 或组合键（ctrl+a、ctrl+shift+tab、shift+…）。
    pub async fn press_key(&mut self, key: &str) -> Result<()> {
        let press = parse_key(key)?;
        for (mkey, mcode, mvk) in &press.modifiers {
            self.dispatch_key(DispatchKeyEventType::RawKeyDown, mkey, mcode, *mvk, None)
                .await?;
        }
        self.dispatch_key(
            DispatchKeyEventType::RawKeyDown,
            &press.key,
            &press.code,
            press.vk,
            press.text.clone(),
        )
        .await?;
        self.dispatch_key(
            DispatchKeyEventType::KeyUp,
            &press.key,
            &press.code,
            press.vk,
            None,
        )
        .await?;
        for (mkey, mcode, mvk) in press.modifiers.iter().rev() {
            self.dispatch_key(DispatchKeyEventType::KeyUp, mkey, mcode, *mvk, None)
                .await?;
        }
        // Enter 常触发表单提交/路由跳转，等页面稳定；其余按键立即返回
        if press.key == "Enter" {
            self.wait_page_ready(1500).await;
        }
        Ok(())
    }

    async fn dispatch_key(
        &self,
        typ: DispatchKeyEventType,
        key: &str,
        code: &str,
        vk: i64,
        text: Option<String>,
    ) -> Result<()> {
        let mut builder = DispatchKeyEventParams::builder()
            .r#type(typ)
            .key(key.to_string())
            .code(code.to_string())
            .windows_virtual_key_code(vk);
        if let Some(t) = text {
            builder = builder.text(t);
        }
        let params = builder
            .build()
            .map_err(|e| HamsterError::Other(format!("按键参数错误：{e}")))?;
        self.tab
            .execute(params)
            .await
            .map_err(|e| HamsterError::Other(format!("按键失败：{e}")))?;
        Ok(())
    }

    /// 截图保存到指定路径（目录自动创建），返回 (绝对路径, "宽x高")。
    pub async fn screenshot(&mut self, path: &std::path::Path) -> Result<(PathBuf, String)> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| HamsterError::Other(format!("截图目录创建失败：{e}")))?;
        }
        let bytes = self
            .tab
            .screenshot(chromiumoxide::page::ScreenshotParams::default())
            .await
            .map_err(|e| HamsterError::Other(format!("截图失败：{e}")))?;
        let size = png_dimensions(&bytes)
            .map(|(w, h)| format!("{w}x{h}"))
            .unwrap_or_else(|| "未知尺寸".into());
        std::fs::write(path, bytes)
            .map_err(|e| HamsterError::Other(format!("截图写入失败：{e}")))?;
        Ok((path.to_path_buf(), size))
    }

    pub async fn close(mut self) {
        let _ = self.browser.close().await;
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(dir) = self.user_data_dir.take() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// 读 PNG 头部 IHDR 的宽高（Chromium 截图固定 PNG；解析失败返回 None）。
fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 24 || bytes[0..8] != SIG {
        return None;
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((w, h))
}

/// 修饰键 → (CDP key, code, Windows VK)。
fn modifier_spec(name: &str) -> Option<(String, String, i64)> {
    let (key, code, vk) = match name {
        "ctrl" | "control" => ("Control", "ControlLeft", 17),
        "shift" => ("Shift", "ShiftLeft", 16),
        "alt" => ("Alt", "AltLeft", 18),
        "meta" | "cmd" | "win" => ("Meta", "MetaLeft", 91),
        _ => return None,
    };
    Some((key.into(), code.into(), vk))
}

/// 解析后的按键：修饰键序列 + 基础键。
pub(crate) struct KeyPress {
    /// 有序修饰键（key, code, VK），按下/释放按此顺序、释放倒序
    pub modifiers: Vec<(String, String, i64)>,
    pub key: String,
    pub code: String,
    pub vk: i64,
    /// 随 down 事件下发的文本：普通字符为原样字符；带 Ctrl/Alt/Meta 时为 None
    pub text: Option<String>,
}

/// 按键描述解析（纯函数）：支持 "ctrl+a"、"ctrl+shift+tab" 组合；单个大写
/// 字母自动加 Shift；数字 code 用 Digit{n}（Chromium 约定）。"ctrl+" 等末尾
/// 空段表示 '+' 键本身。
pub(crate) fn parse_key(spec: &str) -> Result<KeyPress> {
    let s = spec.trim();
    // 字面空格键：" " 被 trim 后需还原；全空串才是非法输入
    let s = if s.is_empty() && !spec.is_empty() {
        " "
    } else {
        s
    };
    if s.is_empty() {
        return Err(HamsterError::config_invalid("按键名为空"));
    }
    let mut parts: Vec<&str> = s.split('+').collect();
    let base_raw = if parts.len() == 2 && parts[0].is_empty() && parts[1].is_empty() {
        parts.clear();
        "+" // 单独的 "+" 键
    } else {
        match parts.pop() {
            Some("") => {
                if parts.is_empty() {
                    parts.clear();
                }
                "+" // "ctrl+" 末尾空段 = '+' 键本身
            }
            Some(b) => b, // 已弹出基础键，剩余段全部是修饰键
            None => {
                return Err(HamsterError::config_invalid(format!(
                    "不支持的按键：{spec}"
                )))
            }
        }
    };
    let mut modifiers: Vec<(String, String, i64)> = Vec::new();
    for m in &parts {
        modifiers.push(
            modifier_spec(m)
                .ok_or_else(|| HamsterError::config_invalid(format!("不支持的按键：{spec}")))?,
        );
    }
    let lowered = base_raw.to_ascii_lowercase();
    let mut text: Option<String> = None;
    let (key, code, vk): (String, String, i64) = match lowered.as_str() {
        "enter" | "return" => {
            text = Some("\r".into());
            ("Enter".into(), "Enter".into(), 13)
        }
        "tab" => {
            text = Some("\t".into());
            ("Tab".into(), "Tab".into(), 9)
        }
        "escape" | "esc" => ("Escape".into(), "Escape".into(), 27),
        "backspace" => ("Backspace".into(), "Backspace".into(), 8),
        "delete" => ("Delete".into(), "Delete".into(), 46),
        "arrowup" | "up" => ("ArrowUp".into(), "ArrowUp".into(), 38),
        "arrowdown" | "down" => ("ArrowDown".into(), "ArrowDown".into(), 40),
        "arrowleft" | "left" => ("ArrowLeft".into(), "ArrowLeft".into(), 37),
        "arrowright" | "right" => ("ArrowRight".into(), "ArrowRight".into(), 39),
        "home" => ("Home".into(), "Home".into(), 36),
        "end" => ("End".into(), "End".into(), 35),
        "pageup" => ("PageUp".into(), "PageUp".into(), 33),
        "pagedown" => ("PageDown".into(), "PageDown".into(), 34),
        "space" => {
            text = Some(" ".into());
            (" ".into(), "Space".into(), 32)
        }
        other => {
            let mut chars = other.chars();
            let c = match (chars.next(), chars.next()) {
                (Some(c), None) => c,
                _ => {
                    return Err(HamsterError::config_invalid(format!(
                        "不支持的按键：{spec}"
                    )))
                }
            };
            let orig = base_raw.chars().next().unwrap_or(c);
            if orig.is_ascii_uppercase() {
                // 大写字母 = shift + 字母
                modifiers.push(("Shift".into(), "ShiftLeft".into(), 16));
            }
            let upper = c.to_ascii_uppercase();
            let code = if c.is_ascii_digit() {
                format!("Digit{c}")
            } else {
                format!("Key{upper}")
            };
            text = Some(orig.to_string());
            (upper.to_string(), code, upper as i64)
        }
    };
    // 组合键的 text 语义：仅 Shift 保留（生成大写字符）；带 Ctrl/Alt/Meta
    // 不下发电文本，避免触发输入
    if !modifiers.is_empty() {
        if modifiers.iter().any(|(k, _, _)| k != "Shift") {
            text = None;
        } else if let Some(c) = text.take().and_then(|t| t.chars().next()) {
            text = Some(c.to_ascii_uppercase().to_string());
        }
    }
    Ok(KeyPress {
        modifiers,
        key,
        code,
        vk,
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_key_named_and_aliases() {
        let p = parse_key("enter").unwrap();
        assert_eq!(
            (p.key.as_str(), p.vk, p.text.as_deref()),
            ("Enter", 13, Some("\r"))
        );
        assert!(p.modifiers.is_empty());

        assert_eq!(parse_key("up").unwrap().key, "ArrowUp");
        assert_eq!(parse_key("esc").unwrap().key, "Escape");
        // 字面空格：" " 与 "space" 等价
        assert_eq!(parse_key(" ").unwrap().key, " ");
        assert_eq!(parse_key("space").unwrap().key, " ");
    }

    #[test]
    fn parse_key_single_char_case_and_digits() {
        let p = parse_key("a").unwrap();
        assert_eq!((p.key.as_str(), p.code.as_str(), p.vk), ("A", "KeyA", 65));
        assert_eq!(p.text.as_deref(), Some("a"));
        assert!(p.modifiers.is_empty());

        // 大写字母自动加 Shift，text 保留大写
        let p = parse_key("A").unwrap();
        assert_eq!(p.text.as_deref(), Some("A"));
        assert_eq!(p.modifiers.len(), 1);
        assert_eq!(p.modifiers[0].0, "Shift");

        // 数字用 Chromium 的 Digit{n} code
        let p = parse_key("5").unwrap();
        assert_eq!(
            (p.code.as_str(), p.vk, p.text.as_deref()),
            ("Digit5", 53, Some("5"))
        );
    }

    #[test]
    fn parse_key_combos() {
        let p = parse_key("ctrl+a").unwrap();
        assert_eq!(p.modifiers.len(), 1);
        assert_eq!(p.modifiers[0].0, "Control");
        assert_eq!(p.key, "A");
        assert_eq!(p.text, None, "带 Ctrl 的组合不下发电文本");

        let p = parse_key("ctrl+shift+Tab").unwrap();
        assert_eq!(
            p.modifiers.iter().map(|m| m.0.as_str()).collect::<Vec<_>>(),
            vec!["Control", "Shift"]
        );
        assert_eq!(p.key, "Tab");

        // 仅 Shift 保留 text 且转为大写
        let p = parse_key("shift+a").unwrap();
        assert_eq!(p.text.as_deref(), Some("A"));

        // "ctrl+" 末尾空段 = '+' 键本身
        let p = parse_key("ctrl+").unwrap();
        assert_eq!(p.key, "+");
        assert_eq!(p.modifiers.len(), 1);

        assert_eq!(parse_key("+").unwrap().key, "+");
    }

    #[test]
    fn parse_key_rejects_invalid() {
        assert!(parse_key("").is_err());
        // 纯空白视为空格键（与 " " 一致），而非报错
        assert_eq!(parse_key("   ").unwrap().key, " ");
        assert!(parse_key("ctrl").is_err(), "只有修饰键没有基础键");
        assert!(parse_key("f5").is_err(), "功能键未支持，应显式报错");
        assert!(parse_key("ctrl+f5").is_err());
    }

    #[test]
    fn png_dimensions_parses_ihdr() {
        let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        b.extend_from_slice(&13u32.to_be_bytes());
        b.extend_from_slice(b"IHDR");
        b.extend_from_slice(&800u32.to_be_bytes());
        b.extend_from_slice(&600u32.to_be_bytes());
        b.extend_from_slice(&[0u8; 5]);
        assert_eq!(png_dimensions(&b), Some((800, 600)));

        assert_eq!(png_dimensions(&b[..20]), None, "不足 24 字节");
        assert_eq!(png_dimensions(b"not a png at all...."), None, "签名不符");
    }
}
