/**
 * 文件页（M3 收尾：files 页补齐）：文件索引的常驻浏览页。
 * 默认「最近文件」（mtime 降序），输入即 FTS5 检索（拼音双形态/子串）；
 * 类型药丸筛选；行级动作 = 打开 + 资源管理器定位；右上角手动重建索引。
 */
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { FolderOpen, FolderSearch, Loader2, RefreshCw, Search } from 'lucide-react';
import { commands, type FileHit } from '@/shared/lib/ipc';
import { FileKindIcon } from '@/features/spotlight/FileKindIcon';

const fmtSize = (n: number) =>
  n >= 1073741824
    ? `${(n / 1073741824).toFixed(1)} GB`
    : n >= 1048576
      ? `${(n / 1048576).toFixed(1)} MB`
      : n >= 1024
        ? `${(n / 1024).toFixed(0)} KB`
        : `${n} B`;

const fmtDate = (sec: number) =>
  sec > 0
    ? new Date(sec * 1000).toLocaleString('zh-CN', {
        month: 'numeric',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
      })
    : '';

/** 类型药丸从当前数据派生（kind 为空的归「文件」，不凭空造空分类） */
const kindOf = (f: FileHit) => f.kind || '文件';

export default function FilesPage() {
  const qc = useQueryClient();
  const [query, setQuery] = useState('');
  const [kind, setKind] = useState<string>('全部');
  const q = query.trim();

  const recent = useQuery({
    queryKey: ['files', 'recent'],
    queryFn: () => commands.recentFiles(50),
  });
  const search = useQuery({
    queryKey: ['files', 'search', q, 50],
    queryFn: () => commands.fileSearch(q, 50),
    enabled: q.length > 0,
  });
  const refresh = useMutation({
    mutationFn: () => commands.fileIndexRefresh(),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['files'] }),
  });

  const loading = q.length > 0 ? search.isLoading : recent.isLoading;
  const rows = useMemo(() => (q.length > 0 ? (search.data ?? []) : (recent.data ?? [])), [q, search.data, recent.data]);
  const kinds = useMemo(() => [...new Set(rows.map(kindOf))], [rows]);
  const shown = kind === '全部' ? rows : rows.filter((f) => kindOf(f) === kind);

  const open = (f: FileHit) => void commands.openPath(f.path).catch(console.error);
  const reveal = (e: React.MouseEvent, f: FileHit) => {
    e.stopPropagation();
    void commands.revealInExplorer(f.path).catch(console.error);
  };

  return (
    <div className="mx-auto max-w-4xl">
      <div className="mb-5 flex items-center justify-between">
        <h1 className="flex items-center gap-2 text-xl font-semibold">
          <FolderOpen size={20} className="text-[var(--accent)]" />
          文件
        </h1>
        <button
          onClick={() => refresh.mutate()}
          disabled={refresh.isPending}
          className="flex items-center gap-1.5 rounded-full bg-[var(--panel)] px-3 py-1.5 text-[12px] text-[var(--text-muted)] ring-1 ring-[var(--border)] transition-colors hover:text-[var(--text)] disabled:opacity-60"
          title="全量重扫文件索引（watcher 平时自动增量维护）"
        >
          {refresh.isPending ? <Loader2 size={12} className="animate-spin" /> : <RefreshCw size={12} />}
          重建索引
        </button>
      </div>

      <div className="flex h-10 items-center gap-3 rounded-xl bg-[var(--panel)] px-3.5 ring-1 ring-[var(--border)] transition-colors focus-within:ring-[var(--accent)]">
        <Search size={15} className="shrink-0 text-[var(--text-muted)]" />
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="搜索文件（拼音 / 首字母 / 文件名）…"
          className="h-full w-full bg-transparent text-[13.5px] text-[var(--text)] outline-none placeholder:text-[var(--text-muted)]"
        />
      </div>

      {kinds.length > 1 && (
        <div className="mt-4 flex flex-wrap items-center gap-2">
          {(['全部', ...kinds] as const).map((k) => (
            <button
              key={k}
              onClick={() => setKind(k)}
              className={`rounded-full px-3 py-1 text-[12px] transition-colors ${
                kind === k
                  ? 'bg-[var(--accent)] font-medium text-white'
                  : 'bg-[var(--panel)] text-[var(--text-muted)] ring-1 ring-[var(--border)] hover:text-[var(--text)]'
              }`}
            >
              {k}
            </button>
          ))}
        </div>
      )}

      <div className="mt-4 pb-6">
        {loading ? (
          <div className="grid place-items-center py-20">
            <Loader2 size={20} className="animate-spin text-[var(--text-muted)]" />
          </div>
        ) : shown.length === 0 ? (
          <p className="py-20 text-center text-[13px] text-[var(--text-muted)]">
            {q.length > 0 ? (
              <>没有匹配「{q}」的文件</>
            ) : (
              <>索引还没有内容：点右上角「重建索引」扫描一遍（索引范围见设置 file_index.roots）</>
            )}
          </p>
        ) : (
          <div className="space-y-0.5">
            {shown.map((f) => (
              <div
                key={f.path}
                onClick={() => open(f)}
                className="flex cursor-pointer items-center gap-3 rounded-xl px-3 py-2 transition-colors hover:bg-[var(--hover)]"
                title={f.path}
              >
                <FileKindIcon kind={f.kind} />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[13.5px] text-[var(--text)]">{f.name}</div>
                  <div className="truncate text-[11px] text-[var(--text-muted)]">{f.path}</div>
                </div>
                <div className="shrink-0 text-right text-[11px] leading-tight text-[var(--text-muted)] tabular-nums">
                  <div>{fmtSize(f.size)}</div>
                  <div className="opacity-70">{fmtDate(f.mtime)}</div>
                </div>
                <button
                  onClick={(e) => reveal(e, f)}
                  title="在资源管理器中显示"
                  className="grid size-8 shrink-0 place-items-center rounded-lg text-[var(--text-muted)] transition-colors hover:bg-[var(--panel-strong)] hover:text-[var(--text)]"
                >
                  <FolderSearch size={15} />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
