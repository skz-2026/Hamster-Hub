import { Navigate, Route, Routes } from 'react-router-dom';
import { AppShell } from '@/app/AppShell';
import HomePage from '@/app/routes/home';
import SpotlightPage from '@/app/routes/spotlight';
import TaskbarPage from '@/app/routes/taskbar';
import DockMenuPage from '@/app/routes/dock-menu';
import AgentPage from '@/app/routes/agent';
import BenchPage from '@/app/routes/bench';
import WorkbenchPage from '@/app/routes/workbench';
import SearchPage from '@/app/routes/search';
import SchedulePage from '@/app/routes/schedule';
import AppsPage from '@/app/routes/apps';
import FilesPage from '@/app/routes/files';
import VaultPage from '@/app/routes/vault';
import SettingsPage from '@/app/routes/settings';

export function AppRoutes() {
  return (
    <Routes>
      {/* Spotlight 独立覆盖层窗口（AppShell 之外，无应用壳） */}
      <Route path="/spotlight" element={<SpotlightPage />} />
      {/* 任务栏独立置顶窗口（桌面接管时贴屏幕底部条） */}
      <Route path="/taskbar" element={<TaskbarPage />} />
      {/* dock 右键菜单独立置顶弹窗（桌面接管时任务栏条内右键弹出） */}
      <Route path="/dock-menu" element={<DockMenuPage />} />
      <Route element={<AppShell />}>
        <Route path="/home" element={<HomePage />} />
        <Route path="/agent" element={<AgentPage />} />
        <Route path="/bench" element={<BenchPage />} />
        {/* 首页（工作台 × 桌面主页合一）：窗口化/接管两形态共用，见 WorkbenchPage */}
        <Route path="/" element={<WorkbenchPage />} />
        <Route path="/search" element={<SearchPage />} />
        <Route path="/schedule" element={<SchedulePage />} />
        <Route path="/apps" element={<AppsPage />} />
        <Route path="/files" element={<FilesPage />} />
        <Route path="/vault" element={<VaultPage />} />
        <Route path="/settings" element={<SettingsPage />} />
        {/* 旧 /desktop 已并入 /（通配重定向兜住旧链接） */}
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
