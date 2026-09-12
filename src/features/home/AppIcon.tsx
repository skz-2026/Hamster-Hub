import { useMemo } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { Folder } from 'lucide-react';

/** data:URL（mock 层）直通，本地图标路径经 tauri asset 协议转换 */
function iconSrc(iconPath: string): string {
  return iconPath.startsWith('data:') ? iconPath : convertFileSrc(iconPath);
}

function hashHue(s: string) {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) | 0;
  return Math.abs(h) % 360;
}

export function Monogram({ name, size }: { name: string; size: number }) {
  const hue = useMemo(() => hashHue(name), [name]);
  const letter = [...name][0]?.toUpperCase() ?? '?';
  return (
    <div
      className="squircle grid place-items-center font-semibold text-white"
      style={{
        width: size,
        height: size,
        fontSize: size * 0.42,
        background: `linear-gradient(135deg, hsl(${hue} 72% 58%), hsl(${(hue + 40) % 360} 68% 42%))`,
      }}
    >
      {letter}
    </div>
  );
}

interface AppIconProps {
  name: string;
  iconPath?: string | null;
  size?: number;
  label?: boolean;
  jiggle?: boolean;
  /** 编辑模式的删除角标 */
  onRemove?: () => void;
  onPointerDown?: (e: React.PointerEvent) => void;
  onClick?: () => void;
  /** 拖拽悬停高亮（合并文件夹提示） */
  mergeHint?: boolean;
  dragging?: boolean;
  className?: string;
}

/** iOS 风 squircle 应用图标（真图标 PNG / 字母渐变占位） */
export function AppIcon({
  name,
  iconPath,
  size = 60,
  label = true,
  jiggle,
  onRemove,
  onPointerDown,
  onClick,
  mergeHint,
  dragging,
  className,
}: AppIconProps) {
  return (
    <button
      type="button"
      onPointerDown={onPointerDown}
      onClick={onClick}
      className={`group relative flex select-none flex-col items-center gap-1.5 outline-none ${className ?? ''}`}
      style={{ width: 84 }}
    >
      <span
        className={`relative block transition-transform duration-150 active:scale-90 ios-ease ${
          jiggle ? 'jiggling' : ''
        } ${dragging ? 'opacity-30 scale-75' : ''} ${
          mergeHint ? 'ring-2 ring-white/80 rounded-[22.5%] scale-105' : ''
        }`}
        style={{ width: size, height: size }}
      >
        {iconPath ? (
          <img
            src={iconSrc(iconPath)}
            alt={name}
            draggable={false}
            className="squircle h-full w-full object-cover"
            style={{ width: size, height: size }}
          />
        ) : (
          <Monogram name={name} size={size} />
        )}
        {onRemove && (
          <span
            role="button"
            aria-label="移除"
            onClick={(e) => {
              e.stopPropagation();
              onRemove();
            }}
            className="absolute -left-1 -top-1 z-10 grid size-5 place-items-center rounded-full bg-neutral-900/85 text-[11px] text-white opacity-0 group-hover:opacity-100"
          >
            ✕
          </span>
        )}
      </span>
      {label && (
        <span className="w-full truncate text-center text-[11.5px] leading-tight text-white [text-shadow:0_1px_3px_rgba(0,0,0,.55)]">
          {name}
        </span>
      )}
    </button>
  );
}

/** 文件夹图标：2×2 缩略拼贴预览（iOS 风） */
export function FolderIcon({
  names,
  iconPaths,
  size = 60,
  label = true,
  name,
  jiggle,
  onRemove,
  onPointerDown,
  onClick,
  mergeHint,
  dragging,
}: {
  names: string[];
  iconPaths: (string | null | undefined)[];
  name: string;
  size?: number;
  label?: boolean;
  jiggle?: boolean;
  onRemove?: () => void;
  onPointerDown?: (e: React.PointerEvent) => void;
  onClick?: () => void;
  mergeHint?: boolean;
  dragging?: boolean;
}) {
  const preview = iconPaths.slice(0, 4);
  return (
    <button
      type="button"
      onPointerDown={onPointerDown}
      onClick={onClick}
      className="group relative flex select-none flex-col items-center gap-1.5 outline-none"
      style={{ width: 84 }}
    >
      <span
        className={`relative block transition-transform duration-150 active:scale-90 ios-ease ${
          jiggle ? 'jiggling' : ''
        } ${dragging ? 'opacity-30 scale-75' : ''} ${
          mergeHint ? 'ring-2 ring-white/80 rounded-[22.5%] scale-105' : ''
        }`}
        style={{ width: size, height: size }}
      >
        <span className="squircle absolute inset-0 grid grid-cols-2 grid-rows-2 gap-[3px] bg-white/22 p-[7px] backdrop-blur-xl">
          {preview.map((p, i) =>
            p ? (
              <img
                key={i}
                src={iconSrc(p)}
                alt=""
                draggable={false}
                className="squircle h-full w-full object-cover"
              />
            ) : (
              <Monogram key={i} name={names[i] ?? '?'} size={Number.MAX_SAFE_INTEGER} />
            ),
          )}
        </span>
        {preview.length === 0 && (
          <span className="absolute inset-0 grid place-items-center text-white/90">
            <Folder size={size * 0.45} />
          </span>
        )}
        {onRemove && (
          <span
            role="button"
            aria-label="删除文件夹"
            onClick={(e) => {
              e.stopPropagation();
              onRemove();
            }}
            className="absolute -left-1 -top-1 z-10 grid size-5 place-items-center rounded-full bg-neutral-900/85 text-[11px] text-white opacity-0 group-hover:opacity-100"
          >
            ✕
          </span>
        )}
      </span>
      {label && (
        <span className="w-full truncate text-center text-[11.5px] leading-tight text-white [text-shadow:0_1px_3px_rgba(0,0,0,.55)]">
          {name}
        </span>
      )}
    </button>
  );
}
