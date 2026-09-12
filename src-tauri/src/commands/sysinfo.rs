//! 系统信息：内存/磁盘/CPU/温度快照 + 进程列表/结束进程（sysinfo 0.36）
//! Disks/Components 是独立集合类型（不挂 System），统一收进 SystemMonitor 一把锁。

use std::sync::Mutex;

use sysinfo::{Components, Disks, Pid, ProcessesToUpdate, System};

use crate::error::AppError;

/// 由 lib.rs manage；所有系统信息命令共享这一份实时状态
pub struct SystemMonitor {
    pub sys: System,
    pub disks: Disks,
    pub components: Components,
}

impl SystemMonitor {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(),
            disks: Disks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
        }
    }
}

pub type SharedMonitor = Mutex<SystemMonitor>;

use tauri::State;

#[tauri::command]
#[specta::specta]
pub fn system_stats(mon: State<'_, SharedMonitor>) -> Result<SystemSnapshot, AppError> {
    let mut m = mon.lock().map_err(|e| AppError::poison(e.to_string()))?;
    Ok(build_snapshot(&mut m))
}

#[tauri::command]
#[specta::specta]
pub fn process_list(
    mon: State<'_, SharedMonitor>,
    sort: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<ProcInfo>, AppError> {
    let mut m = mon.lock().map_err(|e| AppError::poison(e.to_string()))?;
    Ok(collect_processes(
        &mut m,
        sort.as_deref().unwrap_or("mem"),
        limit.unwrap_or(6).clamp(1, 30),
    ))
}

#[tauri::command]
#[specta::specta]
pub fn process_kill(mon: State<'_, SharedMonitor>, pid: u32) -> Result<(), AppError> {
    let mut m = mon.lock().map_err(|e| AppError::poison(e.to_string()))?;
    kill_process(&mut m, pid)
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct MemStat {
    pub total_gb: u32,
    pub used_gb: u32,
    pub percent: u8,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct DiskStat {
    /// 挂载点，如 "C:\"
    pub mount: String,
    pub total_gb: u32,
    pub used_gb: u32,
    pub percent: u8,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct CpuStat {
    pub percent: u8,
    pub core_count: u32,
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct TempStat {
    pub label: String,
    pub celsius: f32,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct SystemSnapshot {
    pub mem: MemStat,
    pub disks: Vec<DiskStat>,
    pub cpu: CpuStat,
    /// Windows 上常为空（依赖传感器驱动），前端按空隐藏
    pub temps: Vec<TempStat>,
}

fn gb(bytes: u64) -> u32 {
    (bytes / 1024 / 1024 / 1024) as u32
}

fn percent_of(used: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }
    ((used as f64 / total as f64) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8
}

/// 刷新并构建快照（纯逻辑独立成函数便于测试）
pub fn build_snapshot(mon: &mut SystemMonitor) -> SystemSnapshot {
    mon.sys.refresh_memory();
    mon.sys.refresh_cpu_usage();
    mon.disks.refresh(true);
    mon.components.refresh(true);

    let mem_total = mon.sys.total_memory();
    let mem_used = mon.sys.used_memory();
    let mem = MemStat {
        total_gb: gb(mem_total),
        used_gb: gb(mem_used),
        percent: percent_of(mem_used, mem_total),
    };

    let disks = mon
        .disks
        .list()
        .iter()
        .filter_map(|d| {
            let total = d.total_space();
            if total == 0 {
                return None;
            }
            let used = total.saturating_sub(d.available_space());
            Some(DiskStat {
                mount: d.mount_point().to_string_lossy().into_owned(),
                total_gb: gb(total),
                used_gb: gb(used),
                percent: percent_of(used, total),
            })
        })
        .collect();

    let cpu = CpuStat {
        percent: mon.sys.global_cpu_usage().round().clamp(0.0, 100.0) as u8,
        core_count: mon.sys.cpus().len() as u32,
        name: mon
            .sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_default(),
    };

    let temps = mon
        .components
        .list()
        .iter()
        .filter_map(|c| {
            c.temperature().map(|t| TempStat {
                label: c.label().to_string(),
                celsius: t,
            })
        })
        .collect();

    SystemSnapshot {
        mem,
        disks,
        cpu,
        temps,
    }
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct ProcInfo {
    pub pid: u32,
    pub name: String,
    pub mem_mb: u32,
    pub cpu: f32,
}

/// 进程列表（按内存或 CPU 排序取 Top N）
pub fn collect_processes(mon: &mut SystemMonitor, sort: &str, limit: u32) -> Vec<ProcInfo> {
    mon.sys.refresh_processes(ProcessesToUpdate::All, true);
    let mut procs: Vec<ProcInfo> = mon
        .sys
        .processes()
        .iter()
        .map(|(pid, p)| ProcInfo {
            pid: pid.as_u32(),
            name: p.name().to_string_lossy().into_owned(),
            mem_mb: (p.memory() / 1024 / 1024) as u32,
            cpu: p.cpu_usage(),
        })
        .collect();
    if sort == "cpu" {
        procs.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
    } else {
        procs.sort_by(|a, b| b.mem_mb.cmp(&a.mem_mb).then(b.cpu.total_cmp(&a.cpu)));
    }
    procs.truncate(limit as usize);
    procs
}

/// 结束进程
pub fn kill_process(mon: &mut SystemMonitor, pid: u32) -> Result<(), AppError> {
    let pid = Pid::from_u32(pid);
    match mon.sys.process(pid) {
        Some(p) => {
            if p.kill() {
                Ok(())
            } else {
                Err(AppError::io(format!("结束进程失败（可能需要权限）: {pid}")))
            }
        }
        None => Err(AppError::validate(format!("进程不存在: {pid}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_on_real_machine() {
        let mut mon = SystemMonitor::new();
        let snap = build_snapshot(&mut mon);
        assert!(snap.mem.total_gb > 0, "物理机应有内存");
        assert!(!snap.disks.is_empty(), "至少一个磁盘分区");
        assert!(snap.cpu.core_count > 0);
    }

    #[test]
    fn process_list_nonempty_and_kill_missing() {
        let mut mon = SystemMonitor::new();
        let procs = collect_processes(&mut mon, "mem", 5);
        assert!(!procs.is_empty(), "真实机器必有进程");
        assert!(procs.len() <= 5);
        // 杀不存在的进程报 VALIDATE
        let err = kill_process(&mut mon, u32::MAX - 1);
        assert!(err.is_err());
    }
}
