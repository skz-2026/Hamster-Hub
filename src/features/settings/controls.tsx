/**
 * 设置页共享原语：
 * - Section：锚点小节（标题 + 内容列），配合页头锚点导航滚动定位；
 * - Group：分组卡，行间分隔线；elevated = 抬层（内部有下拉弹层时盖过后续分组卡）；
 * - Row / BlockRow：右对齐控制行 / 控件下堆叠行；
 * - Switch / ThemeButton：即点即存控件；
 * - AutoSaveField：文本类统一保存交互——失焦 / Enter 保存，Esc 还原，
 *   行内反馈 待保存 / 保存中 / 已保存 / 保存失败（取代散落的「保存 / 应用」按钮）。
 */
import { useEffect, useState } from 'react';
import { Loader2 } from 'lucide-react';
import { useI18n } from '@/shared/i18n/provider';

export function Section({
  id,
  title,
  children,
}: {
  id: string;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section id={id} className="scroll-mt-24">
      <h2 className="mb-2 px-1 text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--text-muted)]">
        {title}
      </h2>
      <div className="flex flex-col gap-3">{children}</div>
    </section>
  );
}

export function Group({
  elevated = false,
  children,
}: {
  elevated?: boolean;
  children: React.ReactNode;
}) {
  return (
    // elevated：.card 的 backdrop-filter 各自成层叠上下文，含下拉的分组需抬层
    <div className={`card divide-y divide-[var(--border)] ${elevated ? 'relative z-20' : ''}`}>
      {children}
    </div>
  );
}

export function Row({
  title,
  desc,
  children,
}: {
  title: string;
  desc?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-6 px-4 py-3.5">
      <div className="min-w-0">
        <div className="text-sm font-medium">{title}</div>
        {desc != null && <div className="mt-0.5 text-xs text-[var(--text-muted)]">{desc}</div>}
      </div>
      {children}
    </div>
  );
}

export function BlockRow({
  title,
  icon,
  desc,
  children,
}: {
  title: string;
  icon?: React.ReactNode;
  desc?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="px-4 py-3.5">
      <div className="flex items-center gap-1.5 text-sm font-medium">
        {icon}
        {title}
      </div>
      {desc != null && (
        <div className="mt-0.5 text-xs leading-relaxed text-[var(--text-muted)]">{desc}</div>
      )}
      <div className="mt-3">{children}</div>
    </div>
  );
}

export function Switch({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
}) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={`relative h-6 w-11 shrink-0 rounded-full transition-colors ${
        checked ? 'bg-[var(--accent)]' : 'bg-[var(--panel-strong)] border border-[var(--border)]'
      }`}
    >
      <span
        className={`absolute top-0.5 size-5 rounded-full bg-white shadow transition-all ${
          checked ? 'left-[22px]' : 'left-0.5'
        }`}
      />
    </button>
  );
}

export function ThemeButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-1.5 rounded-lg border px-3 py-1.5 text-xs transition-colors ${
        active
          ? 'border-[var(--accent)] bg-[var(--accent-weak)] text-[var(--accent)]'
          : 'border-[var(--border)] text-[var(--text-muted)] hover:text-[var(--text)]'
      }`}
    >
      {children}
    </button>
  );
}

/** 文本统一保存交互的输入框；password 自带显示/隐藏，multiline 为 textarea（失焦保存） */
export function AutoSaveField({
  value,
  onSave,
  placeholder,
  ariaLabel,
  className = 'w-40',
  mono = false,
  password = false,
  multiline = false,
  rows = 3,
  allowEmpty = true,
}: {
  value: string;
  onSave: (next: string) => Promise<unknown> | unknown;
  placeholder?: string;
  ariaLabel?: string;
  className?: string;
  mono?: boolean;
  password?: boolean;
  multiline?: boolean;
  rows?: number;
  /** false：清空不保存、直接还原（热键 / 城市等必填项） */
  allowEmpty?: boolean;
}) {
  const [draft, setDraft] = useState(value);
  const [reveal, setReveal] = useState(false);
  const [st, setSt] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle');
  const { t } = useI18n();
  const dirty = draft !== value;

  // 外部值变化（含保存成功回写）后同步草稿
  useEffect(() => setDraft(value), [value]);
  // 已保存提示自动消退
  useEffect(() => {
    if (st !== 'saved') return;
    const t = setTimeout(() => setSt('idle'), 1600);
    return () => clearTimeout(t);
  }, [st]);

  const commit = async () => {
    if (!dirty || st === 'saving') return;
    const next = draft.trim();
    if (!allowEmpty && !next) {
      setDraft(value);
      return;
    }
    setSt('saving');
    try {
      await onSave(next);
      setSt('saved');
    } catch {
      setSt('error');
    }
  };

  const status =
    st === 'idle' ? null : (
      <span
        className={`shrink-0 text-[11px] ${
          st === 'error'
            ? 'text-red-300'
            : st === 'saved'
              ? 'text-emerald-300/90'
              : 'text-[var(--text-muted)]'
        }`}
      >
        {st === 'saving' ? t('settings.save.saving') : st === 'saved' ? t('settings.save.saved') : t('settings.save.error')}
      </span>
    );
  const pending = dirty && st === 'idle' && (
    <span className="shrink-0 text-[11px] text-amber-300/80">{t('settings.save.pending')}</span>
  );

  const base =
    'rounded-lg border border-[var(--border)] bg-transparent px-2.5 text-xs outline-none transition-colors focus:border-[var(--accent)] placeholder:text-[var(--text-muted)]';

  if (multiline) {
    return (
      <div className="flex flex-col items-end gap-1">
        <textarea
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === 'Escape') setDraft(value);
          }}
          rows={rows}
          aria-label={ariaLabel ?? placeholder}
          placeholder={placeholder}
          className={`${base} ${mono ? 'font-mono' : ''} resize-none py-1.5 ${className}`}
        />
        <div className="flex items-center gap-2">
          {pending}
          {st === 'saving' && <Loader2 size={11} className="animate-spin text-[var(--text-muted)]" />}
          {status}
        </div>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2">
      <div className="relative">
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === 'Enter') void commit();
            if (e.key === 'Escape') setDraft(value);
          }}
          type={password && !reveal ? 'password' : 'text'}
          aria-label={ariaLabel ?? placeholder}
          placeholder={placeholder}
          className={`${base} ${mono ? 'font-mono' : ''} h-8 ${className} ${password ? 'pr-9' : ''}`}
        />
        {password && (
          <button
            onClick={() => setReveal((v) => !v)}
            title={reveal ? t('settings.field.hide') : t('settings.field.show')}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-[11px] text-[var(--text-muted)] hover:text-[var(--text)]"
          >
            {reveal ? t('settings.field.hide') : t('settings.field.show')}
          </button>
        )}
      </div>
      {pending}
      {st === 'saving' && <Loader2 size={11} className="shrink-0 animate-spin text-[var(--text-muted)]" />}
      {status}
    </div>
  );
}
