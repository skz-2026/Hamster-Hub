import { describe, expect, it } from 'vitest';
import {
  DEFAULT_TILES,
  addTile,
  cycleSpan,
  moveTile,
  normalizeLayout,
  pluginIdOfTile,
  removeTile,
  type TileSpec,
} from './layout';

describe('dashboard normalizeLayout', () => {
  it('空库回默认布局', () => {
    const l = normalizeLayout(null);
    expect(l.tiles).toEqual([...DEFAULT_TILES]);
    expect(l.customized).toBe(false);
  });

  it('坏数据（缺 span / 未知类型 / 重复项）被清洗，合法项保留原序', () => {
    const l = normalizeLayout({
      customized: true,
      tiles: [
        { type: 'weather', span: 1 },
        { type: 'weather', span: 2 }, // 重复：丢
        { type: 'hacker', span: 1 }, // 未知：丢
        { type: 'todo', span: 3 }, // 非法档位：丢
        { type: 'plugin:demo', span: 2 }, // 插件卡合法
      ] as unknown as TileSpec[],
    });
    expect(l.tiles).toEqual([
      { type: 'weather', span: 1 },
      { type: 'plugin:demo', span: 2 },
    ]);
    expect(l.customized).toBe(true);
  });
});

describe('dashboard moveTile', () => {
  it('前移/后移均按插入语义', () => {
    const t = DEFAULT_TILES.map((x) => x.type);
    // [0] 拖到末尾
    expect(moveTile(DEFAULT_TILES, 0, 5).map((x) => x.type)).toEqual([
      t[1], t[2], t[3], t[4], t[5], t[0],
    ]);
    // 末尾拖到首位
    expect(moveTile(DEFAULT_TILES, 5, 0).map((x) => x.type)).toEqual([
      t[5], t[0], t[1], t[2], t[3], t[4],
    ]);
  });

  it('同位/越界时布局内容不变', () => {
    expect(moveTile(DEFAULT_TILES, 2, 2)).toEqual(DEFAULT_TILES);
    expect(moveTile(DEFAULT_TILES, -1, 0)).toEqual(DEFAULT_TILES);
    expect(moveTile(DEFAULT_TILES, 0, 99)).toEqual(DEFAULT_TILES);
  });
});

describe('dashboard add/remove/cycleSpan', () => {
  it('添加新卡追加到末尾，已存在的卡幂等忽略', () => {
    const added = addTile(DEFAULT_TILES, 'sysinfo');
    expect(added.at(-1)).toEqual({ type: 'sysinfo', span: 1 });
    expect(addTile(added, 'sysinfo')).toEqual(added);
  });

  it('移除按索引，越界原样返回', () => {
    expect(removeTile([{ type: 'todo', span: 1 }], 0)).toEqual([]);
    expect(removeTile(DEFAULT_TILES, 99)).toEqual(DEFAULT_TILES);
  });

  it('档位切换 1↔2 且不动其它卡', () => {
    const next = cycleSpan([{ type: 'weather', span: 1 }, { type: 'todo', span: 2 }], 0);
    expect(next).toEqual([{ type: 'weather', span: 2 }, { type: 'todo', span: 2 }]);
  });
});

describe('dashboard pluginIdOfTile', () => {
  it('插件卡取 id，内置卡返回 null', () => {
    expect(pluginIdOfTile('plugin:hello-hamster')).toBe('hello-hamster');
    expect(pluginIdOfTile('weather')).toBeNull();
  });
});
