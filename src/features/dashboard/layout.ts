/**
 * 首页（工作台 × 桌面主页合一）的布局模型：一份 tiles 清单，两态共用。
 * 布局逻辑全部是纯函数（同 home/layout.ts 范式），组件只做交互粘合。
 * 持久化经 kv（'dashboard.layout'），useDashboardLayout 负责。
 */

/** 内置卡片类型（插件卡为 `plugin:<id>`，见 pluginIdOfTile） */
export type BuiltinTileType =
  | 'weather'
  | 'todo'
  | 'countdown'
  | 'recentFiles'
  | 'topApps'
  | 'focus'
  | 'sysinfo'
  | 'taskmgr';

export type TileType = BuiltinTileType | `plugin:${string}`;

/** 卡片宽度档位：1 = 标准卡（4/12 列），2 = 大卡（8/12 列） */
export type TileSpan = 1 | 2;

export interface TileSpec {
  type: TileType;
  span: TileSpan;
}

export interface DashboardLayout {
  tiles: TileSpec[];
  /** 用户动过布局后置位：默认布局冻结，normalize 不再重置 */
  customized: boolean;
}

/** 内置卡片池（+ 菜单与默认布局的顺序依据） */
export const BUILTIN_TILES: readonly BuiltinTileType[] = [
  'weather',
  'todo',
  'countdown',
  'recentFiles',
  'topApps',
  'focus',
  'sysinfo',
  'taskmgr',
];

/** 默认布局 = 旧工作台的六卡（保持老用户熟悉的样子） */
export const DEFAULT_TILES: readonly TileSpec[] = [
  { type: 'weather', span: 1 },
  { type: 'todo', span: 1 },
  { type: 'countdown', span: 1 },
  { type: 'recentFiles', span: 1 },
  { type: 'topApps', span: 1 },
  { type: 'focus', span: 1 },
];

const isTileType = (v: unknown): v is TileType =>
  typeof v === 'string' &&
  (BUILTIN_TILES.includes(v as BuiltinTileType) || v.startsWith('plugin:'));

/** 存储数据 → 合法布局：坏数据/空库回默认，去掉非法项与重复项（保留首个） */
export function normalizeLayout(stored: DashboardLayout | null): DashboardLayout {
  if (!stored || !Array.isArray(stored.tiles)) {
    return { tiles: [...DEFAULT_TILES], customized: false };
  }
  const seen = new Set<string>();
  const tiles = stored.tiles.filter(
    (t) =>
      t &&
      isTileType(t.type) &&
      (t.span === 1 || t.span === 2) &&
      !seen.has(t.type) &&
      seen.add(t.type),
  );
  return { tiles, customized: stored.customized === true };
}

/** 拖拽落点重排：把 from 拔出插到 to（同位则原样返回，供浅比较短路） */
export function moveTile(tiles: readonly TileSpec[], from: number, to: number): TileSpec[] {
  if (from === to || from < 0 || to < 0 || from >= tiles.length || to >= tiles.length) {
    return [...tiles];
  }
  const next: TileSpec[] = [...tiles];
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved);
  return next;
}

export function removeTile(tiles: readonly TileSpec[], idx: number): TileSpec[] {
  if (idx < 0 || idx >= tiles.length) return [...tiles];
  return tiles.filter((_, i) => i !== idx);
}

/** 添加卡片（已存在则忽略，保持原位与档位）；插件卡默认与内置卡同规格 */
export function addTile(tiles: readonly TileSpec[], type: TileType): TileSpec[] {
  if (tiles.some((t) => t.type === type)) return [...tiles];
  return [...tiles, { type, span: 1 }];
}

/** 编辑态点 ⤢ 切换宽度档位（1 ↔ 2） */
export function cycleSpan(tiles: readonly TileSpec[], idx: number): TileSpec[] {
  if (idx < 0 || idx >= tiles.length) return [...tiles];
  return tiles.map((t, i) => (i === idx ? { ...t, span: t.span === 1 ? 2 : 1 } : t));
}

/** `plugin:xxx` → 插件 id（非插件卡返回 null） */
export function pluginIdOfTile(type: TileType): string | null {
  return type.startsWith('plugin:') ? type.slice('plugin:'.length) : null;
}
