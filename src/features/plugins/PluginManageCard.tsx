/**
 * PluginManageCard：设置页「插件」卡片（生命周期管理）。
 * 停用 = KV `plugins.disabled`（选择器隐藏、已添加槽位显示占位）；
 * 删除 = 整目录移除（两步确认，防误删）。
 */
import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Puzzle } from 'lucide-react';
import { commands } from '@/shared/lib/ipc';
import { useDisabledPlugins, usePluginList } from './registry';

export default function PluginManageCard() {
  const qc = useQueryClient();
  const list = usePluginList();
  const disabled = useDisabledPlugins();
  const [confirmId, setConfirmId] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);

  const plugins = list.data ?? [];

  const remove = async (id: string) => {
    setBusyId(id);
    try {
      await commands.pluginDelete(id);
      await qc.invalidateQueries({ queryKey: ['plugins'] });
    } catch (e) {
      console.error('[plugins] 删除失败', e);
    } finally {
      setBusyId(null);
      setConfirmId(null);
    }
  };

  return (
    <section className="card px-4 py-3.5">
      <div className="text-sm font-medium">插件</div>
      <div className="mt-0.5 text-xs leading-relaxed text-[var(--text-muted)]">
        主屏小组件插件：目录在
        <span className="mx-1 font-mono text-[11px]">应用数据/plugins/</span>
        ，放入文件夹即安装。停用后从选择器隐藏、已添加槽位显示占位。
      </div>

      <div className="mt-3 space-y-1">
        {plugins.length === 0 && (
          <p className="text-xs text-[var(--text-muted)]">尚未安装插件。</p>
        )}
        {plugins.map((p) => {
          const isDisabled = disabled.data.includes(p.id);
          return (
            <div
              key={p.id}
              className="flex items-center justify-between gap-3 rounded-lg px-2 py-1.5 hover:bg-white/[0.04]"
            >
              <div className="min-w-0">
                <div className="flex items-center gap-1.5 text-[12.5px] text-[var(--text)]">
                  <Puzzle size={12} className="shrink-0 text-[var(--text-muted)]" />
                  <span className="truncate">{p.name}</span>
                  <span className="shrink-0 text-[10px] text-[var(--text-muted)]">v{p.version}</span>
                  {p.permissions.length > 0 && (
                    <span
                      title={`声明权限：${p.permissions.join('、')}`}
                      className="shrink-0 rounded-full bg-amber-400/15 px-1.5 text-[9.5px] text-amber-300"
                    >
                      {p.permissions.length} 项权限
                    </span>
                  )}
                </div>
                <div className="truncate text-[10.5px] text-[var(--text-muted)]">
                  {p.description}
                </div>
              </div>
              <div className="flex shrink-0 items-center gap-2">
                {confirmId === p.id ? (
                  <>
                    <button
                      onClick={() => remove(p.id)}
                      disabled={busyId === p.id}
                      className="rounded-full bg-red-500/25 px-2.5 py-1 text-[11px] font-medium text-red-200 transition-colors hover:bg-red-500/40"
                    >
                      确认删除
                    </button>
                    <button
                      onClick={() => setConfirmId(null)}
                      className="rounded-full bg-white/8 px-2.5 py-1 text-[11px] text-white/70 transition-colors hover:bg-white/16"
                    >
                      取消
                    </button>
                  </>
                ) : (
                  <>
                    <button
                      role="switch"
                      aria-checked={!isDisabled}
                      aria-label={`${isDisabled ? '启用' : '停用'} ${p.name}`}
                      onClick={() => disabled.setDisabled(p.id, !isDisabled)}
                      className={`relative h-5 w-9 shrink-0 rounded-full transition-colors ${
                        isDisabled
                          ? 'border border-[var(--border)] bg-[var(--panel-strong)]'
                          : 'bg-[var(--accent)]'
                      }`}
                      title={isDisabled ? '已停用（点击启用）' : '已启用（点击停用）'}
                    >
                      <span
                        className={`absolute top-0.5 size-4 rounded-full bg-white shadow transition-all ${
                          isDisabled ? 'left-0.5' : 'left-[18px]'
                        }`}
                      />
                    </button>
                    <button
                      onClick={() => setConfirmId(p.id)}
                      className="rounded-lg px-2 py-1 text-[11px] text-[var(--text-muted)] transition-colors hover:text-red-300"
                      title={`删除 ${p.name}（整目录移除）`}
                    >
                      删除
                    </button>
                  </>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
