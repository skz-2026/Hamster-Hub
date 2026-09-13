/**
 * PluginWidgetHost：插件小组件宿主。
 * 负责取清单 → 读代码 → 动态 import → 在玻璃卡容器内 render(el, ctx)，
 * 卸载时调用插件的清理函数。插件抛错只降级为卡片内错误提示，绝不影响主屏。
 */
import { useEffect, useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Puzzle } from 'lucide-react';
import { commands } from '@/shared/lib/ipc';
import { translate, getLang } from '@/shared/i18n/core';
import { useI18n } from '@/shared/i18n/provider';
import { loadPluginRender, useDisabledPlugins, usePluginApi } from './registry';

export default function PluginWidgetHost({ pluginId }: { pluginId: string }) {
  const { t } = useI18n();
  const ref = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);
  const list = useQuery({ queryKey: ['plugins', 'list'], queryFn: () => commands.pluginList() });
  const disabled = useDisabledPlugins();
  const manifest = list.data?.find((p) => p.id === pluginId);
  const api = usePluginApi(manifest ?? { id: pluginId, name: pluginId, version: '', description: '' });
  const isDisabled = disabled.data.includes(pluginId);

  useEffect(() => {
    let cancelled = false;
    let cleanup: (() => void) | undefined;
    setError(null);
    if (disabled.data.includes(pluginId)) return; // 停用：不装载，渲染占位

    (async () => {
      try {
        const m = manifest ?? (await commands.pluginList()).find((p) => p.id === pluginId);
        if (!m) {
          setError(translate(getLang(), 'settings.plugins.hostNotInstalled'));
          return;
        }
        const code = await commands.pluginReadCode(pluginId, m.entry);
        const render = await loadPluginRender(pluginId, code);
        const el = ref.current;
        if (!el || cancelled) return;
        el.innerHTML = '';
        const storage = {
          get: (key: string) => commands.pluginStorageGet(m.id, key),
          set: async (key: string, value: string) => {
            await commands.pluginStorageSet(m.id, key, value);
          },
        };
        const maybeCleanup = render(el, { manifest: m, storage, api });
        if (!cancelled && typeof maybeCleanup === 'function') cleanup = maybeCleanup;
      } catch (e) {
        if (!cancelled) setError(String(e).slice(0, 160));
      }
    })();

    return () => {
      cancelled = true;
      try {
        cleanup?.();
      } catch {
        /* 插件清理函数自身出错不再扩散 */
      }
    };
  }, [pluginId, manifest]);

  if (isDisabled) {
    return (
      <div className="grid h-full place-items-center text-[11px] text-white/40">
        <span className="flex items-center gap-1.5">
          <Puzzle size={11} /> {t('settings.plugins.hostDisabled')}
        </span>
      </div>
    );
  }

  return (
    <div ref={ref} className="h-full w-full">
      {error && (
        <div className="flex h-full flex-col justify-center gap-1 px-1">
          <span className="flex items-center gap-1.5 text-[11px] font-medium text-white/85">
            <Puzzle size={12} className="text-amber-300" />
            {t('settings.plugins.hostError', { id: pluginId })}
          </span>
          <span className="line-clamp-2 text-[10px] leading-relaxed text-white/45">{error}</span>
        </div>
      )}
      {!error && !manifest && list.isSuccess && (
        <div className="grid h-full place-items-center text-[11px] text-white/50">{t('settings.plugins.hostNotInstalled')}</div>
      )}
    </div>
  );
}
