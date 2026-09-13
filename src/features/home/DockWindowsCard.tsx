/**
 * dock 悬停窗口卡片：多开应用（≥2 扇可见窗口）悬停图标时列出全部窗口，
 * 点一行把那扇窗口前置（最小化的会还原）。z 序最上层的排最前。
 *
 * 两种渲染位置（与 DockAppMenu 同构）：
 * - anchored（默认）：主窗口内绝对定位，弹在图标上方；
 * - fill：dock-menu 独立置顶弹窗内铺满（任务栏窗口一条高放不下，
 *   由 Rust 创建置顶小窗承载），窗口几何即卡片位置。
 */
import { Minus } from 'lucide-react';
import type { AppWindowInfo } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';

export interface DockWindowsCardProps {
  windows: AppWindowInfo[];
  /** 点击行：前置该窗口（最小化自动还原） */
  onActivate: (id: number) => void;
  /** anchored 模式锚点 x（图标中心，用于水平定位与视口裁剪） */
  x?: number;
  /** dock-menu 独立弹窗内铺满渲染（默认主窗口内绝对定位） */
  fill?: boolean;
  /** 本地形态：鼠标进卡保持（onMouseLeave 由调用方收卡） */
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
}

/** 与命令层 dock.rs 的 HOVER_W 对齐（逻辑 px） */
const CARD_W = 280;

export default function DockWindowsCard({
  windows,
  onActivate,
  x = 0,
  fill = false,
  onMouseEnter,
  onMouseLeave,
}: DockWindowsCardProps) {
  const { t } = useI18n();
  const cls = fill
    ? 'flex h-full w-full flex-col overflow-y-auto rounded-xl bg-[#232028]/97 py-1 ring-1 ring-white/15 shadow-[0_8px_30px_rgba(0,0,0,.5)]'
    : 'absolute bottom-full z-50 mb-1.5 flex w-auto flex-col overflow-y-auto rounded-xl bg-[#232028]/97 py-1 ring-1 ring-white/15 backdrop-blur-xl';

  return (
    <div
      role="list"
      aria-label={t('chrome.dock.pickWindow')}
      className={cls}
      style={
        fill
          ? undefined
          : { left: Math.min(Math.max(x - CARD_W / 2, 8), window.innerWidth - CARD_W - 8) }
      }
      onPointerDown={(e) => e.stopPropagation()}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      {windows.map((w) => (
        <button
          key={w.id}
          role="listitem"
          onClick={() => onActivate(w.id)}
          title={w.title}
          className="flex w-full items-center gap-2.5 px-3 py-[10px] text-left transition-colors hover:bg-white/12"
        >
          {w.png ? (
            <img src={w.png} alt="" draggable={false} className="size-5 shrink-0 object-contain" />
          ) : (
            <span className="size-5 shrink-0 rounded-[5px] bg-white/12 ring-1 ring-white/15" />
          )}
          <span
            className={`min-w-0 flex-1 truncate text-[12.5px] ${
              w.minimized ? 'text-white/55' : 'text-white/90'
            }`}
          >
            {w.title}
          </span>
          {w.minimized && (
            <Minus size={13} className="shrink-0 text-white/45" aria-label={t('chrome.dock.minimized')} />
          )}
        </button>
      ))}
    </div>
  );
}
