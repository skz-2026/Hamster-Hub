/**
 * 通知中心状态与事件接入。
 *
 * 状态用模块级外部 store（`useSyncExternalStore` 消费，避免为一块面板引入 store 依赖）。
 * 历史落 localStorage：v1 取舍——通知是**派生回看数据**，不值得为它改主库迁移链；
 * M4 agent 后台任务变成需要跨会话追踪的实体后，应迁到 hamsterhub.db（同 hamster-index
 * 的「派生库可重建」思路）。
 *
 * 事件源（全部来自 Rust 侧既有事件，本模块不新增 IPC）：
 *   待办到点 todoReminder · 番茄钟完成 focusFinished · 会话退出 benchStreamExit ·
 *   更新就绪 updateProgress(done)。浏览器预览另接一条 DOM 事件用于调试/回归。
 */
import { useSyncExternalStore } from 'react';
import { events, isTauri } from '@/shared/lib/ipc';
import { getLang, translate } from '@/shared/i18n/core';
import {
  dismissItem,
  markAllRead,
  markRead,
  pushItem,
  unreadCount,
  type NotificationItem,
} from './notifications';

const LS_KEY = 'hamsterhub.notifications.v1';

function load(): NotificationItem[] {
  if (typeof localStorage === 'undefined') return [];
  try {
    const raw = JSON.parse(localStorage.getItem(LS_KEY) ?? '[]');
    return Array.isArray(raw) ? (raw as NotificationItem[]).filter(isItem) : [];
  } catch {
    return [];
  }
}

/** 反序列化防御：localStorage 可被外部改写，坏条目直接丢弃而非渲染崩溃 */
function isItem(value: unknown): value is NotificationItem {
  if (!value || typeof value !== 'object') return false;
  const v = value as Record<string, unknown>;
  return typeof v.id === 'string' && typeof v.at === 'number' && typeof v.kind === 'string';
}

let items: NotificationItem[] = load();
const listeners = new Set<() => void>();

function setItems(next: NotificationItem[]): void {
  if (next === items) return;
  items = next;
  try {
    localStorage.setItem(LS_KEY, JSON.stringify(items));
  } catch {
    /* 隐私模式/配额满：内存态照常工作，不因持久化失败丢功能 */
  }
  listeners.forEach((l) => l());
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** 快照引用稳定：读态变更一律经 setItems 换引用，直接暴露给 useSyncExternalStore */
function getSnapshot(): NotificationItem[] {
  return items;
}

export function useNotifications(): NotificationItem[] {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

export function useUnreadCount(): number {
  return useSyncExternalStore(
    subscribe,
    () => unreadCount(items),
    () => 0,
  );
}

/** 入列一条通知（at 缺省取当前时间） */
export function pushNotification(input: Omit<NotificationItem, 'read' | 'at'> & { at?: number }): void {
  setItems(pushItem(items, { read: false, at: Date.now(), ...input }));
}

export function readNotification(id: string): void {
  setItems(markRead(items, id));
}

export function readAllNotifications(): void {
  setItems(markAllRead(items));
}

export function dismissNotification(id: string): void {
  setItems(dismissItem(items, id));
}

export function clearNotifications(): void {
  setItems([]);
}

let bound = false;

/** 事件接入（幂等：StrictMode 双挂载 / 多处调用只生效一次） */
export function bindNotificationSources(): void {
  if (bound) return;
  bound = true;

  events.todoReminder
    .listen((e) =>
      pushNotification({
        id: `todo-${e.payload.id}`,
        kind: 'todo',
        text: e.payload.content,
        route: '/schedule',
      }),
    )
    .catch(console.error);

  // 会话退出：id 以 session 去重，同一会话重复退出事件不堆叠
  events.benchStreamExit
    .listen((e) =>
      pushNotification({ id: `agent-${e.payload.session_id}`, kind: 'agent', route: '/bench' }),
    )
    .catch(console.error);

  // 番茄钟：text 存机器枚举值（focus | break），渲染层映射文案
  events.focusFinished
    .listen((e) => pushNotification({ id: `focus-${Date.now()}`, kind: 'focus', text: e.payload.kind, route: '/' }))
    .catch(console.error);

  // 更新：仅「已下载就绪」这一态值得打扰用户（进度中不通知）
  events.updateProgress
    .listen((e) => {
      if (e.payload.done) {
        pushNotification({ id: 'update-ready', kind: 'update', route: '/settings' });
      }
    })
    .catch(console.error);

  // 浏览器预览：WebDebugBar 派发 DOM 事件造一条样例通知（与 AppShell 的
  // hamster:open-app-picker 同一套「预览态用 DOM 事件」约定），便于人工验证与 e2e 回归
  if (!isTauri && typeof window !== 'undefined') {
    window.addEventListener('hamster:demo-notification', () => {
      pushNotification({
        id: `demo-${Date.now()}`,
        kind: 'agent',
        text: translate(getLang(), 'notifications.demo.body'),
        route: '/bench',
      });
    });
  }
}
