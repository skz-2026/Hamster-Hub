//! Git 工作台引擎：经官方 `git` CLI 读写用户仓库（status/diff/stage/commit/log/
//! pull/push/discard）。
//!
//! 边界约定：
//! - 只调用 git 官方 CLI（PATH 解析），不解析 .git 内部结构
//! - 仓库判断用 `rev-parse --is-inside-work-tree`；非仓库返回 `Ok(None)`（UI 引导）
//! - 写操作（stage/unstage/commit/pull/push/discard）作用于用户自己的代码仓库，
//!   git 本身提供完整可恢复性（reflog/对象库；discard 除外，前端二次确认），
//!   不属于 Agent 配置写入面，不走 SyncEngine 快照
//! - porcelain 输出一律 `-z`（NUL 分隔）+ `core.quotepath=false`（非 ASCII 路径不转义）

use std::path::Path;
use std::process::Command;

use crate::error::{MoltoError, Result};
use crate::model::git::{GitFileStatus, GitLogEntry, GitStatus};

/// 统一构造 git 调用：`git -C <dir> -c core.quotepath=false <args...>`。
fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("-c")
        .arg("core.quotepath=false")
        .args(args)
        .output()
        .map_err(|e| MoltoError::Other(format!("无法启动 git（请确认已安装并在 PATH）：{e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(MoltoError::Other(format!(
            "git {} 失败：{}",
            args.first().unwrap_or(&""),
            stderr
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// 是否为 git 工作区（false = 前端展示「非 git 仓库」引导）。
pub fn is_repo(dir: &Path) -> Result<bool> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
        .map_err(|e| MoltoError::Other(format!("无法启动 git（请确认已安装并在 PATH）：{e}")))?;
    if !out.status.success() {
        return Ok(false);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim() == "true")
}

/// 仓库状态：分支 + ahead/behind + 文件清单（porcelain v1 -z --branch）。
pub fn status(dir: &Path) -> Result<GitStatus> {
    let raw = git(dir, &["status", "--porcelain=v1", "-z", "--branch"])?;
    Ok(parse_status(&raw))
}

/// 解析 `status --porcelain=v1 -z --branch` 输出（纯函数，供测试）。
pub fn parse_status(raw: &str) -> GitStatus {
    let mut out = GitStatus::default();
    for field in raw.split('\0') {
        if field.is_empty() {
            continue;
        }
        if let Some(rest) = field.strip_prefix("## ") {
            // 形如 `main...origin/main [ahead 1, behind 2]` 或 `HEAD (no branch)`
            let head = rest.split_whitespace().next().unwrap_or("");
            if !head.starts_with("HEAD") {
                out.branch = Some(head.split("...").next().unwrap_or(head).to_string());
            }
            if let Some(bracket) = rest[rest.find('[').unwrap_or(rest.len())..].strip_prefix('[') {
                for part in bracket.trim_end_matches(']').split(", ") {
                    let mut kv = part.splitn(2, ' ');
                    match (kv.next(), kv.next()) {
                        (Some("ahead"), Some(n)) => {
                            out.ahead = n.parse().unwrap_or(0);
                        }
                        (Some("behind"), Some(n)) => {
                            out.behind = n.parse().unwrap_or(0);
                        }
                        _ => {}
                    }
                }
            }
            continue;
        }
        // 文件行：`XY path`；重命名 `XY new -> old`。X=index，Y=worktree。
        if field.len() < 4 {
            continue;
        }
        let xy = &field[..2];
        let path = field[3..].to_string();
        out.files.push(GitFileStatus {
            path,
            staged: xy.chars().next().unwrap_or(' ').to_string(),
            worktree: xy.chars().nth(1).unwrap_or(' ').to_string(),
        });
    }
    out
}

/// 单文件统一 diff（staged=true 看 index vs HEAD，false 看工作区 vs index）。
/// 未跟踪文件 `git diff` 为空：改用 `--no-index /dev/null <path>` 生成全文新增
/// （--no-index 有差异时退出码为 1，属正常，不视为错误）。
pub fn diff(dir: &Path, path: &str, staged: bool) -> Result<String> {
    if !staged && is_untracked(dir, path)? {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .arg("-c")
            .arg("core.quotepath=false")
            .args(["diff", "--no-index", "--unified=3", "--", "/dev/null", path])
            .output()
            .map_err(|e| MoltoError::Other(format!("无法启动 git：{e}")))?;
        if out.status.code() == Some(1) || out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
        }
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(MoltoError::Other(format!("git diff 失败：{stderr}")));
    }
    let mut args = vec!["diff", "--unified=3"];
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(path);
    git(dir, &args)
}

/// 路径是否未跟踪（porcelain 输出以 `??` 开头）。
fn is_untracked(dir: &Path, path: &str) -> Result<bool> {
    let out = git(dir, &["status", "--porcelain", "--", path])?;
    Ok(out.starts_with("??"))
}

/// 暂存文件（`git add --`）。untracked 文件同样适用。
pub fn stage(dir: &Path, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut args = vec!["add", "--"];
    args.extend(paths.iter().map(String::as_str));
    git(dir, &args).map(|_| ())
}

/// 取消暂存（`git reset -q HEAD --`）：index 侧改动退回工作区侧。
pub fn unstage(dir: &Path, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut args = vec!["reset", "-q", "HEAD", "--"];
    args.extend(paths.iter().map(String::as_str));
    git(dir, &args).map(|_| ())
}

/// 提交已暂存内容。无暂存内容时由 git 报错（前端先校验并禁用按钮）。
pub fn commit(dir: &Path, message: &str) -> Result<String> {
    let msg = message.trim();
    if msg.is_empty() {
        return Err(MoltoError::config_invalid("提交信息不能为空"));
    }
    git(dir, &["commit", "-m", msg])
}

/// 拉取远端更新（`git pull`，合并/变基策略随用户 git 配置；冲突时原样报错）。
pub fn pull(dir: &Path) -> Result<String> {
    git(dir, &["pull"])
}

/// 推送当前分支（`git push -u origin HEAD`）：首推同名的远端分支并建立
/// upstream 跟踪；无 origin 远端时由 git 报错（前端原样提示）。
pub fn push(dir: &Path) -> Result<String> {
    git(dir, &["push", "-u", "origin", "HEAD"])
}

/// 丢弃文件的未提交改动（`git checkout -q [--|HEAD] -- <path>`，不可恢复）。
/// `staged = true` 连 index 一起回到 HEAD（用于已暂存组）；false 只把工作区
/// 还原到 index（用于未暂存组）。未跟踪文件两个基线里都不存在，由 git 报错
/// （前端对 untracked 不提供丢弃入口）。
pub fn discard(dir: &Path, paths: &[String], staged: bool) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut args = vec!["checkout", "-q"];
    if staged {
        args.push("HEAD");
    }
    args.push("--");
    args.extend(paths.iter().map(String::as_str));
    git(dir, &args).map(|_| ())
}

/// 最近提交历史（`--pretty` 用 \x1f 分隔避免与内容冲突）。
pub fn log(dir: &Path, limit: usize) -> Result<Vec<GitLogEntry>> {
    let raw = git(
        dir,
        &[
            "log",
            &format!("-{limit}"),
            "--pretty=format:%h\x1f%s\x1f%an\x1f%at",
        ],
    )?;
    Ok(raw
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\x1f');
            Some(GitLogEntry {
                hash: parts.next()?.to_string(),
                subject: parts.next()?.to_string(),
                author: parts.next()?.to_string(),
                at: parts.next()?.parse().unwrap_or(0),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 解析是纯函数：直接用真实 git 输出样例断言（含中文路径/重命名/ahead-behind）。
    /// 注意 porcelain 的 XY 前缀含前导空格，用 join 构造避免行续吞空格。
    #[test]
    fn parse_status_branch_files_and_tracking() {
        let raw = [
            "## main...origin/main [ahead 1, behind 2]",
            "M  src/main.rs",
            " M docs/说明.md",
            "A  new.txt",
            "?? untracked.log",
            "R  renamed.ts",
        ]
        .join("\0");
        let st = parse_status(&raw);
        assert_eq!(st.branch.as_deref(), Some("main"));
        assert_eq!(st.ahead, 1);
        assert_eq!(st.behind, 2);
        assert_eq!(st.files.len(), 5);
        assert_eq!(st.files[0].path, "src/main.rs");
        assert_eq!(st.files[0].staged, "M");
        assert_eq!(st.files[0].worktree, " ");
        assert_eq!(st.files[1].staged, " ");
        assert_eq!(st.files[1].worktree, "M");
        assert_eq!(st.files[1].path, "docs/说明.md", "非 ASCII 路径原样保留");
        assert_eq!(st.files[3].staged, "?");
        assert!(st.has_staged());
    }

    #[test]
    fn parse_status_detached_and_clean() {
        let st = parse_status("## HEAD (no branch)\0");
        assert!(st.branch.is_none());
        assert!(!st.has_staged());
        let clean = parse_status("## main...origin/main\0");
        assert_eq!(clean.branch.as_deref(), Some("main"));
        assert!(clean.files.is_empty());
    }

    /// 端到端（需要 git 在 PATH）：临时仓库走一遍 status→stage→commit→log。
    #[test]
    fn status_stage_commit_log_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        if !Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return; // 环境无 git：只跑纯函数用例
        }
        let d = dir.path();
        assert!(Command::new("git")
            .arg("-C")
            .arg(d)
            .arg("init")
            .output()
            .unwrap()
            .status
            .success());
        std::fs::write(d.join("a.txt"), "hello\n").unwrap();

        assert!(is_repo(d).unwrap());
        let st = status(d).unwrap();
        assert_eq!(st.files.len(), 1);
        assert_eq!(st.files[0].staged, "?");
        assert!(!st.has_staged());

        stage(d, &["a.txt".into()]).unwrap();
        let st = status(d).unwrap();
        assert!(st.has_staged());
        assert_eq!(st.files[0].staged, "A");

        unstage(d, &["a.txt".into()]).unwrap();
        assert!(!status(d).unwrap().has_staged());
        stage(d, &["a.txt".into()]).unwrap();

        // 无 user.name/email 的环境（CI）需要临时身份
        for (k, v) in [("user.name", "t"), ("user.email", "t@t")] {
            Command::new("git")
                .arg("-C")
                .arg(d)
                .args(["config", k, v])
                .output()
                .unwrap();
        }
        commit(d, "init commit").unwrap();
        let log = log(d, 5).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].subject, "init commit");

        std::fs::write(d.join("a.txt"), "hello mundo\n").unwrap();
        let diff_text = diff(d, "a.txt", false).unwrap();
        assert!(diff_text.contains("-hello"));
        assert!(diff_text.contains("+hello mundo"));
        assert!(status(d).unwrap().files[0].worktree == "M");

        // 未跟踪文件：全文新增 diff（--no-index 退出码 1 属正常）
        std::fs::write(d.join("new.txt"), "brand new\n").unwrap();
        let new_diff = diff(d, "new.txt", false).unwrap();
        assert!(
            new_diff.contains("+brand new"),
            "untracked 应生成全文新增 diff"
        );
    }

    #[test]
    fn non_repo_is_none_not_error() {
        let dir = tempfile::tempdir().unwrap();
        if !Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return;
        }
        assert!(!is_repo(dir.path()).unwrap(), "空目录不是 git 工作区");
    }

    /// discard：工作区侧只还原工作区（保留暂存），暂存侧连 index 回 HEAD。
    #[test]
    fn discard_restores_worktree_and_head() {
        let dir = tempfile::tempdir().unwrap();
        if !Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return;
        }
        let d = dir.path();
        let run = |args: &[&str]| {
            assert!(Command::new("git")
                .arg("-C")
                .arg(d)
                .args(args)
                .output()
                .unwrap()
                .status
                .success());
        };
        run(&["init"]);
        // Windows 全局 autocrlf 会让 checkout 还原出 CRLF：测试仓库内关掉保证内容可比
        for (k, v) in [
            ("user.name", "t"),
            ("user.email", "t@t"),
            ("core.autocrlf", "false"),
        ] {
            run(&["config", k, v]);
        }
        std::fs::write(d.join("a.txt"), "base\n").unwrap();
        run(&["add", "."]);
        run(&["commit", "-m", "base"]);

        // 工作区改动 + 暂存另一份改动
        std::fs::write(d.join("a.txt"), "work\n").unwrap();
        let st = status(d).unwrap();
        discard(d, &[st.files[0].path.clone()], false).unwrap();
        assert_eq!(
            std::fs::read_to_string(d.join("a.txt")).unwrap(),
            "base\n",
            "工作区丢弃后回到 index 内容"
        );

        std::fs::write(d.join("a.txt"), "staged work\n").unwrap();
        run(&["add", "."]);
        let st = status(d).unwrap();
        assert_eq!(st.files[0].staged, "M");
        discard(d, &[st.files[0].path.clone()], true).unwrap();
        assert!(
            status(d).unwrap().files.is_empty(),
            "暂存侧丢弃后工作区与 index 均回到 HEAD"
        );
        assert_eq!(std::fs::read_to_string(d.join("a.txt")).unwrap(), "base\n");
    }

    /// pull/push：本地 bare 远端两端克隆，B 推送后 A 拉取可见（无网络依赖）。
    #[test]
    fn pull_push_roundtrip_via_local_remote() {
        if !Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return;
        }
        let root = tempfile::tempdir().unwrap();
        let remote = root.path().join("origin.git");
        let a = root.path().join("a");
        let b = root.path().join("b");
        let run_in = |dir: &Path, args: &[&str]| {
            assert!(
                Command::new("git")
                    .arg("-C")
                    .arg(dir)
                    .args(args)
                    .output()
                    .unwrap()
                    .status
                    .success(),
                "git {args:?} 应成功"
            );
        };
        let config_identity = |dir: &Path| {
            // Windows 全局 autocrlf 会让 pull 下来的文件带 CRLF：测试仓库内关掉
            for (k, v) in [
                ("user.name", "t"),
                ("user.email", "t@t"),
                ("core.autocrlf", "false"),
            ] {
                run_in(dir, &["config", k, v]);
            }
        };
        assert!(Command::new("git")
            .arg("init")
            .arg("--bare")
            .arg(&remote)
            .output()
            .unwrap()
            .status
            .success());
        // A：克隆空远端 → 首个提交 → push -u origin HEAD（首推建跟踪）
        assert!(Command::new("git")
            .arg("clone")
            .arg(&remote)
            .arg(&a)
            .output()
            .unwrap()
            .status
            .success());
        config_identity(&a);
        std::fs::write(a.join("a.txt"), "one\n").unwrap();
        run_in(&a, &["add", "."]);
        run_in(&a, &["commit", "-m", "from-a"]);
        push(&a).unwrap();
        // B：克隆 → 新提交 → push
        assert!(Command::new("git")
            .arg("clone")
            .arg(&remote)
            .arg(&b)
            .output()
            .unwrap()
            .status
            .success());
        config_identity(&b);
        std::fs::write(b.join("b.txt"), "two\n").unwrap();
        run_in(&b, &["add", "."]);
        run_in(&b, &["commit", "-m", "from-b"]);
        push(&b).unwrap();
        // A：pull 可见 B 的提交
        pull(&a).unwrap();
        assert_eq!(std::fs::read_to_string(a.join("b.txt")).unwrap(), "two\n");
        let entries = log(&a, 10).unwrap();
        assert!(
            entries.iter().any(|e| e.subject == "from-b"),
            "pull 后本地历史应包含远端提交"
        );
    }
}
