/**
 * SelectPill：composer 内的轻量下拉药丸（项目 / 代理 / 模型 / 推理强度）。
 * 点击弹绝对定位面板，点 backdrop 关闭；accent 态用主题橙（对齐参考稿「完全访问」）。
 */
import { useState } from 'react';
import { ChevronDown } from 'lucide-react';

export interface SelectOption {
  value: string;
  label: string;
  hint?: string;
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
}: {
  icon?: React.ReactNode;
  value: string | null;
  options: SelectOption[];
  onChange: (v: string) => void;
  placeholder: string;
  accent?: boolean;
  title?: string;
  footer?: SelectFooter;
}) {
  const [open, setOpen] = useState(false);
  const current = options.find((o) => o.value === value);

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        title={title ?? current?.label ?? placeholder}
        className={`flex max-w-[280px] items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[12px] transition-colors ${
          accent
            ? 'text-[var(--accent)] hover:bg-[var(--accent-weak)]'
            : 'text-white/70 hover:bg-white/8'
        }`}
      >
        {icon}
        <span className="truncate">{current?.label ?? placeholder}</span>
        <ChevronDown size={12} className="shrink-0 opacity-60" />
      </button>
      {open && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setOpen(false)} />
          <div className="absolute bottom-full left-0 z-50 mb-1.5 max-h-72 w-max min-w-[224px] max-w-[420px] overflow-y-auto rounded-xl border border-white/12 bg-[#26242b] shadow-2xl">
            {options.map((o) => (
              <button
                key={o.value}
                onClick={() => {
                  onChange(o.value);
                  setOpen(false);
                }}
                className={`flex w-full items-center gap-3 px-3 py-2 text-left text-[12.5px] transition-colors hover:bg-white/8 ${
                  o.value === value ? 'text-[var(--accent)]' : 'text-white/80'
                }`}
              >
                <span className="whitespace-nowrap">{o.label}</span>
                {o.hint && (
                  <span className="max-w-[280px] truncate text-[10.5px] text-white/35">{o.hint}</span>
                )}
              </button>
            ))}
            {options.length === 0 && <p className="px-3 py-2.5 text-[12px] text-white/40">暂无可选项</p>}
            {footer && (
              <button
                onClick={() => {
                  setOpen(false);
                  footer.onSelect();
                }}
                className="flex w-full items-center gap-2 border-t border-white/8 px-3 py-2 text-left text-[12.5px] text-white/60 transition-colors hover:bg-white/8 hover:text-white"
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
