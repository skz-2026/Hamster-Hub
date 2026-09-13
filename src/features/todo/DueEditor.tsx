import { useState } from 'react';
import { BellRing, BellOff, Repeat, Trash2 } from 'lucide-react';
import type { Todo } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';
import { fromLocalInputValue, toLocalInputValue } from './due';

const RECUR_VALUES = ['daily', 'weekly', 'monthly', 'weekdays'] as const;

const RECUR_KEY = {
  daily: 'home.todo.recur.daily',
  weekly: 'home.todo.recur.weekly',
  monthly: 'home.todo.recur.monthly',
  weekdays: 'home.todo.recur.weekdays',
} as const;

interface Props {
  todo: Todo;
  onSave: (dueAt: number | null, remind: boolean) => void;
  onRecur: (recur: string | null) => void;
  onClose: () => void;
}

/** 截止时间编辑面板：时刻 + 提醒开关 + 循环规则 */
export default function DueEditor({ todo, onSave, onRecur, onClose }: Props) {
  const { t } = useI18n();
  const remindOn = todo.remind_at != null;
  const [value, setValue] = useState(() => toLocalInputValue(todo.due_at));
  const [remind, setRemind] = useState(remindOn);

  const submit = () => {
    onSave(fromLocalInputValue(value), remind && value !== '');
    onClose();
  };

  const inputCls =
    'rounded-lg border border-[var(--border)] bg-transparent px-2 py-1 text-[12px] text-[var(--text)] outline-none focus:border-[var(--accent)] [color-scheme:dark]';

  return (
    <div className="mx-2 mb-1.5 flex flex-wrap items-center gap-2 rounded-xl bg-[var(--hover)] px-2.5 py-2">
      <input
        type="datetime-local"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        className={inputCls}
        autoFocus
      />
      <button
        onClick={() => setRemind((r) => !r)}
        disabled={!value}
        title={remind ? t('home.todo.remindOn') : t('home.todo.remindOff')}
        className={`flex items-center gap-1 rounded-lg px-2 py-1 text-[11px] transition-colors disabled:opacity-35 ${
          remind && value
            ? 'bg-[var(--accent)]/20 text-[var(--accent)]'
            : 'text-[var(--text-muted)] hover:bg-[var(--border)]'
        }`}
      >
        {remind ? <BellRing size={12} /> : <BellOff size={12} />}
        {t('home.todo.remind')}
      </button>
      <span className="flex items-center gap-1 text-[11px] text-[var(--text-muted)]">
        <Repeat size={12} />
        <select
          value={todo.recur ?? ''}
          onChange={(e) => onRecur(e.target.value || null)}
          className="cursor-pointer bg-transparent text-[11px] text-[var(--text)] outline-none [&>option]:bg-[#26232b]"
        >
          <option value="">{t('home.todo.recur.none')}</option>
          {RECUR_VALUES.map((v) => (
            <option key={v} value={v}>
              {t(RECUR_KEY[v])}
            </option>
          ))}
        </select>
      </span>
      <span className="ml-auto flex items-center gap-1.5">
        {todo.due_at != null && (
          <button
            onClick={() => {
              onSave(null, false);
              onClose();
            }}
            className="flex items-center gap-1 rounded-lg px-2 py-1 text-[11px] text-[var(--text-muted)] transition-colors hover:text-red-400"
            title={t('home.todo.clearDue')}
          >
            <Trash2 size={11} />
            {t('home.todo.clear')}
          </button>
        )}
        <button
          onClick={submit}
          className="rounded-lg bg-[var(--accent)] px-2.5 py-1 text-[11px] text-white transition-opacity hover:opacity-90"
        >
          {t('home.todo.save')}
        </button>
      </span>
    </div>
  );
}
