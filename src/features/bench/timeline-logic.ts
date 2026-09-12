/**
 * 原生视图时间线逻辑（纯函数，自 Molto timeline-logic.ts 1:1 移植）：
 * 工具行分组折叠（①）、历史轮次折叠（③）、滚动锚定辅助（②）。
 * 渲染形态归 TimelineView；本文件不碰 React 与 DOM。
 */
import type { StreamEventDiff } from '@/shared/types/bench';
import type { StreamRow } from '@/shared/types/bench';

/** 时间线输入消息：流式 = registry 行；索引 = 消息投影（diff 恒空） */
export interface TimelineMsg {
  seq: number;
  role: string;
  text: string;
  toolName?: string | null;
  at?: number | null;
  streaming?: boolean;
  diff?: StreamEventDiff | null;
}

/** 工具动作归类 */
export type ToolAction = 'read' | 'edit' | 'command' | 'search' | 'other';

const KEYWORDS: Array<[ToolAction, string[]]> = [
  ['edit', ['edit', 'write', 'filechange', 'apply_patch', 'patch', 'notebookedit']],
  ['read', ['read', 'glob', 'image_view', 'fileread', 'view']],
  ['command', ['bash', 'command', 'commandexecution', 'terminal', 'execute']],
  ['search', ['grep', 'search', 'fetch', 'websearch']],
];

export function classifyToolAction(name: string | null | undefined, hasDiff: boolean): ToolAction {
  if (hasDiff) return 'edit';
  const n = (name ?? '').toLowerCase();
  for (const [action, keys] of KEYWORDS) {
    if (keys.some((k) => n.includes(k))) return action;
  }
  return 'other';
}

export type TimelineRow =
  | { kind: 'msg'; m: TimelineMsg }
  | { kind: 'thinking'; items: TimelineMsg[]; ms: number | null }
  | {
      kind: 'tool-group';
      /** 稳定键（展开态存取用）：块序 + 首个成员 seq */
      id: string;
      items: TimelineMsg[];
      actions: Partial<Record<ToolAction, number>>;
    }
  | { kind: 'turn-header'; id: string; index: number; hiddenCount: number; folded: boolean }
  | { kind: 'working' };

export interface BuildTimelineOptions {
  /** 流式轮次进行中 → 时间线末尾追加「工作中」行 */
  working: boolean;
  /** 已展开的历史轮块 id 集合（未在集合中的历史轮块折叠工具/思考行） */
  expandedTurns: ReadonlySet<string>;
}

interface TurnBlock {
  items: TimelineMsg[];
  /** 块内工具 + 思考行数（折叠时隐藏的数量） */
  workCount: number;
}

/** 用户消息即轮次边界：其后的工具/思考归属该轮 */
function splitTurnBlocks(msgs: TimelineMsg[]): TurnBlock[] {
  const blocks: TurnBlock[] = [{ items: [], workCount: 0 }];
  for (const m of msgs) {
    if (m.role === 'user' && blocks[blocks.length - 1]!.items.length > 0) {
      blocks.push({ items: [], workCount: 0 });
    }
    const block = blocks[blocks.length - 1]!;
    block.items.push(m);
    if (m.role === 'tool' || m.role === 'thinking') block.workCount += 1;
  }
  return blocks;
}

function mergeToolActions(items: TimelineMsg[]): Partial<Record<ToolAction, number>> {
  const actions: Partial<Record<ToolAction, number>> = {};
  for (const m of items) {
    const action = classifyToolAction(m.toolName, Boolean(m.diff));
    actions[action] = (actions[action] ?? 0) + 1;
  }
  return actions;
}

/**
 * 时间线组装：
 * - 连续 thinking → 单张思考卡（含时长）
 * - 连续 tool → 工具组卡（动作计数汇总，默认收起）
 * - 历史轮块（非最后一块且有工具/思考行）→ turn-header 分隔 + 默认折叠其
 *   工具/思考行，user/assistant 消息保留可见；header 点击展开/收起
 */
export function buildTimeline(msgs: TimelineMsg[], opts: BuildTimelineOptions): TimelineRow[] {
  const rows: TimelineRow[] = [];
  const blocks = splitTurnBlocks(msgs);
  const lastBlockIndex = blocks.length - 1;

  blocks.forEach((block, blockIndex) => {
    const isPast = blockIndex < lastBlockIndex;
    const foldId = `turn-${blockIndex}`;
    const folded = isPast && block.workCount > 0 && !opts.expandedTurns.has(foldId);
    if (isPast && block.workCount > 0) {
      rows.push({
        kind: 'turn-header',
        id: foldId,
        index: blockIndex + 1,
        hiddenCount: block.workCount,
        folded,
      });
    }

    let groupIndex = 0;
    let pending: TimelineRow[] = [];
    const flush = (): void => {
      for (const row of pending) {
        if (folded) continue;
        rows.push(row);
      }
      pending = [];
    };

    for (const m of block.items) {
      const last = pending[pending.length - 1];
      if (m.role === 'thinking') {
        if (last && last.kind === 'thinking') {
          last.items!.push(m);
          const firstAt = last.items![0].at;
          last.ms = firstAt != null && m.at != null ? m.at - firstAt : null;
        } else {
          flush();
          pending.push({ kind: 'thinking', items: [m], ms: null });
        }
        continue;
      }
      if (m.role === 'tool') {
        if (last && last.kind === 'tool-group') {
          last.items!.push(m);
          last.actions = mergeToolActions(last.items!);
        } else {
          flush();
          pending.push({
            kind: 'tool-group',
            id: `g-${blockIndex}-${groupIndex++}-${m.seq}`,
            items: [m],
            actions: mergeToolActions([m]),
          });
        }
        continue;
      }
      flush();
      rows.push({ kind: 'msg', m });
    }
    flush();
  });

  if (opts.working) rows.push({ kind: 'working' });
  return rows;
}

/** 滚动锚定（②）：视口是否贴近底部（跟随流式增长的判定阈值） */
export function isNearEnd(
  scrollTop: number,
  scrollHeight: number,
  clientHeight: number,
  threshold = 96,
): boolean {
  return scrollHeight - scrollTop - clientHeight < threshold;
}

/** registry 行 → 时间线输入（流式路径；seq 用数组下标，registry 行只追加不重排） */
export function rowsToTimelineMsgs(rows: StreamRow[]): TimelineMsg[] {
  return rows
    .filter((r) => r.text || r.role === 'tool')
    .map((r, i) => ({
      seq: i,
      role: r.role,
      text: r.text,
      toolName: r.toolName ?? null,
      at: r.at,
      streaming: r.streaming ?? false,
      diff: r.diff ?? null,
    }));
}
