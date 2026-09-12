/**
 * bench 域 hooks：代理清单 / 活会话 / 历史会话 / Recall 检索的 Query 封装。
 * 会话内容不走 Query（events.benchStreamEvent → stream-registry 直推），
 * 这里只管列表与索引元数据。
 */
import { useEffect } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { benchCommands, events } from '@/shared/lib/ipc';
import type { LiveSessionInfo, SearchQuery } from '@/shared/types/bench';
import { applyStreamEvent } from './stream-registry';
import { addMySession, loadMySessions, MY_SESSIONS_EVENT, patchMySession } from './registry';

export function useAgents() {
  const query = useQuery({ queryKey: ['bench', 'agents'], queryFn: () => benchCommands.benchScanAgents(), staleTime: 60_000 });
  return { query };
}

export function useProjects() {
  const query = useQuery({ queryKey: ['bench', 'projects'], queryFn: () => benchCommands.benchListProjects(), staleTime: 60_000 });
  return { query };
}

/** 各 Agent 会话记录发现的历史工作目录（Store 为空时的项目来源） */
export function useWorkspaces() {
  const query = useQuery({
    queryKey: ['bench', 'workspaces'],
    queryFn: () => benchCommands.benchAgentWorkspaces(),
    staleTime: 60_000,
  });
  return { query };
}

export function useLiveSessions() {
  const qc = useQueryClient();
  const query = useQuery({
    queryKey: ['bench', 'live'],
    queryFn: async () => {
      // PTY（TUI）与流式（GUI）活会话合并
      const [pty, stream] = await Promise.all([
        benchCommands.benchListLiveSessions(),
        benchCommands.benchListStreamSessions(),
      ]);
      return [...pty, ...stream];
    },
    refetchInterval: 8_000,
  });
  const invalidate = () => qc.invalidateQueries({ queryKey: ['bench', 'live'] });
  return { query, invalidate };
}

export function useHistorySessions() {
  const qc = useQueryClient();
  const query = useQuery({
    queryKey: ['bench', 'history'],
    queryFn: () => benchCommands.benchListHistorySessions(),
  });
  const remove = useMutation({
    mutationFn: ({ agent, sessionKey }: { agent: string; sessionKey: string }) =>
      benchCommands.benchSessionDelete(agent, sessionKey),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['bench'] }),
  });
  return { query, remove };
}

/** 会话登记表：只含通过松鼠Hub 发起/续聊过的会话（侧栏唯一数据源） */
export function useMySessions() {
  const qc = useQueryClient();
  const query = useQuery({
    queryKey: ['bench', 'my'],
    queryFn: loadMySessions,
  });
  // 登记表任意变更（含 registry.ts 内部的 KV 写入）→ 失效重取
  useEffect(() => {
    const handler = (): void => {
      void qc.invalidateQueries({ queryKey: ['bench', 'my'] });
    };
    window.addEventListener(MY_SESSIONS_EVENT, handler);
    return () => window.removeEventListener(MY_SESSIONS_EVENT, handler);
  }, [qc]);
  const invalidate = () => qc.invalidateQueries({ queryKey: ['bench', 'my'] });
  return { query, invalidate };
}

export function useIndexStatus() {
  const query = useQuery({
    queryKey: ['bench', 'index'],
    queryFn: () => benchCommands.benchIndexStatus(),
    refetchInterval: 30_000,
  });
  return { query };
}

export function useRecallSearch() {
  const search = useMutation({
    mutationFn: (query: SearchQuery) => benchCommands.benchSearchSessions(query),
  });
  return { search };
}

/**
 * bench 事件接线（bench 页挂载时调用一次）：流式事件写 registry 驱动 GUI 对话，
 * 进程退出事件刷新活会话列表。
 */
export function useBenchEvents(enabled: boolean) {
  const qc = useQueryClient();
  useEffect(() => {
    if (!enabled) return;
    const unlistenStream = events.benchStreamEvent.listen((e) => applyStreamEvent(e.payload));
    const unlistenStreamExit = events.benchStreamExit.listen(() =>
      qc.invalidateQueries({ queryKey: ['bench', 'live'] }),
    );
    const unlistenPtyExit = events.benchPtyExit.listen(() =>
      qc.invalidateQueries({ queryKey: ['bench', 'live'] }),
    );
    return () => {
      void unlistenStream.then((fn) => fn());
      void unlistenStreamExit.then((fn) => fn());
      void unlistenPtyExit.then((fn) => fn());
    };
  }, [enabled, qc]);
}

/** 创建流式会话（GUI 对话）并登记到侧栏；首条 prompt 同时作为标题 */
export async function createStreamSession(opts: {
  agentId: string;
  projectDir: string;
  firstPrompt: string | null;
  model: string | null;
  effort: string | null;
  resumeKey: string | null;
}): Promise<LiveSessionInfo> {
  const info = await benchCommands.benchStreamCreate(
    opts.agentId,
    opts.projectDir,
    opts.firstPrompt,
    opts.model,
    opts.effort,
    opts.resumeKey,
    false,
  );
  if (opts.resumeKey) {
    // 续聊：更新既有条目的活会话 id 与活跃时间
    await patchMySession(
      (s) => s.sessionKey === opts.resumeKey,
      { id: info.sessionId },
    );
  } else {
    await addMySession({
      id: info.sessionId,
      agent: opts.agentId,
      projectDir: opts.projectDir,
      title: opts.firstPrompt ?? `${opts.agentId} 会话`,
      sessionKey: info.resumeKey ?? null,
    });
  }
  return info;
}
