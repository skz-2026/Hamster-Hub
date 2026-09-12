//! 启动器：按 CommandSpec 拉起终端运行 Agent CLI。
//!
//! Windows 优先 Windows Terminal（wt），退化到 `cmd /k`；都通过分离进程启动，
//! 上游 自身不保持子进程句柄。

use std::process::Command;

use crate::adapter::CommandSpec;
use crate::error::{HamsterError, Result};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn launch(spec: &CommandSpec) -> Result<()> {
    let dir = &spec.working_dir;
    if !dir.is_dir() {
        return Err(HamsterError::Launch {
            program: spec.program.clone(),
            message: format!("项目目录不存在：{}", dir.display()),
        });
    }

    #[cfg(windows)]
    {
        if let Some(wt) = find_windows_terminal() {
            let mut cmd = Command::new(wt);
            cmd.arg("-d")
                .arg(dir)
                .arg("cmd")
                .arg("/k")
                .arg(&spec.program);
            for arg in &spec.args {
                cmd.arg(arg);
            }
            return spawn_detached(cmd, &spec.program);
        }
        let mut cmd = Command::new("cmd");
        cmd.arg("/C")
            .arg("start")
            .arg("")
            .arg("/D")
            .arg(dir)
            .arg("cmd")
            .arg("/k")
            .arg(&spec.program);
        for arg in &spec.args {
            cmd.arg(arg);
        }
        spawn_detached(cmd, &spec.program)
    }

    #[cfg(not(windows))]
    {
        Command::new(&spec.program)
            .args(&spec.args)
            .current_dir(dir)
            .spawn()
            .map_err(|e| HamsterError::Launch {
                program: spec.program.clone(),
                message: e.to_string(),
            })?;
        Ok(())
    }
}

#[cfg(windows)]
fn spawn_detached(mut cmd: Command, program: &str) -> Result<()> {
    cmd.creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| HamsterError::Launch {
            program: program.to_string(),
            message: e.to_string(),
        })?;
    Ok(())
}

#[cfg(windows)]
fn find_windows_terminal() -> Option<std::path::PathBuf> {
    // wt.exe 是 WindowsApps 的执行别名，不在常规 PATH 目录里逐个出现，
    // 但 %LOCALAPPDATA%\Microsoft\WindowsApps 一定在 PATH 中。
    let local = std::env::var("LOCALAPPDATA").ok()?;
    let candidate = std::path::PathBuf::from(local)
        .join("Microsoft")
        .join("WindowsApps")
        .join("wt.exe");
    candidate.exists().then_some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn launch_rejects_missing_dir() {
        let spec = CommandSpec {
            program: "claude".into(),
            args: vec![],
            working_dir: PathBuf::from("Z:/definitely/not/exist"),
        };
        let err = launch(&spec).unwrap_err();
        assert!(matches!(err, HamsterError::Launch { .. }));
    }
}
