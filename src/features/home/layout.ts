/**
 * 主屏布局纯逻辑（iOS 范式）：分页/文件夹/Dock 数据模型与操作。
 * 全部为纯函数，便于单测（layout.test.ts）；组件只做交互粘合。
 */
import type { AppEntry } from '@/shared/types/ipc';
import burrowWallpaper from '@/assets/wallpapers/burrow.jpg';
import meadowWallpaper from '@/assets/wallpapers/meadow.jpg';

export const GRID_COLS = 7;
export const GRID_ROWS = 5;
export const PAGE_CAPACITY = GRID_COLS * GRID_ROWS;
export const DOCK_CAPACITY = 6;

/**
 * 接管模式底部任务栏条高（逻辑 px），与 src-tauri/desktop_mode.rs 的
 * TASKBAR_H_LOGICAL 保持一致：主窗口虽整屏铺满，但主屏/桌面主页的布局
 * 容器在其基础上预留这 68px（任务栏置顶窗恰好盖住），壁纸与 dock 都
 * 止于任务栏上沿，不再被遮挡。
 */
export const TASKBAR_H_PX = 68;

/** 页面槽位项：`app:${appKey}` 或 `folder:${folderId}` */
export type SlotItem = string;

export interface FolderDef {
  name: string;
  /** 文件夹内应用的原始 appKey */
  apps: string[];
}

export interface HomeLayout {
  version: number;
  wallpaper: string;
  /**
   * 自定义壁纸图片路径（wallpaper = 'custom' 时生效）。
   * 存的是导入进 app_data_dir/wallpapers 的副本路径（经 asset protocol 显示）。
   */
  wallpaperImage?: string;
  pages: SlotItem[][];
  /** Dock 只放应用（原始 appKey） */
  dock: string[];
  folders: Record<string, FolderDef>;
  /**
   * 用户手动整理过布局（拖拽/增删/进 Dock 等任何 commit 都会置位）。
   * 未置位 = 机器生成的默认布局：跟随常用度排名自动重生（首屏常用优先），
   * 置位后冻结，不再自动重排。
   */
  customized?: boolean;
}

/** 壁纸：CSS 渐变（css）或图片（image = 打包资源；imagePath = 本机文件路径） */
export interface Wallpaper {
  name: string;
  /** CSS 渐变背景（兜底层：图片加载失败时依然是协调的底色） */
  css?: string;
  /** 内置图片壁纸的打包资源地址（构建期 hash URL，直接可用） */
  image?: string;
  /** 本机图片路径（自定义壁纸，渲染时需转 asset protocol URL） */
  imagePath?: string;
}

/** 自定义壁纸的 wallpaper id（图片本体在 layout.wallpaperImage） */
export const CUSTOM_WALLPAPER_ID = 'custom';

export const WALLPAPERS: Record<string, Wallpaper> = {
  /** macOS 桌面风：深夜网格渐变（多层 radial 光晕 + 纵向暗角） */
  midnight: {
    name: '深夜',
    css: [
      'radial-gradient(90% 70% at 18% 8%, #2b3a67 0%, rgba(43,58,103,0) 55%)',
      'radial-gradient(80% 60% at 85% 18%, #5b3a7a 0%, rgba(91,58,122,0) 60%)',
      'radial-gradient(120% 90% at 50% 115%, #16233f 0%, rgba(22,35,63,0) 70%)',
      'linear-gradient(180deg, #101423 0%, #171c2e 55%, #0d101b 100%)',
    ].join(', '),
  },
  /** macOS 桌面风：极光（青绿→紫的冷色流光） */
  aurora: {
    name: '极光',
    css: [
      'radial-gradient(70% 55% at 22% 12%, #1d5f5e 0%, rgba(29,95,94,0) 60%)',
      'radial-gradient(75% 60% at 82% 10%, #3c2f6e 0%, rgba(60,47,110,0) 62%)',
      'radial-gradient(110% 80% at 50% 118%, #0f2b33 0%, rgba(15,43,51,0) 70%)',
      'linear-gradient(180deg, #0e1620 0%, #14202b 60%, #0b1219 100%)',
    ].join(', '),
  },
  /** macOS 桌面风：暮色沙丘（暖琥珀暗调） */
  dune: {
    name: '沙丘',
    css: [
      'radial-gradient(85% 60% at 25% 10%, #6e4a2a 0%, rgba(110,74,42,0) 58%)',
      'radial-gradient(75% 55% at 80% 16%, #83353f 0%, rgba(131,53,63,0) 60%)',
      'radial-gradient(120% 85% at 50% 115%, #2a1a12 0%, rgba(42,26,18,0) 70%)',
      'linear-gradient(180deg, #1c1410 0%, #241a12 60%, #15100b 100%)',
    ].join(', '),
  },
  hamster: { name: '胡萝卜', css: 'linear-gradient(160deg,#ffb26b 0%,#ff8a3d 40%,#8f3e12 100%)' },
  /** 内置插画壁纸（AI 生成，随应用打包）：小仓鼠仰望吊灯的木屋小窝 */
  burrow: {
    name: '松果窝',
    image: burrowWallpaper,
    css: 'linear-gradient(180deg,#8a6a4a 0%,#5a422c 55%,#3a2a1a 100%)',
  },
  /** 内置插画壁纸（AI 生成，随应用打包）：小仓鼠从花田探出头看日落 */
  meadow: {
    name: '花田暮色',
    image: meadowWallpaper,
    css: 'linear-gradient(180deg,#3a2c52 0%,#8a4a2c 55%,#2c2014 100%)',
  },
  dusk: { name: '暮色', css: 'linear-gradient(180deg,#1a1a40 0%,#4a1e5c 55%,#c33764 100%)' },
  mint: { name: '薄荷', css: 'linear-gradient(160deg,#43cea2 0%,#185a9d 100%)' },
  sakura: { name: '樱花', css: 'linear-gradient(160deg,#ffdde1 0%,#ee9ca7 55%,#a56a73 100%)' },
  ink: { name: '墨色', css: 'linear-gradient(180deg,#16161a 0%,#3a3a44 100%)' },
};

/**
 * 当前生效壁纸：custom → 用户导入的图片（缺图回退胡萝卜渐变）；
 * 未知 id（历史数据）→ 胡萝卜渐变兜底。渲染层统一走这里取壁纸。
 */
export function resolveWallpaper(
  layout: Pick<HomeLayout, 'wallpaper' | 'wallpaperImage'>,
): Wallpaper {
  if (layout.wallpaper === CUSTOM_WALLPAPER_ID) {
    return layout.wallpaperImage
      ? { name: '自定义', imagePath: layout.wallpaperImage }
      : WALLPAPERS.hamster;
  }
  return WALLPAPERS[layout.wallpaper] ?? WALLPAPERS.hamster;
}

/** 壁纸轮换（控制中心磁贴 / 主屏整理模式）：内置清单顺序循环，custom/未知 id 从头开始 */
export function nextWallpaperId(current: string): string {
  const ids = Object.keys(WALLPAPERS);
  const at = ids.indexOf(current);
  return ids[(at + 1 + ids.length) % ids.length];
}

export const appIdOf = (key: string) => `app:${key}`;
export const folderRefOf = (id: string) => `folder:${id}`;
/** 槽位项 → 应用原始 key（组件统一用本函数，禁止手写 slice 魔数） */
export const rawAppKey = (item: string) => item.slice('app:'.length);
/** 槽位项 → 文件夹 id */
export const rawFolderId = (item: string) => item.slice('folder:'.length);

// ===== 小组件槽位 =====

export type WidgetType = 'clock' | 'weather' | 'todo' | 'countdown' | 'sysinfo' | 'taskmgr';
export const WIDGET_TYPES: WidgetType[] = [
  'clock',
  'weather',
  'todo',
  'countdown',
  'sysinfo',
  'taskmgr',
];
export const WIDGET_LABEL: Record<WidgetType, string> = {
  clock: '时钟',
  weather: '天气',
  todo: '待办',
  countdown: '倒数日',
  sysinfo: '电脑状态',
  taskmgr: '任务管理器',
};

export const widgetRefOf = (type: WidgetType) => `widget:${type}`;
/** 槽位项 → 小组件类型；非小组件或类型非法返回 null */
export function rawWidgetType(item: string): WidgetType | null {
  if (!item.startsWith('widget:')) return null;
  const t = item.slice('widget:'.length) as WidgetType;
  if ((WIDGET_TYPES as string[]).includes(t)) return t;
  return null;
}

// ===== 插件小组件槽位（widget:plugin:<id>，M4 插件域）=====

/** 插件小组件的槽位引用 */
export const pluginWidgetRefOf = (id: string) => `widget:plugin:${id}`;

/** 槽位 → 插件 id（结构性校验，不做存在性判断——未安装插件由渲染层降级） */
export function pluginIdOf(item: string): string | null {
  if (!item.startsWith('widget:plugin:')) return null;
  const id = item.slice('widget:plugin:'.length);
  return /^[a-z0-9_-]+$/.test(id) ? id : null;
}

/**
 * 默认 Dock 候选（小写包含匹配，按优先级）。真机扫描顺序是字母序，会捞到
 * Administrative Tools / Application Verifier 这类系统工具——默认 Dock 只放
 * 用户认识的应用；一个都没匹配到就留空（显示「拖入常用应用」引导）。
 */
const KNOWN_DOCK_APPS = [
  '微信', 'qq', '腾讯会议', '钉钉', 'chrome', 'edge', 'firefox', 'steam',
  'vs code', 'word', 'excel', 'powerpoint', 'wps', '百度网盘', '网易云', 'qq音乐',
];

/**
 * 默认布局：常用度排名优先——Dock 取使用频次前 4，页面按「常用在前」排序
 *（无使用数据的新装应用保持扫描序垫底）。无任何使用数据时 Dock 退回
 * KNOWN_DOCK_APPS 候选表（真机扫描是字母序，会捞到 Administrative Tools
 * 这类系统工具，候选表只放用户认识的应用）；一个都没匹配到就留空
 *（显示「拖入常用应用」引导）。
 */
export function buildDefaultLayout(apps: AppEntry[], usageRank: string[] = []): HomeLayout {
  const byKey = new Map(apps.map((a) => [a.app_key, a]));
  const dock: string[] = [];
  for (const key of usageRank) {
    if (byKey.has(key) && !dock.includes(key)) dock.push(key);
    if (dock.length >= 4) break;
  }
  if (dock.length === 0) {
    for (const want of KNOWN_DOCK_APPS) {
      const hit = apps.find(
        (a) => a.display_name.toLowerCase().includes(want) && !dock.includes(a.app_key),
      );
      if (hit) dock.push(hit.app_key);
      if (dock.length >= 4) break;
    }
  }
  // 页面排序：rank 序在前、无使用数据的按原序（稳定排序）垫底
  const placed = new Set(dock);
  const rank = new Map(usageRank.map((k, i) => [k, i]));
  const rest = apps
    .filter((a) => !placed.has(a.app_key))
    .sort((x, y) => (rank.get(x.app_key) ?? Infinity) - (rank.get(y.app_key) ?? Infinity))
    .map((a) => appIdOf(a.app_key));
  const pages: SlotItem[][] = [];
  for (let i = 0; i < rest.length; i += PAGE_CAPACITY) {
    pages.push(rest.slice(i, i + PAGE_CAPACITY));
  }
  if (pages.length === 0) pages.push([]);
  return { version: 1, wallpaper: 'midnight', pages, dock, folders: {} };
}

/**
 * 规整化：customized 未置位的布局视为「默认布局」，直接按当前常用度重生
 *（首屏常用优先）；已定制布局则保留用户自定义顺序与文件夹结构——剔除已
 * 卸载应用/空文件夹、补充新装应用到最后一页、重分页。
 */
export function normalizeLayout(
  layout: HomeLayout | null,
  apps: AppEntry[],
  usageRank: string[] = [],
): HomeLayout {
  if (!layout || layout.version !== 1 || !layout.customized) {
    return buildDefaultLayout(apps, usageRank);
  }
  const valid = new Set(apps.map((a) => a.app_key));

  const dock = layout.dock.filter((k) => valid.has(k)).slice(0, DOCK_CAPACITY);

  // 1) 文件夹清洗：剔除失效应用，空夹先保留待引用判定
  const cleaned: Record<string, FolderDef> = {};
  for (const [id, f] of Object.entries(layout.folders)) {
    const kept = f.apps.filter((k) => valid.has(k));
    if (kept.length > 0) cleaned[id] = { ...f, apps: kept };
  }

  // 2) 页面引用决定文件夹存留（未被引用 = 孤儿，删除）；合法小组件保留
  const refs = new Set<string>();
  const seen = new Set<string>();
  let items = layout.pages.flat().filter((it) => {
    if (it.startsWith('folder:')) {
      const id = rawFolderId(it);
      if (!cleaned[id] || seen.has(it)) return false; // 失效/重复引用丢弃
      seen.add(it);
      refs.add(id);
      return true;
    }
    if (it.startsWith('widget:')) {
      // 内置类型或插件槽位（结构性放行；未安装插件由渲染层降级为占位）
      if (!(rawWidgetType(it) || pluginIdOf(it)) || seen.has(it)) return false;
      seen.add(it);
      return true;
    }
    if (!it.startsWith('app:')) return false;
    return valid.has(rawAppKey(it));
  });
  const folders: Record<string, FolderDef> = {};
  for (const id of refs) folders[id] = cleaned[id];

  // 3) 已占据（dock 或被引用文件夹内）的应用不出现在页面
  const placed = new Set([...dock, ...Object.values(folders).flatMap((f) => f.apps)]);
  items = items.filter((it) => !it.startsWith('app:') || !placed.has(rawAppKey(it)));

  // 4) 新装应用追加到末尾
  const existing = new Set(items.filter((i) => i.startsWith('app:')).map(rawAppKey));
  for (const a of apps) {
    if (!placed.has(a.app_key) && !existing.has(a.app_key)) items.push(appIdOf(a.app_key));
  }

  const pages: SlotItem[][] = [];
  for (let i = 0; i < items.length; i += PAGE_CAPACITY) {
    pages.push(items.slice(i, i + PAGE_CAPACITY));
  }
  if (pages.length === 0) pages.push([]);

  return { ...layout, pages, dock, folders };
}

/** 容器内重排 */
export function reorder(list: SlotItem[], from: number, to: number): SlotItem[] {
  if (from === to || from < 0 || to < 0 || from >= list.length || to >= list.length) return list;
  const next = [...list];
  const [it] = next.splice(from, 1);
  next.splice(to, 0, it);
  return next;
}

/** 跨页移动（拖到目标页某个槽位索引处） */
export function moveAcrossPages(
  pages: SlotItem[][],
  fromPage: number,
  fromIdx: number,
  toPage: number,
  toIdx: number,
): SlotItem[][] {
  if (!pages[fromPage] || !pages[toPage]) return pages;
  const next = pages.map((p) => [...p]);
  const [it] = next[fromPage].splice(fromIdx, 1);
  const target = next[toPage];
  const at = Math.min(Math.max(toIdx, 0), target.length);
  target.splice(at, 0, it);
  return compactPages(next);
}

/** 页面 → Dock（仅应用；Dock 满则不动） */
export function moveToDock(layout: HomeLayout, pageIdx: number, idx: number): HomeLayout {
  const item = layout.pages[pageIdx]?.[idx];
  if (!item?.startsWith('app:') || layout.dock.length >= DOCK_CAPACITY) return layout;
  const pages = layout.pages.map((p, i) => (i === pageIdx ? p.filter((_, j) => j !== idx) : [...p]));
  return { ...layout, pages: compactPages(pages), dock: [...layout.dock, rawAppKey(item)] };
}

/** Dock → 最后一页末尾 */
export function removeFromDock(layout: HomeLayout, dockIdx: number): HomeLayout {
  const key = layout.dock[dockIdx];
  if (!key) return layout;
  const pages = compactPages([...layout.pages.map((p) => [...p]), [appIdOf(key)]]);
  return { ...layout, pages, dock: layout.dock.filter((_, i) => i !== dockIdx) };
}

/** 拖拽合并到文件夹：目标是应用 → 新建文件夹；目标是文件夹 → 加入 */
export function mergeIntoFolder(
  layout: HomeLayout,
  pageIdx: number,
  dragIdx: number,
  targetIdx: number,
): HomeLayout {
  const page = layout.pages[pageIdx];
  if (!page) return layout;
  const dragItem = page[dragIdx];
  const targetItem = page[targetIdx];
  if (!dragItem?.startsWith('app:') || dragIdx === targetIdx) return layout;
  const dragKey = rawAppKey(dragItem);

  const pages = layout.pages.map((p) => [...p]);
  const folders = { ...layout.folders };

  if (targetItem.startsWith('folder:')) {
    const id = rawFolderId(targetItem);
    if (!folders[id] || folders[id].apps.includes(dragKey)) return layout;
    folders[id] = { ...folders[id], apps: [...folders[id].apps, dragKey] };
    pages[pageIdx] = page.filter((_, i) => i !== dragIdx);
  } else if (targetItem.startsWith('app:')) {
    const targetKey = rawAppKey(targetItem);
    const id = `f${Date.now().toString(36)}`;
    folders[id] = { name: '新建文件夹', apps: [targetKey, dragKey] };
    const keepAt = Math.min(dragIdx, targetIdx);
    pages[pageIdx] = page.filter((_, i) => i !== dragIdx && i !== targetIdx);
    pages[pageIdx].splice(keepAt, 0, folderRefOf(id));
  } else {
    return layout;
  }
  return { ...layout, pages: compactPages(pages), folders };
}

/** 移除页面槽位：文件夹删除时内部应用散落到最后一页 */
export function removeFromPage(layout: HomeLayout, pageIdx: number, idx: number): HomeLayout {
  const page = layout.pages[pageIdx];
  const item = page?.[idx];
  if (!item) return layout;

  if (item.startsWith('folder:')) {
    const id = rawFolderId(item);
    const spill = (layout.folders[id]?.apps ?? []).map(appIdOf);
    const folders = { ...layout.folders };
    delete folders[id];
    const pages = layout.pages.map((p, i) => (i === pageIdx ? p.filter((_, j) => j !== idx) : [...p]));
    return { ...layout, pages: compactPages([...pages, spill]), folders };
  }
  const pages = layout.pages.map((p, i) => (i === pageIdx ? p.filter((_, j) => j !== idx) : [...p]));
  return { ...layout, pages };
}

/** 去掉空页（保留至少一页） */
export function compactPages(pages: SlotItem[][]): SlotItem[][] {
  const next = pages.filter((p) => p.length > 0);
  return next.length > 0 ? next : [[]];
}

/** 添加小组件：追加到最后一个未满页；全部满则开新页 */
export function addWidget(layout: HomeLayout, type: WidgetType): HomeLayout {
  const ref = widgetRefOf(type);
  const pages = layout.pages.map((p) => [...p]);
  const last = pages[pages.length - 1];
  if (last.length < PAGE_CAPACITY) {
    last.push(ref);
  } else {
    pages.push([ref]);
  }
  return { ...layout, pages };
}
