/**
 * 托盘按钮：dock 右端，点击打开 **Windows 原生托盘溢出弹层**
 * （系统自带 UI，含全部后台托盘 app，可直接交互）。
 *
 * Rust 侧流程：短暂显示系统任务栏 → UIA Invoke「显示隐藏的图标」→
 * 原生弹层出现 → 任务栏重新隐藏（弹层为独立窗口，保留在屏幕上）。
 */
import { ChevronUp } from 'lucide-react';
import { useMutation } from '@tanstack/react-query';
import { commands } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';

export default function TrayButton() {
  const { t } = useI18n();
  const open = useMutation({
    mutationFn: () => commands.trayOpenOverflow(),
  });

  return (
    <button
      onClick={(e) => {
        e.stopPropagation();
        open.mutate();
      }}
      title={t('chrome.tray.trayAndApps')}
      aria-label={t('chrome.tray.trayAndApps')}
      className="group grid size-[44px] place-items-center rounded-xl bg-white/[0.08] text-white/55 ring-1 ring-white/12 transition-all hover:bg-white/16 hover:text-white/90"
    >
      {/* ^ 形：对齐 Windows 托盘「显示隐藏的图标」的原生视觉隐喻 */}
      <ChevronUp size={22} strokeWidth={2.2} />
    </button>
  );
}
