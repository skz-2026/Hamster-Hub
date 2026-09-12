# Agent.md — 仓鼠Hub 开发规范（Agent / 人类通用）

> 本文件是本仓库的一等公民开发规范。做任何改动前先读完本文；与 docs/ 冲突时以本文为准并回写文档。

## 项目一句话

Windows 上的 iOS 风格桌面（对标水豚hub 的 macOS 风）：Tauri 2 + React 18 + TS + SQLite。
AI 走「集成 agent 为大脑 + 桌面 MCP」路线，不自研 agent（2026-09-12 决议，见文末状态）。
详细设计见 [docs/01-product-plan.md](docs/01-product-plan.md)（产品）、[docs/02-sdd.md](docs/02-sdd.md)（SDD）、
[docs/03-tech-roadmap.md](docs/03-tech-roadmap.md)（技术路线）、[docs/04-architecture.md](docs/04-architecture.md)（代码架构）。

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

### IPC mock 层（第 1 层的机制）

- 契约单一事实源：Rust 命令 + `#[specta::specta]` → `pnpm codegen:ipc` 生成 `src/shared/types/ipc.ts`（**勿手改**）。
- **应用代码一律 `import { commands, events } from '@/shared/lib/ipc'`**，禁止直连 `@/shared/types/ipc`
  （lib 层在浏览器环境自动替换为 `ipc-mock.ts` 的假实现；types 只放类型）。
- mock 数据（假应用列表/设置/事件总线）在 `src/shared/lib/ipc-mock.ts` 维护，`home.layout` 持久化到 localStorage。
- 新增 Rust 命令后：改 Rust → `pnpm codegen:ipc` → 若浏览器也要用，在 mock 层补同签名假实现。

## 目录速查

```
src/
  app/            壳：AppShell(路由/事件导航)、TitleBar、SideNav、providers、routes/{home,workbench,...}
  features/
    home/         iOS 主屏：HomeScreen(交互)、AppIcon(squircle)、layout.ts(纯函数+单测)、hooks
    settings/     设置域 hooks
  shared/
    lib/ipc.ts    IPC 出口（tauri/浏览器自动切换）★ 应用唯一入口
    lib/ipc-mock.ts
    components/   通用组件（WebDebugBar 等）
    types/ipc.ts  tauri-specta 生成物（勿手改）
src-tauri/
  src/
    commands/     IPC 薄层（settings/system/apps/desktop）
    core/appindex.rs   开始菜单 .lnk 扫描 + 图标缓存
    desktop_mode.rs    系统接管/还原/巡检/看门狗拉起/窗口形态
    store/        SQLite(rusqlite) + settings/config
    events.rs     specta 强类型事件
  crates/
    hamster-platform/  Win32 共享层：taskbar/desktop_icons/snapshot/icons/shell（主程序与看门狗共用）
    hamster-watchdog/  看门狗 bin（主进程死亡→按快照还原系统）
  app.manifest    comctl32 v6 + DPI（测试二进制必需，勿删）
```

## 提交门禁（scripts/check.ps1 一键跑）

- 前端：`pnpm test`（vitest）+ `pnpm build`（tsc --noEmit + vite build）
- Rust：`cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test --workspace`
- CI（.github/workflows/ci.yml）同门禁；tag 触发 release.yml 出 NSIS

## 已知坑（踩过的，别再踩）

1. **vite 依赖重优化会触发整页重载**（lockfile/deps 变化后首次 dev）：表现为路由重置、点击落空、
   「按钮没反应」。先怀疑重载竞态，等几秒重来一次再排查代码。
2. **强杀 tauri dev / cargo 后可能跑旧二进制**（cargo 指纹 mtime 竞态）：杀完进程先 `cargo build` 再起，
   并核对 `target/debug/hamster-hub.exe` 时间戳。
3. **WebView2 的 a11y 树是陈旧缓存**：DOM 内容变了树不更新。验证 UI 状态用截图，别信 a11y。
4. **桌面壳里不要用 alert() 调试**（可能阻塞异步链），dev 版已启用 devtools（F12），优先 console。
5. **合成点击坐标在桌面壳里不可靠**：UI 验证一律在第 1 层做；第 2 层用程序断言（PowerShell/文件/进程）。
6. **vite watch 必须忽略非源码目录**（已配：`src-tauri/e2e/scripts/test-results`）：
    cargo 写 target 里的 dll 会 EBUSY 崩 vite；e2e/scripts 文件变动会触发无谓重载甚至进程退出。
7. **本机网络**：git/npm 全局有死代理 `127.0.0.1:31181`（已用项目 `.npmrc` 覆盖）；cargo 需要
   `NO_PROXY='*'` 前缀；https 偶发吊销检查离线 → curl 加 `--ssl-no-revoke`，cargo 用 `.cargo/config.toml`
   的 `check-revoke=false`（已配）。
8. **测试二进制必须嵌 manifest**（build.rs 已做）：comctl32 v6 缺失会 STATUS_ENTRYPOINT_NOT_FOUND。
9. **水豚hub 常驻且提权**：测系统接管前确认任务栏状态基线（IsWindowVisible），别拿截图肉眼判断。
10. **Alt+Space 是 Windows 系统菜单保留键**，RegisterHotKey 大概率失败：已做容错注册（日志告警 +
    托盘菜单兜底）；Spotlight 默认热键后续可配。
11. **看门狗判活必须校验进程镜像名**（QueryFullProcessImageNameW 包含 hamster-hub）：裸 OpenProcess
    会把「死后被系统复用的 PID」判活，导致崩溃后永不还原（真机踩过）。
12. **主进程死后 stdio 管道即断**：子进程（看门狗）里任何 eprintln 都会 broken-pipe panic——
    看门狗日志一律写 `%APPDATA%\com.hamsterhub.app\watchdog.log`。
13. **tasklist //FI 有时序性延迟**：断言进程存在/退出用 `tasklist | grep`（全量）或 PowerShell
    `Get-Process`，且提取 PID 别用内存数字尾巴。
14. **tauri dev 的 file watcher 会在源码变更时重启 app**：正在跑系统矩阵时不要改 Rust 源码；
    杀主进程测试会让 `pnpm tauri dev` 整体退出（exit 101/1），属预期。
15. **多窗口（main/spotlight）共享 QueryClient 配置**：`refetchOnWindowFocus` 不能关——
    Spotlight 隐藏期间首屏查询失败/为空会被 `staleTime` 缓存，窗口重显靠 focus 重取救回。
16. **FTS5 unicode61 分词**：CJK 连续串是单 token（"微信截图"整体），中文子串检索必须走拼音列；
    拼音列存双形态（`wei xin jie tu` + `weixinjietu`）分别支持音节 AND 与整词前缀。
17. **纯 ASCII 名（Visual Studio Code）没有拼音音节**，首字母缩写（vsc）需在 pinyin_cols 里
    单独提取「词首字母」追加进全拼列与首字母列，否则缩写检索永远空。
18. **tauri bundle resources 不接受 `../` 逃逸路径**（build.rs 阶段就报 doesn't exist）：
    外部构建产物（watchdog exe）先复制到 `src-tauri/resources/` 再以相对路径声明。
19. **主窗口禁止 `transparent: true` + `apply_acrylic`**（2026-09-11 真机踩坑）：WebView2 透明合成
    会把整窗渲染成"透出背后内容"的半透明灰（随背后窗口颜色变化，用户看就是配色全乱），
    且 DOM 实底背景色也压不住。主窗口保持不透明，深浅色观感全靠 CSS 背景承担；
    透明 + 毛玻璃只用于 Spotlight 覆盖层窗口。另：`bg-[var(--bg)]` 这类 Tailwind 任意值
    在壳内不可靠时，用内联 `style={{ background: 'var(--bg, #17151b)' }}` 兜底。
20. **"改了没生效"先查是不是旧实例**：single-instance 插件会让新进程退出并唤起旧窗口——
    打过 NSIS 包后 `target/release/` 的旧 exe 若在运行，`tauri dev` 的新 debug 实例永远起不来。
    用 `Get-Process -Id <pid> | Select Path,StartTime` 查运行实例的来源与启动时间，
    `taskkill` 全部 hamster 进程后再启动。
21. **useNavigate 的 navigate 标识随 location 变化，不能作为「含导航副作用」effect 的依赖**
    （2026-09-12 踩坑，浏览器程序化定位）：`[navigate]` 依赖会让 effect 在**每次路由变化时重跑**；
    若 effect 里做「查状态→强制 navigate」（如 desktopModeIsActive→/desktop），用户的一切导航
    都会在 ~3ms 内被 replace 弹回，表现为**页面所有按钮点击"无效"**（事件已到 DOM、navigate 已执行、
    又被回滚）。症状与坑 1（vite 重载竞态）高度相似，用 pushState 补丁时间线区分。
    修法（AppShell）：初始对齐 `[]` 依赖 + `window.location.hash` 守卫只跑一次；事件监听 effect 用
    alive 标志兜底「注销函数晚于卸载 resolve」造成的 StrictMode 重复监听。

## 状态与下一步（滚动更新）

- ✅ M0 骨架（NSIS 2.27MB、CI、specta 流水线、托盘/单实例/自启）
- ✅ M1 W1：hamster-platform / hamster-watchdog / desktop_mode / appindex（31 个真实图标）
  / iOS 主屏 UI
- ✅ 浏览器层 UI 验证（2026-09-11）：主屏网格/搜索/长按编辑抖动/拖拽建夹/文件夹打开层/退出还原全通过
- ✅ **桌面壳系统矩阵（2026-09-11，T1-T4 全 PASS，程序断言）**：
  T1 进入（快照/看门狗/任务栏隐藏）· T2 退出（还原/快照删/看门狗退）·
  T3 崩溃还原（杀主进程 2s 内任务栏还原，顺带修复看门狗 PID 复用误判与 broken-pipe 崩溃两个缺陷）·
  T4 explorer 重启 rehide 巡检
- ✅ M1 W2 Spotlight：独立全屏覆盖窗口 + 全局热键（Alt+Space 容错注册 + 托盘菜单兜底 +
  Ctrl+Alt+D 桌面模式热键）+ 应用/网页搜索 + 键盘导航，真机与浏览器双层验证通过
- ✅ M1 W3 文件索引（2026-09-11）：迁移 0002（FTS5 外部内容表 + 触发器）、
  fileindex 核心（~ 根展开/分类/拼音双形态列/批量扫描/bm25 检索）、file_search 等 4 命令、
  Spotlight 文件分组（类型彩色图标）；检索支持 中文词→音节、整词拼音前缀、首字母缩写
  （「截图」/「weixin」/「wxjt」三路均真机验证命中）；顺带修复 QueryProvider 关闭
  refetchOnWindowFocus 导致多窗口空结果被永久缓存的问题
- ✅ **M1 W4 收官（2026-09-11）**：
  - core/pinyin.rs 共享模块（拼音双形态 + ASCII 词首缩写 vsc；appindex/fileindex 共用）
  - 迁移 0003 app_fts + app_search 命令（拼音/首字母/中文名 + usage 频次加权），Spotlight 应用路接入
  - notify 增量 watcher（防抖 5s 全量重扫 + FileIndexUpdated 事件）
  - 热键从 settings 读取（search.hotkey / behavior.desktop_mode_hotkey，重启生效）
  - **NSIS 打包验收**：HamsterHub_0.1.0_x64-setup.exe 2.63MB（watchdog 以 resources 打入安装目录）
  - **搜索基准达标**：10 万行 p95=42.6ms（验收 ≤100ms），基准在 `src/bench.rs`
    （`cargo test --release -p hamster-hub --lib search_bench -- --nocapture`）
- ✅ **M2 阶段一：仪表盘 + 桌面整理（2026-09-11，方向调整：AI 后置，先做桌面整理与 UI 打磨）**：
  - 工作台从占位卡片重做为毛玻璃仪表盘（暗色渐变底 + 光晕），六张真实数据卡：
    时钟(农历) / 天气(Open-Meteo 免 key + WMO 码映射 + 30min 缓存，命令层阻塞/异步分离) /
    待办(真实 CRUD) / 倒数日(周末/新年自动计算 + 自定义项) / 最近文件(文件索引) / 常用应用(usage 频次)
  - 应用页重做：7 分类药丸（沟通/办公/开发/娱乐/工具/系统/其他 + 计数）+ 图标宫格 +
    关键词自动分类（features/apps/category.ts 规则表）+ 单击启动
  - Rust 新增：store/todo、commands/todo×4、core/weather、core/countdown、commands/dashboard×5、
    fileindex::recent；reqwest(rustls) + chrono 依赖
  - 浏览器验证截图：工作台仪表盘、待办添加/勾选链路、应用页筛选（开发→VS Code/ZCode/Git）全通过
  - 注意：tauri async command 不能持 rusqlite 连接跨 await（非 Send）——weather_get 采用
    「阻塞段读缓存 → await 网络拉取 → 阻塞段写库/兜底」三段式
- ✅ **M2 阶段二：日程/便签/倒数日管理（2026-09-11，真机验证通过）**：
  - store/note（便签 CRUD+置顶 / 倒数日自定义项 CRUD+日期校验）+ commands/note×8
  - 日程页重做：月视图（农历、今日高亮、倒数日 emoji 标记、翻月）+ 倒数日管理（添加即联动月历）+
    便签墙（添加/双击编辑/置顶/删除）
  - 设置页新增天气城市选择（改城市即刷新工作台天气卡）
  - 真机验证：天气真实拉取（北京 21° 晴）、最近文件显示真实截图、月历联动、便签增删置顶
- ✅ **M2 阶段三：主屏小组件 + 热键 UI + E2E（2026-09-12）**：
  - iOS 主屏小组件：layout.ts widget 槽位（widget:clock/weather/todo/countdown，非法类型/重复自动清理）+
    HomeWidget 玻璃卡（2 列宽，真实数据 hooks）+ 编辑模式「+小组件」菜单 + 移除按钮；18 vitest 用例
  - 设置页热键编辑（Spotlight/桌面模式两行，重启生效）+ 天气城市输入
  - **Playwright E2E**：e2e/smoke.spec.ts 4 场景（工作台+待办 / 应用筛选 / 日程便签+倒数日联动 /
    桌面模式进出），`pnpm e2e`（自动起 5174 dev server）；CI 新增 e2e job
  - vitest 与 playwright 测试文件分离（vite.config test.exclude）
- ✅ **M2 阶段四：系统信息小组件（2026-09-12，用户新增需求）**：
  - sysinfo 0.36 采集：`commands/sysinfo.rs`（SystemMonitor 共享态）——
    `system_stats`（内存/每分区磁盘/CPU 占用+核数+型号/温度传感器尽力而为）/
    `process_list`（按内存或 CPU 排序 TopN）/ `process_kill`
  - 主屏新增「电脑状态」「任务管理器」小组件：进度条（内存/磁盘/CPU）+ 温度行 +
    进程 Top 列表悬停结束进程；浏览器截图验证
  - 真实数据由 Rust 真机测试断言（snapshot_on_real_machine：内存>0/磁盘非空/进程非空）
  - 修复：appindex 后台扫描与并发写撞 WAL 锁（busy_timeout 3s + 3 次退避重试）
- ✅ **M2 阶段五：桌面模式对齐水豚hub（2026-09-12，用户新增需求）**：
  - 启动即进桌面模式：`behavior.desktop_mode_on_launch` 默认 true（设置页有开关，重启生效）
  - 桌面模式形态重构：全屏**应用壳**（侧栏 + 页面）+ **底部常驻 DockBar**（所有页面可见，单击启动），
    默认页 = 工作台仪表盘；侧栏新增「主屏」（iOS 图标网格，自含交互 Dock）与「退出桌面模式」
  - DockBar 组件（features/home/DockBar.tsx）为**布局底部独立区域**（flex 流内，非悬浮——
    内容不被遮挡，用户反馈修正），macOS 风：50px 图标、悬停放大 1.25 上浮、tooltip；
    HomeScreen 交互 Dock 同步悬停放大；两者分离（展示+启动 vs 拖拽/编辑）
  - E2E 更新为「进入→工作台+Dock→主屏→退出还原」全通过
- ✅ **M2 阶段六：桌面接管形态对齐水豚hub（2026-09-12，用户两次纠偏后定稿）**：
  - **桌面模式 = 全屏接管整个桌面，完全照水豚hub**：无顶部菜单栏、无侧栏、无窗口控件，
    默认页 `/desktop`（routes/desktop）= 全屏壁纸（复用主屏壁纸）+ 中央 🐹问候语/超大时钟/
    农历 + 大搜索框（点击进搜索页）+ 4 张玻璃小组件卡（复用 HomeWidget）+ 快捷入口药丸
    （工作台/日程/应用/文件/设置——桌面模式无侧栏的页面导航）
  - **贴边通栏任务栏完全替代系统任务栏**（系统任务栏已被 desktop_mode 隐藏）：x=0 贴边、
    100% 屏宽、贴底零间隙、无圆角、高 ~63px；**左端主屏/搜索/设置入口 → 竖分隔线 → 定制组 →
    分隔线 → 常用组 →（弹性空白）→ 红色退出图标 + 时钟/日期**（系统托盘位）；图标 42px
  - 分组来源：定制 = 主屏编辑模式拖入（layout.dock）；常用 = `top_apps` 频次 TopN 去重取 6；
    悬停放大上浮 + tooltip + 活动路由指示点
  - **窗口化模式**（退出桌面模式后）：红绿灯 TitleBar + SideNav + 页面 + 居中胶囊 Dock（常驻）；
    DockBar 单组件双形态（desktop prop 分支，胶囊 50px 图标）
  - 主屏（/home iOS 网格）顶栏新增「桌面」按钮返回 /desktop；AppShell 桌面模式隐藏 SideNav
  - **修复两个关键缺陷**：① AppShell navigate 依赖坑（见已知坑 21，症状=全页面点击失效）；
    ② WebDebugBar 与贴边任务栏的指针遮挡（上移 bottom-24）
  - E2E 适配：进入桌面模式断言 `#/desktop` + 问候语；退出断言回 `#/` 且胶囊 Dock 常驻；
    「退出桌面模式」选择器限定 `.ios-dock`（WebDebugBar 按钮文字重名）；应用页 Steam 断言限定 `main`
  - 浏览器验证（test-results/dock-verify/）：贴边任务栏 100% 宽/零间隙/时钟、搜索跳转、
    主屏往返、退出还原，程序断言 + 视觉分析全通过
- ✅ **M2 阶段七：深度接管接线（2026-09-12，真机全矩阵验证）**：
  - 用户第三次纠偏：「底部完全替代系统任务栏、全屏接管整个桌面」——此前 desktop_mode.rs 是
    **work-area 模式**（窗口只贴工作区、Windows 任务栏保持可见，深度接管在 hamster-platform
    里建好但未接线），用户看到系统任务栏仍在自然不符需求
  - 重写 desktop_mode.rs 完整接线：enter = 快照落盘 → **隐藏任务栏（Shell_TrayWnd/副屏）+
    隐藏桌面图标（注册表 HideIcons+广播）** → 拉起 hamster-watchdog → 窗口铺满**整屏**
    （显示器全尺寸非工作区）→ 5s 巡检（explorer 重启任务栏复活则重隐藏）；
    exit = 按快照还原 + 删快照（看门狗 ≤2s 自退）
  - **真机程序断言全过（T1-T3）**：进入（任务栏隐藏/看门狗/快照/整屏壁纸）· 热键退出
    （Ctrl+Alt+D：任务栏还原+看门狗自退+快照清理）· 崩溃还原（taskkill 主进程 →
    watchdog.log 记录 restoring，任务栏数秒还原）；主屏往返交互截图确认（31 真实图标恢复）
  - 顺带修复：① SQLite「database disk image is malformed」（强杀旧实例损坏 WAL，删库重建，
    图标重扫 31 个）；② AppShell 启动 hash 守卫（Tauri 初始 URL hash 为 '' 非 '#/'，
    真机启动进不了 /desktop——浏览器测试 URL 总带 '#/' 故未暴露，见坑 21 修法）
  - **用户看不到新效果的三大元凶（本次全中）**：旧实例常驻（坑 20）+ 旧 vite 占 5173 使
    tauri dev 起不来 + 损坏 DB 空图标——排查顺序：进程→端口→日志→DB
  - 壳内合成坐标点击退出钮不可靠（坑 5 复现：隐藏任务栏的 RECT 仍占据命中测试），
    系统级验证一律用热键/程序断言
- ✅ **M2 阶段八：任务栏独立置顶窗（2026-09-12，用户第四次纠偏定稿）**：
  - 用户澄清：底部任务栏是**助手自己的**，替代系统任务栏并**持续渲染**——打开任何应用
    都不能遮挡它。而阶段七把 DockBar 放在主窗口内部，启动其它应用后会被盖住
  - **架构：taskbar 独立窗口**（tauri.conf 静态定义 + capabilities 加 label）：
    always-on-top + skipTaskbar + focusable(false)（可点击不抢焦点，WS_EX_NOACTIVATE），
    贴主窗口所在屏底条（72 逻辑 px，含悬停放大余量）；主窗口 = 屏幕减去底条
  - 前端：`/taskbar` 路由（AppShell 之外，同 Spotlight 模式）渲染 TaskbarPage（深底 DockBar
    桌面形态）；AppShell 在**真机桌面模式不再内嵌 Dock**（`!desktop || !isTauri` 才渲染），
    浏览器预览/E2E 保留内嵌路径不受影响；DockItem 加原生 title 提示（窗口条内自绘 tooltip
    会被窗口边界裁剪）
  - desktop_mode.rs：enter 布局改为 apply_takeover_layout（主窗口留底条 + taskbar 窗
    定位/置顶/显示）；exit 隐藏 taskbar 窗。快照/隐藏/看门狗/巡检不变
  - **真机验证**：taskbar 窗 rect T=1008/高81/Topmost=True/NoActivate=True（EnumWindows 断言）；
    **开最大化终端盖住主窗口后任务栏仍完整置顶可见**（截图确认）——「持续渲染不被遮挡」达标；
    系统任务栏保持隐藏；clippy/test/e2e/build 全绿
  - 排查注记：tauri dev 的 cargo watcher 会在 Rust 源码变化时自动重启 app 并重新进入桌面模式
    （TaskStop 杀不掉 watcher 链，须按进程名清 node/cargo）；测试清理要先杀主进程让看门狗
    还原，**再**清看门狗（顺序反了会留下隐藏的任务栏 + 孤儿快照，需手动 ShowWindow 清理）
- ✅ **M2 阶段九：真机三大痛点修复（2026-09-12，用户实测反馈「UI 丑/功能不正常/图标不显示」）**：
  - **AppHang 挂死（功能不正常的根源）**：微信 xwechat_files 在 Documents（watched root）里
    每几秒刷 apm 指标 → 防抖后立即重扫的无限循环 → DB 长期高压 → 主线程卡死被 Windows
    强杀（事件日志 AppHangB1，exit code 1）。修复：fileindex 重扫后 **30s 冷却**
    （`RESCAN_COOLDOWN`，冷却期变更静默丢弃）
  - **图标 84/115 缺失**：系统工具 lnk 的图标声明是 `shell32.dll,-23` 这类带资源索引的
    字符串，按路径 exists() 过滤全部落空。修复：appindex 提取链末尾**对 .lnk 本身**
    SHGetFileInfo（Windows 自动解析资源索引）→ 真机 115/115 全部有图标
  - **默认 Dock 垃圾**：buildDefaultLayout 取扫描顺序前 4 → Administrative Tools 等
    系统工具进了 Dock。修复：KNOWN_DOCK_APPS 常见应用优先（微信/QQ/钉钉/Chrome…），
    无匹配留空；存量脏 home.layout 已清；单测同步改 19 用例全绿
  - **任务栏窗口内导航 bug**：taskbar 独立窗口点「主屏/搜索/设置」会把路由载入 72px
    窗口条（两 WebView 各有路由器）。修复：DockBar 检测 `#/taskbar` 时改
    `emitTo('main', 'hamster:navigate')`，AppShell 监听后导航主窗口
  - **TaskbarPage 视觉**：深色渐变 + 顶部 1px 高光线（替代平黑底）
  - 真机验证：图标 115/115、任务栏微信/QQ/钉钉/Chrome 真实图标、跨窗口导航正常、
    **90s 稳定性观察存活且 responding=True**（修复前 ~1min 即挂死）
- ✅ **M2 阶段十：macOS 视觉打磨（2026-09-12）**：
  - 壁纸集新增 macOS 桌面风三张（midnight 深夜网格渐变 / aurora 极光 / dune 沙丘，
    多层 radial 光晕 + 纵向暗角），默认 midnight（原亮橙胡萝卜太吵）
  - 桌面主页：96px 超细字重大时钟（-0.02em 字距）+ Spotlight 式搜索框
    （h-52px/黑 30% 底/内阴影高光/hover 微光）+ 组件卡 20px 圆角玻璃 + 顶部景深光晕
  - 任务栏：图标 44px + 间距 2.5、深色三段渐变 + 顶部 1px 高光 + 底部暗角、
    系统图标底座提亮（white/16 + ring/22）、指示点改绝对定位（图标垂直居中 +
    发光 + 悬停 45% 预览）
  - 窗口化 TitleBar 标题绝对居中（macOS 惯例）；全局字体加 SF 系前置 + 选区色
  - 真机视觉分析两轮自审：壁纸/时钟/搜索框/组件/任务栏全部达标，
    修掉指示点弱与图标偏移两处粗糙点；19 单测 / 4 E2E / build 全绿
- ✅ **M2 阶段十一：AI 助手接入（2026-09-12，用户授权自主设计「精美桌面助手 + Agent 就绪」）**：
  - Rust `commands/ai.rs`：`ai_chat(messages)` → OpenAI 兼容 `/chat/completions`
    （配置读设置页 ai.*；未配置返回 `AI_UNCONFIGURED` 错误码；三段式不持 DB 锁跨 await；
    非流式，SSE 流式留作后续）；specta codegen + ipc-mock 假实现
  - `/agent` 对话页（routes/agent）：星光标题栏/欢迎态+建议问题/用户橙泡+助手玻璃泡/
    三点打字动画/Enter 发送 Shift+Enter 换行（含中文输入法 isComposing 保护）/
    清空会话/AI_UNCONFIGURED → 黄色引导条「去配置」直达设置页
  - 桌面主页：快捷入口行首位新增**橙色 AI 助手药丸**（与玻璃药丸区分，Agent 入口）；
    四段式入场动画（rise-in 0.55s iOS 曲线，80ms 错落）
  - 真机验证：AI 入口/欢迎态/发送→未配置引导全链路通过；21 cargo test / 19 vitest /
    4 E2E / clippy / build 全绿
  - **Agent 后续扩展挂点**：ai_chat 加 tools/function-calling → 桌面操作（启动应用/
    整理图标/查日程）；流式 SSE；Spotlight「问 AI」路复用
- ✅ **M2 阶段十二：控制中心 + Spotlight 问 AI + M1/M2 真机复验（2026-09-12）**：
  - **控制中心**（产品规划 P1 补齐）：桌面右上角触发 → iOS 风玻璃面板：磁贴（Wi-Fi/蓝牙
    → ms-settings: 跳系统设置、深浅主题、壁纸轮换）+ **真实主音量滑条** + 快捷动作
    （AI/设置/退出接管）；平台层 `hamster-platform::volume`（IAudioEndpointVolume）
  - **音量真机踩坑**：IMMDevice QueryInterface 不应答 IAudioEndpointVolume（E_NOINTERFACE），
    必须走文档规定的 `IMMDevice::Activate`；windows crate 的 Activate 方法被
    `Win32_System_Com_StructuredStorage`+`Win32_System_Variant` feature 裁剪，需显式开启。
    真机 roundtrip 测试 `volume_roundtrip_real`（--ignored）+ COM 读回对照验证
  - **Spotlight 问 AI**（规划 3.2「尾部入口」）：结果尾部「问 AI」行 → kv 暂存问题 →
    emitTo 主窗口 /agent → 自动发送（pendingQ 一次性消费）
  - **M1/M2 真机复验矩阵**（本轮大改后系统性回归）：
    T1 进入 ✓ · T2 热键退出还原 ✓ · T3 崩溃看门狗还原 ✓ · **T4 explorer 重启重隐藏 ✓**
    （注意：强杀 explorer 后不一定自动重启，需手动 Start-Process 才构成有效测试）·
    应用索引 115/115 图标 ✓ · 文件索引重扫+冷却 ✓ · 主屏/任务栏交互 ✓ · 天气真实数据 ✓ ·
    AI 未配置引导 ✓ · 音量 get/set 分层验证 ✓ · 任务栏 appLaunch 接线（浏览器）+
    lnk ShellExecute 真机 ✓（CUA 无法点击任务栏图标有效区——隐藏任务栏 RECT 遮挡，
    属工具限制非产品缺陷）
  - 全门禁绿：21 cargo test / 19 vitest / 4 E2E / clippy -D warnings / tsc / build
- ✅ **M2 阶段十三：用户实测四问题修复（2026-09-12）**：
  - **① 常用组永远空（真 bug）**：top_apps SQL 用 `count(u.id)` 而 usage_log 根本没有 id 列
    （0001_init.sql 只有 target_key/kind/at）→ 查询必错、前端静默吞掉。修为 `count(*)`；
    另加启动后 invalidate + 30s 轮询（跨窗口无法互发 invalidate）。真机注入 usage 验证
    Steam 出现在常用组 ✓
  - **② 图标模糊**：提取用 SHGFI_LARGEICON（32px）放大到 44-52px 必糊。icons.rs 改为
    **SHIL_JUMBO 系统图像列表（256×256）**优先、LARGEICON 回退（需 Win32_UI_Controls
    feature 取 IImageList）；清空 icons 缓存重扫 → 真机 115/115 全部 256px，任务栏锐利 ✓
  - **③ 任务栏定制**：右键定制组图标 → 移除菜单（贴窗口顶渲染防 72px 裁剪）；
    「+」按钮 → emitTo 主窗口弹出 DockPickerModal（搜索+宫格，受 DOCK_CAPACITY 约束）。
    **useHomeLayout 重构**：去本地 draft，Query 缓存为单一事实源（同窗口全实例即时一致）
    + 持久化后 tauri emit 广播（跨窗口失效重取）+ DOM 事件兜底（浏览器）
  - **④ 点任务栏不切视图**：主窗口被浏览器等盖住时导航事件只改路由不提前台。
    AppShell 的 hamster:navigate 监听补 `show()+setFocus()`（浏览器打开→切回仓鼠视图）
  - 浏览器验证：+ → 选 Edge → 同实例任务栏立即出现 ✓；全门禁绿
- 剩余可选项（阶段十三时点）：代码签名（发布前必须）· 通知中心/灵动岛/锁屏 · macOS 预研 ·
  应用搜索拼音路已在 M1 完成（app_fts）
- ✅ **M2 阶段十四：上游域层整合——代理工作台 MVP（2026-09-12，来源 `D:\ksa\上游`）**：
  - **方案**：原生深度整合（用户拍板：GUI 对话优先）。vendor 4 个零 Tauri 依赖的 上游域 crate
    到 `src-tauri/crates/`（hamster-core/adapters/runtime/index，crate 名不改、Apache-2.0 头保留，
    上游源仓库不动）；管理域（MCP 同步）/交付看板/PTY 终端/远程访问/插件系统留待后续阶段
  - **Rust**：workspace.members +`[workspace.package]`/`[workspace.dependencies]`（rusqlite/thiserror/
    chrono 与主库同版无冲突）；`.cargo/config.toml` 加 `TS_RS_EXPORT_DIR`（ts-rs 导出进 target，防污染）
  - **specta 适配**：vendor 类型补 `specta::Type` derive（specta = rc.22 + **derive feature 必须显式开**，
    单独 `-p` 编译时无 tauri-specta 帮忙开 feature）；导出用
    `Typescript::bigint(BigIntExportBehavior::Number)`（i64 时间戳→number，避开 bigint）
  - **流式通道改事件**：tauri-specta rc.21 对 ipc Channel 的 JS 侧生成「Coming soon」→
    GUI 对话数据改走两个强类型事件：`BenchStreamEvent`（serde transparent，payload=StreamEvent）
    + `BenchStreamExit`；前端 `useBenchEvents` 订阅 → stream-registry
  - **装配层**：`src/bench/`（BenchContext：store_root=%APPDATA%/com.hamsterhub.app/agent/，
    sessions.db 由 hamster-index 自管 schema 不入主库迁移链）+ 15 条 `bench_*` 命令
    （stream create/send/interrupt/kill/list + 历史/索引 Recall 7 条 + agents 2 条）；
    async 命令直调 rusqlite（State 借用不进 spawn_blocking——上游同款）
  - **前端**：**assistant-ui 0.15**（用户选型）——`useExternalStoreRuntime` 桥接移植版
    stream-registry（Vue reactive → 写时复制 + useSyncExternalStore 版本号快照）；StreamRow→
    ThreadMessageLike 映射（工具行→tool-call part，错误行→data part）；消息 part 渲染器
    （markdown-it+hljs / 思考卡 / 工具卡+diff / 打字指示）；timeline-logic 纯函数随测试移植
    （PTY 视图阶段复用）；页面 `/bench`（SideNav「代理」+ 桌面 QUICK_LINKS 入口）
  - **mock 层**：假流式代理（turnStarted→思考→工具→agentDelta 切片→完成完整剧本）走
    mockEvents 总线（与真机事件同通道）；mock 命令名与生成绑定一致（benchXxx）
  - **验证**：vendor 随行 186 测试 ✓ · clippy（6 crate）零警告 ✓ · tsc/vitest 31 ✓ ·
    E2E +2 场景（GUI 流式对话/Recall 命中回看）6/6 ✓ · 浏览器全流程截图 ✓ ·
    codegen 绑定生成 ✓；**hamster-hub 本体 clippy 待应用进程释放 watchdog 文件锁后
    重跑 check.ps1**（运行中的 debug 实例锁 resources/hamster-watchdog.exe，非代码问题）
  - 已知取舍：组件 ≤250 行达标；bench 命令名带前缀避免与未来域冲突；RecallSearch 上下文
    渲染复用 SnapshotMessage（未走 timeline 折叠）；pty_* 命令与 SessionManager 留待终端模式阶段
- ▶ **进行中：任务栏托盘区镜像（工作区未提交）**——`hamster-platform/tray.rs`（UIA 枚举常驻+溢出区、
  BitBlt 图标裁剪、Invoke 点击转发）+ `tray_list`/`tray_click` 命令 + `TrayArea.tsx`（10s 轮询）。
  已知权宜：每次枚举任务栏闪烁 ~1s；右键转发未做（当前仅左键 Invoke 语义）→ 列入 M3 收尾项
- ▶ **方向决议（2026-09-12，用户确认）：AI 原生桌面——不自研 agent，集成 agent 为大脑**
  - 否决阶段十一挂点「ai_chat 加 tools/function-calling 自研桌面操作」：流式/tool-calling/MCP
    生态全由集成 agent CLI 原生承担，自研是重复造轮子，且维护面从「养一个 agent 引擎」缩到
    「一个 MCP server + 入口胶水」
  - 架构三层：**大脑** = bench 会话（hamster-runtime，BenchStreamEvent 流式通道已有）；
    **手脚** = 桌面 MCP server（既有 IPC 能力包 stdio MCP，经 hamster-core MCP 同步引擎注册进
    四家 agent，用户可关；高危工具白名单 + 调用审计）；**兜底** = ai_chat 降级（未装 agent 的
    用户 + 微任务分层路由——「加个待办」级别不拉起 coding agent）
  - 里程碑改版（已回写 docs/01 §4、docs/02 §3.9、docs/03 §3.4+§4）：
    M3 接管收尾&公开发布（托盘收尾/通知中心 v1/search+files 页/代码签名，v0.2.0）→
    M4 Agent 原生桌面（MCP server 5-8 核心工具 + Spotlight 问 AI 接 bench 会话 + 后台任务
    通知联动 + 主屏 agent 小组件，v0.3.0）→ M5 生态（灵动岛/锁屏/主题/插件/macOS 预研）
  - 通知中心从 P2 提为 M3 前置（agent 异步通知的承载）；macOS 预研继续后置
- ▶ **补强（2026-09-12，用户提出）：computer use 列入 M4——竞争点（NomiFun 等竞品均已具备）**
  - 竞品：NomiFun（糯米饭，nomifun-desktop，开源、React+Rust 同构栈）内置会话引擎原生
    browser-use + computer-use；上游源仓库有现成 `hamster-mcp` crate（MCP stdio server：
    browser 8 工具 CDP + computer 4 工具 xcap/enigo，注入走会话级 `--mcp-config` 不写用户
    配置——见 `D:\ksa\上游\crates\hamster-mcp` 与 `D:\ksa\上游\docs\verify-agent.md`，
    注入参考 上游仓库 channel 模块的 verify.rs）
  - 方案：M4 vendor hamster-mcp 为第 5 个 crate，desktop 工具组（应用/文件/待办/音量等
    既有能力）并入同一 server，形成 desktop/browser/computer 三组工具；注册主路 = 会话级
    `--mcp-config`（零侵入），hamster-core 同步引擎持久化注册可选
  - 安全红线：computer use 默认关闭；确认档位（每步/仅高危/全自动）；审计 + 截图留证；
    急停热键；操作期 UI 宣告。桌面接管形态下系统任务栏已隐藏、误操作更难自救——
    先过安全设计再放开能力
  - 已回写 docs/01（竞品表 NomiFun 行、§1.2 机会点、§3.2 功能表、M4 路线、风险表）、
    docs/02 §3.9、docs/03（§2 技术栈、§3.4 手脚+安全设计+验收、§4 M4、§5 风险）
- ✅ **M4 阶段一：Agent 原生桌面核心（2026-09-12，门禁全绿）**：
  - **vendor `hamster-mcp`（第 5 个 上游 crate）→ 桌面 MCP server**：保留 browser-use
    （chromiumoxide CDP 8 工具，快照 [ref] 编号）+ computer-use（xcap 截图 + enigo 鼠标键盘
    4 工具，截图以 image content 直供视觉模型）；新增 `desktop.rs` 工具组（同仓鼠目录约定，
    零 Tauri）：app_search / app_launch（查 exec_target→ShellExecute→落 usage_log 与常用组
    同源）/ file_search / file_open（**白名单**=settings 的 file_index.roots，~ 按 USERPROFILE
    展开）/ todo_list / todo_create / todo_set_done / volume_get / volume_set——共 21 工具。
    DB 查询 = FTS5 短语前缀（`"q"*`）+ LIKE 子串回退；连接 WAL + busy_timeout 3s 与主进程并发
  - **承载形态演进（用户两次决策定稿）**：① 独立 hamster-mcp.exe（上游原形）→
    ② 合并进主二进制（`hamster-hub.exe mcp serve` 子命令，argv 在 Tauri/单实例初始化前分发，
    无 GUI 无冲突）→ ③ **HTTP 内嵌为正式承载**（用户拍板「HTTP 通用，与 agent 无关」）：
    `src/mcp_server.rs` McpHub——127.0.0.1 随机端口 + 每会话 Bearer 令牌（Authorization 头 /
    `?token=` 双通道）+ Streamable HTTP 单帧回（通知 202 / GET 405）；stdio 子命令保留作
    调试/兜底。内嵌红利：急停=吊销令牌（在途调用立即 401）、CU 档位即时生效
    （settings_save → set_cu_allowed）、审计直写助手目录、主库连接免跨进程争用
  - **注入（统一 HTTP 形态）**：claude = `--mcp-config` 内联 JSON（type:"http"+headers）+
    `--allowedTools mcp__hamster-desktop`（Bash/Edit 等宿主工具 headless 自动拒绝）+
    `--append-system-prompt` persona；其它方言 = ACP `session/new` mcpServers（runtime
    StreamOptions 扩展 `mcp_servers` 字段透传，默认空=行为不变）+ persona 前缀回退
  - **`bench_assistant_create`（bench/assistant.rs）**：桌面助手预设——仓鼠 persona
    （settings.agent.assistant_persona 可覆盖）、不绑项目目录（cwd=store_root/assistant）、
    默认代理解析 claude→zcode→首个已装；spawn 公共路径从 bench_stream_create 重构为
    `spawn_stream_session` 共用；令牌 mint→bind→（kill 时）revoke 全生命周期
  - **安全门（hamster-mcp mcp.rs）**：computer_* 四工具由 cu_allowed 拦截（默认关），
    错误码 -32001 带开启引导；HAMSTER_CU_MODE env 供 stdio 模式；每次 tools/call
    审计 JSONL（ts/tool/args/ok/err）。**注意：安全门在 tools/call 层不区分传输**，
    HTTP/stdio 共用
  - **Settings.agent**（serde default 旧数据兼容）：computer_use_enabled（默认 false）+
    assistant_persona；settings_save 即时联动 McpHub
  - **前端**：codegen（benchAssistantCreate/AssistantCreateArgs/AssistantSessionInfo/
    Settings.agent）；ipc-mock 假助手会话（复用假流式代理）；`features/agent/AssistantPanel`
    （欢迎态/流式复用 bench ChatSessionView + stream-registry/失败回退快问/工具接入徽标）；
    /agent 页双模式（桌面助手默认 + 快问兜底，pendingQ 优先路由助手）；设置页新增
    CU 开关 + persona 编辑；e2e +1 场景（7/7 全过）
  - **门禁**：cargo fmt/clippy -D warnings/test 全绿（hamster-mcp 27 + hamster-hub 31 含
    mcp_server 6 个 HTTP 回环用例：令牌 mint/revoke 急停、CU 拒绝→放行、todo HTTP 往返、
    202/404/405/400）；pnpm build + vitest 31 绿；真机冒烟：`hamster-hub.exe mcp serve`
    tools/list ✓、CU 默认拒绝 ✓、真实 DB todo_list + 审计 JSONL ✓
  - 已知取舍：①chromiumoxide/enigo/xcap 链入主二进制（debug 41MB，release LTO+strip 待
    实测，装机包涨幅需盯轻量承诺）；②未认 HTTP 的旧版 agent 自然降级为无工具普通会话；
    ③codex 无会话级 MCP 注入通道，需走 hamster-core 同步引擎持久化注册（后续阶段）；
    ④确认档位（每步/仅高危）、通知中心联动、主屏 agent 小组件 → M4 阶段二
- ✅ **M4 阶段二：桌面 MCP 分发——按 agent 开关写入配置（2026-09-12，用户定调「HTTP 通用，
  与啥 agent 没关系」+「写入 agent 配置自然生效」）**：
  - **目的澄清（本阶段根本）**：内嵌 MCP 不是「给助手加工具」，而是把仓鼠Hub 做成这台
    机器上 agent 操作桌面的**统一入口与信任层**——桌面对人是什么，对 agent 就是什么。
    承载随之演进：独立 exe → 合并子命令 → **HTTP 内嵌常驻**（可寻址服务，非会话插件）
  - **hamster-core 同步引擎接线**（第 5 个 vendor 能力域启用）：设置页按 agent 开关 →
    hamster-desktop HTTP IR（`model::mcp::McpServer` + `Transport::Http`）upsert 进
    Store 源 → `SyncEngine::plan/apply` 写入该 agent 配置文件（备份 + 原子写 +
    history.jsonl）；关闭 = 源临时置 disabled 仅对该 target apply（渲染省略 = 摘除）+
    target 停用 + 源恢复；Drift 统一 Overwrite（render「读现有→合并」保留用户既有条目，
    apply 自动备份可回滚）
  - **接入三要素**：①固定默认端口 47613（settings.agent.mcp_port 可配；占用回退随机 +
    设置页黄字提示）；②用户级长效令牌（settings.agent.mcp_user_token，首启自动生成，
    设置页可再生成——旧令牌经 rotate_user_token 即刻失效）；③三命令
    agent_mcp_status / agent_mcp_set_enabled / agent_mcp_access_info
  - **前端**：`features/agent/McpSyncCard`（设置页卡片：URL/令牌 展示复制再生成 +
    per-agent 开关列表）；settings_save 钩子联动令牌轮换 + CU 档位
  - 门禁全绿：cargo fmt/clippy -D/test（+2 分发用例）/ pnpm build + vitest 31 / e2e 7/7
  - 遗留：per-agent 开关的真机端到端（在运行中的应用里开关并核对各 agent 配置文件）；
    codex/oc/pi 等未入 registry 的 agent 走「手动复制接入信息」兜底（McpAccessInfo
    已给全 url+token）；审计 UI 与「agent 正在操作」宣告 → M4 阶段三

