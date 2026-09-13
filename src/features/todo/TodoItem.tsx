import { useState } from 'react';
import { AlarmClock, Repeat, X } from 'lucide-react';
import type { Todo } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';
import { dueLabel, isOverdue } from './due';
import DueEditor from './DueEditor';

interface Props {
  todo: Todo;
  onToggle: (done: boolean) => void;
  onSetDue: (dueAt: number | null, remind: boolean) => void;
  onSetRecur: (recur: string | null) => void;
  onRemove: () => void;
}

const RECUR_KEY = {
  daily: 'home.todo.recur.daily',
  weekly: 'home.todo.recur.weekly',
  monthly: 'home.todo.recur.monthly',
  weekdays: 'home.todo.recur.weekdays',
} as const;

/** 单条待办行：勾选 / 内容 / 截止 chip（点开编辑）/ 循环徽标 / 删除 */
export default function TodoItem({ todo, onToggle, onSetDue, onSetRecur, onRemove }: Props) {
  const { t } = useI18n();
  const [editing, setEditing] = useState(false);
  const overdue = !todo.done && isOverdue(todo.due_at);
  const label = dueLabel(todo.due_at);
  const recurText =
    todo.recur && todo.recur in RECUR_KEY
      ? t(RECUR_KEY[todo.recur as keyof typeof RECUR_KEY])
      : todo.recur;

  return (
    <div>
      <div className="group flex items-center gap-2.5 rounded-lg px-2 py-1.5 transition-colors hover:bg-[var(--hover)]">
        <button
          role="checkbox"
          aria-checked={todo.done}
          onClick={() => onToggle(!todo.done)}
          title={todo.recur ? t('home.todo.toggleDoneRecur') : t('home.todo.toggleDone')}
          className={`grid size-[18px] shrink-0 place-items-center rounded-full border transition-colors ${
            todo.done
              ? 'border-[var(--accent)] bg-[var(--accent)] text-white'
              : 'border-[var(--text-muted)]'
          }`}
        >
          {todo.done && (
            <svg viewBox="0 0 10 8" width="9" height="7" fill="none">
              <path d="M1 4l2.5 2.5L9 1" stroke="currentColor" strokeWidth="1.6" />
            </svg>
          )}
        </button>
        <span
          className={`min-w-0 flex-1 truncate text-[13px] ${
            todo.done ? 'text-[var(--text-muted)] line-through' : 'text-[var(--text)]'
          }`}
        >
          {todo.content}
        </span>
        {todo.recur && (
          <span
            className="flex shrink-0 items-center gap-0.5 rounded-full bg-[var(--accent)]/15 px-1.5 py-0.5 text-[10px] text-[var(--accent)]"
            title={recurText ? t('home.todo.recur.badge', { rule: recurText }) : undefined}
          >
            <Repeat size={9} />
            {recurText}
          </span>
        )}
        <button
          onClick={() => setEditing((v) => !v)}
          title={label || t('home.todo.setDue')}
          className={`flex shrink-0 items-center gap-0.5 rounded-full px-1.5 py-0.5 text-[10px] tabular-nums transition-colors ${
            label ? '' : 'text-[var(--text-muted)] opacity-0 group-hover:opacity-100'
          } ${
            overdue
              ? 'bg-red-500/15 text-red-400'
              : label
                ? 'bg-[var(--hover)] text-[var(--text-muted)] hover:text-[var(--text)]'
                : 'hover:bg-[var(--hover)]'
          }`}
        >
          {todo.remind_at != null && <AlarmClock size={9} />}
          {label || t('home.todo.addDue')}
        </button>
        <button
          onClick={onRemove}
          title={t('home.todo.delete')}
          className="shrink-0 text-[var(--text-muted)] opacity-0 transition-opacity hover:text-red-400 group-hover:opacity-100"
        >
          <X size={13} />
        </button>
      </div>
      {editing && (
        <DueEditor
          todo={todo}
          onSave={onSetDue}
          onRecur={onSetRecur}
          onClose={() => setEditing(false)}
        />
      )}
    </div>
  );
}
