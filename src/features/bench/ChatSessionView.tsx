/**
 * ChatSessionView：单个会话视图，按通道分流：
 * - stream（GUI 流式）：assistant-ui 原生对话（useExternalStoreRuntime 桥接 registry）
 * - pty（TUI/混用模式）：xterm 终端（TerminalView），用户在 TUI 里直接交互
 */
import { useCallback, useMemo, useSyncExternalStore } from 'react';
import { AssistantRuntimeProvider, useExternalStoreRuntime, type AppendMessage } from '@assistant-ui/react';
import { Bot, SquareStack } from 'lucide-react';
import { benchCommands } from '@/shared/lib/ipc';
import type { LiveSessionInfo } from '@/shared/types/bench';
import ChatThread from './ChatThread';
import TerminalView from './TerminalView';
import { appendMessageText, rowsToThreadMessages } from './chat-mapping';
import { getStreamRows, getStreamVersion, isTurnRunning, pushUserOptimistic, subscribeStreamRows } from './stream-registry';

export default function ChatSessionView({
  session,
  agentName,
  onPtyReady,
  onKill,
}: {
  session: LiveSessionInfo;
  agentName: string;
  onPtyReady?: (info: LiveSessionInfo) => void;
  onKill: () => void;
}) {
  const isPty = session.channel === 'pty';
  const sid = session.sessionId;
  // 版本号订阅：任何流式事件（含 turnStarted 等无行变更事件）都驱动重渲染
  const version = useSyncExternalStore(
    (cb) => subscribeStreamRows(sid, cb),
    () => getStreamVersion(sid),
  );

  const running = isPty ? session.running : isTurnRunning(sid);
  const messages = useMemo(() => rowsToThreadMessages(getStreamRows(sid), running), [sid, version, running]);

  const onNew = useCallback(
    async (message: AppendMessage) => {
      const text = appendMessageText(message.content);
      if (!text) return;
      pushUserOptimistic(sid, text);
      await benchCommands.benchStreamSend(sid, text, null, null);
    },
    [sid],
  );
  const onCancel = useCallback(async () => {
    await benchCommands.benchStreamInterrupt(sid);
  }, [sid]);

  return (
    <div className="flex h-full flex-col">
      {/* 会话头部 */}
      <header className="flex shrink-0 items-center gap-2.5 border-b border-white/8 px-5 py-3">
        <span className="grid size-8 place-items-center rounded-lg bg-[var(--accent-weak)] ring-1 ring-white/10">
          <Bot size={15} className="text-[var(--accent)]" />
        </span>
        <div className="min-w-0">
          <p className="truncate text-[13px] font-medium leading-tight">
            {agentName}
            {isPty && <span className="ml-2 rounded bg-white/8 px-1.5 py-0.5 text-[10px] text-white/50">TUI</span>}
          </p>
          <p className="truncate text-[11px] text-white/45">{session.projectDir}</p>
        </div>
        <div className="ml-auto flex items-center gap-2">
          {running && (
            <span className="flex items-center gap-1.5 rounded-full bg-[var(--accent-weak)] px-2.5 py-1 text-[11px] text-[var(--accent)]">
              <span className="size-1.5 animate-pulse rounded-full bg-[var(--accent)]" />
              运行中
            </span>
          )}
          <button
            onClick={onKill}
            title="结束会话"
            className="flex items-center gap-1.5 rounded-full bg-white/8 px-3 py-1.5 text-[11.5px] text-white/70 ring-1 ring-white/10 transition-colors hover:bg-red-400/15 hover:text-red-200"
          >
            <SquareStack size={12} />
            结束
          </button>
        </div>
      </header>
      <div className="min-h-0 flex-1">
        {isPty ? (
          <TerminalView session={session} onPtyReady={onPtyReady} />
        ) : (
          <StreamThread session={session} running={running} messages={messages} onNew={onNew} onCancel={onCancel} />
        )}
      </div>
    </div>
  );
}

/** stream 通道：assistant-ui runtime 桥接（hooks 不可条件化，拆出子组件） */
function StreamThread({
  session,
  running,
  messages,
  onNew,
  onCancel,
}: {
  session: LiveSessionInfo;
  running: boolean;
  messages: ReturnType<typeof rowsToThreadMessages>;
  onNew: (m: AppendMessage) => Promise<void>;
  onCancel: () => Promise<void>;
}) {
  const sid = session.sessionId;
  const runtime = useExternalStoreRuntime({
    isRunning: running,
    messages,
    convertMessage: (m) => m,
    onNew,
    onCancel,
  });

  return (
    <AssistantRuntimeProvider runtime={runtime}>
      <div className="flex h-full flex-col">
        <div className="min-h-0 flex-1">
          <ChatThread />
        </div>
      </div>
    </AssistantRuntimeProvider>
  );
}
