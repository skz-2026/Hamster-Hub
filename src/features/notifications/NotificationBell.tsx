import { Bell } from 'lucide-react';
import { useI18n } from '@/shared/i18n/provider';
import { useUnreadCount } from './store';

/**
 * 通知中心触发钮：铃铛 + 未读角标。
 * variant 决定皮肤——`glass` 用于壁纸之上的桌面主页热区（与控制中心钮同一视觉语言），
 * `chrome` 用于窗口化标题栏（走主题 token，浅色主题下不能靠透明白底）。
 */
export function NotificationBell({
  open,
  onToggle,
  variant = 'glass',
}: {
  open: boolean;
  onToggle: () => void;
  variant?: 'glass' | 'chrome';
}) {
  const { t } = useI18n();
  const unread = useUnreadCount();
  const label = t('notifications.title');

  const skin =
    variant === 'glass'
      ? `size-10 bg-white/10 text-white/85 ring-1 ring-white/15 backdrop-blur-xl hover:bg-white/20 hover:text-white ${
          open ? 'bg-white/20 text-white' : ''
        }`
      : `size-7 bg-[var(--panel-strong)] text-[var(--text-muted)] ring-1 ring-[var(--border)] hover:bg-[var(--hover-strong)] hover:text-[var(--text)] ${
          open ? 'bg-[var(--hover-strong)] text-[var(--text)]' : ''
        }`;

  return (
    <button
      onClick={onToggle}
      data-testid="notification-bell"
      title={label}
      aria-label={label}
      aria-expanded={open}
      className={`relative grid shrink-0 place-items-center rounded-full transition-all ${skin}`}
    >
      <Bell size={variant === 'glass' ? 17 : 14} />
      {unread > 0 && (
        <span
          data-testid="notification-badge"
          className="absolute -right-0.5 -top-0.5 grid h-[17px] min-w-[17px] place-items-center rounded-full bg-[var(--accent)] px-1 text-[10px] font-semibold leading-none text-white ring-2 ring-black/20"
        >
          {unread > 9 ? '9+' : unread}
        </span>
      )}
    </button>
  );
}
