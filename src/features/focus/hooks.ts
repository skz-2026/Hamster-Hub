import { useEffect } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { commands, events, type FocusStatus } from '@/shared/lib/ipc';

/** 番茄钟：状态由后端秒级 tick 事件驱动，完成/停止刷新历史 */
export function useFocus() {
  const qc = useQueryClient();
  const statusQ = useQuery({
    queryKey: ['focus', 'status'],
    queryFn: () => commands.focusStatus(),
  });
  const historyQ = useQuery({
    queryKey: ['focus', 'history'],
    queryFn: () => commands.focusHistory(7),
  });

  useEffect(() => {
    let unbind: Array<() => void> = [];
    let stopped = false;
    Promise.all([
      events.focusTick.listen((e) => {
        qc.setQueryData<FocusStatus | null>(['focus', 'status'], (prev) =>
          prev ? { ...prev, remaining_secs: e.payload.remaining_secs, paused: e.payload.paused } : prev,
        );
      }),
      events.focusFinished.listen(() => {
        qc.setQueryData<FocusStatus | null>(['focus', 'status'], null);
        void qc.invalidateQueries({ queryKey: ['focus', 'history'] });
      }),
    ]).then((fns) => {
      if (stopped) fns.forEach((f) => f());
      else unbind = fns;
    });
    return () => {
      stopped = true;
      unbind.forEach((f) => f());
    };
  }, [qc]);

  const applyStatus = (s: FocusStatus | null) => qc.setQueryData(['focus', 'status'], s);

  const start = useMutation({
    mutationFn: (minutes: number) => commands.focusStart(minutes, null),
    onSuccess: applyStatus,
  });
  const startBreak = useMutation({
    mutationFn: (minutes: number) => commands.focusBreak(minutes),
    onSuccess: applyStatus,
  });
  const pause = useMutation({
    mutationFn: () => commands.focusPause(),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['focus', 'status'] }),
  });
  const resume = useMutation({
    mutationFn: () => commands.focusResume(),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['focus', 'status'] }),
  });
  const stop = useMutation({
    mutationFn: () => commands.focusStop(),
    onSuccess: () => {
      qc.setQueryData<FocusStatus | null>(['focus', 'status'], null);
      void qc.invalidateQueries({ queryKey: ['focus', 'history'] });
    },
  });

  return { statusQ, historyQ, start, startBreak, pause, resume, stop };
}
