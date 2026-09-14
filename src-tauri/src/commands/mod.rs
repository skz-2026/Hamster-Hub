pub mod ai;
pub mod apps;
pub mod control;
pub mod dashboard;
pub mod desktop;
pub mod dock;
pub mod files;
pub mod focus;
pub mod note;
pub mod settings;
pub mod sysinfo;
pub mod system;
pub mod todo;
pub mod tray;
pub mod updater;
pub mod vault;
// 托管式分屏（dock 右键「分屏添加」→ 会话维持 → 随时移出/退出；
// 会话在 src/split_mode.rs，几何与窗口状态在 hamster-platform::tile）
pub mod split;
