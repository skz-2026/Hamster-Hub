/**
 * AssistantChat：桌面助手的 chatbot 视图（可复用）。
 * 输入 = bench stream-registry 的行（同一事件通道），输出 = 对话气泡：
 * 用户右气泡 / 助手左气泡（Markdown）/ 工具调用折叠为 chips / 系统错误高亮。
 * 思考流默认隐藏（降噪）；配合 useStreamRows 在任意宿主（悬浮球/侧栏/弹窗）挂载。
 */
import { useEffect, useCallback, useRef, useSyncExternalStore } from 'react';
import Markdown from '@/features/bench/Markdown';
import {
  getStreamRows,
  getStreamVersion,
  subscribeStreamRows,
} from '@/features/bench/stream-registry';
import type { StreamRow } from '@/shared/types/bench';

/** 订阅某会话的行 + 版本号（ FloatingAgent 等宿主复用） */
export function useStreamRows(sessionId: string | null) {
  const version = useSyncExternalStore(
    useCallback(
      (cb: () => void) => (sessionId ? subscribeStreamRows(sessionId, cb) : () => {}),
      [sessionId],
    ),
    useCallback(() => (sessionId ? getStreamVersion(sessionId) : 0), [sessionId]),
  );
  return { version, rows: sessionId ? getStreamRows(sessionId) : [] };
}

function ToolChip({ row }: { row: StreamRow }) {
  const done = row.status === 'completed';
  const failed = row.status === 'failed' || row.status === 'error';
  return (
    <span
      title={row.text}
      className={`inline-flex max-w-full items-center gap-1 truncate rounded-full px-2 py-0.5 text-[10.5px] ${
        failed ? 'bg-red-500/15 text-red-300' : 'bg-white/10 text-white/65'
      }`}
    >
      🔧 {row.toolName ?? 'tool'}
      <span className="text-white/40">{done ? '✓' : failed ? '✗' : '…'}</span>
    </span>
  );
}

export default function AssistantChat({
  rows,
  running,
  className,
}: {
  rows: StreamRow[];
  running: boolean;
  className?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);

  // 新内容自动滚底
  useEffect(() => {
    const el = ref.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [rows.length, running]);

  // 思考流不渲染（降噪）；用户/助手/工具/系统四种形态
  const visible = rows.filter((r) => r.role !== 'thinking');

  return (
    <div ref={ref} className={`space-y-2 overflow-y-auto text-[11.5px] leading-relaxed ${className ?? ''}`}>
      {visible.map((r) => {
        if (r.role === 'user') {
          return (
            <div key={r.itemId} className="flex justify-end">
              <div className="max-w-[85%] rounded-2xl rounded-br-sm bg-[var(--accent)]/85 px-3 py-1.5 text-white">
                {r.text}
              </div>
            </div>
          );
        }
        if (r.role === 'tool') {
          return (
            <div key={r.itemId} className="flex">
              <ToolChip row={r} />
            </div>
          );
        }
        if (r.role === 'system') {
          return (
            <p key={r.itemId} className="text-amber-300">
              {r.text}
            </p>
          );
        }
        return (
          <div
            key={r.itemId}
            className="max-w-full whitespace-pre-wrap break-words rounded-2xl rounded-bl-sm bg-white/10 px-3 py-1.5 text-white/90"
          >
            <Markdown text={r.text} />
            {r.streaming && <span className="ml-0.5 animate-pulse text-white/50">▍</span>}
          </div>
        );
      })}
      {running && (
        <div className="flex gap-1 pl-1" aria-label="assistant typing">
          <span className="size-1.5 animate-bounce rounded-full bg-white/50" />
          <span className="size-1.5 animate-bounce rounded-full bg-white/40 [animation-delay:0.15s]" />
          <span className="size-1.5 animate-bounce rounded-full bg-white/30 [animation-delay:0.3s]" />
        </div>
      )}
    </div>
  );
}
