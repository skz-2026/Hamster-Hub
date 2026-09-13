/**
 * SelectPill：composer 内的轻量下拉药丸（项目 / 代理 / 模型 / 推理强度）。
 * 点击弹绝对定位面板，点 backdrop 关闭；accent 态用主题橙（对齐参考稿「完全访问」）。
 * variant="field" 用于常规表单（设置页）：主题变量描边样式 + 可向下展开。
 */
import { useState } from 'react';
import { ChevronDown } from 'lucide-react';
import { useI18n } from '@/shared/i18n/provider';

export interface SelectOption {
  value: string;
  label: string;
  hint?: string;
  /** 选项行首图标（如 Agent 品牌头像） */
  icon?: React.ReactNode;
}

/** 面板底部固定动作（如「浏览文件夹…」） */
export interface SelectFooter {
  label: string;
  icon?: React.ReactNode;
  onSelect: () => void;
}

export default function SelectPill({
  icon,
  value,
  options,
  onChange,
  placeholder,
  accent = false,
  title,
  footer,
  direction = 'up',
  variant = 'composer',
  onOpenChange,
}: {
  icon?: React.ReactNode;
  value: string | null;
  options: SelectOption[];
  onChange: (v: string) => void;
  placeholder: string;
  accent?: boolean;
  title?: string;
  footer?: SelectFooter;
  /** 面板展开方向：composer 底行向上（默认），表单页向下 */
  direction?: 'up' | 'down';
  /** composer = 药丸融入深底；field = 描边输入框样式（跟随主题变量） */
  variant?: 'composer' | 'field';
  /** 开合上报（宿主据此抬层，避免弹层被相邻层叠上下文压住） */
  onOpenChange?: (open: boolean) => void;
}) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const current = options.find((o) => o.value === value);
  const close = () => {
    setOpen(false);
    onOpenChange?.(false);
  };

  return (
    <div className="relative">
      <button
        onClick={() =>
          setOpen((v) => {
            onOpenChange?.(!v);
            return !v;
          })
        }
        title={title ?? current?.label ?? placeholder}
        className={`flex max-w-[280px] items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[12px] transition-colors ${
          accent
            ? 'text-[var(--accent)] hover:bg-[var(--accent-weak)]'
            : variant === 'field'
              ? 'h-8 border border-[var(--border)] text-[var(--text-muted)] hover:border-[var(--accent)] hover:text-[var(--text)]'
              : 'text-[var(--text-muted)] hover:bg-[var(--hover)]'
        }`}
      >
        {icon}
        <span className="truncate">{current?.label ?? placeholder}</span>
        <ChevronDown size={12} className="shrink-0 opacity-60" />
      </button>
      {open && (
        <>
          <div className="fixed inset-0 z-40" onClick={close} />
          <div
            className={`absolute left-0 z-50 max-h-72 w-max min-w-[224px] max-w-[420px] overflow-y-auto rounded-xl border border-[var(--border)] bg-[var(--popover)] shadow-2xl ${
              direction === 'down' ? 'top-full mt-1.5' : 'bottom-full mb-1.5'
            }`}
          >
            {options.map((o) => (
              <button
                key={o.value}
                onClick={() => {
                  onChange(o.value);
                  close();
                }}
                className={`flex w-full items-center gap-3 px-3 py-2 text-left text-[12.5px] transition-colors hover:bg-[var(--hover)] ${
                  o.value === value ? 'text-[var(--accent)]' : 'text-[var(--text)]'
                }`}
              >
                {o.icon}
                <span className="whitespace-nowrap">{o.label}</span>
                {o.hint && (
                  <span className="max-w-[280px] truncate text-[10.5px] text-[var(--text-muted)]">{o.hint}</span>
                )}
              </button>
            ))}
            {options.length === 0 && <p className="px-3 py-2.5 text-[12px] text-[var(--text-muted)]">{t('chrome.select.noOptions')}</p>}
            {footer && (
              <button
                onClick={() => {
                  close();
                  footer.onSelect();
                }}
                className="flex w-full items-center gap-2 border-t border-[var(--border)] px-3 py-2 text-left text-[12.5px] text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--text)]"
              >
                {footer.icon}
                <span className="whitespace-nowrap">{footer.label}</span>
              </button>
            )}
          </div>
        </>
      )}
    </div>
  );
}
