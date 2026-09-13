/**
 * dock 悬停窗口卡片：应用悬停图标时列出其全部窗口，点一行把那扇窗口
 * 前置（最小化的会还原）；行尾 X 直接关闭那扇窗口（多开应用可逐窗关闭，
 * 单窗口应用即快捷退出）。z 序最上层的排最前。
 *
 * 两种渲染位置（与 DockAppMenu 同构）：
 * - anchored（默认）：主窗口内绝对定位，弹在图标上方；
 * - fill：dock-menu 独立置顶弹窗内铺满（任务栏窗口一条高放不下，
 *   由 Rust 创建置顶小窗承载），窗口几何即卡片位置。
 */
import { Minus, X } from 'lucide-react';
import type { AppWindowInfo } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';

export interface DockWindowsCardProps {
  windows: AppWindowInfo[];
  /** 点击行：前置该窗口（最小化自动还原） */
  onActivate: (id: number) => void;
  /** 行尾 X：关闭该窗口（调用方负责刷新卡片，空了收卡） */
  onCloseWindow: (id: number) => void;
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
  onCloseWindow,
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
        // 行整体可点（前置），行内 X 是嵌套按钮——button 不能嵌 button，
        // 外层用 div 承载点击语义
        <div
          key={w.id}
          role="button"
          tabIndex={0}
          onClick={() => onActivate(w.id)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') onActivate(w.id);
          }}
          title={w.title}
          className="group/row flex w-full cursor-pointer items-center gap-2.5 px-3 py-[9px] text-left transition-colors hover:bg-white/12 focus-visible:bg-white/12 focus-visible:outline-none"
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
          <button
            onClick={(e) => {
              e.stopPropagation();
              onCloseWindow(w.id);
            }}
            title={t('chrome.dock.closeWindow')}
            aria-label={`${t('chrome.dock.closeWindow')}: ${w.title}`}
            className="shrink-0 rounded-md p-1 text-white/35 transition-colors group-hover/row:text-white/70 hover:bg-red-500/25 hover:!text-red-200"
          >
            <X size={13} strokeWidth={2.4} />
          </button>
        </div>
      ))}
    </div>
  );
}
