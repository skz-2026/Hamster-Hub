/**
 * dock 右键菜单独立弹窗的内容体（路由 /dock-menu，由 Rust dock_menu_open
 * 定位显示）。载荷拉取 + 事件增量更新 + 动作广播给主窗口执行 + 失焦/Esc 收回。
 */
import { useEffect, useState } from 'react';
import { emit, listen } from '@tauri-apps/api/event';
import { commands, type DockMenuPayload } from '@/shared/lib/ipc';
import DockAppMenu from './DockAppMenu';

export default function DockMenuWindow() {
  const [payload, setPayload] = useState<DockMenuPayload | null>(null);

  useEffect(() => {
    // 拉取 + focus/可见时重拉兜底：弹窗常驻隐藏、挂载早于首次右键，
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
    listen<DockMenuPayload>('hamster:dock-menu-payload', (e) => setPayload(e.payload))
      .then((fn) => (un = fn))
      .catch(console.error);
    // 任务栏窗不抢焦点，点它不会让本窗失焦——收回也由任务栏侧主动触发
    const hide = () => commands.dockMenuHide().catch(console.error);
    window.addEventListener('blur', hide);
    const esc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') hide();
    };
    window.addEventListener('keydown', esc);
    return () => {
      un?.();
      window.removeEventListener('blur', hide);
      window.removeEventListener('keydown', esc);
      window.removeEventListener('focus', refresh);
      document.removeEventListener('visibilitychange', onVis);
    };
  }, []);

  if (!payload) return null;

  /** 动作广播给主窗口执行（remove 需 layout commit；close/new 失效主窗口查询） */
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
        onNewInstance={() => act('new')}
        onCloseApp={() => act('close')}
        onRemove={() => act('remove')}
      />
    </div>
  );
}
