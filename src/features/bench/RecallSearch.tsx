/**
 * RecallSearch：跨代理会话全文搜索（FTS5）+ 命中上下文回看 + 索引状态。
 */
import { useState } from 'react';
import { ArrowLeft, Database, RefreshCw, Search } from 'lucide-react';
import { benchCommands } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';
import type { TKey } from '@/shared/i18n/core';
import type { SearchHit, SnapshotMessage } from '@/shared/types/bench';
import { useIndexStatus, useRecallSearch } from './hooks';

function roleBadge(role: SnapshotMessage['role'], t: (key: TKey) => string): string {
  // 显式映射（枚举 → key，禁止动态拼 key）
  const map: Record<SnapshotMessage['role'], TKey> = {
    user: 'bench.role.user',
    assistant: 'bench.role.assistant',
    thinking: 'bench.role.thinking',
    tool: 'bench.role.tool',
    system: 'bench.role.system',
  };
  return t(map[role]);
}

function MessageRow({ m }: { m: SnapshotMessage }) {
  const { t } = useI18n();
  const accent =
    m.role === 'user'
      ? 'text-[var(--text)]'
      : m.role === 'assistant'
        ? 'text-[var(--text)]'
        : 'text-[var(--text-muted)]';
  return (
    <div className="rounded-xl bg-[var(--panel)] px-3 py-2 ring-1 ring-[var(--border)]">
      <div className="flex items-center gap-2 pb-0.5">
        <span className="rounded bg-[var(--panel-strong)] px-1.5 py-0.5 text-[10px] text-[var(--text-muted)]">{roleBadge(m.role, t)}</span>
        {m.toolName && <span className="font-mono text-[10.5px] text-[var(--text-muted)]">{m.toolName}</span>}
      </div>
      <p className={`whitespace-pre-wrap text-[12.5px] leading-relaxed ${accent}`}>{m.text}</p>
    </div>
  );
}

export default function RecallSearch({ onBack }: { onBack: () => void }) {
  const { t } = useI18n();
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
      <header className="flex shrink-0 items-center gap-2.5 border-b border-[var(--border)] px-5 py-3">
        <button onClick={onBack} title={t('bench.recall.back')} className="grid size-8 place-items-center rounded-lg text-[var(--text-muted)] hover:bg-[var(--hover)] hover:text-[var(--text)]">
          <ArrowLeft size={15} />
        </button>
        <Database size={15} className="text-[var(--accent)]" />
        <h2 className="text-[13px] font-medium">{t('bench.recall.title')}</h2>
        {status && (
          <span className="ml-auto text-[11px] text-[var(--text-muted)]">
            {t('bench.recall.indexStatus', {
              sessions: status.sessions,
              messages: status.messages,
              kb: (status.bytes / 1024).toFixed(0),
            })}
            <button
              onClick={() => benchCommands.benchReindex().then(() => index.query.refetch())}
              title={t('bench.recall.reindexTitle')}
              className="ml-2 inline-flex items-center gap-1 text-[var(--accent)] hover:brightness-125"
            >
              <RefreshCw size={11} />
              {t('bench.recall.reindex')}
            </button>
          </span>
        )}
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
        <div className="flex items-center gap-2 rounded-[18px] bg-[var(--panel-strong)] px-4 py-2.5 ring-1 ring-[var(--border)] backdrop-blur focus-within:ring-[var(--accent)]/40">
          <Search size={15} className="shrink-0 text-[var(--text-muted)]" />
          <input
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.nativeEvent.isComposing) run();
            }}
            placeholder={t('bench.recall.searchPlaceholder')}
            className="flex-1 bg-transparent text-[13px] text-[var(--text)] outline-none placeholder:text-[var(--text-muted)]"
            autoFocus
          />
        </div>

        {/* 命中列表 */}
        {!context && submitted && (
          <div className="space-y-2 pt-4">
            {search.isPending && <p className="text-[12px] text-[var(--text-muted)]">{t('bench.recall.searching')}</p>}
            {search.data?.length === 0 && <p className="text-[12px] text-[var(--text-muted)]">{t('bench.recall.noHits', { query: submitted })}</p>}
            {search.data?.map((h) => (
              <button
                key={`${h.agent}/${h.sessionKey}/${h.seq}`}
                onClick={() => openContext(h)}
                className="block w-full rounded-xl bg-[var(--panel)] px-3.5 py-2.5 text-left ring-1 ring-[var(--border)] transition-colors hover:bg-[var(--hover)]"
              >
                <div className="flex items-center gap-2">
                  <span className="shrink-0 rounded bg-[var(--panel-strong)] px-1.5 py-0.5 text-[10px] uppercase text-[var(--text-muted)]">{h.agent}</span>
                  <span className="truncate text-[12.5px] font-medium">{h.sessionTitle}</span>
                </div>
                <p
                  className="mt-1 line-clamp-2 text-[12px] leading-relaxed text-[var(--text-muted)] [&_mark]:rounded [&_mark]:bg-[var(--accent-weak)] [&_mark]:px-0.5 [&_mark]:text-[var(--accent)]"
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
              className="flex items-center gap-1.5 rounded-full bg-[var(--panel)] px-3 py-1.5 text-[11.5px] text-[var(--text-muted)] ring-1 ring-[var(--border)] transition-colors hover:bg-[var(--hover)]"
            >
              <ArrowLeft size={12} />
              {t('bench.recall.backToResults')}
            </button>
            <p className="text-[12px] text-[var(--text-muted)]">
              {context.hit.sessionTitle} · {context.hit.projectPath}
            </p>
            {context.messages.map((m) => (
              <MessageRow key={m.seq} m={m} />
            ))}
          </div>
        )}

        {!submitted && (
          <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
            <Search size={26} className="text-[var(--text-muted)] opacity-50" />
            <p className="text-[12.5px] text-[var(--text-muted)]">{t('bench.recall.hint')}</p>
          </div>
        )}
      </div>
    </div>
  );
}
