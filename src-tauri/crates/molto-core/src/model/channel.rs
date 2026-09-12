//! Channel 域模型（v2.1）：GitHub issue 跟单的状态机与配置面。
//!
//! Channel 是第三个「宿主零侵入」交互面：监控 issue → 本地 worktree 隔离开发 →
//! 官方 headless Agent 一次性任务 → PR/评论反馈。设计见 docs/channel-design.md。

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Channel 配置（单一 GitHub 仓库源；M1 一个 channel，M3 再抽象多源）。
/// token 只存本机 store_root/channel/config.json（红线④本地优先）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChannelConfig {
    pub owner: String,
    pub repo: String,
    /// 工作基线分支（worktree 从这里切出；PR 也回这里）
    pub base_branch: String,
    /// 触发标签：只跟单打了该 label 的 issue（空 = 全部 open issue）
    pub label: String,
    /// GitHub PAT（repo 权限即可）；空 = 只读轮询（不能回帖/建 PR）
    pub token: String,
    /// 承担开发任务的 Agent id（须声明 task_mode）
    pub agent_id: String,
    /// 开发 Agent 模型（空 = Agent 默认；须经该 Agent 的 model 启动参数声明）
    #[serde(default)]
    pub dev_model: String,
    /// 开发 Agent 推理强度（空 = Agent 默认）
    #[serde(default)]
    pub dev_effort: String,
    /// 验证 Agent id（空 = 与开发 Agent 相同；须声明 task_mode）
    #[serde(default)]
    pub verify_agent_id: String,
    /// 验证 Agent 模型（空 = Agent 默认）
    #[serde(default)]
    pub verify_model: String,
    /// 验证 Agent 推理强度（空 = Agent 默认）
    #[serde(default)]
    pub verify_effort: String,
    /// 源码仓库 owner（PR 目标；空 = 与 issue 仓库相同。源码 private、issue 发布在
    /// public 仓库时两者不同）
    #[serde(default)]
    pub source_owner: String,
    /// 源码仓库 repo（同上；push 永远走 repo_dir 的 origin，PR 建到这里）
    #[serde(default)]
    pub source_repo: String,
    /// IM 群机器人 webhook（企业微信/钉钉/飞书）；空 = 关闭推送
    #[serde(default)]
    pub notify_webhook: String,
    /// webhook 类型："wecom" | "dingtalk" | "feishu"
    #[serde(default)]
    pub notify_kind: String,
    /// 本地仓库路径（须为 git 工作区，含 origin 指向 owner/repo）
    pub repo_dir: String,
    /// worktree 隔离根目录（默认 Molto store_root/worktrees）
    pub worktrees_root: String,
    /// 验证目标启动命令（如 cebot dev 模式；空 = 不做自动验证）
    #[serde(default)]
    pub verify_command: String,
    /// 验证目标端口（8889 dev / 17980 prod；0 = 不做自动验证）
    #[serde(default)]
    #[ts(type = "number")]
    pub verify_port: u16,
    /// 自动闭环：开发完成后自动触发浏览器验证（需 verify_command/verify_port）
    #[serde(default)]
    pub auto_verify: bool,
    /// 全自动起点：捕获 issue 后自动开始开发（默认关，开了即全自动）
    #[serde(default)]
    pub auto_develop: bool,
    /// 验证失败的自动迭代上限（开发⇄验证循环轮数；默认 3）
    #[serde(default = "default_verify_max_iterations")]
    #[ts(type = "number")]
    pub verify_max_iterations: u32,
    /// 轮询间隔秒（≥30）
    #[ts(type = "number")]
    pub poll_interval_secs: u64,
    /// 单次 agent 任务超时秒（熔断，对标 mini-swe-agent wall-time limit）
    #[ts(type = "number")]
    pub task_timeout_secs: u64,
}

/// 跟单的 issue 引用（GitHub REST 只取所需字段）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChannelIssue {
    #[ts(type = "number")]
    pub number: u64,
    pub title: String,
    pub body: String,
    pub author: String,
    pub url: String,
}

/// issue 评论（解析 /molto 指令的原始输入）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChannelComment {
    pub author: String,
    pub body: String,
    /// Unix 毫秒
    #[ts(type = "number")]
    pub at: i64,
}

/// 跟单任务状态机（M1 使用 Captured→Developing→DevDone→PrCreated→Merged；
/// Verifying 预留给 M2 浏览器验证关卡）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ChannelTaskState {
    /// 已捕获：worktree 已建，等待开发
    #[default]
    Captured,
    /// headless Agent 开发中
    Developing,
    /// 开发完成（diff 待人审/建 PR）
    DevDone,
    /// 浏览器验证中（M2）
    Verifying,
    /// 验证通过，等开发者确认
    AwaitingHuman,
    /// PR 已建，等 issue 关闭
    PrCreated,
    /// issue 已关闭且 worktree 已清理
    Merged,
    /// 受阻（附原因），可重试
    Blocked,
}

/// 开发者经 issue 评论下达的指令（/molto 前缀协议）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ChannelCommand {
    /// /molto approve —— 认可当前产出（M1 PR 已建时仅回执）
    Approve,
    /// /molto redo: <反馈> —— 带反馈重跑开发
    Redo(String),
    /// 非 Molto 指令
    None,
}

/// 一个跟单任务：issue ↔ worktree ↔ agent 会话 ↔ PR 的绑定（派生数据，
/// 持久化为 tasks.json，可随时删除重建）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChannelTask {
    /// "{owner}/{repo}#{number}"
    pub id: String,
    pub issue: ChannelIssue,
    pub state: ChannelTaskState,
    /// worktree 绝对路径
    pub worktree_dir: String,
    /// 开发分支名 molto/issue-{number}
    pub branch: String,
    /// 承担任务的 Agent
    pub agent_id: String,
    /// 已建 PR（head 分支、编号、url）
    #[ts(type = "number | null")]
    pub pr_number: Option<u64>,
    pub pr_url: Option<String>,
    /// 最近一次 agent 任务日志尾部（供看板查看）
    pub log_tail: String,
    /// 待处理的 redo 反馈（poller 从评论解析后入队）
    pub pending_feedback: Vec<String>,
    /// 受阻原因
    pub error: Option<String>,
    /// 跨轮 agent 会话 key（首轮 headless 输出提取；redo 时经 resume_args 续聊）
    #[serde(default)]
    pub session_key: Option<String>,
    /// 浏览器验证结论文本（verify 编排产出）
    #[serde(default)]
    pub verify_summary: Option<String>,
    /// 验证已进行的轮次（开发⇄验证迭代计数）
    #[serde(default)]
    #[ts(type = "number")]
    pub verify_rounds: u32,
    /// 验证截图绝对路径列表（本机保存）
    #[serde(default)]
    pub verify_shots: Vec<String>,
    /// Unix 毫秒
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

impl ChannelTask {
    pub fn task_id(owner: &str, repo: &str, number: u64) -> String {
        format!("{owner}/{repo}#{number}")
    }

    pub fn branch_name(number: u64) -> String {
        format!("molto/issue-{number}")
    }
}

/// 验证结论（从验证 agent 输出中的 `VERIFY: PASS|FAIL` 协议行解析）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum VerifyVerdict {
    /// 验收通过
    Pass,
    /// 验收失败（附原因——将作为反馈回开发轮）
    Fail(String),
    /// 未输出结论协议行（人工判断）
    #[default]
    Unknown,
}

/// 浏览器验证报告（verify 编排产出；进任务持久化 + 回帖 issue）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VerifyReport {
    /// 验证 agent 的结论文本（stdout result 字段）
    pub summary: String,
    /// 结构化结论
    pub verdict: VerifyVerdict,
    /// 截图绝对路径列表（本机保存，看板可查看）
    pub screenshots: Vec<String>,
    /// agent 是否正常完成（验收结论以 summary 为准，人最终确认）
    pub success: bool,
}

fn default_verify_max_iterations() -> u32 {
    3
}

/// 解析 issue 评论为 Molto 指令（纯函数，首行 /molto 前缀生效）。
pub fn parse_command(body: &str) -> ChannelCommand {
    let first = body.trim().lines().next().unwrap_or("").trim();
    if let Some(rest) = first.strip_prefix("/molto approve") {
        let _ = rest;
        return ChannelCommand::Approve;
    }
    if let Some(rest) = first.strip_prefix("/molto redo") {
        let feedback = rest.trim_start_matches([':', '：']).trim();
        return ChannelCommand::Redo(feedback.to_string());
    }
    ChannelCommand::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_parsing_prefix_and_feedback() {
        assert_eq!(parse_command("/molto approve"), ChannelCommand::Approve);
        assert_eq!(
            parse_command("/molto approve 后续说明忽略"),
            ChannelCommand::Approve
        );
        assert_eq!(
            parse_command("/molto redo: 登录按钮点击无响应，请修复"),
            ChannelCommand::Redo("登录按钮点击无响应，请修复".into())
        );
        assert_eq!(
            parse_command("/molto redo 带中文冒号：样式错位"),
            // 反馈内容本身可含中文冒号：只有紧跟 redo 的分隔冒号会被剥除
            ChannelCommand::Redo("带中文冒号：样式错位".into())
        );
        assert_eq!(
            parse_command("普通评论 /molto approve"),
            ChannelCommand::None
        );
        assert_eq!(parse_command("/moltosaurus 绝不匹配"), ChannelCommand::None);
        assert_eq!(parse_command(""), ChannelCommand::None);
    }

    #[test]
    fn task_id_and_branch_format() {
        assert_eq!(ChannelTask::task_id("o", "r", 7), "o/r#7");
        assert_eq!(ChannelTask::branch_name(7), "molto/issue-7");
    }
}
