import { useEffect, useMemo, useRef } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { emit, listen } from '@tauri-apps/api/event';
import { commands, events, isTauri, type AppEntry } from '@/shared/lib/ipc';
import type { HomeLayout } from './layout';
import { normalizeLayout } from './layout';

export function useApps() {
  const qc = useQueryClient();
  const q = useQuery({
    queryKey: ['apps', 'all'],
    queryFn: () => commands.appList(),
    staleTime: Infinity,
  });

  // 索引后台扫描完成后刷新
  useEffect(() => {
    let un: (() => void) | undefined;
    events.appIndexUpdated
      .listen(() => qc.invalidateQueries({ queryKey: ['apps', 'all'] }))
      .then((fn) => (un = fn));
    return () => un?.();
  }, [qc]);

  return q;
}

const LAYOUT_KEY = 'home.layout';

/**
 * 主屏布局：Query 缓存为单一事实源 + 防抖持久化 + 跨实例/跨窗口同步。
 * commit 直接写缓存（同窗口所有实例即时一致）；持久化成功后广播事件，
 * 其它窗口（主窗口/任务栏各自持有 QueryClient）失效重取。
 */
export function useHomeLayout(apps: AppEntry[]) {
  const qc = useQueryClient();
  const { data: stored } = useQuery({
    queryKey: ['home', 'layout'],
    queryFn: async () => {
      const raw = await commands.kvGet(LAYOUT_KEY);
      if (!raw) return null;
      try {
        return JSON.parse(raw) as HomeLayout;
      } catch {
        return null;
      }
    },
    staleTime: Infinity,
  });

  const save = useMutation({
    mutationFn: (l: HomeLayout) => commands.kvSet(LAYOUT_KEY, JSON.stringify(l)),
    onSuccess: () => {
      // 广播布局变更：tauri emit 跨窗口；DOM 事件兜底同窗口非 React 链路
      if (isTauri) emit('hamster:layout-updated').catch(console.error);
    },
  });

  // 其它实例/窗口提交的布局变更 → 本窗口失效重取
  useEffect(() => {
    const invalidate = () =>
      qc.invalidateQueries({ queryKey: ['home', 'layout'] });
    window.addEventListener('hamster:layout-updated-dom', invalidate);
    let un: (() => void) | undefined;
    if (isTauri) {
      listen('hamster:layout-updated', invalidate)
        .then((fn) => (un = fn))
        .catch(console.error);
    }
    return () => {
      window.removeEventListener('hamster:layout-updated-dom', invalidate);
      un?.();
    };
  }, [qc]);

  const timer = useRef<number | undefined>(undefined);

  const commit = (next: HomeLayout) => {
    // 直接写共享缓存：同窗口的任务栏/主屏/弹窗实例立即一致
    qc.setQueryData(['home', 'layout'], next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => save.mutate(next), 500);
  };

  const layout = useMemo(() => normalizeLayout(stored ?? null, apps), [stored, apps]);
  return { layout, commit, ready: stored !== undefined, isSaving: save.isPending };
}
