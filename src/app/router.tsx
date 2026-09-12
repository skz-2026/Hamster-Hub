import { Navigate, Route, Routes } from 'react-router-dom';
import { AppShell } from '@/app/AppShell';
import HomePage from '@/app/routes/home';
import SpotlightPage from '@/app/routes/spotlight';
import TaskbarPage from '@/app/routes/taskbar';
import DesktopPage from '@/app/routes/desktop';
import AgentPage from '@/app/routes/agent';
import BenchPage from '@/app/routes/bench';
import WorkbenchPage from '@/app/routes/workbench';
import SearchPage from '@/app/routes/search';
import SchedulePage from '@/app/routes/schedule';
import AppsPage from '@/app/routes/apps';
import FilesPage from '@/app/routes/files';
import SettingsPage from '@/app/routes/settings';

export function AppRoutes() {
  return (
    <Routes>
      {/* Spotlight 独立覆盖层窗口（AppShell 之外，无应用壳） */}
      <Route path="/spotlight" element={<SpotlightPage />} />
      {/* 任务栏独立置顶窗口（桌面接管时贴屏幕底部条） */}
      <Route path="/taskbar" element={<TaskbarPage />} />
      <Route element={<AppShell />}>
        <Route path="/home" element={<HomePage />} />
        <Route path="/desktop" element={<DesktopPage />} />
        <Route path="/agent" element={<AgentPage />} />
        <Route path="/bench" element={<BenchPage />} />
        <Route path="/" element={<WorkbenchPage />} />
        <Route path="/search" element={<SearchPage />} />
        <Route path="/schedule" element={<SchedulePage />} />
        <Route path="/apps" element={<AppsPage />} />
        <Route path="/files" element={<FilesPage />} />
        <Route path="/settings" element={<SettingsPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
