import { useEffect, useState } from 'react';
import { AlarmClock, X } from 'lucide-react';
import { events, type TodoReminder } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';
import { dueLabel } from './due';

/** 待办提醒应用内提示：监听后端调度事件，右下角浮出 8 秒 */
export default function ReminderToast() {
  const { t } = useI18n();
  const [note, setNote] = useState<TodoReminder | null>(null);

  useEffect(() => {
    let unbind: (() => void) | undefined;
    let stopped = false;
    events.todoReminder.listen((e) => setNote(e.payload)).then((fn) => {
      if (stopped) fn();
      else unbind = fn;
    });
    return () => {
      stopped = true;
      unbind?.();
    };
  }, []);

  useEffect(() => {
    if (!note) return;
    const t = setTimeout(() => setNote(null), 8000);
    return () => clearTimeout(t);
  }, [note]);

  if (!note) return null;
  const when = dueLabel(note.due_at);

  return (
    <div
      data-testid="reminder-toast"
      className="ios-ease fixed bottom-5 right-5 z-50 flex max-w-[320px] items-start gap-2.5 rounded-2xl bg-[#26232b]/95 px-4 py-3 shadow-xl ring-1 ring-white/10"
    >
      <AlarmClock size={16} className="mt-0.5 shrink-0 text-[var(--accent)]" />
      <div className="min-w-0">
        <p className="text-[13px] font-medium text-white/95">{t('home.reminder.title')}</p>
        <p className="mt-0.5 truncate text-[12px] text-white/70">{note.content}</p>
        {when && <p className="mt-0.5 text-[11px] text-[var(--accent)]">{when}</p>}
      </div>
      <button
        onClick={() => setNote(null)}
        className="shrink-0 text-white/40 transition-colors hover:text-white/80"
        title={t('home.reminder.ok')}
      >
        <X size={13} />
      </button>
    </div>
  );
}
