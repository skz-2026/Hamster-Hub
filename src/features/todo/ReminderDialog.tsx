import { useEffect, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { AlarmClock, CalendarClock, Check } from 'lucide-react';
import { commands, events, type TodoReminder } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';
import { dueLabel } from './due';

/**
 * 待办到点提醒弹框（全局挂在 AppShell，任意页面/双形态都会弹）：
 * 后端调度线程扫到 remind_at 到点 → todoReminder 事件 → 居中模态弹框。
 * 与旧 toast 的差别：**不自动消失**，必须用户表态（完成 / 知道了 / Esc）才关，
 * 否则离开电脑回来提醒早已飘走。多条同时到点排队逐条处理。
 */
export default function ReminderDialog() {
  const { t } = useI18n();
  const qc = useQueryClient();
  const [queue, setQueue] = useState<TodoReminder[]>([]);
  const note = queue[0] ?? null;

  useEffect(() => {
    let unbind: (() => void) | undefined;
    let stopped = false;
    // 同 id 去重：后端 reminded_at 已防重发，这里只兜事件重放（如 dev 双挂载竞态）
    events.todoReminder
      .listen((e) =>
        setQueue((q) => (q.some((x) => x.id === e.payload.id) ? q : [...q, e.payload])),
      )
      .then((fn) => {
        if (stopped) fn();
        else unbind = fn;
      });
    return () => {
      stopped = true;
      unbind?.();
    };
  }, []);

  // Esc = 知道了（弹框为模态，无输入焦点劫持问题）
  useEffect(() => {
    if (!note) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setQueue((q) => q.slice(1));
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [note]);

  if (!note) return null;
  const when = dueLabel(note.due_at);

  const complete = () => {
    // 勾选失败（如窗口刚唤醒 IPC 抖动）不吞弹框，照常下一条；待办仍可手动勾。
    // 弹框在 useTodos 之外直调命令，完成后须手动失效 todo 查询，卡片才会刷新
    commands.todoToggle(note.id, true)
      .catch(() => {})
      .finally(() => qc.invalidateQueries({ queryKey: ['todo'] }));
    setQueue((q) => q.slice(1));
  };

  return (
    <div
      data-testid="reminder-dialog"
      className="fixed inset-0 z-50 grid place-items-center bg-black/45 p-6 backdrop-blur-xl"
    >
      <div
        role="alertdialog"
        aria-modal="true"
        aria-label={t('home.reminder.title')}
        className="rise-in w-[min(400px,88vw)] rounded-3xl bg-[#1c1a22]/95 p-6 text-white ring-1 ring-white/14 backdrop-blur-2xl"
        style={{ boxShadow: '0 32px 80px rgba(0,0,0,.5)' }}
      >
        <div className="flex items-center gap-2.5">
          <span className="grid size-9 shrink-0 place-items-center rounded-full bg-[var(--accent)]/15 text-[var(--accent)]">
            <AlarmClock size={17} />
          </span>
          <div className="min-w-0">
            <p className="text-[14px] font-semibold leading-tight">{t('home.reminder.title')}</p>
            {queue.length > 1 && (
              <p className="mt-0.5 text-[11px] text-white/50" data-testid="reminder-more">
                {t('home.reminder.more', { n: queue.length - 1 })}
              </p>
            )}
          </div>
        </div>

        <p className="mt-4 break-words text-[15px] font-medium leading-relaxed text-white/95">
          {note.content}
        </p>
        {when && (
          <p className="mt-1.5 flex items-center gap-1 text-[12px] text-[var(--accent)]">
            <CalendarClock size={12} />
            {when}
          </p>
        )}

        <div className="mt-5 flex gap-2">
          <button
            onClick={complete}
            data-testid="reminder-done"
            className="flex h-9 flex-1 items-center justify-center gap-1.5 rounded-xl bg-[var(--accent)] text-[13px] font-medium text-white transition-opacity hover:opacity-90"
          >
            <Check size={14} />
            {t('home.reminder.done')}
          </button>
          <button
            autoFocus
            onClick={() => setQueue((q) => q.slice(1))}
            data-testid="reminder-dismiss"
            className="h-9 flex-1 rounded-xl bg-white/10 text-[13px] text-white/85 ring-1 ring-white/12 transition-colors hover:bg-white/18"
          >
            {t('home.reminder.ok')}
          </button>
        </div>
      </div>
    </div>
  );
}
