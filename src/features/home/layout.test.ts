import { describe, expect, it } from 'vitest';
import {
  addWidget,
  buildDefaultLayout,
  compactPages,
  DOCK_CAPACITY,
  mergeIntoFolder,
  moveAcrossPages,
  moveToDock,
  normalizeLayout,
  removeFromDock,
  removeFromPage,
  reorder,
  widgetRefOf,
  type HomeLayout,
} from './layout';
import type { AppEntry } from '@/shared/types/ipc';
const app = (key: string, name = key): AppEntry => ({
  app_key: key,
  display_name: name,
  exec_target: `C:\\lnk\\${key}.lnk`,
  kind: 'lnk',
  icon_path: null,
});

describe('buildDefaultLayout', () => {
  it('常见应用优先进 Dock（而非扫描顺序前 4），其余按 35/页分页', () => {
    const apps = Array.from({ length: 40 }, (_, i) => app(`k${i}`));
    // k7 名字含「微信」、k3 含「Chrome」——应入选 Dock，而不是 k0-k3
    apps[7] = app('k7', '微信');
    apps[3] = app('k3', 'Google Chrome');
    const l = buildDefaultLayout(apps);
    expect(l.dock).toContain('k7');
    expect(l.dock).toContain('k3');
    expect(l.dock).toHaveLength(2); // 其余名字不匹配候选表
    expect(l.pages).toHaveLength(2);
    expect(l.pages.flat().length).toBe(40 - 2);
  });

  it('无常见应用时 Dock 留空，全部进页面', () => {
    const l = buildDefaultLayout([app('a'), app('b')]);
    expect(l.dock).toEqual([]);
    expect(l.pages).toEqual([['app:a', 'app:b']]);
  });

  it('无应用时保留一个空页', () => {
    expect(buildDefaultLayout([]).pages).toEqual([[]]);
  });
});

describe('normalizeLayout', () => {
  it('剔除已卸载应用并补齐新装应用', () => {
    // 名称按 KNOWN_DOCK_APPS 优先级递进：微信→QQ→钉钉→Chrome，Dock 顺序与选入顺序一致
    const apps = [
      app('a', '微信'), app('b', 'QQ'), app('c', '钉钉'), app('d', 'Chrome'),
      app('e'), app('f'),
    ];
    const base = buildDefaultLayout(apps);
    // f 被卸载，g 新装
    const next = normalizeLayout(base, [...apps.slice(0, 5), app('g')]);
    const flat = next.pages.flat();
    expect(flat.some((i) => i === 'app:f')).toBe(false);
    expect(flat.some((i) => i === 'app:g')).toBe(true);
    expect(next.dock).toEqual(['a', 'b', 'c', 'd']);
  });

  it('Dock 超容自动截断', () => {
    const base: HomeLayout = {
      version: 1,
      wallpaper: 'hamster',
      pages: [],
      dock: ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'],
      folders: {},
      customized: true,
    };
    const next = normalizeLayout(
      base,
      'abcdefgh'.split('').map((c) => app(c)),
    );
    expect(next.dock).toHaveLength(DOCK_CAPACITY);
  });

  it('空文件夹与其页面引用一并清理', () => {
    const base: HomeLayout = {
      version: 1,
      wallpaper: 'hamster',
      pages: [['folder:f1', 'app:x']],
      dock: [],
      folders: { f1: { name: '空夹', apps: ['gone'] }, f2: { name: '孤儿', apps: ['x'] } },
    };
    const next = normalizeLayout(base, [app('x')]);
    expect(next.folders).toEqual({});
    expect(next.pages.flat()).toEqual(['app:x']);
  });

  it('损坏布局（version 不符）回退默认', () => {
    const next = normalizeLayout({ version: 0 } as unknown as HomeLayout, [app('a', '微信')]);
    expect(next.version).toBe(1);
    expect(next.dock).toEqual(['a']);
  });
});

describe('reorder / moveAcrossPages', () => {
  it('容器内重排', () => {
    expect(reorder(['a', 'b', 'c', 'd'], 0, 2)).toEqual(['b', 'c', 'a', 'd']);
    expect(reorder(['a', 'b', 'c'], 1, 1)).toEqual(['a', 'b', 'c']);
    expect(reorder(['a'], 0, 5)).toEqual(['a']);
  });

  it('跨页移动并压缩空页', () => {
    const pages = [['a', 'b'], ['c']];
    expect(moveAcrossPages(pages, 0, 1, 1, 0)).toEqual([['a'], ['b', 'c']]);
    // 整页拖空后该页消失
    expect(moveAcrossPages([['a'], ['b']], 0, 0, 1, 1)).toEqual([['b', 'a']]);
  });
});

describe('dock 交互', () => {
  const base = (): HomeLayout => ({
    version: 1,
    wallpaper: 'hamster',
    pages: [['app:p1', 'app:p2']],
    dock: ['d1', 'd2'],
    folders: {},
  });

  it('页面应用入 Dock', () => {
    const next = moveToDock(base(), 0, 1);
    expect(next.dock).toEqual(['d1', 'd2', 'p2']);
    expect(next.pages.flat()).toEqual(['app:p1']);
  });

  it('Dock 满时不动', () => {
    const full = { ...base(), dock: ['a', 'b', 'c', 'd', 'e', 'f'] };
    expect(moveToDock(full, 0, 0).dock).toHaveLength(DOCK_CAPACITY);
  });

  it('Dock 移出到最后一页', () => {
    const next = removeFromDock(base(), 0);
    expect(next.dock).toEqual(['d2']);
    expect(next.pages[1]).toEqual(['app:d1']);
  });
});

describe('文件夹合并与删除', () => {
  const base = (): HomeLayout => ({
    version: 1,
    wallpaper: 'hamster',
    pages: [['app:a', 'app:b', 'app:c']],
    dock: [],
    folders: {},
  });

  it('拖到应用上 → 新建文件夹占位在较前位置', () => {
    const next = mergeIntoFolder(base(), 0, 2, 0);
    expect(next.pages[0]).toHaveLength(2);
    const ref = next.pages[0][0];
    expect(ref.startsWith('folder:')).toBe(true);
    const folder = next.folders[ref.slice('folder:'.length)];
    expect(folder.apps).toEqual(['a', 'c']);
  });

  it('拖到已有文件夹 → 追加成员', () => {
    let l = mergeIntoFolder(base(), 0, 1, 0); // a+b 成夹
    const ref = l.pages[0].find((i) => i.startsWith('folder:'))!;
    const before = l.folders[ref.slice('folder:'.length)].apps;
    expect(before).toEqual(['a', 'b']);
    const idx = l.pages[0].indexOf(ref);
    const next = mergeIntoFolder(l, 0, l.pages[0].indexOf('app:c'), idx);
    expect(next.folders[ref.slice('folder:'.length)].apps).toEqual(['a', 'b', 'c']);
  });

  it('删除文件夹 → 成员散落到最后一页', () => {
    const l = mergeIntoFolder(base(), 0, 1, 0);
    const pageIdx = 0;
    const folderIdx = l.pages[0].findIndex((i) => i.startsWith('folder:'));
    const next = removeFromPage(l, pageIdx, folderIdx);
    const flat = next.pages.flat().sort();
    expect(flat).toEqual(['app:a', 'app:b', 'app:c']);
    expect(Object.keys(next.folders)).toHaveLength(0);
  });
});

describe('compactPages', () => {
  it('空页清理且至少留一页', () => {
    expect(compactPages([['a'], [], ['b']])).toEqual([['a'], ['b']]);
    expect(compactPages([[], []])).toEqual([[]]);
  });
});

describe('小组件槽位', () => {
  it('normalizeLayout 保留合法小组件、剔除非法/重复', () => {
    const base: HomeLayout = {
      version: 1,
      wallpaper: 'hamster',
      pages: [['widget:clock', 'widget:hack', 'widget:clock', 'app:x']],
      dock: [],
      folders: {},
      customized: true,
    };
    const next = normalizeLayout(base, [app('x')]);
    expect(next.pages.flat()).toEqual(['widget:clock', 'app:x']);
  });

  it('未定制布局跟随常用度重生：Dock 取排名前 4、页面常用在前', () => {
    const apps = [app('a'), app('b'), app('c'), app('d'), app('e'), app('f')];
    const rank = ['f', 'e', 'd', 'c', 'b', 'a'];
    const l = normalizeLayout({ version: 1, wallpaper: 'ink', pages: [], dock: [], folders: {} }, apps, rank);
    expect(l.dock).toEqual(['f', 'e', 'd', 'c']);
    expect(l.pages[0]).toEqual(['app:b', 'app:a']);
  });

  it('已定制布局不自动重排（customized 冻结），仅做清洗', () => {
    const base: HomeLayout = {
      version: 1,
      wallpaper: 'ink',
      pages: [['app:c', 'app:a']],
      dock: ['b'],
      folders: {},
      customized: true,
    };
    const apps = [app('a'), app('b'), app('c'), app('d')];
    const next = normalizeLayout(base, apps, ['d', 'c', 'b', 'a']);
    // 顺序保持用户排的 c 在 a 前；新装 d 追加到末尾
    expect(next.pages[0]).toEqual(['app:c', 'app:a', 'app:d']);
    expect(next.dock).toEqual(['b']);
  });

  it('addWidget 追加到未满页，满页则开新页', () => {
    let l = buildDefaultLayout([app('a'), app('b'), app('c'), app('d'), app('e'), app('f')]);
    l = addWidget(l, 'weather');
    expect(l.pages[0][l.pages[0].length - 1]).toBe('widget:weather');
    // 装满第一页后添加 → 新页（4 个常见应用进 Dock + 35 页）
    const apps = Array.from({ length: 39 }, (_, i) => app(`k${i}`));
    apps[0] = app('k0', '微信');
    apps[1] = app('k1', 'Chrome');
    apps[2] = app('k2', 'Steam');
    apps[3] = app('k3', '钉钉');
    let full = buildDefaultLayout(apps);
    expect(full.pages).toHaveLength(1);
    full = addWidget(full, 'todo');
    expect(full.pages).toHaveLength(2);
    expect(full.pages[1]).toEqual(['widget:todo']);
  });

  it('编辑模式移除小组件走 removeFromPage（散落逻辑不适用于 widget，直接删除）', () => {
    // d1-d4 是常见应用进 Dock，a/b 在页面上
    let l = buildDefaultLayout([
      app('d1', '微信'),
      app('d2', 'Chrome'),
      app('d3', 'Steam'),
      app('d4', '钉钉'),
      app('a'),
      app('b'),
    ]);
    l = addWidget(l, 'clock');
    const idx = l.pages[0].indexOf('widget:clock');
    const next = removeFromPage(l, 0, idx);
    expect(next.pages[0]).toEqual(['app:a', 'app:b']);
  });
});
