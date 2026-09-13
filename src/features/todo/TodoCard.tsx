import { useMemo, useState } from 'react';
import { CalendarClock, ListTodo, Plus, Loader2, BellRing } from 'lucide-react';
import { useI18n } from '@/shared/i18n/provider';
import { useTodos } from './hooks';
import { parseQuickAdd } from './nlp';
import { dueLabel } from './due';
import TodoItem from './TodoItem';

/** 待办卡（工作台）：自然语言快速添加（"明天下午3点交房租"）/ 勾选完成 / 截止+提醒 / 悬停删除 */
export default function TodoCard() {
  const { t } = useI18n();
  const { query, create, toggle, setDue, setRecur, remove } = useTodos();
  const [draft, setDraft] = useState('');
  const todos = query.data ?? [];
  const undone = todos.filter((t) => !t.done).length;

  // 输入即解析：chip 预览将抽出的截止时间
  const parsed = useMemo(() => parseQuickAdd(draft), [draft]);

  const submit = () => {
    if (!draft.trim()) return;
    create.mutate(
      { content: parsed.content || draft.trim(), dueAt: parsed.dueAt, remind: parsed.dueAt != null },
      { onSuccess: () => setDraft('') },
    );
  };

  return (
    <section className="card col-span-12 flex h-full min-h-[220px] flex-col p-5 md:col-span-4">
      <header className="mb-3 flex items-center justify-between">
        <div className="flex items-center gap-2 text-sm font-medium text-[var(--text)]">
          <ListTodo size={16} className="text-[var(--accent)]" />
          {t('home.todo.title')}
        </div>
        <span className="text-[11px] text-[var(--text-muted)]">
          {undone > 0 ? t('home.todo.undone', { n: undone }) : t('home.todo.allDone')}
        </span>
      </header>

      <div className="mb-2 flex gap-2">
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && submit()}
          placeholder={t('home.todo.addPlaceholder')}
          maxLength={200}
          className="h-9 min-w-0 flex-1 rounded-lg border border-[var(--border)] bg-transparent px-3 text-[13px] text-[var(--text)] placeholder:text-[var(--text-muted)] outline-none focus:border-[var(--accent)]"
        />
        <button
          onClick={submit}
          disabled={!draft.trim() || create.isPending}
          className="grid size-9 shrink-0 place-items-center rounded-lg bg-[var(--accent)] text-white transition-opacity hover:opacity-90 disabled:opacity-40"
          title={t('home.todo.add')}
        >
          {create.isPending ? (
            <Loader2 size={15} className="animate-spin" />
          ) : (
            <Plus size={15} />
          )}
        </button>
      </div>

      {draft.trim() && parsed.dueAt != null && (
        <p className="mb-2 flex items-center gap-1.5 px-1 text-[11px] text-[var(--text-muted)]">
          <CalendarClock size={11} className="text-[var(--accent)]" />
          {dueLabel(parsed.dueAt)}
          <BellRing size={10} className="text-[var(--accent)]" />
          {t('home.todo.remindChip')}
        </p>
      )}

      <div className="-mx-1 flex-1 space-y-0.5 overflow-y-auto px-1">
        {query.isLoading ? (
          <div className="grid place-items-center py-6">
            <Loader2 size={16} className="animate-spin text-[var(--text-muted)]" />
          </div>
        ) : todos.length === 0 ? (
          <p className="py-6 text-center text-xs text-[var(--text-muted)]">
            {t('home.todo.empty')}
          </p>
        ) : (
          todos.map((t) => (
            <TodoItem
              key={t.id}
              todo={t}
              onToggle={(done) => toggle.mutate({ id: t.id, done })}
              onSetDue={(dueAt, remind) => setDue.mutate({ id: t.id, dueAt, remind })}
              onSetRecur={(recur) => setRecur.mutate({ id: t.id, recur })}
              onRemove={() => remove.mutate(t.id)}
            />
          ))
        )}
      </div>
    </section>
  );
}
