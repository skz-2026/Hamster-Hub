import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { commands, isTauri, type CountdownItem, type FileHit, type WeatherNow, type AppEntry } from '@/shared/lib/ipc';

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

export type { WeatherNow, CountdownItem, FileHit, AppEntry };
export { isTauri };
