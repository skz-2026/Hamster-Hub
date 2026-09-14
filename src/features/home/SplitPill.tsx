/**
 * 任务栏「分屏 n/4」状态胶囊（仅桌面接管态渲染）：左半是状态 + 管理入口，
 * 右半 ✕ 一键退出（全部窗口还原回加入分屏前的位置）。没开分屏时不渲染。
 *
 * 「随时退出」的两个入口：这里的 ✕（点一下就退）与管理弹层里的「退出分屏」
 * （带文字说明，容错）。加窗入口统一在 dock 图标右键菜单里（分屏添加）。
 */
import { SplitSquareHorizontal, X } from 'lucide-react';
import { useI18n } from '@/shared/i18n/provider';
import type { SplitState } from '@/shared/lib/ipc';

export interface SplitPillProps {
  state: SplitState | undefined;
  /** 点胶囊主体：打开管理弹层（传胶囊中心 x，任务栏窗内逻辑 px） */
  onManage: (x: number) => void;
  /** 点 ✕：退出分屏 */
  onExit: () => void;
}

export default function SplitPill({ state, onManage, onExit }: SplitPillProps) {
  const { t } = useI18n();
  if (!state?.active) return null;
  const n = state.members.length;

  return (
    <div className="flex items-center self-end rounded-xl bg-white/[0.16] text-white/95 ring-1 ring-white/22 backdrop-blur-xl drop-shadow-[0_4px_10px_rgba(0,0,0,.35)]">
      <button
        onClick={(e) => {
          const r = e.currentTarget.getBoundingClientRect();
          onManage((r.left + r.right) / 2);
        }}
        title={t('split.manageTip')}
        className="flex items-center gap-1.5 rounded-l-xl py-1.5 pl-2.5 pr-2 transition-colors hover:bg-white/15"
      >
        <SplitSquareHorizontal size={13} strokeWidth={2} />
        <span className="text-[12px] tabular-nums">{t('split.pill', { n, m: state.capacity })}</span>
      </button>
      <span aria-hidden className="h-4 w-px bg-white/20" />
      <button
        onClick={onExit}
        title={t('split.exitTip')}
        aria-label={t('split.exit')}
        className="rounded-r-xl py-1.5 pl-1.5 pr-2 text-white/70 transition-colors hover:bg-red-500/40 hover:text-white"
      >
        <X size={12} strokeWidth={2.6} />
      </button>
    </div>
  );
}
