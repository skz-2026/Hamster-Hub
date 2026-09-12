//! 真实环境冒烟（默认忽略，手动执行）：只读扫描本机真实 Agent 会话目录，
//! 摄取到临时索引库后验证全文检索回环。绝不写任何真实 ~/ 文件。
//!
//! 执行：cargo test -p hamster-index --test real_home_smoke -- --ignored --nocapture

use std::path::PathBuf;

use hamster_core::{Registry, SearchQuery};
use hamster_index::{ingest_all, IndexStore};

#[test]
#[ignore = "读本机真实 Agent 会话（只读），仅手动冒烟"]
fn real_home_index_smoke() {
    let home = std::env::var("HAMSTER_INDEX_SMOKE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .map(PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
        });

    let registry = build_registry(&home);
    let tmp = tempfile::tempdir().unwrap();
    let store = IndexStore::open(&tmp.path().join("smoke.db")).unwrap();

    let report = ingest_all(&registry, &store).unwrap();
    println!(
        "ingest: scanned={} updated={} skipped={} errors={}",
        report.scanned, report.updated, report.skipped, report.errors
    );
    let status = store.stats().unwrap();
    println!(
        "index: sessions={} messages={} bytes={}",
        status.sessions, status.messages, status.bytes
    );
    assert!(status.sessions > 0, "真实环境应至少索引到 1 个会话");
    assert!(status.messages > 0, "真实环境应至少索引到 1 条消息");

    // 摄取-检索回环：用一条真实消息的片段做检索词
    let sample = store
        .first_message_sample(6)
        .unwrap()
        .expect("应有消息可采样");
    let hits = store
        .search(&SearchQuery {
            text: sample.clone(),
            agents: vec![],
            project: None,
            limit: Some(5),
        })
        .unwrap();
    println!("search {:?} → {} hits", sample, hits.len());
    assert!(!hits.is_empty(), "真实片段检索应有命中");

    // 活会话→索引桥接回归（GUI 原生视图报错 bug 的实机面）：取一条真实会话的
    // (agent, project) 调 latest_for_project，必须返回该项目的会话而非报错
    // （曾因 SQL 引用不存在的 last_active_at 列整体失败，前端显示「未入索引」）。
    let probe = hits
        .iter()
        .find(|h| h.project_path.as_deref().is_some_and(|p| !p.is_empty()))
        .expect("真实数据应有带项目路径的命中");
    let proj = probe.project_path.clone().unwrap();
    let found = store.latest_for_project(&probe.agent, &proj).unwrap();
    println!(
        "latest_for_project(agent={}, project={}) → {:?}",
        probe.agent,
        proj,
        found.as_ref().map(|(k, _)| k.clone())
    );
    assert!(
        found.is_some(),
        "真实 (agent, project) 必须能桥接到索引会话键"
    );
}

fn build_registry(home: &std::path::Path) -> Registry {
    hamster_adapters::build_registry(home)
}
