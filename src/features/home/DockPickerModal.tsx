import { useMemo, useState } from 'react';
import { Loader2, Search, X } from 'lucide-react';
import { useApps, useHomeLayout } from './hooks';
import { AppIcon } from './AppIcon';
import { DOCK_CAPACITY } from './layout';
import { useI18n } from '@/shared/i18n/provider';

/**
 * 「添加到 Dock」选择器（主窗口居中弹层；任务栏窗口太小放不下弹层，
 * 由任务栏「+」经事件唤起主窗口）。受 DOCK_CAPACITY 上限约束。
 * 候选按常用度排序（常用在前），避免百余个字母序应用里难以找到目标。
 */
export function DockPickerModal({ onClose }: { onClose: () => void }) {
  const { t } = useI18n();
  const { data: apps = [], isLoading } = useApps();
  const { layout, usageRank, commit } = useHomeLayout(apps);
  const [q, setQ] = useState('');

  const inDock = useMemo(() => new Set(layout.dock), [layout.dock]);
  const full = layout.dock.length >= DOCK_CAPACITY;
  const query = q.trim().toLowerCase();
  const rankIdx = useMemo(() => new Map(usageRank.map((k, i) => [k, i])), [usageRank]);
  const candidates = apps
    .filter(
      (a) =>
        !inDock.has(a.app_key) &&
        (!query || a.display_name.toLowerCase().includes(query)),
    )
    // 常用在前，没用过的保持字母序垫底（稳定排序）
    .sort((x, y) => (rankIdx.get(x.app_key) ?? Infinity) - (rankIdx.get(y.app_key) ?? Infinity));

  const add = (key: string) => {
    if (full) return;
    commit({ ...layout, dock: [...layout.dock, key] });
    onClose();
  };

  return (
    <div
      className="absolute inset-0 z-50 grid place-items-center bg-black/45 backdrop-blur-xl"
      onClick={onClose}
    >
      <div
        className="flex h-[min(560px,78vh)] w-[min(720px,84vw)] flex-col gap-4 rounded-[28px] bg-[#1c1a22]/95 p-6 text-white ring-1 ring-white/14 backdrop-blur-2xl"
        style={{ boxShadow: '0 32px 80px rgba(0,0,0,.5)' }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-3">
          <h2 className="text-[15px] font-semibold">{t('chrome.action.addToDock')}</h2>
          <span className="text-[11.5px] text-white/45 tabular-nums">
            {layout.dock.length}/{DOCK_CAPACITY}
          </span>
          <div className="relative ml-auto w-[240px]">
            <Search size={13} className="absolute left-3 top-1/2 -translate-y-1/2 text-white/45" />
            <input
              autoFocus
              value={q}
              onChange={(e) => setQ(e.target.value)}
              placeholder={t('chrome.picker.searchApps')}
              className="h-8 w-full rounded-full bg-white/10 pl-8 pr-3 text-[12.5px] text-white outline-none ring-1 ring-white/12 placeholder:text-white/40 focus:ring-white/30"
            />
          </div>
          <button
            onClick={onClose}
            className="grid size-8 place-items-center rounded-full bg-white/10 text-white/70 transition-colors hover:bg-white/20 hover:text-white"
            aria-label={t('chrome.action.close')}
          >
            <X size={14} />
          </button>
        </div>

        {isLoading && apps.length === 0 ? (
          <div className="grid flex-1 place-items-center text-white/60">
            <Loader2 className="animate-spin" size={20} />
          </div>
        ) : full ? (
          <p className="grid flex-1 place-items-center text-[13px] text-white/50">
            {t('chrome.picker.dockFull', { n: DOCK_CAPACITY })}
          </p>
        ) : candidates.length === 0 ? (
          <p className="grid flex-1 place-items-center text-[13px] text-white/50">
            {query ? t('chrome.picker.noMatch', { q }) : t('chrome.picker.noApps')}
          </p>
        ) : (
          <div className="grid flex-1 auto-rows-min grid-cols-7 content-start justify-items-center gap-y-5 overflow-y-auto px-1">
            {candidates.map((a) => (
              <button
                key={a.app_key}
                onClick={() => add(a.app_key)}
                title={t('chrome.picker.addTitle', { name: a.display_name })}
                className="rounded-2xl p-1 transition-transform hover:scale-105"
              >
                <AppIcon
                  name={a.display_name}
                  iconPath={a.icon_path}
                  size={52}
                  onPointerDown={undefined}
                />
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
