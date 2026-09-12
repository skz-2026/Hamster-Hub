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
    // 任何手动整理都视为定制（customized 置位）：默认布局从此冻结，
    // normalizeLayout 不再按常用度自动重排
    const marked: HomeLayout = { ...next, customized: true };
    // 直接写共享缓存：同窗口的任务栏/主屏/弹窗实例立即一致
    qc.setQueryData(['home', 'layout'], marked);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => save.mutate(marked), 500);
  };

  // 常用度排名（默认布局的排序依据）。staleTime Infinity：只在失效时重取
  //（启动应用等），不能随 30s 轮询刷新——默认布局会跟着重排，图标不能在
  // 用户注视/交互时自己换位；本次会话内的排名变化下次进入主屏再生效
  const { data: top = [] } = useQuery({
    queryKey: ['apps', 'top', 60],
    queryFn: () => commands.topApps(60),
    staleTime: Infinity,
  });
  const usageRank = useMemo(() => top.map((a) => a.app_key), [top]);

  const layout = useMemo(
    () => normalizeLayout(stored ?? null, apps, usageRank),
    [stored, apps, usageRank],
  );
  return { layout, usageRank, commit, ready: stored !== undefined, isSaving: save.isPending };
}
