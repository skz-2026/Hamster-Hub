/**
 * 消息 part 渲染器（assistant-ui components 配置用）：
 * Text → markdown；Reasoning → 思考卡（流式展开/完成后可折叠）；
 * tool-call → 工具卡（状态 + diff）；data:error → 错误卡；data:working → 打字指示。
 */
import { useState } from 'react';
import { Check, ChevronDown, ChevronRight, Loader2, TriangleAlert, X } from 'lucide-react';
import type { DataMessagePartProps, ReasoningMessagePartProps, TextMessagePartProps, ToolCallMessagePartProps } from '@assistant-ui/react';
import Markdown from './Markdown';

interface ToolResult {
  text: string;
  status: string | null;
  diff: { path: string; oldText: string | null; newText: string } | null;
}

export function MarkdownText({ text }: TextMessagePartProps) {
  return (
    <div className={text ? undefined : 'hidden'}>
      <Markdown text={text} />
    </div>
  );
}

export function ThinkingCard({ text, status }: ReasoningMessagePartProps) {
  const running = status?.type === 'running';
  const [open, setOpen] = useState(false);
  const expanded = open || running;
  return (
    <div className="rounded-xl bg-white/5 text-[12px] text-white/55 ring-1 ring-white/8">
      <button onClick={() => setOpen((v) => !v)} className="flex w-full items-center gap-1.5 px-3 py-2 text-left">
        {expanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
        <span className="text-white/65">{running ? '思考中…' : '已思考'}</span>
        {running && <span className="size-1.5 animate-pulse rounded-full bg-[var(--accent)]" />}
      </button>
      {expanded && <div className="whitespace-pre-wrap px-3 pb-2.5 leading-relaxed">{text}</div>}
    </div>
  );
}

function DiffCard({ path, oldText, newText }: { path: string; oldText: string | null; newText: string }) {
  return (
    <div className="mt-1.5 overflow-hidden rounded-lg border border-white/10 bg-black/30 text-[11.5px]">
      <div className="border-b border-white/8 px-2.5 py-1 font-mono text-white/55">{path}</div>
      <pre className="overflow-x-auto px-2.5 py-1.5 font-mono leading-[1.6]">
        {(oldText ?? '')
          .split('\n')
          .filter((l) => l.length > 0)
          .map((l, i) => (
            <div key={`o${i}`} className="text-red-300/70">{`- ${l}`}</div>
          ))}
        {newText
          .split('\n')
          .map((l, i) => (
            <div key={`n${i}`} className="text-emerald-300/85">{`+ ${l}`}</div>
          ))}
      </pre>
    </div>
  );
}

/** 工具卡：toolName + 状态图标 + 结果文本（result 内嵌上游的 status/diff payload） */
export function ToolCard({ toolName, result, isError }: ToolCallMessagePartProps) {
  const r = (result ?? {}) as Partial<ToolResult>;
  const failed = isError || r.status === 'failed';
  const running = r.status === 'inProgress';
  return (
    <div className="max-w-[85%] overflow-hidden rounded-xl bg-white/5 ring-1 ring-white/8">
      <div className="flex items-center gap-1.5 px-3 py-2 text-[12px] text-white/75">
        {running ? (
          <Loader2 size={12} className="shrink-0 animate-spin text-[var(--accent)]" />
        ) : failed ? (
          <X size={12} className="shrink-0 text-red-400" />
        ) : (
          <Check size={12} className="shrink-0 text-emerald-400/80" />
        )}
        <span className="text-white/45">{toolName}</span>
        {r.text && <span className="truncate font-mono">{r.text}</span>}
      </div>
      {r.diff && <DiffCard path={r.diff.path} oldText={r.diff.oldText} newText={r.diff.newText} />}
    </div>
  );
}

export function ErrorCard({ data }: DataMessagePartProps) {
  return (
    <div className="flex max-w-[85%] items-start gap-2 rounded-xl bg-red-400/10 px-3.5 py-2.5 text-[12.5px] leading-relaxed text-red-200/90 ring-1 ring-red-300/20">
      <TriangleAlert size={15} className="mt-0.5 shrink-0" />
      <span className="whitespace-pre-wrap">{String(data)}</span>
    </div>
  );
}

export function WorkingDots() {
  return (
    <div className="flex items-center gap-1.5 rounded-2xl rounded-bl-md bg-white/8 px-4 py-3 ring-1 ring-white/10">
      {[0, 1, 2].map((i) => (
        <span key={i} className="size-1.5 animate-bounce rounded-full bg-white/60" style={{ animationDelay: `${i * 140}ms` }} />
      ))}
    </div>
  );
}
