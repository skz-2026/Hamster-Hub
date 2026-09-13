import { useState } from 'react';
import { commands } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';
import { NotificationBell } from '@/features/notifications/NotificationBell';
import { NotificationCenter } from '@/features/notifications/NotificationCenter';

interface TitleBarProps {
  coreVersion: string | null;
  onMinimize: () => void;
  onClose: () => void; // 关闭 = 隐藏到托盘
}

/** 自定义标题栏（macOS 红绿灯风）：左侧窗口控制 + 应用标识，整条为拖拽区 */
export function TitleBar({ coreVersion, onMinimize, onClose }: TitleBarProps) {
  const { t } = useI18n();
  const [pinned, setPinned] = useState(false);
  const [ncOpen, setNcOpen] = useState(false);

  const togglePin = async () => {
    const next = !pinned;
    setPinned(next);
    try {
      await commands.windowSetPinned(next);
    } catch {
      setPinned(!next);
    }
  };

  return (
    <header
      data-tauri-drag-region
      className="relative flex h-10 shrink-0 select-none items-center pl-3 pr-4"
    >
      {/* macOS 红绿灯：隐藏到托盘 / 最小化 / 置顶 */}
      <div className="flex items-center gap-2">
        <TrafficLight color="#ff5f57" title={t('chrome.titlebar.hideToTray')} onClick={onClose}>
          ✕
        </TrafficLight>
        <TrafficLight color="#febc2e" title={t('chrome.titlebar.minimize')} onClick={onMinimize}>
          −
        </TrafficLight>
        <TrafficLight
          color="#28c840"
          title={pinned ? t('chrome.titlebar.unpin') : t('chrome.titlebar.pin')}
          onClick={togglePin}
          lit={pinned}
        >
          ⬆
        </TrafficLight>
      </div>

      {/* 窗口标题绝对居中（macOS 惯例） */}
      <div className="pointer-events-none absolute inset-x-0 flex items-center justify-center gap-2">
        <span className="text-[12.5px] font-medium tracking-wide text-[var(--text)]/85">
          {t('chrome.app.name')}
        </span>
        {coreVersion && (
          <span className="rounded-full border border-[var(--border)] px-2 py-px text-[10px] text-[var(--text-muted)]">
            {t('chrome.titlebar.coreVersion', { v: coreVersion })}
          </span>
        )}
      </div>

      {/* 通知中心（窗口化形态的入口：桌面接管态由桌面主页热区承载） */}
      <div className="relative z-10 ml-auto flex items-center">
        <NotificationBell
          variant="chrome"
          open={ncOpen}
          onToggle={() => setNcOpen((v) => !v)}
        />
      </div>
      <NotificationCenter open={ncOpen} onClose={() => setNcOpen(false)} />
    </header>
  );
}

/** macOS 红绿灯窗口钮：悬停浮现功能符号；lit 态符号常亮（如已置顶） */
function TrafficLight({
  color,
  title,
  onClick,
  lit,
  children,
}: {
  color: string;
  title: string;
  onClick: () => void;
  lit?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      title={title}
      aria-label={title}
      aria-pressed={lit}
      onClick={onClick}
      className="group/light grid size-3 place-items-center rounded-full ring-1 ring-black/25 transition-transform hover:scale-110"
      style={{ background: color }}
    >
      <span
        className={`scale-95 text-[8px] font-bold leading-none text-black/60 transition-opacity ${
          lit ? 'opacity-100' : 'opacity-0 group-hover/light:opacity-100'
        }`}
      >
        {children}
      </span>
    </button>
  );
}
