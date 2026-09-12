//! 仓鼠Hub 内核：领域模型 + 同步引擎 + 备份 + 存储 + Doctor。
//!
//! 本 crate 不依赖任何内部 crate；上层（adapters/app/未来 cli）只能向下依赖这里。

pub mod adapter;
pub mod atomic;
pub mod backup;
pub mod doctor;
pub mod error;
pub mod git;
pub mod hash;
pub mod inventory;
pub mod launcher;
pub mod model;
pub mod registry;
pub mod scanner;
pub mod status;
pub mod store;
pub mod sync;

pub use adapter::{AgentAdapter, CommandSpec};
pub use error::{HamsterError, Result};
pub use model::agent::{AgentInfo, Caps};
pub use model::channel::{
    parse_command, ChannelCommand, ChannelComment, ChannelConfig, ChannelIssue, ChannelTask,
    ChannelTaskState, VerifyReport,
};
pub use model::git::{GitFileStatus, GitLogEntry, GitStatus};
pub use model::mcp::{McpServer, Scope, Transport};
pub use model::rules::{RuleFile, SkillInfo};
pub use model::runtime::{
    LaunchOption, LaunchOptions, LaunchOverrides, LiveSessionInfo, PromptInject, RuntimeSpec,
    SessionChannel, SessionKind, TaskModeSpec,
};
pub use model::session::{
    ChunkParse, IndexStatus, SearchHit, SearchQuery, SessionMessagesPage, SessionMeta,
    SessionSource, SessionSummary, SnapshotMessage, SnapshotRole,
};
pub use model::streaming::{
    LiveStreamInfo, ProtocolDialect, StreamConfigChoice, StreamConfigOption, StreamEvent,
    StreamEventDiff, StreamEventKind, StructuredChannel,
};
pub use model::workspace::WorkspaceRecord;
pub use registry::Registry;
pub use store::{HistoryEntry, Store, StoreConfig};
pub use sync::{ApplyReport, DriftDecision, FilePlan, SyncEngine, TargetAction};
