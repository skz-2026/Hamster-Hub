/**
 * BenchWelcome：代理工作台首页（对齐 ZCode 风参考稿）——
 * 平底暗黑 + 大水印 + 居中问候语 + 内嵌 composer（项目/代理/模型/推理强度集成在输入卡）
 * + 建议 chips。发送即创建会话并携带首条 prompt 进入对话。
 */
import { useEffect, useMemo, useState } from 'react';
import {
  ArrowUp,
  Bot,
  Bug,
  FileSearch,
  FlaskConical,
  FolderOpen,
  FolderTree,
  FolderPlus,
  Loader2,
  Plus,
} from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { isTauri } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';
import type { TKey, TParams } from '@/shared/i18n/core';
import { useAgents, useProjects, useWorkspaces, createStreamSession } from './hooks';
import SelectPill from './SelectPill';
import AgentAvatar from './AgentAvatar';
import type { LiveSessionInfo } from '@/shared/types/bench';

type TFn = (key: TKey, params?: TParams) => string;

function greetingLine(now: Date, t: TFn): string {
  const h = now.getHours();
  if (h < 5) return t('bench.welcome.greeting.night');
  if (h < 9) return t('bench.welcome.greeting.earlyMorning');
  if (h < 12) return t('bench.welcome.greeting.morning');
  if (h < 14) return t('bench.welcome.greeting.noon');
  if (h < 18) return t('bench.welcome.greeting.afternoon');
  return t('bench.welcome.greeting.evening');
}

export default function BenchWelcome({ onCreated }: { onCreated: (info: LiveSessionInfo) => void }) {
  const { t } = useI18n();
  const agents = useAgents();
  const projects = useProjects();
  const workspaces = useWorkspaces();
  const [agentId, setAgentId] = useState('');
  const [projectDir, setProjectDir] = useState('');
  const [model, setModel] = useState('');
  const [effort, setEffort] = useState('');
  const [text, setText] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  const streamable = useMemo(
    () => (agents.query.data ?? []).filter((a) => a.installed && a.streaming),
    [agents.query.data],
  );
  const agent = streamable.find((a) => a.id === agentId) ?? null;

  // 快捷 prompt 卡片（label + 发给 AI 的 prompt 正文，均为用户可见文案）
  const suggestions = [
    { icon: FolderTree, label: t('bench.welcome.suggest.repoTitle'), prompt: t('bench.welcome.suggest.repoPrompt') },
    { icon: Bug, label: t('bench.welcome.suggest.fixTitle'), prompt: t('bench.welcome.suggest.fixPrompt') },
    { icon: FlaskConical, label: t('bench.welcome.suggest.testTitle'), prompt: t('bench.welcome.suggest.testPrompt') },
    { icon: FileSearch, label: t('bench.welcome.suggest.reviewTitle'), prompt: t('bench.welcome.suggest.reviewPrompt') },
  ];

  // 项目选项 = 最近使用（Store）∪ 各 Agent 会话记录发现的目录（去重，发现的在前）
  const projectOptions = useMemo(() => {
    const seen = new Set<string>();
    const out: Array<{ value: string; label: string; hint?: string }> = [];
    for (const dir of workspaces.query.data ?? []) {
      if (seen.has(dir.dir)) continue;
      seen.add(dir.dir);
      out.push({ value: dir.dir, label: dir.dir.split('\\').pop() || dir.dir, hint: dir.dir });
    }
    for (const dir of projects.query.data ?? []) {
      if (seen.has(dir)) continue;
      seen.add(dir);
      out.push({ value: dir, label: dir.split('\\').pop() || dir, hint: dir });
    }
    return out;
  }, [workspaces.query.data, projects.query.data]);

  // 预选：首个可用代理 + 其默认模型/推理强度；项目取最近使用
  useEffect(() => {
    if (!agentId && streamable.length > 0) {
      const first = streamable[0];
      setAgentId(first.id);
      setModel(first.launchOptions?.model?.default ?? '');
      setEffort(first.launchOptions?.effort?.default ?? '');
    }
  }, [agentId, streamable]);
  useEffect(() => {
    if (!projectDir && projectOptions.length > 0) {
      setProjectDir(projectOptions[0].value);
    }
  }, [projectDir, projectOptions]);

  const switchAgent = (id: string) => {
    setAgentId(id);
    const a = streamable.find((x) => x.id === id);
    setModel(a?.launchOptions?.model?.default ?? '');
    setEffort(a?.launchOptions?.effort?.default ?? '');
  };

  /** 原生文件夹选择器（浏览器层用 prompt 兜底）；历史列表解决不了时选任意目录 */
  const browseFolder = async () => {
    if (isTauri) {
      const picked = await openDialog({ directory: true, multiple: false });
      if (typeof picked === 'string' && picked.trim()) setProjectDir(picked);
    } else {
      const picked = window.prompt(t('bench.welcome.promptProjectPath'));
      if (picked && picked.trim()) setProjectDir(picked.trim());
    }
  };

  const send = async () => {
    if (!text.trim() || busy) return;
    if (!agent) {
      setError(t('bench.welcome.error.noStreamableAgent'));
      return;
    }
    if (!projectDir.trim()) {
      setError(t('bench.welcome.error.selectProjectDir'));
      return;
    }
    setBusy(true);
    setError('');
    try {
      const info = await createStreamSession({
        agentId: agent.id,
        projectDir: projectDir.trim(),
        firstPrompt: text.trim(),
        model: model || null,
        effort: effort || null,
        resumeKey: null,
      });
      setText('');
      onCreated(info);
    } catch (e) {
      setError(String(e).replace(/^Error:\s*/, ''));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex h-full flex-col items-center justify-center overflow-y-auto px-6 pb-16">
      {/* 品牌水印（完整显示，不与问候语重叠） */}
      <span className="select-none text-[110px] leading-[0.9] opacity-[0.05] xl:text-[150px]" aria-hidden>
        🐹
      </span>
      <h1 className="mt-3 text-center text-[26px] font-medium text-[var(--text)] xl:text-[30px]">{greetingLine(new Date(), t)}</h1>

      {/* 内嵌 composer（不可 overflow-hidden：下拉面板需溢出卡片；宽度随窗口比例伸缩） */}
      <div className="mt-9 w-full max-w-[620px] rounded-2xl bg-[var(--popover)] shadow-[0_8px_32px_rgba(0,0,0,0.12)] ring-1 ring-[var(--border)] xl:max-w-[760px] 2xl:max-w-[880px]">
        {/* 项目药丸行 */}
        <div className="flex items-center rounded-t-2xl border-b border-[var(--border)] bg-[var(--panel)] px-2.5 py-2">
          <SelectPill
            icon={<FolderOpen size={13} className="text-[var(--text-muted)]" />}
            value={projectDir}
            options={projectOptions}
            onChange={setProjectDir}
            placeholder={t('bench.welcome.selectProjectDir')}
            title={projectDir || t('bench.welcome.selectProjectDir')}
            footer={{
              label: t('bench.welcome.browseFolders'),
              icon: <FolderPlus size={13} className="text-[var(--text-muted)]" />,
              onSelect: () => void browseFolder(),
            }}
          />
        </div>

        <textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
              e.preventDefault();
              void send();
            }
          }}
          rows={2}
          autoFocus
          placeholder={t('bench.welcome.composerPlaceholder')}
          className="w-full resize-none bg-transparent px-4 pt-3 text-[13.5px] leading-relaxed text-[var(--text)] outline-none placeholder:text-[var(--text-muted)]"
        />

        {/* 底部工具行 */}
        <div className="flex items-center gap-1.5 px-2.5 pb-2.5">
          <button
            title={t('bench.welcome.addAttachment')}
            disabled
            className="grid size-8 place-items-center rounded-lg text-[var(--text-muted)] hover:bg-[var(--hover)] disabled:cursor-not-allowed"
          >
            <Plus size={15} />
          </button>
          <SelectPill
            icon={
              agent ? (
                <AgentAvatar agentId={agent.id} size={14} />
              ) : (
                <Bot size={13} className="text-[var(--text-muted)]" />
              )
            }
            value={agentId}
            options={streamable.map((a) => ({
              value: a.id,
              label: a.name,
              hint: a.version ? `v${a.version}` : undefined,
              icon: <AgentAvatar agentId={a.id} size={16} />,
            }))}
            onChange={switchAgent}
            placeholder={t('bench.welcome.selectAgent')}
            accent
            title={t('bench.welcome.agentTitle')}
          />
          <div className="ml-auto flex items-center gap-1.5">
            {agent?.launchOptions?.model && (
              <SelectPill
                value={model}
                options={agent.launchOptions.model.choices.map((c) => ({ value: c, label: c }))}
                onChange={setModel}
                placeholder={t('bench.welcome.selectModel')}
              />
            )}
            {agent?.launchOptions?.effort && (
              <SelectPill
                value={effort}
                options={agent.launchOptions.effort.choices.map((c) => ({ value: c, label: c }))}
                onChange={setEffort}
                placeholder={t('bench.welcome.effort')}
              />
            )}
            <button
              onClick={() => void send()}
              disabled={!text.trim() || busy}
              className="ml-1 grid size-9 place-items-center rounded-xl bg-[var(--accent)] text-white transition-all hover:brightness-110 disabled:opacity-35 disabled:hover:brightness-100"
              title={t('bench.welcome.send')}
            >
              {busy ? <Loader2 size={16} className="animate-spin" /> : <ArrowUp size={17} strokeWidth={2.4} />}
            </button>
          </div>
        </div>
      </div>

      {error && <p className="mt-3 text-[12px] text-red-500">{error}</p>}

      {/* 建议 chips */}
      <div className="mt-6 flex flex-wrap justify-center gap-2.5">
        {suggestions.map(({ icon: Icon, label, prompt }) => (
          <button
            key={label}
            onClick={() => setText(prompt)}
            className="flex items-center gap-1.5 rounded-lg bg-[var(--panel)] px-3.5 py-2 text-[12.5px] text-[var(--text-muted)] ring-1 ring-[var(--border)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--text)]"
          >
            <Icon size={13} className="text-[var(--text-muted)]" />
            {label}
          </button>
        ))}
      </div>
    </div>
  );
}
