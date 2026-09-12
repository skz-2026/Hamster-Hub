import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { commands, type Settings } from '@/shared/lib/ipc';

export function useSettings() {
  return useQuery({
    queryKey: ['settings'],
    queryFn: () => commands.settingsLoad(),
  });
}

export function useSaveSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (s: Settings) => commands.settingsSave(s),
    onSuccess: (saved) => qc.setQueryData(['settings'], saved),
  });
}

/** 局部更新设置并持久化 */
export function usePatchSettings() {
  const { data: settings } = useSettings();
  const save = useSaveSettings();
  return {
    isSaving: save.isPending,
    patch: (partial: DeepPartial<Settings>) => {
      if (!settings) return;
      save.mutate(deepMerge(settings, partial));
    },
  };
}

type DeepPartial<T> = { [K in keyof T]?: T[K] extends object ? DeepPartial<T[K]> : T[K] };

function deepMerge<T>(base: T, patch: DeepPartial<T>): T {
  const out = { ...base } as Record<string, unknown>;
  for (const [k, v] of Object.entries(patch as Record<string, unknown>)) {
    out[k] =
      v && typeof v === 'object' && !Array.isArray(v)
        ? deepMerge(out[k] as Record<string, unknown>, v as DeepPartial<Record<string, unknown>>)
        : v;
  }
  return out as T;
}
