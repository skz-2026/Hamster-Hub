/**
 * 主屏壁纸选择器（设置 → 外观）：内置渐变/插画缩略图 + 本地图片自定义。
 * 即点即存：写 home.layout（KV），经 useHomeLayout 的 commit 直写缓存 +
 * 防抖持久化 + 广播，主屏/桌面主页/任务栏窗口即时跟随。
 */
import { useState } from 'react';
import { ImagePlus, Loader2 } from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { useApps, useHomeLayout } from '@/features/home/hooks';
import {
  CUSTOM_WALLPAPER_ID,
  WALLPAPERS,
  resolveWallpaper,
  type Wallpaper,
} from '@/features/home/layout';
import { wallpaperSrcOf } from '@/features/home/WallpaperLayer';
import { commands, isTauri } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';

export function WallpaperPicker() {
  const { data: apps = [] } = useApps();
  const { layout, commit, ready } = useHomeLayout(apps);
  const [importing, setImporting] = useState(false);
  const { t } = useI18n();

  if (!ready) {
    return <span className="text-xs text-[var(--text-muted)]">{t('settings.wallpaper.loading')}</span>;
  }

  const pickCustom = async () => {
    if (!isTauri || importing) return;
    setImporting(true);
    try {
      // 传上一张自定义壁纸路径：导入成功后后端会清掉旧副本（仅限 wallpapers 目录内）
      const picked = await openDialog({
        multiple: false,
        directory: false,
        filters: [{ name: t('settings.wallpaper.filterName'), extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif'] }],
      });
      const path = Array.isArray(picked) ? picked[0] : picked;
      if (!path) return;
      const imported = await commands.wallpaperImageImport(
        path,
        layout.wallpaperImage ?? null,
      );
      commit({ ...layout, wallpaper: CUSTOM_WALLPAPER_ID, wallpaperImage: imported });
    } catch (e) {
      console.error('[settings] 自定义壁纸导入失败', e);
    } finally {
      setImporting(false);
    }
  };

  const hasCustom = Boolean(layout.wallpaperImage);

  return (
    <div className="flex flex-wrap items-center gap-2">
      {Object.entries(WALLPAPERS).map(([id, w]) => (
        <WallpaperThumb
          key={id}
          wallpaper={w}
          title={w.name}
          active={layout.wallpaper === id}
          onClick={() => commit({ ...layout, wallpaper: id })}
        />
      ))}
      {hasCustom && (
        <WallpaperThumb
          wallpaper={resolveWallpaper(layout)}
          title={t('settings.wallpaper.customImage')}
          active={layout.wallpaper === CUSTOM_WALLPAPER_ID}
          onClick={pickCustom}
        />
      )}
      <button
        onClick={pickCustom}
        disabled={importing}
        title={t('settings.wallpaper.pickTitle')}
        aria-label={t('settings.wallpaper.pickAria')}
        className="grid h-10 w-16 place-items-center rounded-lg border border-dashed border-[var(--border)] text-[var(--text-muted)] transition-colors hover:border-[var(--accent)] hover:text-[var(--text)] disabled:opacity-60"
      >
        {importing ? <Loader2 size={14} className="animate-spin" /> : <ImagePlus size={14} />}
      </button>
    </div>
  );
}

/** 壁纸缩略图：渐变做底色兜底，图片壁纸叠 cover 图 */
function WallpaperThumb({
  wallpaper,
  title,
  active,
  onClick,
}: {
  wallpaper: Wallpaper;
  title: string;
  active: boolean;
  onClick: () => void;
}) {
  const src = wallpaperSrcOf(wallpaper);
  return (
    <button
      onClick={onClick}
      title={title}
      aria-label={title}
      className={`h-10 w-16 shrink-0 overflow-hidden rounded-lg transition-transform hover:scale-105 ${
        active
          ? 'ring-2 ring-[var(--accent)] ring-offset-2 ring-offset-[var(--bg)]'
          : 'ring-1 ring-[var(--border)]'
      }`}
      style={{ background: wallpaper.css }}
    >
      {src && <img src={src} alt="" className="size-full object-cover" />}
    </button>
  );
}
