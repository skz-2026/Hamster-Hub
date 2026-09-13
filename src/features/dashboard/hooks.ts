import { useRef } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { commands, isTauri, type CountdownItem, type FileHit, type WeatherNow, type AppEntry } from '@/shared/lib/ipc';
import { normalizeLayout, type DashboardLayout, type TileSpec } from './layout';

export function useWeather() {
  return useQuery({
    queryKey: ['weather', 'now'],
    queryFn: () => commands.weatherGet(),
    staleTime: 10 * 60_000,
    refetchInterval: 30 * 60_000,
    // 浏览器 mock / 真机均可用
    retry: false,
    enabled: true,
  });
}

export function useCountdown() {
  return useQuery({
    queryKey: ['countdown', 'list'],
    queryFn: () => commands.countdownList(),
    staleTime: 60 * 60_000,
  });
}

export function useRecentFiles(limit = 6) {
  return useQuery({
    queryKey: ['files', 'recent', limit],
    queryFn: () => commands.recentFiles(limit),
    staleTime: 60_000,
    // 浏览器 mock 有假数据；真机无索引时返回空数组
  });
}

export function useTopApps(limit = 8) {
  return useQuery({
    queryKey: ['apps', 'top', limit],
    queryFn: () => commands.topApps(limit),
    staleTime: 60_000,
    // 常用组跨窗口（主窗口/任务栏）无法互发 invalidate，靠短轮询最终一致
    refetchInterval: 30_000,
  });
}

export function useSystemStats() {
  return useQuery({
    queryKey: ['sys', 'stats'],
    queryFn: () => commands.systemStats(),
    refetchInterval: 5_000,
    staleTime: 4_000,
    retry: false,
  });
}

export function useProcessList(limit = 5) {
  return useQuery({
    queryKey: ['sys', 'proc', limit],
    queryFn: () => commands.processList('mem', limit),
    refetchInterval: 5_000,
    staleTime: 4_000,
    retry: false,
  });
}

export function useKillProcess() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (pid: number) => commands.processKill(pid),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['sys'] }),
  });
}

const DASH_LAYOUT_KEY = 'dashboard.layout';

/**
 * 首页编排布局：同 useHomeLayout 范式——Query 缓存为单一事实源 + 防抖持久化。
 * 首页只在主窗口渲染，无需跨窗口同步（home 是主窗口+任务栏两处用才要）。
 */
export function useDashboardLayout() {
  const qc = useQueryClient();
  const { data: stored } = useQuery({
    queryKey: ['dashboard', 'layout'],
    queryFn: async () => {
      const raw = await commands.kvGet(DASH_LAYOUT_KEY);
      if (!raw) return null;
      try {
        return JSON.parse(raw) as DashboardLayout;
      } catch {
        return null;
      }
    },
    staleTime: Infinity,
  });

  const save = useMutation({
    mutationFn: (l: DashboardLayout) => commands.kvSet(DASH_LAYOUT_KEY, JSON.stringify(l)),
  });

  const timer = useRef<number | undefined>(undefined);
  const commit = (tiles: TileSpec[]) => {
    const next: DashboardLayout = { tiles, customized: true };
    qc.setQueryData(['dashboard', 'layout'], next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => save.mutate(next), 500);
  };

  const layout = normalizeLayout(stored ?? null);
  return { layout, commit, ready: stored !== undefined };
}

export type { WeatherNow, CountdownItem, FileHit, AppEntry };
export { isTauri };
