/**
 * 分屏管理弹层（任务栏「分屏 n/4」胶囊点开）：成员清单 + 逐行移出/关闭 +
 * 底部「退出分屏」。
 *
 * 两种宿主（与 DockAppMenu/DockWindowsCard 同构）：
 * - 任务栏独立置顶弹窗（/dock-menu，kind=split）：高度由 Rust 按成员数算，
 *   成员数一变就 splitMenuOpen 重开一次同步高度（见下方 effect）；
 * - 浏览器预览 / 主窗口内：本地绝对定位浮层，高度自适应，不需要同步。
 *
 * 两个动作的语义差别要分清：**移出**=退出分屏、窗口留在原处（回到加入前的位置）；
 * **关闭**=投递 WM_CLOSE 把窗口关掉，会话自愈剔除它、剩下的补位。
 */
import { useEffect, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Ungroup, X } from 'lucide-react';
import { commands, isTauri } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';
import { useSplitState, useSplitActions } from './hooks';

export interface SplitPanelProps {
  /** 弹窗内的锚点 x（重开同步高度用；本地浮层形态不需要） */
  anchorX?: number;
  /** 独立置顶弹窗形态（高度由 Rust 定） */
  inPopup?: boolean;
  /** 打开时的成员数（避免首次拉到数据时白重开一次） */
  initialCount?: number;
  /** 收回自己（本地浮层形态）；弹窗形态走 dockMenuHide */
  onDismiss?: () => void;
}

/** 与命令层 dock.rs 的 SPLIT_W 对齐（逻辑 px） */
const PANEL_W = 280;

export default function SplitPanel({
  anchorX = 0,
  inPopup = false,
  initialCount = 0,
  onDismiss,
}: SplitPanelProps) {
  const { t } = useI18n();
  const qc = useQueryClient();
  const { data } = useSplitState(true);
  const { remove, exit } = useSplitActions();
  const members = data?.members ?? [];
  const capacity = data?.capacity ?? 4;

  const dismiss = () => {
    if (inPopup || isTauri) commands.dockMenuHide().catch(console.error);
    onDismiss?.();
  };

  // 弹窗高度 = 标题 + 成员行数 × 行高 + 退出按钮（Rust 按 split_count 算）。
  // 成员数一变（用户关掉分屏内的窗口、外部轮询发现少了）就重开一次同步高度，
  // 免得留出一行空白。首次拉数据（0 → n）跳过，打开时 Rust 已经按 n 算过了。
  const lastCount = useRef(initialCount);
  useEffect(() => {
    if (data === undefined) return;
    // 会话结束（整体退出 / 最后一个成员被移出）：浮层自己也该收起来
    if (!data.active) {
      dismiss();
      return;
    }
    if (!inPopup) return; // 本地浮层高度自适应，不用同步
    const n = data.members.length;
    if (n === lastCount.current) return;
    lastCount.current = n;
    commands.splitMenuOpen(anchorX, n).catch(console.error);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data, inPopup, anchorX]);

  /** 逐行关闭：WM_CLOSE 是异步的（应用可能弹保存确认），稍候再看会话少了谁 */
  const closeWindow = (windowId: number) => {
    commands.appWindowClose(windowId).catch(console.error);
    window.setTimeout(() => {
      commands
        .splitState()
        .then((s) => qc.setQueryData(['split', 'state'], s))
        .catch(console.error);
    }, 450);
  };

  const cls = inPopup
    ? 'flex h-full w-full flex-col overflow-hidden rounded-xl bg-[#232028]/97 py-1 text-white/90 ring-1 ring-white/15 shadow-[0_8px_30px_rgba(0,0,0,.5)]'
    : 'absolute bottom-full right-2 z-50 mb-1.5 flex flex-col overflow-hidden rounded-xl bg-[#232028]/97 py-1 text-white/90 ring-1 ring-white/15 backdrop-blur-xl';

  return (
    <div
      role="dialog"
      aria-label={t('split.panelTitle', { n: members.length, m: capacity })}
      className={cls}
      style={inPopup ? undefined : { width: PANEL_W }}
      onPointerDown={(e) => e.stopPropagation()}
    >
      {/* 标题行：h-[30px] 与 dock.rs 的 SPLIT_HEAD_H 对齐 */}
      <div className="flex h-[30px] shrink-0 items-center gap-2 px-3.5 text-[12.5px] font-medium">
        {t('split.panelTitle', { n: members.length, m: capacity })}
      </div>
      {/* 成员行：h-[34px] 与 SPLIT_ITEM_H 对齐（空态共用这一行） */}
      {members.length === 0 ? (
        <div className="flex h-[34px] shrink-0 flex-col justify-center px-3.5 leading-tight">
          <span className="text-[12px] text-white/85">{t('split.empty')}</span>
          <span className="text-[10.5px] text-white/50">{t('split.hint')}</span>
        </div>
      ) : (
        members.map((m) => (
          <div
            key={m.windowId}
            className="group/row flex h-[34px] shrink-0 items-center gap-2.5 px-3.5 transition-colors hover:bg-white/10"
          >
            {m.png ? (
              <img src={m.png} alt="" draggable={false} className="size-5 shrink-0 object-contain" />
            ) : (
              <span className="size-5 shrink-0 rounded-[5px] bg-white/12 ring-1 ring-white/15" />
            )}
            <span className="min-w-0 flex-1 truncate text-[12.5px]" title={m.title || m.process}>
              {m.title || m.process}
            </span>
            <button
              onClick={() => remove.mutate(m.windowId)}
              title={t('chrome.dock.splitRemove')}
              aria-label={`${t('chrome.dock.splitRemove')}: ${m.title || m.process}`}
              className="shrink-0 rounded-md p-1 text-white/35 transition-colors group-hover/row:text-white/70 hover:bg-white/15 hover:!text-white/95"
            >
              <Ungroup size={13} strokeWidth={2.2} />
            </button>
            <button
              onClick={() => closeWindow(m.windowId)}
              title={t('split.closeTip')}
              aria-label={`${t('split.close')}: ${m.title || m.process}`}
              className="shrink-0 rounded-md p-1 text-white/35 transition-colors group-hover/row:text-white/70 hover:bg-red-500/25 hover:!text-red-200"
            >
              <X size={13} strokeWidth={2.4} />
            </button>
          </div>
        ))
      )}
      {/* 底部：h-[44px] 与 SPLIT_FOOT_H 对齐 */}
      <div className="mt-auto flex h-[44px] shrink-0 items-center px-2.5">
        <button
          onClick={() => exit.mutate()}
          title={t('split.exitTip')}
          className="w-full rounded-lg bg-white/12 py-1.5 text-[12.5px] text-white/90 transition-colors hover:bg-red-500/30 hover:text-white"
        >
          {t('split.exit')}
        </button>
      </div>
    </div>
  );
}
