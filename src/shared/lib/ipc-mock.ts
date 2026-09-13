/**
 * IPC mock 层：浏览器环境（无 tauri 后端）下的假实现，与生成契约同签名。
 * 仅由 lib/ipc.ts 在非 tauri 环境装配；应用代码不直接 import 本文件。
 */
import { classifyApp } from '@/features/apps/category';
import type {
  AppEntry,
  AppHealth,
  PluginInfo,
  CountdownCustom,
  CountdownItem,
  FileHit,
  GenOptions,
  Note,
  ProcInfo,
  Settings,
  SystemSnapshot,
  Todo,
  TodoReminder,
  FocusFinished,
  FocusStatus,
  FocusTick,
  UpdateInfo,
  UpdateProgress,
  VaultItem,
  VaultItemInput,
  VaultSecret,
  VaultStatus,
  WeatherNow,
} from '@/shared/types/ipc';
import type {
  AgentInfo,
  AgentMcpStatus,
  AssistantCreateArgs,
  AssistantSessionInfo,
  IndexStatus,
  LiveSessionInfo,
  McpAccessInfo,
  SearchHit,
  SearchQuery,
  SessionMessagesPage,
  SessionSummary,
  SnapshotMessage,
  StreamCreateArgs,
  StreamEvent,
  WorkspaceRecord,
} from '@/shared/types/bench';

// ===== 假数据 =====

// 演示数据语言跟随浏览器预览的 settings.behavior.language（真机不走本文件）。
// 截图脚本在 addInitScript 里种 en 种子后整页加载，模块初始化时即定型。
const MOCK_EN = (() => {
  try {
    const all = JSON.parse(localStorage.getItem('hamsterhub.mock') || '{}');
    return JSON.parse(all.settings || '{}')?.behavior?.language === 'en';
  } catch {
    return false;
  }
})();
const L = (zh: string, en: string): string => (MOCK_EN ? en : zh);

function svgIcon(letter: string, hue: number): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="120" height="120"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${hue},72%,58%)"/><stop offset="1" stop-color="hsl(${(hue + 40) % 360},68%,42%)"/></linearGradient></defs><rect width="120" height="120" rx="0" fill="url(#g)"/><text x="60" y="79" font-size="50" text-anchor="middle" fill="white" font-family="sans-serif" font-weight="600">${letter}</text></svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

const RAW_APPS: [string, string | null][] = [
  [L('微信', 'WeChat'), svgIcon(L('微', 'W'), 120)], ['QQ', svgIcon('Q', 210)], [L('腾讯会议', 'Tencent Meeting'), svgIcon(L('会', 'T'), 200)],
  [L('钉钉', 'DingTalk'), svgIcon(L('钉', 'D'), 220)], ['Steam', svgIcon('S', 165)], [L('网易云音乐', 'NetEase Music'), svgIcon(L('云', 'N'), 340)],
  [L('QQ音乐', 'QQ Music'), svgIcon(L('音', 'Q'), 350)], ['Chrome', svgIcon('C', 50)], ['Edge', svgIcon('E', 210)],
  ['VS Code', svgIcon('V', 230)], ['ZCode', svgIcon('Z', 260)], ['Git', svgIcon('G', 20)],
  ['GIMP', svgIcon('G', 280)], ['Snipaste', svgIcon('S', 90)], ['Everything', null],
  ['potPlayer', null], ['OBS', svgIcon('O', 360)], ['Unity', null],
  ['Blender', svgIcon('B', 30)], ['WPS', svgIcon('W', 300)], ['Word', svgIcon('W', 220)],
  ['Excel', svgIcon('X', 140)], ['PowerPoint', svgIcon('P', 25)], [L('百度网盘', 'Baidu Netdisk'), svgIcon(L('盘', 'B'), 330)],
  [L('迅雷', 'Thunder'), svgIcon(L('雷', 'T'), 60)], [L('计算器', 'Calculator'), null], [L('记事本', 'Notepad'), null], [L('画图', 'Paint'), null],
];

const APPS: AppEntry[] = RAW_APPS.map(([name, icon], i) => ({
  app_key: `mock:${name.toLowerCase()}`,
  display_name: name,
  exec_target: `C:\\mock\\${i}.lnk`,
  kind: 'lnk',
  icon_path: icon,
}));

const MOCK_FILES: FileHit[] = [
  { path: L('C:\\Users\\you\\Desktop\\微信截图_0901.png', 'C:\\Users\\you\\Desktop\\screenshot_0901.png'), name: L('微信截图_0901.png', 'screenshot_0901.png'), ext: 'png', kind: L('图片', 'image'), size: 240000, mtime: 1757000000 },
  { path: L('C:\\Users\\you\\Documents\\年度总结.docx', 'C:\\Users\\you\\Documents\\Annual_Review.docx'), name: L('年度总结.docx', 'Annual_Review.docx'), ext: 'docx', kind: L('文档', 'doc'), size: 88000, mtime: 1756900000 },
  { path: L('C:\\Users\\you\\Documents\\旅游计划.xlsx', 'C:\\Users\\you\\Documents\\Travel_Plan.xlsx'), name: L('旅游计划.xlsx', 'Travel_Plan.xlsx'), ext: 'xlsx', kind: L('文档', 'doc'), size: 45000, mtime: 1756800000 },
  { path: 'C:\\Users\\you\\Downloads\\hamster-hub-setup.exe', name: 'hamster-hub-setup.exe', ext: 'exe', kind: '', size: 2300000, mtime: 1757100000 },
  { path: L('C:\\Users\\you\\Pictures\\旅行vlog.mp4', 'C:\\Users\\you\\Pictures\\travel_vlog.mp4'), name: L('旅行vlog.mp4', 'travel_vlog.mp4'), ext: 'mp4', kind: L('视频', 'video'), size: 89000000, mtime: 1756500000 },
  { path: L('C:\\Users\\you\\Documents\\报销单2026.xlsx', 'C:\\Users\\you\\Documents\\Expense_2026.xlsx'), name: L('报销单2026.xlsx', 'Expense_2026.xlsx'), ext: 'xlsx', kind: L('文档', 'doc'), size: 30000, mtime: 1757200000 },
  { path: 'C:\\Users\\you\\Projects\\hamster-hub\\lib.rs', name: 'lib.rs', ext: 'rs', kind: L('代码', 'code'), size: 42000, mtime: 1756700000 },
  { path: L('C:\\Users\\you\\Downloads\\素材包.zip', 'C:\\Users\\you\\Downloads\\assets_pack.zip'), name: L('素材包.zip', 'assets_pack.zip'), ext: 'zip', kind: L('压缩包', 'archive'), size: 51200000, mtime: 1756400000 },
  { path: L('C:\\Users\\you\\Music\\夜曲.mp3', 'C:\\Users\\you\\Music\\nocturne.mp3'), name: L('夜曲.mp3', 'nocturne.mp3'), ext: 'mp3', kind: L('音频', 'audio'), size: 8200000, mtime: 1756000000 },
];

let MOCK_PROCS: ProcInfo[] = [
  { pid: 4820, name: 'chrome.exe', mem_mb: 1180, cpu: 4.2 },
  { pid: 3911, name: 'WeChat.exe', mem_mb: 824, cpu: 1.1 },
  { pid: 7788, name: 'Code.exe', mem_mb: 741, cpu: 2.8 },
  { pid: 2044, name: 'steam.exe', mem_mb: 312, cpu: 0.7 },
  { pid: 880, name: 'explorer.exe', mem_mb: 128, cpu: 0.4 },
  { pid: 9917, name: 'ZCode.exe', mem_mb: 96, cpu: 5.6 },
];

const DEFAULT_SETTINGS: Settings = {
  appearance: { theme: 'dark', accent: '#ff8a3d', glass: 'acrylic', font_scale: 1 },
  search: { hotkey: 'Alt+Space', search_apps: true, search_files: true },
  file_index: {
    roots: ['~/Desktop', '~/Documents', '~/Downloads'],
    max_files: 100000,
  },
  weather: { provider: 'open-meteo', city_id: '', qweather_key: null },
  ai: { base_url: 'https://open.bigmodel.cn/api/paas/v4', model: '', api_key: null },
  behavior: { autostart: false, start_minimized: false, language: 'zh-CN', desktop_mode_hotkey: 'Ctrl+Alt+D', desktop_mode_on_launch: true },
  agent: { computer_use_enabled: false, assistant_persona: '', mcp_port: null, mcp_user_token: null, mcp_agents: [], assistant_agent_id: '', assistant_model: '', assistant_effort: '', cli_paths: {} },
};

/** mock 的按 agent 分发开关状态（内存） */
let mockMcpAgents: string[] = [];

// ===== UI 插件 mock（浏览器预览：内置示例的假清单 + 假代码，localStorage 存储）=====
const MOCK_PLUGINS: PluginInfo[] = [
  {
    id: 'hello-hamster',
    name: '你好仓鼠',
    version: '1.0.0',
    description: '最小示例：点击计数 + 插件私有存储',
    author: '仓鼠Hub',
    entry: 'widget.js',
    permissions: ['todo.add'],
    entryPath: 'mock://hello-hamster/widget.js',
  },
  {
    id: 'hitokoto',
    name: '一言',
    version: '1.0.0',
    description: '随机刷一句一言（fetch 网络）',
    author: '仓鼠Hub',
    entry: 'widget.js',
    permissions: [],
    entryPath: 'mock://hitokoto/widget.js',
  },
];
const MOCK_PLUGIN_CODE: Record<string, string> = {
  'hello-hamster': `export default function (el, ctx) {
  let count = 0;
  el.innerHTML = '<div style="display:flex;flex-direction:column;justify-content:center;height:100%;gap:6px"><div style="font-size:11px;font-weight:500;color:rgba(255,255,255,.9)">你好仓鼠（浏览器预览）</div><div style="display:flex;align-items:center;gap:10px"><button data-act style="border:0;border-radius:999px;padding:4px 12px;font-size:12px;cursor:pointer;background:rgba(255,138,61,.85);color:#fff">囤一口</button><span data-n style="font-size:22px;font-weight:300;color:#fff">0</span></div><button data-todo style="align-self:flex-start;border:1px solid rgba(255,255,255,.18);border-radius:999px;padding:2px 10px;font-size:10px;cursor:pointer;background:transparent;color:rgba(255,255,255,.75)">记一条「喂仓鼠」到待办</button></div>';
  const n = el.querySelector('[data-n]');
  const paint = () => (n.textContent = String(count));
  ctx.storage.get('count').then((v) => { count = Number(v ?? 0); paint(); });
  const onClick = () => { count += 1; paint(); ctx.storage.set('count', String(count)); };
  el.querySelector('[data-act]').addEventListener('click', onClick);
  const t = el.querySelector('[data-todo]');
  const onTodo = async () => {
    t.textContent = '记录中…';
    try { await ctx.api.todoAdd({ content: '喂仓鼠（来自插件）' }); t.textContent = '✓ 已记入待办'; }
    catch { t.textContent = '失败，重试？'; }
  };
  t.addEventListener('click', onTodo);
  return () => { el.querySelector('[data-act]')?.removeEventListener('click', onClick); t?.removeEventListener('click', onTodo); };
}`,
  hitokoto: `export default function (el) {
  el.innerHTML = '<div style="display:flex;flex-direction:column;justify-content:center;height:100%;gap:6px"><div data-l style="font-size:12px;color:rgba(255,255,255,.9)">一言（浏览器预览）</div></div>';
  const l = el.querySelector('[data-l]');
  const load = () => fetch('https://v1.hitokoto.cn/?max_length=24').then((r) => r.json()).then((d) => (l.textContent = d.hitokoto)).catch(() => (l.textContent = '网络不可用'));
  load();
  const t = setInterval(load, 30000);
  return () => clearInterval(t);
}`,
};

// ===== mock 事件总线（模拟 tauri-specta 生成的事件对象） =====

type Cb<T> = (e: { event: string; payload: T }) => void;

function mockEvent<T>(name: string) {
  const cbs = new Set<Cb<T>>();
  return {
    listen(cb: Cb<T>) {
      cbs.add(cb);
      return Promise.resolve(() => cbs.delete(cb));
    },
    once(cb: Cb<T>) {
      const wrap: Cb<T> = (e) => {
        cbs.delete(wrap);
        cb(e);
      };
      cbs.add(wrap);
      return Promise.resolve(() => cbs.delete(wrap));
    },
    _emit(payload: T) {
      cbs.forEach((cb) => cb({ event: name, payload }));
    },
  };
}

export const mockEvents = {
  coreReady: mockEvent<{ version: string }>('coreReady'),
  desktopModeChanged: mockEvent<{ active: boolean }>('desktopModeChanged'),
  appIndexUpdated: mockEvent<{ count: number }>('appIndexUpdated'),
  benchStreamEvent: mockEvent<StreamEvent>('benchStreamEvent'),
  benchStreamExit: mockEvent<{ sessionId: string }>('benchStreamExit'),
  benchPtyExit: mockEvent<{ sessionId: string; exitCode: number }>('benchPtyExit'),
  todoReminder: mockEvent<TodoReminder>('todoReminder'),
  focusTick: mockEvent<FocusTick>('focusTick'),
  focusFinished: mockEvent<FocusFinished>('focusFinished'),
  vaultLocked: mockEvent<{ reason: string }>('vaultLocked'),
  updateProgress: mockEvent<UpdateProgress>('updateProgress'),
};

// ===== mock 状态 =====

let desktopActive = false;
let MOCK_VOLUME = { level: 0.6, muted: false };

// 番茄钟 mock 计时（与 Rust ticker 语义一致：暂停不走秒，归零发 finished）
let mockFocus: FocusStatus | null = null;
let mockFocusTimer: ReturnType<typeof setInterval> | null = null;
function focusStopMock() {
  if (mockFocusTimer) {
    clearInterval(mockFocusTimer);
    mockFocusTimer = null;
  }
}
function focusRunMock() {
  focusStopMock();
  mockFocusTimer = setInterval(() => {
    if (!mockFocus) return focusStopMock();
    if (mockFocus.paused) return;
    mockFocus.remaining_secs -= 1;
    mockEvents.focusTick._emit({
      kind: mockFocus.kind,
      remaining_secs: mockFocus.remaining_secs,
      paused: false,
    });
    if (mockFocus.remaining_secs <= 0) {
      const kind = mockFocus.kind;
      mockFocus = null;
      focusStopMock();
      mockEvents.focusFinished._emit({ kind });
    }
  }, 1000);
}
const LS_KEY = 'hamsterhub.mock';

function ls(): Record<string, string> {
  try {
    return JSON.parse(localStorage.getItem(LS_KEY) ?? '{}');
  } catch {
    return {};
  }
}
function lsSet(key: string, value: string) {
  const all = ls();
  all[key] = value;
  localStorage.setItem(LS_KEY, JSON.stringify(all));
}

// ===== 密码箱 mock 状态（浏览器调试用：明文存 localStorage；真机为字段级加密 + 内存密钥）=====

interface MockVaultEntry extends VaultItem {
  secret: VaultSecret;
  deleted: boolean;
}
interface MockVaultState {
  master: string | null;
  hint: string | null;
  autoLockSecs: number;
  unlocked: boolean;
  seq: number;
  entries: MockVaultEntry[];
}

function vaultLoad(): MockVaultState {
  const raw = ls()['vault'];
  if (raw) return JSON.parse(raw) as MockVaultState;
  return { master: null, hint: null, autoLockSecs: 300, unlocked: false, seq: 0, entries: [] };
}
function vaultSave(s: MockVaultState) {
  lsSet('vault', JSON.stringify(s));
}
function vaultErr(code: string, message: string): never {
  throw { code, message };
}
function vaultStatusOf(s: MockVaultState): VaultStatus {
  return {
    initialized: s.master != null,
    unlocked: s.unlocked,
    hint: s.hint,
    auto_lock_secs: s.autoLockSecs,
  };
}
/** 与 Rust core::vault::password_strength 同款启发式（仅 mock 展示对齐用） */
function vaultStrengthMock(password: string): number {
  if (!password) return 0;
  const hasUpper = /[A-Z]/.test(password);
  const hasLower = /[a-z]/.test(password);
  const hasDigit = /\d/.test(password);
  const hasSymbol = /[^A-Za-z0-9]/.test(password);
  let pool = 0;
  if (hasUpper) pool += 26;
  if (hasLower) pool += 26;
  if (hasDigit) pool += 10;
  if (hasSymbol) pool += 32;
  const entropy = password.length * Math.log2(pool || 1);
  let score = entropy < 28 ? 0 : entropy < 40 ? 1 : entropy < 60 ? 2 : entropy < 90 ? 3 : 4;
  const unique = new Set(Array.from(password)).size;
  if (1 - unique / password.length > 0.6) score = Math.min(score, 2);
  return score;
}

// ===== mock commands（与生成契约同签名） =====

export const mockCommands = {
  async aiChat(messages: { role: string; content: string }[]): Promise<string> {
    console.log('[mock] AI 会话', messages.length, '条');
    await new Promise((r) => setTimeout(r, 800));
    const last = messages[messages.length - 1]?.content ?? '';
    return L(
      `（浏览器 mock 回复）收到：「${last.slice(0, 50)}」。真机配置 API Key 后，这里会是模型的真实回答——这也是后续 Agent 的接入点。`,
      `(Browser mock reply) Got it: "${last.slice(0, 50)}". With a real API key configured, the model's actual answer appears here — also the hookup point for agents.`,
    );
  },
  async volumeGet(): Promise<{ level: number; muted: boolean }> {
    return MOCK_VOLUME;
  },
  async volumeSet(level: number, muted: boolean): Promise<null> {
    MOCK_VOLUME = { level, muted };
    return null;
  },
  async settingsLoad(): Promise<Settings> {
    const raw = ls()['settings'];
    return raw ? { ...DEFAULT_SETTINGS, ...JSON.parse(raw) } : DEFAULT_SETTINGS;
  },
  async settingsSave(settings: Settings): Promise<Settings> {
    lsSet('settings', JSON.stringify(settings));
    return settings;
  },
  async kvGet(key: string): Promise<string | null> {
    return ls()[key] ?? null;
  },
  async kvSet(key: string, value: string): Promise<null> {
    lsSet(key, value);
    return null;
  },
  async appHealth(): Promise<AppHealth> {
    return { name: 'hamster-hub', version: '0.1.0-web' };
  },
  async updateCheck(): Promise<UpdateInfo | null> {
    // 假更新：让浏览器层能走完「发现新版本 → 下载进度 → 安装」全流程 UI
    return { version: '9.9.9', notes: null };
  },
  async updateInstall(): Promise<null> {
    const total = 100_000;
    for (let p = 0; p <= 100; p += 10) {
      await new Promise((r) => setTimeout(r, 150));
      mockEvents.updateProgress._emit({ downloaded: (p / 100) * total, total, done: false });
    }
    mockEvents.updateProgress._emit({ downloaded: total, total, done: true });
    return null;
  },
  async windowSetPinned(_pinned: boolean): Promise<null> {
    console.log('[mock] windowSetPinned', _pinned);
    return null;
  },
  async openUrl(url: string): Promise<null> {
    console.log('[mock] 打开 URL', url);
    return null;
  },
  async wallpaperImageImport(path: string, _previous: string | null): Promise<string> {
    // 浏览器无 asset protocol，返回原路径仅供预览（真机为 appdata 副本路径）
    console.log('[mock] wallpaperImageImport', path);
    return path;
  },
  async fileSearch(query: string, limit: number | null): Promise<FileHit[]> {
    const q = query.trim().toLowerCase();
    if (!q) return [];
    return MOCK_FILES.filter((f) => f.name.toLowerCase().includes(q)).slice(0, limit ?? 6);
  },
  async todoList(): Promise<Todo[]> {
    const raw = ls()['todos'];
    if (raw) return JSON.parse(raw);
    const seed: Todo[] = [
      { id: 1, content: L('体验仓鼠Hub 桌面模式', 'Try HamsterHub desktop mode'), done: false, created_at: 0, due_at: null, remind_at: null, reminded_at: null, recur: null },
      { id: 2, content: L('把常用应用拖进 Dock', 'Drag favorite apps into the Dock'), done: true, created_at: 0, due_at: null, remind_at: null, reminded_at: null, recur: null },
    ];
    lsSet('todos', JSON.stringify(seed));
    return seed;
  },
  async todoCreate(content: string, dueAt: number | null, remind: boolean | null): Promise<Todo> {
    const list = await mockCommands.todoList();
    const t: Todo = {
      id: Date.now(),
      content,
      done: false,
      created_at: Math.floor(Date.now() / 1000),
      due_at: dueAt,
      remind_at: dueAt != null && remind ? dueAt : null,
      reminded_at: null,
      recur: null,
    };
    lsSet('todos', JSON.stringify([t, ...list]));
    // 浏览器预览：8 秒后模拟一次到点提醒（真机由 Rust 调度线程发系统通知）
    if (t.remind_at != null) {
      setTimeout(
        () => mockEvents.todoReminder._emit({ id: t.id, content: t.content, due_at: t.due_at }),
        8000,
      );
    }
    return t;
  },
  async todoSetDue(id: number, dueAt: number | null, remind: boolean): Promise<Todo> {
    const list = await mockCommands.todoList();
    const t = list.find((x) => x.id === id);
    if (!t) throw new Error(`待办不存在: ${id}`);
    const next: Todo = {
      ...t,
      due_at: dueAt,
      remind_at: dueAt != null && remind ? dueAt : null,
      reminded_at: null,
    };
    lsSet('todos', JSON.stringify(list.map((x) => (x.id === id ? next : x))));
    return next;
  },
  async todoSetRecur(id: number, recur: string | null): Promise<Todo> {
    const list = await mockCommands.todoList();
    const t = list.find((x) => x.id === id);
    if (!t) throw new Error(`待办不存在: ${id}`);
    const next: Todo = { ...t, recur };
    lsSet('todos', JSON.stringify(list.map((x) => (x.id === id ? next : x))));
    return next;
  },
  async todoToggle(id: number, done: boolean): Promise<null> {
    const list = await mockCommands.todoList();
    const now = Math.floor(Date.now() / 1000);
    lsSet(
      'todos',
      JSON.stringify(
        list.map((t) => {
          if (t.id !== id) return t;
          // 循环待办完成 = 滚到下一次（与 Rust set_done 语义一致）
          if (done && t.recur) {
            const base = Math.max(t.due_at ?? now, now);
            const day = 86400;
            const next =
              t.recur === 'daily' ? base + day
              : t.recur === 'weekly' ? base + 7 * day
              : t.recur === 'monthly' ? base + 30 * day
              : base + day; // weekdays 简化，浏览器预览用
            return { ...t, done: false, due_at: next, remind_at: t.remind_at != null ? next : null, reminded_at: null };
          }
          return { ...t, done };
        }),
      ),
    );
    return null;
  },
  async todoDelete(id: number): Promise<null> {
    const list = await mockCommands.todoList();
    lsSet('todos', JSON.stringify(list.filter((t) => t.id !== id)));
    return null;
  },
  // ===== 番茄钟 mock（setInterval 秒级 tick，语义与 Rust ticker 一致）=====
  async focusStart(minutes: number, todoId: number | null): Promise<FocusStatus> {
    focusStopMock();
    mockFocus = {
      kind: 'focus',
      total_secs: minutes * 60,
      remaining_secs: minutes * 60,
      paused: false,
      todo_id: todoId,
    };
    focusRunMock();
    return mockFocus;
  },
  async focusBreak(minutes: number): Promise<FocusStatus> {
    focusStopMock();
    mockFocus = {
      kind: 'break',
      total_secs: minutes * 60,
      remaining_secs: minutes * 60,
      paused: false,
      todo_id: null,
    };
    focusRunMock();
    return mockFocus;
  },
  async focusPause(): Promise<null> {
    if (mockFocus) mockFocus.paused = true;
    return null;
  },
  async focusResume(): Promise<null> {
    if (mockFocus) mockFocus.paused = false;
    return null;
  },
  async focusStop(): Promise<null> {
    focusStopMock();
    mockFocus = null;
    return null;
  },
  async focusStatus(): Promise<FocusStatus | null> {
    return mockFocus;
  },
  async focusHistory(_days: number | null): Promise<{ day: string; minutes: number }[]> {
    const d = new Date();
    const day = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
    return [{ day, minutes: 25 }];
  },
  async weatherGet(): Promise<WeatherNow> {
    return {
      city: L('北京', 'Beijing'),
      temp: 26,
      kind: L('多云', 'Cloudy'),
      emoji: '⛅',
      temp_min: 20,
      temp_max: 28,
      fetched_at: Math.floor(Date.now() / 1000),
    };
  },
  async weatherSetCity(city: string): Promise<null> {
    console.log('[mock] 设置城市', city);
    return null;
  },
  async countdownList(): Promise<CountdownItem[]> {
    return [
      { title: L('距离周末', 'To weekend'), days: 1, kind: 'auto', emoji: '🛋️' },
      { title: L('新年', 'New Year'), days: 112, kind: 'auto', emoji: '🎊' },
    ];
  },
  async recentFiles(limit: number | null): Promise<FileHit[]> {
    return [...MOCK_FILES].sort((a, b) => b.mtime - a.mtime).slice(0, limit ?? 6);
  },
  async topApps(_limit: number | null): Promise<AppEntry[]> {
    return APPS.slice(0, 8);
  },
  async noteList(): Promise<Note[]> {
    const raw = ls()['notes'];
    if (raw) return JSON.parse(raw);
    const seed: Note[] = [
      { id: 1, content: L('明早先回张总消息 📮', 'Reply to Mr. Zhang first thing tomorrow 📮'), pinned: true, updated_at: 0 },
      { id: 2, content: L('仓鼠Hub 发布前记得跑一遍还原测试矩阵', 'Run the restore-test matrix before HamsterHub ships'), pinned: false, updated_at: 0 },
    ];
    lsSet('notes', JSON.stringify(seed));
    return seed;
  },
  async noteCreate(content: string): Promise<Note> {
    const list = await mockCommands.noteList();
    const n: Note = {
      id: Date.now(),
      content,
      pinned: false,
      updated_at: Math.floor(Date.now() / 1000),
    };
    lsSet('notes', JSON.stringify([n, ...list]));
    return n;
  },
  async noteUpdate(id: number, content: string): Promise<null> {
    const list = await mockCommands.noteList();
    lsSet(
      'notes',
      JSON.stringify(
        list.map((n) =>
          n.id === id ? { ...n, content, updated_at: Math.floor(Date.now() / 1000) } : n,
        ),
      ),
    );
    return null;
  },
  async noteTogglePin(id: number, pinned: boolean): Promise<null> {
    const list = await mockCommands.noteList();
    lsSet('notes', JSON.stringify(list.map((n) => (n.id === id ? { ...n, pinned } : n))));
    return null;
  },
  async noteDelete(id: number): Promise<null> {
    const list = await mockCommands.noteList();
    lsSet('notes', JSON.stringify(list.filter((n) => n.id !== id)));
    return null;
  },
  async countdownCustomList(): Promise<CountdownCustom[]> {
    const raw = ls()['cd-custom'];
    if (raw) return JSON.parse(raw);
    const seed: CountdownCustom[] = [
      { id: 1, title: '项目上线', target_date: '2026-10-01', emoji: '🚀' },
    ];
    lsSet('cd-custom', JSON.stringify(seed));
    return seed;
  },
  async countdownCustomCreate(title: string, targetDate: string, emoji: string | null): Promise<CountdownCustom> {
    const list = await mockCommands.countdownCustomList();
    const item: CountdownCustom = {
      id: Date.now(),
      title,
      target_date: targetDate,
      emoji: emoji ?? '🎯',
    };
    lsSet('cd-custom', JSON.stringify([...list, item]));
    return item;
  },
  async countdownCustomDelete(id: number): Promise<null> {
    const list = await mockCommands.countdownCustomList();
    lsSet('cd-custom', JSON.stringify(list.filter((c) => c.id !== id)));
    return null;
  },
  async fileIndexRefresh(): Promise<null> {
    console.log('[mock] 重扫文件索引');
    return null;
  },
  async systemStats(): Promise<SystemSnapshot> {
    return {
      mem: { total_gb: 16, used_gb: 10, percent: 63 },
      disks: [
        { mount: 'C:\\', total_gb: 476, used_gb: 371, percent: 78 },
        { mount: 'D:\\', total_gb: 931, used_gb: 412, percent: 44 },
      ],
      cpu: { percent: 14, core_count: 16, name: 'Mock CPU' },
      temps: [{ label: 'CPU Package', celsius: 61 }],
    };
  },
  async processList(_sort: string | null, limit: number | null): Promise<ProcInfo[]> {
    return MOCK_PROCS.slice(0, limit ?? 6);
  },
  async processKill(pid: number): Promise<null> {
    MOCK_PROCS = MOCK_PROCS.filter((p) => p.pid !== pid);
    return null;
  },
  async openPath(path: string): Promise<null> {
    console.log('[mock] 打开文件', path);
    return null;
  },
  async revealInExplorer(path: string): Promise<null> {
    console.log('[mock] 定位文件', path);
    return null;
  },
  async appList(): Promise<AppEntry[]> {
    return APPS;
  },
  async appLaunch(appKey: string): Promise<null> {
    console.log('[mock] 启动应用', appKey);
    return null;
  },
  async appSearch(query: string, _limit: number | null): Promise<AppEntry[]> {
    const q = query.trim().toLowerCase();
    if (!q) return [];
    // 常见应用拼音/缩写映射（模拟 Rust pinyin_cols 效果）
    const PY: Record<string, string> = {
      微信: 'weixin wx', qq: 'qq tencent', 腾讯会议: 'tengxun huiyi hy', 钉钉: 'dingding dd',
      网易云音乐: 'wangyiyun wy', qq音乐: 'yinyue yy', steam: 'steam', chrome: 'chrome',
      edge: 'edge', 'vs code': 'vscode vsc', zcode: 'zcode', git: 'git', gimp: 'gimp',
      snipaste: 'snipaste', everything: 'everything', potplayer: 'potplayer', obs: 'obs',
      unity: 'unity', blender: 'blender', wps: 'wps', word: 'word', excel: 'excel',
      powerpoint: 'ppt', 百度网盘: 'wangpan wp', 迅雷: 'xunlei xl', 计算器: 'jisuanqi jsq',
      记事本: 'jishiben jsb', 画图: 'huatu ht',
    };
    return APPS.filter((a) => {
      const py = PY[a.display_name.toLowerCase()] ?? '';
      return a.display_name.toLowerCase().includes(q) || py.includes(q);
    }).slice(0, 8);
  },
  async desktopModeEnter(): Promise<null> {
    if (!desktopActive) {
      desktopActive = true;
      mockEvents.desktopModeChanged._emit({ active: true });
    }
    return null;
  },
  async desktopModeExit(): Promise<null> {
    if (desktopActive) {
      desktopActive = false;
      mockEvents.desktopModeChanged._emit({ active: false });
    }
    return null;
  },
  async desktopModeIsActive(): Promise<boolean> {
    return desktopActive;
  },
  async startMenuOpen(): Promise<null> {
    // 浏览器预览无系统壳：注入 Ctrl+Esc 唤出真开始菜单仅 tauri 环境有效
    return null;
  },

  // ===== 代理工作台（bench）：GUI 流式对话 + Recall（与生成绑定同名）=====

  async benchScanAgents(): Promise<AgentInfo[]> {
    // 对齐真机：手动指定的路径（settings.agent.cli_paths）优先，文件存在即视为已装
    const paths = (await mockCommands.settingsLoad()).agent.cli_paths ?? {};
    return MOCK_AGENTS.map((a) => {
      const p = paths[a.id];
      if (!p) return a;
      return { ...a, installed: true, program: p };
    });
  },
  async benchListProjects(): Promise<string[]> {
    return MOCK_PROJECTS;
  },
  async benchAgentWorkspaces(): Promise<WorkspaceRecord[]> {
    return [
      ...MOCK_PROJECTS.map((dir) => ({ dir, source: 'recent' })),
      { dir: 'D:\\work\\ml-pipeline', source: 'claude projects' },
      { dir: 'C:\\Users\\you\\Desktop\\blog', source: 'codex sessions' },
    ];
  },
  async benchListStreamSessions(): Promise<LiveSessionInfo[]> {
    return [...liveStreams.values()].map((s) => s.info);
  },
  async benchListHistorySessions(): Promise<SessionSummary[]> {
    return MOCK_HISTORY;
  },
  async benchStreamCreate(args: StreamCreateArgs): Promise<LiveSessionInfo> {
    const { agentId, projectDir, firstPrompt, resumeKey } = args;
    const agent = MOCK_AGENTS.find((a) => a.id === agentId);
    if (!agent?.installed) throw new Error('BENCH_AGENT_NOT_INSTALLED');
    if (!agent.streaming) throw new Error('BENCH_UNSUPPORTED');
    if (!projectDir.trim()) throw new Error('BENCH_NO_PROJECT_DIR');
    // spawn 幂等（对齐真机）：同一 rollout 已有活会话直接复用
    if (resumeKey) {
      const existing = [...liveStreams.values()].find(
        (s) => s.info.running && s.info.resumeKey === resumeKey,
      );
      if (existing) return existing.info;
    }
    const info: LiveSessionInfo = {
      sessionId: `mock-${Date.now().toString(36)}`,
      agentId,
      kind: 'agent',
      channel: 'stream',
      projectDir,
      running: true,
      exitCode: null,
      startedAt: Date.now(),
      lastActiveAt: Date.now(),
      resumeKey: resumeKey ?? null,
    };
    liveStreams.set(info.sessionId, { info, busy: false, timers: [] });
    if (firstPrompt?.trim()) {
      scheduleFakeTurn(info.sessionId, firstPrompt.trim());
    }
    return info;
  },
  // ===== 桌面助手（浏览器假会话复用同一假流式代理；真机由 Rust 注入 hamster-desktop MCP server）=====
  async benchAssistantCreate(args: AssistantCreateArgs): Promise<AssistantSessionInfo> {
    const agentId = args.agentId ?? 'claude';
    const session = await mockCommands.benchStreamCreate({
      agentId,
      projectDir: 'C:\\Users\\hamster\\AppData\\Roaming\\com.hamsterhub.app\\assistant',
      firstPrompt: args.firstPrompt ?? null,
      model: args.model ?? null,
      effort: null,
      resumeKey: null,
      fork: false,
    });
    return { session, agentId, mcpInjected: true, computerUseEnabled: false };
  },
  // ===== Agent CLI 手动指定路径（与真机同语义：校验非空后写进 settings 持久层）=====
  async agentCliPathSet(agentId: string, path: string | null): Promise<Record<string, string>> {
    const trimmed = path?.trim() || null;
    if (trimmed && !trimmed.includes('\\') && !trimmed.includes('/')) {
      throw new Error(`路径不存在或不是文件: ${trimmed}`);
    }
    const current = await mockCommands.settingsLoad();
    // 生成类型是 Partial<Record<string,string>>（值含 undefined），先收敛成确定值
    const cliPaths: Record<string, string> = {};
    for (const [k, v] of Object.entries(current.agent.cli_paths ?? {})) {
      if (typeof v === 'string') cliPaths[k] = v;
    }
    if (trimmed) cliPaths[agentId] = trimmed;
    else delete cliPaths[agentId];
    await mockCommands.settingsSave({ ...current, agent: { ...current.agent, cli_paths: cliPaths } });
    return cliPaths;
  },
  // ===== 桌面 MCP 分发（浏览器假数据：开关状态仅存内存）=====
  async agentMcpStatus(): Promise<AgentMcpStatus[]> {
    return MOCK_AGENTS.filter((a) => a.installed).map((a) => ({
      agentId: a.id,
      agentName: a.name,
      installed: true,
      mcpCapable: true,
      enabled: mockMcpAgents.includes(a.id),
      configPath: `C:\\Users\\hamster\\.${a.id}\\config`,
    }));
  },
  async agentMcpSetEnabled(agentId: string, enabled: boolean): Promise<null> {
    if (enabled && !mockMcpAgents.includes(agentId)) mockMcpAgents.push(agentId);
    if (!enabled) mockMcpAgents = mockMcpAgents.filter((a) => a !== agentId);
    console.log('[mock] agentMcpSetEnabled', agentId, enabled);
    return null;
  },
  async agentMcpAccessInfo(): Promise<McpAccessInfo> {
    return {
      url: 'http://127.0.0.1:47613/mcp',
      token: 'mock-user-token-0000',
      port: 47613,
      portFellBack: false,
      defaultPort: 47613,
    };
  },
  // ===== UI 插件（浏览器：mock 清单 + 代码 + localStorage 私有存储）=====
  async pluginList(): Promise<PluginInfo[]> {
    return MOCK_PLUGINS;
  },
  async pluginReadCode(pluginId: string, _entry: string): Promise<string> {
    const code = MOCK_PLUGIN_CODE[pluginId];
    if (!code) throw new Error('PLUGIN_NOT_FOUND');
    return code;
  },
  async pluginStorageGet(pluginId: string, key: string): Promise<string | null> {
    return localStorage.getItem(`plugin.${pluginId}.${key}`);
  },
  async pluginStorageSet(pluginId: string, key: string, value: string): Promise<null> {
    localStorage.setItem(`plugin.${pluginId}.${key}`, value);
    return null;
  },
  async pluginDelete(pluginId: string): Promise<null> {
    const i = MOCK_PLUGINS.findIndex((p) => p.id === pluginId);
    if (i >= 0) MOCK_PLUGINS.splice(i, 1);
    delete MOCK_PLUGIN_CODE[pluginId];
    console.log('[mock] pluginDelete', pluginId);
    return null;
  },
  async pluginBridgeCall(pluginId: string, capability: string, payload: string): Promise<string> {
    console.log('[mock] pluginBridgeCall', pluginId, capability);
    const args = JSON.parse(payload || '{}') as Record<string, unknown>;
    if (capability === 'todo.add') {
      const t = await mockCommands.todoCreate(String(args.content ?? '（插件）'), null, null);
      return JSON.stringify(t);
    }
    if (capability === 'todo.list') return JSON.stringify(await mockCommands.todoList());
    if (capability === 'apps.search')
      return JSON.stringify(
        APPS.filter((a) => a.display_name.includes(String(args.query ?? '')))
          .slice(0, 8)
          .map((a) => ({ appKey: a.app_key, name: a.display_name })),
      );
    if (capability === 'apps.launch') return JSON.stringify({ launched: args.app_key ?? '' });
    throw new Error('未知能力');
  },
  async benchStreamSend(sessionId: string, text: string): Promise<null> {
    const s = liveStreams.get(sessionId);
    if (!s || !text.trim()) return null;
    scheduleFakeTurn(sessionId, text.trim());
    return null;
  },
  async benchStreamInterrupt(sessionId: string): Promise<null> {
    const s = liveStreams.get(sessionId);
    if (s) {
      s.timers.forEach(clearTimeout);
      s.timers = [];
      s.busy = false;
      emitEv(sessionId, 'turnCompleted', '');
    }
    return null;
  },
  async benchStreamKill(sessionId: string): Promise<null> {
    const s = liveStreams.get(sessionId);
    if (s) {
      s.timers.forEach(clearTimeout);
      s.info.running = false;
      s.info.exitCode = 0;
      emitEv(sessionId, 'exit', '');
      mockEvents.benchStreamExit._emit({ sessionId });
      liveStreams.delete(sessionId);
    }
    return null;
  },
  // ===== PTY 终端（假 TUI：banner + 回显）=====
  async benchListLiveSessions(): Promise<LiveSessionInfo[]> {
    return [...ptySessions.values()].map((s) => s.info);
  },
  async benchPtyCreate(
    args: {
      agentId: string;
      projectDir: string;
      firstPrompt: string | null;
      resumeKey: string | null;
      cols: number;
      rows: number;
    },
    onData: { onmessage: (data: number[]) => void },
  ): Promise<LiveSessionInfo> {
    const agent = MOCK_AGENTS.find((a) => a.id === args.agentId);
    if (!agent?.installed || !agent.chat) throw new Error('BENCH_UNSUPPORTED');
    if (!args.projectDir.trim()) throw new Error('BENCH_NO_PROJECT_DIR');
    // spawn 幂等（对齐真机单写者语义）：重复 attach 把输出汇切换到新回调
    if (args.resumeKey) {
      const existing = [...ptySessions.values()].find(
        (s) => s.info.running && s.info.resumeKey === args.resumeKey,
      );
      if (existing) {
        existing.onData.current = onData;
        existing.push('\r\n\x1b[2m(输出已重新附着到当前视图)\x1b[0m\r\n> ');
        return existing.info;
      }
    }
    const info: LiveSessionInfo = {
      sessionId: `mock-pty-${Date.now().toString(36)}`,
      agentId: args.agentId,
      kind: 'agent',
      channel: 'pty',
      projectDir: args.projectDir,
      running: true,
      exitCode: null,
      startedAt: Date.now(),
      lastActiveAt: Date.now(),
      resumeKey: args.resumeKey ?? null,
    };
    const timers: ReturnType<typeof setTimeout>[] = [];
    // 数据统一走 create 时注册的 Channel 回调（与真机 ipc Channel 同语义）
    const onDataRef = { current: onData };
    const push = (text: string) => onDataRef.current.onmessage(Array.from(new TextEncoder().encode(text)));
    ptySessions.set(info.sessionId, { info, timers, onData: onDataRef, push });
    // 假 TUI：banner + resume 提示
    const banner =
      `\x1b[2m${new Date().toLocaleString()}\x1b[0m\r\n` +
      `\x1b[1;33m${agent.name}\x1b[0m \x1b[2m(mock TUI · ${args.resumeKey ? `resume ${args.resumeKey}` : '新会话'})\x1b[0m\r\n` +
      (args.resumeKey ? '已恢复上一段会话历史（浏览器 mock 模拟）\r\n' : '') +
      '> \x1b[2m（在终端里直接输入）\x1b[0m\r\n';
    timers.push(setTimeout(() => push(banner), 150));
    return info;
  },
  async benchPtyWrite(sessionId: string, data: string): Promise<null> {
    const s = ptySessions.get(sessionId);
    if (!s) return null;
    s.info.lastActiveAt = Date.now();
    s.push(
      data.includes('\r')
        ? `\r\n\x1b[2m[mock] 已收到输入：${data.replace(/[\r\n]+/g, '')}\x1b[0m\r\n> `
        : data,
    );
    return null;
  },
  async benchPtySendPrompt(sessionId: string, text: string): Promise<null> {
    return mockCommands.benchPtyWrite(sessionId, `${text}\r`);
  },
  async benchPtyResize(): Promise<null> {
    return null;
  },
  async benchPtyKill(sessionId: string): Promise<null> {
    const s = ptySessions.get(sessionId);
    if (s) {
      s.timers.forEach(clearTimeout);
      s.info.running = false;
      s.info.exitCode = 0;
      mockEvents.benchPtyExit._emit({ sessionId, exitCode: 0 });
      ptySessions.delete(sessionId);
    }
    return null;
  },
  async benchSearchSessions(query: SearchQuery): Promise<SearchHit[]> {
    const q = query.text.trim().toLowerCase();
    if (!q) return [];
    return MOCK_HITS.filter(
      (h) =>
        (h.snippet.toLowerCase().includes(q) || h.sessionTitle.toLowerCase().includes(q)) &&
        (query.agents.length === 0 || query.agents.includes(h.agent)),
    ).slice(0, query.limit ?? 20);
  },
  async benchSessionMessages(
    agent: string,
    sessionKey: string,
    aroundSeq: number,
    window: number,
  ): Promise<SnapshotMessage[]> {
    const all = MOCK_MESSAGES[`${agent}/${sessionKey}`] ?? [];
    return all
      .filter((m) => Math.abs(m.seq - aroundSeq) <= window)
      .sort((a, b) => a.seq - b.seq);
  },
  async benchListSessionMessages(
    agent: string,
    sessionKey: string,
    fromSeq: number,
    limit: number,
  ): Promise<SessionMessagesPage> {
    const all = MOCK_MESSAGES[`${agent}/${sessionKey}`] ?? [];
    const list = all.sort((a, b) => a.seq - b.seq).filter((m) => m.seq > fromSeq);
    return { total: all.length, messages: list.slice(-limit) };
  },
  async benchIndexStatus(): Promise<IndexStatus> {
    const msgs = Object.values(MOCK_MESSAGES).reduce((n, list) => n + list.length, 0);
    return { sessions: Object.keys(MOCK_MESSAGES).length, messages: msgs, bytes: 96 * 1024 };
  },
  async benchIndexRefresh(): Promise<IndexStatus> {
    return mockCommands.benchIndexStatus();
  },
  async benchReindex(): Promise<IndexStatus> {
    return mockCommands.benchIndexStatus();
  },
  async benchSessionDelete(_agent: string, _sessionKey: string): Promise<null> {
    console.log('[mock] 删除历史会话', _agent, _sessionKey);
    return null;
  },
  async benchLatestIndexedSession(agent: string, projectDir: string): Promise<SessionSummary | null> {
    return (
      MOCK_HISTORY.find((h) => h.agent === agent && h.projectPath === projectDir) ?? null
    );
  },
  // ===== 托盘（浏览器预览无系统弹层，仅日志）=====
  async trayOpenOverflow() {
    console.log('[mock] 打开 Windows 原生托盘溢出弹层');
    return null;
  },

  // ===== 密码箱（明文 mock，仅浏览器调试；真机为字段级加密）=====
  async vaultStatus(): Promise<VaultStatus> {
    return vaultStatusOf(vaultLoad());
  },
  async vaultSetup(password: string, hint: string | null, autoLockSecs: number | null): Promise<VaultStatus> {
    const s = vaultLoad();
    if (s.master != null) vaultErr('VALIDATE', '密码箱已初始化');
    s.master = password;
    s.hint = hint?.trim() || null;
    s.autoLockSecs = Math.min(3600, Math.max(30, autoLockSecs ?? 300));
    s.unlocked = true;
    vaultSave(s);
    return vaultStatusOf(s);
  },
  async vaultUnlock(password: string): Promise<VaultStatus> {
    await new Promise((r) => setTimeout(r, 250)); // 模拟 Argon2id 派生耗时
    const s = vaultLoad();
    if (s.master !== password) vaultErr('VAULT_AUTH', '主密码不正确');
    s.unlocked = true;
    vaultSave(s);
    return vaultStatusOf(s);
  },
  async vaultLock(): Promise<VaultStatus> {
    const s = vaultLoad();
    s.unlocked = false;
    vaultSave(s);
    mockEvents.vaultLocked._emit({ reason: 'manual' });
    return vaultStatusOf(s);
  },
  async vaultSetAutoLock(secs: number): Promise<VaultStatus> {
    const s = vaultLoad();
    s.autoLockSecs = Math.min(3600, Math.max(30, secs));
    vaultSave(s);
    return vaultStatusOf(s);
  },
  async vaultChangePassword(oldPassword: string, newPassword: string, hint: string | null): Promise<VaultStatus> {
    const s = vaultLoad();
    if (s.master !== oldPassword) vaultErr('VAULT_AUTH', '主密码不正确');
    s.master = newPassword;
    s.hint = hint?.trim() || null;
    vaultSave(s);
    return vaultStatusOf(s);
  },
  async vaultItemList(): Promise<VaultItem[]> {
    return vaultLoad()
      .entries.filter((e) => !e.deleted)
      .sort((a, b) => Number(b.favorite) - Number(a.favorite) || b.updated_at - a.updated_at)
      .map(({ secret, deleted, ...meta }) => {
        void secret;
        void deleted;
        return meta;
      });
  },
  async vaultItemSearch(query: string): Promise<VaultItem[]> {
    const q = query.trim().toLowerCase();
    const all = await mockCommands.vaultItemList();
    if (!q) return all;
    return all.filter(
      (i) =>
        i.title.toLowerCase().includes(q) ||
        i.username.toLowerCase().includes(q) ||
        i.url.toLowerCase().includes(q),
    );
  },
  async vaultItemCreate(input: VaultItemInput): Promise<VaultItem> {
    const s = vaultLoad();
    if (!input.title.trim()) vaultErr('VALIDATE', '标题不能为空');
    s.seq += 1;
    const now = Math.floor(Date.now() / 1000);
    const entry: MockVaultEntry = {
      id: s.seq,
      title: input.title.trim(),
      username: input.username.trim(),
      url: input.url.trim(),
      favorite: input.favorite,
      created_at: now,
      updated_at: now,
      password_updated_at: now,
      deleted: false,
      secret: { password: input.password, notes: input.notes.trim() },
    };
    s.entries.push(entry);
    vaultSave(s);
    const { secret, deleted, ...meta } = entry;
    void secret;
    void deleted;
    return meta;
  },
  async vaultItemUpdate(id: number, input: VaultItemInput): Promise<VaultItem> {
    const s = vaultLoad();
    const entry = s.entries.find((e) => e.id === id && !e.deleted);
    if (!entry) vaultErr('VALIDATE', `密码条目不存在: ${id}`);
    const now = Math.floor(Date.now() / 1000);
    if (entry.secret.password !== input.password) entry.password_updated_at = now;
    entry.title = input.title.trim();
    entry.username = input.username.trim();
    entry.url = input.url.trim();
    entry.favorite = input.favorite;
    entry.updated_at = now;
    entry.secret = { password: input.password, notes: input.notes.trim() };
    vaultSave(s);
    const { secret, deleted, ...meta } = entry;
    void secret;
    void deleted;
    return meta;
  },
  async vaultItemToggleFavorite(id: number, favorite: boolean): Promise<VaultItem> {
    const s = vaultLoad();
    const entry = s.entries.find((e) => e.id === id && !e.deleted);
    if (!entry) vaultErr('VALIDATE', `密码条目不存在: ${id}`);
    entry.favorite = favorite;
    vaultSave(s);
    const { secret, deleted, ...meta } = entry;
    void secret;
    void deleted;
    return meta;
  },
  async vaultItemDelete(id: number): Promise<null> {
    const s = vaultLoad();
    const entry = s.entries.find((e) => e.id === id && !e.deleted);
    if (!entry) vaultErr('VALIDATE', `密码条目不存在: ${id}`);
    entry.deleted = true; // 软删除，与真机一致（回收站 M2）
    vaultSave(s);
    return null;
  },
  async vaultItemReveal(id: number): Promise<VaultSecret> {
    const entry = vaultLoad().entries.find((e) => e.id === id && !e.deleted);
    if (!entry) vaultErr('VALIDATE', `密码条目不存在: ${id}`);
    return entry.secret;
  },
  async vaultCopyField(id: number, field: string): Promise<number> {
    const s = vaultLoad();
    const entry = s.entries.find((e) => e.id === id && !e.deleted);
    if (!entry) vaultErr('VALIDATE', `密码条目不存在: ${id}`);
    const text = field === 'username' ? entry.username : entry.secret.password;
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      console.log('[mock] 复制到剪贴板', field);
    }
    return field === 'password' ? 15 : 0;
  },
  async vaultPasswordGenerate(options: GenOptions): Promise<string> {
    const UPPER = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
    const LOWER = 'abcdefghijklmnopqrstuvwxyz';
    const DIGITS = '0123456789';
    const SYMBOLS = '!@#$%^&*()-_=+[]{}<>?,.';
    const AMBIGUOUS = new Set(['I', 'l', '1', 'O', '0', 'o']);
    const classes: string[] = [];
    if (options.upper) classes.push(UPPER);
    if (options.lower) classes.push(LOWER);
    if (options.digits) classes.push(DIGITS);
    if (options.symbols) classes.push(SYMBOLS);
    if (classes.length === 0) vaultErr('VALIDATE', '至少启用一个字符类');
    const pools = classes
      .map((c) => Array.from(c).filter((ch) => !options.exclude_ambiguous || !AMBIGUOUS.has(ch)))
      .filter((c) => c.length > 0);
    const randIdx = (n: number) => {
      const buf = new Uint32Array(1);
      const zone = Math.floor(0x100000000 / n) * n;
      do {
        crypto.getRandomValues(buf);
      } while (buf[0] >= zone);
      return buf[0] % n;
    };
    const length = Math.min(64, Math.max(8, options.length));
    const chars = pools.map((p) => p[randIdx(p.length)]);
    const pool = pools.flat();
    while (chars.length < length) chars.push(pool[randIdx(pool.length)]);
    for (let i = chars.length - 1; i > 0; i -= 1) {
      const j = randIdx(i + 1);
      [chars[i], chars[j]] = [chars[j], chars[i]];
    }
    return chars.slice(0, length).join('');
  },
  async vaultPasswordStrength(password: string): Promise<number> {
    return vaultStrengthMock(password);
  },
};

// ===== bench mock 状态与假流式代理 =====

const MOCK_AGENTS: AgentInfo[] = [
  {
    id: 'claude',
    name: 'Claude Code',
    installed: true,
    version: '2.1.251',
    configRoot: 'C:\\Users\\you\\.claude',
    program: 'C:\\Users\\you\\AppData\\Roaming\\npm\\claude.cmd',
    capabilities: { mcp: true, projectMcp: true, rules: true, skills: true, toolGranularity: true },
    chat: true,
    promptInject: 'argv',
    launchOptions: {
      model: { choices: ['default', 'sonnet', 'opus', 'haiku'], argTemplate: '--model {v}', default: 'default', streamOverride: true },
      effort: null,
    },
    streaming: true,
  },
  {
    id: 'codex',
    name: 'Codex',
    installed: true,
    version: '0.152.0',
    configRoot: 'C:\\Users\\you\\.codex',
    program: 'C:\\Users\\you\\AppData\\Roaming\\npm\\codex.cmd',
    capabilities: { mcp: true, projectMcp: false, rules: false, skills: false, toolGranularity: false },
    chat: true,
    promptInject: null,
    launchOptions: {
      model: { choices: ['gpt-5.3', 'gpt-5.3-mini'], argTemplate: '-m {v}', default: 'gpt-5.3', streamOverride: false },
      effort: { choices: ['minimal', 'low', 'medium', 'high'], argTemplate: '-c model_reasoning_effort={v}', default: 'medium', streamOverride: false },
    },
    streaming: true,
  },
  {
    id: 'zcode',
    name: 'ZCode',
    installed: true,
    version: '0.4.2',
    configRoot: 'C:\\Users\\you\\.zcode',
    program: null,
    capabilities: { mcp: true, projectMcp: true, rules: true, skills: false, toolGranularity: false },
    chat: true,
    promptInject: 'argv',
    launchOptions: null,
    streaming: false,
  },
  {
    id: 'gemini',
    name: 'Gemini CLI',
    installed: false,
    version: null,
    configRoot: null,
    program: null,
    capabilities: { mcp: true, projectMcp: false, rules: false, skills: false, toolGranularity: false },
    chat: false,
    promptInject: null,
    launchOptions: null,
    streaming: false,
  },
];

const MOCK_PROJECTS = ['D:\\ksa\\desk-helper', 'D:\ksa\notes-app', 'D:\\work\\api-server'];

const MOCK_HISTORY: SessionSummary[] = [
  {
    agent: 'claude',
    sessionKey: '8f2a11cd-90ab',
    projectPath: 'D:\\ksa\\desk-helper',
    title: '修复 Spotlight 拼音首字母搜索的回退',
    lastActiveAt: Date.now() - 3600_000 * 5,
    sourcePath: 'C:\\Users\\you\\.claude\\projects\\D--ksa-desk-helper\\8f2a11cd.jsonl',
    resumable: true,
    resumeViaTui: false,
  },
  {
    agent: 'codex',
    sessionKey: 'thread_9d01',
    projectPath: 'D:\ksa\notes-app',
    title: '为 ACP 方言补充 opencode 模型切换',
    lastActiveAt: Date.now() - 3600_000 * 26,
    sourcePath: 'C:\\Users\\you\\.codex\\sessions\\thread_9d01.jsonl',
    resumable: true,
    resumeViaTui: true,
  },
  {
    agent: 'claude',
    sessionKey: '3c77b0e4-55f1',
    projectPath: 'D:\\work\\api-server',
    title: '订单导出接口加流式分页',
    lastActiveAt: Date.now() - 3600_000 * 72,
    sourcePath: 'C:\\Users\\you\\.claude\\projects\\D--work-api-server\\3c77b0e4.jsonl',
    resumable: true,
    resumeViaTui: false,
  },
];

const MOCK_HITS: SearchHit[] = [
  {
    agent: 'claude',
    sessionKey: '8f2a11cd-90ab',
    seq: 4,
    sessionTitle: '修复 Spotlight 拼音首字母搜索的回退',
    projectPath: 'D:\\ksa\\desk-helper',
    at: Date.now() - 3600_000 * 5,
    snippet: '…把 `pinyin` 的首字母形态列加进 FTS5 触发器，查询时先走 <mark>match</mark> 再退化 LIKE…',
  },
  {
    agent: 'claude',
    sessionKey: '3c77b0e4-55f1',
    seq: 2,
    sessionTitle: '订单导出接口加流式分页',
    projectPath: 'D:\\work\\api-server',
    at: Date.now() - 3600_000 * 72,
    snippet: '…导出走 SSE 后内存占用从 1.2GB 降到 80MB，分页窗口用 cursor <mark>match</mark> 上一页末尾…',
  },
];

const MOCK_MESSAGES: Record<string, SnapshotMessage[]> = {
  'claude/8f2a11cd-90ab': [
    { seq: 0, role: 'user', text: 'Spotlight 搜「wx」搜不到微信，拼音首字母好像没进索引', toolName: null, at: Date.now() - 3600_000 * 5 },
    { seq: 1, role: 'thinking', text: '先看 file_fts 的触发器是否只写了全拼形态…', toolName: null, at: Date.now() - 3600_000 * 5 },
    { seq: 2, role: 'tool', text: 'Grep pinyin_cols migrations/', toolName: 'grep', at: Date.now() - 3600_000 * 5 },
    { seq: 3, role: 'assistant', text: '确认了：0002 迁移只写了全拼列，没有首字母形态。我来补上。', toolName: null, at: Date.now() - 3600_000 * 5 },
    { seq: 4, role: 'assistant', text: '把首字母形态列加进 FTS5 触发器，查询时先走 match 再退化 LIKE，已验证 wx → 微信 命中。', toolName: null, at: Date.now() - 3600_000 * 5 },
  ],
};

interface PtySession {
  info: LiveSessionInfo;
  timers: ReturnType<typeof setTimeout>[];
  onData: { current: { onmessage: (data: number[]) => void } };
  push: (text: string) => void;
}

const ptySessions = new Map<string, PtySession>();

interface LiveStream {
  info: LiveSessionInfo;
  busy: boolean;
  timers: ReturnType<typeof setTimeout>[];
}

const liveStreams = new Map<string, LiveStream>();

let fakeItemSeq = 0;

/** 假代理事件统一走 benchStreamEvent 事件总线（与真机 specta 事件同通道） */
function emitEv(sessionId: string, kind: StreamEvent['kind'], text: string, extra?: Partial<StreamEvent>): void {
  mockEvents.benchStreamEvent._emit({
    sessionId,
    itemId: '',
    at: Date.now(),
    kind,
    text,
    toolName: null,
    status: null,
    diff: null,
    ...extra,
  });
}

/** 假流式代理：一个轮次的完整剧本（思考 → 工具 → 流式回答 → 收口） */
/** 「分组/整理」指令的假回合：真实改写 home.layout 并广播刷新，
 *  浏览器预览即可所见即所得（真机由 agent 经 hamster-desktop MCP 工具完成同样的写入）。
 *  分组方案与真机 agent 提示词同思路：≥2 个同类应用成夹，系统/杂项散放，Dock 与壁纸保留。 */
function scheduleOrganizeTurn(sessionId: string, userText: string): void {
  const s = liveStreams.get(sessionId);
  if (!s || s.busy) return;
  s.busy = true;
  fakeItemSeq += 1;
  const n = fakeItemSeq;
  const at = (ms: number, fn: () => void) => s.timers.push(setTimeout(fn, ms));

  const CAT_NAME: Record<string, string> = {
    communication: L('沟通', 'Communication'),
    office: L('办公', 'Office'),
    dev: L('开发', 'Dev'),
    entertainment: L('娱乐', 'Fun'),
    tools: L('工具', 'Tools'),
  };
  const groups = new Map<string, string[]>();
  const rest: string[] = [];
  for (const a of APPS) {
    const c = classifyApp(a.display_name);
    if (c === 'other' || c === 'system') {
      rest.push(a.app_key);
      continue;
    }
    const cur = groups.get(c) ?? [];
    cur.push(a.app_key);
    groups.set(c, cur);
  }
  const folders: Record<string, { name: string; apps: string[] }> = {};
  const folderSlots: string[] = [];
  const leftovers: string[] = [];
  let fi = 0;
  for (const [c, keys] of groups) {
    if (keys.length < 2) {
      leftovers.push(...keys);
      continue;
    }
    const id = `mock-f${fi++}`;
    folders[id] = { name: CAT_NAME[c] ?? c, apps: keys };
    folderSlots.push(`folder:${id}`);
  }
  let wallpaper = 'burrow';
  let dock: string[] = [];
  const stored = ls()['home.layout'];
  if (stored) {
    try {
      const prev = JSON.parse(stored) as { wallpaper?: string; dock?: string[] };
      wallpaper = prev.wallpaper ?? wallpaper;
      dock = prev.dock ?? dock;
    } catch {
      // 损坏布局按默认处理
    }
  }
  const pages: string[][] = [[]];
  for (const slot of [
    ...folderSlots,
    ...rest.map((k) => `app:${k}`),
    ...leftovers.map((k) => `app:${k}`),
  ]) {
    const page = pages[pages.length - 1];
    if (page.length >= 35) pages.push([slot]);
    else page.push(slot);
  }
  const layout = { version: 1, wallpaper, pages, dock, folders, customized: true };
  const names = Object.values(folders).map((f) => `${f.name}(${f.apps.length})`);
  const reply = L(
    `已按用途整理完成：${names.join('、')}。` +
      '其余应用排在文件夹后面，Dock 与壁纸保持不变，主屏已即时刷新。' +
      '点文件夹即可展开查看；长按图标进入编辑模式后可拖拽调整、点标题重命名。',
    `Organized by purpose: ${names.join(', ')}. ` +
      'Remaining apps follow the folders; the dock and wallpaper are untouched, and the home screen has refreshed instantly. ' +
      'Tap a folder to expand it; long-press an icon to enter edit mode, drag to rearrange, or tap a folder title to rename.',
  );

  at(0, () => emitEv(sessionId, 'turnStarted', ''));
  at(120, () => emitEv(sessionId, 'userEcho', userText, { itemId: `u-${n}` }));
  at(500, () =>
    emitEv(sessionId, 'toolItem', L('home_apps_list 列出本机应用', 'home_apps_list enumerates installed apps'), {
      itemId: `t-${n}-1`,
      toolName: 'home_apps_list',
      status: 'inProgress',
    }),
  );
  at(1200, () =>
    emitEv(sessionId, 'toolItem', L(`home_apps_list 共 ${APPS.length} 个应用`, `home_apps_list found ${APPS.length} apps`), {
      itemId: `t-${n}-1`,
      toolName: 'home_apps_list',
      status: 'completed',
    }),
  );
  at(1600, () =>
    emitEv(sessionId, 'toolItem', L('home_layout_set 写入分组布局', 'home_layout_set writes the grouped layout'), {
      itemId: `t-${n}-2`,
      toolName: 'home_layout_set',
      status: 'inProgress',
    }),
  );
  at(2400, () => {
    lsSet('home.layout', JSON.stringify(layout));
    window.dispatchEvent(new Event('hamster:layout-updated-dom'));
    emitEv(sessionId, 'toolItem', `home_layout_set 已写入 ${names.length} 个文件夹，主屏已刷新`, {
      itemId: `t-${n}-2`,
      toolName: 'home_layout_set',
      status: 'completed',
    });
  });
  at(3000, () => emitEv(sessionId, 'agentDone', reply, { itemId: `a-${n}` }));
  at(3200, () => {
    emitEv(sessionId, 'turnCompleted', '');
    s.busy = false;
    s.info.lastActiveAt = Date.now();
  });
}

function scheduleFakeTurn(sessionId: string, userText: string): void {
  // 整理/分组类指令走「真实改写布局」的演示回合（所见即所得）
  if (/整理|分组|分类|文件夹|organi[sz]e|group|folder|sort|categor/i.test(userText)) {
    scheduleOrganizeTurn(sessionId, userText);
    return;
  }
  const s = liveStreams.get(sessionId);
  if (!s || s.busy) return;
  s.busy = true;
  s.info.lastActiveAt = Date.now();
  fakeItemSeq += 1;
  const n = fakeItemSeq;
  const at = (ms: number, fn: () => void) => s.timers.push(setTimeout(fn, ms));

  const answer = L(
    `好的，收到你的问题。针对「${userText.slice(0, 24)}${userText.length > 24 ? '…' : ''}」，我的建议如下：\n\n` +
      `1. **先定位**：从入口函数开始跟踪数据流，确认问题出现在哪一层。\n` +
      `2. **再验证**：补一个最小复现用例，避免只修表象。\n` +
      `3. **最后收口**：回归相关路径后提交。\n\n` +
      '示例代码：\n\n```rust\nfn main() {\n    println!("仓鼠Hub 🐹");\n}\n```\n\n' +
      '如果方向不对，告诉我更多上下文，我再调整。（浏览器 mock 输出——真机上将由官方 CLI 流式返回）',
    `Got it. Regarding "${userText.slice(0, 24)}${userText.length > 24 ? '…' : ''}", here is my advice:\n\n` +
      `1. **Locate first**: trace the data flow from the entry function to find the failing layer.\n` +
      `2. **Then verify**: add a minimal reproducer so you fix the cause, not the symptom.\n` +
      `3. **Close out**: regression-test the related paths, then commit.\n\n` +
      'Sample code:\n\n```rust\nfn main() {\n    println!("HamsterHub 🐹");\n}\n```\n\n' +
      'If the direction is off, give me more context and I will adjust. (Browser mock output — the real CLI streams here on desktop.)',
  );

  const reasonText = L(
    '先理解需求，再拆解步骤：定位 → 验证 → 收口，回复保持简洁。',
    'Understand the request, then decompose: locate → verify → close out. Keep replies concise.',
  );
  const diffText = '+let bench = BenchModule::new(ctx);\n+app.manage(bench);';

  at(0, () => emitEv(sessionId, 'turnStarted', ''));
  at(120, () => emitEv(sessionId, 'userEcho', userText, { itemId: `u-${n}` }));
  at(300, () => emitEv(sessionId, 'reasoningDelta', reasonText.slice(0, 12), { itemId: `r-${n}` }));
  at(900, () => emitEv(sessionId, 'reasoningDelta', reasonText.slice(12), { itemId: `r-${n}` }));
  at(1400, () => emitEv(sessionId, 'reasoningDone', reasonText, { itemId: `r-${n}` }));
  at(1650, () => emitEv(sessionId, 'toolItem', 'Read src/lib.rs', { itemId: `t-${n}-1`, toolName: 'read', status: 'inProgress' }));
  at(2200, () => emitEv(sessionId, 'toolItem', 'Read src/lib.rs', { itemId: `t-${n}-1`, toolName: 'read', status: 'completed' }));
  at(2450, () => emitEv(sessionId, 'toolItem', 'Edit src/main.rs', { itemId: `t-${n}-2`, toolName: 'edit', status: 'inProgress', diff: { path: 'src/main.rs', oldText: null, newText: diffText } }));
  at(3000, () => emitEv(sessionId, 'toolItem', 'Edit src/main.rs', { itemId: `t-${n}-2`, toolName: 'edit', status: 'completed', diff: { path: 'src/main.rs', oldText: null, newText: diffText } }));

  // agentDelta 按 24 字符切片流式推送，模拟真实打字节奏
  const chunk = 24;
  let cursor = 3200;
  for (let i = 0; i < answer.length; i += chunk) {
    const piece = answer.slice(i, i + chunk);
    at(cursor, () => emitEv(sessionId, 'agentDelta', piece, { itemId: `a-${n}` }));
    cursor += 90;
  }
  at(cursor + 120, () => emitEv(sessionId, 'agentDone', answer, { itemId: `a-${n}` }));
  at(cursor + 240, () => {
    emitEv(sessionId, 'turnCompleted', '');
    s.busy = false;
    s.info.lastActiveAt = Date.now();
  });
}
