/**
 * 自然语言快速添加解析：从一句话里抽出截止时间。
 * 策略：zh.strict（精确）→ zh.casual（相对语）→ en.casual（英文兜底）。
 * 中文星期表达指向"本周"（常为过去）：无过去词时向前滚动 7 天；
 * casual 对"周五下午"类（星期+时段无钟点）会误解析，用星期一致性检查拦截。
 * 纯函数：输入原句 + 参考时刻，输出剥离时间后的内容与截止时刻。
 */
import { zh, en, type ParsedResult } from 'chrono-node';

export interface QuickParse {
  /** 剥离时间表述后的任务内容 */
  content: string;
  /** 截止时刻（unixepoch 秒；null = 未解析出未来时间） */
  dueAt: number | null;
  /** 命中的时间表述原文（用于 UI 预览 chip） */
  matched: string | null;
}

/** 已过去的解析结果视为无截止（容忍 1 分钟时钟误差） */
const PAST_TOLERANCE_MS = 60_000;
const WEEK_MS = 7 * 24 * 3600_000;

/** 明确指过去的词：命中则不向前滚动（"上周五"/"昨天"/"3天前"） */
const PAST_WORDS = /上周|上个星期|星期之前|昨天|前天|年前|个月前|天前|小时前|分钟前|ago|yesterday|last /i;

/** 提取匹配文本里的星期（周X/星期X/礼拜X → JS getDay 值；一=1 … 六=6，日/天=0） */
function weekdayIn(text: string): number | null {
  const m = text.match(/[周星期礼拜]\s*([一二三四五六日天])/);
  if (!m) return null;
  const i = '一二三四五六日天'.indexOf(m[1]);
  return i >= 6 ? 0 : i + 1;
}

export function parseQuickAdd(input: string, now: Date = new Date()): QuickParse {
  const text = input.trim();
  if (!text) return { content: '', dueAt: null, matched: null };

  const hit: ParsedResult | null =
    zh.strict.parse(text, now)[0] ??
    zh.casual.parse(text, now)[0] ??
    en.casual.parse(text, now)[0] ??
    null;
  if (!hit?.start) return { content: text, dueAt: null, matched: null };

  // casual 的"周五下午"类误解析：解析出的星期与文本星期不符则放弃
  const wantWd = weekdayIn(hit.text);
  if (wantWd !== null && hit.start.date().getDay() !== wantWd) {
    return { content: text, dueAt: null, matched: null };
  }

  let due = hit.start.date();
  const saidPast = PAST_WORDS.test(hit.text);
  if (!saidPast) {
    // "周五"指最近的将来那个周五（中文默认给本周，常为过去）
    while (due.getTime() < now.getTime() - PAST_TOLERANCE_MS) {
      due = new Date(due.getTime() + WEEK_MS);
    }
  } else if (due.getTime() <= now.getTime() - PAST_TOLERANCE_MS) {
    return { content: text, dueAt: null, matched: null };
  }

  // 剥掉时间表述，合并多余空白
  const content = (text.slice(0, hit.index) + text.slice(hit.index + hit.text.length))
    .replace(/\s{2,}/g, ' ')
    .trim();
  return {
    content: content || text,
    dueAt: Math.floor(due.getTime() / 1000),
    matched: hit.text,
  };
}
