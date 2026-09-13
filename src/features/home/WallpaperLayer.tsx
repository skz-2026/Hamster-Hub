/**
 * 壁纸图片层：cover 铺满容器的底层 <img>（渐变壁纸直接用容器 css，不渲染本层）。
 * 内置插画壁纸是打包资源地址直接可用；自定义壁纸存的是本机路径，
 * 需经 asset protocol 转换（scope 限 $APPDATA/**），浏览器预览无该协议则整层隐藏。
 */
import { convertFileSrc } from '@tauri-apps/api/core';
import { isTauri } from '@/shared/lib/ipc';
import type { Wallpaper } from './layout';

export function wallpaperSrcOf(wallpaper: Wallpaper): string | undefined {
  if (wallpaper.image) return wallpaper.image;
  return wallpaper.imagePath && isTauri ? convertFileSrc(wallpaper.imagePath) : undefined;
}

export function WallpaperLayer({ wallpaper }: { wallpaper: Wallpaper }) {
  const src = wallpaperSrcOf(wallpaper);
  if (!src) return null;
  return (
    <img
      src={src}
      alt=""
      aria-hidden
      draggable={false}
      className="pointer-events-none absolute inset-0 size-full select-none object-cover"
    />
  );
}
