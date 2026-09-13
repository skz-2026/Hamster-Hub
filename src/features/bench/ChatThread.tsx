/**
 * ChatThread：assistant-ui Thread 的仓鼠Hub 风格组合。
 * 视口自动滚动 + 用户/助手气泡 + 底部 composer（运行中显示停止按钮）。
 * 消息 part 渲染由 ChatParts 提供；数据源由外层 AssistantRuntimeProvider 注入。
 *
 * composerHost：可选外置挂载点——composer 经 portal 渲染进去（React context 穿透
 * portal，ComposerPrimitive 照常工作），供 /agent 页把输入框统一停靠在页面底部。
 */
import { ArrowDown, ArrowUp, Square } from 'lucide-react';
import { ComposerPrimitive, ThreadPrimitive } from '@assistant-ui/react';
import { createPortal } from 'react-dom';
import { useI18n } from '@/shared/i18n/provider';
import { AssistantMessage, UserMessage } from './ChatParts.message';

/** 线程空态（无消息时的欢迎面） */
function ThreadEmpty() {
  const { t } = useI18n();
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
      <span className="grid size-14 place-items-center rounded-2xl bg-[var(--panel)] text-2xl ring-1 ring-[var(--border)]">🐹</span>
      <div>
        <p className="text-[14px] font-medium text-[var(--text)]">{t('bench.chat.emptyTitle')}</p>
        <p className="mt-1 text-[12px] text-[var(--text-muted)]">{t('bench.chat.emptyHint')}</p>
      </div>
    </div>
  );
}

export default function ChatThread({
  composerHost,
  placeholder,
}: {
  composerHost?: HTMLElement | null;
  placeholder?: string;
}) {
  const { t } = useI18n();
  const resolvedPlaceholder = placeholder ?? t('bench.chat.composerPlaceholder');
  const composer = (
    <ComposerPrimitive.Root className="flex items-end gap-2.5 rounded-[22px] bg-[var(--panel-strong)] p-2 pl-5 ring-1 ring-[var(--border)] backdrop-blur-xl focus-within:ring-[var(--accent)]/40">
      <ComposerPrimitive.Input
        rows={1}
        autoFocus
        placeholder={resolvedPlaceholder}
        className="max-h-28 min-h-[28px] flex-1 resize-none bg-transparent py-1.5 text-[13.5px] text-[var(--text)] outline-none placeholder:text-[var(--text-muted)]"
      />
      <ComposerPrimitive.Cancel className="grid size-9 shrink-0 place-items-center rounded-full bg-[var(--panel)] text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--text)]">
        <Square size={13} strokeWidth={2.4} />
      </ComposerPrimitive.Cancel>
      <ComposerPrimitive.Send className="grid size-9 shrink-0 place-items-center rounded-full bg-[var(--accent)] text-white transition-all hover:brightness-110 disabled:opacity-35 disabled:hover:brightness-100">
        <ArrowUp size={17} strokeWidth={2.4} />
      </ComposerPrimitive.Send>
    </ComposerPrimitive.Root>
  );

  return (
    <ThreadPrimitive.Root className="h-full">
      <div className="relative flex h-full flex-col">
        <ThreadPrimitive.Viewport autoScroll className="min-h-0 flex-1 space-y-3 overflow-y-auto px-5 py-4">
          <ThreadPrimitive.Empty>
            <ThreadEmpty />
          </ThreadPrimitive.Empty>
          <ThreadPrimitive.Messages components={{ UserMessage, AssistantMessage }} />
        </ThreadPrimitive.Viewport>

        {/* 回到底部悬浮按钮（composer 内嵌时让开其高度） */}
        <div
          className={`pointer-events-none absolute inset-x-0 flex justify-center ${
            composerHost ? 'bottom-4' : 'bottom-[96px]'
          }`}
        >
          <ThreadPrimitive.ScrollToBottom asChild>
            <button className="pointer-events-auto grid size-8 place-items-center rounded-full bg-[var(--panel-strong)] text-[var(--text-muted)] ring-1 ring-[var(--border)] backdrop-blur transition-colors hover:text-[var(--text)]">
              <ArrowDown size={15} />
            </button>
          </ThreadPrimitive.ScrollToBottom>
        </div>

        {/* 输入栏（含运行中停止）；外置挂载点存在时 portal 过去，视觉上并入宿主布局 */}
        {composerHost ? createPortal(composer, composerHost) : <div className="shrink-0 px-5 pb-4">{composer}</div>}
      </div>
    </ThreadPrimitive.Root>
  );
}
