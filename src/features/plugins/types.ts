/**
 * UI 插件契约（用户可扩展的主屏小组件，M4 阶段四）。
 *
 * 插件作者视角：插件目录里放 plugin.json + widget.js，
 * widget.js 默认导出 `(el: HTMLElement, ctx: PluginContext) => (() => void) | void`。
 * 沙箱能力：DOM + fetch 网络 + ctx.storage 私有存储；不开放原生 IPC。
 */

/** 插件上下文：传给 render 的能力集合 */
export interface PluginContext {
  manifest: {
    id: string;
    name: string;
    version: string;
    description: string;
  };
  /** 插件私有 KV（后端 settings 表 `plugin.<id>.<key>` 命名空间） */
  storage: {
    get(key: string): Promise<string | null>;
    set(key: string, value: string): Promise<void>;
  };
}

export type PluginRenderFn = (el: HTMLElement, ctx: PluginContext) => (() => void) | void;

/** 布局槽位引用（与 builtin 的 `widget:${type}` 同构） */
export const pluginWidgetRefOf = (id: string) => `widget:plugin:${id}`;

/** 槽位 → 插件 id；非插件槽位返回 null */
export function pluginIdOf(item: string): string | null {
  if (!item.startsWith('widget:plugin:')) return null;
  const id = item.slice('widget:plugin:'.length);
  return /^[a-z0-9_-]+$/.test(id) ? id : null;
}
