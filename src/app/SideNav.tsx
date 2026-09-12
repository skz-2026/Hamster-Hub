import { NavLink } from 'react-router-dom';
import {
  Bot,
  LayoutDashboard,
  Search,
  CalendarDays,
  LayoutGrid,
  FolderOpen,
  Settings,
  Smartphone,
} from 'lucide-react';

const NAV = [
  { to: '/', label: '工作台', icon: LayoutDashboard },
  { to: '/home', label: '主屏', icon: Smartphone },
  { to: '/search', label: '搜索', icon: Search },
  { to: '/schedule', label: '日程', icon: CalendarDays },
  { to: '/apps', label: '应用', icon: LayoutGrid },
  { to: '/files', label: '文件', icon: FolderOpen },
] as const;

/** 左下角常驻入口（Dock 位）：代理工作台 + 设置 */
const BOTTOM_NAV = [
  { to: '/bench', label: '代理', icon: Bot },
] as const;

/** 左侧导航（窗口化模式）；桌面模式为沉浸全屏桌面，不渲染侧栏 */
export function SideNav() {
  return (
    <nav className="flex w-[76px] shrink-0 flex-col items-center gap-1 py-3">
      {NAV.map(({ to, label, icon: Icon }) => (
        <NavLink
          key={to}
          to={to}
          title={label}
          className={({ isActive }) =>
            `flex w-[60px] flex-col items-center gap-1 rounded-xl px-1 py-2 text-[11px] transition-colors ${
              isActive
                ? 'bg-[var(--accent-weak)] text-[var(--accent)]'
                : 'text-[var(--text-muted)] hover:bg-[var(--hover)] hover:text-[var(--text)]'
            }`
          }
        >
          <Icon size={20} strokeWidth={1.8} />
          {label}
        </NavLink>
      ))}

      <div className="mt-auto flex flex-col items-center gap-1">
        {BOTTOM_NAV.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            title={label}
            className={({ isActive }) =>
              `flex w-[60px] flex-col items-center gap-1 rounded-xl px-1 py-2 text-[11px] transition-colors ${
                isActive
                  ? 'bg-[var(--accent-weak)] text-[var(--accent)]'
                  : 'text-[var(--text-muted)] hover:bg-[var(--hover)] hover:text-[var(--text)]'
              }`
            }
          >
            <Icon size={20} strokeWidth={1.8} />
            {label}
          </NavLink>
        ))}
        <NavLink
          to="/settings"
          title="设置"
          className={({ isActive }) =>
            `flex w-[60px] flex-col items-center gap-1 rounded-xl px-1 py-2 text-[11px] transition-colors ${
              isActive
                ? 'bg-[var(--accent-weak)] text-[var(--accent)]'
                : 'text-[var(--text-muted)] hover:bg-[var(--hover)] hover:text-[var(--text)]'
            }`
          }
        >
          <Settings size={20} strokeWidth={1.8} />
          设置
        </NavLink>
      </div>
    </nav>
  );
}
