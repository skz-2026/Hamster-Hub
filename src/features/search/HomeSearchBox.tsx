/**
 * 桌面主页内联搜索框：输入即在本页下方出结果下拉，不跳转 /search。
 * 检索与搜索页/Spotlight 共用 useUnifiedSearch（应用拼音/文件 FTS5），结果四路
 * 同序：应用 → 文件 → 网页 → 问 AI；键盘可达（↑↓ 选择、Enter 执行、Esc 清空）。
 * 视觉沿用桌面主页玻璃语言（深色胶囊 + 毛玻璃下拉）。
 */
import { useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Globe, Search, Sparkles, CornerDownLeft } from 'lucide-react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { commands, type AppEntry } from '@/shared/lib/ipc';
import { useUnifiedSearch } from './useUnifiedSearch';
import { Monogram } from '@/features/home/AppIcon';
import { FileKindIcon } from '@/features/spotlight/FileKindIcon';
import { useI18n } from '@/shared/i18n/provider';

const SEARCH_ENGINE = 'https://www.baidu.com/s?wd=';

interface ResultItem {
  key: string;
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

export function HomeSearchBox({
  className = '',
  style,
}: {
  className?: string;
  style?: React.CSSProperties;
}) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [query, setQuery] = useState('');
  const [sel, setSel] = useState(0);
  const [open, setOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const q = query.trim();
  const { apps, files } = useUnifiedSearch(q, 8, 8);

  const results = useMemo<ResultItem[]>(() => {
    if (!q) return [];
    return [
      ...apps.map<ResultItem>((a) => ({
        key: `app:${a.app_key}`,
        title: a.display_name,
        subtitle: t('agent.appEnterHint'),
        icon: appIcon(a),
        run: () => void commands.appLaunch(a.app_key).catch(console.error),
      })),
      ...files.map<ResultItem>((f) => ({
        key: `file:${f.path}`,
        title: f.name,
        subtitle: `${f.kind || t('agent.fileFallback')} · ${f.path}`,
        icon: <FileKindIcon kind={f.kind} />,
        run: () => void commands.openPath(f.path).catch(console.error),
      })),
      {
        key: 'web',
        title: t('agent.webSearch', { q }),
        subtitle: t('agent.webBaidu'),
        icon: (
          <span className="squircle grid size-8 shrink-0 place-items-center bg-[#2f6be0] text-white">
            <Globe size={16} />
          </span>
        ),
        run: () => void commands.openUrl(SEARCH_ENGINE + encodeURIComponent(q)).catch(console.error),
      },
      {
        key: 'ai',
        title: t('agent.askAi', { q }),
        subtitle: t('agent.aiAssistant'),
        icon: (
          <span className="squircle grid size-8 shrink-0 place-items-center bg-[var(--accent)] text-white">
            <Sparkles size={15} />
          </span>
        ),
        // 与 /search 页同路径：暂存问题 → 助手页自动发送
        run: () => {
          commands
            .kvSet('agent.pendingQ', q)
            .catch(() => {})
            .finally(() => navigate('/agent'));
        },
      },
    ];
  }, [apps, files, q, navigate, t]);

  useEffect(() => setSel(0), [q]);
  useEffect(() => {
    if (open) listRef.current?.children[sel]?.scrollIntoView({ block: 'nearest' });
  }, [sel, open]);

  const run = (r: ResultItem) => {
    r.run();
    setQuery('');
    setOpen(false);
    inputRef.current?.blur();
  };

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSel((s) => Math.min(s + 1, results.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSel((s) => Math.max(s - 1, 0));
    } else if (e.key === 'Enter') {
      const r = results[sel];
      if (r) run(r);
    } else if (e.key === 'Escape') {
      setQuery('');
      setOpen(false);
    }
  };

  return (
    <div
      ref={wrapRef}
      // z-30：本组件带 rise-in 动画（fill both）自成 stacking context，
      // 下拉要压过 DOM 里更靠后的快捷入口/卡片网格，必须整块提到兄弟之上
      className={`relative z-30 ${className}`}
      style={style}
      onBlur={(e) => {
        // 焦点离开整块（点击别处）才收起；点结果行不收（见 onMouseDown）
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setOpen(false);
      }}
    >
      <div
        className={`group flex h-[52px] items-center gap-3 rounded-full bg-black/30 px-6 ring-1 ring-white/15 backdrop-blur-2xl transition-all hover:bg-black/38 focus-within:bg-black/38 focus-within:ring-white/25 shadow-[0_8px_32px_rgba(0,0,0,.35),inset_0_1px_0_rgba(255,255,255,.08)]`}
      >
        <Search size={19} className="shrink-0 text-white/60 transition-colors group-focus-within:text-white/85" />
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setOpen(true);
          }}
          onFocus={() => setOpen(true)}
          onKeyDown={onKey}
          placeholder={t('chrome.desktop.searchPlaceholder')}
          className="h-full w-full bg-transparent text-[15px] text-white outline-none placeholder:text-white/55"
        />
      </div>

      {/* 结果下拉：绝对定位于胶囊下方，毛玻璃与桌面主页同一视觉语言 */}
      {open && q && (
        <div
          ref={listRef}
          className="absolute inset-x-0 top-[calc(100%+6px)] z-30 max-h-[46vh] overflow-y-auto rounded-3xl bg-neutral-900/85 p-1.5 ring-1 ring-white/12 backdrop-blur-2xl"
        >
          {results.length === 0 ? (
            <p className="px-4 py-6 text-center text-[13px] text-white/60">{t('agent.noResults', { q })}</p>
          ) : (
            results.map((r, i) => (
              <button
                key={r.key}
                // mousedown 不夺焦：防止 input 先 blur 卸载列表导致 click 丢失
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => run(r)}
                onMouseEnter={() => setSel(i)}
                className={`ios-ease flex w-full items-center gap-3 rounded-xl px-3 py-2 text-left ${
                  i === sel ? 'bg-white/16' : ''
                }`}
              >
                {r.icon}
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-[14px] text-white">{r.title}</span>
                  <span className="block truncate text-[11px] text-white/55">{r.subtitle}</span>
                </span>
                {i === sel && <CornerDownLeft size={14} className="shrink-0 text-white/50" />}
              </button>
            ))
          )}
        </div>
      )}
    </div>
  );
}
