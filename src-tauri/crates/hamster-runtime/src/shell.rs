//! 工作台通用终端的 shell 选择（纯函数 + PATH 探测）。
//!
//! 优先现代 shell（pwsh / $SHELL），按可用性回退——与 Agent 会话同一 PTY
//! 宿主（runtime-design §3），仅 program/args 不同，零额外机制。

use crate::spawn::SpawnPlan;

/// Windows 交互 shell 的优先顺序（pwsh → powershell → cmd）。
const WINDOWS_SHELLS: &[&str] = &["pwsh.exe", "powershell.exe", "cmd.exe"];

/// 交互式登录参数：各 shell 让 PTY 进入正常提示符所需的最小参数。
pub fn shell_plan(is_windows: bool) -> SpawnPlan {
    if is_windows {
        for program in WINDOWS_SHELLS {
            if in_path(program) {
                return SpawnPlan {
                    program: (*program).to_string(),
                    args: Vec::new(),
                };
            }
        }
        // PATH 探测失败也交由 spawn 报错（powershell 在 Win11 必在系统 PATH）
        SpawnPlan {
            program: "powershell.exe".to_string(),
            args: Vec::new(),
        }
    } else {
        let program = std::env::var("SHELL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "/bin/bash".to_string());
        SpawnPlan {
            program,
            args: vec!["-l".to_string()],
        }
    }
}

/// 在 PATH 各条目下探测可执行文件（Windows 需补 .exe 由调用方保证）。
fn in_path(program: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(program).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_picks_first_available() {
        let plan = shell_plan(true);
        assert!(
            WINDOWS_SHELLS.contains(&plan.program.as_str()),
            "必须在候选序列内，实际选了 {}",
            plan.program
        );
        assert!(plan.args.is_empty(), "Windows shell 无需登录参数");
    }

    #[test]
    fn unix_uses_shell_env_or_bash() {
        let plan = shell_plan(false);
        assert!(!plan.program.is_empty());
        assert_eq!(plan.args, vec!["-l".to_string()]);
    }
}
