/**
 * 插件注册表：列表查询 + 入口模块加载缓存。
 * 加载方式 = 后端读代码文本 → Blob URL → 原生动态 import
 * （浏览器 / Tauri WebView 同一通路，dev 与产物行为一致）。
 */
import { useQuery } from '@tanstack/react-query';
import { commands } from '@/shared/lib/ipc';
import type { PluginContext, PluginRenderFn } from './types';

export function usePluginList() {
  return useQuery({
    queryKey: ['plugins', 'list'],
    queryFn: () => commands.pluginList(),
    staleTime: 30_000,
  });
}

/** 插件 id → 已加载模块（跨组件共享，避免主屏/桌面重复 import） */
const moduleCache = new Map<string, Promise<PluginRenderFn>>();

async function importViaBlob(code: string): Promise<PluginRenderFn> {
  const url = URL.createObjectURL(new Blob([code], { type: 'text/javascript' }));
  try {
    // @vite-ignore：运行时原生动态 import，不走 Vite 预打包
    const mod = (await import(/* @vite-ignore */ url)) as {
      default?: PluginRenderFn;
    };
    if (typeof mod.default !== 'function') {
      throw new Error('widget.js 需要默认导出 (el, ctx) => void 函数');
    }
    return mod.default;
  } finally {
    // 模块求值完成后回收 URL（延迟释放，避免竞态）
    setTimeout(() => URL.revokeObjectURL(url), 60_000);
  }
}

export function loadPluginRender(pluginId: string, code: string): Promise<PluginRenderFn> {
  let p = moduleCache.get(pluginId);
  if (!p) {
    p = importViaBlob(code);
    moduleCache.set(pluginId, p);
  }
  return p;
}

/** 插件已卸载/重载时清缓存（供后续「插件管理」使用） */
export function invalidatePluginModule(pluginId: string) {
  moduleCache.delete(pluginId);
}

export function makePluginContext(manifest: {
  id: string;
  name: string;
  version: string;
  description: string;
}): PluginContext {
  return {
    manifest,
    storage: {
      get: (key) => commands.pluginStorageGet(manifest.id, key),
      set: async (key, value) => {
        await commands.pluginStorageSet(manifest.id, key, value);
      },
    },
  };
}
