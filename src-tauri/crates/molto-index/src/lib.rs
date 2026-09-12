//! Molto Index（v2.0 R2）：跨 Agent 会话全文索引。
//!
//! 边界（runtime-design.md §4）：对 Agent 落盘文件**只读**；SQLite（WAL + FTS5
//! trigram）是**派生数据**，可随时重建；解析纯函数在 adapters，这里只做
//! 摄取编排（IO + 游标）与检索。

pub mod ingest;
pub mod store;

pub use ingest::{ingest_all, IngestReport};
pub use store::IndexStore;
