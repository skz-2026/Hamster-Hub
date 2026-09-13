import { useState } from 'react';
import { NavLink, useNavigate } from 'react-router-dom';
import {
  Bot,
  KeyRound,
  LayoutDashboard,
  Search,
  CalendarDays,
  LayoutGrid,
  FolderOpen,
  Settings,
  Smartphone,
} from 'lucide-react';
import { commands } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';

/** 上组导航首位：工作台（主屏按钮紧随其后，见 HomeScreenBtn） */
const NAV_TOP = [{ to: '/', labelKey: 'chrome.nav.workbench', icon: LayoutDashboard }] as const;

const NAV_REST = [
  { to: '/search', labelKey: 'chrome.nav.search', icon: Search },
  { to: '/schedule', labelKey: 'chrome.nav.schedule', icon: CalendarDays },
  { to: '/apps', labelKey: 'chrome.nav.apps', icon: LayoutGrid },
  { to: '/files', labelKey: 'chrome.nav.files', icon: FolderOpen },
  { to: '/vault', labelKey: 'chrome.nav.vault', icon: KeyRound },
] as const;

/** 左下角常驻入口（Dock 位）：代理工作台 + 设置 */
const BOTTOM_NAV = [{ to: '/bench', labelKey: 'chrome.nav.agent', icon: Bot }] as const;

const NAV_BTN_CLS = (isActive: boolean) =>
  `flex w-[60px] flex-col items-center gap-1 rounded-xl px-1 py-2 text-[11px] transition-colors ${
    isActive
      ? 'bg-[var(--accent-weak)] text-[var(--accent)]'
      : 'text-[var(--text-muted)] hover:bg-[var(--hover)] hover:text-[var(--text)]'
  }`;

/** 左侧导航（窗口化模式）；桌面模式为沉浸全屏桌面，不渲染侧栏 */
export function SideNav() {
  const { t } = useI18n();
  return (
    <nav className="flex w-[76px] shrink-0 flex-col items-center gap-1 py-3">
      {NAV_TOP.map(({ to, labelKey, icon: Icon }) => (
        <NavLink key={to} to={to} title={t(labelKey)} className={({ isActive }) => NAV_BTN_CLS(isActive)}>
          <Icon size={20} strokeWidth={1.8} />
          {t(labelKey)}
        </NavLink>
      ))}
      <HomeScreenBtn />
      {NAV_REST.map(({ to, labelKey, icon: Icon }) => (
        <NavLink key={to} to={to} title={t(labelKey)} className={({ isActive }) => NAV_BTN_CLS(isActive)}>
          <Icon size={20} strokeWidth={1.8} />
          {t(labelKey)}
        </NavLink>
      ))}

      <div className="mt-auto flex flex-col items-center gap-1">
        {BOTTOM_NAV.map(({ to, labelKey, icon: Icon }) => (
          <NavLink key={to} to={to} title={t(labelKey)} className={({ isActive }) => NAV_BTN_CLS(isActive)}>
            <Icon size={20} strokeWidth={1.8} />
            {t(labelKey)}
          </NavLink>
        ))}
        <NavLink to="/settings" title={t('chrome.nav.settings')} className={({ isActive }) => NAV_BTN_CLS(isActive)}>
          <Settings size={20} strokeWidth={1.8} />
          {t('chrome.nav.settings')}
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
  const { t } = useI18n();
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
    <button onClick={go} title={t('chrome.nav.home')} className={NAV_BTN_CLS(false)} disabled={busy}>
      <Smartphone size={20} strokeWidth={1.8} />
      {t('chrome.nav.home')}
    </button>
  );
}
