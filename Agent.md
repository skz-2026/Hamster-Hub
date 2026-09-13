# Agent.md — 仓鼠Hub 开发规范（Agent / 人类通用）

> 本文件是本仓库的一等公民开发规范。做任何改动前先读完本文；与 docs/ 冲突时以本文为准并回写文档。

## 项目一句话

Windows 上的 iOS 风格桌面 + agent 工作台（bench）：Tauri 2 + React 18 + TS + SQLite。
AI 走「集成 agent 为大脑 + 桌面 MCP」路线，不自研 agent（2026-09-12 决议）：
bench 域层 crates 整合自上游开源项目并更名 hamster-*，桌面能力经 hamster-mcp 暴露给外部 agent。
详细设计见 [docs/01-product-plan.md](docs/01-product-plan.md)（产品）、[docs/02-sdd.md](docs/02-sdd.md)（SDD）、
[docs/03-tech-roadmap.md](docs/03-tech-roadmap.md)（技术路线）、[docs/04-architecture.md](docs/04-architecture.md)（代码架构）。
注意：docs/04 是 M0 约定稿，部分结构已演进（无 zustand、router.tsx、bench/crates 未收录），
冲突时以本文件「目录速查」和仓库现状为准。

## 强制工作流：浏览器优先，两层验证

**任何前端 UI 改动，先在浏览器里开发和验证，通过后才进桌面壳。** 这不是建议，是流程（2026-09-11 真机排障教训固化）。

### 第 1 层：浏览器（开发 + UI 效果验证）

```bash
pnpm dev          # 纯 vite，浏览器打开 http://localhost:5173
                  # （与 tauri dev 并存时改用 pnpm dev:web，端口 5174——5173 被 tauri devUrl 占用）
```

- 浏览器里没有 tauri 后端，`@/shared/lib/ipc` 会自动切换到 **IPC mock 层**（见下），UI 全功能可用。
- 右下角 **WebDebugBar** 可模拟「进入/退出桌面模式」事件，驱动 AppShell 进出 `/home` 路由。
- 用 DevTools 点选元素、看 console、断点——不要在桌面壳里做这些事。
- 主屏所有交互（网格/分页/长按抖动/拖拽/文件夹/Dock/搜索/主题）都在这层验证 + 截图。
- 自动化回归：`pnpm e2e`（Playwright 冒烟，自动起 5174 dev server；CI 同款）。改主流程 UI 后跑一遍。

### 第 2 层：桌面壳（只测系统集成）

```bash
pnpm tauri dev    # vite + cargo run 一键起
```

只验证浏览器测不了的：

| 项 | 方法 |
|---|---|
| 任务栏/桌面图标接管与还原 | 设置页或托盘进入/退出；用 PowerShell `IsWindowVisible(Shell_TrayWnd)` 断言（不要靠截图肉眼） |
| 崩溃还原 | 进入桌面模式 → `taskkill /IM hamster-hub.exe /F` → ≤4s 任务栏应被 `hamster-watchdog.exe` 还原 |
| explorer 重启重隐藏 | 进入后重启 explorer → 5s 内 rehide 巡检应重新隐藏任务栏 |
| 真实应用索引/图标 | 看 `%APPDATA%/com.hamsterhub.app/icons/` PNG 产物 + 主屏渲染 |
| 托盘/单实例/自启 | 手动 |
| Dock 系统托盘区弹出 | `commands/tray.rs` open_overflow（UIA 借壳方案）；实验/断言脚本 `scripts/uia-*.ps1` |

### IPC mock 层（第 1 层的机制）

- 契约单一事实源：Rust 命令 + `#[specta::specta]` → `pnpm codegen:ipc` 生成 `src/shared/types/ipc.ts`（**勿手改**）。
- **应用代码一律 `import { commands, events } from '@/shared/lib/ipc'`**，禁止直连 `@/shared/types/ipc`
  （lib 层在浏览器环境自动替换为 `ipc-mock.ts` 的假实现；types 只放类型）。
- mock 数据（假应用列表/设置/事件总线）在 `src/shared/lib/ipc-mock.ts` 维护，`home.layout` 持久化到 localStorage；
  bench 的 GUI 流式/PTY/Recall 也已 mock，浏览器层全功能可用。
- 新增 Rust 命令后：改 Rust → `pnpm codegen:ipc` → 若浏览器也要用，在 mock 层补同签名假实现。

## 目录速查

```
src/
  app/            壳：AppShell(路由/事件导航)、SideNav、TitleBar、providers、router.tsx
    routes/       AppShell 内：home(iOS 主屏) desktop agent bench workbench(仪表盘,默认/) search
                  schedule apps files settings；AppShell 外独立窗口：spotlight / taskbar
  features/
    home/         iOS 主屏：HomeScreen、AppIcon、DockBar、TrayArea、HomeWidget、layout.ts(纯函数+单测)
    bench/        代理工作台：ChatThread/ChatSessionView/TerminalView(xterm)/SessionSidebar、
                  stream-registry + timeline-logic(流→UI 时间线,含单测)、registry、AgentAvatar
    agent/        桌面助手：AssistantPanel(persona 面板)、McpSyncCard(MCP 接入状态/开关)
    plugins/      UI 插件：registry、PluginWidgetHost(主屏挂载)、PluginManageCard
    search/       useUnifiedSearch（Spotlight 与搜索页共用）
    settings/ apps/ schedule/ todo/ dashboard/ workbench/ spotlight/ control/
  assets/agents/  agent 品牌 SVG 头像（claude/codex/gemini/glm 等 25 个）
  shared/
    lib/ipc.ts    IPC 出口（tauri/浏览器自动切换）★ 应用唯一入口
    lib/ipc-mock.ts
    i18n/         多语言：core.ts(translate/getLang) + provider.tsx(useI18n) +
                  zh-CN|zh-TW|en 三份词典（按命名空间分文件，编译期强制 key 对齐）
    components/   通用组件（WebDebugBar、StubRoute）
    types/ipc.ts  tauri-specta 生成物（勿手改）；types/bench.ts 为前端手写类型
src-tauri/
  src/
    bench/        代理工作台装配：commands(全部 bench_* IPC)、assistant(persona+MCP 注入+接入信息)
    commands/     其余 IPC 薄层：settings/system/apps/desktop/files/todo/note/dashboard/sysinfo/control/tray/ai
    core/         appindex(.lnk 扫描+图标缓存)、fileindex(FTS5+拼音+watcher)、pinyin、weather、countdown
    desktop_mode.rs  系统接管/还原/巡检/看门狗拉起/窗口形态
    mcp_server.rs    McpHub：内嵌 HTTP MCP server（127.0.0.1 + 双令牌）
    plugins/      UI 插件宿主（首启自举示例/存储/bridge）
    store/        主库 hamsterhub.db：db(迁移链)、settings、todo、note、config
    events.rs     specta 强类型事件（DesktopModeChanged / BenchStream* 等）
  crates/         bench 域层（整合自上游开源项目，零 Tauri 依赖）
    hamster-core/      领域内核：model(agent/mcp/session/streaming/...)、sync、backup、doctor、store
    hamster-adapters/  Agent 配置适配层：claude/codex/gemini/zcode/acp（纯函数 + fixture 往返测试）
    hamster-runtime/   PTY 会话宿主：拉起官方 CLI、流转发/合帧、resize、退出上报
    hamster-index/     跨 Agent 会话全文索引（FTS5；派生库，可重建）
    hamster-mcp/       桌面 MCP server：desktop/browser/computer 工具组 + 审计（含 mcp serve stdio）
    hamster-platform/  Win32 共享层：taskbar/desktop_icons/snapshot/icons/shell/tray/volume/wind_guard
                       （主程序与看门狗共用）
    hamster-watchdog/  看门狗 bin（主进程死亡→按快照还原系统）
  app.manifest    comctl32 v6 + DPI（测试二进制必需，勿删）
```

## 代码规则：模块化，禁止巨石文件

核心约束：**一个源文件一个职责**。routes 页面是组装层，不是业务容器——
不要把所有业务功能塞进一个页面/源文件里。
拆分信号：文件 ≈300 行；一个组件混着取数/布局/交互规则/渲染细节 3+ 职责；
diff 正在往大文件里继续堆功能——先拆再写。

前端：
- `app/routes/*/index.tsx` 只做页面组装（引 feature 组件 + hooks + 布局），不放业务逻辑。
  范式：home 页 5 行全托 HomeScreen；bench 页只组合 SessionSidebar/ChatSessionView/TerminalView。
- 业务 UI 进 `features/<域>/`，**一组件一文件**（bench 的 ChatThread / ChatParts /
  ChatParts.message / SelectPill / AgentAvatar 即范式），复杂页面拆子视图。
- **可测逻辑抽纯函数 .ts + 同名 .test.ts**（layout.ts / timeline-logic.ts / stream-registry.ts 模式），
  别把规则写死在 JSX 里。
- 数据访问优先沉到域 `features/<域>/hooks.ts`，多组件共享的查询/变更必须走这里；
  无论哪层，IPC 一律从 `@/shared/lib/ipc` 出口 import（见「IPC mock 层」）。
- features 之间**禁止横向 import**，跨域复用上提 shared/（components/lib/types）。
- 存量超标页（settings/schedule/agent）按「改哪拆哪」渐进拆分，不做一次性大重构；
  settings 拆出 features/settings/controls.tsx 就是进行中的范式。

Rust：
- `src/commands/` 与 `src/bench/` 的命令函数只做参数校验 + State 取用 + 调域层 + 错误映射；
  域逻辑进 `core/` 或 crates，新领域优先独立 crate（零 Tauri 依赖）。
- 一个 agent / 一个子系统一个文件（adapters 每 agent 一文件即范式）；
  解析、映射类逻辑写成纯函数 + fixture 测试。

## 多语言（i18n）约定：新页面/新组件强制接入

**所有用户可见文案一律走 i18n（zh-CN / zh-TW / en），禁止在 JSX/组件里硬编码中文或英文文案。**
新增页面、feature 组件、弹层、toast、placeholder/title/aria-label 全部适用；这是代码评审的硬性项。

- **用法**：组件内 `const { t } = useI18n()`（`@/shared/i18n/provider`），`t('ns.key', { n: 3 })`；
  非 React 纯函数模块用 `translate(getLang(), 'ns.key', …)`（`@/shared/i18n/core`）。占位符写作 `{name}`。
- **词典**：`src/shared/i18n/{zh-CN,zh-TW,en}/<ns>.ts` 按命名空间分文件，key 以 ns 为前缀（如 `home.todo.title`）。
  zh-CN 是基准，三份词典 key 必须完全一致——**缺译由 `pnpm typecheck` 编译期报错拦下**，不靠人肉对齐。
  新领域加新命名空间：三份 `<ns>.ts` + 在各语言 `index.ts` 注册。
- **禁止动态拼 key**（`` t(`x.${v}`) `` 类型会挂）。枚举型标签用显式映射：
  `const LABELS = { a: 'ns.key.a' } as const` → `t(LABELS[v])`。
- **zh-TW 用台湾习惯用词**，不做简繁字面转换：设置→設定、保存→儲存、文件→檔案、网络→網路、
  视频→影片、软件→軟體、默认→預設、搜索→搜尋、插件→外掛程式、主屏→主畫面、日程→行程、服务器→伺服器。
  品牌：zh-CN 仓鼠Hub / zh-TW 倉鼠Hub / en HamsterHub。
- **「AI Agent」是不翻译的固定术语（2026-09-13 约定）**：中文界面一律直书 `AI Agent`（含空格），
  禁止写成「代理」；三语统一显示 AI Agent。例外：`Agent CLI`（指命令行程序本体）保持原样。
- **不翻译**：代码注释（保持中文）、日志（console/eprintln）、数据匹配关键词
  （如 `apps/category.ts` 的分类关键词、`todo/nlp.ts` 的解析词）、后端返回的数据值
  （应用名/插件名/城市名/壁纸名）。界面语言切换必须即时生效，不得要求重启。
- **Rust 侧面向用户的文案**（托盘菜单、天气状况等）按 `settings.behavior.language` 输出，
  参照 `lib.rs` 的 `tray_labels` / `core/weather.rs` 的 `describe_code`；语言变化后需重建的
  UI（托盘）在 `settings_save` 里刷新（`crate::refresh_tray_menu`）。
- 语言持久化字段：`settings.behavior.language`（默认 `zh-CN`），由 `I18nProvider` 消费；
  切换即 `patch({ behavior: { language } })`。

## bench 域红线（改 crates/ 前必读）

- **adapters 是纯函数层**：每 agent 一文件，顺序 = 路径常量 → detect → read_mcp → render_mcp；
  不做任何写入 IO（目标文件现状由引擎传入）；改适配器必须同步维护 fixture 往返测试。
- **runtime「宿主零侵入」**：只拉起官方 CLI / 转发流 / resize / 终止，**不写任何 Agent 配置文件**；
  输出按 8ms/32KB 合帧转发，别改成直转发。
- **index 对 Agent 落盘文件只读**：sessions.db（WAL+FTS5）是派生数据、可随时重建，schema 由
  hamster-index 自管，**不进 hamsterhub.db 迁移链**。
- **桌面 MCP 双令牌**：用户级长效令牌（外部宿主接入，存 settings）+ 会话令牌（bench 助手，急停可吊销）；
  computer-use 受 settings 开关门控 + `agent/audit.jsonl` 调用审计，别绕过。
- 域层 crate（hamster-{core,adapters,runtime,index,mcp}）保持**零 Tauri 依赖**；
  `src-tauri/src/bench` 只是薄胶水层，域逻辑别写进 commands。

## 提交门禁（scripts/check.ps1 一键跑）

- check.ps1 实跑：`pnpm typecheck` + `pnpm build`（tsc --noEmit + vite build）+
  `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` + `cargo test`（manifest 指 src-tauri）。
- **未进门禁、改到相关文件时本地自跑**：`pnpm test`（vitest：layout/stream-registry/timeline-logic 等）、
  `pnpm e2e`（Playwright 冒烟，mock 层驱动）。
- CI（.github/workflows/ci.yml）：frontend(typecheck+build) / e2e(Playwright) / rust(fmt+clippy+test)
  三 job；tag 触发 release.yml 出 NSIS。

## 已知坑（踩过的，别再踩）

踩坑档案已单独立档：[docs/05-pitfalls.md](docs/05-pitfalls.md)（vite 重载竞态、看门狗 PID 复用、
WebView2 a11y 陈旧缓存、FTS5 拼音分词等 21 条）。
**排障、写系统级代码、或遇到「改了没生效」时先查它**；新踩的坑往那里追加（症状 → 根因 → 修法），
并同步评估是否要回写本文的流程或红线。

