import { useEffect, useState } from 'react';
import { Solar } from 'lunar-javascript';

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
  const time = `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`;
  const date = new Intl.DateTimeFormat('zh-CN', {
    month: 'long',
    day: 'numeric',
    weekday: 'long',
  }).format(now);

  let lunar = '';
  try {
    const l = Solar.fromJsDate(now).getLunar();
    const jieqi = l.getJieQi();
    const base = `农历${l.getMonthInChinese()}月${l.getDayInChinese()} ${l.getYearInGanZhi()}年`;
    lunar = jieqi ? `${base} · ${jieqi}` : base;
  } catch {
    lunar = '农历';
  }
  return { time, date, lunar };
}

export function greetingOf(now: Date) {
  const h = now.getHours();
  if (h < 5) return '夜深了';
  if (h < 9) return '早上好';
  if (h < 12) return '上午好';
  if (h < 14) return '中午好';
  if (h < 18) return '下午好';
  return '晚上好';
}
