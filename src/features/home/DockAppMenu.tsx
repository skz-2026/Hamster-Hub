/**
 * dock 应用图标右键菜单（纵向）：关闭窗口（仅运行中显示）/ 多开应用 /
 * 分屏添加或移出（仅桌面接管态显示）/ 从任务栏移除（仅定制组显示）。
 * 单击 Dock 图标仍是默认逻辑（打开/激活已有窗口），多开与分屏是右键里的显式动作。
 *
 * 两种渲染位置：
 * - anchored（默认）：主窗口内绝对定位，弹在图标上方（底部指示点上方留 6px）；
 * - fill：dock-menu 独立置顶弹窗内铺满窗口——任务栏窗口只有一条高，纵向
 *   菜单放不下，由 Rust 创建置顶小窗承载（Windows 任务栏自己的右键菜单
 *   也是独立 popup 窗口），窗口几何即菜单位置。
 */
import { CopyPlus, SquareX, Trash2, SplitSquareHorizontal, Ungroup } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { useI18n } from '@/shared/i18n/provider';

export interface DockAppMenuProps {
  /** anchored 模式锚点 x（右键时的 clientX，用于水平定位与视口裁剪） */
  x?: number;
  /** 应用当前是否有可见窗口（决定是否显示「关闭窗口」） */
  running: boolean;
  /** 定制组项才可移除；常用组是自动排名，不提供移除 */
  removable: boolean;
  /** 支持多开才显示「多开应用」；单实例应用点了只会收敛回已有窗口 */
  multiCapable?: boolean;
  /**
   * 显示分屏那一行（桌面接管态 + 应用运行中）。三态由 splitMember / splitFull 决定：
   * 已在分屏里 → 「移出分屏」；已满 4/4 → 禁用的「分屏已满」；否则「分屏添加」。
   */
  splitEnabled?: boolean;
  splitMember?: boolean;
  splitFull?: boolean;
  /** dock-menu 独立弹窗内铺满渲染（默认主窗口内绝对定位） */
  fill?: boolean;
  onNewInstance: () => void;
  onCloseApp: () => void;
  onRemove: () => void;
  onSplitAdd?: () => void;
  onSplitRemove?: () => void;
}

export default function DockAppMenu({
  x = 0,
  running,
  removable,
  multiCapable = true,
  splitEnabled = false,
  splitMember = false,
  splitFull = false,
  fill = false,
  onNewInstance,
  onCloseApp,
  onRemove,
  onSplitAdd,
  onSplitRemove,
}: DockAppMenuProps) {
  const { t } = useI18n();
  // 与命令层 dock.rs 的尺寸常量对齐（MENU_W / ITEM_H / PAD_H）
  const cls = fill
    ? 'flex h-full w-full flex-col overflow-hidden rounded-xl bg-[#232028]/97 py-1 text-[12.5px] text-white/90 ring-1 ring-white/15 shadow-[0_8px_30px_rgba(0,0,0,.5)]'
    : 'absolute bottom-full z-50 mb-1.5 flex w-44 flex-col overflow-hidden rounded-xl bg-[#232028]/97 py-1 text-[12.5px] text-white/90 ring-1 ring-white/15 backdrop-blur-xl';

  return (
    <div
      className={cls}
      style={fill ? undefined : { left: Math.min(Math.max(x - 70, 8), window.innerWidth - 190) }}
      onPointerDown={(e) => e.stopPropagation()}
    >
      {running && (
        <MenuRow icon={SquareX} label={t('chrome.dock.closeWindow')} danger onClick={onCloseApp} />
      )}
      {multiCapable && (
        <MenuRow icon={CopyPlus} label={t('chrome.dock.newInstance')} onClick={onNewInstance} />
      )}
      {/* 分屏：进得来（添加）也出得去（移出）；满 4 扇时给一行禁用态说明原因。
          顺序在「多开」之后（都是「再来一扇窗」的语义），移除留到最后 */}
      {splitEnabled &&
        (splitMember ? (
          <MenuRow icon={Ungroup} label={t('chrome.dock.splitRemove')} onClick={onSplitRemove!} />
        ) : splitFull ? (
          <MenuRow
            icon={SplitSquareHorizontal}
            label={t('chrome.dock.splitFull')}
            disabled
            onClick={() => {}}
          />
        ) : (
          <MenuRow
            icon={SplitSquareHorizontal}
            label={t('chrome.dock.splitAdd')}
            onClick={onSplitAdd!}
          />
        ))}
      {/* 破坏性动作（移除定制项）与上方动作组之间加分隔，避免误点 */}
      {removable && (running || multiCapable || splitEnabled) && (
        <span aria-hidden className="my-0.5 h-px bg-white/12" />
      )}
      {removable && (
        <MenuRow icon={Trash2} label={t('chrome.action.remove')} danger onClick={onRemove} />
      )}
    </div>
  );
}

/** 菜单行：图标 + 文案，危险动作（关闭/移除）红色系；disabled 行只作说明 */
function MenuRow({
  icon: Icon,
  label,
  danger,
  disabled,
  onClick,
}: {
  icon: LucideIcon;
  label: string;
  danger?: boolean;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`flex w-full items-center gap-2.5 px-3.5 py-[7px] text-left transition-colors ${
        disabled
          ? 'cursor-not-allowed text-white/40'
          : danger
            ? 'text-red-300 hover:bg-red-500/20'
            : 'text-white/90 hover:bg-white/12'
      }`}
    >
      <Icon size={14} strokeWidth={2} className="shrink-0 opacity-90" />
      {label}
    </button>
  );
}
