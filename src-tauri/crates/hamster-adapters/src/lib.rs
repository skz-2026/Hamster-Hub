//! 适配层：每个 Agent 一个文件。
//!
//! 结构约定（AGENTS.md §6 / architecture §5）：
//! 路径常量 → detect → read_mcp → render_mcp → 其余 → fixture 往返测试。
//! Adapter 是纯函数层：不做任何写入 IO，目标文件现状由引擎传入。

pub mod acp;
pub mod claude;
pub mod codex;
pub mod gemini;
pub mod jsonutil;
pub mod session_skill;
pub mod zcode;

use std::path::Path;

use hamster_core::Registry;

use crate::acp::{AcpAgentAdapter, SPECS};
use crate::claude::ClaudeAdapter;
use crate::codex::CodexAdapter;
use crate::gemini::GeminiAdapter;
use crate::zcode::ZcodeAdapter;

/// 适配器注册表。
///
/// 两类接入形态：
/// - 专设适配器（claude/codex/gemini/zcode）：完整接管配置面 + 运行时面
/// - `acp::SPECS` 通道型适配器：原生支持 ACP 的 Agent，只接结构化对话通道
///   （新增 = `acp.rs` 表内加一项，无需新文件；docs/runtime-design.md §11.5）
pub fn build_registry(home: &Path) -> Registry {
    let mut adapters: Vec<Box<dyn hamster_core::adapter::AgentAdapter>> = vec![
        Box::new(ClaudeAdapter::new(home)),
        Box::new(CodexAdapter::new(home)),
        Box::new(GeminiAdapter::new(home)),
        Box::new(ZcodeAdapter::new(home)),
    ];
    adapters.extend(SPECS.iter().map(|s| {
        Box::new(AcpAgentAdapter::new(home, s)) as Box<dyn hamster_core::adapter::AgentAdapter>
    }));
    Registry::new(adapters)
}

/// npm 全局 bin 目录（Windows = %APPDATA%\npm）。取不到 → None。
fn npm_global_bin_dir() -> Option<std::path::PathBuf> {
    if cfg!(windows) {
        let dir = std::path::PathBuf::from(std::env::var_os("APPDATA")?).join("npm");
        dir.is_dir().then_some(dir)
    } else {
        None // 非 Windows：全局 npm bin 通常已在 PATH，交给 PATH 解析
    }
}

/// 候选可执行名（Windows：npm shim 优先 .cmd，再 .exe / 裸名）。
fn program_candidates(name: &str) -> Vec<String> {
    if cfg!(windows) {
        vec![
            format!("{name}.cmd"),
            format!("{name}.exe"),
            name.to_string(),
        ]
    } else {
        vec![name.to_string()]
    }
}

/// 收集某 CLI 在本机的全部候选路径：npm 全局 bin 优先，其余按 PATH 顺序，
/// 规范化去重。不运行任何外部命令。
pub(crate) fn find_programs(name: &str) -> Vec<std::path::PathBuf> {
    let mut out: Vec<std::path::PathBuf> = Vec::new();
    let mut push = |p: std::path::PathBuf| {
        let key = p.canonicalize().unwrap_or(p.clone());
        if !out
            .iter()
            .any(|x| x.canonicalize().unwrap_or(x.clone()) == key)
        {
            out.push(p);
        }
    };
    if let Some(dir) = npm_global_bin_dir() {
        for c in program_candidates(name) {
            let p = dir.join(&c);
            if p.is_file() {
                push(p);
            }
        }
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            for c in program_candidates(name) {
                let p = dir.join(&c);
                if p.is_file() {
                    push(p);
                }
            }
        }
    }
    out
}

/// 版本探测结果的缓存条目（TTL 内不重复 spawn `--version`）。
const BEST_TTL: std::time::Duration = std::time::Duration::from_secs(600);

/// 探测缓存：路径 → (探测时刻, 结果)
type BestCache = std::sync::Mutex<
    std::collections::HashMap<String, (std::time::Instant, Option<std::path::PathBuf>)>,
>;

fn best_cache() -> &'static BestCache {
    static CACHE: std::sync::OnceLock<BestCache> = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// 解析某 CLI 的最佳可执行：单候选直接用（不 spawn）；多候选逐个探测
/// `--version` 取最高版本（同版本/探测失败保持原序 = npm 全局优先），
/// 结果缓存 BEST_TTL。探测是唯一会运行外部命令的路径。
pub(crate) fn resolve_best_program(name: &str) -> Option<std::path::PathBuf> {
    let candidates = find_programs(name);
    if candidates.len() <= 1 {
        return candidates.into_iter().next();
    }
    if let Some((at, hit)) = best_cache().lock().ok().and_then(|c| c.get(name).cloned()) {
        if at.elapsed() < BEST_TTL {
            return hit;
        }
    }
    let best = pick_best_by_version(candidates, probe_cli_version);
    if let Ok(mut c) = best_cache().lock() {
        c.insert(name.to_string(), (std::time::Instant::now(), best.clone()));
    }
    best
}

/// 多候选取最高版本（probe 可注入）：有版本的候选按版本降序、版本相同按
/// 原序（npm 全局的受管理副本优先于 PATH 上的陈旧副本）；全部探测失败
/// → 首个候选。
fn pick_best_by_version(
    candidates: Vec<std::path::PathBuf>,
    probe: impl Fn(&std::path::Path) -> Option<String>,
) -> Option<std::path::PathBuf> {
    use std::cmp::Ordering;
    candidates
        .iter()
        .enumerate()
        .map(|(i, p)| (probe(p).and_then(|s| Version::parse(&s)), i, p))
        .min_by(|a, b| match (&a.0, &b.0) {
            (Some(x), Some(y)) => y.cmp(x).then(a.1.cmp(&b.1)),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => a.1.cmp(&b.1),
        })
        .map(|(_, _, p)| p.clone())
}

/// 宽松 semver：取输出里第一段 x.y[.z]（不足补 0，多余忽略）。
#[derive(Clone, Debug, PartialEq, Eq)]
struct Version(u64, u64, u64);

impl Version {
    /// 从任意文本提取首个版本号（如 "claude 2.1.251 (Claude Code)" → 2.1.251）
    fn parse(text: &str) -> Option<Version> {
        for token in text.split_whitespace() {
            let t = token.trim_start_matches('v');
            let nums: Vec<u64> = t
                .split(|c: char| c == '.' || !c.is_ascii_digit())
                .filter(|s| !s.is_empty())
                .filter_map(|s| s.parse().ok())
                .collect();
            if !nums.is_empty() {
                let g = |i: usize| nums.get(i).copied().unwrap_or(0);
                return Some(Version(g(0), g(1), g(2)));
            }
        }
        None
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.0, self.1, self.2).cmp(&(other.0, other.1, other.2))
    }
}

/// `--version` 探测：spawn + 轮询等待（超时强杀），取 stdout/stderr 首行文本。
fn probe_cli_version(path: &std::path::Path) -> Option<String> {
    use std::io::Read;
    use std::process::Command;
    use std::time::{Duration, Instant};

    const DEADLINE: Duration = Duration::from_secs(4);
    let mut cmd = Command::new(path);
    cmd.arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    // CLI 是控制台程序：GUI 主进程探测时会闪黑框，CREATE_NO_WINDOW 压住
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = cmd.spawn().ok()?;
    let deadline = Instant::now() + DEADLINE;
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            // 超时（如 TUI 形态的 CLI 不吃 --version，进了交互界面）：强杀不等待
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    let mut buf = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_string(&mut buf);
    }
    if buf.is_empty() {
        if let Some(mut err) = child.stderr.take() {
            let _ = err.read_to_string(&mut buf);
        }
    }
    if !status.success() && buf.is_empty() {
        return None;
    }
    Some(buf)
}

/// 启动程序解析：最佳可执行（用户场景：多版本默认取最高），解析失败回退裸名交 PATH。
/// 背景：机器上常有多份同名 CLI（第三方工具自带的旧版可能排在 PATH 前列），
/// 旧版读不了新版 rollout（实测：codex 0.144.5 resume 0.152.0 会话报
/// "paginated_threads is not supported yet"），故优先受管理的全局安装
/// （find_programs 的候选序），多版本时再按版本择优。
pub(crate) fn resolve_program(name: &str) -> String {
    resolve_best_program(name)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn unknown_tool_falls_back_to_path() {
        // npm 全局不可能有这个名字：必须回退裸名（PATH 解析）
        assert_eq!(
            resolve_program("hamster-no-such-tool"),
            "hamster-no-such-tool"
        );
    }

    fn probe_of<'a>(versions: &'a [(&'a str, &'a str)]) -> impl Fn(&Path) -> Option<String> + 'a {
        let map: std::collections::HashMap<String, String> = versions
            .iter()
            .map(|(p, v)| (p.to_string(), v.to_string()))
            .collect();
        move |p: &Path| p.to_str().and_then(|key| map.get(key).cloned())
    }

    /// 多候选：取版本最高者（与候选顺序无关）。
    #[test]
    fn pick_best_prefers_highest_version() {
        let candidates = vec![PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")];
        let probe = probe_of(&[("a", "1.2.0"), ("b", "10.0.1"), ("c", "2.9.9")]);
        assert_eq!(
            pick_best_by_version(candidates, probe),
            Some(PathBuf::from("b"))
        );
    }

    /// 同版本：保持候选原序（npm 全局的受管理副本优先）。
    #[test]
    fn pick_best_keeps_order_on_tie() {
        let candidates = vec![PathBuf::from("npm-first"), PathBuf::from("path-second")];
        let probe = probe_of(&[("npm-first", "0.152.0"), ("path-second", "0.152.0")]);
        assert_eq!(
            pick_best_by_version(candidates, probe),
            Some(PathBuf::from("npm-first"))
        );
    }

    /// 探测全失败：回退首个候选（不因版本探测把可用 CLI 弄丢）。
    #[test]
    fn pick_best_falls_back_to_first_when_unprobeable() {
        let candidates = vec![PathBuf::from("x"), PathBuf::from("y")];
        assert_eq!(
            pick_best_by_version(candidates, probe_of(&[])),
            Some(PathBuf::from("x"))
        );
    }

    /// 版本号提取：官方 `--version` 输出的常见形态 + v 前缀 + 两段式。
    #[test]
    fn version_parse_handles_common_outputs() {
        assert_eq!(
            Version::parse("claude 2.1.251 (Claude Code)"),
            Some(Version(2, 1, 251))
        );
        assert_eq!(Version::parse("v0.152.0"), Some(Version(0, 152, 0)));
        assert_eq!(Version::parse("1.18"), Some(Version(1, 18, 0)));
        assert_eq!(Version::parse("no version here"), None);
    }

    /// 位数不齐的版本比较（1.10 > 1.9）：逐段数值比较，不是字符串比较。
    #[test]
    fn version_compare_is_numeric() {
        assert!(Version(1, 10, 0) > Version(1, 9, 99));
    }
}
