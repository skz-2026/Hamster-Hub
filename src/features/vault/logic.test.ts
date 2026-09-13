import { describe, expect, it } from 'vitest';
import { AUTOLOCK_OPTIONS, avatarHue, autolockKey, initialOf, strengthTone, vaultErrIs } from './logic';

describe('vault.logic', () => {
  it('initialOf 取首个字符并大写（兼容中文与 emoji 码点）', () => {
    expect(initialOf('GitHub')).toBe('G');
    expect(initialOf('知乎')).toBe('知');
    expect(initialOf('🎮 游戏平台')).toBe('🎮');
    expect(initialOf('  spaced  ')).toBe('S');
    expect(initialOf('')).toBe('?');
  });

  it('avatarHue 稳定且落在色环内', () => {
    expect(avatarHue('GitHub')).toBe(avatarHue('GitHub'));
    expect(avatarHue('GitHub')).not.toBe(avatarHue('GitLab'));
    for (const h of [avatarHue('a'), avatarHue('知'), avatarHue('🎮')]) {
      expect(h).toBeGreaterThanOrEqual(0);
      expect(h).toBeLessThan(360);
    }
  });

  it('strengthTone 夹紧到 0..=4', () => {
    expect(strengthTone(0)).toContain('red');
    expect(strengthTone(4)).toContain('emerald');
    expect(strengthTone(9)).toBe(strengthTone(4));
    expect(strengthTone(-3)).toBe(strengthTone(0));
  });

  it('自动锁定选项 key 与词典一致', () => {
    expect(AUTOLOCK_OPTIONS.every((s) => autolockKey(s).startsWith('pages.vault.autolock.'))).toBe(true);
  });

  it('vaultErrIs 按 code 匹配 AppError 形状', () => {
    expect(vaultErrIs({ code: 'VAULT_AUTH', message: '主密码不正确' }, 'VAULT_AUTH')).toBe(true);
    expect(vaultErrIs({ code: 'VALIDATE', message: 'x' }, 'VAULT_AUTH')).toBe(false);
    expect(vaultErrIs(new Error('boom'), 'VAULT_AUTH')).toBe(false);
    expect(vaultErrIs(null, 'VAULT_AUTH')).toBe(false);
  });
});
