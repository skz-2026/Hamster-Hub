/**
 * 通知中心纯逻辑（无 React / 无 IPC 依赖，单测覆盖）：
 * 条目模型、环形缓冲（同 id 去重 + 条数上限）、按日分组、相对时间。
 *
 * 文案不进这里：`text` 只承载**用户数据**（待办内容）或**机器枚举值**（focus/break），
 * 标题按 `kind` 在渲染层走 i18n——语言切换后历史条目能同步换语言，不会留下
 * 入列时定格的旧语言快照。
 */

/** 通知类别：待办到点 / 番茄钟完成 / AI Agent 会话结束 / 应用更新就绪 */
export type NotificationKind = 'todo' | 'focus' | 'agent' | 'update';

export interface NotificationItem {
  /** 稳定唯一键：同 id 再次入列视为同一条（替换并置顶，不重复堆叠） */
  id: string;
  kind: NotificationKind;
  /** epoch ms */
  at: number;
  read: boolean;
  /** 用户数据（待办内容）或机器枚举值（番茄钟 kind），渲染层按 kind 解释 */
  text?: string;
  /** 点击跳转目标路由；缺省表示纯告知、不跳转 */
  route?: string;
}

/** 历史上限：超出丢弃最旧（通知中心是近期回看，不是事件日志） */
export const MAX_ITEMS = 50;

const ONE_DAY_MS = 86_400_000;

export type TimeAgo =
  | { unit: 'now' }
  | { unit: 'min'; n: number }
  | { unit: 'hour'; n: number }
  | { unit: 'day'; n: number };

export type DayBucket = 'today' | 'yesterday' | 'earlier';

export interface DayGroup {
  bucket: DayBucket;
  items: NotificationItem[];
}

/** 本地时区当天零点；跨时区/夏令时都由 Date 自行处理 */
export function startOfDay(ts: number): number {
  const d = new Date(ts);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

/** 归入「今天 / 昨天 / 更早」；未来时间（时钟回拨）也归今天 */
export function dayBucket(at: number, now: number): DayBucket {
  // 夏令时会带来 ±1h 偏差，round 后仍是正确的天数差
  const days = Math.round((startOfDay(now) - startOfDay(at)) / ONE_DAY_MS);
  if (days <= 0) return 'today';
  if (days === 1) return 'yesterday';
  return 'earlier';
}

/**
 * 分组：输入「最新在前」的条目列表，输出按 今天 → 昨天 → 更早 三段（空段不出现），
 * 段内保持入参顺序。列表已按时间倒序，故分组用一次遍历即可。
 */
export function groupByDay(list: NotificationItem[], now: number): DayGroup[] {
  const order: DayBucket[] = ['today', 'yesterday', 'earlier'];
  const buckets = new Map<DayBucket, NotificationItem[]>();
  for (const item of list) {
    const bucket = dayBucket(item.at, now);
    const arr = buckets.get(bucket);
    if (arr) arr.push(item);
    else buckets.set(bucket, [item]);
  }
  return order
    .filter((bucket) => buckets.has(bucket))
    .map((bucket) => ({ bucket, items: buckets.get(bucket)! }));
}

/** 入列：最新在前，同 id 替换，超上限截断 */
export function pushItem(list: NotificationItem[], item: NotificationItem): NotificationItem[] {
  return [item, ...list.filter((x) => x.id !== item.id)].slice(0, MAX_ITEMS);
}

export function markRead(list: NotificationItem[], id: string): NotificationItem[] {
  return list.map((x) => (x.id === id && !x.read ? { ...x, read: true } : x));
}

export function markAllRead(list: NotificationItem[]): NotificationItem[] {
  return list.every((x) => x.read) ? list : list.map((x) => ({ ...x, read: true }));
}

export function dismissItem(list: NotificationItem[], id: string): NotificationItem[] {
  return list.filter((x) => x.id !== id);
}

export function unreadCount(list: NotificationItem[]): number {
  return list.reduce((n, x) => n + (x.read ? 0 : 1), 0);
}

/** 相对时间分档（渲染层再套 i18n）：<1 分钟 = 刚刚 */
export function timeAgo(at: number, now: number): TimeAgo {
  const secs = Math.floor((now - at) / 1000);
  if (secs < 60) return { unit: 'now' };
  const mins = Math.floor(secs / 60);
  if (mins < 60) return { unit: 'min', n: mins };
  const hours = Math.floor(mins / 60);
  if (hours < 24) return { unit: 'hour', n: hours };
  return { unit: 'day', n: Math.floor(hours / 24) };
}
