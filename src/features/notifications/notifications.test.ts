import { describe, expect, it } from 'vitest';
import {
  MAX_ITEMS,
  dayBucket,
  dismissItem,
  groupByDay,
  markAllRead,
  markRead,
  pushItem,
  startOfDay,
  timeAgo,
  unreadCount,
  type NotificationItem,
} from './notifications';

const at = (ts: number, id = `n-${ts}`): NotificationItem => ({
  id,
  kind: 'agent',
  at: ts,
  read: false,
});

describe('pushItem', () => {
  it('最新在前', () => {
    const list = pushItem(pushItem([], at(1000, 'a')), at(2000, 'b'));
    expect(list.map((x) => x.id)).toEqual(['b', 'a']);
  });

  it('同 id 替换并置顶，不重复堆叠', () => {
    const first = pushItem([], at(1000, 'a'));
    const list = pushItem(pushItem(first, at(2000, 'b')), { ...at(3000, 'a'), read: true });
    expect(list.map((x) => x.id)).toEqual(['a', 'b']);
    expect(list[0].read).toBe(true);
    expect(list[0].at).toBe(3000);
  });

  it('超上限截断最旧', () => {
    let list: NotificationItem[] = [];
    for (let i = 0; i < MAX_ITEMS + 10; i += 1) list = pushItem(list, at(1000 + i, `n${i}`));
    expect(list).toHaveLength(MAX_ITEMS);
    expect(list[0].id).toBe(`n${MAX_ITEMS + 9}`);
    expect(list.some((x) => x.id === 'n0')).toBe(false);
  });
});

describe('读态操作', () => {
  it('markRead 只改目标且保持不可变', () => {
    const list = [at(2000, 'a'), at(1000, 'b')];
    const next = markRead(list, 'b');
    expect(next.map((x) => x.read)).toEqual([false, true]);
    expect(list.every((x) => !x.read)).toBe(true);
  });

  it('markAllRead 全已读后原样返回（引用不变，避免无谓重渲染）', () => {
    const list = [at(2000, 'a'), at(1000, 'b')];
    const all = markAllRead(list);
    expect(all.every((x) => x.read)).toBe(true);
    expect(markAllRead(all)).toBe(all);
  });

  it('dismissItem 命中删除、未命中原样', () => {
    const list = [at(2000, 'a'), at(1000, 'b')];
    expect(dismissItem(list, 'a').map((x) => x.id)).toEqual(['b']);
    expect(dismissItem(list, 'zzz')).toHaveLength(2);
  });

  it('unreadCount 只数未读', () => {
    const list = [at(3000, 'a'), { ...at(2000, 'b'), read: true }, at(1000, 'c')];
    expect(unreadCount(list)).toBe(2);
    expect(unreadCount([])).toBe(0);
  });
});

describe('dayBucket / groupByDay', () => {
  const now = new Date(2026, 8, 13, 10, 0, 0).getTime(); // 2026-09-13 10:00 本地

  it('今天 / 昨天 / 更早 的天界划分（跨午夜）', () => {
    const day = (d: number, h = 0, m = 0, s = 0, ms = 0) => new Date(2026, 8, d, h, m, s, ms).getTime();
    expect(startOfDay(now)).toBe(day(13));
    expect(dayBucket(now, now)).toBe('today');
    // 今天 00:00:00.001 仍是今天
    expect(dayBucket(day(13, 0, 0, 0, 1), now)).toBe('today');
    // 昨天 23:59:59.999 与昨天 00:00 都归昨天
    expect(dayBucket(day(12, 23, 59, 59, 999), now)).toBe('yesterday');
    expect(dayBucket(day(12), now)).toBe('yesterday');
    // 前天 23:59:59.999 与昨天 00:00 只差 1ms，但必须落在不同桶
    expect(dayBucket(day(11, 23, 59, 59, 999), now)).toBe('earlier');
    expect(dayBucket(day(11), now)).toBe('earlier');
  });

  it('未来时间（时钟回拨）归今天', () => {
    expect(dayBucket(now + 3600_000, now)).toBe('today');
  });

  it('分组保序、空段不出现、段内保持入参顺序', () => {
    const list = [
      at(new Date(2026, 8, 13, 9, 0).getTime(), 'today-1'),
      at(new Date(2026, 8, 13, 8, 0).getTime(), 'today-2'),
      at(new Date(2026, 8, 12, 9, 0).getTime(), 'yesterday-1'),
      at(new Date(2026, 8, 1, 9, 0).getTime(), 'earlier-1'),
    ];
    const groups = groupByDay(list, now);
    expect(groups.map((g) => g.bucket)).toEqual(['today', 'yesterday', 'earlier']);
    expect(groups[0].items.map((x) => x.id)).toEqual(['today-1', 'today-2']);
    expect(groups[1].items.map((x) => x.id)).toEqual(['yesterday-1']);
  });

  it('只有今天的条目时只出一段', () => {
    const groups = groupByDay([at(now)], now);
    expect(groups).toHaveLength(1);
    expect(groups[0].bucket).toBe('today');
  });
});

describe('timeAgo', () => {
  const now = 1_000_000_000_000;

  it('分档：刚刚 / 分钟 / 小时 / 天', () => {
    expect(timeAgo(now - 1_000, now)).toEqual({ unit: 'now' });
    expect(timeAgo(now - 59_000, now)).toEqual({ unit: 'now' });
    expect(timeAgo(now - 60_000, now)).toEqual({ unit: 'min', n: 1 });
    expect(timeAgo(now - 59 * 60_000, now)).toEqual({ unit: 'min', n: 59 });
    expect(timeAgo(now - 60 * 60_000, now)).toEqual({ unit: 'hour', n: 1 });
    expect(timeAgo(now - 23 * 3600_000, now)).toEqual({ unit: 'hour', n: 23 });
    expect(timeAgo(now - 24 * 3600_000, now)).toEqual({ unit: 'day', n: 1 });
  });

  it('未来时间不退化成负数', () => {
    expect(timeAgo(now + 5_000, now)).toEqual({ unit: 'now' });
  });
});
