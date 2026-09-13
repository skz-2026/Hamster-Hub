import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  AlarmClock,
  Bell,
  CheckCheck,
  Download,
  Sparkles,
  Timer,
  Trash2,
  X,
} from 'lucide-react';
import { useI18n } from '@/shared/i18n/provider';
import { groupByDay, timeAgo, unreadCount, type NotificationItem, type TimeAgo } from './notifications';
import {
  clearNotifications,
  dismissNotification,
  readAllNotifications,
  readNotification,
  useNotifications,
} from './store';

/**
 * 通知中心（iOS 风，顶部右侧滑出的玻璃面板）：
 * 按 今天/昨天/更早 分组回看通知——待办到点、番茄钟完成、AI Agent 会话结束、更新就绪。
 * 点击条目跳到对应页面（并标记已读），行内 ✕ 单条移除，头部可全部已读 / 清空。
 */
export function NotificationCenter({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const items = useNotifications();
  const [now, setNow] = useState(() => Date.now());
  const panelRef = useRef<HTMLElement>(null);

  // 打开时对齐一次相对时间，之后每 30s 走一次（只有面板开着才需要「刚刚 → 5 分钟前」）
  useEffect(() => {
    if (!open) return;
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(timer);
  }, [open]);

  // 点击面板外关闭
  useEffect(() => {
    if (!open) return;
    const onDown = (e: PointerEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) onClose();
    };
    window.addEventListener('pointerdown', onDown);
    return () => window.removeEventListener('pointerdown', onDown);
  }, [open, onClose]);

  // Esc 关闭（与控制中心同理；面板非模态、不劫持输入焦点）
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, onClose]);

  if (!open) return null;

  const unread = unreadCount(items);
  const groups = groupByDay(items, now);

  const openItem = (item: NotificationItem) => {
    readNotification(item.id);
    if (!item.route) return;
    onClose();
    navigate(item.route);
  };

  return (
    <aside
      ref={panelRef}
      data-testid="notification-center"
      aria-label={t('notifications.title')}
      className="rise-in absolute right-5 top-[68px] z-50 flex max-h-[min(560px,72vh)] w-[336px] flex-col rounded-[28px] bg-black/38 p-3 text-white ring-1 ring-white/15 backdrop-blur-2xl"
      style={{ boxShadow: '0 24px 64px rgba(0,0,0,.45), inset 0 1px 0 rgba(255,255,255,.12)' }}
    >
      <header className="flex items-center gap-2 px-1.5 pb-2 pt-1">
        <Bell size={14} className="text-white/70" />
        <span className="text-[12.5px] font-semibold">{t('notifications.title')}</span>
        {unread > 0 && (
          <span
            data-testid="notification-unread"
            className="rounded-full bg-[var(--accent)]/85 px-1.5 py-px text-[10px] font-medium text-white"
          >
            {t('notifications.unread', { n: unread })}
          </span>
        )}
        <div className="ml-auto flex items-center gap-0.5">
          {items.length > 0 && (
            <button
              onClick={readAllNotifications}
              title={t('notifications.markAllRead')}
              aria-label={t('notifications.markAllRead')}
              className="grid size-7 place-items-center rounded-full text-white/60 transition-colors hover:bg-white/12 hover:text-white"
            >
              <CheckCheck size={14} />
            </button>
          )}
          {items.length > 0 && (
            <button
              onClick={clearNotifications}
              title={t('notifications.clear')}
              aria-label={t('notifications.clear')}
              className="grid size-7 place-items-center rounded-full text-white/60 transition-colors hover:bg-white/12 hover:text-white"
            >
              <Trash2 size={14} />
            </button>
          )}
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto pr-0.5">
        {items.length === 0 ? (
          <div className="flex flex-col items-center gap-1.5 px-6 py-9 text-center">
            <Bell size={20} className="text-white/30" />
            <p className="text-[12.5px] text-white/60">{t('notifications.empty.title')}</p>
            <p className="text-[11px] leading-snug text-white/40">{t('notifications.empty.desc')}</p>
          </div>
        ) : (
          groups.map((group) => (
            <section key={group.bucket} data-testid={`notification-group-${group.bucket}`}>
              <p className="px-2 pb-1.5 pt-2 text-[11px] font-medium text-white/45">
                {t(DAY_LABELS[group.bucket])}
              </p>
              <div className="flex flex-col gap-1.5">
                {group.items.map((item) => (
                  <Row key={item.id} item={item} now={now} onOpen={openItem} />
                ))}
              </div>
            </section>
          ))
        )}
      </div>
    </aside>
  );
}

/** 条目行：kind 决定图标与配色；未读点 + 相对时间 + 行内移除 */
function Row({
  item,
  now,
  onOpen,
}: {
  item: NotificationItem;
  now: number;
  onOpen: (item: NotificationItem) => void;
}) {
  const { t } = useI18n();
  const { titleKey, Icon, tint } = KIND_META[item.kind];
  const clickable = Boolean(item.route);

  return (
    <div
      data-testid="notification-item"
      data-kind={item.kind}
      data-read={item.read}
      role={clickable ? 'button' : undefined}
      tabIndex={clickable ? 0 : undefined}
      onClick={() => onOpen(item)}
      onKeyDown={(e) => {
        if (clickable && (e.key === 'Enter' || e.key === ' ')) {
          e.preventDefault();
          onOpen(item);
        }
      }}
      className={`group/row relative flex items-start gap-2.5 rounded-[20px] bg-white/8 p-3 pr-9 text-left ring-1 ring-white/10 transition-colors ${
        clickable ? 'cursor-pointer hover:bg-white/14' : 'cursor-default'
      }`}
    >
      <span
        className="grid size-8 shrink-0 place-items-center rounded-full"
        style={{ background: `${tint}2e`, color: tint }}
      >
        <Icon size={15} />
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex items-baseline gap-1.5">
          {!item.read && (
            <span
              data-testid="notification-dot"
              className="mt-1 size-1.5 shrink-0 self-center rounded-full bg-[var(--accent)]"
            />
          )}
          <span className="truncate text-[12.5px] font-medium">{t(titleKey)}</span>
          <span className="ml-auto shrink-0 text-[10.5px] tabular-nums text-white/45">
            {agoLabel(timeAgo(item.at, now), t)}
          </span>
        </div>
        <p className="mt-0.5 break-words text-[11.5px] leading-snug text-white/65">
          {bodyOf(item, t)}
        </p>
      </div>
      <button
        onClick={(e) => {
          e.stopPropagation();
          dismissNotification(item.id);
        }}
        title={t('notifications.dismiss')}
        aria-label={t('notifications.dismiss')}
        className="absolute right-2 top-2 grid size-6 place-items-center rounded-full text-white/40 transition-all hover:bg-white/12 hover:text-white md:opacity-0 md:group-hover/row:opacity-100"
      >
        <X size={12} />
      </button>
    </div>
  );
}

/** 类别 → 图标/配色/标题 key（显式映射，禁止动态拼 key） */
const KIND_META = {
  todo: { titleKey: 'notifications.kind.todo', Icon: AlarmClock, tint: '#ff9f0a' },
  focus: { titleKey: 'notifications.kind.focus', Icon: Timer, tint: '#30d158' },
  agent: { titleKey: 'notifications.kind.agent', Icon: Sparkles, tint: '#bf5af2' },
  update: { titleKey: 'notifications.kind.update', Icon: Download, tint: '#0a84ff' },
} as const;

const DAY_LABELS = {
  today: 'notifications.day.today',
  yesterday: 'notifications.day.yesterday',
  earlier: 'notifications.day.earlier',
} as const;

type TFn = ReturnType<typeof useI18n>['t'];

/** 正文：待办显示用户内容本身；番茄钟按机器值（focus/break）映射；其余按类别固定文案 */
function bodyOf(item: NotificationItem, t: TFn): string {
  switch (item.kind) {
    case 'todo':
      return item.text || t('notifications.todo.body');
    case 'focus':
      return item.text === 'break' ? t('notifications.break.body') : t('notifications.focus.body');
    case 'agent':
      return item.text || t('notifications.agent.body');
    case 'update':
      return t('notifications.update.body');
  }
}

function agoLabel(ago: TimeAgo, t: TFn): string {
  switch (ago.unit) {
    case 'now':
      return t('notifications.ago.now');
    case 'min':
      return t('notifications.ago.min', { n: ago.n });
    case 'hour':
      return t('notifications.ago.hour', { n: ago.n });
    case 'day':
      return t('notifications.ago.day', { n: ago.n });
  }
}
