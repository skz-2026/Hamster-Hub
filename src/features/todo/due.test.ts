import { describe, expect, it } from 'vitest';
import { dueLabel, isOverdue, toLocalInputValue, fromLocalInputValue } from './due';

// 2026-09-13（周日）10:00
const NOW = new Date(2026, 8, 13, 10, 0, 0);
const s = (d: Date) => Math.floor(d.getTime() / 1000);

describe('dueLabel', () => {
  it('无截止为空串', () => {
    expect(dueLabel(null, NOW)).toBe('');
    expect(dueLabel(undefined, NOW)).toBe('');
  });

  it('今天 / 明天相对词 + 时刻', () => {
    expect(dueLabel(s(new Date(2026, 8, 13, 14, 0)), NOW)).toBe('今天 14:00');
    expect(dueLabel(s(new Date(2026, 8, 14, 9, 5)), NOW)).toBe('明天 09:05');
  });

  it('日期任务（00:00）不显示时刻', () => {
    expect(dueLabel(s(new Date(2026, 8, 13, 0, 0)), NOW)).toBe('今天');
  });

  it('未来日期显示月日+周', () => {
    expect(dueLabel(s(new Date(2026, 8, 18, 12, 0)), NOW)).toBe('9月18日(五) 12:00');
  });

  it('昨天与逾期', () => {
    expect(dueLabel(s(new Date(2026, 8, 12, 9, 0)), NOW)).toBe('已逾期 · 昨天 09:00');
    expect(dueLabel(s(new Date(2026, 8, 11, 0, 0)), NOW)).toBe('已逾期 · 9月11日(五)');
  });
});

describe('isOverdue', () => {
  it('过期/未来/空', () => {
    expect(isOverdue(s(new Date(2026, 8, 13, 9, 0)), NOW)).toBe(true);
    expect(isOverdue(s(new Date(2026, 8, 13, 10, 0)), NOW)).toBe(false);
    expect(isOverdue(s(new Date(2026, 8, 20, 9, 0)), NOW)).toBe(false);
    expect(isOverdue(null, NOW)).toBe(false);
  });
});

describe('datetime-local 互转', () => {
  it('roundtrip 保留到分钟', () => {
    const dueAt = s(new Date(2026, 8, 14, 15, 30, 12));
    const v = toLocalInputValue(dueAt);
    expect(v).toBe('2026-09-14T15:30');
    expect(fromLocalInputValue(v)).toBe(s(new Date(2026, 8, 14, 15, 30, 0)));
  });

  it('空值', () => {
    expect(toLocalInputValue(null)).toBe('');
    expect(fromLocalInputValue('')).toBeNull();
  });
});
