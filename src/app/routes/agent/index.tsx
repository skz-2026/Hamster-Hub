/**
 * AI 助手页（M4「Agent 原生桌面」入口）：
 * - 桌面助手（默认）：bench 会话驱动（本机 Agent + hamster-desktop MCP 工具），
 *   能实际操作这台电脑 —— AssistantPanel；
 * - 快问（兜底）：OpenAI 兼容直连（ai_chat），未装 Agent / 微任务的轻路径。
 *
 * 底部为**统一输入栏**：模式切换贴在输入框上方，两种模式共用同一只输入框、
 * 同一位置——桌面助手会话建立后，bench ChatThread 的 composer 经 portal 渲染进
 * 底部槽位（能力不变：流式/停止/续问），视觉与位置不跳变。
 * 空会话（欢迎态）时整组（欢迎语 + toggle + 输入框）垂直居中；开聊后落回底部。
 * Spotlight「问 AI」经 agent.pendingQ 进入本页，优先路由桌面助手。
 */
import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowUp, Eraser, Loader2, Settings2, Sparkles, TriangleAlert } from 'lucide-react';
import { commands } from '@/shared/lib/ipc';
import AssistantPanel, { type AssistantPanelHandle } from '@/features/agent/AssistantPanel';

interface Msg {
  role: 'user' | 'assistant';
  content: string;
}

const SYSTEM_PROMPT =
  '你是仓鼠Hub 桌面助手内置的 AI 助手，回答简洁、友好、实用，默认使用中文。';

type Mode = 'assistant' | 'quick';

export default function AgentPage() {
  const navigate = useNavigate();
  const [mode, setMode] = useState<Mode>('assistant');
  const [pendingQ, setPendingQ] = useState<string | null>(null);
  // 桌面助手状态镜像（AssistantPanel 上报）：决定底部槽位渲染 portal 还是统一输入框
  const [assistantSt, setAssistantSt] = useState({ hasSession: false, starting: false });
  const [composerHost, setComposerHost] = useState<HTMLDivElement | null>(null);
  const assistantRef = useRef<AssistantPanelHandle>(null);

  // 快问模式状态
  const [messages, setMessages] = useState<Msg[]>([]);
  const [input, setInput] = useState('');
  const [busy, setBusy] = useState(false);
  const [unconfigured, setUnconfigured] = useState(false);
  const listRef = useRef<HTMLDivElement>(null);
  const taRef = useRef<HTMLTextAreaElement>(null);

  // 新消息自动滚到底（快问模式）
  useEffect(() => {
    listRef.current?.scrollTo({ top: listRef.current.scrollHeight, behavior: 'smooth' });
  }, [messages, busy, mode]);

  const sendQuick = async () => {
    const text = input.trim();
    if (!text || busy) return;
    setInput('');
    setUnconfigured(false);
    const next: Msg[] = [...messages, { role: 'user', content: text }];
    setMessages(next);
    setBusy(true);
    try {
      const reply = await commands.aiChat([
        { role: 'system', content: SYSTEM_PROMPT },
        ...next.map((m) => ({ role: m.role, content: m.content })),
      ]);
      setMessages([...next, { role: 'assistant', content: reply }]);
    } catch (e) {
      if (String(e).includes('AI_UNCONFIGURED')) {
        setUnconfigured(true);
      } else {
        setMessages([
          ...next,
          { role: 'assistant', content: `请求失败：${String(e).slice(0, 200)}` },
        ]);
      }
    } finally {
      setBusy(false);
    }
  };

  // 统一输入框发送：快问走 ai_chat；桌面助手未开会话时用首问启动会话
  // （已开会话时输入框是 ChatThread portal 过来的，不经过这里）
  const composerSend = () => {
    if (mode === 'quick') {
      void sendQuick();
      return;
    }
    const text = input.trim();
    if (!text || assistantSt.starting) return;
    assistantRef.current?.startSession(text);
    setInput('');
  };

  // Spotlight「问 AI」入口：读取暂存问题（一次性），优先路由桌面助手
  useEffect(() => {
    commands
      .kvGet('agent.pendingQ')
      .then((q) => {
        if (q && q.trim()) {
          commands.kvSet('agent.pendingQ', '').catch(() => {});
          setPendingQ(q.trim());
          setMode('assistant');
        }
      })
      .catch(() => {});
  }, []);

  // 空会话（欢迎态）：整组居中；开聊后输入栏落回底部
  const isEmpty =
    mode === 'assistant' ? !assistantSt.hasSession : messages.length === 0 && !unconfigured;

  const showUnified = mode === 'quick' || !assistantSt.hasSession;
  const sendDisabled = !input.trim() || (mode === 'quick' ? busy : assistantSt.starting);

  // 模式切换药丸（桌面助手 / 快问）
  const modeToggle = (
    <div className="flex rounded-full bg-black/30 p-0.5 ring-1 ring-white/10">
      {(
        [
          ['assistant', '桌面助手'],
          ['quick', '快问'],
        ] as const
      ).map(([m, label]) => (
        <button
          key={m}
          onClick={() => setMode(m)}
          className={`rounded-full px-3 py-1 text-[11.5px] transition-colors ${
            mode === m
              ? 'bg-[var(--accent)] font-medium text-white'
              : 'text-white/55 hover:text-white/85'
          }`}
        >
          {label}
        </button>
      ))}
    </div>
  );

  // 统一输入框（两种模式同一只；助手会话中由 portal 接管此槽位）
  const unifiedComposer = showUnified && (
    <div className="flex items-end gap-2.5 rounded-[22px] bg-black/30 p-2 pl-5 ring-1 ring-white/12 backdrop-blur-xl focus-within:ring-white/25">
      <textarea
        ref={taRef}
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
            e.preventDefault();
            composerSend();
          }
        }}
        rows={1}
        placeholder={
          mode === 'quick'
            ? '问点什么…（Enter 发送，Shift+Enter 换行）'
            : '给桌面助手下达任务…（如「把「周五交周报」记成待办，然后打开计算器」）'
        }
        className="max-h-28 min-h-[28px] flex-1 resize-none bg-transparent py-1.5 text-[13.5px] text-white outline-none placeholder:text-white/40"
      />
      <button
        onClick={composerSend}
        disabled={sendDisabled}
        title={mode === 'quick' ? '发送' : '启动桌面助手'}
        className="grid size-9 shrink-0 place-items-center rounded-full bg-[var(--accent)] text-white transition-all hover:brightness-110 disabled:opacity-35 disabled:hover:brightness-100"
      >
        {(mode === 'quick' ? busy : assistantSt.starting) ? (
          <Loader2 size={16} className="animate-spin" />
        ) : (
          <ArrowUp size={17} strokeWidth={2.4} />
        )}
      </button>
    </div>
  );

  /** 输入组：toggle 在上 + composer 槽位；hero 居中时收窄并水平居中 */
  const composerArea = (centered: boolean) => (
    <div className={centered ? 'w-full max-w-2xl' : ''}>
      <div className={`mb-2 flex gap-2 ${centered ? 'justify-center' : 'items-center'}`}>
        {modeToggle}
        {mode === 'quick' && messages.length > 0 && (
          <button
            onClick={() => {
              setMessages([]);
              setUnconfigured(false);
            }}
            className="flex items-center gap-1.5 rounded-full bg-white/8 px-3 py-1.5 text-[11.5px] text-white/70 ring-1 ring-white/10 transition-colors hover:bg-white/15 hover:text-white"
          >
            <Eraser size={12} />
            清空会话
          </button>
        )}
        {mode === 'assistant' && assistantSt.hasSession && (
          <span className="ml-auto flex items-center gap-1.5 text-[11px] text-white/40">
            <span className="size-1.5 rounded-full bg-emerald-400/80" />
            助手会话进行中（结束按钮在会话头部）
          </span>
        )}
      </div>
      {/* composer 槽位：助手会话中 ChatThread 经 portal 渲染进来 */}
      <div ref={setComposerHost} />
      {unifiedComposer}
    </div>
  );

  return (
    <div className="relative flex h-full flex-col overflow-hidden rounded-2xl bg-[radial-gradient(120%_120%_at_20%_0%,#232032_0%,#16141d_55%,#1a1520_100%)] text-white">
      {/* 背景光晕 */}
      <div className="pointer-events-none absolute -right-24 -top-24 size-96 rounded-full bg-[var(--accent)] opacity-[0.08] blur-3xl" />
      <div className="pointer-events-none absolute -bottom-32 -left-24 size-96 rounded-full bg-indigo-400 opacity-[0.07] blur-3xl" />

      {/* 顶栏 */}
      <header className="relative z-10 flex shrink-0 items-center gap-2.5 px-6 py-4">
        <span className="grid size-9 place-items-center rounded-xl bg-[var(--accent-weak)] ring-1 ring-white/10">
          <Sparkles size={17} className="text-[var(--accent)]" />
        </span>
        <div>
          <h1 className="text-[15px] font-semibold leading-tight">AI 助手</h1>
          <p className="text-[11px] text-white/45">
            {mode === 'assistant' ? 'Agent 驱动 · 可操作桌面' : '快问 · AI 直连补全'}
          </p>
        </div>
      </header>

      {/* 主体：AssistantPanel 单实例（跨布局切换保持会话状态），容器随空/非空换布局 */}
      {mode === 'assistant' ? (
        <>
          <div
            className={`relative z-10 min-h-0 flex-1 px-6 ${
              isEmpty ? 'flex flex-col items-center justify-center gap-6 pb-10' : 'pt-1'
            }`}
          >
            <AssistantPanel
              ref={assistantRef}
              autoQuestion={pendingQ}
              onConsumed={() => setPendingQ(null)}
              onFallback={() => setMode('quick')}
              composerHost={composerHost}
              onStateChange={setAssistantSt}
            />
            {isEmpty && composerArea(true)}
          </div>
          {!isEmpty && (
            <div className="relative z-10 shrink-0 px-6 pb-4 pt-2">{composerArea(false)}</div>
          )}
        </>
      ) : isEmpty ? (
        /* 空会话：欢迎内容 + toggle + 输入框成组垂直居中 */
        <div className="relative z-10 flex min-h-0 flex-1 flex-col items-center justify-center gap-6 px-8 pb-10">
          <div className="flex flex-col items-center gap-3 text-center">
            <span className="grid size-14 place-items-center rounded-2xl bg-white/6 ring-1 ring-white/10">
              <Sparkles size={24} className="text-[var(--accent)]" />
            </span>
            <div>
              <p className="text-[14px] font-medium text-white/85">有什么可以帮你？</p>
              <p className="mt-1 text-[12px] text-white/45">
                问答、写作、翻译、点子……配置模型后即可开始
              </p>
            </div>
            <div className="mt-1 flex flex-wrap justify-center gap-2">
              {['帮我写一条周报开头', '推荐几个效率工具', '解释一下什么是 MCP'].map((s) => (
                <button
                  key={s}
                  onClick={() => {
                    setInput(s);
                    taRef.current?.focus();
                  }}
                  className="rounded-full bg-white/8 px-3.5 py-1.5 text-[11.5px] text-white/70 ring-1 ring-white/10 transition-colors hover:bg-white/15 hover:text-white"
                >
                  {s}
                </button>
              ))}
            </div>
          </div>
          {composerArea(true)}
        </div>
      ) : (
        <>
          {/* 消息区 */}
          <div className="relative z-10 min-h-0 flex-1 px-6 pt-1">
            <div ref={listRef} className="h-full space-y-4 overflow-y-auto pb-3">
              {messages.map((m, i) => (
                <div key={i} className={`flex ${m.role === 'user' ? 'justify-end' : 'justify-start'}`}>
                  <div
                    className={`max-w-[72%] whitespace-pre-wrap rounded-2xl px-4 py-2.5 text-[13.5px] leading-relaxed ${
                      m.role === 'user'
                        ? 'rounded-br-md bg-[var(--accent)]/85 text-white'
                        : 'rounded-bl-md bg-white/8 text-white/90 ring-1 ring-white/10'
                    }`}
                  >
                    {m.content}
                  </div>
                </div>
              ))}

              {busy && (
                <div className="flex justify-start">
                  <div className="flex items-center gap-1.5 rounded-2xl rounded-bl-md bg-white/8 px-4 py-3 ring-1 ring-white/10">
                    {[0, 1, 2].map((i) => (
                      <span
                        key={i}
                        className="size-1.5 animate-bounce rounded-full bg-white/60"
                        style={{ animationDelay: `${i * 140}ms` }}
                      />
                    ))}
                  </div>
                </div>
              )}

              {unconfigured && (
                <div className="flex justify-center">
                  <div className="flex items-center gap-3 rounded-2xl bg-amber-400/10 px-5 py-4 ring-1 ring-amber-300/25">
                    <TriangleAlert size={18} className="shrink-0 text-amber-300" />
                    <div className="text-[12.5px] leading-relaxed text-white/85">
                      还没有配置 AI 模型——先到设置页填写 API 地址、模型名与 Key。
                    </div>
                    <button
                      onClick={() => navigate('/settings')}
                      className="flex shrink-0 items-center gap-1.5 rounded-full bg-amber-300/20 px-3.5 py-1.5 text-[12px] font-medium text-amber-200 transition-colors hover:bg-amber-300/30"
                    >
                      <Settings2 size={13} />
                      去配置
                    </button>
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* 底部输入栏：toggle 贴输入框上方 */}
          <div className="relative z-10 shrink-0 px-6 pb-4 pt-2">{composerArea(false)}</div>
        </>
      )}
    </div>
  );
}
