import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Search, LayoutDashboard, CalendarDays, LayoutGrid, FolderOpen, Settings, SlidersHorizontal, Sparkles, Bot } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { commands } from '@/shared/lib/ipc';
import { useClock, useDateTimeInfo, greetingOf } from '@/features/workbench/hooks';
import { useApps, useHomeLayout } from '@/features/home/hooks';
import { WALLPAPERS, type WidgetType } from '@/features/home/layout';
import { HomeWidget } from '@/features/home/HomeWidget';
import { ControlCenter } from '@/features/control/ControlCenter';

/** 桌面小组件行（对齐水豚：问候语下方一排玻璃卡） */
const WIDGETS: WidgetType[] = ['clock', 'weather', 'todo', 'countdown'];

/** 快捷入口（桌面模式无侧栏，页面导航收进这里） */
const QUICK_LINKS: { label: string; to: string; Icon: LucideIcon }[] = [
  { label: '工作台', to: '/', Icon: LayoutDashboard },
  { label: '代理', to: '/bench', Icon: Bot },
  { label: '日程', to: '/schedule', Icon: CalendarDays },
  { label: '应用', to: '/apps', Icon: LayoutGrid },
  { label: '文件', to: '/files', Icon: FolderOpen },
  { label: '设置', to: '/settings', Icon: Settings },
];

/**
 * 桌面主页（桌面模式默认页，macOS 桌面质感）：
 * 全屏网格渐变壁纸 + 中央问候语/超大细体时钟/农历 + Spotlight 式搜索框 +
 * 小组件卡片行 + 快捷入口。底部为独立置顶任务栏窗口（完全替代系统任务栏）。
 */
export default function DesktopPage() {
  const navigate = useNavigate();
  const now = useClock();
  const { time, date, lunar } = useDateTimeInfo(now);
  const { data: apps = [] } = useApps();
  const { layout } = useHomeLayout(apps);
  const wallpaper = WALLPAPERS[layout.wallpaper] ?? WALLPAPERS.midnight;
  const [ccOpen, setCcOpen] = useState(false);

  // 桌面模式已关则退回窗口化工作台（事件导航由 AppShell 统一处理，这里兜底）
  useEffect(() => {
    commands
      .desktopModeIsActive()
      .then((active) => {
        if (!active) navigate('/', { replace: true });
      })
      .catch(() => {});
  }, [navigate]);

  return (
    <div
      className="relative flex h-full flex-col items-center justify-center gap-8 overflow-hidden px-8 text-white"
      style={{ background: wallpaper.css }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {/* 柔光晕（模拟 macOS 壁纸景深） */}
      <div className="pointer-events-none absolute inset-x-0 top-0 h-64 bg-gradient-to-b from-white/[0.05] to-transparent" />

      {/* 控制中心触发钮（右上角热区） */}
      <button
        onClick={() => setCcOpen((v) => !v)}
        title="控制中心"
        aria-label="控制中心"
        className="absolute right-5 top-5 z-40 grid size-10 place-items-center rounded-full bg-white/10 text-white/85 ring-1 ring-white/15 backdrop-blur-xl transition-all hover:bg-white/20 hover:text-white"
      >
        <SlidersHorizontal size={17} />
      </button>
      <ControlCenter open={ccOpen} onClose={() => setCcOpen(false)} />

      {/* 问候 + 大时钟（SF 风：细体、收紧字距、纵向呼吸；错落入场） */}
      <div className="rise-in relative flex flex-col items-center gap-2">
        <span className="text-5xl leading-none drop-shadow-[0_4px_16px_rgba(0,0,0,.4)]">🐹</span>
        <h1 className="text-[22px] font-medium tracking-wide text-white/95 [text-shadow:0_2px_16px_rgba(0,0,0,.45)]">
          {greetingOf(now)}，欢迎回来
        </h1>
        <div className="text-[96px] font-extralight leading-none tracking-[-0.02em] tabular-nums [text-shadow:0_6px_32px_rgba(0,0,0,.45)]">
          {time.slice(0, 5)}
        </div>
        <div className="text-[13.5px] font-normal text-white/75 [text-shadow:0_1px_8px_rgba(0,0,0,.4)]">
          {date} · {lunar}
        </div>
      </div>

      {/* Spotlight 式大搜索框 */}
      <button
        onClick={() => navigate('/search')}
        className="rise-in group relative flex h-[52px] w-[min(600px,68vw)] items-center gap-3 rounded-full bg-black/30 px-6 ring-1 ring-white/15 backdrop-blur-2xl transition-all hover:bg-black/38 hover:ring-white/25"
        style={{
          animationDelay: '80ms',
          boxShadow: '0 8px 32px rgba(0,0,0,.35), inset 0 1px 0 rgba(255,255,255,.08)',
        }}
      >
        <Search size={19} className="shrink-0 text-white/60 transition-colors group-hover:text-white/85" />
        <span className="text-[15px] text-white/55 transition-colors group-hover:text-white/70">
          搜索应用、文件…
        </span>
      </button>

      {/* 小组件卡片行 */}
      <div className="rise-in flex flex-wrap justify-center gap-4" style={{ animationDelay: '160ms' }}>
        {WIDGETS.map((t) => (
          <div key={t} className="w-[180px]">
            <HomeWidget type={t} />
          </div>
        ))}
      </div>

      {/* 快捷入口（AI 助手优先 + 页面导航） */}
      <div className="rise-in flex items-center gap-2.5" style={{ animationDelay: '240ms' }}>
        <button
          onClick={() => navigate('/agent')}
          className="flex items-center gap-2 rounded-full bg-[var(--accent)]/85 px-[18px] py-[7px] text-[12.5px] font-semibold text-white shadow-[0_4px_18px_rgba(240,112,15,.35)] transition-all hover:brightness-110"
        >
          <Sparkles size={14} />
          AI 助手
        </button>
        {QUICK_LINKS.map(({ label, to, Icon }) => (
          <button
            key={to}
            onClick={() => navigate(to)}
            className="flex items-center gap-1.5 rounded-full bg-white/10 px-4 py-[7px] text-[12.5px] font-medium text-white/85 ring-1 ring-white/12 backdrop-blur-xl transition-all hover:bg-white/18 hover:ring-white/22"
          >
            <Icon size={14} />
            {label}
          </button>
        ))}
      </div>
    </div>
  );
}
