/**
 * McpSyncCard：设置页「桌面 MCP」卡片（M4 分发模型的核心 UI）。
 * - 接入信息：URL（固定端口）+ 用户级长效令牌（可再生成，旧令牌即刻失效）；
 * - 按 agent 开关：开启 = 把 hamster-desktop 的标准 HTTP MCP 配置写入该 agent
 *   的配置文件（hamster-core 同步引擎，备份 + 历史，可回滚）；关闭 = 从配置摘除。
 * 任何支持 MCP 的宿主（Claude Desktop / Cursor / 各 agent CLI）都能接这个端点。
 */
import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Copy, Eye, EyeOff, Loader2, RefreshCw } from 'lucide-react';
import { commands } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';

export default function McpSyncCard() {
  const qc = useQueryClient();
  const { t } = useI18n();
  const access = useQuery({ queryKey: ['agent-mcp-access'], queryFn: () => commands.agentMcpAccessInfo() });
  const status = useQuery({ queryKey: ['agent-mcp-status'], queryFn: () => commands.agentMcpStatus() });
  const [revealed, setRevealed] = useState(false);
  const [busyId, setBusyId] = useState<string | null>(null);

  const toggle = async (agentId: string, enabled: boolean) => {
    setBusyId(agentId);
    try {
      await commands.agentMcpSetEnabled(agentId, enabled);
      await qc.invalidateQueries({ queryKey: ['agent-mcp-status'] });
    } catch (e) {
      console.error('[mcp-sync] 分发失败', e);
    } finally {
      setBusyId(null);
    }
  };

  const regenerate = async () => {
    const s = await commands.settingsLoad();
    s.agent.mcp_user_token = crypto.randomUUID();
    await commands.settingsSave(s);
    await qc.invalidateQueries({ queryKey: ['agent-mcp-access'] });
  };

  const agents = (status.data ?? []).filter((s) => s.installed && s.mcpCapable);
  const copy = (text: string) => navigator.clipboard.writeText(text).catch(() => {});

  return (
    <section className="card px-4 py-3.5">
      <div className="text-sm font-medium">{t('agent.mcpCardTitle')}</div>
      <div className="mt-0.5 text-xs leading-relaxed text-[var(--text-muted)]">
        {t('agent.mcpCardDesc')}
      </div>

      {/* 接入信息 */}
      <div className="mt-3 space-y-1.5 rounded-xl bg-[var(--panel-strong)] p-3 font-mono text-[11px] leading-relaxed">
        <div className="flex items-center justify-between gap-2">
          <span className="shrink-0 text-[var(--text-muted)]">URL</span>
          <button
            onClick={() => copy(access.data?.url ?? '')}
            title={t('agent.copyUrl')}
            className="truncate text-left text-[var(--text)] underline-offset-2 hover:underline"
          >
            {access.data?.url ?? '…'}
          </button>
        </div>
        <div className="flex items-center justify-between gap-2">
          <span className="shrink-0 text-[var(--text-muted)]">Token</span>
          <span className="flex min-w-0 items-center gap-1.5">
            <span className="truncate">
              {revealed ? access.data?.token : '••••••••-••••-••••'}
            </span>
            <button onClick={() => setRevealed((v) => !v)} className="shrink-0 text-[var(--text-muted)] hover:text-[var(--text)]" aria-label={t('agent.toggleToken')}>
              {revealed ? <EyeOff size={12} /> : <Eye size={12} />}
            </button>
            <button onClick={() => copy(access.data?.token ?? '')} className="shrink-0 text-[var(--text-muted)] hover:text-[var(--text)]" aria-label={t('agent.copyToken')}>
              <Copy size={12} />
            </button>
            <button
              onClick={regenerate}
              title={t('agent.regenerateToken')}
              className="shrink-0 text-[var(--text-muted)] hover:text-[var(--accent)]"
            >
              <RefreshCw size={12} />
            </button>
          </span>
        </div>
      </div>
      {access.data?.portFellBack && (
        <p className="mt-2 text-[11px] leading-relaxed text-amber-500">
          {t('agent.portFallback', { port: access.data.defaultPort })}
        </p>
      )}

      {/* 按 agent 开关 */}
      <div className="mt-3 space-y-1">
        {agents.length === 0 && (
          <p className="text-xs text-[var(--text-muted)]">
            {t('agent.noAgents')}
          </p>
        )}
        {agents.map((a) => (
          <div key={a.agentId} className="flex items-center justify-between rounded-lg px-2 py-1.5 hover:bg-[var(--hover)]">
            <div className="min-w-0">
              <span className="text-[12.5px] text-[var(--text)]">{a.agentName}</span>
              <span className="ml-2 truncate text-[10.5px] text-[var(--text-muted)]">{a.configPath}</span>
            </div>
            {busyId === a.agentId ? (
              <Loader2 size={14} className="shrink-0 animate-spin text-[var(--text-muted)]" />
            ) : (
              <MiniSwitch checked={a.enabled} onChange={(v) => toggle(a.agentId, v)} label={t('agent.distributeTo', { name: a.agentName })} />
            )}
          </div>
        ))}
      </div>
    </section>
  );
}

function MiniSwitch({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
  label: string;
}) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      aria-label={label}
      title={label}
      onClick={() => onChange(!checked)}
      className={`relative h-5 w-9 shrink-0 rounded-full transition-colors ${
        checked ? 'bg-[var(--accent)]' : 'bg-[var(--panel-strong)] border border-[var(--border)]'
      }`}
    >
      <span
        className={`absolute top-0.5 size-4 rounded-full bg-white shadow transition-all ${
          checked ? 'left-[18px]' : 'left-0.5'
        }`}
      />
    </button>
  );
}
