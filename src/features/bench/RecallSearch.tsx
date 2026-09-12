/**
 * RecallSearch：跨代理会话全文搜索（FTS5）+ 命中上下文回看 + 索引状态。
 */
import { useState } from 'react';
import { ArrowLeft, Database, RefreshCw, Search } from 'lucide-react';
import { benchCommands } from '@/shared/lib/ipc';
import type { SearchHit, SnapshotMessage } from '@/shared/types/bench';
import { useIndexStatus, useRecallSearch } from './hooks';

function roleBadge(role: SnapshotMessage['role']): string {
  const map: Record<SnapshotMessage['role'], string> = {
    user: '用户',
    assistant: '代理',
    thinking: '思考',
    tool: '工具',
    system: '系统',
  };
  return map[role];
}

function MessageRow({ m }: { m: SnapshotMessage }) {
  const accent =
    m.role === 'user'
      ? 'text-white/90'
      : m.role === 'assistant'
        ? 'text-white/80'
        : 'text-white/50';
  return (
    <div className="rounded-xl bg-white/5 px-3 py-2 ring-1 ring-white/8">
      <div className="flex items-center gap-2 pb-0.5">
        <span className="rounded bg-white/8 px-1.5 py-0.5 text-[10px] text-white/55">{roleBadge(m.role)}</span>
        {m.toolName && <span className="font-mono text-[10.5px] text-white/40">{m.toolName}</span>}
      </div>
      <p className={`whitespace-pre-wrap text-[12.5px] leading-relaxed ${accent}`}>{m.text}</p>
    </div>
  );
}

export default function RecallSearch({ onBack }: { onBack: () => void }) {
  const [text, setText] = useState('');
  const [submitted, setSubmitted] = useState('');
  const [context, setContext] = useState<{ hit: SearchHit; messages: SnapshotMessage[] } | null>(null);
  const { search } = useRecallSearch();
  const index = useIndexStatus();

  const run = async () => {
    const q = text.trim();
    if (!q) return;
    setSubmitted(q);
    search.mutate({ text: q, agents: [], project: null, limit: 30 });
  };

  const openContext = async (hit: SearchHit) => {
    const messages = await benchCommands.benchSessionMessages(hit.agent, hit.sessionKey, hit.seq, 6);
    setContext({ hit, messages });
  };

  const status = index.query.data;

  return (
    <div className="flex h-full flex-col">
      <header className="flex shrink-0 items-center gap-2.5 border-b border-white/8 px-5 py-3">
        <button onClick={onBack} title="返回" className="grid size-8 place-items-center rounded-lg text-white/60 hover:bg-white/10 hover:text-white">
          <ArrowLeft size={15} />
        </button>
        <Database size={15} className="text-[var(--accent)]" />
        <h2 className="text-[13px] font-medium">Recall · 会话全文搜索</h2>
        {status && (
          <span className="ml-auto text-[11px] text-white/40">
            {status.sessions} 会话 · {status.messages} 消息 · {(status.bytes / 1024).toFixed(0)} KB
            <button
              onClick={() => benchCommands.benchReindex().then(() => index.query.refetch())}
              title="重建索引"
              className="ml-2 inline-flex items-center gap-1 text-[var(--accent)] hover:brightness-125"
            >
              <RefreshCw size={11} />
              重建
            </button>
          </span>
        )}
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
        <div className="flex items-center gap-2 rounded-[18px] bg-black/30 px-4 py-2.5 ring-1 ring-white/12 backdrop-blur focus-within:ring-white/25">
          <Search size={15} className="shrink-0 text-white/40" />
          <input
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.nativeEvent.isComposing) run();
            }}
            placeholder="搜索所有代理的历史会话…"
            className="flex-1 bg-transparent text-[13px] text-white outline-none placeholder:text-white/40"
            autoFocus
          />
        </div>

        {/* 命中列表 */}
        {!context && submitted && (
          <div className="space-y-2 pt-4">
            {search.isPending && <p className="text-[12px] text-white/40">搜索中…</p>}
            {search.data?.length === 0 && <p className="text-[12px] text-white/40">没有关于「{submitted}」的命中</p>}
            {search.data?.map((h) => (
              <button
                key={`${h.agent}/${h.sessionKey}/${h.seq}`}
                onClick={() => openContext(h)}
                className="block w-full rounded-xl bg-white/5 px-3.5 py-2.5 text-left ring-1 ring-white/8 transition-colors hover:bg-white/10"
              >
                <div className="flex items-center gap-2">
                  <span className="shrink-0 rounded bg-white/8 px-1.5 py-0.5 text-[10px] uppercase text-white/55">{h.agent}</span>
                  <span className="truncate text-[12.5px] font-medium">{h.sessionTitle}</span>
                </div>
                <p
                  className="mt-1 line-clamp-2 text-[12px] leading-relaxed text-white/55 [&_mark]:rounded [&_mark]:bg-[var(--accent-weak)] [&_mark]:px-0.5 [&_mark]:text-[var(--accent)]"
                  dangerouslySetInnerHTML={{ __html: h.snippet }}
                />
              </button>
            ))}
          </div>
        )}

        {/* 命中上下文回看 */}
        {context && (
          <div className="space-y-2 pt-4">
            <button
              onClick={() => setContext(null)}
              className="flex items-center gap-1.5 rounded-full bg-white/8 px-3 py-1.5 text-[11.5px] text-white/70 ring-1 ring-white/10 transition-colors hover:bg-white/15"
            >
              <ArrowLeft size={12} />
              返回结果
            </button>
            <p className="text-[12px] text-white/45">
              {context.hit.sessionTitle} · {context.hit.projectPath}
            </p>
            {context.messages.map((m) => (
              <MessageRow key={m.seq} m={m} />
            ))}
          </div>
        )}

        {!submitted && (
          <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
            <Search size={26} className="text-white/25" />
            <p className="text-[12.5px] text-white/45">输入关键词，搜索 Claude / Codex 等代理的全部历史会话</p>
          </div>
        )}
      </div>
    </div>
  );
}
