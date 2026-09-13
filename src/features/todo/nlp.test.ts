import { describe, expect, it } from 'vitest';
import { parseQuickAdd } from './nlp';

// 固定参考时刻：2026-09-13（周日）10:00 本地时间
const NOW = new Date(2026, 8, 13, 10, 0, 0);

function secs(d: Date): number {
  return Math.floor(d.getTime() / 1000);
}

describe('parseQuickAdd 中文', () => {
  it('明天 + 时刻', () => {
    const r = parseQuickAdd('明天下午3点交房租', NOW);
    expect(r.content).toBe('交房租');
    expect(r.matched).toBe('明天下午3点');
    const want = secs(new Date(2026, 8, 14, 15, 0, 0));
    expect(r.dueAt).toBe(want);
  });

  it('星期表达滚动到未来（中文默认给本周，常为过去）', () => {
    const r = parseQuickAdd('周五开会', NOW);
    expect(r.content).toBe('开会');
    const got = new Date((r.dueAt ?? 0) * 1000);
    expect(got.getDay()).toBe(5); // 周五
    expect(got.getTime()).toBeGreaterThan(NOW.getTime());
    // 最近的将来周五：2026-09-18 12:00（默认午时）
    expect(got.toDateString()).toBe(new Date(2026, 8, 18, 12, 0).toDateString());
  });

  it('无时间表述原样返回', () => {
    const r = parseQuickAdd('给仓鼠喂粮', NOW);
    expect(r.content).toBe('给仓鼠喂粮');
    expect(r.dueAt).toBeNull();
    expect(r.matched).toBeNull();
  });

  it('过去的时间不作为截止', () => {
    const r = parseQuickAdd('昨天的事', NOW);
    expect(r.dueAt).toBeNull();
  });
});

describe('parseQuickAdd 英文兜底', () => {
  it('tomorrow 5pm', () => {
    const r = parseQuickAdd('pay rent tomorrow 5pm', NOW);
    expect(r.content).toBe('pay rent');
    const want = secs(new Date(2026, 8, 14, 17, 0, 0));
    expect(r.dueAt).toBe(want);
  });

  it('相对小时（today 10 点输入，3 小时后）', () => {
    const r = parseQuickAdd('standup in 2 hours', NOW);
    expect(r.content).toBe('standup');
    const want = secs(new Date(2026, 8, 13, 12, 0, 0));
    expect(r.dueAt).toBe(want);
  });
});

describe('parseQuickAdd 边界', () => {
  it('空串', () => {
    expect(parseQuickAdd('', NOW)).toEqual({ content: '', dueAt: null, matched: null });
  });

  it('剥离后内容为空则保留原句', () => {
    const r = parseQuickAdd('明天', NOW);
    expect(r.content).toBe('明天');
    expect(r.dueAt).not.toBeNull();
  });
});
