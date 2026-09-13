/**
 * 密码箱纯展示逻辑（加密/生成/强度判定在后端 Rust 侧，这里只做呈现辅助）。
 */
import type { TKey } from '@/shared/i18n/core';

/** 强度 0..=4 → 文案 key（与 Rust core::vault::password_strength 分带一致） */
export const STRENGTH_KEYS: readonly TKey[] = [
  'pages.vault.strength.0',
  'pages.vault.strength.1',
  'pages.vault.strength.2',
  'pages.vault.strength.3',
  'pages.vault.strength.4',
] as const;

/** 强度分段的填充色（0 最弱红 → 4 最强绿） */
export function strengthTone(score: number): string {
  return [
    'bg-red-500/80',
    'bg-orange-400/90',
    'bg-amber-300/90',
    'bg-lime-400/90',
    'bg-emerald-400/90',
  ][Math.max(0, Math.min(4, score))];
}

/** 自动锁定选项（秒；后端 clamp 30..=3600） */
export const AUTOLOCK_OPTIONS: readonly number[] = [60, 300, 900, 1800, 3600];

export function autolockKey(secs: number): TKey {
  return `pages.vault.autolock.${secs}` as TKey;
}

/** 条目头像字母（取首个字符，兼容中文/emoji 的码点边界） */
export function initialOf(title: string): string {
  const first = Array.from(title.trim())[0];
  return first ? first.toUpperCase() : '?';
}

/** 标题 → 稳定色相（头像底色，同一条目不变色） */
export function avatarHue(title: string): number {
  let h = 0;
  for (const ch of title) h = (h * 31 + ch.codePointAt(0)!) % 360;
  return h;
}

/** 后端 AppError 按 code 识别（VAULT_AUTH / VAULT_LOCKED …） */
export function vaultErrIs(e: unknown, code: string): boolean {
  return typeof e === 'object' && e != null && (e as { code?: string }).code === code;
}
