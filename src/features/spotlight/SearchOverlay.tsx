/**
 * Spotlight 搜索覆盖层（/spotlight 独立窗口路由）：
 * 全屏毛玻璃 + 中央搜索框，应用/网页两路结果，键盘优先导航。
 * 文件检索路在 M2 fileindex 接入。
 */
import { useEffect, useMemo, useRef, useState } from 'react';
import {
  Search,
  Globe,
  Sparkles,
  CornerDownLeft,
  FileText,
  Film,
  Music,
  Image as ImageIcon,
  Archive,
  Code2,
  File as FileIcon,
  ListTodo,
} from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { commands, isTauri, type FileHit } from '@/shared/lib/ipc';
import { Monogram } from '@/features/home/AppIcon';
import { FileKindIcon } from './FileKindIcon';
import { useUnifiedSearch } from '@/features/search/useUnifiedSearch';
import { parseQuickAdd } from '@/features/todo/nlp';
import { dueLabel } from '@/features/todo/due';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useI18n } from '@/shared/i18n/provider';

const SEARCH_ENGINE = 'https://www.baidu.com/s?wd=';

interface ResultItem {
  kind: 'app' | 'file' | 'web' | 'ai' | 'todo';
  key: string;
  title: string;
  subtitle: string;
  iconPath?: string | null;
  fileHit?: FileHit;
  run: () => void;
}

export default function SearchOverlay() {
  const { t } = useI18n();
  const [query, setQuery] = useState('');
  const [sel, setSel] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const q = query.trim().toLowerCase();

  // 应用（拼音/首字母/名称检索）与文件（FTS5）两路真实查询（与 /search 页共用 hook）
  const { apps: searchedApps, files: fileHits } = useUnifiedSearch(q, 8, 6);

  const close = () => {
    if (isTauri) {
      getCurrentWindow().hide();
    } else {
      window.location.hash = '#/';
    }
  };

  const results = useMemo<ResultItem[]>(() => {
    const appItems: ResultItem[] = searchedApps.map((a) => ({
        kind: 'app' as const,
        key: a.app_key,
        title: a.display_name,
        subtitle: t('agent.groupApps'),
        iconPath: a.icon_path,
        run: () => {
          commands.appLaunch(a.app_key).catch(console.error);
          close();
        },
      }));
    const webItems: ResultItem[] = q
      ? [
          {
            kind: 'web' as const,
            key: `web:${q}`,
            title: t('agent.webSearch', { q: query.trim() }),
            subtitle: t('agent.webBaidu'),
            iconPath: null,
            run: () => {
              commands.openUrl(SEARCH_ENGINE + encodeURIComponent(query.trim())).catch(console.error);
              close();
            },
          },
        ]
      : [];
    const fileItems: ResultItem[] = fileHits.map((f) => ({
      kind: 'file' as const,
      key: f.path,
      title: f.name,
      subtitle: `${f.kind || t('agent.fileFallback')} · ${f.path}`,
      fileHit: f,
      run: () => {
        commands.openPath(f.path).catch(console.error);
        close();
      },
    }));
    // 快速捕获：输入即解析截止时间，回车记待办（"明天下午3点交房租"）
    const parsed = parseQuickAdd(query.trim());
    const todoItems: ResultItem[] = q
      ? [
          {
            kind: 'todo' as const,
            key: `todo:${q}`,
            title: t('agent.todoQuickAdd', { q: parsed.content || query.trim() }),
            subtitle: parsed.dueAt != null ? dueLabel(parsed.dueAt) : t('agent.todoNoDue'),
            iconPath: null,
            run: () => {
              commands
                .todoCreate(parsed.content || query.trim(), parsed.dueAt, parsed.dueAt != null ? true : null)
                .then(() => close())
                .catch(console.error);
            },
          },
        ]
      : [];
    // 尾部「问 AI」入口：暂存问题 → 跳主窗口 AI 助手页自动发送
    const aiItem: ResultItem[] = q
      ? [
          {
            kind: 'ai' as const,
            key: `ai:${q}`,
            title: t('agent.askAi', { q: query.trim() }),
            subtitle: t('agent.aiAssistant'),
            iconPath: null,
            run: async () => {
              const question = query.trim();
              close();
              try {
                await commands.kvSet('agent.pendingQ', question);
              } catch {
                /* 暂存失败仍尝试跳转 */
              }
              if (isTauri) {
                const { emitTo } = await import('@tauri-apps/api/event');
                await emitTo('main', 'hamster:navigate', { to: '/agent' });
              } else {
                window.location.hash = '#/agent';
              }
            },
          },
        ]
      : [];
    return [...appItems, ...fileItems, ...todoItems, ...webItems, ...aiItem];
  }, [searchedApps, q, query, fileHits, t]);

  // 失焦自动隐藏（桌面壳窗口语义；浏览器预览不隐藏）
  useEffect(() => {
    if (!isTauri) return;
    const w = getCurrentWindow();
    const onBlur = () => void w.hide();
    window.addEventListener('blur', onBlur);
    return () => window.removeEventListener('blur', onBlur);
  }, []);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    setSel(0);
  }, [q]);

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSel((s) => Math.min(s + 1, results.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSel((s) => Math.max(s - 1, 0));
    } else if (e.key === 'Enter') {
      results[sel]?.run();
    } else if (e.key === 'Escape') {
      close();
    }
  };

  // 选中项滚动可见
  useEffect(() => {
    listRef.current?.children[sel]?.scrollIntoView({ block: 'nearest' });
  }, [sel]);

  return (
    <div
      className="grid h-screen w-full place-items-start justify-items-center bg-black/35 pt-[16vh] backdrop-blur-2xl"
      onClick={close}
    >
      <div className="w-[min(620px,86vw)]" onClick={(e) => e.stopPropagation()}>
        <div className="ios-ease flex h-12 items-center gap-3 rounded-2xl bg-white/16 px-4 ring-1 ring-white/18 backdrop-blur-xl focus-within:ring-white/35">
          <Search size={18} className="shrink-0 text-white/70" />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onKey}
            placeholder={t('agent.searchPlaceholder')}
            className="h-full w-full bg-transparent text-[16px] text-white placeholder:text-white/50 outline-none"
          />
          <kbd className="rounded bg-white/12 px-1.5 py-0.5 text-[10px] text-white/60">Esc</kbd>
        </div>

        {q && (
          <div
            ref={listRef}
            className="mt-2 max-h-[46vh] overflow-y-auto rounded-2xl bg-black/30 p-1.5 ring-1 ring-white/12 backdrop-blur-xl"
          >
            {results.length === 0 && (
              <p className="px-4 py-6 text-center text-[13px] text-white/60">
                {t('agent.noResults', { q: query.trim() })}
              </p>
            )}
            {results.map((r, i) => (
              <button
                key={r.key}
                onClick={r.run}
                onMouseEnter={() => setSel(i)}
                className={`ios-ease flex w-full items-center gap-3 rounded-xl px-3 py-2 text-left ${
                  i === sel ? 'bg-white/16' : ''
                }`}
              >
                {r.kind === 'web' ? (
                  <span className="squircle grid size-8 shrink-0 place-items-center bg-[#2f6be0] text-white">
                    <Globe size={16} />
                  </span>
                ) : r.kind === 'ai' ? (
                  <span className="squircle grid size-8 shrink-0 place-items-center bg-[var(--accent)] text-white">
                    <Sparkles size={15} />
                  </span>
                ) : r.kind === 'todo' ? (
                  <span className="squircle grid size-8 shrink-0 place-items-center bg-[var(--accent)] text-white">
                    <ListTodo size={16} />
                  </span>
                ) : r.kind === 'file' && r.fileHit ? (
                  <FileKindIcon kind={r.fileHit.kind} />
                ) : r.iconPath ? (
                  <img
                    src={r.iconPath.startsWith('data:') ? r.iconPath : convertFileSrc(r.iconPath)}
                    alt=""
                    className="squircle size-8 shrink-0 object-cover"
                  />
                ) : (
                  <Monogram name={r.title} size={32} />
                )}
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-[14px] text-white">{r.title}</span>
                  <span className="block text-[11px] text-white/55">{r.subtitle}</span>
                </span>
                {i === sel && <CornerDownLeft size={14} className="shrink-0 text-white/50" />}
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
