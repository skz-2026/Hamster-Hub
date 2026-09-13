import { describe, expect, it } from 'vitest';
import { formatMmss } from './fmt';

describe('formatMmss', () => {
  it('分钟秒补零', () => {
    expect(formatMmss(0)).toBe('00:00');
    expect(formatMmss(65)).toBe('01:05');
    expect(formatMmss(25 * 60)).toBe('25:00');
  });

  it('负数按 0 处理', () => {
    expect(formatMmss(-3)).toBe('00:00');
  });
});
