import { useEffect, useMemo, useRef } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { emit, listen } from '@tauri-apps/api/event';
import { commands, events, isTauri, type AppEntry } from '@/shared/lib/ipc';
import type { HomeLayout } from './layout';
import { normalizeLayout, resolveWallpaper } from './layout';

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

/** 壁纸跟随主屏布局（/、/home、接管子页 backdrop 换壁纸即时同步） */
export function useWallpaper() {
  const { data: apps = [] } = useApps();
  const { layout } = useHomeLayout(apps);
  return resolveWallpaper(layout);
}

/**
 * dock 应用运行态：2.5s 轮询（EnumWindows 全集一次取回，毫秒级），
 * 返回「当前有可见窗口」的 app_key 集合。keys 需传稳定引用（useMemo），
 * 否则每次渲染都开新 Query 缓存项。
 */
export function useRunningAppKeys(keys: string[]) {
  const q = useQuery({
    queryKey: ['apps', 'running', keys],
    queryFn: () => commands.appsRunning(keys),
    refetchInterval: 2500,
    staleTime: 1500,
  });
  return useMemo(() => new Set(q.data ?? []), [q.data]);
}

/**
 * 已知不支持多开的应用（内置名单 + 多开尝试自学习）。菜单据此隐藏
 * 「多开应用」。不轮询：launchNew 后 invalidate 即时刷新。
 */
export function useSingleInstanceApps(keys: string[]) {
  const q = useQuery({
    queryKey: ['apps', 'single', keys],
    queryFn: () => commands.appMultiFlags(keys),
    staleTime: Infinity,
  });
  return useMemo(() => new Set(q.data ?? []), [q.data]);
}

/**
 * 托管分屏会话（桌面接管态）。2.5s 轮询：分屏里的窗口可能被用户在别处关掉
 * （任务栏、应用自己退出），Rust 侧读快照时顺手剔除死窗并让剩下的补位——
 * 轮询就能拿到补位后的新布局。不开分屏（enabled=false）时不轮询。
 */
export function useSplitState(enabled: boolean) {
  const qc = useQueryClient();
  // 分屏动作可能发生在另一个窗口（任务栏胶囊 ↔ 置顶管理弹层各自一份 Query 缓存），
  // 变更方广播事件，这里失效重取，胶囊状态才不会慢半拍
  useEffect(() => {
    if (!isTauri) return;
    let un: (() => void) | undefined;
    listen('hamster:split-changed', () => qc.invalidateQueries({ queryKey: ['split'] }))
      .then((fn) => (un = fn))
      .catch(console.error);
    return () => un?.();
  }, [qc]);
  return useQuery({
    queryKey: ['split', 'state'],
    queryFn: () => commands.splitState(),
    enabled,
    refetchInterval: enabled ? 2500 : false,
    staleTime: 1500,
  });
}

/** 分屏动作：加窗 / 移出 / 退出。Rust 返回新快照，直接写进缓存省一轮往返 */
export function useSplitActions() {
  const qc = useQueryClient();
  const put = (s: Awaited<ReturnType<typeof commands.splitState>>) => {
    qc.setQueryData(['split', 'state'], s);
    if (isTauri) emit('hamster:split-changed').catch(console.error);
  };
  return {
    add: useMutation({ mutationFn: (appKey: string) => commands.splitAdd(appKey), onSuccess: put }),
    remove: useMutation({
      mutationFn: (windowId: number) => commands.splitRemove(windowId),
      onSuccess: put,
    }),
    exit: useMutation({ mutationFn: () => commands.splitExit(), onSuccess: put }),
  };
}
