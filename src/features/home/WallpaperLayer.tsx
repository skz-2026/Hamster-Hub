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
    <div className="pointer-events-none absolute inset-0" aria-hidden>
      <img
        src={src}
        alt=""
        draggable={false}
        className="size-full select-none object-cover"
      />
      {/* 用户图片亮度不可控（浅色木纹/纯白图上白字与玻璃卡直接糊掉）：
          图片壁纸统一压暗一档，文字与玻璃卡保持可读；渐变壁纸不渲染本层、不受影响 */}
      <div className="absolute inset-0 bg-black/30" />
    </div>
  );
}
