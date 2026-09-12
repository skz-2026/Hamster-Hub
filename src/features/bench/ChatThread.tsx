/**
 * ChatThread：assistant-ui Thread 的仓鼠Hub 风格组合。
 * 视口自动滚动 + 用户/助手气泡 + 底部 composer（运行中显示停止按钮）。
 * 消息 part 渲染由 ChatParts 提供；数据源由外层 AssistantRuntimeProvider 注入。
 */
import { ArrowDown, ArrowUp, Square } from 'lucide-react';
import { ComposerPrimitive, ThreadPrimitive } from '@assistant-ui/react';
import { AssistantMessage, UserMessage } from './ChatParts.message';

/** 线程空态（无消息时的欢迎面） */
function ThreadEmpty() {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
      <span className="grid size-14 place-items-center rounded-2xl bg-white/6 text-2xl ring-1 ring-white/10">🐹</span>
      <div>
        <p className="text-[14px] font-medium text-white/85">开始与编码代理对话</p>
        <p className="mt-1 text-[12px] text-white/45">选择代理与项目目录后，发送第一条消息</p>
      </div>
    </div>
  );
}

export default function ChatThread() {
  return (
    <ThreadPrimitive.Root className="h-full">
      <div className="relative flex h-full flex-col">
        <ThreadPrimitive.Viewport autoScroll className="min-h-0 flex-1 space-y-3 overflow-y-auto px-5 py-4">
          <ThreadPrimitive.Empty>
            <ThreadEmpty />
          </ThreadPrimitive.Empty>
          <ThreadPrimitive.Messages components={{ UserMessage, AssistantMessage }} />
        </ThreadPrimitive.Viewport>

        {/* 回到底部悬浮按钮 */}
        <div className="pointer-events-none absolute inset-x-0 bottom-[96px] flex justify-center">
          <ThreadPrimitive.ScrollToBottom asChild>
            <button className="pointer-events-auto grid size-8 place-items-center rounded-full bg-white/10 text-white/70 ring-1 ring-white/15 backdrop-blur transition-colors hover:bg-white/20 hover:text-white">
              <ArrowDown size={15} />
            </button>
          </ThreadPrimitive.ScrollToBottom>
        </div>

        {/* 输入栏（含运行中停止） */}
        <div className="shrink-0 px-5 pb-4">
          <ComposerPrimitive.Root className="flex items-end gap-2.5 rounded-[22px] bg-black/30 p-2 pl-5 ring-1 ring-white/12 backdrop-blur-xl focus-within:ring-white/25">
            <ComposerPrimitive.Input
              rows={1}
              autoFocus
              placeholder="给代理发消息…（Enter 发送，Shift+Enter 换行）"
              className="max-h-28 min-h-[28px] flex-1 resize-none bg-transparent py-1.5 text-[13.5px] text-white outline-none placeholder:text-white/40"
            />
            <ComposerPrimitive.Cancel className="grid size-9 shrink-0 place-items-center rounded-full bg-white/12 text-white transition-colors hover:bg-white/20">
              <Square size={13} strokeWidth={2.4} />
            </ComposerPrimitive.Cancel>
            <ComposerPrimitive.Send className="grid size-9 shrink-0 place-items-center rounded-full bg-[var(--accent)] text-white transition-all hover:brightness-110 disabled:opacity-35 disabled:hover:brightness-100">
              <ArrowUp size={17} strokeWidth={2.4} />
            </ComposerPrimitive.Send>
          </ComposerPrimitive.Root>
        </div>
      </div>
    </ThreadPrimitive.Root>
  );
}
