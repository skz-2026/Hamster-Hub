import { useEffect, useState } from 'react';
import { Solar } from 'lunar-javascript';
import { getLang, translate } from '@/shared/i18n/core';

export function useClock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const t = setInterval(() => setNow(new Date()), 1000);
    return () => clearInterval(t);
  }, []);
  return now;
}

const pad = (n: number) => String(n).padStart(2, '0');

/** 时间 + 公历日期 + 农历（本地计算，无网络） */
export function useDateTimeInfo(now: Date) {
  const lang = getLang();
  const time = `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`;
  const date = new Intl.DateTimeFormat(lang, {
    month: 'long',
    day: 'numeric',
    weekday: 'long',
  }).format(now);

  let lunar = '';
  try {
    const l = Solar.fromJsDate(now).getLunar();
    const jieqi = l.getJieQi();
    // 农历月日为文化词汇：中文保留汉字写法，en 用数字写法（Lunar {month}/{day}）。
    // 库里有 getMonth/getDay 但 shims.d.ts 只声明了最小面，这里做窄化访问，不改公共 shim。
    const numeric = l as unknown as { getMonth(): number; getDay(): number };
    const month = lang === 'en' ? String(numeric.getMonth()) : l.getMonthInChinese();
    const day = lang === 'en' ? String(numeric.getDay()) : l.getDayInChinese();
    const base = translate(lang, 'pages.workbench.lunarDate', {
      month,
      day,
      year: l.getYearInGanZhi(),
    });
    lunar = jieqi ? `${base} · ${jieqi}` : base;
  } catch {
    lunar = translate(lang, 'pages.workbench.lunarFallback');
  }
  return { time, date, lunar };
}

export function greetingOf(now: Date) {
  const h = now.getHours();
  const lang = getLang();
  if (h < 5) return translate(lang, 'pages.greeting.lateNight');
  if (h < 9) return translate(lang, 'pages.greeting.earlyMorning');
  if (h < 12) return translate(lang, 'pages.greeting.morning');
  if (h < 14) return translate(lang, 'pages.greeting.noon');
  if (h < 18) return translate(lang, 'pages.greeting.afternoon');
  return translate(lang, 'pages.greeting.evening');
}
