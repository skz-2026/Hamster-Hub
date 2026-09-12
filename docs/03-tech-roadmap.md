# 仓鼠Hub 技术路线

> 版本 v1.0 ｜ 选型结论先行：**Tauri 2 + React 18 + TypeScript + Tailwind + SQLite(FTS5)**

## 1. 框架选型：为什么是 Tauri 而不是 Electron

| 方案 | 产物体积 | 空闲内存 | 原生能力 | 结论 |
|---|---|---|---|---|
| **Tauri 2（选定）** | ~8-12MB | ~120-180MB（系统 WebView2） | Rust 直调 Win32，无桥接损耗 | ✅ 符合「轻量桌面助手」定位 |
| Electron | 80-150MB | 250-400MB | Node 生态最熟 | ❌ 内存体积与产品承诺冲突 |
| Wails (Go) | 类似 Tauri | 类似 | Go 调 Win32 较绕（cgo） | 备选 |
| WPF/WinUI3 (C#) | 中 | 低 | 最原生 | ❌ UI 开发效率低，团队栈不符 |

关键理由：
1. **资源承诺**：产品把「空闲内存 ≤ 200MB」写进了验收，Electron 做不到，Tauri 贴线可达。
2. **本产品重度依赖原生能力**：开始菜单/注册表/UWP 枚举、图标提取、全局热键、文件监听、DPAPI 加密——Rust 生态（`windows` crate）覆盖完整。
3. **安全模型**：Tauri capabilities 白名单与「前端零系统访问」的架构原则天然契合。
4. 风险与缓解：Rust 学习曲线 → 架构上把业务收敛在少数 command 层，core 服务用纯 Rust 逐步练手；界面逻辑全在 TS。

## 2. 技术栈清单

| 层 | 选型 | 用途 | 备选/说明 |
|---|---|---|---|
| 应用框架 | Tauri 2.x | 壳、IPC、多窗口、插件 | 稳定版 |
| 前端框架 | React 18 + TypeScript 5 | UI | 路由 `react-router` |
| 构建 | Vite 6 | 前端构建 | Tauri 官方模板 |
| 样式 | Tailwind CSS 4 + CSS Variables | 主题 token | – |
| 状态 | Zustand（UI 态）+ TanStack Query 5（IPC 数据） | 分层状态 | Query 统一缓存/失效 |
| 网格布局 | react-grid-layout | 工作台拖拽缩放 | 自研成本高，不重复造 |
| 动效 | framer-motion | 过渡动画 | – |
| IPC 类型 | tauri-specta | Rust→TS 类型生成 | 杜绝手写双份类型 |
| 数据库 | rusqlite（bundled + FTS5） | 本地存储/全文索引 | 不用 tauri-plugin-sql（前端 SQL 控制力差） |
| 文件监听 | notify 7 | 索引增量 | ReadDirectoryChangesW |
| 模糊匹配 | nucleo-matcher | 搜索排序 | helix 编辑器同款，纯 Rust |
| 拼音 | pinyin crate | 索引拼音列 | – |
| HTTP | reqwest | 天气/热榜/AI | – |
| 异步运行时 | tokio | 调度器/后台任务 | – |
| 窗口效果 | window-vibrancy | Mica/Acrylic | Win10 降级链路 |
| Win32 系统交互 | windows crate（raw FFI） | 任务栏 AppBar/SetWindowPos、图标注册表、WorkerW、亮度/音量 WMI | 系统接管唯一可行路线，封装进 `platform/` |
| 农历 | lunar-javascript（前端） | 农历/节气/节日 | 纯本地计算 |
| 官方插件 | single-instance / global-shortcut / autostart / notification / updater | 系统集成 | – |
| 测试 | cargo test / Vitest / Playwright | 三层测试 | – |
| CI/CD | GitHub Actions + tauri-action | 构建/发布 | NSIS 安装包 + portable zip |

## 3. 关键技术方案

### 3.1 窗口毛玻璃与 Win10/11 兼容

- Win11：`window_vibrancy::apply_mica`（跟随系统深浅色）。
- Win10 1809+：`apply_acrylic`（亚克力，注意拖动窗口时的重绘抖动，用 DWM 圆角+分层窗口缓解）。
- 兜底：不支持的系统纯色半透明 + 前端 blur 模拟。
- 启动时探测一次并缓存，用户也可在设置强制指定。

### 3.2 应用枚举与图标（最大原生风险点）

- `.lnk`：`lnk` crate 纯 Rust 解析（免 COM）；目标为 `explorer.exe`/卸载器 的条目过滤掉。
- UWP：`shell:AppsFolder` 枚举（`windows` crate ISHELLItem + IEnumIDList），拿 PFN;AppId 与显示名。
- 图标：`SHGetFileInfoW(SHGFI_ICON|SHGFI_LARGEICON)` → HICON 转 RGBA → `image` crate 编 PNG → 内容哈希缓存目录。失败占位。
- 已知坑：0x0 空图标、多尺寸图标选 48px、系统图标（此电脑等）排除——M1 阶段建立真机样本库回归。

### 3.3 搜索：FTS5 + 模糊融合

- FTS5 承担子串/前缀（`name MATCH 'wx*'` + 自定义 tokenizer 处理中英混排），nucleo 承担乱序模糊与打分，两者并集按 `score = 0.6*fuzzy + 0.4*frecency` 排序。
- 拼音列索引期生成：中文文件名 → 全拼串 + 首字母串（「微信.png」→ `weixin` / `wx`）。
- 容量预估：10 万文件 SQLite ≈ 25-40MB，查询 p95 < 50ms（FTS 命中路径）。

### 3.4 AI 流式

- reqwest SSE 逐 chunk → `app_handle.emit("ai://chunk/{id}", delta)` → 前端增量渲染。
- 中断：前端关闭面板 → invoke `ai_cancel(id)` → tokio CancellationToken。
- Key 存储：Windows DPAPI（`CryptProtectData`）加密后落盘，仅本机可解。

### 3.5 桌面接管与状态恢复（iOS 桌面模式的地基，竞品方案实证）

进入「桌面模式」时依次执行，退出/崩溃时反向还原：

| 步骤 | 实现方案 | 要点 |
|---|---|---|
| 1. 状态快照 | 写 `system-state-recovery.json`：任务栏窗口 hwnd / ex_style / appbar_state / 分层属性、桌面图标可见性注册表值 | 快照先行，任何后续操作失败都按快照回滚 |
| 2. 隐藏任务栏 | 对 Shell_TrayWnd / Shell_SecondaryTrayWnd：去 WS_EX_TOPMOST + `SHAppBarMessage(ABM_SETSTATE, ABS_AUTOHIDE)` + 移出工作区边缘（SetWindowPos） | Win11 与 Win10 行为差异大，需分别适配与回归 |
| 3. 隐藏桌面图标 | 注册表 `HKCU\...\Explorer\Advanced\HideIcons=1` + `SHChangeNotify` 广播；保留原始值用于还原 | 或 FOLDERFLAGS 路线，实测取舍 |
| 4. 主屏幕层 | 全屏无边框窗口置于 WorkerW 之下（`SetParent` 到 Progman/WorkerW）或 always-on-bottom 全屏窗 | 竞品采用「隐藏图标 + 全屏底层窗」方案（实测其桌面层窗口接管全屏点击），我们默认同路线，WorkerW 为备选 |
| 5. 看门狗进程 | 独立小 exe（`hamster-watchdog.exe`）：心跳监测主进程，主进程死亡且未正常还原时按快照恢复任务栏/图标并弹引导 | 参考竞品 `capybara-system-state-watchdog.exe`（714KB）实证该方案必要 |
| 6. 卸载清理 | NSIS 卸载脚本 + 竞品同款 `clean` 工具兜底：删除自启项、还原注册表、清理数据目录（可选保留） | 「卸载无残留」写进验收 |

**还原测试矩阵（每版本必过）**：正常退出 / 主进程被杀 / 看门狗被杀（双双阵亡时下次开机自检还原）/ 断电重启 / 资源管理器重启（explorer.exe 崩溃会重建任务栏，需重隐藏并保持快照一致）。

### 3.6 iOS 风格 UI 实现要点

- **图标 squircle**：CSS `border-radius: 22.5%`（iOS 超椭圆近似）+ 图标缓存 PNG 按主题底色重绘（浅色网格底/深色网格底两套）。
- **长按编辑**：指针事件（pointerdown 500ms 长按判定）+ 拖拽用 transform 合成层（60fps）；「抖动」用 CSS 动画 ±2° 交替。
- **分页**：横向滚动容器 + scroll-snap + 页码点；手势支持触控板双指滑动。
- **Dock**：独立置顶小窗（毛玻璃 acrylic + 圆角遮罩），常驻主屏之上。
- **Spotlight/控制中心**：独立覆盖层窗口（透明全屏），热键/热区触发，失焦即隐。
- **壁纸**：主屏窗口直接渲染（图片/纯色/渐变），不用系统壁纸（避免与 DWM 壁纸切换竞态）。

### 3.7 自更新

- `tauri-plugin-updater` + GitHub Releases 作为更新源；构建产物签名（minisign），M2 前购置代码签名证书（杀软误报对桌面接管类应用是致命项）。

## 4. 阶段技术路线（与产品里程碑对齐）

### M0 骨架（2 周）✅ 已完成（2026-09-11）
- Tauri+React+Tailwind+CI 跑通；无边框亚克力主窗口 + 自定义标题栏；tauri-specta 类型流水线；托盘/自启/单实例。
- 验收全绿：`pnpm tauri dev` 一键起、NSIS 打包（2.27MB）、cargo test / clippy -D / tsc+vite build 全过。

### M1 桌面接管（4 周）
- W1：**系统接管三件套**——状态快照/任务栏隐藏/图标隐藏/还原 + `hamster-watchdog.exe`（workspace 独立 bin）+ 还原测试矩阵（正常退出/杀进程/断电/explorer 重启）。
- W1-2：appindex（lnk + UWP + 图标 squircle 化缓存）。
- W2-3：主屏幕 Home 全屏窗（壁纸 + 图标网格 + 分页 scroll-snap + 页码点 + 长按编辑/拖拽/抖动 + iOS 文件夹缩放打开）。
- W3-4：iOS Dock 置顶小窗（毛玻璃 + 圆角）；「进入/退出桌面模式」入口与还原动效。
- 验收：进入/退出 ≤ 2s 且系统完全还原（含崩溃场景）；网格拖拽 60fps（见产品规划 §3.3）。

### M2 桌面模式 MVP（4 周，v0.2.0 公开版）
- fileindex（扫描/监听/拼音/FTS）+ Spotlight 搜索浮层（apps+files+web，Alt+Space）。
- iOS 风设置 App（壁纸/热键/图标布局/退出模式/开机进入桌面模式）。
- 性能达标（冷启 ≤ 2s、内存 ≤ 250MB、搜索 p95 ≤ 100ms）+ 代码签名 + Playwright E2E 冒烟。

### M3 iOS 全家桶（6 周）
- 小组件系统（时钟/天气/待办/日历/电池，iOS 尺寸规范）+ 控制中心（Wi-Fi/蓝牙/亮度/音量/勿扰磁贴）+ AI 问答（Spotlight 尾部 + 独立面板）+ 待办/便签/倒数日。
- 多显示器与高 DPI 全面适配、自更新灰度。

### M4 生态（持续）
- 通知中心、灵动岛、锁屏、多任务视图。
- 壁纸/主题市场、小组件插件化（前端动态导入 + manifest 权限沙箱）、macOS 技术预研。

## 5. 技术风险登记册

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| **系统接管崩溃残留/兼容性（最高风险）** | 高 | 高 | 快照→操作→看门狗三段式（§3.5）；还原测试矩阵每版本必过；Win10/Win11 双轨回归 |
| **杀软/EDR 误报** | 中 | 高 | M2 前购代码签名证书；行为透明说明；规避高危 API 组合 |
| UWP 枚举与图标提取兼容坑 | 高 | 中 | M1 第一周专项攻坚；真机样本库；失败占位不阻塞 |
| Acrylic 在 Win10 拖动抖动 | 中 | 低 | 降级纯色选项；圆角分层窗口 |
| 热榜接口变动/反爬 | 高 | 低 | 源抽象可单点禁用；缓存兜底；P2 定位 |
| 全屏 WebView 拖拽掉帧 | 中 | 中 | transform 合成层 + will-change；图标位图预缩放；DOM 虚拟化（分页天然隔离） |
| Rust 交付速度 | 中 | 中 | command 层薄、core 纯函数；难点（原生 API）集中攻坚，不在业务流里铺开 |
| WebView2 依赖（精简系统/LTSC 未装） | 低 | 中 | 安装器内置 WebView2 引导下载 |
| FTS 中文 tokenizer 质量 | 中 | 中 | 自定义简单二元+拼音列冗余，实测不满意再换 jieba 分词列 |

## 6. 工程规范

- 分支：`main` 保护 + 短周期 feature 分支；commit 用 Conventional Commits；每里程碑打 tag 发 Release。
- 代码质量：Rust `cargo fmt + clippy -D warnings`；TS ESLint + Prettier；CI 强制。
- 版本：SemVer，`v0.x.y`；DB 迁移只增不改，破坏性变更写迁移与回滚说明。
- 依赖治理：每月 `cargo audit` + `pnpm audit`；新增依赖需在 PR 说明理由；锁定 lockfile。
- 发布流程：tag → CI 构建（NSIS+portable）→ 签名 → Draft Release → 冒烟清单 → publish。

## 7. 开发环境

- Windows 10/11 + VS2022 Build Tools（MSVC）+ Rust stable + Node 20 LTS + pnpm。
- 日常命令：`pnpm i` → `pnpm tauri dev`；类型再生成 `pnpm codegen:ipc`；基准 `pnpm bench`。
