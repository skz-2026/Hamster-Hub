<div align="center">

# 🐹 仓鼠Hub（HamsterHub）

**把桌面，囤进一个窝 —— Windows 上的 iOS 风格主屏幕，内置本地优先桌面助手与 AI Agent。**

[English](./README.md) · 简体中文

<img src="./docs/images/home-desktop.png" width="860" alt="仓鼠Hub 桌面模式" />

</div>

## 🎬 演示视频

<p align="center">
  <video src="https://raw.githubusercontent.com/skz-2026/Hamster-Hub/main/docs/hamster-hub-manual.mp4" controls muted playsinline width="48%">中文演示视频</video>
  <video src="https://raw.githubusercontent.com/skz-2026/Hamster-Hub/main/docs/hamster-hub-manual-en.mp4" controls muted playsinline width="48%">English demo video</video>
</p>

▶ [中文版演示 (.mp4)](./docs/hamster-hub-manual.mp4) · [English walkthrough (.mp4)](./docs/hamster-hub-manual-en.mp4)

## 1. 产品简介

仓鼠Hub 是一款让桌面像 iPhone 一样好用的 Windows 应用：问候大时钟 + 小组件仪表盘、
Spotlight 搜索、贴边定制的底部任务栏、可长按编辑的应用网格，底层是一套完整的本地优先
桌面助手（应用 / 文件 / 待办 / 便签 / 密码箱）。核心功能永久免费、无广告。
支持 Windows 10（1809+）与 Windows 11。

| | |
|---|---|
| <img src="./docs/images/home-windowed.png" width="100%" alt="窗口化模式" /><br><sub>**一套首页，两种形态** —— 窗口化工作台：红绿灯标题栏 + 侧栏 + 居中胶囊 Dock。</sub> | <img src="./docs/images/spotlight.png" width="100%" alt="Spotlight 搜索" /><br><sub>**Spotlight**（`Alt+Space`）—— 应用 / 本地文件 / 网页 / 问 AI 一框四路，支持拼音与首字母匹配。</sub> |
| <img src="./docs/images/bench.png" width="100%" alt="代理工作台" /><br><sub>**Agent 工作台** —— 托管 Claude Code / Codex / ZCode / Gemini 等编码代理，流式对话 UI。</sub> | <img src="./docs/images/vault.png" width="100%" alt="密码箱" /><br><sub>**密码箱** —— 本地密码管理，字段级加密 + 自动锁定。</sub> |
| <img src="./docs/images/files.png" width="100%" alt="文件" /><br><sub>**文件** —— 最近文件分类视图，按名称或拼音直达。</sub> | <img src="./docs/images/settings.png" width="100%" alt="设置" /><br><sub>**设置** —— 主题、壁纸、语言（EN / 简体中文 / 繁體中文）、桌面模式选项。</sub> |

### 功能亮点

- **桌面接管** —— 一键进入全屏桌面模式：定制常驻任务栏完全替代系统任务栏、隐藏桌面
  图标。进出都做状态快照与完整还原，并配有看门狗进程（`hamster-watchdog.exe`），即使
  应用崩溃或被强杀也能在数秒内还原桌面。
- **iOS 风格主屏** —— 小组件仪表盘（天气 / 待办 / 倒数日 / 最近文件 / 常用应用 /
  番茄钟），应用网格支持分页、文件夹与长按编辑；深色 / 浅色 / 像素三套主题，壁纸与
  强调色可换。
- **Spotlight 搜索** —— 一个热键直达应用、本地文件索引（FTS5，拼音 / 首字母）、
  网页搜索与问 AI 分层路由。
- **日常助手** —— 待办、便签、倒数日、日程、天气，小组件与独立页面双形态。
- **Agent 工作台** —— 编码代理的 GUI 宿主：流式对话（思考卡 / 工具卡 / diff 渲染），
  附 Recall 跨会话全文搜索。
- **本地优先、注重隐私** —— 数据全部落在本地 SQLite；仅天气、网页搜索与 AI 功能需要
  网络。密码箱以 Argon2id 派生密钥 + AES-256-GCM 做字段级加密。

## 2. 开发简介

Tauri 2（Rust 核心）+ React 18 + TypeScript + Tailwind 4 + Zustand / TanStack Query +
SQLite（rusqlite, FTS5）。IPC 类型由
[tauri-specta](https://github.com/oscartbeaumont/tauri-specta) 从 Rust 单向生成，
前后端不手写重复类型。

**环境要求：** Node ≥ 22.13 · pnpm 11 · Rust stable（MSVC）· VS Build Tools。

```bash
pnpm install

pnpm dev           # ① 浏览器优先开发：localhost:5173，IPC 自动走 mock，
                   #    右下角 WebDebugBar 可模拟桌面模式
pnpm dev:web       #    同上但用 5174 端口（tauri dev 占用 5173 时用这个）
pnpm test          # vitest 单测
pnpm e2e           # Playwright 端到端测试
pnpm tauri dev     # ② 桌面壳集成测试：系统接管 / 托盘 / 看门狗
pnpm codegen:ipc   # Rust 命令变更后重新生成 IPC TypeScript 类型
pnpm tauri build   # 打 NSIS 安装包（产物在 src-tauri/target/release）
```

其它脚本：`pnpm icon`（重生成占位图标）、`scripts/check.ps1`（本地全量检查，同 CI 门禁）。

开发工作流是浏览器优先：先在 `pnpm dev` 里用 mock IPC 把功能验干净，再进
`pnpm tauri dev` 验证系统级集成层。

### 文档索引

| 文档 | 内容 |
|---|---|
| [用户手册](./docs/06-user-manual.zh-CN.md) | 面向使用者的全部功能指导，含全功能截图 |
| [产品规划](./docs/01-product-plan.md) | 定位、竞品、用户画像、功能优先级、路线图、指标与风险 |
| [软件设计文档 SDD](./docs/02-sdd.md) | 系统架构、窗口模型、模块设计、数据库 Schema、IPC 契约、UI 规范 |
| [技术路线](./docs/03-tech-roadmap.md) | Tauri 2 选型论证、技术栈清单、关键技术方案、工程规范 |
| [代码架构](./docs/04-architecture.md) | 仓库结构、前端/Rust 分层、数据流、测试架构 |
| [踩坑档案](./docs/05-pitfalls.md) | 持续更新的踩坑记录 |

## 3. MIT License

本项目以 MIT 协议开源。

<div align="center">
<sub>把桌面，囤进一个窝。· Hoard your desktop into one cozy nest.</sub>
</div>
