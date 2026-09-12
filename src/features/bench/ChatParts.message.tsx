/**
 * 消息级组件（ThreadPrimitive.Messages 的 UserMessage / AssistantMessage）：
 * 用户右对齐气泡；助手为左对齐容器，part 分发到 ChatParts 渲染器。
 */
import { MessagePrimitive } from '@assistant-ui/react';
import { ErrorCard, MarkdownText, ThinkingCard, ToolCard, WorkingDots } from './ChatParts';

export function UserMessage() {
  return (
    <div className="flex justify-end">
      <div className="max-w-[72%] rounded-2xl rounded-br-md bg-[var(--accent)]/85 px-4 py-2.5 text-[13.5px] leading-relaxed text-white">
        <MessagePrimitive.Parts components={{ Text: UserText }} />
      </div>
    </div>
  );
}

export function AssistantMessage() {
  return (
    <div className="flex justify-start">
      <div className="max-w-[85%] space-y-2">
        <MessagePrimitive.Parts
          components={{
            Text: MarkdownText,
            Reasoning: ThinkingCard,
            tools: { Fallback: ToolCard },
            data: { by_name: { error: ErrorCard, working: WorkingDots } },
          }}
        />
      </div>
    </div>
  );
}

function UserText({ text }: { text: string }) {
  return <div className="whitespace-pre-wrap">{text}</div>;
}
