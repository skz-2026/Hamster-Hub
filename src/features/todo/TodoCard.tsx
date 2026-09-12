import { useState } from 'react';
import { ListTodo, Plus, X, Loader2 } from 'lucide-react';
import { useTodos } from './hooks';

/** 待办卡（工作台）：快速添加 / 勾选完成 / 悬停删除 */
export default function TodoCard() {
  const { query, create, toggle, remove } = useTodos();
  const [draft, setDraft] = useState('');
  const todos = query.data ?? [];
  const undone = todos.filter((t) => !t.done).length;

  const submit = () => {
    const content = draft.trim();
    if (!content) return;
    create.mutate(content, { onSuccess: () => setDraft('') });
  };

  return (
    <section className="card col-span-12 flex h-full min-h-[220px] flex-col p-5 md:col-span-4">
      <header className="mb-3 flex items-center justify-between">
        <div className="flex items-center gap-2 text-sm font-medium text-[var(--text)]">
          <ListTodo size={16} className="text-[var(--accent)]" />
          待办
        </div>
        <span className="text-[11px] text-[var(--text-muted)]">
          {undone > 0 ? `未完成 ${undone}` : '全部完成 🎉'}
        </span>
      </header>

      <div className="mb-3 flex gap-2">
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && submit()}
          placeholder="添加待办，回车确认"
          maxLength={200}
          className="h-9 min-w-0 flex-1 rounded-lg border border-[var(--border)] bg-transparent px-3 text-[13px] text-[var(--text)] placeholder:text-[var(--text-muted)] outline-none focus:border-[var(--accent)]"
        />
        <button
          onClick={submit}
          disabled={!draft.trim() || create.isPending}
          className="grid size-9 shrink-0 place-items-center rounded-lg bg-[var(--accent)] text-white transition-opacity hover:opacity-90 disabled:opacity-40"
          title="添加"
        >
          {create.isPending ? (
            <Loader2 size={15} className="animate-spin" />
          ) : (
            <Plus size={15} />
          )}
        </button>
      </div>

      <div className="-mx-1 flex-1 space-y-1 overflow-y-auto px-1">
        {query.isLoading ? (
          <div className="grid place-items-center py-6">
            <Loader2 size={16} className="animate-spin text-[var(--text-muted)]" />
          </div>
        ) : todos.length === 0 ? (
          <p className="py-6 text-center text-xs text-[var(--text-muted)]">
            暂无待办，加一条吧
          </p>
        ) : (
          todos.map((t) => (
            <div
              key={t.id}
              className="group flex items-center gap-2.5 rounded-lg px-2 py-1.5 transition-colors hover:bg-[var(--hover)]"
            >
              <button
                role="checkbox"
                aria-checked={t.done}
                onClick={() => toggle.mutate({ id: t.id, done: !t.done })}
                className={`grid size-[18px] shrink-0 place-items-center rounded-full border transition-colors ${
                  t.done
                    ? 'border-[var(--accent)] bg-[var(--accent)] text-white'
                    : 'border-[var(--text-muted)]'
                }`}
              >
                {t.done && (
                  <svg viewBox="0 0 10 8" width="9" height="7" fill="none">
                    <path d="M1 4l2.5 2.5L9 1" stroke="currentColor" strokeWidth="1.6" />
                  </svg>
                )}
              </button>
              <span
                className={`min-w-0 flex-1 truncate text-[13px] ${
                  t.done ? 'text-[var(--text-muted)] line-through' : 'text-[var(--text)]'
                }`}
              >
                {t.content}
              </span>
              <button
                onClick={() => remove.mutate(t.id)}
                title="删除"
                className="text-[var(--text-muted)] opacity-0 transition-opacity hover:text-red-400 group-hover:opacity-100"
              >
                <X size={13} />
              </button>
            </div>
          ))
        )}
      </div>
    </section>
  );
}
