import type { LucideIcon } from 'lucide-react';
import { useI18n } from '@/shared/i18n/provider';

/** 里程碑占位页：模块排期中，保持信息架构完整 */
export function StubRoute({
  icon: Icon,
  title,
  milestone,
  features,
}: {
  icon: LucideIcon;
  title: string;
  milestone: string;
  features: string[];
}) {
  const { t } = useI18n();
  return (
    <div className="mx-auto max-w-5xl">
      <h1 className="mb-5 text-xl font-semibold">{title}</h1>
      <section className="card flex flex-col items-center gap-4 border-dashed px-6 py-16">
        <span className="grid size-14 place-items-center rounded-2xl bg-[var(--accent-weak)] text-[var(--accent)]">
          <Icon size={26} strokeWidth={1.8} />
        </span>
        <div className="text-[15px] font-medium">
          {t('chrome.stub.milestoneLive', { milestone })}
        </div>
        <ul className="flex max-w-md flex-wrap justify-center gap-2">
          {features.map((f) => (
            <li
              key={f}
              className="rounded-full border border-[var(--border)] px-3 py-1 text-xs text-[var(--text-muted)]"
            >
              {f}
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
