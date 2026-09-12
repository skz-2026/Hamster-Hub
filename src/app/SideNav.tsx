import { useState } from 'react';
import { NavLink, useNavigate } from 'react-router-dom';
import {
  Bot,
  LayoutDashboard,
  MonitorSmartphone,
  Search,
  CalendarDays,
  LayoutGrid,
  FolderOpen,
  Settings,
  Smartphone,
} from 'lucide-react';
import { commands } from '@/shared/lib/ipc';

/** 上组导航首位：工作台（主屏按钮紧随其后，见 HomeScreenBtn） */
const NAV_TOP = [{ to: '/', label: '工作台', icon: LayoutDashboard }] as const;

const NAV_REST = [
  { to: '/search', label: '搜索', icon: Search },
  { to: '/schedule', label: '日程', icon: CalendarDays },
  { to: '/apps', label: '应用', icon: LayoutGrid },
  { to: '/files', label: '文件', icon: FolderOpen },
] as const;

/** 左下角常驻入口（Dock 位）：桌面快捷进入 + 代理工作台 + 设置 */
const BOTTOM_NAV = [{ to: '/bench', label: '代理', icon: Bot }] as const;

const NAV_BTN_CLS = (isActive: boolean) =>
  `flex w-[60px] flex-col items-center gap-1 rounded-xl px-1 py-2 text-[11px] transition-colors ${
    isActive
      ? 'bg-[var(--accent-weak)] text-[var(--accent)]'
      : 'text-[var(--text-muted)] hover:bg-[var(--hover)] hover:text-[var(--text)]'
  }`;

/** 左侧导航（窗口化模式）；桌面模式为沉浸全屏桌面，不渲染侧栏 */
export function SideNav() {
  return (
    <nav className="flex w-[76px] shrink-0 flex-col items-center gap-1 py-3">
      {NAV_TOP.map(({ to, label, icon: Icon }) => (
        <NavLink key={to} to={to} title={label} className={({ isActive }) => NAV_BTN_CLS(isActive)}>
          <Icon size={20} strokeWidth={1.8} />
          {label}
        </NavLink>
      ))}
      <HomeScreenBtn />
      {NAV_REST.map(({ to, label, icon: Icon }) => (
        <NavLink key={to} to={to} title={label} className={({ isActive }) => NAV_BTN_CLS(isActive)}>
          <Icon size={20} strokeWidth={1.8} />
          {label}
        </NavLink>
      ))}

      <div className="mt-auto flex flex-col items-center gap-1">
        <DesktopEnterBtn />
        {BOTTOM_NAV.map(({ to, label, icon: Icon }) => (
          <NavLink key={to} to={to} title={label} className={({ isActive }) => NAV_BTN_CLS(isActive)}>
            <Icon size={20} strokeWidth={1.8} />
            {label}
          </NavLink>
        ))}
        <NavLink to="/settings" title="设置" className={({ isActive }) => NAV_BTN_CLS(isActive)}>
          <Settings size={20} strokeWidth={1.8} />
          设置
        </NavLink>
      </div>
    </nav>
  );
}

/**
 * 主屏入口：主屏是桌面接管态里的层，未接管直接去 /home 会被 HomeScreen 弹回工作台。
 * 这里点一下 = 自动进入桌面接管 → 直接落到主屏（AppShell 事件对 /home 不劫持）。
 */
function HomeScreenBtn() {
  const navigate = useNavigate();
  const [busy, setBusy] = useState(false);
  const go = async () => {
    if (busy) return;
    setBusy(true);
    try {
      const active = await commands.desktopModeIsActive().catch(() => false);
      if (!active) await commands.desktopModeEnter();
      navigate('/home');
    } catch (e) {
      console.error('[sidenav] 进入主屏失败', e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <button onClick={go} title="主屏" className={NAV_BTN_CLS(false)} disabled={busy}>
      <Smartphone size={20} strokeWidth={1.8} />
      主屏
    </button>
  );
}

/** 桌面模式快捷进入（不用绕设置页；点击后整屏切到 /desktop，本按钮随之消失） */
function DesktopEnterBtn() {
  const [busy, setBusy] = useState(false);
  const enter = () => {
    if (busy) return;
    setBusy(true);
    commands
      .desktopModeEnter()
      .catch(console.error)
      .finally(() => setBusy(false));
  };
  return (
    <button
      onClick={enter}
      title="进入桌面模式"
      className="flex w-[60px] flex-col items-center gap-1 rounded-xl bg-[var(--accent-weak)] px-1 py-2 text-[11px] text-[var(--accent)] transition-colors hover:brightness-110 disabled:opacity-60"
      disabled={busy}
    >
      <MonitorSmartphone size={20} strokeWidth={1.8} />
      桌面
    </button>
  );
}
