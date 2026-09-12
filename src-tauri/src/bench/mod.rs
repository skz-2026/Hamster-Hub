//! bench（代理工作台）：agent 域层装配（域层 crate 源自上游开源项目，整合时已更名）。
//!
//! 分层对齐 docs/04-architecture.md：本模块是薄胶水层，
//! 域逻辑全在 crates/hamster-{core,adapters,runtime,index,mcp}（零 Tauri 依赖）。
//! 数据根 = %APPDATA%/com.hamsterhub.app/agent/（sessions.db 由 hamster-index 自管 schema，
//! 不进 hamsterhub.db 迁移链）。
pub mod assistant;
pub mod commands;

use std::path::PathBuf;

use hamster_adapters::build_registry;
use hamster_core::backup::BackupManager;
use hamster_core::registry::Registry;
use hamster_core::store::Store;
use hamster_runtime::ScrollbackStore;

use crate::error::AppError;

pub struct BenchContext {
    /// bench 数据根（app_data/agent）
    pub store_root: PathBuf,
    pub store: Store,
    pub backup: BackupManager,
    pub registry: Registry,
    /// 终端 scrollback 快照存储（上游 §10.2；PTY 终端模式启用，GUI 流式暂未用到）
    #[allow(dead_code)]
    pub scrollback: ScrollbackStore,
}

impl BenchContext {
    pub fn init(data_dir: &std::path::Path) -> Result<Self, AppError> {
        let store_root = data_dir.join("agent");
        std::fs::create_dir_all(&store_root).map_err(|e| AppError::io(e.to_string()))?;
        let store = Store::new(&store_root);
        let backup = BackupManager::new(store_root.join("backups"));
        // registry 检测各 Agent CLI 的安装与配置根（~/.claude 等），需真实 home
        let home = std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        let registry = build_registry(&home);

        // 首次启动：为全部已注册 Agent 预置用户级分发目标（上游同款）
        let mut config = store.load_config().map_err(bench_err)?;
        let ids: Vec<&str> = registry.list().iter().map(|a| a.id()).collect();
        config.ensure_targets(&ids);
        if !store.has_config() {
            store.save_config(&config).map_err(bench_err)?;
        }

        let scrollback = ScrollbackStore::new(store_root.join("runtime"));
        Ok(Self {
            store_root,
            store,
            backup,
            registry,
            scrollback,
        })
    }
}

/// HamsterError → 仓鼠Hub AppError（code 原样透传，前端按 code 引导）
pub fn bench_err(e: hamster_core::HamsterError) -> AppError {
    AppError::new(e.code(), e.message())
}
