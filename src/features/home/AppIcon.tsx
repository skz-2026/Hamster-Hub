import { useEffect, useMemo, useRef, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { Folder } from 'lucide-react';
import { useI18n } from '@/shared/i18n/provider';

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
  const { t } = useI18n();
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
            aria-label={t('home.action.remove')}
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

/** 文件夹图标：2×2 缩略拼贴预览（iOS 风）；悬停弹出卡片，里面的应用可直接点击启动 */
export function FolderIcon({
  apps,
  name,
  size = 60,
  label = true,
  jiggle,
  onRemove,
  onPointerDown,
  onClick,
  onAppOpen,
  mergeHint,
  dragging,
}: {
  /** 文件夹内应用（key 用于启动回调） */
  apps: { key: string; name: string; iconPath?: string | null }[];
  name: string;
  size?: number;
  label?: boolean;
  jiggle?: boolean;
  onRemove?: () => void;
  onPointerDown?: (e: React.PointerEvent) => void;
  onClick?: () => void;
  /** 悬停卡片里点击应用时触发；缺省则不启用悬停卡片 */
  onAppOpen?: (key: string) => void;
  mergeHint?: boolean;
  dragging?: boolean;
}) {
  const { t } = useI18n();
  const names = apps.map((a) => a.name);
  const iconPaths = apps.map((a) => a.iconPath);
  const preview = iconPaths.slice(0, 4);
  // 悬停卡片：延迟弹出（划过不闪窗）；fixed 定位 + 视口内收拢，
  // 避免被主屏分页滚动容器裁剪。优先弹在图标下方，下方放不下再弹上方。
  const btnRef = useRef<HTMLButtonElement>(null);
  const [card, setCard] = useState<{ left: number; top: number; width: number; cols: number } | null>(null);
  const hoverable = !!onAppOpen && !jiggle && apps.length > 0;
  const openTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const clearTimers = () => {
    if (openTimer.current) clearTimeout(openTimer.current);
    if (closeTimer.current) clearTimeout(closeTimer.current);
  };
  useEffect(() => clearTimers, []);
  const scheduleOpen = () => {
    clearTimers();
    openTimer.current = setTimeout(() => {
      const rect = btnRef.current?.getBoundingClientRect();
      if (!rect) return;
      const cols = Math.min(4, apps.length);
      const rows = Math.ceil(apps.length / cols);
      const cell = 56;
      const gap = 10;
      // 32 = p-4 左右内边距；+8 = 纵向滚动条（全局 6px）占位余量，避免挤窄网格裁掉最右列
      const width = cols * cell + (cols - 1) * gap + 40;
      const height = Math.min(rows * 66 + (rows - 1) * gap + 32, 296);
      const below = rect.bottom + height + 16 < window.innerHeight - 90;
      const left = Math.min(Math.max(rect.left + rect.width / 2 - width / 2, 8), window.innerWidth - width - 8);
      setCard({ left, top: below ? rect.bottom + 10 : Math.max(rect.top - height - 10, 8), width, cols });
    }, 350);
  };
  const scheduleClose = () => {
    clearTimers();
    closeTimer.current = setTimeout(() => setCard(null), 140);
  };
  return (
    <div onMouseEnter={hoverable ? scheduleOpen : undefined} onMouseLeave={hoverable ? scheduleClose : undefined}>
      <button
        type="button"
        ref={btnRef}
        onPointerDown={(e) => {
          clearTimers();
          setCard(null);
          onPointerDown?.(e);
        }}
        onClick={() => {
          clearTimers();
          setCard(null);
          onClick?.();
        }}
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
              aria-label={t('home.action.deleteFolder')}
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
      {card && (
        <div
          className="ios-folder-panel fixed z-50 rounded-[26px] p-4"
          style={{ left: card.left, top: card.top, width: card.width }}
        >
          <div
            data-inner-scroll
            className="grid max-h-[264px] justify-center justify-items-center gap-x-[10px] gap-y-3 overflow-y-auto overscroll-contain"
            style={{ gridTemplateColumns: `repeat(${card.cols}, 56px)` }}
          >
            {apps.map((a) => (
              <button
                key={a.key}
                type="button"
                title={a.name}
                onClick={() => {
                  setCard(null);
                  onAppOpen?.(a.key);
                }}
                className="flex w-14 flex-col items-center gap-1 rounded-xl p-1 transition-colors hover:bg-white/12"
              >
                {a.iconPath ? (
                  <img
                    src={iconSrc(a.iconPath)}
                    alt={a.name}
                    draggable={false}
                    className="squircle h-11 w-11 object-cover"
                  />
                ) : (
                  <Monogram name={a.name} size={44} />
                )}
                <span className="w-full truncate text-center text-[10.5px] leading-tight text-white [text-shadow:0_1px_3px_rgba(0,0,0,.55)]">
                  {a.name}
                </span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
