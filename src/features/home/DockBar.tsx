import type { ReactNode } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { emitTo, emit, listen } from '@tauri-apps/api/event';
import { useQueryClient } from '@tanstack/react-query';
import { useLocation, useNavigate } from 'react-router-dom';
import { House, LayoutGrid, LogOut, Bot, MonitorSmartphone, Plus, Search, Settings } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { commands, isTauri, type AppEntry, type AppWindowInfo } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';
import { getLang } from '@/shared/i18n/core';
import type { TKey } from '@/shared/i18n/core';
import { useTopApps } from '@/features/dashboard/hooks';
import TrayButton from './TrayArea';
import { useApps, useHomeLayout, useRunningAppKeys, useSingleInstanceApps, useSplitActions, useSplitState } from './hooks';
import DockAppMenu from './DockAppMenu';
import DockWindowsCard from './DockWindowsCard';
import SplitPill from './SplitPill';
import SplitPanel from './SplitPanel';
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

/** 系统入口项：to = 应用内路由；onClick = 模式动作（如进入桌面） */
type SystemEntry = {
  labelKey: TKey;
  Icon: LucideIcon;
  to?: string;
  onClick?: () => void;
};

/** 系统入口（任务栏左端 / 胶囊右段）：应用内路由（设置单独挂右端，见 SETTINGS_ENTRY） */
const SYSTEM_ICONS: SystemEntry[] = [
  { labelKey: 'chrome.nav.agent', to: '/bench', Icon: Bot },
  { labelKey: 'chrome.nav.home', to: '/home', Icon: LayoutGrid },
  { labelKey: 'chrome.nav.search', to: '/search', Icon: Search },
];

/** 设置入口：桌面任务栏挪到右端（退出/托盘一侧），胶囊里仍是系统组末位 */
const SETTINGS_ENTRY: SystemEntry = { labelKey: 'chrome.nav.settings', to: '/settings', Icon: Settings };

/**
 * 底部常驻 Dock（应用壳布局底部的独立区域，内容不被遮挡）。
 * 定制 = 主屏编辑模式拖入；常用 = 启动频次 TopN（与定制组去重）。
 * 悬停放大上浮 + tooltip + 活动路由指示点；拖拽编辑在 HomeScreen 的交互 Dock。
 */
export function DockBar({ desktop }: DockBarProps) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const qc = useQueryClient();
  const { pathname } = useLocation();
  const { data: apps = [] } = useApps();
  const { layout, commit } = useHomeLayout(apps);
  const { data: top = [] } = useTopApps(10);
  const now = useClockMinute();
  // 右键菜单：目标应用 + 锚点坐标 + 是否定制组（可移除）
  const [menu, setMenu] = useState<{ key: string; x: number; removable: boolean } | null>(null);
  // 悬停窗口卡片（本地渲染形态；任务栏窗口走独立弹窗）：目标 + 锚点 + 窗口清单
  const [peek, setPeek] = useState<{ key: string; x: number; windows: AppWindowInfo[] } | null>(null);
  // menu 的 ref 镜像：右键与悬停互斥——定时器/查询回调里读不到新鲜 state，
  // 用 ref 判断「菜单已开」则不再弹卡
  const menuKeyRef = useRef<string | null>(null);
  useEffect(() => {
    menuKeyRef.current = menu?.key ?? null;
  }, [menu]);

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
  const frequent = useMemo(
    () => top.filter((a) => !layout.dock.includes(a.app_key)).slice(0, desktop ? 6 : 4),
    [top, layout.dock, desktop],
  );
  // 运行态轮询目标：定制 + 常用两组去重后的 key（稳定引用，见 useRunningAppKeys）
  const dockAppKeys = useMemo(
    () => [...layout.dock, ...frequent.map((a) => a.app_key)],
    [layout.dock, frequent],
  );
  const runningKeys = useRunningAppKeys(dockAppKeys);
  // 已知单实例应用（内置名单 + 自学习）：菜单隐藏「多开应用」
  const singleApps = useSingleInstanceApps(dockAppKeys);
  // 托管分屏（仅桌面接管态）：胶囊状态 + 右键菜单那一行的三态（添加/移出/已满）。
  // 会话由 Rust 维持，这里 2.5s 轮询快照；窗口化胶囊形态不开放分屏入口，
  // 故 enabled=desktop——不轮询也就没有后台开销
  const split = useSplitState(!!desktop);
  const splitActions = useSplitActions();
  const splitMembers = useMemo(
    () => new Set((split.data?.members ?? []).map((m) => m.appKey)),
    [split.data],
  );
  const splitCount = split.data?.members.length ?? 0;
  const splitFull = splitCount >= (split.data?.capacity ?? 4);
  // 本地（浏览器预览/主窗口内）分屏管理浮层：任务栏窗一条高放不下，走独立弹窗
  const [splitPanelOpen, setSplitPanelOpen] = useState(false);
  const exitDesktop = () => commands.desktopModeExit().catch(console.error);
  const enterDesktop = () => commands.desktopModeEnter().catch(console.error);
  /** 开始：注入 Ctrl+Esc 召出系统真实开始菜单（全局键注入，任务栏窗口内调用同样有效） */
  const openStartMenu = () => commands.startMenuOpen().catch(console.error);

  // 窗口化胶囊里「主屏」不可达（主屏仅在接管态开放，点了会被弹回），同一位换「桌面模式」快捷进入
  const systemEntries: SystemEntry[] = desktop
    ? SYSTEM_ICONS
    : [
        ...SYSTEM_ICONS.map((e): SystemEntry =>
          e.to === '/home'
            ? { labelKey: 'chrome.nav.desktopMode', Icon: MonitorSmartphone, onClick: enterDesktop }
            : e,
        ),
        SETTINGS_ENTRY,
      ];
  /** 系统入口组（两形态共用渲染；桌面任务栏只吃原生 tooltip） */
  const renderSystem = (nativeTipOnly: boolean) =>
    systemEntries.map(({ labelKey, to, Icon, onClick }) => (
      <DockItem
        key={labelKey}
        label={t(labelKey)}
        active={to !== undefined && pathname === to}
        nativeTipOnly={nativeTipOnly}
        onClick={to !== undefined ? () => goRoute(to) : onClick!}
      >
        <SystemTile Icon={Icon} size={size} />
      </DockItem>
    ));

  /** 启动 + 常用组即时刷新（usage 已落库，invalidate 触发 top_apps 重取） */
  const launchApp = (key: string) => {
    commands.appLaunch(key).catch(console.error);
    qc.invalidateQueries({ queryKey: ['apps', 'top'] });
    qc.invalidateQueries({ queryKey: ['apps', 'running'] });
  };
  /** 多开：绕过「已运行激活」，直接再开一个窗口/实例（单击仍是默认逻辑） */
  const launchNew = (key: string) => {
    commands.appLaunchNew(key).catch(console.error);
    qc.invalidateQueries({ queryKey: ['apps', 'top'] });
    qc.invalidateQueries({ queryKey: ['apps', 'running'] });
    // 多开能力自学习在启动后 ~1.5s 出结论，延迟失效让菜单条件项即时跟上
    window.setTimeout(() => qc.invalidateQueries({ queryKey: ['apps', 'single'] }), 2200);
    setMenu(null);
  };
  /** 关闭应用：温和关掉其全部可见窗口（WM_CLOSE，应用可弹保存确认） */
  const closeApp = (key: string) => {
    commands.appClose(key).catch(console.error);
    qc.invalidateQueries({ queryKey: ['apps', 'running'] });
    setMenu(null);
  };

  /** 分屏添加：把该应用的窗口纳入托管（会话由 Rust 维持，胶囊随轮询出现） */
  const splitAdd = (key: string) => {
    // 已满 4 扇 / 该应用没有可分屏的窗口：后端拒绝，这里只记日志（入口已按状态收窄）
    splitActions.add.mutate(key, { onError: (e) => console.error('[split] 加入分屏失败', e) });
    setMenu(null);
  };
  /** 分屏移出：把该应用在分屏里的那扇窗口移出并还原到加入前的位置 */
  const splitRemove = (key: string) => {
    const m = split.data?.members.find((x) => x.appKey === key);
    if (m) splitActions.remove.mutate(m.windowId);
    setMenu(null);
  };

  /** 任务栏定制组的增删（主屏编辑模式之外的第二入口） */
  const removeDockItem = (key: string) => {
    const idx = layout.dock.indexOf(key);
    if (idx >= 0) commit(removeFromDock(layout, idx));
    setMenu(null);
    syncLayout();
  };
  /** 悬停窗口卡片：停 350ms 拉窗口清单，≥2 扇才出卡（扫过图标不弹）。
   *  任务栏窗口一条高 → 独立置顶弹窗承载；主窗口/胶囊 → 本地渲染 */
  const peekTimer = useRef<number | undefined>(undefined);
  const peekClose = useRef<number | undefined>(undefined);
  const beginPeek = (key: string, x: number) => {
    window.clearTimeout(peekTimer.current);
    window.clearTimeout(peekClose.current);
    peekTimer.current = window.setTimeout(() => {
      if (menuKeyRef.current || popupKindRef.current === 'menu') return; // 右键菜单已开：不弹卡
      qc.fetchQuery({
        queryKey: ['apps', 'windows', key],
        queryFn: () => commands.appWindows(key),
        staleTime: 1500,
      })
        .then((windows) => {
          // 查询在途期间用户可能已右键：菜单优先，放弃弹卡；
          // ≥1 扇窗口就出卡——单窗口应用也提供前置 + 行尾 X 快捷关闭
          if (menuKeyRef.current || popupKindRef.current === 'menu' || windows.length < 1)
            return;
          if (inTaskbarWindow) {
            commands
              .dockMenuOpen({
                kind: 'hover',
                appKey: key,
                x,
                removable: false,
                running: true,
                multi: true,
                // 悬停卡片不含分屏项（它是「挑一扇窗口前置」的菜单）
                splitAvailable: false,
                splitMember: false,
                splitFull: false,
                splitCount: 0,
                windows,
              })
              .catch(console.error);
          } else {
            setPeek({ key, x, windows });
          }
        })
        .catch(() => {/* 索引外 key 等错误：静默不出卡 */});
    }, 350);
  };
  const endPeek = () => {
    window.clearTimeout(peekTimer.current);
    if (inTaskbarWindow) {
      // 卡片自己监听该事件：鼠标没进卡片时宽限后收回
      emit('hamster:dock-hover-leave', {}).catch(console.error);
    } else {
      // 宽限 250ms：鼠标从图标移进卡片不打断
      peekClose.current = window.setTimeout(() => setPeek(null), 250);
    }
  };
  const keepPeek = () => window.clearTimeout(peekClose.current);
  /** 卡片行 X：关单扇窗口 → 稍候重取（WM_CLOSE 异步，应用可能弹确认）→
   *  更新卡片，窗口关光就收卡 */
  const closePeekWindow = (key: string, x: number, id: number) => {
    commands.appWindowClose(id).catch(console.error);
    qc.invalidateQueries({ queryKey: ['apps', 'running'] });
    window.setTimeout(() => {
      qc.fetchQuery({
        queryKey: ['apps', 'windows', key],
        queryFn: () => commands.appWindows(key),
        staleTime: 0,
      })
        .then((ws) => setPeek(ws.length ? { key, x, windows: ws } : null))
        .catch(console.error);
    }, 450);
  };

  /** 右键呼出应用菜单：任务栏独立窗口一条高放不下纵向菜单，弹独立置顶
   *  小窗（载荷 + 定位后端算）；主窗口/胶囊内空间充足，本地渲染 */
  const openContextMenu = (key: string, x: number, removable: boolean) => {
    // 右键优先：掐掉悬停探测与宽限收卡，已开的悬停卡片立即让位
    window.clearTimeout(peekTimer.current);
    window.clearTimeout(peekClose.current);
    setPeek(null);
    if (inTaskbarWindow) {
      commands
        .dockMenuOpen({
          kind: 'menu',
          appKey: key,
          x,
          removable,
          running: runningKeys.has(key),
          multi: !singleApps.has(key),
          // 分屏那一行：桌面接管态 + 应用在运行才给；已在分屏里就显示「移出」，
          // 满了显示禁用行说明原因（不做静默隐藏，否则用户以为功能丢了）
          splitAvailable:
            !!desktop && runningKeys.has(key) && (splitMembers.has(key) || !splitFull),
          splitMember: splitMembers.has(key),
          splitFull,
          splitCount: 0,
          windows: [],
        })
        .catch(console.error);
    } else {
      setMenu({ key, x, removable });
    }
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
  /** 分屏胶囊：任务栏窗弹独立置顶弹层（一条高放不下成员列表）；浏览器预览/主窗口内本地浮层 */
  const openSplitPanel = (x: number) => {
    if (inTaskbarWindow) commands.splitMenuOpen(x, splitCount).catch(console.error);
    else setSplitPanelOpen(true);
  };

  // 本地分屏浮层：点外面/Esc 收回（浮层自身 stopPropagation，不会误关）
  useEffect(() => {
    if (!splitPanelOpen) return;
    const close = () => setSplitPanelOpen(false);
    window.addEventListener('pointerdown', close);
    window.addEventListener('keydown', close);
    return () => {
      window.removeEventListener('pointerdown', close);
      window.removeEventListener('keydown', close);
    };
  }, [splitPanelOpen]);

  // 点击菜单外 / Esc 关闭
  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    const esc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setMenu(null);
    };
    window.addEventListener('pointerdown', close);
    window.addEventListener('keydown', esc);
    return () => {
      window.removeEventListener('pointerdown', close);
      window.removeEventListener('keydown', esc);
    };
  }, [menu]);

  // 任务栏窗口任意点击收回菜单弹窗：任务栏不抢焦点，点它不会让弹窗失焦
  useEffect(() => {
    if (!inTaskbarWindow) return;
    const hide = () => commands.dockMenuHide().catch(console.error);
    window.addEventListener('pointerdown', hide);
    return () => window.removeEventListener('pointerdown', hide);
  }, [inTaskbarWindow]);
  // 任务栏窗口的右键菜单走独立弹窗（本地 menu state 不参与），
  // 弹窗状态由 Rust 广播到这里：菜单开着 = 悬停探测全部抑制
  const popupKindRef = useRef<string | null>(null);
  useEffect(() => {
    if (!inTaskbarWindow) return;
    let un: (() => void) | undefined;
    listen<{ kind: string | null }>('hamster:dock-popup-state', (e) => {
      popupKindRef.current = e.payload.kind ?? null;
    })
      .then((fn) => (un = fn))
      .catch(console.error);
    return () => un?.();
  }, [inTaskbarWindow]);

  // 弹窗菜单动作回传：渲染了 DockBar 的窗口执行（remove 需要 layout commit；
  // close/new 顺带失效本窗口查询，运行指示点即时变化）。弹窗自身只广播不执行。
  // 桌面接管 = 任务栏窗执行（主窗口不渲染 DockBar）；窗口化 = 主窗口执行
  // ——两形态互斥不会双执行，任务栏窗反而不能排除（否则桌面模式下动作没人接）。
  // 监听器必须挂载一次：top 每 30s 轮询 + 每次启动都失效重取，若依赖
  // [layout, top] 反复重挂，退订又是 listen() 异步赋值——竞态漏退订会让
  // 监听器叠加，一次「多开」被放大成 N 个窗口。handler 走 ref 取最新。
  const actionHandlers = useRef({ closeApp, launchNew, removeDockItem, splitAdd, splitRemove });
  actionHandlers.current = { closeApp, launchNew, removeDockItem, splitAdd, splitRemove };
  useEffect(() => {
    if (!isTauri) return;
    let un: (() => void) | undefined;
    let disposed = false;
    let lastKey = '';
    let lastAt = 0;
    listen<{ action: string; key: string }>('hamster:dock-menu-action', (e) => {
      const { action, key } = e.payload;
      // 同一应用 400ms 内的重复事件只执行一次（监听器叠加的兜底保险）
      const now = Date.now();
      if (key === lastKey && now - lastAt < 400) return;
      lastKey = key;
      lastAt = now;
      const h = actionHandlers.current;
      if (action === 'close') h.closeApp(key);
      else if (action === 'new') h.launchNew(key);
      else if (action === 'remove') h.removeDockItem(key);
      else if (action === 'split-add') h.splitAdd(key);
      else if (action === 'split-remove') h.splitRemove(key);
    })
      .then((fn) => {
        // cleanup 先于 promise 到达：立即退订，杜绝泄漏
        if (disposed) fn();
        else un = fn;
      })
      .catch(console.error);
    return () => {
      disposed = true;
      un?.();
    };
  }, []);

  if (desktop) {
    // 贴边通栏任务栏（完全替代系统任务栏）：左端入口 + 定制（可增删）…… 右端退出 + 时钟
    // 真任务栏窗口（TaskbarPage 已画条底色）：不带 ios-dock 皮肤——bar 里套 bar
    // 会在窗口顶部夹出露壁纸缝 + 双重高光线；浏览器预览/主窗口内嵌仍需要皮肤
    const barCls = inTaskbarWindow
      ? 'flex w-full items-end gap-2.5 rounded-none px-4 pb-[7px] pt-0'
      : 'ios-dock flex w-full items-end gap-2.5 rounded-none px-4 py-[7px]';
    return (
      <div className="relative z-30 flex shrink-0">
        <div className={barCls}>
          {/* 开始：Windows 徽标位（唤出系统真实开始菜单，非仿制面板） */}
          <DockItem label={t('chrome.action.start')} nativeTipOnly={desktop} onClick={openStartMenu}>
            <StartLogo size={size} />
          </DockItem>
          {renderSystem(true)}
          {/* 快速回到桌面主页：主窗口常被其他应用盖住，任务栏是唯一常驻入口
              （真任务栏窗走 emitTo 让主窗口导航 + 前置） */}
          <DockItem label={t('chrome.shell.backToDesktop')} nativeTipOnly={desktop} onClick={() => goRoute('/')}>
            <span
              className="squircle grid place-items-center bg-orange-500/45 text-white ring-1 ring-white/25"
              style={{ width: size, height: size }}
            >
              <House size={Math.round(size * 0.55)} strokeWidth={1.8} />
            </span>
          </DockItem>
          {layout.dock.length > 0 && <Divider />}
          {layout.dock.map((key) => {
            const a = appByKey.get(key);
            if (!a) return null;
            return (
              <AppDockItem
                key={key}
                app={a}
                size={size}
                nativeTipOnly={desktop}
                running={runningKeys.has(key)}
                onLaunch={launchApp}
                onPeek={beginPeek}
                onPeekEnd={endPeek}
                onContextMenu={(x) => openContextMenu(key, x, true)}
              />
            );
          })}
          {/* 添加定制应用（主窗口弹选择器） */}
          {layout.dock.length < DOCK_CAPACITY && (
            <button
              onClick={openPicker}
              title={t('chrome.action.addAppToDock')}
              aria-label={t('chrome.action.addAppToDock')}
              className="group grid size-[44px] place-items-center rounded-xl bg-white/[0.08] text-white/55 ring-1 ring-white/12 transition-all hover:bg-white/16 hover:text-white/90"
            >
              <Plus size={19} />
            </button>
          )}
          {frequent.length > 0 && <Divider />}
          {frequent.map((a) => (
            <AppDockItem
              key={a.app_key}
              app={a}
              size={size}
              nativeTipOnly={desktop}
              running={runningKeys.has(a.app_key)}
              onLaunch={launchApp}
              onPeek={beginPeek}
              onPeekEnd={endPeek}
              onContextMenu={(x) => openContextMenu(a.app_key, x, false)}
            />
          ))}
          <div className="ml-auto flex items-end gap-2 pl-3">
            {/* 分屏状态胶囊（没开分屏时自身不渲染）：点开管理，✕ 一键退出 */}
            <SplitPill
              state={split.data}
              onManage={openSplitPanel}
              onExit={() => splitActions.exit.mutate()}
            />
            {/* 设置入口（右端）：与胶囊系统组末位同一渲染 */}
            <DockItem
              label={t('chrome.nav.settings')}
              active={pathname === '/settings'}
              nativeTipOnly={desktop}
              onClick={() => goRoute('/settings')}
            >
              <SystemTile Icon={Settings} size={size} />
            </DockItem>
            <Divider />
            <DockItem label={t('chrome.action.exitDesktop')} nativeTipOnly={desktop} onClick={exitDesktop}>
              <span
                className="squircle grid place-items-center bg-red-500/45 text-white ring-1 ring-white/25"
                style={{ width: size, height: size }}
              >
                <LogOut size={Math.round(size * 0.55)} strokeWidth={1.8} />
              </span>
            </DockItem>
            <Divider />
            {/* 托盘按钮：主窗口弹后台应用面板（跨窗口走 emitTo） */}
            <TrayButton />
            <div className="flex flex-col items-end justify-center self-stretch px-1.5 leading-tight">
              <span className="text-[13px] font-medium tabular-nums text-white [text-shadow:0_1px_3px_rgba(0,0,0,.45)]">
                {now.time}
              </span>
              <span className="text-[10px] text-white/70">{now.date}</span>
            </div>
          </div>
        </div>

        {/* 右键菜单：关闭窗口（运行中）/ 多开 / 分屏添加或移出（接管态）/ 移除（定制组）。
            任务栏独立窗口 72px 高 → 横向贴顶胶囊；主窗口内 → 纵向弹上方 */}
        {menu && (
          <DockAppMenu
            x={menu.x}
            running={runningKeys.has(menu.key)}
            removable={menu.removable}
            multiCapable={!singleApps.has(menu.key)}
            splitEnabled={
              !!desktop &&
              runningKeys.has(menu.key) &&
              (splitMembers.has(menu.key) || !splitFull)
            }
            splitMember={splitMembers.has(menu.key)}
            splitFull={splitFull}
            onCloseApp={() => closeApp(menu.key)}
            onNewInstance={() => launchNew(menu.key)}
            onRemove={() => removeDockItem(menu.key)}
            onSplitAdd={() => splitAdd(menu.key)}
            onSplitRemove={() => splitRemove(menu.key)}
          />
        )}
        {/* 悬停窗口卡片（浏览器预览/主窗口内嵌形态；真任务栏窗走独立弹窗） */}
        {peek && (
          <DockWindowsCard
            x={peek.x}
            windows={peek.windows}
            onMouseEnter={keepPeek}
            onMouseLeave={() => setPeek(null)}
            onCloseWindow={(id) => closePeekWindow(peek.key, peek.x, id)}
            onActivate={(id) => {
              commands.appWindowActivate(id).catch(console.error);
              setPeek(null);
            }}
          />
        )}
        {/* 分屏管理浮层（浏览器预览/主窗口内嵌形态；真任务栏窗走独立置顶弹窗） */}
        {splitPanelOpen && (
          <SplitPanel initialCount={splitCount} onDismiss={() => setSplitPanelOpen(false)} />
        )}
      </div>
    );
  }

  // 窗口化：居中胶囊
  const customGroup = layout.dock.map((key) => {
    const a = appByKey.get(key);
    return a ? (
      <AppDockItem
        key={key}
        app={a}
        size={size}
        running={runningKeys.has(key)}
        onLaunch={launchApp}
        onPeek={beginPeek}
        onPeekEnd={endPeek}
        onContextMenu={(x) => openContextMenu(key, x, true)}
      />
    ) : null;
  });
  const frequentGroup = frequent.map(
    (a) => (
      <AppDockItem
        key={a.app_key}
        app={a}
        size={size}
        running={runningKeys.has(a.app_key)}
        onLaunch={launchApp}
        onPeek={beginPeek}
        onPeekEnd={endPeek}
        onContextMenu={(x) => openContextMenu(a.app_key, x, false)}
      />
    ),
  );
  const groups = [
    { key: 'custom', nodes: customGroup },
    { key: 'frequent', nodes: frequentGroup },
    { key: 'system', nodes: renderSystem(false) },
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
      {/* 右键菜单：胶囊空间充足，纵向弹在图标上方 */}
      {menu && (
        <DockAppMenu
          x={menu.x}
          running={runningKeys.has(menu.key)}
          removable={menu.removable}
          multiCapable={!singleApps.has(menu.key)}
          onCloseApp={() => closeApp(menu.key)}
          onNewInstance={() => launchNew(menu.key)}
          onRemove={() => removeDockItem(menu.key)}
        />
      )}
      {/* 悬停窗口卡片：胶囊空间充足，本地渲染弹在图标上方 */}
      {peek && (
        <DockWindowsCard
          x={peek.x}
          windows={peek.windows}
          onMouseEnter={keepPeek}
          onMouseLeave={() => setPeek(null)}
          onCloseWindow={(id) => closePeekWindow(peek.key, peek.x, id)}
          onActivate={(id) => {
            commands.appWindowActivate(id).catch(console.error);
            setPeek(null);
          }}
        />
      )}
    </div>
  );
}

/** 应用图标 Dock 项：真图标 PNG / 字母占位，单击启动（顺手刷新常用组），
 * 运行中点亮底部指示点；右键呼出应用菜单（关闭/多开/移除）；
 * 悬停（仅运行中）触发窗口卡片探测（≥2 扇窗口才出卡） */
function AppDockItem({
  app,
  size,
  running = false,
  onLaunch,
  onPeek,
  onPeekEnd,
  onContextMenu,
  nativeTipOnly = false,
}: {
  app: AppEntry;
  size: number;
  running?: boolean;
  onLaunch: (key: string) => void;
  /** 悬停窗口卡片：进入图标（运行中才调，传图标中心 x） */
  onPeek?: (key: string, x: number) => void;
  /** 鼠标离开图标 */
  onPeekEnd?: () => void;
  /** 右键呼出应用菜单（传窗口内 clientX） */
  onContextMenu?: (x: number) => void;
  nativeTipOnly?: boolean;
}) {
  return (
    <DockItem
      label={app.display_name}
      running={running}
      nativeTipOnly={nativeTipOnly}
      onClick={() => onLaunch(app.app_key)}
      onMouseEnter={
        onPeek && running
          ? (e) => {
              const r = e.currentTarget.getBoundingClientRect();
              onPeek(app.app_key, (r.left + r.right) / 2);
            }
          : undefined
      }
      onMouseLeave={onPeekEnd}
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

/** Windows 风格四格开始徽标（入口是系统真实开始菜单，非应用内仿制面板） */
function StartLogo({ size }: { size: number }) {
  return (
    <span
      className="squircle grid place-items-center bg-white/[0.16] ring-1 ring-white/22 backdrop-blur-xl drop-shadow-[0_4px_10px_rgba(0,0,0,.35)]"
      style={{ width: size, height: size }}
    >
      <svg width={Math.round(size * 0.5)} height={Math.round(size * 0.5)} viewBox="0 0 22 22" fill="none" aria-hidden>
        <rect x="2" y="2" width="8" height="8" rx="1.4" fill="#4CA8EF" />
        <rect x="12" y="2" width="8" height="8" rx="1.4" fill="#4CA8EF" />
        <rect x="2" y="12" width="8" height="8" rx="1.4" fill="#4CA8EF" />
        <rect x="12" y="12" width="8" height="8" rx="1.4" fill="#4CA8EF" />
      </svg>
    </span>
  );
}

/** Dock 通用项：tooltip + 悬停放大上浮 + 底部活动指示点（macOS 运行指示）。
 * 指示点点亮条件：active（系统入口 = 当前路由）或 running（应用图标 = 进程在跑，
 * 解决 Windows 任务栏 pin 后看不出开没开的诟病）。
 * nativeTipOnly：桌面任务栏条内不渲染自绘 tooltip（超出窗口上边界会被裁剪，
 * 交给按钮原生 title 弹系统提示）；窗口化胶囊内空间充足，仍用自绘样式。 */
function DockItem({
  label,
  onClick,
  active,
  running = false,
  onContextMenu,
  onMouseEnter,
  onMouseLeave,
  nativeTipOnly = false,
  children,
}: {
  label: string;
  onClick: () => void;
  active?: boolean;
  running?: boolean;
  onContextMenu?: (e: React.MouseEvent) => void;
  onMouseEnter?: (e: React.MouseEvent) => void;
  onMouseLeave?: () => void;
  nativeTipOnly?: boolean;
  children: ReactNode;
}) {
  return (
    <div
      className="group relative flex flex-col items-center"
      onContextMenu={onContextMenu}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
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
      {/* 指示点不占布局流，图标保持垂直居中；运行中常亮，悬停微亮 */}
      <span
        aria-hidden
        className={`absolute -bottom-[5px] left-1/2 h-[5px] w-[5px] -translate-x-1/2 rounded-full bg-white shadow-[0_0_6px_rgba(255,255,255,.9)] transition-opacity ${
          active || running ? 'opacity-100' : 'opacity-0 group-hover:opacity-45'
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
  const date = new Intl.DateTimeFormat(getLang() === 'en' ? 'en-US' : 'zh-CN', {
    month: 'long',
    day: 'numeric',
    weekday: 'long',
  }).format(now);
  return { time, date };
}
