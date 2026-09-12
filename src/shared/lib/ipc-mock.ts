/**
 * IPC mock 层：浏览器环境（无 tauri 后端）下的假实现，与生成契约同签名。
 * 仅由 lib/ipc.ts 在非 tauri 环境装配；应用代码不直接 import 本文件。
 */
import type {
  AppEntry,
  AppHealth,
  PluginInfo,
  CountdownCustom,
  CountdownItem,
  FileHit,
  Note,
  ProcInfo,
  Settings,
  SystemSnapshot,
  Todo,
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
  StreamEvent,
  WorkspaceRecord,
} from '@/shared/types/bench';

// ===== 假数据 =====

function svgIcon(letter: string, hue: number): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="120" height="120"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${hue},72%,58%)"/><stop offset="1" stop-color="hsl(${(hue + 40) % 360},68%,42%)"/></linearGradient></defs><rect width="120" height="120" rx="27" fill="url(#g)"/><text x="60" y="79" font-size="50" text-anchor="middle" fill="white" font-family="sans-serif" font-weight="600">${letter}</text></svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

const RAW_APPS: [string, string | null][] = [
  ['微信', svgIcon('微', 120)], ['QQ', svgIcon('Q', 210)], ['腾讯会议', svgIcon('会', 200)],
  ['钉钉', svgIcon('钉', 220)], ['Steam', svgIcon('S', 165)], ['网易云音乐', svgIcon('云', 340)],
  ['QQ音乐', svgIcon('音', 350)], ['Chrome', svgIcon('C', 50)], ['Edge', svgIcon('E', 210)],
  ['VS Code', svgIcon('V', 230)], ['ZCode', svgIcon('Z', 260)], ['Git', svgIcon('G', 20)],
  ['GIMP', svgIcon('G', 280)], ['Snipaste', svgIcon('S', 90)], ['Everything', null],
  ['potPlayer', null], ['OBS', svgIcon('O', 360)], ['Unity', null],
  ['Blender', svgIcon('B', 30)], ['WPS', svgIcon('W', 300)], ['Word', svgIcon('W', 220)],
  ['Excel', svgIcon('X', 140)], ['PowerPoint', svgIcon('P', 25)], ['百度网盘', svgIcon('盘', 330)],
  ['迅雷', svgIcon('雷', 60)], ['计算器', null], ['记事本', null], ['画图', null],
];

const APPS: AppEntry[] = RAW_APPS.map(([name, icon], i) => ({
  app_key: `mock:${name.toLowerCase()}`,
  display_name: name,
  exec_target: `C:\\mock\\${i}.lnk`,
  kind: 'lnk',
  icon_path: icon,
}));

const MOCK_FILES: FileHit[] = [
  { path: 'C:\\Users\\you\\Desktop\\微信截图_0901.png', name: '微信截图_0901.png', ext: 'png', kind: '图片', size: 240000, mtime: 1757000000 },
  { path: 'C:\\Users\\you\\Documents\\年度总结.docx', name: '年度总结.docx', ext: 'docx', kind: '文档', size: 88000, mtime: 1756900000 },
  { path: 'C:\\Users\\you\\Documents\\旅游计划.xlsx', name: '旅游计划.xlsx', ext: 'xlsx', kind: '文档', size: 45000, mtime: 1756800000 },
  { path: 'C:\\Users\\you\\Downloads\\hamster-hub-setup.exe', name: 'hamster-hub-setup.exe', ext: 'exe', kind: '', size: 2300000, mtime: 1757100000 },
  { path: 'C:\\Users\\you\\Pictures\\旅行vlog.mp4', name: '旅行vlog.mp4', ext: 'mp4', kind: '视频', size: 89000000, mtime: 1756500000 },
  { path: 'C:\\Users\\you\\Documents\\报销单2026.xlsx', name: '报销单2026.xlsx', ext: 'xlsx', kind: '文档', size: 30000, mtime: 1757200000 },
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
  agent: { computer_use_enabled: false, assistant_persona: '', mcp_port: null, mcp_user_token: null, mcp_agents: [] },
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
    entryPath: 'mock://hello-hamster/widget.js',
  },
  {
    id: 'hitokoto',
    name: '一言',
    version: '1.0.0',
    description: '随机刷一句一言（fetch 网络）',
    author: '仓鼠Hub',
    entry: 'widget.js',
    entryPath: 'mock://hitokoto/widget.js',
  },
];
const MOCK_PLUGIN_CODE: Record<string, string> = {
  'hello-hamster': `export default function (el, ctx) {
  let count = 0;
  el.innerHTML = '<div style="display:flex;flex-direction:column;justify-content:center;height:100%;gap:6px"><div style="font-size:11px;font-weight:500;color:rgba(255,255,255,.9)">你好仓鼠（浏览器预览）</div><div style="display:flex;align-items:center;gap:10px"><button data-act style="border:0;border-radius:999px;padding:4px 12px;font-size:12px;cursor:pointer;background:rgba(255,138,61,.85);color:#fff">囤一口</button><span data-n style="font-size:22px;font-weight:300;color:#fff">0</span></div></div>';
  const n = el.querySelector('[data-n]');
  const paint = () => (n.textContent = String(count));
  ctx.storage.get('count').then((v) => { count = Number(v ?? 0); paint(); });
  const onClick = () => { count += 1; paint(); ctx.storage.set('count', String(count)); };
  el.querySelector('[data-act]').addEventListener('click', onClick);
  return () => el.querySelector('[data-act]')?.removeEventListener('click', onClick);
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
};

// ===== mock 状态 =====

let desktopActive = false;
let MOCK_VOLUME = { level: 0.6, muted: false };
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

// ===== mock commands（与生成契约同签名） =====

export const mockCommands = {
  async aiChat(messages: { role: string; content: string }[]): Promise<string> {
    console.log('[mock] AI 会话', messages.length, '条');
    await new Promise((r) => setTimeout(r, 800));
    const last = messages[messages.length - 1]?.content ?? '';
    return `（浏览器 mock 回复）收到：「${last.slice(0, 50)}」。真机配置 API Key 后，这里会是模型的真实回答——这也是后续 Agent 的接入点。`;
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
  async windowSetPinned(_pinned: boolean): Promise<null> {
    console.log('[mock] windowSetPinned', _pinned);
    return null;
  },
  async openUrl(url: string): Promise<null> {
    console.log('[mock] 打开 URL', url);
    return null;
  },
  async fileSearch(query: string, _limit: number | null): Promise<FileHit[]> {
    const q = query.trim().toLowerCase();
    if (!q) return [];
    return MOCK_FILES.filter((f) => f.name.toLowerCase().includes(q)).slice(0, 6);
  },
  async todoList(): Promise<Todo[]> {
    const raw = ls()['todos'];
    if (raw) return JSON.parse(raw);
    const seed: Todo[] = [
      { id: 1, content: '体验仓鼠Hub 桌面模式', done: false, created_at: 0 },
      { id: 2, content: '把常用应用拖进 Dock', done: true, created_at: 0 },
    ];
    lsSet('todos', JSON.stringify(seed));
    return seed;
  },
  async todoCreate(content: string): Promise<Todo> {
    const list = await mockCommands.todoList();
    const t: Todo = {
      id: Date.now(),
      content,
      done: false,
      created_at: Math.floor(Date.now() / 1000),
    };
    lsSet('todos', JSON.stringify([t, ...list]));
    return t;
  },
  async todoToggle(id: number, done: boolean): Promise<null> {
    const list = await mockCommands.todoList();
    lsSet(
      'todos',
      JSON.stringify(list.map((t) => (t.id === id ? { ...t, done } : t))),
    );
    return null;
  },
  async todoDelete(id: number): Promise<null> {
    const list = await mockCommands.todoList();
    lsSet('todos', JSON.stringify(list.filter((t) => t.id !== id)));
    return null;
  },
  async weatherGet(): Promise<WeatherNow> {
    return {
      city: '北京',
      temp: 26,
      kind: '多云',
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
      { title: '距离周末', days: 1, kind: 'auto', emoji: '🛋️' },
      { title: '新年', days: 112, kind: 'auto', emoji: '🎊' },
    ];
  },
  async recentFiles(_limit: number | null): Promise<FileHit[]> {
    return [...MOCK_FILES].sort((a, b) => b.mtime - a.mtime).slice(0, 6);
  },
  async topApps(_limit: number | null): Promise<AppEntry[]> {
    return APPS.slice(0, 8);
  },
  async noteList(): Promise<Note[]> {
    const raw = ls()['notes'];
    if (raw) return JSON.parse(raw);
    const seed: Note[] = [
      { id: 1, content: '明早先回张总消息 📮', pinned: true, updated_at: 0 },
      { id: 2, content: '仓鼠Hub 发布前记得跑一遍还原测试矩阵', pinned: false, updated_at: 0 },
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
    return MOCK_AGENTS;
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
  async benchStreamCreate(
    agentId: string,
    projectDir: string,
    firstPrompt: string | null,
    _model: string | null,
    _effort: string | null,
    resumeKey: string | null,
    _fork: boolean,
  ): Promise<LiveSessionInfo> {
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
    const session = await mockCommands.benchStreamCreate(
      agentId,
      'C:\\Users\\hamster\\AppData\\Roaming\\com.hamsterhub.appgentssistant',
      args.firstPrompt ?? null,
      args.model ?? null,
      null,
      null,
      false,
    );
    return { session, agentId, mcpInjected: true, computerUseEnabled: false };
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
};

// ===== bench mock 状态与假流式代理 =====

const MOCK_AGENTS: AgentInfo[] = [
  {
    id: 'claude',
    name: 'Claude Code',
    installed: true,
    version: '2.1.251',
    configRoot: 'C:\\Users\\you\\.claude',
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
function scheduleFakeTurn(sessionId: string, userText: string): void {
  const s = liveStreams.get(sessionId);
  if (!s || s.busy) return;
  s.busy = true;
  s.info.lastActiveAt = Date.now();
  fakeItemSeq += 1;
  const n = fakeItemSeq;
  const at = (ms: number, fn: () => void) => s.timers.push(setTimeout(fn, ms));

  const answer =
    `好的，收到你的问题。针对「${userText.slice(0, 24)}${userText.length > 24 ? '…' : ''}」，我的建议如下：\n\n` +
    `1. **先定位**：从入口函数开始跟踪数据流，确认问题出现在哪一层。\n` +
    `2. **再验证**：补一个最小复现用例，避免只修表象。\n` +
    `3. **最后收口**：回归相关路径后提交。\n\n` +
    '示例代码：\n\n```rust\nfn main() {\n    println!("仓鼠Hub 🐹");\n}\n```\n\n' +
    '如果方向不对，告诉我更多上下文，我再调整。（浏览器 mock 输出——真机上将由官方 CLI 流式返回）';

  const reasonText = '先理解需求，再拆解步骤：定位 → 验证 → 收口，回复保持简洁。';
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
