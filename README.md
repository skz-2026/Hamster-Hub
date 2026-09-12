# 仓鼠Hub（HamsterHub）

> 把桌面，囤进一个窝。—— Windows 上的 iOS 风格桌面

对标水豚hub（macOS 风）的桌面管理工具，走 **iOS 风格**差异化：主屏幕（图标网格/分页/文件夹/长按编辑）+ Dock + Spotlight 搜索 + 控制中心，底层是完整的本地优先桌面助手（应用/文件索引/待办/AI），无广告。

## 文档索引

| 文档 | 内容 |
|---|---|
| [产品规划](./docs/01-product-plan.md) | 定位、竞品、用户画像、功能优先级、MVP 范围、版本路线图、指标与风险 |
| [软件设计文档 SDD](./docs/02-sdd.md) | 系统架构、窗口模型、模块设计、数据库 Schema、IPC 契约、UI 规范、非功能设计 |
| [技术路线](./docs/03-tech-roadmap.md) | Tauri 2 选型论证、技术栈清单、关键技术方案、阶段路线、风险登记册、工程规范 |
| [代码架构](./docs/04-architecture.md) | 仓库结构、前端/Rust 分层、IPC 设计、数据流、代码规范、测试架构 |

## 快速开始

环境要求：Node ≥ 20 + pnpm 11 + Rust stable（MSVC）+ VS Build Tools。

**开发工作流（浏览器优先，详见 [Agent.md](./Agent.md)）**：

```bash
pnpm dev           # ① 纯浏览器开发：localhost:5173，IPC 自动走 mock，右下角 WebDebugBar 可模拟桌面模式
pnpm dev:web       #    同上但用 5174 端口（tauri dev 占用 5173 时用这个）
pnpm test          # vitest 单测（layout 纯函数）
pnpm tauri dev     # ② 桌面壳集成测试：系统接管/托盘/看门狗（浏览器验证通过后才进这层）
pnpm codegen:ipc   # Rust 命令变更后重新生成 IPC TypeScript 类型
pnpm tauri build   # 打 NSIS 安装包（产物在 src-tauri/target/release）
```

其它脚本：`pnpm icon`（重生成占位图标）、`scripts/check.ps1`（本地全量检查，同 CI 门禁）。

## 技术栈

Tauri 2（Rust 核心）+ React 18 + TypeScript + Tailwind 4 + Zustand/TanStack Query + SQLite(rusqlite, FTS5)。IPC 类型由 tauri-specta 从 Rust 单向生成，前后端不手写重复类型。

## 当前状态

**M1 桌面接管 + M2 桌面助手已完成（2026-09-12）**：

- 工程：Tauri 2 + React 18 + TS + Tailwind，IPC 类型由 tauri-specta 从 Rust 单向生成
- 桌面接管三件套：任务栏/图标隐藏 + 状态快照恢复 + `hamster-watchdog.exe` 看门狗（崩溃 ≤4s 还原）
- iOS 主屏：图标网格/分页/文件夹/长按编辑 + 小组件（时钟/天气/待办/倒数日/系统状态）
- 桌面模式 = **全屏接管整个桌面（对齐水豚hub）**：默认页为水豚式桌面主页（壁纸 + 问候大时钟 +
  搜索 + 小组件），**底部贴边通栏任务栏完全替代系统任务栏**（左端主屏/搜索/设置 + 定制/常用
  分组，右端退出 + 时钟）；退出后为窗口化模式（红绿灯标题栏 + 侧栏 + 居中胶囊 Dock）
- Spotlight 搜索（热键 + 拼音/首字母/文件/网页）、待办/便签/倒数日/天气、NSIS 打包
- **代理工作台（/bench，Molto 整合 MVP）**：GUI 流式对话托管 Claude Code / Codex 等编码代理
  （assistant-ui 消息流：思考卡/工具卡/diff/markdown 高亮）+ Recall 跨代理会话全文搜索；
  Rust 域层 vendor 自 Molto（`D:\ksa\Molto`，molto-core/adapters/runtime/index 四个零 Tauri
  依赖 crate），PTY 终端模式与交付看板留待后续

下一步：代码签名（发布前必须）· 通知中心/灵动岛（M3+）。详见 [Agent.md](./Agent.md) 状态滚动更新。
