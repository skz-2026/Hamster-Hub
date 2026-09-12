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
import { useAgents, useProjects, useWorkspaces, createStreamSession } from './hooks';
import SelectPill from './SelectPill';
import type { LiveSessionInfo } from '@/shared/types/bench';

const SUGGESTIONS = [
  { icon: FolderTree, label: '梳理仓库结构', prompt: '帮我梳理一下这个仓库的目录结构和核心模块' },
  { icon: Bug, label: '修复报错', prompt: '遇到一个报错，帮我定位并修复：' },
  { icon: FlaskConical, label: '写单元测试', prompt: '为下面的函数补一组单元测试：' },
  { icon: FileSearch, label: '代码审查', prompt: '帮我审查这段代码，指出问题和改进点：' },
];

function greetingLine(now: Date): string {
  const h = now.getHours();
  if (h < 5) return '夜深了，有什么想让我帮忙的吗';
  if (h < 12) return `${h < 9 ? '早上好' : '上午好'}呀，有什么想让我帮忙的吗`;
  if (h < 14) return '中午好，有什么想让我帮忙的吗';
  if (h < 18) return '下午好，有什么想让我帮忙的吗';
  return '晚上好，有什么想让我帮忙的吗';
}

export default function BenchWelcome({ onCreated }: { onCreated: (info: LiveSessionInfo) => void }) {
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
      const picked = window.prompt('输入项目目录路径（浏览器预览层无原生选择器）');
      if (picked && picked.trim()) setProjectDir(picked.trim());
    }
  };

  const send = async () => {
    if (!text.trim() || busy) return;
    if (!agent) {
      setError('没有支持 GUI 对话的代理（需已安装且支持流式通道）');
      return;
    }
    if (!projectDir.trim()) {
      setError('请先选择项目目录');
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
      <h1 className="mt-3 text-center text-[26px] font-medium text-white/90 xl:text-[30px]">{greetingLine(new Date())}</h1>

      {/* 内嵌 composer（不可 overflow-hidden：下拉面板需溢出卡片；宽度随窗口比例伸缩） */}
      <div className="mt-9 w-full max-w-[620px] rounded-2xl bg-[#232228] ring-1 ring-white/10 xl:max-w-[760px] 2xl:max-w-[880px]">
        {/* 项目药丸行 */}
        <div className="flex items-center rounded-t-2xl border-b border-white/6 bg-white/3 px-2.5 py-2">
          <SelectPill
            icon={<FolderOpen size={13} className="text-white/50" />}
            value={projectDir}
            options={projectOptions}
            onChange={setProjectDir}
            placeholder="选择项目目录"
            title={projectDir || '选择项目目录'}
            footer={{
              label: '浏览文件夹…',
              icon: <FolderPlus size={13} className="text-white/50" />,
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
          placeholder="向代理提问，描述你的任务…（Enter 发送，Shift+Enter 换行）"
          className="w-full resize-none bg-transparent px-4 pt-3 text-[13.5px] leading-relaxed text-white outline-none placeholder:text-white/35"
        />

        {/* 底部工具行 */}
        <div className="flex items-center gap-1.5 px-2.5 pb-2.5">
          <button
            title="添加附件（即将支持）"
            disabled
            className="grid size-8 place-items-center rounded-lg text-white/40 hover:bg-white/6 disabled:cursor-not-allowed"
          >
            <Plus size={15} />
          </button>
          <SelectPill
            icon={<Bot size={13} />}
            value={agentId}
            options={streamable.map((a) => ({ value: a.id, label: a.name, hint: a.version ? `v${a.version}` : undefined }))}
            onChange={switchAgent}
            placeholder="选择代理"
            accent
            title="代理（需支持 GUI 流式对话）"
          />
          <div className="ml-auto flex items-center gap-1.5">
            {agent?.launchOptions?.model && (
              <SelectPill
                value={model}
                options={agent.launchOptions.model.choices.map((c) => ({ value: c, label: c }))}
                onChange={setModel}
                placeholder="选择模型"
              />
            )}
            {agent?.launchOptions?.effort && (
              <SelectPill
                value={effort}
                options={agent.launchOptions.effort.choices.map((c) => ({ value: c, label: c }))}
                onChange={setEffort}
                placeholder="推理强度"
              />
            )}
            <button
              onClick={() => void send()}
              disabled={!text.trim() || busy}
              className="ml-1 grid size-9 place-items-center rounded-xl bg-[var(--accent)] text-white transition-all hover:brightness-110 disabled:opacity-35 disabled:hover:brightness-100"
              title="发送并创建会话"
            >
              {busy ? <Loader2 size={16} className="animate-spin" /> : <ArrowUp size={17} strokeWidth={2.4} />}
            </button>
          </div>
        </div>
      </div>

      {error && <p className="mt-3 text-[12px] text-red-300">{error}</p>}

      {/* 建议 chips */}
      <div className="mt-6 flex flex-wrap justify-center gap-2.5">
        {SUGGESTIONS.map(({ icon: Icon, label, prompt }) => (
          <button
            key={label}
            onClick={() => setText(prompt)}
            className="flex items-center gap-1.5 rounded-lg bg-white/5 px-3.5 py-2 text-[12.5px] text-white/70 ring-1 ring-white/8 transition-colors hover:bg-white/10 hover:text-white"
          >
            <Icon size={13} className="text-white/45" />
            {label}
          </button>
        ))}
      </div>
    </div>
  );
}
