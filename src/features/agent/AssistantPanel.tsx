/**
 * AssistantPanel：桌面助手（M4「Agent 原生桌面」前端入口）。
 * 经 benchAssistantCreate 创建 bench 桌面助手会话（persona + hamster-desktop
 * MCP server 注入，不绑项目目录），流式渲染复用 bench 域的 ChatSessionView
 * + stream-registry（同一事件通道）。微任务兜底（ai_chat）不在此层。
 */
import { useCallback, useEffect, useMemo, useState } from 'react';
import { Bot, Hammer, Play, ShieldAlert, TriangleAlert } from 'lucide-react';
import { benchCommands, commands } from '@/shared/lib/ipc';
import type { AssistantSessionInfo } from '@/shared/types/bench';
import ChatSessionView from '@/features/bench/ChatSessionView';
import { dropStreamRows } from '@/features/bench/stream-registry';
import { useAgents, useBenchEvents } from '@/features/bench/hooks';

/** 错误码 → 引导文案（未装 agent / hamster-mcp.exe 缺失） */
function errorHint(err: unknown): { title: string; detail: string } {
  const text = String(err);
  if (text.includes('MCP_EXE_MISSING')) {
    return {
      title: '桌面 MCP server 缺失',
      detail: 'hamster-mcp.exe 不存在：重新构建（cargo build -p hamster-mcp）或重装应用。',
    };
  }
  if (text.includes('未找到已安装的 Agent')) {
    return {
      title: '没有可用的 Agent CLI',
      detail: '桌面助手由本机已安装的 Agent 驱动（Claude Code / ZCode 等），请先安装任意一个。',
    };
  }
  return { title: '桌面助手启动失败', detail: text.slice(0, 200) };
}

export default function AssistantPanel({
  autoQuestion,
  onConsumed,
  onFallback,
}: {
  /** 外部带入的首个问题（Spotlight「问 AI」路由）；非空时自动启动会话 */
  autoQuestion?: string | null;
  /** 首个问题已消费回调 */
  onConsumed?: () => void;
  /** 启动失败时请求回退快问模式 */
  onFallback?: () => void;
}) {
  const [session, setSession] = useState<AssistantSessionInfo | null>(null);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<{ title: string; detail: string } | null>(null);
  const [question, setQuestion] = useState('');
  const agents = useAgents();
  // 流式事件订阅（与 bench 会话共用同一事件通道与 stream-registry）
  useBenchEvents(true);

  const agentName = useMemo(() => {
    const map = new Map<string, string>();
    for (const a of agents.query.data ?? []) map.set(a.id, a.name);
    return (id: string) => map.get(id) ?? id;
  }, [agents.query.data]);

  const start = useCallback(
    async (firstPrompt: string | null) => {
      if (starting || session) return;
      setStarting(true);
      setError(null);
      try {
        const info = await benchCommands.benchAssistantCreate({
          firstPrompt: firstPrompt,
          model: null,
          agentId: null,
        });
        setSession(info);
      } catch (e) {
        setError(errorHint(e));
      } finally {
        setStarting(false);
      }
    },
    [session, starting],
  );

  // Spotlight「问 AI」/ 页面预置问题：自动启动
  useEffect(() => {
    if (autoQuestion?.trim() && !session && !starting) {
      start(autoQuestion.trim());
      onConsumed?.();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoQuestion, session]);

  const closeSession = () => {
    if (session) {
      void commands.benchStreamKill(session.session.sessionId).catch(() => {});
      dropStreamRows(session.session.sessionId);
    }
    setSession(null);
  };

  // 已有会话：整幅复用 bench 会话视图（流式/工具卡/中断/续问全能力）
  if (session) {
    return (
      <div className="relative flex h-full flex-col">
        {session.mcpInjected && (
          <div
            className="flex shrink-0 items-center gap-1.5 border-b border-white/8 bg-black/25 px-5 py-1.5 text-[11px] text-white/55"
            title="本会话注入了 hamster-desktop MCP server：应用/文件/待办/音量 + 浏览器/桌面操作（后者受安全开关门控，全部调用留审计）"
          >
            <Hammer size={11} className="text-[var(--accent)]" />
            桌面工具已接入
            {session.computerUseEnabled ? '' : ' · 桌面点击/键入未开启（设置 → Agent）'}
          </div>
        )}
        <div className="min-h-0 flex-1">
          <ChatSessionView
            session={session.session}
            agentName={`${agentName(session.agentId)} · 桌面助手`}
            onKill={closeSession}
          />
        </div>
      </div>
    );
  }

  // 启动失败：错误 + 回退入口
  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-4 px-8 text-center">
        <span className="grid size-14 place-items-center rounded-2xl bg-amber-400/10 ring-1 ring-amber-300/25">
          <TriangleAlert size={24} className="text-amber-300" />
        </span>
        <div>
          <p className="text-[14px] font-medium text-white/85">{error.title}</p>
          <p className="mt-1.5 max-w-md text-[12px] leading-relaxed text-white/50">{error.detail}</p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => start(question.trim() || autoQuestion || null)}
            className="rounded-full bg-white/8 px-4 py-1.5 text-[12px] text-white/75 ring-1 ring-white/12 transition-colors hover:bg-white/15"
          >
            重试
          </button>
          {onFallback && (
            <button
              onClick={onFallback}
              className="rounded-full bg-[var(--accent-weak)] px-4 py-1.5 text-[12px] font-medium text-white/85 ring-1 ring-white/12 transition-colors hover:brightness-110"
            >
              用快问模式继续（AI 直连）
            </button>
          )}
        </div>
      </div>
    );
  }

  // 欢迎态：能力说明 + 首问输入
  return (
    <div className="flex h-full flex-col items-center justify-center gap-5 px-8 text-center">
      <span className="grid size-14 place-items-center rounded-2xl bg-[var(--accent-weak)] ring-1 ring-white/10">
        <Bot size={24} className="text-[var(--accent)]" />
      </span>
      <div>
        <p className="text-[14px] font-medium text-white/85">桌面助手 · 听得懂话，而且能动</p>
        <p className="mt-1.5 max-w-md text-[12px] leading-relaxed text-white/50">
          由本机 Agent 驱动，经桌面工具直接操作这台电脑：启动应用、找文件、记待办、调音量，
          需要时还能开浏览器查资料。所有工具调用留审计记录。
        </p>
      </div>
      <div className="flex w-full max-w-md items-center gap-2">
        <input
          value={question}
          onChange={(e) => setQuestion(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.nativeEvent.isComposing && question.trim()) {
              start(question.trim());
            }
          }}
          placeholder="例如：把「周五交周报」记成待办，然后打开计算器"
          className="h-10 flex-1 rounded-xl bg-black/30 px-4 text-[12.5px] text-white outline-none ring-1 ring-white/12 placeholder:text-white/35 focus:ring-white/25"
        />
        <button
          onClick={() => start(question.trim() || null)}
          disabled={starting}
          title="启动桌面助手"
          aria-label="启动桌面助手"
          className="grid size-10 shrink-0 place-items-center rounded-xl bg-[var(--accent)] text-white transition-all hover:brightness-110 disabled:opacity-40"
        >
          {starting ? (
            <span className="size-4 animate-spin rounded-full border-2 border-white/40 border-t-white" />
          ) : (
            <Play size={15} className="ml-0.5" />
          )}
        </button>
      </div>
      <p className="flex items-center gap-1.5 text-[11px] text-white/40">
        <ShieldAlert size={12} />
        桌面点击/键入（computer use）默认关闭，可在设置 → Agent 中开启
      </p>
    </div>
  );
}
