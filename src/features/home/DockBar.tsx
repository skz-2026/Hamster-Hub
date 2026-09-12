import type { ReactNode } from 'react';
import { useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { emitTo, emit } from '@tauri-apps/api/event';
import { useQueryClient } from '@tanstack/react-query';
import { useLocation, useNavigate } from 'react-router-dom';
import { LayoutGrid, LogOut, Bot, Plus, Search, Settings } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { commands, isTauri, type AppEntry } from '@/shared/lib/ipc';
import { useTopApps } from '@/features/dashboard/hooks';
import { useApps, useHomeLayout } from './hooks';
import { Monogram } from './AppIcon';
import { DOCK_CAPACITY, removeFromDock } from './layout';

interface DockBarProps {
  /**
   * 桌面模式（全屏接管）：贴边通栏任务栏，完全替代系统任务栏——
   * 左端主屏/搜索/设置入口 + 定制/常用分组，右端退出按钮 + 时钟日期（系统托盘位）。
   * 窗口化模式：居中胶囊 Dock。
   */
  desktop?: boolean;
}

/** 系统入口（任务栏左端 / 胶囊右段）：应用内路由 */
const SYSTEM_ICONS: { label: string; to: string; Icon: LucideIcon }[] = [
  { label: '代理', to: '/bench', Icon: Bot },
  { label: '主屏', to: '/home', Icon: LayoutGrid },
  { label: '搜索', to: '/search', Icon: Search },
  { label: '设置', to: '/settings', Icon: Settings },
];

/**
 * 底部常驻 Dock（应用壳布局底部的独立区域，内容不被遮挡）。
 * 定制 = 主屏编辑模式拖入；常用 = 启动频次 TopN（与定制组去重）。
 * 悬停放大上浮 + tooltip + 活动路由指示点；拖拽编辑在 HomeScreen 的交互 Dock。
 */
export function DockBar({ desktop }: DockBarProps) {
  const navigate = useNavigate();
  const qc = useQueryClient();
  const { pathname } = useLocation();
  const { data: apps = [] } = useApps();
  const { layout, commit } = useHomeLayout(apps);
  const { data: top = [] } = useTopApps(10);
  const now = useClockMinute();
  // 右键菜单（任务栏定制组）：目标 dock 索引 + 屏幕坐标
  const [menu, setMenu] = useState<{ idx: number; x: number } | null>(null);

  // 任务栏独立窗口内不能本地 navigate（会把 /home 载入 72px 窗口条），
  // 改为发事件让主窗口导航并前置
  const inTaskbarWindow = isTauri && window.location.hash.startsWith('#/taskbar');
  const goRoute = (to: string) => {
    if (inTaskbarWindow) {
      emitTo('main', 'hamster:navigate', { to }).catch(console.error);
    } else {
      navigate(to);
    }
  };

  const size = desktop ? 44 : 50;
  const appByKey = new Map(apps.map((a) => [a.app_key, a]));
  const frequent = top.filter((a) => !layout.dock.includes(a.app_key)).slice(0, desktop ? 6 : 4);
  const exitDesktop = () => commands.desktopModeExit().catch(console.error);

  /** 启动 + 常用组即时刷新（usage 已落库，invalidate 触发 top_apps 重取） */
  const launchApp = (key: string) => {
    commands.appLaunch(key).catch(console.error);
    qc.invalidateQueries({ queryKey: ['apps', 'top'] });
  };

  /** 任务栏定制组的增删（主屏编辑模式之外的第二入口） */
  const removeDockItem = (idx: number) => {
    commit(removeFromDock(layout, idx));
    setMenu(null);
    syncLayout();
  };
  const syncLayout = () => {
    if (isTauri) emit('hamster:layout-updated').catch(console.error);
  };
  const openPicker = () => {
    if (inTaskbarWindow) {
      emitTo('main', 'hamster:open-app-picker', {}).catch(console.error);
    } else {
      window.dispatchEvent(new CustomEvent('hamster:open-app-picker'));
    }
  };

  // 点击菜单外关闭
  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    window.addEventListener('pointerdown', close);
    return () => window.removeEventListener('pointerdown', close);
  }, [menu]);

  if (desktop) {
    // 贴边通栏任务栏（完全替代系统任务栏）：左端入口 + 定制（可增删）…… 右端退出 + 时钟
    return (
      <div className="relative z-30 flex shrink-0">
        {/* 桌面任务栏：条底色由 TaskbarPage 统一绘制，去掉 ios-dock 皮肤
            （bar 里套 bar 会在窗口顶部夹出一条露壁纸的缝 + 双重高光线） */}
        <div className="flex w-full items-end gap-2.5 rounded-none px-4 pb-[7px] pt-0">
          {SYSTEM_ICONS.map(({ label, to, Icon }) => (
            <DockItem key={to} label={label} active={pathname === to} nativeTipOnly={desktop} onClick={() => goRoute(to)}>
              <SystemTile Icon={Icon} size={size} />
            </DockItem>
          ))}
          {layout.dock.length > 0 && <Divider />}
          {layout.dock.map((key, idx) => {
            const a = appByKey.get(key);
            if (!a) return null;
            return (
              <AppDockItem
                key={key}
                app={a}
                size={size}
                nativeTipOnly={desktop}
                onLaunch={launchApp}
                onContextMenu={
                  desktop
                    ? (x: number) => setMenu({ idx, x })
                    : undefined
                }
              />
            );
          })}
          {/* 添加定制应用（主窗口弹选择器） */}
          {layout.dock.length < DOCK_CAPACITY && (
            <button
              onClick={openPicker}
              title="添加应用到任务栏"
              aria-label="添加应用到任务栏"
              className="group grid size-[44px] place-items-center rounded-xl bg-white/[0.08] text-white/55 ring-1 ring-white/12 transition-all hover:bg-white/16 hover:text-white/90"
            >
              <Plus size={19} />
            </button>
          )}
          {frequent.length > 0 && <Divider />}
          {frequent.map((a) => (
            <AppDockItem key={a.app_key} app={a} size={size} nativeTipOnly={desktop} onLaunch={launchApp} />
          ))}
          <div className="ml-auto flex items-end gap-2 pl-3">
            <DockItem label="退出桌面模式" nativeTipOnly={desktop} onClick={exitDesktop}>
              <span
                className="squircle grid place-items-center bg-red-500/45 text-white ring-1 ring-white/25"
                style={{ width: size, height: size }}
              >
                <LogOut size={Math.round(size * 0.55)} strokeWidth={1.8} />
              </span>
            </DockItem>
            <Divider />
            <div className="flex flex-col items-end justify-center self-stretch px-1.5 leading-tight">
              <span className="text-[13px] font-medium tabular-nums text-white [text-shadow:0_1px_3px_rgba(0,0,0,.45)]">
                {now.time}
              </span>
              <span className="text-[10px] text-white/70">{now.date}</span>
            </div>
          </div>
        </div>

        {/* 右键菜单：从任务栏移除（贴窗口顶渲染，避免 72px 窗口裁剪） */}
        {menu && (
          <div
            className="absolute top-1 z-50 flex items-center gap-1 rounded-xl bg-[#232028]/97 py-1 pl-3 pr-1 text-[12px] text-white/90 ring-1 ring-white/15 backdrop-blur-xl"
            style={{ left: Math.min(menu.x, window.innerWidth - 200) }}
            onPointerDown={(e) => e.stopPropagation()}
          >
            <span className="text-white/50">定制区</span>
            <button
              onClick={() => removeDockItem(menu.idx)}
              className="rounded-lg bg-red-500/25 px-2.5 py-1 font-medium text-red-200 transition-colors hover:bg-red-500/40"
            >
              移除
            </button>
            <button
              onClick={() => setMenu(null)}
              className="rounded-lg bg-white/8 px-2.5 py-1 transition-colors hover:bg-white/16"
            >
              取消
            </button>
          </div>
        )}
      </div>
    );
  }

  // 窗口化：居中胶囊
  const customGroup = layout.dock.map((key) => {
    const a = appByKey.get(key);
    return a ? <AppDockItem key={key} app={a} size={size} onLaunch={launchApp} /> : null;
  });
  const frequentGroup = frequent.map(
    (a) => <AppDockItem key={a.app_key} app={a} size={size} onLaunch={launchApp} />,
  );
  const systemGroup = SYSTEM_ICONS.map(({ label, to, Icon }) => (
    <DockItem key={to} label={label} active={pathname === to} onClick={() => goRoute(to)}>
      <SystemTile Icon={Icon} size={size} />
    </DockItem>
  ));
  const groups = [
    { key: 'custom', nodes: customGroup },
    { key: 'frequent', nodes: frequentGroup },
    { key: 'system', nodes: systemGroup },
  ].filter((g) => g.nodes.some(Boolean));

  return (
    <div className="relative z-30 flex shrink-0 justify-center pb-2 pt-1">
      <div className="ios-dock flex items-end gap-3 rounded-[24px] px-3.5 py-2">
        {groups.map((g, i) => (
          <div key={g.key} className="flex items-end gap-3">
            {i > 0 && <Divider className="mx-0.5" />}
            {g.nodes}
          </div>
        ))}
      </div>
    </div>
  );
}

/** 应用图标 Dock 项：真图标 PNG / 字母占位，单击启动（顺手刷新常用组） */
function AppDockItem({
  app,
  size,
  onLaunch,
  onContextMenu,
  nativeTipOnly = false,
}: {
  app: AppEntry;
  size: number;
  onLaunch: (key: string) => void;
  /** 桌面任务栏定制组：右键呼出移除菜单（传窗口内 clientX） */
  onContextMenu?: (x: number) => void;
  nativeTipOnly?: boolean;
}) {
  return (
    <DockItem
      label={app.display_name}
      nativeTipOnly={nativeTipOnly}
      onClick={() => onLaunch(app.app_key)}
      onContextMenu={
        onContextMenu
          ? (e) => {
              e.preventDefault();
              onContextMenu(e.clientX);
            }
          : undefined
      }
    >
      {app.icon_path ? (
        <img
          src={app.icon_path.startsWith('data:') ? app.icon_path : convertFileSrc(app.icon_path)}
          alt={app.display_name}
          draggable={false}
          className="squircle object-cover drop-shadow-[0_4px_10px_rgba(0,0,0,.35)]"
          style={{ width: size, height: size }}
        />
      ) : (
        <Monogram name={app.display_name} size={size} />
      )}
    </DockItem>
  );
}

/** 系统图标底座：半透明玻璃 squircle + 线性图标（macOS 系统图标风） */
function SystemTile({ Icon, size }: { Icon: LucideIcon; size: number }) {
  return (
    <span
      className="squircle grid place-items-center bg-white/[0.16] text-white/95 ring-1 ring-white/22 backdrop-blur-xl drop-shadow-[0_4px_10px_rgba(0,0,0,.35)]"
      style={{ width: size, height: size }}
    >
      <Icon size={Math.round(size * 0.52)} strokeWidth={1.8} />
    </span>
  );
}

/** Dock 通用项：tooltip + 悬停放大上浮 + 底部活动指示点（macOS 运行指示）。
 * nativeTipOnly：桌面任务栏条内不渲染自绘 tooltip（超出窗口上边界会被裁剪，
 * 交给按钮原生 title 弹系统提示）；窗口化胶囊内空间充足，仍用自绘样式。 */
function DockItem({
  label,
  onClick,
  active,
  onContextMenu,
  nativeTipOnly = false,
  children,
}: {
  label: string;
  onClick: () => void;
  active?: boolean;
  onContextMenu?: (e: React.MouseEvent) => void;
  nativeTipOnly?: boolean;
  children: ReactNode;
}) {
  return (
    <div className="group relative flex flex-col items-center" onContextMenu={onContextMenu}>
      {!nativeTipOnly && (
        <span className="pointer-events-none absolute -top-8 left-1/2 z-10 -translate-x-1/2 whitespace-nowrap rounded-lg bg-neutral-900/85 px-2.5 py-1 text-[11px] text-white opacity-0 ring-1 ring-white/15 transition-opacity group-hover:opacity-100">
          {label}
        </span>
      )}
      <button
        aria-label={label}
        title={label}
        onClick={onClick}
        className="origin-bottom transition-transform duration-150 ease-out group-hover:scale-125 group-hover:-translate-y-1.5 active:scale-95"
      >
        {children}
      </button>
      {/* 指示点不占布局流，图标保持垂直居中；活动/悬停时点亮 */}
      <span
        aria-hidden
        className={`absolute -bottom-[5px] left-1/2 h-[5px] w-[5px] -translate-x-1/2 rounded-full bg-white shadow-[0_0_6px_rgba(255,255,255,.9)] transition-opacity ${
          active ? 'opacity-100' : 'opacity-0 group-hover:opacity-45'
        }`}
      />
    </div>
  );
}

function Divider({ className = '' }: { className?: string }) {
  return <span aria-hidden className={`h-8 w-px self-center bg-white/22 ${className}`} />;
}

/** 分钟级时钟（任务栏右端） */
function useClockMinute() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const t = setInterval(() => setNow(new Date()), 10_000);
    return () => clearInterval(t);
  }, []);
  const time = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
  const date = new Intl.DateTimeFormat('zh-CN', { month: 'long', day: 'numeric', weekday: 'long' }).format(now);
  return { time, date };
}
