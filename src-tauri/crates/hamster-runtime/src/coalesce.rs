//! PTY 输出合帧（runtime-design.md §3.3）：最多 8ms 或 32KB 合并一帧。
//!
//! 高吞吐（`cargo build` 输出每秒数千小 chunk）不满屏打爆 ipc Channel；
//! 低延迟（击键回显）不攒批——首字节到发送最多隔一个窗口期，交互无感。

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// 合帧窗口：窗口期内到达的 chunk 合并为一次回调。
pub const COALESCE_WINDOW: Duration = Duration::from_millis(8);
/// 单帧字节上限：攒到即发，不等窗口结束。
pub const COALESCE_MAX_BYTES: usize = 32 * 1024;

/// 从 rx 收 chunk，按窗口/上限合并后回调 on_data；rx 关闭时冲刷余量并返回。
/// 窗口自首个 pending 字节起算，超时即发，不等待后续读取。
pub fn run_coalesce(rx: Receiver<Vec<u8>>, on_data: &mut dyn FnMut(&[u8])) {
    let mut pending: Vec<u8> = Vec::new();
    let mut window_start: Option<Instant> = None;
    loop {
        let received = match window_start {
            // 无在途字节：阻塞等首个 chunk（不空转）
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
            Some(start) => {
                let remaining = COALESCE_WINDOW.saturating_sub(start.elapsed());
                match rx.recv_timeout(remaining) {
                    Ok(chunk) => Ok(chunk),
                    Err(RecvTimeoutError::Timeout) => {
                        on_data(&pending);
                        pending.clear();
                        window_start = None;
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        };
        if pending.is_empty() {
            window_start = Some(Instant::now());
        }
        pending.extend_from_slice(&received.unwrap_or_default());
        if pending.len() >= COALESCE_MAX_BYTES {
            on_data(&pending);
            pending.clear();
            window_start = None;
        }
    }
    if !pending.is_empty() {
        on_data(&pending);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;

    /// 跑一遍合帧并收集输出（独立线程 + 输出通道）。
    fn coalesce_inputs(inputs: Vec<Vec<u8>>) -> (Vec<Vec<u8>>, thread::JoinHandle<()>) {
        let (tx_in, rx_in) = mpsc::channel();
        let (tx_out, rx_out) = mpsc::channel();
        let handle = thread::spawn(move || {
            run_coalesce(rx_in, &mut |bytes| {
                tx_out.send(bytes.to_vec()).unwrap();
            });
        });
        for chunk in inputs {
            tx_in.send(chunk).unwrap();
        }
        drop(tx_in); // 断开发送端：合帧线程冲刷余量后退出，collect 才能收尾
        (rx_out.into_iter().collect(), handle)
    }

    #[test]
    fn merged_bytes_preserve_order_and_content() {
        let inputs: Vec<Vec<u8>> = (0..100u8).map(|i| vec![i; 1024]).collect();
        let (outputs, handle) = coalesce_inputs(inputs);
        handle.join().unwrap();
        // 100KB 必然触发多次 32KB 上限冲刷，帧数远少于输入 chunk 数
        assert!(outputs.len() < 100);
        assert!(outputs.len() >= 4);
        let total: Vec<u8> = outputs.concat();
        assert_eq!(total.len(), 100 * 1024);
        for (i, chunk) in total.chunks(1024).enumerate() {
            assert_eq!(chunk, &vec![i as u8; 1024][..chunk.len()]);
        }
    }

    #[test]
    fn single_byte_flushes_within_window_even_without_eof() {
        let (tx_in, rx_in) = mpsc::channel();
        let (tx_out, rx_out) = mpsc::channel();
        thread::spawn(move || {
            run_coalesce(rx_in, &mut |bytes| {
                tx_out.send(bytes.to_vec()).unwrap();
            });
        });
        tx_in.send(b"x".to_vec()).unwrap();
        // 发送端不关闭：窗口期过后必须自动冲刷（击键回显不攒批）
        let out = rx_out
            .recv_timeout(Duration::from_millis(500))
            .expect("窗口期后未冲刷");
        assert_eq!(out, b"x");
        drop(tx_in);
    }

    #[test]
    fn oversize_chunk_flushes_immediately() {
        let (tx_in, rx_in) = mpsc::channel();
        let (tx_out, rx_out) = mpsc::channel();
        thread::spawn(move || {
            run_coalesce(rx_in, &mut |bytes| {
                tx_out.send(bytes.to_vec()).unwrap();
            });
        });
        tx_in.send(vec![0u8; COALESCE_MAX_BYTES]).unwrap();
        let out = rx_out
            .recv_timeout(Duration::from_millis(500))
            .expect("超上限未立即冲刷");
        assert_eq!(out.len(), COALESCE_MAX_BYTES);
        drop(tx_in);
    }

    #[test]
    fn tail_flushed_on_sender_disconnect() {
        let inputs = vec![b"tail".to_vec()];
        let (outputs, handle) = coalesce_inputs(inputs);
        handle.join().unwrap();
        assert_eq!(outputs, vec![b"tail".to_vec()]);
    }
}
