/**
 * 插件注册表：列表查询 + 入口模块加载缓存。
 * 加载方式 = 后端读代码文本 → Blob URL → 原生动态 import
 * （浏览器 / Tauri WebView 同一通路，dev 与产物行为一致）。
 */
import { useQuery, useQueryClient } from '@tanstack/react-query';
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

const DISABLED_KEY = 'plugins.disabled';

/** 停用列表（KV `plugins.disabled`；停用的插件从选择器隐藏、槽位显示占位） */
export function useDisabledPlugins() {
  const q = useQuery({
    queryKey: ['plugins', 'disabled'],
    queryFn: async () => {
      const raw = await commands.kvGet(DISABLED_KEY);
      try {
        return raw ? (JSON.parse(raw) as string[]) : [];
      } catch {
        return [] as string[];
      }
    },
  });
  const setDisabled = async (id: string, disabled: boolean) => {
    const cur = q.data ?? [];
    const next = disabled ? [...new Set([...cur, id])] : cur.filter((x) => x !== id);
    await commands.kvSet(DISABLED_KEY, JSON.stringify(next));
    await q.refetch();
  };
  return { data: q.data ?? [], setDisabled, refetch: q.refetch };
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

/** 桥接写操作 → 相关查询失效（插件改动即时反映到 UI，不受全局 staleTime 拖延） */
const BRIDGE_INVALIDATIONS: Record<string, string[][]> = {
  'todo.add': [['todo']],
  'todo.list': [],
  'apps.launch': [['apps', 'top'], ['apps', 'all']],
  'apps.search': [],
};

export function usePluginApi(manifest: {
  id: string;
  name: string;
  version: string;
  description: string;
  permissions?: string[] | null;
}): PluginContext['api'] {
  const qc = useQueryClient();
  const perms = manifest.permissions ?? [];
  const bridge = (capability: string, payload?: unknown) =>
    commands
      .pluginBridgeCall(manifest.id, capability, JSON.stringify(payload ?? {}))
      .then((r) => {
        for (const key of BRIDGE_INVALIDATIONS[capability] ?? []) {
          void qc.invalidateQueries({ queryKey: key });
        }
        return JSON.parse(r) as unknown;
      });
  // 仅声明的权限会出现在 api 上（未声明 = undefined，插件侧可判空降级）
  const api: PluginContext['api'] = {};
  if (perms.includes('todo.list')) api.todoList = () => bridge('todo.list');
  if (perms.includes('todo.add')) api.todoAdd = (p) => bridge('todo.add', p);
  if (perms.includes('apps.search')) api.appsSearch = (p) => bridge('apps.search', p);
  if (perms.includes('apps.launch')) api.appsLaunch = (p) => bridge('apps.launch', p);
  return api;
}

export function makePluginContext(manifest: {
  id: string;
  name: string;
  version: string;
  description: string;
  permissions?: string[] | null;
}, api: PluginContext['api']): PluginContext {
  return {
    manifest,
    storage: {
      get: (key) => commands.pluginStorageGet(manifest.id, key),
      set: async (key, value) => {
        await commands.pluginStorageSet(manifest.id, key, value);
      },
    },
    api,
  };
}
