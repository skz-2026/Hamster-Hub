import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Bluetooth,
  Gamepad2,
  Image as ImageIcon,
  LogOut,
  Moon,
  Settings as SettingsIcon,
  Sparkles,
  Sun,
  Volume1,
  Volume2,
  VolumeX,
  Wifi,
} from 'lucide-react';
import { commands } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';
import { useSettings, usePatchSettings } from '@/features/settings/hooks';
import { useApps, useHomeLayout } from '@/features/home/hooks';
import { nextWallpaperId, resolveWallpaper } from '@/features/home/layout';

/**
 * 控制中心（iOS 风，桌面主页右上角滑出玻璃面板）：
 * 磁贴 = Wi-Fi/蓝牙（跳系统设置）、深浅主题、壁纸轮换；滑条 = 主音量（真实生效）；
 * 底部快捷 = AI 助手/设置/退出桌面模式。亮度等需要硬件 API 的项后续版本补。
 */
export function ControlCenter({ open, onClose }: { open: boolean; onClose: () => void }) {
  const navigate = useNavigate();
  const { t } = useI18n();
  const { data: settings } = useSettings();
  const { patch } = usePatchSettings();
  const { data: apps = [] } = useApps();
  const { layout, commit } = useHomeLayout(apps);

  const [level, setLevel] = useState(0.6);
  const [muted, setMuted] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  // 打开时取真实音量；点击面板外关闭
  useEffect(() => {
    if (!open) return;
    commands.volumeGet().then((v) => {
      setLevel(v.level);
      setMuted(v.muted);
    }).catch(() => {});
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: PointerEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) onClose();
    };
    window.addEventListener('pointerdown', onDown);
    return () => window.removeEventListener('pointerdown', onDown);
  }, [open, onClose]);

  if (!open) return null;

  const applyVolume = (next: number, nextMuted?: boolean) => {
    setLevel(next);
    commands.volumeSet(next, nextMuted ?? muted).catch(console.error);
  };
  const toggleMute = () => {
    const next = !muted;
    setMuted(next);
    commands.volumeSet(level, next).catch(console.error);
  };
  const cycleWallpaper = () => {
    commit({ ...layout, wallpaper: nextWallpaperId(layout.wallpaper) });
  };
  const theme = settings?.appearance.theme ?? 'dark';
  // 主题磁贴三态循环：深色 → 浅色 → 像素 → 深色，图标显示即将切到的目标
  const toggleTheme = () =>
    patch({ appearance: { theme: theme === 'dark' ? 'light' : theme === 'light' ? 'pixel' : 'dark' } });
  const nextThemeTile =
    theme === 'dark'
      ? { icon: <Sun size={18} />, label: t('home.cc.lightMode') }
      : theme === 'light'
        ? { icon: <Gamepad2 size={18} />, label: t('home.cc.pixelTheme') }
        : { icon: <Moon size={18} />, label: t('home.cc.darkMode') };
  const openMs = (page: string) =>
    commands.openUrl(`ms-settings:${page}`).catch(console.error);

  const VolIcon = muted || level === 0 ? VolumeX : level < 0.5 ? Volume1 : Volume2;

  return (
    <div
      ref={panelRef}
      className="rise-in absolute right-5 top-5 z-40 w-[336px] rounded-[28px] bg-black/38 p-4 text-white ring-1 ring-white/15 backdrop-blur-2xl"
      style={{ boxShadow: '0 24px 64px rgba(0,0,0,.45), inset 0 1px 0 rgba(255,255,255,.12)' }}
    >
      {/* 磁贴：连接 + 外观 */}
      <div className="grid grid-cols-2 gap-2.5">
        <Tile icon={<Wifi size={18} />} tint="#0a84ff" label="Wi-Fi" sub={t('home.cc.systemSettings')}
          onClick={() => openMs('network')} />
        <Tile icon={<Bluetooth size={18} />} tint="#0a84ff" label={t('home.cc.bluetooth')} sub={t('home.cc.systemSettings')}
          onClick={() => openMs('bluetooth')} />
        <Tile
          icon={nextThemeTile.icon}
          tint="#ff9f0a"
          label={nextThemeTile.label}
          sub={t('home.cc.tapToSwitch')}
          onClick={toggleTheme}
        />
        <Tile
          icon={<ImageIcon size={18} />}
          tint="#bf5af2"
          label={resolveWallpaper(layout).name}
          sub={t('home.cc.tapToCycle')}
          onClick={cycleWallpaper}
        />
      </div>

      {/* 音量滑条（真实主音量；输入框整体为命中区，轨道自绘细条） */}
      <div className="mt-2.5 flex items-center gap-3 rounded-[22px] bg-white/8 px-4 py-2 ring-1 ring-white/10">
        <button onClick={toggleMute} className="text-white/85 transition-transform active:scale-90">
          <VolIcon size={19} />
        </button>
        <input
          type="range"
          min={0}
          max={100}
          value={Math.round(level * 100)}
          onChange={(e) => applyVolume(Number(e.target.value) / 100)}
          className="cc-range h-7 flex-1 cursor-pointer appearance-none bg-transparent"
          aria-label={t('home.cc.volume')}
        />
        <span className="w-8 text-right text-[11px] tabular-nums text-white/60">
          {Math.round(level * 100)}
        </span>
      </div>

      {/* 快捷动作 */}
      <div className="mt-2.5 flex gap-2">
        <Action icon={<Sparkles size={13} />} label={t('home.cc.aiAssistant')} onClick={() => { onClose(); navigate('/agent'); }} />
        <Action icon={<SettingsIcon size={13} />} label={t('home.cc.settings')} onClick={() => { onClose(); navigate('/settings'); }} />
        <Action
          icon={<LogOut size={13} />}
          label={t('home.cc.exitTakeover')}
          danger
          onClick={() => commands.desktopModeExit().catch(console.error)}
        />
      </div>
    </div>
  );
}

/** iOS 控制中心磁贴：彩圈图标 + 标题/副标题 */
function Tile({
  icon,
  tint,
  label,
  sub,
  onClick,
}: {
  icon: React.ReactNode;
  tint: string;
  label: string;
  sub: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className="flex items-center gap-3 rounded-[22px] bg-white/8 px-3.5 py-3 text-left ring-1 ring-white/10 transition-colors hover:bg-white/14"
    >
      <span
        className="grid size-9 shrink-0 place-items-center rounded-full text-white"
        style={{ background: tint }}
      >
        {icon}
      </span>
      <span className="min-w-0">
        <span className="block truncate text-[12.5px] font-medium leading-tight">{label}</span>
        <span className="block truncate text-[10.5px] text-white/50">{sub}</span>
      </span>
    </button>
  );
}

function Action({
  icon,
  label,
  danger,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  danger?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex flex-1 items-center justify-center gap-1.5 rounded-full px-3 py-2 text-[11.5px] font-medium ring-1 transition-colors ${
        danger
          ? 'bg-red-500/25 text-red-200 ring-red-300/25 hover:bg-red-500/40'
          : 'bg-white/10 text-white/85 ring-white/12 hover:bg-white/18'
      }`}
    >
      {icon}
      {label}
    </button>
  );
}
