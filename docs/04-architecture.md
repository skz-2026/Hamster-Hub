# 仓鼠Hub 代码架构

> 版本 v1.0 ｜ 目录与分层是**约定**，M0 搭骨架时按此建立并由 lint/模板固化

## 1. 仓库结构（monorepo 单仓）

```
hamster-hub/                          #仓鼠Hub
├─ src/                               # 前端 (React + TS)
│  ├─ main.tsx                        # 入口：挂载 + 路由 + providers
│  ├─ app/                            # 应用层
│  │  ├─ routes/                      # 页面路由（5 模块 + 设置）
│  │  │  ├─ workbench/ search/ schedule/ apps/ files/ settings/
│  │  ├─ providers/                   # QueryProvider · ThemeProvider · HotkeyProvider
│  │  └─ router.ts                    # 路由表（主窗口内页切换，非多页 SPA 跳转）
│  ├─ features/                       # 业务模块（按领域，互不横向 import）
│  │  ├─ workbench/                   # 网格容器 + 布局 store + 编辑模式
│  │  ├─ widgets/                     # 小组件（见 §2.3）
│  │  ├─ search/                      # 搜索浮层 UI + useUnifiedSearch
│  │  ├─ apps/                        # 应用宫格/分组/全量列表
│  │  ├─ files/                       # 文件浏览/筛选
│  │  ├─ schedule/                    # 日程
│  │  ├─ todo/ note/ countdown/       # 轻业务（常并入 widgets 内 hooks）
│  │  ├─ weather/ hotlist/ ai/        # 领域 hooks + 面板
│  │  └─ settings/                    # 设置面板分组
│  ├─ shared/                         # 跨模块复用（只能被 features/app import）
│  │  ├─ components/                  # Button · Card · Modal · Icon · EmptyState · ScrollArea
│  │  ├─ hooks/                       # useCommand · useTauriEvent · useDebounce · useFrecency
│  │  ├─ lib/                         # ipc 客户端封装 · datetime · lunar 封装 · constants
│  │  ├─ stores/                      # zustand：layoutStore · uiStore
│  │  └─ types/                       # tauri-specta 生成的 IPC 类型（勿手改）
│  └─ styles/                         # tokens.css · tailwind 入口 · 字体
├─ src-tauri/                         # Rust 核心
│  ├─ src/
│  │  ├─ main.rs                      # 入口（仅 windows_subsystem 配置）
│  │  ├─ lib.rs                       # Builder 装配：插件/commands/事件/单实例
│  │  ├─ commands/                    # IPC 薄层：参数校验 → 调 core → 序列化
│  │  │  └─ mod.rs apps.rs files.rs search.rs todo.rs note.rs countdown.rs
│  │  │     schedule.rs weather.rs hotlist.rs ai.rs settings.rs system.rs
│  │  ├─ core/                        # 领域服务（禁止 import tauri，纯 Rust 可单测）
│  │  │  ├─ appindex/                 # mod.rs · lnk_scan.rs · uwp_scan.rs · icon_cache.rs
│  │  │  ├─ fileindex/                # mod.rs · walker.rs · watcher.rs · classify.rs · pinyin.rs
│  │  │  ├─ search/                   # mod.rs · provider.rs(trait) · rank.rs(frecency)
│  │  │  ├─ weather/                  # mod.rs · open_meteo.rs · types.rs
│  │  │  ├─ hotlist/                  # mod.rs · weibo.rs · zhihu.rs · bilibili.rs · cache.rs
│  │  │  ├─ ai/                       # mod.rs · openai_compat.rs · stream.rs
│  │  │  └─ scheduler/                # jobs.rs（interval 任务注册）
│  │  ├─ store/                       # db.rs(连接/迁移) · repo/(todo/note/...) · config.rs
│  │  ├─ platform/                    # Win32 封装（feature gate "windows"）
│  │  │  └─ lnk.rs icons.rs uwp.rs desktop.rs tray.rs
│  │  ├─ events.rs                    # 强类型事件名 + payload
│  │  └─ error.rs                     # AppError(code/message) + thiserror
│  ├─ migrations/                     # 0001_init.sql …（只增不改）
│  ├─ capabilities/                   # Tauri 权限白名单（最小化）
│  ├─ icons/  resources/              # 应用图标 · 内置资源（城市库/占位图）
│  └─ tauri.conf.json
├─ docs/                              # 本套文档
├─ scripts/                           # dev.ps1 · codegen.ps1 · bench.ps1
├─ .github/workflows/                 # ci.yml · release.yml
└─ package.json  pnpm-workspace.yaml  rust-toolchain.toml
```

## 2. 前端架构

### 2.1 分层与依赖规则

```
app（路由/providers）
  ↓
features（业务模块，横向禁止互引；公共需求下沉 shared）
  ↓
shared（组件/hooks/lib/types）          逆向 import 一律 lint error
```

### 2.2 状态管理双轨制

| 轨道 | 工具 | 存什么 | 例子 |
|---|---|---|---|
| 服务态（来自 Rust） | TanStack Query | 一切 invoke 结果：缓存、失效、重试 | `useQuery({queryKey:['apps','installed']})`，启动后 `invalidateQueries` |
| UI 态 | Zustand | 纯前端状态 | 工作台编辑模式、搜索框选中项、布局草稿 |

约定：组件内不出现裸 `invoke`，统一走 `shared/lib/ipc.ts` 封装（类型化 + 错误 toast 统一出口）。

### 2.3 小组件框架

- `features/widgets/registry.ts` 导出 `WidgetDefinition[]`（SDD §3.1），`workbench` 只消费注册表渲染，新增小组件零改动容器——为 M4 插件化预留。
- 每个小组件目录：`index.ts(definition) · View.tsx · hooks.ts · settings.tsx(可选)`，组件 `React.lazy` 按需加载。

## 3. Rust 架构

### 3.1 三层职责

```
commands（IPC 薄层：解参数/调服务/回 Result<AppError>，不写业务）
   ↓
core（领域服务：状态在 store，逻辑纯函数化，依赖 trait 注入）
   ↓
store（rusqlite 连接池 + repo） / platform（Win32 胶水）
```

- 装配在 `lib.rs`：构建 `AppState { db, config, providers… }`（`tauri::State` 注入 commands）。
- 并发：`tokio` 运行时由 Tauri 提供；DB 用 `r2d2_sqlite` 池；长任务（索引/AI 流）spawn 后通过 event 汇报，command 立即返回。

### 3.2 Provider trait（可测试与可替换的核心抽象）

```rust
// core/search/provider.rs
pub trait SearchProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn search(&self, query: &SearchQuery) -> Vec<SearchHit>;
}
// core/weather/mod.rs
#[async_trait]
pub trait WeatherProvider: Send + Sync {
    async fn now(&self, city: &City) -> Result<WeatherNow>;
}
// core/hotlist、core/ai 同理
```

单测用内存假实现；新增源（如和风天气、抖音热榜）= 新增一个 impl + 配置开关，零侵入。

## 4. IPC 层设计

- **类型单一事实源**：Rust 侧 `#[derive(specta::Type)]` + command 注册表，`pnpm codegen:ipc` 生成 `src/shared/types/ipc.ts`，前端 import 即类型安全。
- **错误规范**：所有 command 返回 `Result<T, AppError>`；`AppError.code` 稳定枚举（`IO`/`DB`/`NET`/`CANCEL`/`VALIDATE`/`NOT_FOUND`），前端按 code 决定 toast 还是静默。
- **事件规范**：命名空间小写 `域://动作`（`index://progress`）；payload 也是 specta 类型；前端 `useTauriEvent('index://progress', cb)` 自动清理。

### 4.1 端到端示例：待办创建

```rust
// commands/todo.rs —— 薄层
#[tauri::command]
#[specta::specta]
pub fn todo_create(state: State<'_, AppState>, content: String, due: Option<String>) -> Result<Todo> {
    let content = content.trim().check_len(500)?;      // VALIDATE
    state.repo.todo_create(content, due)
}
```

```ts
// shared/lib/ipc.ts —— 类型化封装（生成类型自动推导入参出参）
export const api = {
  todoCreate: (content: string, due?: string) =>
    invoke<Todo>('todo_create', { content, due }),
};

// features/widgets/todo/hooks.ts
export function useCreateTodo() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ content, due }: NewTodo) => api.todoCreate(content, due),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['todo', 'list'] }),
  });
}
```

### 4.2 小组件骨架约定

```tsx
// features/widgets/todo/index.ts
export const todoWidget: WidgetDefinition = {
  type: 'todo', title: '待办', minW: 3, minH: 4, defaultW: 4, defaultH: 5,
  component: lazy(() => import('./View')),
};
```

## 5. 数据流总览

```
持久层:  SQLite(业务+FTS+缓存)      settings.json(热配置)
            ↑↓ repo                    ↑↓ config.rs
服务层:  core::*(领域服务) ← trait 注入 ← AppState
            ↑↓ commands / events(IPC)
视图层:  TanStack Query(缓存/失效) ←→ Zustand(UI态) → React 组件
后台流:  scheduler → (天气30m/热榜5m/提醒1m/索引健康1h) → event 推送 → Query 失效 → UI 刷新
```

三条典型流：
1. **读**：组件 → useQuery → ipc 封装 → command → repo → SQLite；event 到达后 invalidate 触发重取。
2. **写**：组件 → useMutation → command → repo 事务 → onSuccess invalidate。
3. **后台**：scheduler/core 任务 → event → 前端对应 queryKey 失效（如 `['weather','now']`）。

## 6. 代码规范要点

- Rust：`fmt` 默认；clippy `pedantic` 子集按仓库 lint 表；模块 `mod.rs` 只做导出；pub API 写 doc comment（rustdoc 即接口文档）。
- TS：strict；组件 ≤ 250 行（超了拆 hooks/子组件）；禁 `any`（生成类型覆盖不到处用 `unknown` + 收窄）；路径别名 `@/`。
- 命名：Rust 蛇形、TS 驼峰、DB 蛇形、事件 `域://动作`；IPC command 与前端 api 方法同名蛇形/驼峰一一对应。
- 提交：Conventional Commits（`feat(appindex): …`），scope 用模块名。

## 7. 测试架构

```
单元:  src-tauri/core/**  #[cfg(test)] 就地测（拼音/排序/lnk解析/分类）
       src/features/**   Vitest + Testing Library（hooks/组件逻辑）
集成:  src-tauri/tests/  临时目录+内存库：索引增删改→搜索端到端、迁移
E2E:   e2e/              Playwright 驱动 WebView2：搜索三击键、待办冒烟、主题切换
基准:  benches/          criterion：FTS 查询 p95；脚本级：冷启动/内存快照
CI 门禁: fmt+clippy+eslint+单测 必绿；集成与 E2E 每 PR；基准每版本人工复核
```

## 8. 演进预留（防腐设计）

| 未来需求 | 现在埋的点 |
|---|---|
| 小组件插件市场 | 前端注册表 + 懒加载 + manifest 式 Definition |
| 新搜索源/天气源/热榜源/AI 服务商 | Provider trait + 配置开关 |
| macOS 版 | `platform/` feature gate 隔离 Win32；core 与 UI 天然跨平台 |
| 云同步待办 | repo 层接口化，本地实现可换远程实现 |
| 多语言 | 文案集中在 `shared/lib/i18n`（zh-CN 起步） |
