/**
 * 锁屏解锁：主密码输入 + 提示语 + 错误反馈（闲置锁定时多一条说明横幅）。
 */
import { useState } from 'react';
import { KeyRound, Loader2, MoonStar } from 'lucide-react';
import { useI18n } from '@/shared/i18n/provider';
import { vaultErrIs } from './logic';
import { useVaultActions } from './hooks';

export function VaultUnlock({
  hint,
  idle,
  onUnlocked,
}: {
  hint: string | null;
  idle: boolean;
  onUnlocked: () => void;
}) {
  const { t } = useI18n();
  const { unlock } = useVaultActions();
  const [master, setMaster] = useState('');
  const [err, setErr] = useState('');

  const submit = async () => {
    setErr('');
    try {
      await unlock.mutateAsync(master);
      onUnlocked();
    } catch (e) {
      setErr(
        vaultErrIs(e, 'VAULT_AUTH')
          ? t('pages.vault.unlock.failed')
          : String((e as { message?: string })?.message ?? e),
      );
    }
  };

  return (
    <div className="mx-auto flex max-w-sm flex-col items-center px-4 py-16">
      <div className="mb-4 grid size-14 place-items-center rounded-2xl bg-[var(--accent-weak)] text-[var(--accent)]">
        <KeyRound size={26} strokeWidth={1.8} />
      </div>
      <h1 className="text-lg font-semibold">{t('pages.vault.unlock.title')}</h1>
      <p className="mt-1.5 mb-6 text-[12.5px] text-[var(--text-muted)]">
        {t('pages.vault.unlock.desc')}
      </p>

      {idle && (
        <p className="mb-4 flex items-center gap-1.5 rounded-xl bg-[var(--hover)] px-3 py-2 text-xs text-[var(--text-muted)]">
          <MoonStar size={13} />
          {t('pages.vault.unlock.idle')}
        </p>
      )}

      <div className="card flex w-full flex-col gap-3 p-5">
        <input
          type="password"
          value={master}
          onChange={(e) => setMaster(e.target.value)}
          placeholder={t('pages.vault.unlock.master')}
          autoFocus
          className="w-full rounded-xl bg-[var(--hover)] px-3.5 py-2.5 text-sm text-[var(--text)] outline-none ring-1 ring-transparent transition-shadow placeholder:text-[var(--text-muted)] focus:ring-[var(--accent)]"
          onKeyDown={(e) => e.key === 'Enter' && submit()}
        />
        {hint && (
          <p className="text-xs text-[var(--text-muted)]">{t('pages.vault.unlock.hint', { hint })}</p>
        )}
        {err && <p className="text-xs text-red-400">{err}</p>}
        <button
          onClick={submit}
          disabled={unlock.isPending || !master}
          className="flex items-center justify-center gap-2 rounded-xl bg-[var(--accent)] px-4 py-2.5 text-sm font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-60"
        >
          {unlock.isPending && <Loader2 size={15} className="animate-spin" />}
          {t('pages.vault.unlock.submit')}
        </button>
      </div>
    </div>
  );
}
