/**
 * 会话登记表：只管理通过松鼠Hub 发起/续聊过的会话（侧栏唯一数据源）。
 * 持久化在 hamsterhub.db 的 KV（key=bench.mySessions，白名单已加）。
 * sessionKey 为 agent 侧会话键：resume 时已知；新建会话为空，
 * 恢复时经 benchLatestIndexedSession（agent+项目 → 最新键）回退解析。
 */
import { commands } from '@/shared/lib/ipc';

export interface MySession {
  /** 创建时的活会话 id（本运行内匹配活会话用） */
  id: string;
  agent: string;
  projectDir: string;
  /** 标题（首条 prompt 截断） */
  title: string;
  /** agent 侧会话键（恢复用；新建会话首轮完成前为 null） */
  sessionKey: string | null;
  createdAt: number;
  lastActiveAt: number;
}

const KEY = 'bench.mySessions';

/** 登记表变更通知（Query 层监听后 invalidate；同 home.layout 的 DOM 事件兜底模式） */
export const MY_SESSIONS_EVENT = 'bench:my-sessions';

function notify(): void {
  if (typeof window !== 'undefined') window.dispatchEvent(new Event(MY_SESSIONS_EVENT));
}

export async function loadMySessions(): Promise<MySession[]> {
  const raw = await commands.kvGet(KEY);
  if (!raw) return [];
  try {
    const list = JSON.parse(raw) as MySession[];
    return Array.isArray(list) ? list : [];
  } catch {
    return [];
  }
}

export async function saveMySessions(list: MySession[]): Promise<void> {
  await commands.kvSet(KEY, JSON.stringify(list));
  notify();
}

/** 登记一个新会话（welcome/sheet 创建后调用；title = 首条 prompt 截断） */
export async function addMySession(entry: {
  id: string;
  agent: string;
  projectDir: string;
  title: string;
  sessionKey: string | null;
}): Promise<void> {
  const list = await loadMySessions();
  const now = Date.now();
  list.unshift({
    id: entry.id,
    agent: entry.agent,
    projectDir: entry.projectDir,
    title: entry.title.length > 40 ? `${entry.title.slice(0, 40)}…` : entry.title,
    sessionKey: entry.sessionKey,
    createdAt: now,
    lastActiveAt: now,
  });
  await saveMySessions(list.slice(0, 200));
}

/** 更新条目（回填 sessionKey / 活跃时间 / 活会话 id） */
export async function patchMySession(
  match: (s: MySession) => boolean,
  patch: Partial<Pick<MySession, 'id' | 'sessionKey' | 'lastActiveAt' | 'title'>>,
): Promise<void> {
  const list = await loadMySessions();
  const next = list.map((s) => (match(s) ? { ...s, ...patch, lastActiveAt: Date.now() } : s));
  await saveMySessions(next);
}

export async function removeMySession(match: (s: MySession) => boolean): Promise<void> {
  const list = await loadMySessions();
  await saveMySessions(list.filter((s) => !match(s)));
}
