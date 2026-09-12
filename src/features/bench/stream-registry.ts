/**
 * 流式会话注册表（自 上游 stream-registry.ts 移植为 React 版）：
 * StreamEvent → 渲染行的唯一归属地。行数据常驻内存，视图重建/切换时取回，零丢失。
 * 事件累积为「行」，增量按 itemId 拼接；与 Vue 版的差异仅在响应式实现——
 * 这里用订阅通知 + 写时复制（每次变更产生新数组/新行引用）适配 useSyncExternalStore。
 */
import type { StreamEvent, StreamRow } from '@/shared/types/bench';

/** sessionId → 行数组（数组引用仅在变更时替换，保证 React 快照稳定） */
const rowsBySession = new Map<string, StreamRow[]>();

/** 轮次进行中（turnStarted → turnCompleted）；会话存活 ≠ 正在工作 */
const turnRunningBySession = new Set<string>();

type Listener = () => void;
const listenersBySession = new Map<string, Set<Listener>>();

/** 每会话单调递增版本号：任何事件（含无行变更的 turn 事件）都推进，驱动 React 重渲染 */
const versionBySession = new Map<string, number>();

function emit(sessionId: string): void {
  versionBySession.set(sessionId, (versionBySession.get(sessionId) ?? 0) + 1);
  listenersBySession.get(sessionId)?.forEach((fn) => fn());
}

/** 订阅某会话的行变更（React: useSyncExternalStore(subscribe, getVersion)） */
export function subscribeStreamRows(sessionId: string, fn: Listener): () => void {
  let set = listenersBySession.get(sessionId);
  if (!set) {
    set = new Set();
    listenersBySession.set(sessionId, set);
  }
  set.add(fn);
  return () => set?.delete(fn);
}

/** 会话数据版本号（useSyncExternalStore 的快照值） */
export function getStreamVersion(sessionId: string): number {
  return versionBySession.get(sessionId) ?? 0;
}

export function getStreamRows(sessionId: string): StreamRow[] {
  let rows = rowsBySession.get(sessionId);
  if (!rows) {
    rows = [];
    rowsBySession.set(sessionId, rows);
  }
  return rows;
}

export function dropStreamRows(sessionId: string): void {
  rowsBySession.delete(sessionId);
  turnRunningBySession.delete(sessionId);
  emit(sessionId);
}

/** 该会话是否有进行中的轮次（流式视图的「工作中」依据） */
export function isTurnRunning(sessionId: string): boolean {
  return turnRunningBySession.has(sessionId);
}

/**
 * 索引播种（resume 场景）：把索引里的历史消息搬入 registry 作为基线行，
 * 新轮次的流式增量在其后追加。仅在 registry 为空时执行一次。
 */
export function seedStreamRows(
  sessionId: string,
  messages: Array<{ seq: number; role: string; text: string; toolName: string | null; at: number | null }>,
): void {
  const rows = rowsBySession.get(sessionId);
  if (rows && rows.length > 0) return;
  const next: StreamRow[] = messages.map((m) => ({
    itemId: `idx-${m.seq}`,
    role: m.role as StreamRow['role'],
    text: m.text,
    toolName: m.toolName ?? undefined,
    at: m.at ?? Date.now(),
  }));
  rowsBySession.set(sessionId, next);
  emit(sessionId);
}

const PENDING_SID = '__pending__';

let optimisticSeq = 0;

/** 乐观回显：发送瞬间上屏；UserEcho 到达后采纳权威行（按文本去重） */
export function pushUserOptimistic(sessionId: string, text: string): void {
  optimisticSeq += 1;
  appendRow(sessionId, {
    itemId: `optimistic-${optimisticSeq}`,
    role: 'user',
    text,
    at: Date.now(),
    pending: true,
  });
}

/**
 * 会话 id 就绪后迁移占位行（创建期间事件可能已带真实 id 先行到达：
 * 文本与既有权威 user 行相同的占位行丢弃，其余搬入）。
 */
export function adoptPendingRow(sessionId: string): void {
  const pending = rowsBySession.get(PENDING_SID);
  if (!pending?.length) return;
  const target = getStreamRows(sessionId);
  const moved = pending.filter(
    (r) => !target.some((t) => t.role === 'user' && !t.pending && t.text === r.text),
  );
  rowsBySession.set(sessionId, [...target, ...moved]);
  rowsBySession.delete(PENDING_SID);
  emit(sessionId);
}

function appendRow(sessionId: string, row: StreamRow): void {
  rowsBySession.set(sessionId, [...getStreamRows(sessionId), row]);
  emit(sessionId);
}

/** 按 itemId 找行（itemId 为空或找不到返回 null） */
function findRow(sessionId: string, itemId: string): { row: StreamRow; index: number } | null {
  if (!itemId) return null;
  const list = rowsBySession.get(sessionId);
  if (!list) return null;
  for (let i = list.length - 1; i >= 0; i--) {
    if (list[i].itemId === itemId) return { row: list[i], index: i };
  }
  return null;
}

/**
 * 空 itemId 增量的归属（方言未带 messageId 的兜底）：并入该角色最后一条
 * 仍在流式中的行——同一轮内同角色只应有一条流式行；找不到才新建。
 */
function findStreamingTailIndex(sessionId: string, role: 'assistant' | 'thinking'): number {
  const list = rowsBySession.get(sessionId);
  if (!list) return -1;
  for (let i = list.length - 1; i >= 0; i--) {
    if (list[i].role === role && list[i].streaming) return i;
  }
  return -1;
}

/** 替换指定下标的行（写时复制：新数组 + 新行对象） */
function patchRow(
  sessionId: string,
  index: number,
  patch: (row: StreamRow) => StreamRow,
): void {
  const list = rowsBySession.get(sessionId);
  if (!list) return;
  const next = list.slice();
  next[index] = patch(list[index]);
  rowsBySession.set(sessionId, next);
  emit(sessionId);
}

/**
 * 事件归并（幂等语义：Done 事件覆盖该行全文；delta 追加）。
 * UserEcho 首次出现即入列（后续重复忽略——codex 只发一次，防御）。
 */
export function applyStreamEvent(ev: StreamEvent): void {
  const sid = ev.sessionId;
  switch (ev.kind) {
    case 'userEcho': {
      if (findRow(sid, ev.itemId)) break;
      // 丢弃同文本的乐观行（权威事件采纳）
      const list = rowsBySession.get(sid);
      if (list) {
        rowsBySession.set(
          sid,
          list.filter((r) => !(r.role === 'user' && r.pending && r.text === ev.text)),
        );
      }
      appendRow(sid, { itemId: ev.itemId, role: 'user', text: ev.text, at: ev.at });
      break;
    }
    case 'agentDelta': {
      const hit = findRow(sid, ev.itemId);
      if (hit) {
        patchRow(sid, hit.index, (r) => ({ ...r, text: r.text + ev.text }));
      } else {
        const tail = findStreamingTailIndex(sid, 'assistant');
        if (tail >= 0) patchRow(sid, tail, (r) => ({ ...r, text: r.text + ev.text }));
        else
          appendRow(sid, {
            itemId: ev.itemId,
            role: 'assistant',
            text: ev.text,
            at: ev.at,
            streaming: true,
          });
      }
      break;
    }
    case 'agentDone': {
      const hit = findRow(sid, ev.itemId);
      if (hit) {
        patchRow(sid, hit.index, (r) => ({ ...r, text: ev.text, streaming: false }));
      } else {
        appendRow(sid, { itemId: ev.itemId, role: 'assistant', text: ev.text, at: ev.at });
      }
      break;
    }
    case 'reasoningDelta': {
      const hit = findRow(sid, ev.itemId);
      if (hit) {
        patchRow(sid, hit.index, (r) => ({ ...r, text: r.text + ev.text }));
      } else {
        const tail = findStreamingTailIndex(sid, 'thinking');
        if (tail >= 0) patchRow(sid, tail, (r) => ({ ...r, text: r.text + ev.text }));
        else
          appendRow(sid, {
            itemId: ev.itemId,
            role: 'thinking',
            text: ev.text,
            at: ev.at,
            streaming: true,
          });
      }
      break;
    }
    case 'reasoningDone': {
      const hit = findRow(sid, ev.itemId);
      if (hit) patchRow(sid, hit.index, (r) => ({ ...r, text: ev.text, streaming: false }));
      break;
    }
    case 'toolItem': {
      const hit = findRow(sid, ev.itemId);
      if (hit) {
        patchRow(sid, hit.index, (r) => ({
          ...r,
          text: ev.text,
          status: ev.status ?? undefined,
          diff: ev.diff ?? r.diff,
        }));
      } else {
        appendRow(sid, {
          itemId: ev.itemId,
          role: 'tool',
          text: ev.text,
          toolName: ev.toolName ?? undefined,
          status: ev.status ?? undefined,
          at: ev.at,
          diff: ev.diff ?? null,
        });
      }
      break;
    }
    case 'error': {
      appendRow(sid, { itemId: ev.itemId, role: 'system', text: ev.text, at: ev.at });
      break;
    }
    case 'turnStarted': {
      turnRunningBySession.add(sid);
      emit(sid);
      break;
    }
    case 'turnCompleted': {
      turnRunningBySession.delete(sid);
      // ACP 无条目 Done 事件：轮次结束即收口全部流式行（去掉输入光标，
      // 并让空 id 兜底不会把下一轮增量并进上一轮的气泡）
      const list = rowsBySession.get(sid);
      if (list?.some((r) => r.streaming)) {
        rowsBySession.set(
          sid,
          list.map((r) => (r.streaming ? { ...r, streaming: false } : r)),
        );
      }
      emit(sid);
      break;
    }
    case 'exit': {
      turnRunningBySession.delete(sid);
      emit(sid);
      break;
    }
    default:
      break;
  }
}
