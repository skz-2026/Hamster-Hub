/**
 * 代理工作台（/bench）：上游域层整合 —— GUI 流式对话（assistant-ui）+ PTY TUI
 * （xterm，codex 等 resumeViaTui 会话的续聊）+ Recall 全文搜索。
 * 已打开的会话进「会话池」保持挂载（切走仅隐藏，终端缓冲/对话状态不丢）。
 */
import { useMemo, useState } from 'react';
import type { LiveSessionInfo, SessionSummary } from '@/shared/types/bench';
import { benchCommands } from '@/shared/lib/ipc';
import { useAgents, useBenchEvents, useLiveSessions } from '@/features/bench/hooks';
import { dropStreamRows } from '@/features/bench/stream-registry';
import BenchWelcome from '@/features/bench/BenchWelcome';
import ChatSessionView from '@/features/bench/ChatSessionView';
import SessionSidebar from '@/features/bench/SessionSidebar';
import NewSessionSheet from '@/features/bench/NewSessionSheet';
import RecallSearch from '@/features/bench/RecallSearch';

type MainView = { kind: 'welcome' } | { kind: 'chat'; uid: string } | { kind: 'recall' };

interface PoolEntry {
  uid: string;
  info: LiveSessionInfo;
}

export default function BenchPage() {
  const live = useLiveSessions();
  const agents = useAgents();
  useBenchEvents(true);
  const [view, setView] = useState<MainView>({ kind: 'welcome' });
  const [sheetOpen, setSheetOpen] = useState(false);
  const [pool, setPool] = useState<PoolEntry[]>([]);

  const agentName = useMemo(() => {
    const map = new Map<string, string>();
    for (const a of agents.query.data ?? []) map.set(a.id, a.name);
    return (id: string) => map.get(id) ?? id;
  }, [agents.query.data]);

  /** 打开（或激活）一个会话视图；uid 重复或 info 已在池中时只激活 */
  const openOrActivate = (info: LiveSessionInfo): void => {
    setPool((prev) => {
      const existing = prev.find(
        (e) => e.uid === info.sessionId || e.info.sessionId === info.sessionId,
      );
      if (existing) {
        setView({ kind: 'chat', uid: existing.uid });
        return prev;
      }
      setView({ kind: 'chat', uid: info.sessionId });
      return [...prev, { uid: info.sessionId, info }];
    });
  };

  const closeSession = (uid: string): void => {
    const entry = pool.find((e) => e.uid === uid);
    if (entry) {
      const real = !entry.info.sessionId.startsWith('pty:') ? entry.info.sessionId : null;
      if (entry.info.channel === 'pty') {
        // 占位会话的真实 sid 由 TerminalView 卸载清理兜底；真实 sid 此处直接结束
        if (real) void benchCommands.benchPtyKill(real).catch(() => {});
      } else {
        void benchCommands.benchStreamKill(entry.info.sessionId).catch(() => {});
        dropStreamRows(entry.info.sessionId);
      }
      live.invalidate();
    }
    setPool((prev) => prev.filter((e) => e.uid !== uid));
    if (view.kind === 'chat' && view.uid === uid) setView({ kind: 'welcome' });
  };

  return (
    <div className="relative flex h-full overflow-hidden rounded-2xl bg-[#1a191d] text-white">
      <SessionSidebar
        activeSessionId={view.kind === 'chat' ? view.uid : null}
        onSelectSession={openOrActivate}
        onResumed={(s) => {
          live.invalidate();
          openOrActivate(s);
        }}
        onNewSession={() => setSheetOpen(true)}
        onOpenRecall={() => setView({ kind: 'recall' })}
      />

      <main className="relative min-w-0 flex-1">
        {/* 会话池：全部保持挂载，非活跃会话仅隐藏（终端缓冲/对话状态保留） */}
        {pool.map(({ uid, info }) => (
          <div key={uid} className={view.kind === 'chat' && view.uid === uid ? 'h-full' : 'hidden'}>
            <ChatSessionView
              session={info}
              agentName={agentName(info.agentId)}
              onPtyReady={(real) =>
                setPool((prev) => prev.map((e) => (e.uid === uid ? { ...e, info: real } : e)))
              }
              onKill={() => closeSession(uid)}
            />
          </div>
        ))}
        {view.kind === 'recall' && <RecallSearch onBack={() => setView({ kind: 'welcome' })} />}
        {view.kind === 'welcome' && (
          <BenchWelcome
            onCreated={(info) => {
              live.invalidate();
              openOrActivate(info);
            }}
          />
        )}
      </main>

      <NewSessionSheet
        open={sheetOpen}
        onClose={() => setSheetOpen(false)}
        onCreated={(info) => {
          live.invalidate();
          openOrActivate(info);
        }}
      />
    </div>
  );
}
