/**
 * stream-registry 归并语义回归（自 Molto 移植）：ACP 消息增量的聚合（气泡不碎）、
 * 乐观回显采纳、占位行迁移。
 */
import { describe, expect, it } from 'vitest';
import type { StreamEvent } from '@/shared/types/bench';
import {
  adoptPendingRow,
  applyStreamEvent,
  getStreamRows,
  pushUserOptimistic,
} from './stream-registry';

const ev = (over: Partial<StreamEvent>) =>
  applyStreamEvent({
    sessionId: 's1',
    itemId: '',
    at: 1,
    kind: 'agentDelta',
    text: '',
    toolName: null,
    status: null,
    diff: null,
    ...over,
  });

describe('stream-registry acp deltas', () => {
  it('同 messageId 的增量聚合为单一助手行', () => {
    ev({ sessionId: 'agg', itemId: 'msg_1', text: '你好' });
    ev({ sessionId: 'agg', itemId: 'msg_1', text: '！有什么' });
    ev({ sessionId: 'agg', itemId: 'msg_1', text: '可以帮你的吗?' });
    const rows = getStreamRows('agg').filter((r) => r.role === 'assistant');
    expect(rows.length).toBe(1);
    expect(rows[0]!.text).toBe('你好！有什么可以帮你的吗?');
    expect(rows[0]!.streaming).toBe(true);
  });

  it('空 itemId 增量并入最后一条流式助手行（方言未带 messageId 的兜底）', () => {
    ev({ sessionId: 'bare', itemId: '', text: '部' });
    ev({ sessionId: 'bare', itemId: '', text: '分A' });
    ev({ sessionId: 'bare', itemId: '', text: '分B' });
    const rows = getStreamRows('bare').filter((r) => r.role === 'assistant');
    expect(rows.length).toBe(1);
    expect(rows[0]!.text).toBe('部分A分B');
  });

  it('轮次结束收口流式行；下一轮新 messageId 开新行', () => {
    ev({ sessionId: 'turns', itemId: 'msg_a', text: '第一轮' });
    ev({ sessionId: 'turns', kind: 'turnCompleted', text: '' });
    expect(getStreamRows('turns').every((r) => !r.streaming)).toBe(true);
    ev({ sessionId: 'turns', itemId: 'msg_b', text: '第二轮' });
    const rows = getStreamRows('turns').filter((r) => r.role === 'assistant');
    expect(rows.length).toBe(2);
    expect(rows[1]!.text).toBe('第二轮');
  });

  it('UserEcho 到达后采纳并去重同文本乐观行', () => {
    pushUserOptimistic('opt', '帮我改一下');
    expect(getStreamRows('opt').some((r) => r.pending)).toBe(true);
    ev({ sessionId: 'opt', itemId: 'u-1', kind: 'userEcho', text: '帮我改一下' });
    const rows = getStreamRows('opt').filter((r) => r.role === 'user');
    expect(rows.length).toBe(1);
    expect(rows[0]!.pending).toBeUndefined();
  });

  it('adoptPendingRow 搬入占位行并去重', () => {
    pushUserOptimistic('__pending__', '问题 A');
    adoptPendingRow('real-sid');
    expect(getStreamRows('real-sid').some((r) => r.text === '问题 A')).toBe(true);
    expect(getStreamRows('__pending__')).toHaveLength(0);
  });
});
