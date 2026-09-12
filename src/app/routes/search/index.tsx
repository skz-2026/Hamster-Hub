/**
 * 搜索页（M3 收尾：search 页补齐）：Spotlight 的常驻页面形态。
 * 应用（拼音/首字母）+ 文件（FTS5）+ 网页 + 问 AI 四路结果聚合，键盘可达
 * （↑↓ 选择、Enter 执行）；检索与 Spotlight 共用 useUnifiedSearch，语义一致。
 */
import { useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Globe, Search, Sparkles, CornerDownLeft } from 'lucide-react';
import { commands, type AppEntry, type FileHit } from '@/shared/lib/ipc';
import { useUnifiedSearch } from '@/features/search/useUnifiedSearch';
import { Monogram } from '@/features/home/AppIcon';
import { FileKindIcon } from '@/features/spotlight/FileKindIcon';
import { convertFileSrc } from '@tauri-apps/api/core';

const SEARCH_ENGINE = 'https://www.baidu.com/s?wd=';

interface ResultItem {
  key: string;
  group: '应用' | '文件' | '网页' | 'AI';
  title: string;
  subtitle: string;
  icon: React.ReactNode;
  run: () => void;
}

function appIcon(a: AppEntry) {
  return a.icon_path ? (
    <img
      src={a.icon_path.startsWith('data:') ? a.icon_path : convertFileSrc(a.icon_path)}
      alt=""
      className="squircle size-8 shrink-0 object-cover"
    />
  ) : (
    <Monogram name={a.display_name} size={32} />
  );
}

/** 应用/文件/网页/AI 四路结果聚合（与 Spotlight 同序：应用 → 文件 → 网页 → AI） */
function useResults(q: string): ResultItem[] {
  const navigate = useNavigate();
  const { apps, files } = useUnifiedSearch(q, 12, 20);
  return useMemo<ResultItem[]>(() => {
    if (!q) return [];
    const ask = async () => {
      try {
        await commands.kvSet('agent.pendingQ', q);
      } catch {
        /* 暂存失败仍跳转，助手页按普通问题处理 */
      }
      navigate('/agent');
    };
    return [
      ...apps.map<ResultItem>((a) => ({
        key: `app:${a.app_key}`,
        group: '应用',
        title: a.display_name,
        subtitle: '应用 · Enter 启动',
        icon: appIcon(a),
        run: () => void commands.appLaunch(a.app_key).catch(console.error),
      })),
      ...files.map<ResultItem>((f) => ({
        key: `file:${f.path}`,
        group: '文件',
        title: f.name,
        subtitle: `${f.kind || '文件'} · ${f.path}`,
        icon: <FileKindIcon kind={f.kind} />,
        run: () => void commands.openPath(f.path).catch(console.error),
      })),
      {
        key: 'web',
        group: '网页',
        title: `搜索「${q}」`,
        subtitle: '网页 · 百度',
        icon: (
          <span className="squircle grid size-8 shrink-0 place-items-center bg-[#2f6be0] text-white">
            <Globe size={16} />
          </span>
        ),
        run: () => void commands.openUrl(SEARCH_ENGINE + encodeURIComponent(q)).catch(console.error),
      },
      {
        key: 'ai',
        group: 'AI',
        title: `问 AI：「${q}」`,
        subtitle: 'AI 助手',
        icon: (
          <span className="squircle grid size-8 shrink-0 place-items-center bg-[var(--accent)] text-white">
            <Sparkles size={15} />
          </span>
        ),
        run: () => void ask(),
      },
    ];
  }, [apps, files, q, navigate]);
}

export default function SearchPage() {
  const [query, setQuery] = useState('');
  const [sel, setSel] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const q = query.trim();
  const results = useResults(q);

  useEffect(() => inputRef.current?.focus(), []);
  useEffect(() => setSel(0), [q]);
  useEffect(() => {
    listRef.current?.children[sel]?.scrollIntoView({ block: 'nearest' });
  }, [sel]);

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSel((s) => Math.min(s + 1, results.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSel((s) => Math.max(s - 1, 0));
    } else if (e.key === 'Enter') {
      results[sel]?.run();
    }
    // Esc 不在此劫持：桌面接管子页由 AppShell 统一处理返回桌面
  };

  let lastGroup = '';

  return (
    <div className="mx-auto max-w-3xl">
      <div className="mb-5 flex items-center justify-between">
        <h1 className="flex items-center gap-2 text-xl font-semibold">
          <Search size={20} className="text-[var(--accent)]" />
          搜索
        </h1>
        <span className="text-xs text-[var(--text-muted)]">拼音 / 首字母缩写 / 中文子串均可命中</span>
      </div>

      <div className="flex h-12 items-center gap-3 rounded-2xl bg-[var(--panel)] px-4 ring-1 ring-[var(--border)] transition-colors focus-within:ring-[var(--accent)]">
        <Search size={17} className="shrink-0 text-[var(--text-muted)]" />
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={onKey}
          placeholder="搜索应用、文件、网页…"
          className="h-full w-full bg-transparent text-[14.5px] text-[var(--text)] outline-none placeholder:text-[var(--text-muted)]"
        />
      </div>

      {!q ? (
        <div className="mt-6 rounded-2xl bg-[var(--panel)] px-5 py-8 text-center ring-1 ring-[var(--border)]">
          <p className="text-sm text-[var(--text-muted)]">
            输入即搜：应用（如「微信」「wx」「weixin」）、本地文件、网页与问 AI。
          </p>
          <p className="mt-2 text-xs text-[var(--text-muted)] opacity-70">
            全局随时可用 Alt+Space 召出 Spotlight，与本页同一检索。
          </p>
        </div>
      ) : (
        <div ref={listRef} className="mt-4 space-y-1 pb-6">
          {results.length === 0 && (
            <p className="py-16 text-center text-[13px] text-[var(--text-muted)]">
              没有匹配「{q}」的结果
            </p>
          )}
          {results.map((r, i) => {
            const header = r.group !== lastGroup ? r.group : null;
            lastGroup = r.group;
            return (
              <div key={r.key}>
                {header && (
                  <div className="mb-1 mt-4 px-1 text-[11px] font-medium tracking-wide text-[var(--text-muted)]">
                    {header}
                  </div>
                )}
                <button
                  onClick={r.run}
                  onMouseEnter={() => setSel(i)}
                  className={`flex w-full items-center gap-3 rounded-xl px-3 py-2 text-left transition-colors ${
                    i === sel ? 'bg-[var(--hover)]' : ''
                  }`}
                >
                  {r.icon}
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-[13.5px] text-[var(--text)]">{r.title}</span>
                    <span className="block truncate text-[11px] text-[var(--text-muted)]">{r.subtitle}</span>
                  </span>
                  {i === sel && <CornerDownLeft size={14} className="shrink-0 text-[var(--text-muted)]" />}
                </button>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
