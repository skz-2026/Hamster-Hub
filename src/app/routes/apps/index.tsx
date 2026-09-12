import { useMemo, useState } from 'react';
import { LayoutGrid, Loader2 } from 'lucide-react';
import { commands } from '@/shared/lib/ipc';
import { useApps } from '@/features/home/hooks';
import { AppIcon, Monogram } from '@/features/home/AppIcon';
import { convertFileSrc } from '@tauri-apps/api/core';
import { CATEGORIES, withCategories, type AppCategory } from '@/features/apps/category';

const CAT_COLOR: Record<AppCategory, string> = {
  沟通: 'bg-sky-500/25 text-sky-300',
  办公: 'bg-blue-500/25 text-blue-300',
  开发: 'bg-teal-500/25 text-teal-300',
  娱乐: 'bg-purple-500/25 text-purple-300',
  工具: 'bg-amber-500/25 text-amber-300',
  系统: 'bg-slate-500/25 text-slate-300',
  其他: 'bg-neutral-500/25 text-neutral-300',
};

/** 应用页：分类药丸 + 真实图标宫格（桌面整理核心，对标 CapyHub 应用页） */
export default function AppsPage() {
  const { data: apps = [], isLoading } = useApps();
  const [cat, setCat] = useState<'全部' | AppCategory>('全部');

  const categorized = useMemo(() => withCategories(apps), [apps]);
  const shown = useMemo(
    () => (cat === '全部' ? categorized : categorized.filter((a) => a.category === cat)),
    [categorized, cat],
  );

  const countOf = (c: '全部' | AppCategory) =>
    c === '全部' ? categorized.length : categorized.filter((a) => a.category === c).length;

  return (
    <div className="mx-auto max-w-6xl">
      <div className="mb-5 flex items-center justify-between">
        <h1 className="flex items-center gap-2 text-xl font-semibold">
          <LayoutGrid size={20} className="text-[var(--accent)]" />
          应用
        </h1>
        <span className="text-xs text-[var(--text-muted)]">
          {isLoading ? '索引中…' : `共 ${categorized.length} 个 · 单击启动`}
        </span>
      </div>

      {/* 分类药丸 */}
      <div className="mb-6 flex flex-wrap items-center gap-2">
        {(['全部', ...CATEGORIES] as const).map((c) => (
          <button
            key={c}
            onClick={() => setCat(c)}
            className={`rounded-full px-3.5 py-1.5 text-[12.5px] transition-colors ${
              cat === c
                ? 'bg-[var(--accent)] font-medium text-white'
                : 'bg-[var(--panel)] text-[var(--text-muted)] hover:text-[var(--text)] ring-1 ring-[var(--border)]'
            }`}
          >
            {c}
            <span className="ml-1 opacity-60 tabular-nums">{countOf(c)}</span>
          </button>
        ))}
      </div>

      {/* 图标宫格 */}
      {isLoading ? (
        <div className="grid place-items-center py-24">
          <Loader2 size={22} className="animate-spin text-[var(--text-muted)]" />
        </div>
      ) : shown.length === 0 ? (
        <p className="py-24 text-center text-sm text-[var(--text-muted)]">
          该分类下暂无应用
        </p>
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(92px,1fr))] justify-items-center gap-y-5">
          {shown.map((a) => (
            <button
              key={a.app_key}
              onClick={() => commands.appLaunch(a.app_key).catch(console.error)}
              className="group flex w-[84px] flex-col items-center gap-1.5 rounded-xl p-2 outline-none transition-colors hover:bg-[var(--hover)]"
              title={a.category}
            >
              {a.icon_path ? (
                <img
                  src={a.icon_path.startsWith('data:') ? a.icon_path : convertFileSrc(a.icon_path)}
                  alt=""
                  draggable={false}
                  className="squircle size-[52px] object-cover transition-transform active:scale-90"
                />
              ) : (
                <Monogram name={a.display_name} size={52} />
              )}
              <span className="w-full truncate text-center text-[11.5px] text-[var(--text)] opacity-90">
                {a.display_name}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
