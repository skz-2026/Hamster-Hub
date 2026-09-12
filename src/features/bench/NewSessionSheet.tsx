/**
 * NewSessionSheet：新建 GUI 流式会话 —— 选代理（需支持流式通道）+ 项目目录 +
 * 可选模型/推理强度 + 首条 prompt。浏览器层目录为手填（真机走 dialog 插件）。
 */
import { useEffect, useMemo, useState } from 'react';
import { Bot, FolderOpen, Loader2, X } from 'lucide-react';
import type { AgentInfo, LiveSessionInfo } from '@/shared/types/bench';
import { useAgents, useProjects, createStreamSession } from './hooks';

function AgentCard({ a, selected, onSelect }: { a: AgentInfo; selected: boolean; onSelect: () => void }) {
  const disabled = !a.installed || !a.streaming;
  return (
    <button
      onClick={() => !disabled && onSelect()}
      disabled={disabled}
      className={`flex items-center gap-2.5 rounded-xl px-3 py-2.5 text-left ring-1 transition-all ${
        selected ? 'bg-[var(--accent-weak)] ring-[var(--accent)]/50' : 'ring-white/10 hover:bg-white/6'
      } ${disabled ? 'cursor-not-allowed opacity-40' : ''}`}
    >
      <span className="grid size-8 shrink-0 place-items-center rounded-lg bg-white/8 ring-1 ring-white/10">
        <Bot size={15} className="text-[var(--accent)]" />
      </span>
      <div className="min-w-0">
        <p className="truncate text-[12.5px] font-medium">{a.name}</p>
        <p className="truncate text-[10.5px] text-white/45">
          {disabled ? (a.installed ? '暂不支持 GUI 对话' : '未安装') : a.version ? `v${a.version} · GUI 流式` : 'GUI 流式'}
        </p>
      </div>
    </button>
  );
}

export default function NewSessionSheet({
  open,
  onClose,
  onCreated,
}: {
  open: boolean;
  onClose: () => void;
  onCreated: (info: LiveSessionInfo) => void;
}) {
  const agents = useAgents();
  const projects = useProjects();
  const [agentId, setAgentId] = useState('');
  const [projectDir, setProjectDir] = useState('');
  const [prompt, setPrompt] = useState('');
  const [model, setModel] = useState('');
  const [effort, setEffort] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  const agent = useMemo(() => (agents.query.data ?? []).find((a) => a.id === agentId) ?? null, [agents.query.data, agentId]);

  useEffect(() => {
    if (agent) {
      setModel(agent.launchOptions?.model?.default ?? '');
      setEffort(agent.launchOptions?.effort?.default ?? '');
    }
  }, [agent]);

  if (!open) return null;

  const selectable = (agents.query.data ?? []).filter((a) => a.installed && a.streaming);

  const create = async () => {
    if (!agent || !projectDir.trim() || busy) return;
    setBusy(true);
    setError('');
    try {
      const info = await createStreamSession({
        agentId: agent.id,
        projectDir: projectDir.trim(),
        firstPrompt: prompt.trim() || null,
        model: model || null,
        effort: effort || null,
        resumeKey: null,
      });
      setPrompt('');
      onCreated(info);
      onClose();
    } catch (e) {
      setError(String(e).replace(/^Error:\s*/, ''));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/45 p-6 backdrop-blur-sm" onClick={onClose}>
      <div
        className="w-[520px] rounded-2xl border border-white/12 bg-[#221e28] p-5 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between pb-3">
          <h2 className="text-[15px] font-semibold">新建代理会话</h2>
          <button onClick={onClose} className="grid size-7 place-items-center rounded-lg text-white/50 hover:bg-white/10 hover:text-white">
            <X size={15} />
          </button>
        </div>

        <p className="pb-1.5 text-[11px] font-medium uppercase tracking-wide text-white/40">代理（支持 GUI 流式）</p>
        <div className="grid grid-cols-2 gap-2">
          {selectable.map((a) => (
            <AgentCard key={a.id} a={a} selected={a.id === agentId} onSelect={() => setAgentId(a.id)} />
          ))}
          {agents.query.isLoading && <p className="py-2 text-[12px] text-white/40">扫描已安装代理…</p>}
        </div>

        <p className="pb-1.5 pt-3.5 text-[11px] font-medium uppercase tracking-wide text-white/40">项目目录</p>
        <div className="flex items-center gap-2 rounded-xl bg-black/25 px-3 ring-1 ring-white/10 focus-within:ring-white/25">
          <FolderOpen size={14} className="shrink-0 text-white/40" />
          <input
            value={projectDir}
            onChange={(e) => setProjectDir(e.target.value)}
            list="bench-projects"
            placeholder="D:\path\to\project"
            className="w-full bg-transparent py-2 text-[12.5px] text-white outline-none placeholder:text-white/35"
          />
          <datalist id="bench-projects">
            {(projects.query.data ?? []).map((p) => (
              <option key={p} value={p} />
            ))}
          </datalist>
        </div>

        {agent?.launchOptions?.model && (
          <div className="pt-3.5">
            <p className="pb-1.5 text-[11px] font-medium uppercase tracking-wide text-white/40">模型 / 推理强度</p>
            <div className="flex gap-2">
              <select
                value={model}
                onChange={(e) => setModel(e.target.value)}
                className="flex-1 rounded-xl bg-black/25 px-3 py-2 text-[12.5px] text-white outline-none ring-1 ring-white/10"
              >
                {agent.launchOptions.model.choices.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
              {agent.launchOptions.effort && (
                <select
                  value={effort}
                  onChange={(e) => setEffort(e.target.value)}
                  className="flex-1 rounded-xl bg-black/25 px-3 py-2 text-[12.5px] text-white outline-none ring-1 ring-white/10"
                >
                  {agent.launchOptions.effort.choices.map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
                </select>
              )}
            </div>
          </div>
        )}

        <p className="pb-1.5 pt-3.5 text-[11px] font-medium uppercase tracking-wide text-white/40">首条消息（可选）</p>
        <textarea
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          rows={2}
          placeholder="创建后立即发送…"
          className="w-full resize-none rounded-xl bg-black/25 px-3 py-2 text-[12.5px] text-white outline-none ring-1 ring-white/10 placeholder:text-white/35 focus-within:ring-white/25"
        />

        {error && <p className="pt-2 text-[12px] text-red-300">{error}</p>}

        <div className="flex justify-end gap-2 pt-4">
          <button onClick={onClose} className="rounded-xl px-4 py-2 text-[12.5px] text-white/60 hover:bg-white/8 hover:text-white">
            取消
          </button>
          <button
            onClick={create}
            disabled={!agent || !projectDir.trim() || busy}
            className="flex items-center gap-1.5 rounded-xl bg-[var(--accent)] px-4 py-2 text-[12.5px] font-medium text-white transition-all hover:brightness-110 disabled:opacity-40"
          >
            {busy && <Loader2 size={13} className="animate-spin" />}
            创建会话
          </button>
        </div>
      </div>
    </div>
  );
}
