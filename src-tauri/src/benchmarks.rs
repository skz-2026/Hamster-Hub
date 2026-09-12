// 搜索基准：10 万行量级下 file/app FTS 检索延迟（p50/p95）
// 用法: cd src-tauri && cargo test --release -p hamster-hub --lib bench -- --nocapture
// （只打印不断言阈值，人工对照产品验收 p95 ≤ 100ms）
#[cfg(test)]
mod search_bench {
    use crate::core::pinyin::pinyin_cols;
    use rusqlite::Connection;
    use std::time::Instant;

    #[allow(dead_code)]
    fn now_us() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros()
    }

    #[test]
    fn bench_search_latency() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(concat!(
            include_str!("../migrations/0001_init.sql"),
            "\n",
            include_str!("../migrations/0002_fileindex.sql"),
            "\n",
            include_str!("../migrations/0003_apps_pinyin.sql"),
        ))
        .unwrap();

        // 构造 10 万文件行（混合中文名）
        c.execute("BEGIN", []).unwrap();
        {
            let mut stmt = c
                .prepare(
                    "INSERT INTO file_meta(path,name,ext,dir,size,mtime,kind,pinyin_full,pinyin_initials)
                     VALUES(?1,?2,?3,'',0,0,'',?4,?5)",
                )
                .unwrap();
            for i in 0..100_000u32 {
                let name = if i % 3 == 0 {
                    format!("微信截图_{i:06}.png")
                } else if i % 3 == 1 {
                    format!("年度总结_{i}.docx")
                } else {
                    format!("report_{i}.pdf")
                };
                let (full, init) = pinyin_cols(&name);
                stmt.execute(rusqlite::params![
                    format!(r"C:\data\{i}\{name}"),
                    name,
                    name.rsplit('.').next().unwrap(),
                    full,
                    init
                ])
                .unwrap();
            }
        }
        c.execute("COMMIT", []).unwrap();

        let queries = [
            "weixin",
            "截图",
            "wxjt",
            "ndzj",
            "report",
            "微信截图",
            "annual",
        ];
        let mut all: Vec<u128> = Vec::new();
        for q in queries {
            let mut times = Vec::new();
            for _ in 0..30 {
                let t0 = Instant::now();
                let hits = crate::core::fileindex::search(&c, q, 8).unwrap();
                let us = (Instant::now() - t0).as_micros();
                times.push(us);
                all.push(us);
                let _ = hits.len();
            }
            times.sort();
            println!(
                "q={q:?} p50={:.1}ms p95={:.1}ms",
                times[times.len() / 2] as f64 / 1000.0,
                times[(times.len() as f64 * 0.95) as usize] as f64 / 1000.0
            );
        }
        all.sort();
        println!(
            "TOTAL n={} p50={:.1}ms p95={:.1}ms max={:.1}ms",
            all.len(),
            all[all.len() / 2] as f64 / 1000.0,
            all[(all.len() as f64 * 0.95) as usize] as f64 / 1000.0,
            *all.last().unwrap() as f64 / 1000.0
        );
    }
}
