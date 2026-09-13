/**
 * dock 弹窗的内容体（路由 /dock-menu，由 Rust dock_menu_open 定位显示）。
 * kind = menu：右键菜单（载荷拉取 + 事件增量 + 动作广播给渲染 DockBar 的
 * 窗口执行 + 失焦/Esc 收回）；kind = hover：悬停窗口卡片（点行直接
 * app_window_activate，鼠标离开/任务栏图标离开收回，不抢焦点）。
 */
import { useEffect, useRef, useState } from 'react';
import { emit, listen } from '@tauri-apps/api/event';
import { commands, type DockMenuPayload } from '@/shared/lib/ipc';
import DockAppMenu from './DockAppMenu';
import DockWindowsCard from './DockWindowsCard';

export default function DockMenuWindow() {
  const [payload, setPayload] = useState<DockMenuPayload | null>(null);
  // kind 的 ref 镜像：hover-leave 监听器挂载一次，回调里需读最新 kind——
  // 右键菜单打开时鼠标离开图标的事件不得把菜单当悬停卡片收掉
  const kindRef = useRef<string | null>(null);
  kindRef.current = payload?.kind ?? null;
  // hover 卡片待收计时（图标离开/卡片离开触发；进卡/换目标取消）
  const hoverClose = useRef<number | undefined>(undefined);
  const cancelHoverClose = () => window.clearTimeout(hoverClose.current);
  const armHoverClose = (ms: number) => {
    cancelHoverClose();
    hoverClose.current = window.setTimeout(() => {
      commands.dockMenuHide().catch(console.error);
    }, ms);
  };

  useEffect(() => {
    // 拉取 + focus/可见时重拉兜底：弹窗常驻隐藏、挂载早于首次打开，
    // 事件推送万一丢失（能力/时序），show + focus 触发的重拉保证必渲染
    const refresh = () =>
      commands.dockMenuPayload().then(setPayload).catch(console.error);
    refresh();
    window.addEventListener('focus', refresh);
    const onVis = () => {
      if (!document.hidden) refresh();
    };
    document.addEventListener('visibilitychange', onVis);
    let un: (() => void) | undefined;
    listen<DockMenuPayload>('hamster:dock-menu-payload', (e) => {
      cancelHoverClose(); // 换目标重开：待收计时作废
      setPayload(e.payload);
    })
      .then((fn) => (un = fn))
      .catch(console.error);
    // 任务栏窗不抢焦点，点它不会让本窗失焦——收回也由任务栏侧主动触发
    const hide = () => commands.dockMenuHide().catch(console.error);
    window.addEventListener('blur', hide);
    const esc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') hide();
    };
    window.addEventListener('keydown', esc);
    // 任务栏图标鼠标离开：鼠标若没进卡片，短暂宽限后收回
    let unLeave: (() => void) | undefined;
    listen('hamster:dock-hover-leave', () => {
      if (kindRef.current === 'hover') armHoverClose(350);
    })
      .then((fn) => (unLeave = fn))
      .catch(console.error);
    return () => {
      un?.();
      unLeave?.();
      cancelHoverClose();
      window.removeEventListener('blur', hide);
      window.removeEventListener('keydown', esc);
      window.removeEventListener('focus', refresh);
      document.removeEventListener('visibilitychange', onVis);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!payload) return null;

  if (payload.kind === 'hover') {
    return (
      <div
        className="h-screen w-full"
        onMouseEnter={cancelHoverClose}
        onMouseLeave={() => armHoverClose(200)}
      >
        <DockWindowsCard
          fill
          windows={payload.windows}
          onActivate={(id) => {
            commands.appWindowActivate(id).catch(console.error);
            commands.dockMenuHide().catch(console.error);
          }}
        />
      </div>
    );
  }

  /** 动作广播给渲染 DockBar 的窗口执行（remove 需 layout commit；close/new 失效其查询） */
  const act = (action: 'close' | 'new' | 'remove') => {
    emit('hamster:dock-menu-action', { action, key: payload.appKey }).catch(console.error);
    commands.dockMenuHide().catch(console.error);
  };

  return (
    <div className="h-screen w-full">
      <DockAppMenu
        fill
        running={payload.running}
        removable={payload.removable}
        multiCapable={payload.multi}
        onNewInstance={() => act('new')}
        onCloseApp={() => act('close')}
        onRemove={() => act('remove')}
      />
    </div>
  );
}
