/**
 * PluginWidgetHost：插件小组件宿主。
 * 负责取清单 → 读代码 → 动态 import → 在玻璃卡容器内 render(el, ctx)，
 * 卸载时调用插件的清理函数。插件抛错只降级为卡片内错误提示，绝不影响主屏。
 */
import { useEffect, useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Puzzle } from 'lucide-react';
import { commands } from '@/shared/lib/ipc';
import { loadPluginRender, makePluginContext } from './registry';

export default function PluginWidgetHost({ pluginId }: { pluginId: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);
  const list = useQuery({ queryKey: ['plugins', 'list'], queryFn: () => commands.pluginList() });
  const manifest = list.data?.find((p) => p.id === pluginId);

  useEffect(() => {
    let cancelled = false;
    let cleanup: (() => void) | undefined;
    setError(null);

    (async () => {
      try {
        const m = manifest ?? (await commands.pluginList()).find((p) => p.id === pluginId);
        if (!m) {
          setError('插件未安装');
          return;
        }
        const code = await commands.pluginReadCode(pluginId, m.entry);
        const render = await loadPluginRender(pluginId, code);
        const el = ref.current;
        if (!el || cancelled) return;
        el.innerHTML = '';
        const maybeCleanup = render(el, makePluginContext(m));
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

  return (
    <div ref={ref} className="h-full w-full">
      {error && (
        <div className="flex h-full flex-col justify-center gap-1 px-1">
          <span className="flex items-center gap-1.5 text-[11px] font-medium text-white/85">
            <Puzzle size={12} className="text-amber-300" />
            插件 {pluginId} 出错
          </span>
          <span className="line-clamp-2 text-[10px] leading-relaxed text-white/45">{error}</span>
        </div>
      )}
      {!error && !manifest && list.isSuccess && (
        <div className="grid h-full place-items-center text-[11px] text-white/50">插件未安装</div>
      )}
    </div>
  );
}
