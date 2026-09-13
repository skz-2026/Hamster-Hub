/**
 * 主屏悬浮球 v2：固定会话的 AI 整理助理（M4「Agent 原生桌面」快入口）。
 *
 * 会话记忆：
 * - 会话三元组（sessionId/resumeKey/agentId）持久化在 KV，面板收起/重挂不丢；
 * - 进程存活 → 直接续问（benchStreamSend，同会话记忆延续）；
 * - app 重启后进程已死 → 经 resumeKey 重建会话（codex thread/resume、
 *   claude --resume），hamster-desktop MCP 照常注入，历史消息从会话索引
 *   回播（seedStreamRows）——「恢复记忆继续对话」。
 *
 * 渲染复用 AssistantChat（chatbot 视图，可被其他入口复用）+ bench stream-registry。
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowUp, Bot, Loader2, RotateCcw, Settings2, Square, X } from 'lucide-react';
import { benchCommands, commands, events, isTauri } from '@/shared/lib/ipc';
import { listen } from '@tauri-apps/api/event';
import { useI18n } from '@/shared/i18n/provider';
import { useAgents, useBenchEvents } from '@/features/bench/hooks';
import {
  dropStreamRows,
  getStreamRows,
  isTurnRunning,
  seedStreamRows,
} from '@/features/bench/stream-registry';
import AssistantChat, { useStreamRows } from '@/features/agent/AssistantChat';
import type { AssistantSessionInfo } from '@/shared/types/bench';

type Phase = 'idle' | 'starting' | 'running' | 'done' | 'error';

/** 会话持久化键（KV）：悬浮球跨收起/重启的固定会话 */
const SESSION_KV = 'assistant.session.v1';

interface PersistedSession {
  sessionId: string;
  resumeKey: string | null;
  agentId: string;
}

async function loadPersisted(): Promise<PersistedSession | null> {
  try {
    const raw = await commands.kvGet(SESSION_KV);
    if (!raw) return null;
    const p = JSON.parse(raw) as PersistedSession;
    return p.sessionId && p.agentId ? p : null;
  } catch {
    return null;
  }
}

export default function FloatingAgent({ hidden }: { hidden?: boolean }) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [phase, setPhase] = useState<Phase>('idle');
  const [session, setSession] = useState<AssistantSessionInfo | null>(null);
  const [kv, setKv] = useState<PersistedSession | null>(null);
  const [restored, setRestored] = useState(false);
  const [input, setInput] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [refreshed, setRefreshed] = useState(false);
  const agents = useAgents();
  // bench 事件接线（registry 驱动 chatbot 视图；与 /agent 页同通道，路由互斥不叠加）
  useBenchEvents(true);

  const sid = session?.session.sessionId ?? null;
  const { version, rows } = useStreamRows(sid);
  // 等首轮 turnStarted 落地再允许收口（续问/首问在事件到达前 isTurnRunning=false）
  const awaitingTurn = useRef(false);

  // 启动恢复：读 KV → 会话仍存活则重挂（历史从会话索引回播），否则保留 resumeKey 备用
  useEffect(() => {
    let alive = true;
    (async () => {
      const p = await loadPersisted();
      if (!alive) return;
      setKv(p);
      if (!p) return;
      try {
        const live = await benchCommands.benchListStreamSessions();
        const hit = live.find((s) => s.sessionId === p.sessionId && s.running);
        if (!hit) return; // 进程已死：保留 kv.resumeKey，发送时走恢复
        setSession({
          session: hit,
          agentId: p.agentId,
          mcpInjected: true,
          computerUseEnabled: false,
        });
        setRestored(true);
        // 历史回播：会话索引的消息播种进 registry（registry 非空则跳过）
        if (getStreamRows(hit.sessionId).length === 0) {
          const page = await benchCommands.benchListSessionMessages(
            p.agentId,
            p.resumeKey ?? '',
            0,
            400,
          );
          if (alive && getStreamRows(hit.sessionId).length === 0) {
            seedStreamRows(
              hit.sessionId,
              // 注入的人设/环境上下文不进对话视图
              page.messages
                .filter((m) => m.role !== 'system')
                .map((m) => ({
                  seq: m.seq,
                  role: m.role,
                  text: m.text,
                  toolName: m.toolName,
                  at: m.at,
                })),
            );
          }
        }
      } catch {
        // 非 tauri / 索引不可用：按无历史处理
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  const send = async () => {
    const text = input.trim();
    if (!text || phase === 'starting' || phase === 'running') return;
    setInput('');
    setError(null);
    // 已有存活会话：同会话续问（记忆延续，不重建）
    if (session) {
      awaitingTurn.current = true;
      setPhase('running');
      try {
        await benchCommands.benchStreamSend(session.session.sessionId, text, null, null);
      } catch (e) {
        setError(errText(e));
        setPhase('error');
      }
      return;
    }
    // 新会话或恢复记忆（resumeKey 存在 → 重建同线程，MCP 照常注入）
    setPhase('starting');
    try {
      const info = await benchCommands.benchAssistantCreate({
        firstPrompt: text,
        model: null,
        effort: null,
        agentId: kv?.agentId ?? null,
        resumeKey: kv?.resumeKey ?? null,
      });
      setSession(info);
      setRestored(!!kv?.resumeKey);
      awaitingTurn.current = true;
      setPhase('running');
      // 持久化失败不影响本轮会话（下次发送会重试写入）
      commands
        .kvSet(
          SESSION_KV,
          JSON.stringify({
            sessionId: info.session.sessionId,
            resumeKey: info.session.resumeKey ?? null,
            agentId: info.agentId,
          } satisfies PersistedSession),
        )
        .catch((e) => console.error('[ball] 会话持久化失败', e));
    } catch (e) {
      setError(errText(e));
      setPhase('error');
    }
  };

  const stop = () => {
    if (!sid) return;
    void commands.benchStreamKill(sid).catch(() => {});
    setPhase('done');
  };

  /** 错误信息提取：tauri invoke 可能抛对象/非 Error，统一转可读字符串 */
  const errText = (e: unknown): string =>
    e instanceof Error ? e.message : typeof e === 'string' ? e : JSON.stringify(e);

  const reset = () => {
    if (sid) {
      void commands.benchStreamKill(sid).catch(() => {});
      dropStreamRows(sid);
    }
    void commands.kvSet(SESSION_KV, '').catch(() => {});
    setKv(null);
    setSession(null);
    setError(null);
    setRestored(false);
    setPhase('idle');
  };

  // 首个 turnStarted 落地 → 撤销等待标记
  useEffect(() => {
    if (phase !== 'running' || !sid) return;
    if (isTurnRunning(sid)) awaitingTurn.current = false;
  }, [phase, sid, version]);

  // 轮次收口：等待标记已清 + 无进行中回合 + 至少有一条行 → 完成
  useEffect(() => {
    if (phase !== 'running' || !sid || awaitingTurn.current) return;
    if (!isTurnRunning(sid) && rows.length > 1) {
      const t = window.setTimeout(() => setPhase((p) => (p === 'running' ? 'done' : p)), 800);
      return () => window.clearTimeout(t);
    }
  }, [phase, sid, version, rows.length]);

  // 进程退出兜底收口（助手单轮即退出；留出布局广播的到达时间）
  useEffect(() => {
    if (!sid) return;
    let alive = true;
    let un: (() => void) | undefined;
    events.benchStreamExit
      .listen((e) => {
        if (e.payload.session_id !== sid) return;
        window.setTimeout(() => {
          if (alive) setPhase((p) => (p === 'running' ? 'done' : p));
        }, 500);
      })
      .then((fn) => {
        if (alive) un = fn;
        else fn();
      });
    return () => {
      alive = false;
      un?.();
    };
  }, [sid]);

  // 布局被助手改写：主屏经 useHomeLayout 自动刷新（所见即所得），这里只挂提示角标
  useEffect(() => {
    if (!isTauri) return;
    let alive = true;
    let un: (() => void) | undefined;
    listen('hamster:layout-updated', () => {
      setRefreshed(true);
      window.setTimeout(() => setRefreshed(false), 2600);
    })
      .then((fn) => {
        if (alive) un = fn;
        else fn();
      })
      .catch(() => {});
    return () => {
      alive = false;
      un?.();
    };
  }, []);

  const noneInstalled = !(agents.query.data ?? []).some((a) => a.installed);
  const noAgent = noneInstalled || (error ?? '').includes('未找到已安装的 Agent');
  const busy = phase === 'starting' || phase === 'running';
  const hasChat = !!sid && (rows.length > 0 || phase === 'running');

  if (hidden) return null;

  return (
    <div className="absolute bottom-24 right-4 z-40 flex flex-col items-end gap-2">
      {open && (
        <div className="flex w-[min(340px,80vw)] flex-col overflow-hidden rounded-2xl bg-neutral-900/92 ring-1 ring-white/15 backdrop-blur-xl">
          {/* 标题条 */}
          <div className="flex shrink-0 items-center gap-2 px-3.5 pb-1.5 pt-3">
            <Bot size={14} className="text-[var(--accent)]" />
            <span className="text-[12px] font-medium text-white/90">{t('home.ball.title')}</span>
            <span className="flex-1" />
            {phase === 'running' && (
              <button
                onClick={stop}
                className="flex items-center gap-1 rounded-full bg-white/10 px-2 py-0.5 text-[10.5px] text-white/80 transition-colors hover:bg-white/20"
              >
                <Square size={9} />
                {t('home.ball.stop')}
              </button>
            )}
            {session && (
              <button
                onClick={reset}
                title={t('home.ball.newChat')}
                aria-label={t('home.ball.newChat')}
                className="grid size-5 place-items-center rounded-full text-white/50 transition-colors hover:text-white"
              >
                <RotateCcw size={12} />
              </button>
            )}
            <button
              onClick={() => setOpen(false)}
              aria-label={t('home.ball.collapse')}
              className="text-white/50 transition-colors hover:text-white"
            >
              <X size={14} />
            </button>
          </div>

          {noAgent && (phase === 'idle' || phase === 'error') ? (
            /* 未配置 Agent：提醒 + 引导 */
            <div className="px-3.5 pb-3.5">
              <p className="text-[12px] font-medium text-white/90">{t('agent.errNoAgentTitle')}</p>
              <p className="mt-1 text-[11px] leading-relaxed text-white/60">
                {t('agent.errNoAgentDetail')}
              </p>
              <button
                onClick={() => navigate('/agent')}
                className="mt-2.5 flex items-center gap-1.5 rounded-full bg-[var(--accent-weak)] px-3 py-1.5 text-[11.5px] font-medium text-[var(--accent)] ring-1 ring-white/15 transition-colors hover:brightness-110"
              >
                <Settings2 size={12} />
                {t('home.ball.goConfig')}
              </button>
            </div>
          ) : (
            <>
              {hasChat && sid && (
                <AssistantChat rows={rows} running={phase === 'running'} className="max-h-64 px-3.5 pb-2" />
              )}
              {restored && phase === 'idle' && (
                <p className="px-3.5 pb-1.5 text-[10.5px] text-white/45">{t('home.ball.restored')}</p>
              )}

              {phase !== 'running' && (
                <div className="flex shrink-0 items-end gap-1.5 px-3 pb-3 pt-1">
                  <textarea
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' && !e.shiftKey) {
                        e.preventDefault();
                        void send();
                      }
                      if (e.key === 'Escape') setOpen(false);
                    }}
                    rows={2}
                    placeholder={t('home.ball.placeholder')}
                    className="ios-ease max-h-24 min-h-[38px] flex-1 resize-none rounded-xl bg-white/12 px-3 py-2 text-[12px] text-white placeholder:text-white/45 ring-1 ring-white/15 outline-none focus:bg-white/18"
                  />
                  <button
                    onClick={() => void send()}
                    disabled={!input.trim() || phase === 'starting'}
                    aria-label={t('home.ball.send')}
                    className="grid size-9 shrink-0 place-items-center rounded-full bg-[var(--accent)] text-white transition-opacity disabled:opacity-40"
                  >
                    {phase === 'starting' ? (
                      <Loader2 size={15} className="animate-spin" />
                    ) : (
                      <ArrowUp size={15} />
                    )}
                  </button>
                </div>
              )}

              {phase === 'error' && error && !noAgent && (
                <p className="px-3.5 pb-3 text-[11px] text-amber-400">{error.slice(0, 160)}</p>
              )}
            </>
          )}
        </div>
      )}

      {/* 球：工作中转圈；布局被改写后右上角打勾提示 */}
      <button
        onClick={() => setOpen((v) => !v)}
        aria-label={t('home.ball.title')}
        title={t('home.ball.title')}
        className="relative grid size-12 place-items-center rounded-full bg-gradient-to-br from-[var(--accent)] to-rose-500 text-white shadow-xl ring-1 ring-white/25 transition-transform hover:scale-105 active:scale-95 ios-ease"
      >
        {busy ? <Loader2 size={20} className="animate-spin" /> : <Bot size={20} />}
        {refreshed && (
          <span className="absolute -right-0.5 -top-0.5 grid size-4 place-items-center rounded-full bg-emerald-400 text-[9px] font-bold text-black">
            ✓
          </span>
        )}
      </button>
    </div>
  );
}
