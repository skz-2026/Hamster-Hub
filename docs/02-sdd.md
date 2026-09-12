# 仓鼠Hub 软件设计文档（SDD）

> 版本 v1.0 ｜ 对应产品规划 [01-product-plan.md](./01-product-plan.md) ｜ 技术选型详见 [03-tech-roadmap.md](./03-tech-roadmap.md)

## 1. 文档范围与术语

- 范围：M0–M3 范围内的系统设计（M4 插件化仅做扩展点预留）。
- 术语：
  - **工作台（Workbench）**：主窗口内卡片式小组件网格页面。
  - **统一搜索（Unified Search）**：热键呼出的 overlay，聚合应用/文件/网页/AI 四路结果。
  - **索引（Index）**：对已安装应用与配置目录内文件的本地元数据库。
  - **IPC**：前端 WebView 与 Rust 核心之间的 Tauri command/event 通道。

## 2. 系统总体设计

### 2.1 分层架构

```
┌────────────────────────────────────────────────────────┐
│  前端 (React + TS, WebView2)                            │
│  路由壳: 工作台/搜索/日程/应用/文件/设置                    │
│  小组件框架 · 状态管理 · 主题系统                          │
├──────────── Tauri IPC (commands / events) ─────────────┤
│  Rust 核心                                              │
│  commands(薄层) → core 领域服务 → store(SQLite+配置)     │
│  领域服务: appindex · fileindex · search · weather       │
│           hotlist · ai · scheduler · platform(Win32)    │
├────────────────────────────────────────────────────────┤
│  OS: 文件系统 · 开始菜单/注册表/UWP · Shell · 通知 · 网络  │
└────────────────────────────────────────────────────────┘
```

设计原则：
1. **前端零系统访问**：所有文件/注册表/Shell 操作只发生在 Rust 侧，Tauri capabilities 最小化授权。
2. **core 与 tauri 解耦**：领域服务是纯 Rust 模块（依赖 trait 接口），不 import tauri，可独立单测。
3. **本地优先**：默认无网络也全功能可用（除天气/热榜/AI 三个明示联网服务）。

### 2.2 进程与窗口模型

| 窗口 | 用途 | 关键属性 | 时机 |
|---|---|---|---|
| `home` 主屏幕 | iOS 桌面：壁纸 + 图标网格/分页/文件夹/小组件 | 全屏、无边框、always-on-bottom（桌面层） | 「进入桌面模式」后常驻 |
| `dock` | 任务栏/Dock：桌面模式为贴边通栏任务栏（完全替代系统任务栏，含左端入口/定制/常用分组与右端退出+时钟），窗口化为居中胶囊；应用壳内组件 | 无边框、置顶、透明、不抢焦点 | 常驻 |
| `spotlight` 搜索浮层 | 应用/文件/网页/AI 四路搜索 | 无边框、透明全屏覆盖、失焦即隐 | `Alt+Space` 热键 |
| `control-center` | iOS 控制中心磁贴（M3） | 无边框、右上滑出/热键 | M3 |
| `settings` | iOS 风设置 App | 普通无边框窗口 + 亚克力 | 用户从主屏进入 |
| 托盘 | 常驻入口：进入/退出桌面模式、设置、退出 | 菜单 | 随进程 |
| 看门狗进程 | `hamster-watchdog.exe` 独立 bin：心跳监测主进程，异常时按快照还原系统 | 无窗口、开机自启可选 | 桌面模式期间 |

进程模型：主进程多 WebView（home/dock/spotlight 共享一进程）；看门狗为独立 exe（cargo workspace 成员），与主进程心跳（命名管道/共享内存事件），主进程正常退出时发「已还原」信号避免误恢复。单实例由插件保证，二次启动激活主屏。

### 2.3 启动流程（桌面模式）

```
exe 启动 → 单实例检查 → 初始化配置/DB → 创建托盘
→ 启动即桌面模式（behavior.desktop_mode_on_launch 默认 true，可设置关闭）：
  ① 写 system-state.json 快照（任务栏可见性/桌面图标注册表值，desktop_mode.rs）
  ② 隐藏任务栏（Shell_TrayWnd/副任务栏）+ 隐藏桌面图标（HideIcons 注册表+广播）
  ③ 主窗口铺满整屏（显示器全尺寸）→ 拉起 hamster-watchdog（传主进程 pid）→ 5s 巡检重隐藏
  ④ 前端路由到 /desktop 桌面主页（AppShell 初始对齐，hash ''/#'/' 守卫）
→ 后台异步：应用索引 / 文件索引 / 天气缓存，event 推送渐进渲染
退出模式（红色退出钮/热键/托盘）：按快照还原 → 删快照（看门狗 ≤2s 自退）→ 窗口化 1280×800
崩溃兜底：主进程死亡 → 看门狗凭快照还原任务栏/图标 → 删快照自退
```

## 3. 模块详细设计

### 3.1 主屏幕框架（Home）与图标网格

- **职责**：iOS 主屏幕范式——图标网格、多分页、文件夹、长按编辑模式、小组件插槽、布局持久化。
- **布局模型**：
  - 网格：桌面 7 列 × 5 行（按屏幕比例自适应），图标占 1 格，小组件占 2×2 / 4×2（iOS 尺寸规范）。
  - 分页：横向页数组 `pages: Page[]`，每页 `slots: Slot[]`（`{kind: app|folder|widget, ref, page, index}`）。
  - 文件夹：`{id, name, appKeys[]}`，点开缩放至全屏网格覆盖层（iOS 转场）。
  - Dock：独立 `dockSlots[6]`，跨页常驻。
  - 持久化：`home_layout` JSON（settings 表 key），含 pages/dock/壁纸/页序。
- **编辑模式**：长按 500ms 进入——图标抖动（CSS ±2° 交替）、拖拽重排（transform 合成层）、拖拽合并建夹、跨页拖拽（边缘悬停翻页）、移除；点空白/完成退出。
- **小组件注册表**（M3 启用）：前端 `widgets/index.ts` 维护 `WidgetDefinition[]`：

```ts
interface WidgetDefinition {
  type: 'clock' | 'weather' | 'todo' | 'calendar' | 'battery';
  title: string;
  size: 'small' | 'medium';             // 2x2 | 4x2（iOS 规范）
  component: React.ComponentType;       // 懒加载
  settingsSchema?: JSONSchema;
}
```

- **数据获取**：视图不直接 invoke，统一走 TanStack Query hooks（`useApps()`、`useWeather()`…），缓存/失效策略集中管理。

### 3.2 应用索引服务（appindex）

- **职责**：枚举本机可启动应用，产出稳定 `AppEntry`，供启动、搜索、常用应用宫格消费。
- **数据源（合并去重）**：
  1. 开始菜单 `.lnk`：`%ProgramData%` 与 `%AppData%` 下 `Microsoft\Windows\Start Menu`，用 `lnk` crate 解析目标路径与参数；
  2. UWP/商店应用：枚举 `shell:AppsFolder`（SHCreateItemFromParsingName + IEnumIDList），取 PFN 与显示名；
  3. 注册表卸载表（可选补充）：`HKLM/HKCU\...\Uninstall` 的 DisplayName/Icon/InstallLocation，仅用于补全元信息。
- **身份键 `app_key`**：`.lnk` 用规范化路径，UWP 用 `PFN!AppId`，保证增删可 diff。
- **图标**：`SHGetFileInfoW` 取大图标 HICON → RGBA → PNG，缓存于 `%AppData%/hamsterhub/icons/{hash}.png`；UWP 从包 logo 资产拷贝转换。图标提取失败用默认占位。
- **刷新**：启动时全量（≤500 个，秒级）+ 对开始菜单目录 `notify` 监听增量。
- **启动执行**：`.lnk` 直接 `explorer.exe /start`（或 ShellExecute）；UWP 用 `explorer.exe shell:AppsFolder\{PFN}!{AppId}`；以管理员权限应用启动需提权确认（P2 再做，默认跳过）。

### 3.3 文件索引服务（fileindex）

- **职责**：对用户配置目录建可搜索元数据索引（MVP：桌面/文档/下载/图片/视频）。
- **扫描**：`walkdir` + 忽略规则（`node_modules`、`.git`、`AppData`、隐藏/系统目录），单卷限流（防机械盘打满 IO），默认上限 10 万条（可调）。
- **增量**：`notify` 推荐 watcher（ReadDirectoryChangesW）+ 2s 防抖批量 upsert/delete。
- **分类**：按扩展名映射 `kind`：image/video/audio/archive/doc/code/font/other。
- **拼音**：索引时生成 `pinyin_full`（全拼）与 `pinyin_initials`（首字母），存 FTS 表，保证「wx → 微信截图」类检索。

### 3.4 统一搜索服务（search）

- **职责**：一次查询聚合多 Provider 结果并统一排序。
- **Provider 抽象**：

```rust
trait SearchProvider {
    fn id(&self) -> &'static str;                      // apps / files / web / actions
    fn search(&self, query: &str, limit: usize) -> Vec<SearchHit>;
}
```

- **匹配策略**：FTS5 前缀/子串命中 + `nucleo-matcher` 模糊得分融合；得分因子：匹配质量、启动频次（frecency：次数×时间衰减）、最近使用（文件 mtime）、是否固定。
- **频次统计**：`usage_log` 表记录应用/文件打开事件，frecency 权重让「常用者靠前」。
- **网页路**：纯前端行为——query 非空且无本地强命中时展示「用浏览器搜索 "{q}"」，不请求网络。
- **AI 路（M2）**：结果区固定尾部入口「问 AI」，回车/点击进入流式回答面板。
- **时序预算**：本地 apps+files 合并 ≤ 50ms（5 万条库）；输入防抖 80ms。

### 3.5 天气服务（weather）

- **Provider**：Open-Meteo（免 key，默认）+ 预留 `WeatherProvider` trait 接和风天气（用户自配 key）。
- **城市**：MVP 手动选择（内置中国城市库，支持拼音过滤）；不做 IP 定位默认。
- **缓存**：结果写 `weather_cache`，TTL 30 分钟；启动先读缓存渲染再后台刷新。
- **展示**：当前（温度/现象/高低温/风力）+ 24h 折线 + 7d 列表。

### 3.6 日历与农历

- 农历/节气/节日由前端 `lunar-javascript` 计算（纯本地、零网络）。
- 日程事件（M2）：存 `schedule_event`，提醒由 Rust `scheduler` 每分钟轮询到期项，经系统通知推送（`tauri-plugin-notification`）。

### 3.7 待办 / 便签 / 倒数日

纯本地 CRUD，Rust 侧仅做存储与校验，逻辑在前端。倒数日支持周年循环（生日/纪念日按年重算）。

### 3.8 热榜服务（hotlist）

- **源**：微博 `weibo.com/ajax/side/hotSearch`、知乎热榜、B 站排行，Rust `reqwest` 抓取（无 CORS 问题），每源独立失败降级。
- **缓存**：内存 + DB 双层，TTL 5 分钟；全部失败显示上次缓存并标注时间。
- **合规**：仅展示标题与跳转原文链接，不存储正文。

### 3.9 AI 网关（ai）

- OpenAI 兼容协议（baseURL + apiKey + model 用户自配，默认引导配置智谱 GLM）。
- 流式：Rust `reqwest` SSE 解析 → `ai://chunk/{id}` event 推送前端逐 token 渲染；会话上下文仅存内存，不做云端历史。
- 上下文入口：搜索框「问 AI」、独立聊天面板（M2）。

### 3.10 调度器（scheduler）

单一后台任务循环（tokio interval），注册任务：天气刷新（30min）、热榜刷新（5min，仅热榜页可见时）、日程提醒检查（1min）、索引健康检查（启动 + 每小时）。

## 4. 数据设计

存储：SQLite（`%AppData%/hamsterhub/hamsterhub.db`，WAL 模式，FTS5）+ 配置 JSON（`settings.json`，热改项）。迁移：`src-tauri/migrations/` 顺序 SQL，内嵌 `user_version` 管理。

```sql
-- ============ 核心业务表 ============
CREATE TABLE settings (
  key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at INTEGER NOT NULL
);

CREATE TABLE todo (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  content TEXT NOT NULL,
  done INTEGER NOT NULL DEFAULT 0,
  due_date TEXT,                          -- YYYY-MM-DD，可空
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  completed_at INTEGER
);

CREATE TABLE note (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  content TEXT NOT NULL,
  color TEXT, pinned INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);

CREATE TABLE countdown (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT NOT NULL,
  target_date TEXT NOT NULL,              -- YYYY-MM-DD
  repeat_yearly INTEGER NOT NULL DEFAULT 0,
  emoji TEXT, sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE schedule_event (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT NOT NULL,
  start_at TEXT NOT NULL,                 -- RFC3339 本地时间
  end_at TEXT, all_day INTEGER NOT NULL DEFAULT 0,
  location TEXT, color TEXT,
  remind_minutes INTEGER,                 -- NULL=不提醒
  recurrence TEXT                         -- NULL|daily|weekly|monthly|yearly
);

-- ============ 应用 ============
CREATE TABLE app_group (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE, sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE app_pin (                    -- 常用应用/分组固定
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  group_id INTEGER NOT NULL REFERENCES app_group(id) ON DELETE CASCADE,
  app_key TEXT NOT NULL,                  -- lnk 规范化路径 或 PFN!AppId
  display_name TEXT NOT NULL,
  icon_path TEXT, exec_target TEXT NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  UNIQUE(group_id, app_key)
);

-- ============ 索引 ============
CREATE TABLE app_meta (                   -- 已安装应用全量（缓存，可重建）
  app_key TEXT PRIMARY KEY,
  display_name TEXT NOT NULL, exec_target TEXT NOT NULL,
  kind TEXT NOT NULL,                     -- lnk | uwp
  icon_path TEXT, indexed_at INTEGER NOT NULL
);
CREATE VIRTUAL TABLE app_fts USING fts5(
  display_name, pinyin_full, pinyin_initials,
  content='app_meta', content_rowid='rowid'
);

CREATE TABLE file_meta (
  id INTEGER PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL, ext TEXT, dir TEXT NOT NULL,
  size INTEGER NOT NULL, mtime INTEGER NOT NULL,
  kind TEXT NOT NULL
);
CREATE VIRTUAL TABLE file_fts USING fts5(
  name, pinyin_full, pinyin_initials, path,
  content='file_meta', content_rowid='id'
);

CREATE TABLE usage_log (                  -- frecency 启动统计
  target_key TEXT NOT NULL,               -- app_key 或文件路径
  target_kind TEXT NOT NULL,              -- app | file
  at INTEGER NOT NULL
);

CREATE TABLE weather_cache (
  city_id TEXT PRIMARY KEY, payload TEXT NOT NULL, fetched_at INTEGER NOT NULL
);

CREATE TABLE hotlist_cache (
  source TEXT PRIMARY KEY, payload TEXT NOT NULL, fetched_at INTEGER NOT NULL
);
```

配置文件 `settings.json`（结构示例）：

```jsonc
{
  "appearance": { "theme": "dark", "accent": "#FF8A3D", "glass": "acrylic", "fontScale": 1.0 },
  "search":    { "hotkey": "Alt+Space", "providers": { "apps": true, "files": true } },
  "fileIndex": { "roots": ["~/Desktop", "~/Documents", "~/Downloads", "~/Pictures", "~/Videos"],
                 "maxFiles": 100000 },
  "weather":   { "provider": "open-meteo", "cityId": "101010100", "qweatherKey": null },
  "ai":        { "baseUrl": "https://open.bigmodel.cn/api/paas/v4", "model": "", "apiKey": "" },
  "behavior":  { "autostart": true, "startMinimized": false, "language": "zh-CN" }
}
```

## 5. 接口设计（IPC 契约）

约定：command 一律蛇形命名，返回 `Result<T, AppError>`；`AppError = { code, message, detail? }`；类型经 `tauri-specta` 生成 TS 绑定，前后端不手写重复类型。

### 5.1 Commands（MVP 子集）

| Command | 入参 | 出参 | 说明 |
|---|---|---|---|
| `app_list_installed` | – | `Vec<AppEntry>` | 全量已安装应用 |
| `app_launch` | `app_key` | `()` | 启动并记 usage |
| `app_pin_list` / `app_pin_add` / `app_pin_remove` / `app_pin_move` | … | … | 常用应用宫格 CRUD |
| `file_search` | `query, filters?, limit` | `Vec<FileHit>` | 文件检索 |
| `file_index_status` | – | `IndexStatus` | 进度/总数/耗时 |
| `recent_files` | `limit` | `Vec<FileHit>` | 最近文件 |
| `search_unified` | `query, limit_per` | `SearchResponse` | 分组结果 |
| `todo_list/create/update/delete/toggle` | … | … | 待办 CRUD |
| `weather_get` | – | `WeatherNow` | 缓存或即时 |
| `weather_set_city` | `cityId` | `()` | 切换城市 |
| `settings_get` / `settings_set` | `key` / `key,value` | `JsonValue` | 跨重启偏好 |
| `open_path` / `reveal_in_explorer` | `path` | `()` | 打开/定位文件 |
| `window_set_pinned` | `pinned` | `()` | 主窗口置顶 |

### 5.2 Events

| Event | 载荷 | 触发 |
|---|---|---|
| `index://progress` | `{stage, done, total}` | 索引扫描中 |
| `index://done` | `{apps, files, costMs}` | 索引完成 |
| `weather://updated` | `WeatherNow` | 后台刷新成功 |
| `hotlist://updated` | `Vec<HotItem>` | 热榜刷新 |
| `ai://chunk/{id}` | `{delta}` / `{done, error?}` | AI 流式 |
| `remind://due` | `ScheduleEvent` | 日程到期 |

### 5.3 示例

```jsonc
// invoke("search_unified", { query: "wx", limitPer: 5 })
{
  "groups": [
    { "provider": "apps",  "hits": [ { "title": "微信", "subtitle": "WeChat", "iconPath": "...", "score": 96.2, "action": { "type": "launch", "appKey": "C:\\...\\WeChat.lnk" } } ] },
    { "provider": "files", "hits": [ { "title": "微信截图_20260910.png", "subtitle": "...\\Pictures", "score": 71.0, "action": { "type": "openPath", "path": "..." } } ] }
  ],
  "elapsedMs": 18
}
```

## 6. 关键流程时序

### 6.1 全局搜索

```
用户 Alt+Space → 热键插件显示 search 窗口 → 输入 "wx"(防抖80ms)
→ invoke search_unified → Rust: 并发 app_fts / file_fts 查询 + nucleo 重排
→ 返回分组结果(≤50ms) → 前端分组渲染 → 回车: 取首命中执行 action(launch/openPath)
→ Rust 执行 ShellExecute + 写 usage_log
```

### 6.2 文件索引增量

```
notify 事件(创建/修改/删除) → 防抖2s合并 → upsert/delete file_meta(+FTS)
→ 每秒限速写入 → 失败重试队列(启动时回放)
```

## 7. UI/UX 规范

### 7.1 信息架构（iOS 范式）

- **主屏幕**：壁纸 + 图标网格（squircle 图标 + 名称白字投影）+ 多分页（scroll-snap + 页码圆点）+ 网格内小组件（M3）。
- **桌面主页（桌面模式默认页 /desktop，对齐水豚hub）**：全屏壁纸 + 中央问候语/超大时钟/农历 + 大搜索框 + 小组件卡片行 + 快捷入口药丸；无顶部菜单栏、无侧栏、无窗口控件（完全沉浸）。
- **任务栏/Dock**：桌面模式（全屏接管）= **贴边通栏任务栏，完全替代系统任务栏**——x=0 贴边、100% 屏宽、贴底零间隙、无圆角：左端主屏/搜索/设置入口 → 竖分隔线 → 定制组 → 分隔线 → 常用组 →（弹性空白）→ 红色退出图标 + 时钟/日期（系统托盘位）；窗口化模式 = 居中胶囊（常驻）。定制 = 主屏编辑模式拖入（`home_layout.dock`）；常用 = 启动频次 TopN（与定制组去重）。悬停放大上浮 + tooltip + 活动路由指示点。
- **Spotlight**：`Alt+Space` 呼出全屏毛玻璃覆盖层，居中搜索框 + 分组结果（应用/文件/网页/问 AI），键盘优先导航。
- **控制中心（M3）**：右上热区滑出，磁贴网格：Wi-Fi/蓝牙/勿扰/亮度滑条/音量滑条/锁屏/设置。
- **设置 App**：iOS 风列表（分组圆角卡片、右箭头、开关），非桌面模式下以普通窗口呈现。
- **托盘菜单**：进入/退出桌面模式、Spotlight、设置、退出。

### 7.2 设计 Token（iOS 风格化）

| Token | 值 |
|---|---|
| 主色 accent | `#FF8A3D`（胡萝卜橙，对应 iOS 橙色系强调） |
| 图标 | squircle：`border-radius: 22.5%`，60×60（网格）/ 52×52（Dock）；名称 12px 白字 + 文字投影 |
| 毛玻璃 | `backdrop-filter: blur(30px) saturate(1.8)`；Dock/Spotlight/文件夹底 `rgba(255,255,255,.12)`（暗模式 `rgba(30,30,32,.55)`） |
| 圆角 | Dock 容器 28px；文件夹打开层 40px；设置卡片 12px（iOS 分组列表规范） |
| 字体 | MiSans / HarmonyOS Sans（-apple-system fallback）；时钟用 SF Pro 风格数字（tabular-nums + 半粗） |
| 动效 | iOS 曲线 `cubic-bezier(0.32, 0.72, 0, 1)`；文件夹打开 320ms 缩放转场；图标抖动 8° 相位差；分页 240ms snap |

### 7.3 热键表（默认，均可改）

| 热键 | 功能 |
|---|---|
| `Alt+Space` | 呼出/隐藏全局搜索 |
| `Esc` | 关闭搜索浮层 |
| `↑ ↓` / `Enter` / `Tab` | 搜索结果导航 / 执行 / 切换分组 |
| `Ctrl+N`（搜索内） | 以网页搜索当前词 |

## 8. 非功能设计

| 维度 | 设计 |
|---|---|
| 性能 | 预算见产品规划 §5；索引批量事务写入；搜索 SQL 走 FTS 索引，无全表扫 |
| 内存 | 图标 PNG 惰性加载 + LRU（上限 300 张）；FTS 索引约 40B/条量级，10 万条 ≈ 4MB |
| 安全 | capabilities 仅开放 `core:window`/自建 commands；无 `fs`/`shell:open` 泛权限，路径打开走白名单 command；AI key 存 DPAPI 加密（Win `windows` crate CryptProtectData） |
| 隐私 | 默认零遥测；天气/热榜/AI 三处联网在设置页明示清单 |
| 兼容 | Win11 用 Mica，Win10 降级 Acrylic，1809 以下降级纯色（启动时探测）；100%-200% DPI 与多屏后续验证 |
| 可测试 | core 服务纯函数化 + trait 注入假实现；DB 测试用临时库 |
| 可扩展 | 小组件前端注册表天然可插件化（M4）；Rust Provider trait 可加新搜索源 |

## 9. 错误处理与降级

| 场景 | 策略 |
|---|---|
| 天气网络失败 | 读 stale 缓存并标注「x 分钟前」；无缓存显示重试态 |
| 热榜单源失败 | 隐藏该源，其余正常 |
| AI 未配置/失败 | 入口显示「去设置」引导，不阻塞搜索 |
| 索引目录被移除 | watcher 捕获后清空该 root 条目并提示 |
| 图标提取失败 | 占位图标 + 后台重试一次 |
| DB 损坏 | 启动校验，失败则重命名备份并重建业务表（索引可重建，用户数据优先导出） |

## 10. 测试策略

| 层 | 工具 | 覆盖点 |
|---|---|---|
| Rust 单测 | `cargo test` | 拼音生成、frecency 排序、lnk 解析、分类映射、FTS 查询构造 |
| Rust 集成 | 临时目录+临时库 | 索引增删改、搜索端到端、迁移幂等 |
| 前端单测 | Vitest + Testing Library | 小组件渲染、hooks、布局持久化序列化 |
| E2E（M2 起） | Playwright(WebView2) | 搜索三击键命中、待办 CRUD 冒烟、主题切换 |
| 手工矩阵 | – | Win10 1809/21H2 × Win11 23H2/24H2 × DPI 100/150/200 |
| 性能基准 | 基准脚本入库 | 冷启动、搜索 p95、内存快照，每版本跑一次对比 |
