/**
 * timeline-logic 纯函数回归（自 Molto 移植）：工具分组 / 轮次折叠 / 滚动锚定判定。
 */
import { describe, expect, it } from 'vitest';
import {
  buildTimeline,
  classifyToolAction,
  isNearEnd,
  rowsToTimelineMsgs,
  type TimelineMsg,
} from './timeline-logic';

const msg = (over: Partial<TimelineMsg>): TimelineMsg => ({
  seq: 0,
  role: 'user',
  text: '',
  toolName: null,
  at: 1,
  ...over,
});

describe('classifyToolAction', () => {
  it('diff 优先归类为编辑；其余按工具名关键词', () => {
    expect(classifyToolAction('Write', true)).toBe('edit');
    expect(classifyToolAction('Edit', false)).toBe('edit');
    expect(classifyToolAction('fileChange', false)).toBe('edit');
    expect(classifyToolAction('Read', false)).toBe('read');
    expect(classifyToolAction('Bash', false)).toBe('command');
    expect(classifyToolAction('Grep', false)).toBe('search');
    expect(classifyToolAction('mystery', false)).toBe('other');
  });
});

describe('buildTimeline', () => {
  it('连续工具行归组，动作计数正确', () => {
    const rows = buildTimeline(
      [
        msg({ seq: 0, role: 'user', text: '改一下' }),
        msg({ seq: 1, role: 'tool', text: 'a.rs', toolName: 'Read' }),
        msg({ seq: 2, role: 'tool', text: 'b.rs', toolName: 'Edit', diff: { path: 'b.rs', oldText: null, newText: 'x' } }),
        msg({ seq: 3, role: 'tool', text: 'cargo test', toolName: 'Bash' }),
        msg({ seq: 4, role: 'assistant', text: '改好了' }),
      ],
      { working: false, expandedTurns: new Set() },
    );
    const groups = rows.filter((r) => r.kind === 'tool-group');
    expect(groups.length).toBe(1);
    if (groups[0]!.kind !== 'tool-group') return;
    expect(groups[0]!.items.length).toBe(3);
    expect(groups[0]!.actions).toEqual({ read: 1, edit: 1, command: 1 });
  });

  it('单条工具也成组（渲染层对单条直渲染）', () => {
    const rows = buildTimeline(
      [msg({ seq: 0, role: 'user', text: 'hi' }), msg({ seq: 1, role: 'tool', text: 'ls', toolName: 'Bash' })],
      { working: false, expandedTurns: new Set() },
    );
    expect(rows.some((r) => r.kind === 'tool-group')).toBe(true);
  });

  it('历史轮块折叠：header 计数正确，最后一块不折叠；展开后内容可见', () => {
    const msgs: TimelineMsg[] = [
      msg({ seq: 0, role: 'user', text: '第一轮' }),
      msg({ seq: 1, role: 'tool', text: 'ls', toolName: 'Bash' }),
      msg({ seq: 2, role: 'assistant', text: '完成' }),
      msg({ seq: 3, role: 'user', text: '第二轮' }),
      msg({ seq: 4, role: 'tool', text: 'pwd', toolName: 'Bash' }),
      msg({ seq: 5, role: 'assistant', text: '好的' }),
    ];
    const folded = buildTimeline(msgs, { working: false, expandedTurns: new Set() });
    const headers = folded.filter((r) => r.kind === 'turn-header');
    expect(headers.length).toBe(1);
    if (headers[0]!.kind !== 'turn-header') return;
    expect(headers[0]!.index).toBe(1);
    expect(headers[0]!.hiddenCount).toBe(1);
    expect(headers[0]!.folded).toBe(true);
    // 折叠块的工具行被隐藏：第一轮的工具组不出现
    expect(
      folded.some(
        (r) => r.kind === 'tool-group' && r.items.some((i) => i.text === 'ls'),
      ),
    ).toBe(false);
    // 最后一轮永远展开
    expect(folded.some((r) => r.kind === 'msg' && r.m.text === '好的')).toBe(true);

    const expanded = buildTimeline(msgs, { working: false, expandedTurns: new Set(['turn-0']) });
    expect(
      expanded.some(
        (r) => r.kind === 'tool-group' && r.items.some((i) => i.text === 'ls'),
      ),
    ).toBe(true);
  });

  it('纯消息轮块（无工具/思考）不产生 header', () => {
    const rows = buildTimeline(
      [
        msg({ seq: 0, role: 'user', text: 'A' }),
        msg({ seq: 1, role: 'assistant', text: 'B' }),
        msg({ seq: 2, role: 'user', text: 'C' }),
        msg({ seq: 3, role: 'assistant', text: 'D' }),
      ],
      { working: false, expandedTurns: new Set() },
    );
    expect(rows.some((r) => r.kind === 'turn-header')).toBe(false);
  });
});

describe('滚动锚定', () => {
  it('isNearEnd 判定视口贴近底部', () => {
    expect(isNearEnd(1000, 1200, 300)).toBe(true);
    expect(isNearEnd(0, 2000, 300)).toBe(false);
  });
});

describe('rowsToTimelineMsgs', () => {
  it('携带 diff 且过滤空行', () => {
    const out = rowsToTimelineMsgs([
      {
        itemId: '1',
        role: 'assistant',
        text: 'hi',
        at: 1,
        diff: { path: 'a.rs', oldText: null, newText: 'x' },
      },
      { itemId: '2', role: 'thinking', text: '', at: 2 },
    ]);
    expect(out.length).toBe(1);
    expect(out[0]!.diff?.path).toBe('a.rs');
  });
});
