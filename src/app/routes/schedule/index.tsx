import { useState } from 'react';
import {
  CalendarDays,
  ChevronLeft,
  ChevronRight,
  Plus,
  StickyNote,
  Pin,
  PinOff,
  Trash2,
  Loader2,
  Target,
} from 'lucide-react';
import { Solar } from 'lunar-javascript';
import { useI18n } from '@/shared/i18n/provider';
import { useNotes, useCountdownCustom } from './hooks';

/** 周几用字面量 key 数组（周一起始），禁止动态拼 key */
const WEEK_KEYS = [
  'pages.weekday.mon',
  'pages.weekday.tue',
  'pages.weekday.wed',
  'pages.weekday.thu',
  'pages.weekday.fri',
  'pages.weekday.sat',
  'pages.weekday.sun',
] as const;

function fmt(d: Date) {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

/** 日程页：月视图（农历/节日高亮）+ 倒数日管理 + 便签墙 */
export default function SchedulePage() {
  const { t } = useI18n();
  const today = new Date();
  const [cursor, setCursor] = useState(() => new Date(today.getFullYear(), today.getMonth(), 1));
  const notes = useNotes();
  const countdown = useCountdownCustom();

  // 月历格子
  const year = cursor.getFullYear();
  const month = cursor.getMonth();
  const first = new Date(year, month, 1);
  const daysInMonth = new Date(year, month + 1, 0).getDate();
  // 周一开头
  const lead = (first.getDay() + 6) % 7;
  const cells: (number | null)[] = [
    ...Array.from({ length: lead }, () => null),
    ...Array.from({ length: daysInMonth }, (_, i) => i + 1),
  ];
  const todayStr = fmt(today);
  const isCurrentMonth = year === today.getFullYear() && month === today.getMonth();

  const shift = (delta: number) => setCursor(new Date(year, month + delta, 1));

  return (
    <div className="mx-auto max-w-6xl">
      <h1 className="mb-5 flex items-center gap-2 text-xl font-semibold">
        <CalendarDays size={20} className="text-[var(--accent)]" />
        {t('pages.schedule.title')}
      </h1>

      <div className="grid grid-cols-12 gap-4">
        {/* 月历 */}
        <section className="card col-span-12 p-5 lg:col-span-7">
          <header className="mb-4 flex items-center justify-between">
            <span className="text-[15px] font-medium tabular-nums">
              {t('pages.schedule.monthTitle', { year, month: month + 1 })}
            </span>
            <div className="flex gap-1">
              <NavBtn onClick={() => shift(-1)}>
                <ChevronLeft size={15} />
              </NavBtn>
              <button
                onClick={() => setCursor(new Date(today.getFullYear(), today.getMonth(), 1))}
                className="rounded-lg px-2.5 py-1 text-xs text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)]"
              >
                {t('pages.schedule.thisMonth')}
              </button>
              <NavBtn onClick={() => shift(1)}>
                <ChevronRight size={15} />
              </NavBtn>
            </div>
          </header>

          <div className="grid grid-cols-7 gap-1 text-center">
            {WEEK_KEYS.map((key) => (
              <div key={key} className="pb-2 text-[11px] text-[var(--text-muted)]">
                {t(key)}
              </div>
            ))}
            {cells.map((d, i) => {
              if (d === null) return <div key={`e${i}`} />;
              const date = new Date(year, month, d);
              const dateStr = fmt(date);
              const isToday = dateStr === todayStr;
              const isFuture = dateStr > todayStr;
              let lunarText = '';
              try {
                const l = Solar.fromJsDate(date).getLunar();
                lunarText =
                  l.getDayInChinese() === '初一'
                    ? `${l.getMonthInChinese()}月`
                    : l.getDayInChinese();
              } catch {
                lunarText = '';
              }
              const countdownForDay = (countdown.query.data ?? []).find(
                (c) => c.target_date === dateStr,
              );
              return (
                <div
                  key={dateStr}
                  className={`flex aspect-square flex-col items-center justify-center rounded-xl py-1 transition-colors ${
                    isToday
                      ? 'bg-[var(--accent)] font-semibold text-white'
                      : countdownForDay
                        ? 'bg-[var(--accent-weak)]'
                        : isFuture
                          ? 'hover:bg-[var(--hover)]'
                          : 'opacity-50'
                  }`}
                >
                  <span className="text-[14px] leading-none tabular-nums">{d}</span>
                  <span className="mt-0.5 text-[9.5px] leading-none opacity-70">{lunarText}</span>
                  {countdownForDay && (
                    <span className="mt-0.5 text-[9px] leading-none">{countdownForDay.emoji}</span>
                  )}
                </div>
              );
            })}
          </div>
        </section>

        {/* 右列：倒数日管理 + 便签 */}
        <div className="col-span-12 flex flex-col gap-4 lg:col-span-5">
          <CountdownManager countdown={countdown} />
          <NotesBoard notes={notes} />
        </div>
      </div>
    </div>
  );
}

function NavBtn({ onClick, children }: { onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      onClick={onClick}
      className="grid size-7 place-items-center rounded-lg text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--text)]"
    >
      {children}
    </button>
  );
}

function CountdownManager({
  countdown,
}: {
  countdown: ReturnType<typeof useCountdownCustom>;
}) {
  const { t } = useI18n();
  const [title, setTitle] = useState('');
  const [date, setDate] = useState('');
  const items = countdown.query.data ?? [];

  const submit = () => {
    if (!title.trim() || !date) return;
    countdown.create.mutate(
      { title: title.trim(), targetDate: date },
      { onSuccess: () => setTitle('') },
    );
  };

  return (
    <section className="card p-5">
      <header className="mb-3 flex items-center gap-2 text-sm font-medium">
        <Target size={16} className="text-[var(--accent)]" />
        {t('pages.schedule.countdown')}
      </header>

      <div className="mb-3 flex gap-2">
        <input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder={t('pages.schedule.eventPlaceholder')}
          className="h-8 min-w-0 flex-1 rounded-lg border border-[var(--border)] bg-transparent px-2.5 text-xs outline-none focus:border-[var(--accent)]"
        />
        <input
          type="date"
          value={date}
          onChange={(e) => setDate(e.target.value)}
          className="h-8 rounded-lg border border-[var(--border)] bg-transparent px-2 text-xs text-[var(--text)] outline-none focus:border-[var(--accent)]"
        />
        <button
          onClick={submit}
          disabled={!title.trim() || !date || countdown.create.isPending}
          className="grid size-8 shrink-0 place-items-center rounded-lg bg-[var(--accent)] text-white transition-opacity hover:opacity-90 disabled:opacity-40"
        >
          {countdown.create.isPending ? (
            <Loader2 size={13} className="animate-spin" />
          ) : (
            <Plus size={14} />
          )}
        </button>
      </div>

      <div className="space-y-1.5">
        {items.length === 0 && (
          <p className="py-2 text-center text-xs text-[var(--text-muted)]">
            {t('pages.schedule.countdownHint')}
          </p>
        )}
        {items.map((c) => {
          const days = Math.ceil(
            (new Date(c.target_date).getTime() - new Date().setHours(0, 0, 0, 0)) / 86400000,
          );
          return (
            <div
              key={c.id}
              className="group flex items-center justify-between rounded-lg px-2 py-1.5 transition-colors hover:bg-[var(--hover)]"
            >
              <span className="truncate text-[13px]">
                {c.emoji} {c.title}
              </span>
              <span className="flex items-center gap-2">
                <span className="rounded-full bg-sky-500/20 px-2 py-0.5 text-[10.5px] tabular-nums text-sky-300">
                  {days >= 0
                    ? days === 0
                      ? t('pages.schedule.today')
                      : t('pages.schedule.daysLater', { days })
                    : t('pages.schedule.expired')}
                </span>
                <button
                  onClick={() => countdown.remove.mutate(c.id)}
                  className="text-[var(--text-muted)] opacity-0 transition-opacity hover:text-red-400 group-hover:opacity-100"
                >
                  <Trash2 size={12} />
                </button>
              </span>
            </div>
          );
        })}
      </div>
    </section>
  );
}

function NotesBoard({ notes }: { notes: ReturnType<typeof useNotes> }) {
  const { t } = useI18n();
  const [draft, setDraft] = useState('');
  const [editing, setEditing] = useState<number | null>(null);
  const [editDraft, setEditDraft] = useState('');
  const list = notes.query.data ?? [];

  const submit = () => {
    const content = draft.trim();
    if (!content) return;
    notes.create.mutate(content, { onSuccess: () => setDraft('') });
  };

  return (
    <section className="card flex-1 p-5">
      <header className="mb-3 flex items-center justify-between text-sm font-medium">
        <span className="flex items-center gap-2">
          <StickyNote size={16} className="text-[var(--accent)]" />
          {t('pages.schedule.notes')}
        </span>
        <span className="text-[11px] text-[var(--text-muted)]">{t('pages.schedule.noteCount', { count: list.length })}</span>
      </header>

      <div className="mb-3 flex gap-2">
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && submit()}
          placeholder={t('pages.schedule.notePlaceholder')}
          className="h-8 min-w-0 flex-1 rounded-lg border border-[var(--border)] bg-transparent px-2.5 text-xs outline-none focus:border-[var(--accent)]"
        />
        <button
          onClick={submit}
          disabled={!draft.trim() || notes.create.isPending}
          className="grid size-8 shrink-0 place-items-center rounded-lg bg-[var(--accent)] text-white transition-opacity hover:opacity-90 disabled:opacity-40"
        >
          <Plus size={14} />
        </button>
      </div>

      <div className="-mx-1 max-h-64 space-y-1.5 overflow-y-auto px-1">
        {notes.query.isLoading ? (
          <Loader2 className="mx-auto mt-3 animate-spin text-[var(--text-muted)]" size={16} />
        ) : list.length === 0 ? (
          <p className="py-3 text-center text-xs text-[var(--text-muted)]">{t('pages.schedule.notesEmpty')}</p>
        ) : (
          list.map((n) => (
            <div
              key={n.id}
              className="group flex items-start gap-2 rounded-lg border border-[var(--border)] px-2.5 py-2"
              style={n.pinned ? { borderColor: 'var(--accent)' } : undefined}
            >
              {editing === n.id ? (
                <input
                  autoFocus
                  value={editDraft}
                  onChange={(e) => setEditDraft(e.target.value)}
                  onBlur={() => {
                    if (editDraft.trim()) notes.update.mutate({ id: n.id, content: editDraft.trim() });
                    setEditing(null);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
                    if (e.key === 'Escape') setEditing(null);
                  }}
                  className="min-w-0 flex-1 bg-transparent text-[13px] outline-none"
                />
              ) : (
                <span
                  onDoubleClick={() => {
                    setEditing(n.id);
                    setEditDraft(n.content);
                  }}
                  title={t('pages.schedule.doubleClickEdit')}
                  className="min-w-0 flex-1 break-words text-[13px]"
                >
                  {n.content}
                </span>
              )}
              <span className="flex shrink-0 gap-1 opacity-0 transition-opacity group-hover:opacity-100">
                <IconBtn
                  title={n.pinned ? t('pages.schedule.unpin') : t('pages.schedule.pin')}
                  onClick={() => notes.togglePin.mutate({ id: n.id, pinned: !n.pinned })}
                >
                  {n.pinned ? <PinOff size={12} /> : <Pin size={12} />}
                </IconBtn>
                <IconBtn title={t('pages.schedule.delete')} onClick={() => notes.remove.mutate(n.id)}>
                  <Trash2 size={12} />
                </IconBtn>
              </span>
            </div>
          ))
        )}
      </div>
    </section>
  );
}

function IconBtn({
  title,
  onClick,
  children,
}: {
  title: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      title={title}
      onClick={onClick}
      className="grid size-6 place-items-center rounded-md text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--text)]"
    >
      {children}
    </button>
  );
}
