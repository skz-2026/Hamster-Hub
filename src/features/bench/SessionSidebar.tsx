/**
 * SessionSidebar：左栏 = 新会话/Recall + 会话搜索 + 活会话 + 我的会话（按项目分组）。
 * 只渲染/管理通过松鼠Hub 发起或续聊过的会话（登记表，KV 持久化）：
 * 点击条目 → 聚焦活实例 → 有 sessionKey 直接流式续聊 → 无 key 走
 * latest_indexed_session 回退解析 → 仍无则同项目开新会话。悬停 × 可移除登记。
 */
import { useMemo, useState } from 'react';
import { ChevronDown, ChevronRight, Loader2, Plus, Search, Terminal, X } from 'lucide-react';
import { benchCommands } from '@/shared/lib/ipc';
import type { LiveSessionInfo } from '@/shared/types/bench';
import type { MySession } from './registry';
import { removeMySession } from './registry';
import AgentAvatar from './AgentAvatar';
import { useAgents, useLiveSessions, useMySessions, createStreamSession } from './hooks';
import { seedStreamRows } from './stream-registry';

function relTime(ms: number): string {
  const diff = Date.now() - ms;
  if (diff < 60_000) return '刚刚';
  if (diff < 3600_000) return `${Math.floor(diff / 60_000)} 分钟前`;
  if (diff < 86400_000) return `${Math.floor(diff / 3600_000)} 小时前`;
  return `${Math.floor(diff / 86400_000)} 天前`;
}

const normPath = (p: string): string =>
  p.replace(/[\\/]+/g, '/').replace(/\/$/, '').toLowerCase();

interface ProjGroup {
  key: string;
  dir: string;
  name: string;
  sessions: MySession[];
  latest: number;
}

function LiveItem({ s, name, active, onClick }: { s: LiveSessionInfo; name: string; active: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className={`w-full rounded-xl px-3 py-2 text-left transition-colors ${active ? 'bg-[var(--accent-weak)]' : 'hover:bg-[var(--hover)]'}`}
    >
      <div className="flex items-center gap-1.5">
        <AgentAvatar agentId={s.agentId} size={16} title={name} />
        <span className={`truncate text-[12.5px] font-medium ${s.running ? '' : 'opacity-70'}`}>{name}</span>
        <span className="ml-auto shrink-0 text-[10.5px] text-white/40">{relTime(s.lastActiveAt)}</span>
      </div>
      <p className="mt-0.5 truncate pl-[22px] text-[11px] text-white/45">{s.projectDir}</p>
    </button>
  );
}

function MyItem({ s, name, resuming, onOpen, onRemove }: { s: MySession; name: string; resuming: boolean; onOpen: () => void; onRemove: () => void }) {
  return (
    <div className="group relative">
      <button
        onClick={onOpen}
        className={`w-full rounded-lg py-1.5 pl-2.5 pr-7 text-left transition-colors hover:bg-[var(--hover)] ${resuming ? 'opacity-60' : ''}`}
      >
        <div className="flex items-center gap-1.5">
          {resuming ? (
            <Loader2 size={14} className="shrink-0 animate-spin text-[var(--accent)]" />
          ) : (
            <AgentAvatar agentId={s.agent} size={14} title={name} />
          )}
          <span className="truncate text-[12px]">{s.title}</span>
        </div>
        <p className="mt-0.5 pl-0.5 text-[10px] text-white/35">{relTime(s.lastActiveAt)}</p>
      </button>
      <button
        onClick={(e) => {
          e.stopPropagation();
          onRemove();
        }}
        title="从列表移除"
        className="absolute right-1.5 top-1.5 hidden size-5 place-items-center rounded text-white/40 hover:bg-white/10 hover:text-white group-hover:grid"
      >
        <X size={11} />
      </button>
    </div>
  );
}

export default function SessionSidebar({
  activeSessionId,
  onSelectSession,
  onResumed,
  onNewSession,
  onOpenRecall,
}: {
  activeSessionId: string | null;
  onSelectSession: (s: LiveSessionInfo) => void;
  onResumed: (s: LiveSessionInfo) => void;
  onNewSession: () => void;
  onOpenRecall: () => void;
}) {
  const live = useLiveSessions();
  const my = useMySessions();
  const agents = useAgents();
  const [search, setSearch] = useState('');
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [resumingKey, setResumingKey] = useState<string | null>(null);
  const [error, setError] = useState('');

  const agentName = useMemo(() => {
    const map = new Map<string, string>();
    for (const a of agents.query.data ?? []) map.set(a.id, a.name);
    return (id: string) => map.get(id) ?? id;
  }, [agents.query.data]);

  const mySessions = my.query.data ?? [];
  const registryKeys = useMemo(() => new Set(mySessions.map((s) => s.sessionKey).filter(Boolean)), [mySessions]);
  const registryIds = useMemo(() => new Set(mySessions.map((s) => s.id)), [mySessions]);

  const q = search.trim().toLowerCase();
  // 活会话只显示登记表内的（通过松鼠Hub 发起的）
  const liveSessions = (live.query.data ?? []).filter(
    (s) =>
      (registryKeys.has(s.resumeKey ?? '\u0000') || registryIds.has(s.sessionId)) &&
      (!q || agentName(s.agentId).toLowerCase().includes(q) || s.projectDir.toLowerCase().includes(q)),
  );
  const myFiltered = mySessions.filter(
    (s) => !q || s.title.toLowerCase().includes(q) || s.projectDir.toLowerCase().includes(q) || s.agent.includes(q),
  );

  // 按项目分组（组按最近活跃排序，组内同理）
  const groups = useMemo<ProjGroup[]>(() => {
    const map = new Map<string, ProjGroup>();
    for (const s of myFiltered) {
      const key = normPath(s.projectDir);
      let g = map.get(key);
      if (!g) {
        g = {
          key,
          dir: s.projectDir,
          name: s.projectDir.split(/[\\/]/).filter(Boolean).pop() ?? s.projectDir,
          sessions: [],
          latest: 0,
        };
        map.set(key, g);
      }
      g.sessions.push(s);
      g.latest = Math.max(g.latest, s.lastActiveAt);
    }
    const list = [...map.values()];
    for (const g of list) g.sessions.sort((a, b) => b.lastActiveAt - a.lastActiveAt);
    list.sort((a, b) => b.latest - a.latest);
    return list;
  }, [myFiltered]);

  const isExpanded = (g: ProjGroup): boolean => {
    if (q) return true;
    if (expanded.has(g.key)) return true;
    if (collapsed.has(g.key)) return false;
    return groups[0]?.key === g.key;
  };
  const toggleGroup = (g: ProjGroup): void => {
    const open = isExpanded(g);
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (open) next.add(g.key);
      else next.delete(g.key);
      return next;
    });
    setExpanded((prev) => {
      const next = new Set(prev);
      if (open) next.delete(g.key);
      else next.add(g.key);
      return next;
    });
  };

  /** 条目续聊：聚焦活实例 → sessionKey 流式 resume → latest 回退解析 key → 同项目新会话 */
  const continueEntry = async (entry: MySession): Promise<void> => {
    setError('');
    const fresh = ((await live.query.refetch()).data ?? []) as LiveSessionInfo[];
    const running = fresh.find(
      (x) =>
        x.running &&
        ((entry.sessionKey && x.resumeKey === entry.sessionKey) || x.sessionId === entry.id),
    );
    if (running) {
      onResumed(running);
      return;
    }
    const agent = (agents.query.data ?? []).find((a) => a.id === entry.agent);
    if (!agent?.installed || !agent.streaming) {
      setError(`${agentName(entry.agent)} 未安装或不支持 GUI 对话`);
      return;
    }
    setResumingKey(entry.id);
    try {
      let sessionKey = entry.sessionKey;
      if (!sessionKey) {
        const latest = await benchCommands.benchLatestIndexedSession(entry.agent, entry.projectDir);
        sessionKey = latest?.sessionKey ?? null;
      }
      const base = {
        agentId: entry.agent,
        projectDir: entry.projectDir,
        firstPrompt: null,
        model: null,
        effort: null,
      };
      if (!sessionKey) {
        // 该项目还没有可恢复的 rollout：直接开一个同项目新会话
        const info = await createStreamSession({ ...base, resumeKey: null });
        my.invalidate();
        onResumed(info);
        return;
      }
      const info = await createStreamSession({ ...base, resumeKey: sessionKey });
      try {
        const page = await benchCommands.benchListSessionMessages(entry.agent, sessionKey, -1, 60);
        seedStreamRows(info.sessionId, page.messages);
      } catch {
        /* 播种失败不阻断续聊 */
      }
      my.invalidate();
      onResumed(info);
    } catch (e) {
      setError(`${agentName(entry.agent)} 流式续聊失败（${String(e).replace(/^Error:\s*/, '')}）`);
    } finally {
      setResumingKey(null);
    }
  };

  return (
    <aside className="flex w-[264px] shrink-0 flex-col border-r border-white/8">
      <div className="flex gap-2 px-3 pb-2 pt-3">
        <button
          onClick={onNewSession}
          className="flex flex-1 items-center justify-center gap-1.5 rounded-xl bg-[var(--accent)] px-3 py-2 text-[12.5px] font-medium text-white transition-all hover:brightness-110"
        >
          <Plus size={14} strokeWidth={2.4} />
          新会话
        </button>
        <button
          onClick={onOpenRecall}
          title="Recall 全文搜索"
          className="grid size-9 place-items-center rounded-xl bg-white/8 text-white/70 ring-1 ring-white/10 transition-colors hover:bg-white/15 hover:text-white"
        >
          <Search size={15} />
        </button>
      </div>

      <div className="px-3 pb-2">
        <div className="flex items-center gap-1.5 rounded-xl bg-black/25 px-2.5 py-1.5 ring-1 ring-white/8 focus-within:ring-white/20">
          <Search size={12} className="shrink-0 text-white/35" />
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="搜索会话…"
            className="w-full bg-transparent text-[12px] text-white outline-none placeholder:text-white/35"
          />
        </div>
      </div>

      <div className="min-h-0 flex-1 space-y-4 overflow-y-auto px-3 pb-3">
        <section>
          <p className="flex items-center gap-1.5 px-1 pb-1.5 text-[11px] font-medium uppercase tracking-wide text-white/40">
            <Terminal size={11} />
            活会话
          </p>
          <div className="space-y-0.5">
            {liveSessions.map((s) => (
              <LiveItem
                key={s.sessionId}
                s={s}
                name={agentName(s.agentId)}
                active={s.sessionId === activeSessionId}
                onClick={() => onSelectSession(s)}
              />
            ))}
            {liveSessions.length === 0 && <p className="px-1 py-1.5 text-[11.5px] text-white/35">暂无进行中的会话</p>}
          </div>
        </section>

        <section>
          <p className="flex items-center gap-1.5 px-1 pb-1.5 text-[11px] font-medium uppercase tracking-wide text-white/40">
            会话（按项目）
          </p>
          <div className="space-y-1">
            {groups.map((g) => {
              const open = isExpanded(g);
              return (
                <div key={g.key}>
                  <button
                    onClick={() => toggleGroup(g)}
                    className="flex w-full items-center gap-1.5 rounded-lg px-1.5 py-1.5 text-left transition-colors hover:bg-[var(--hover)]"
                  >
                    {open ? <ChevronDown size={12} className="shrink-0 text-white/45" /> : <ChevronRight size={12} className="shrink-0 text-white/45" />}
                    <span className="truncate text-[12px] font-medium text-white/80">{g.name}</span>
                    <span className="ml-auto shrink-0 text-[10px] text-white/35">{g.sessions.length}</span>
                  </button>
                  {open && (
                    <div className="ml-2 border-l border-white/8 pl-1.5">
                      {g.sessions.map((s) => (
                        <MyItem
                          key={s.id}
                          s={s}
                          name={agentName(s.agent)}
                          resuming={resumingKey === s.id}
                          onOpen={() => void continueEntry(s)}
                          onRemove={() => {
                            void removeMySession((x) => x.id === s.id).then(() => my.invalidate());
                          }}
                        />
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
            {my.query.isLoading && <p className="px-1 py-2 text-[11.5px] text-white/35">加载中…</p>}
            {!my.query.isLoading && groups.length === 0 && (
              <p className="px-1 py-2 text-[11.5px] leading-relaxed text-white/35">
                {q ? '没有匹配的会话' : '还没有会话——从上方发起第一段对话'}
              </p>
            )}
          </div>
        </section>

        {error && <p className="px-1 text-[11.5px] leading-relaxed text-red-300">{error}</p>}
      </div>
    </aside>
  );
}
