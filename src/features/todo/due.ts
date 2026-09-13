/**
 * 截止时间展示辅助：相对日期标签 + 逾期判断（纯函数）。
 * 展示规则：日期任务（00:00）只显日期；带时刻显示 HH:MM；
 * 今天/明天用相对词，逾期加"已逾期"。文案随界面语言（zh 保留中文格式）。
 */
import { getLang, translate } from '@/shared/i18n/core';

const WEEK = ['日', '一', '二', '三', '四', '五', '六'];
const WEEK_EN = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const MONTH_EN = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

export function isOverdue(dueAt: number | null | undefined, now: Date = new Date()): boolean {
  if (!dueAt) return false;
  return dueAt * 1000 < now.getTime();
}

function startOfDay(d: Date): number {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

function hm(d: Date): string {
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

/** "9月18日(五)" / "Sep 18 (Fri)" */
function mdw(d: Date): string {
  if (getLang() === 'en') {
    return `${MONTH_EN[d.getMonth()]} ${d.getDate()} (${WEEK_EN[d.getDay()]})`;
  }
  return `${d.getMonth() + 1}月${d.getDate()}日(${WEEK[d.getDay()]})`;
}

/** 截止时间短标签：null → ''；"明天 14:00" / "Today" / "9月18日(五)" / "Overdue · Sep 11 (Fri)" */
export function dueLabel(dueAt: number | null | undefined, now: Date = new Date()): string {
  if (!dueAt) return '';
  const lang = getLang();
  const d = new Date(dueAt * 1000);
  const dayOffset = Math.round((startOfDay(d) - startOfDay(now)) / 86_400_000);
  const hasClock = d.getHours() !== 0 || d.getMinutes() !== 0;
  const time = hasClock ? ` ${hm(d)}` : '';

  let label: string;
  if (dayOffset === 0) label = `${translate(lang, 'home.due.today')}${time}`;
  else if (dayOffset === 1) label = `${translate(lang, 'home.due.tomorrow')}${time}`;
  else if (dayOffset === -1) label = `${translate(lang, 'home.due.yesterday')}${time}`;
  else label = `${mdw(d)}${time}`;

  return dayOffset < 0 ? translate(lang, 'home.due.overdue', { label }) : label;
}

/** ISO datetime-local 值（<input type="datetime-local">）↔ unixepoch 秒 */
export function toLocalInputValue(dueAt: number | null | undefined): string {
  if (!dueAt) return '';
  const d = new Date(dueAt * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function fromLocalInputValue(value: string): number | null {
  if (!value) return null;
  const ts = new Date(value).getTime();
  return Number.isFinite(ts) ? Math.floor(ts / 1000) : null;
}
