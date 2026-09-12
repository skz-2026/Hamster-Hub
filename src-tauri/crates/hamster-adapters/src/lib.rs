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

/// 启动程序解析：上游 安装渠道（npm 全局）的副本优先，回退裸名交 PATH。
/// 背景：机器上常有多份同名 CLI（第三方工具自带的旧版可能排在 PATH 前列），
/// 旧版读不了新版 rollout（实测：codex 0.144.5 resume 0.152.0 会话报
/// "paginated_threads is not supported yet"），故固定用受管理的全局安装。
pub(crate) fn resolve_program(name: &str) -> String {
    npm_global_bin_dir()
        .map(|dir| dir.join(format!("{name}.cmd")))
        .filter(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_tool_falls_back_to_path() {
        // npm 全局不可能有这个名字：必须回退裸名（PATH 解析）
        assert_eq!(
            resolve_program("hamster-no-such-tool"),
            "hamster-no-such-tool"
        );
    }
}
