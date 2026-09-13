/**
 * AssistantChat：桌面助手的 chatbot 视图（可复用）。
 * 输入 = bench stream-registry 的行（同一事件通道），输出 = 对话气泡：
 * 用户右气泡 / 助手左气泡（Markdown）/ 工具调用折叠为 chips / 系统错误高亮。
 * 思考流渲染为「💭 深度思考」折叠块：进行中自动展开直播，完成后自动收起；
 * 配合 useStreamRows 在任意宿主（悬浮球/侧栏/弹窗）挂载。
 */
import { useEffect, useCallback, useRef, useState, useSyncExternalStore } from 'react';
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

/** 深度思考折叠块：流式直播 → 完成自动收起；用户手点后以手动状态为准 */
function ThinkingBlock({ row }: { row: StreamRow }) {
  const streaming = !!row.streaming;
  const [open, setOpen] = useState(streaming);
  const [touched, setTouched] = useState(false);

  useEffect(() => {
    if (touched) return;
    if (streaming) setOpen(true);
    else setOpen(false);
  }, [streaming, touched]);

  const text = typeof row.text === 'string' ? row.text : '';
  return (
    <div className="rounded-xl bg-white/[0.06] px-2.5 py-1.5">
      <button
        onClick={() => {
          setTouched(true);
          setOpen((v) => !v);
        }}
        className="flex w-full items-center gap-1.5 text-left text-[10.5px] text-white/55 transition-colors hover:text-white/80"
      >
        <span>💭</span>
        <span>{streaming ? '深度思考中…' : '深度思考'}</span>
        <span className="flex-1" />
        <span className="text-white/35">{open ? '▾' : '▸'}</span>
      </button>
      {open && (
        <div className="mt-1 max-h-40 overflow-y-auto whitespace-pre-wrap break-words text-[11px] text-white/55">
          {text || '（无思考内容）'}
        </div>
      )}
    </div>
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

  // 防御：方言归一化偶发非字符串 text（如对象），渲染前统一转字符串
  const visible = rows.map((r) => ({
    ...r,
    text: typeof r.text === 'string' ? r.text : JSON.stringify(r.text),
  }));

  // 用户消息剥离「人设注入前缀」（resume 回播的历史首条会带整段 persona）
  const userText = (raw: string): string => {
    const at = raw.lastIndexOf('[用户]');
    return at >= 0 ? raw.slice(at + '[用户]'.length).trim() : raw;
  };

  return (
    <div ref={ref} className={`space-y-2 overflow-y-auto text-[11.5px] leading-relaxed ${className ?? ''}`}>
      {visible.map((r) => {
        if (r.role === 'user') {
          const text = userText(r.text);
          return (
            <div key={r.itemId} className="flex justify-end">
              <div
                title={text}
                className="max-w-[85%] rounded-2xl rounded-br-sm bg-[var(--accent)]/85 px-3 py-1.5 text-white [display:-webkit-box] [-webkit-box-orient:vertical] [-webkit-line-clamp:4] overflow-hidden"
              >
                {text}
              </div>
            </div>
          );
        }
        if (r.role === 'thinking') {
          return <ThinkingBlock key={r.itemId} row={r} />;
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
