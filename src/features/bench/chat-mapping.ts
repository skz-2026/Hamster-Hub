/**
 * StreamRow[] → assistant-ui ThreadMessageLike[] 映射（chat-mapping）：
 * 行与消息 1:1 对应；工具行 → tool-call part（text/status/diff 装进 result payload），
 * 错误行 → data part（name=error），流式中的行 → running status。
 * 轮次进行中且末尾无流式行时追加「工作中」指示消息（对齐 上游 working 行）。
 */
import type { ThreadMessageLike } from '@assistant-ui/react';
import type { StreamRow } from '@/shared/types/bench';

export function rowsToThreadMessages(rows: StreamRow[], isRunning: boolean): ThreadMessageLike[] {
  const visible = rows.filter((r) => r.text || r.role === 'tool');
  const messages: ThreadMessageLike[] = visible.map((r, i) => {
    const id = r.itemId || `m-${i}`;
    const status = r.streaming ? { type: 'running' as const } : { type: 'complete' as const, reason: 'stop' as const };
    if (r.role === 'user') {
      return { role: 'user', id, content: r.text, createdAt: new Date(r.at) };
    }
    if (r.role === 'assistant') {
      return { role: 'assistant', id, content: [{ type: 'text', text: r.text }], status, createdAt: new Date(r.at) };
    }
    if (r.role === 'thinking') {
      return { role: 'assistant', id, content: [{ type: 'reasoning', text: r.text }], status, createdAt: new Date(r.at) };
    }
    if (r.role === 'tool') {
      return {
        role: 'assistant',
        id,
        content: [
          {
            type: 'tool-call',
            toolCallId: id,
            toolName: r.toolName ?? 'tool',
            args: {},
            argsText: '',
            result: { text: r.text, status: r.status ?? null, diff: r.diff ?? null },
          },
        ],
        status,
        createdAt: new Date(r.at),
      };
    }
    // system / error 行
    return { role: 'assistant', id, content: [{ type: 'data', name: 'error', data: r.text }], status, createdAt: new Date(r.at) };
  });

  const last = visible[visible.length - 1];
  const hasStreamingTail = last?.role === 'assistant' || last?.role === 'thinking';
  if (isRunning && !hasStreamingTail) {
    messages.push({ role: 'assistant', id: 'working', content: [{ type: 'data', name: 'working', data: true }], status: { type: 'running' } });
  }
  return messages;
}

/** AppendMessage → 纯文本（onNew 用） */
export function appendMessageText(content: readonly { type: string; text?: string }[]): string {
  return content
    .map((p) => (p.type === 'text' ? (p.text ?? '') : ''))
    .join('')
    .trim();
}
