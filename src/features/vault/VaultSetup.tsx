/**
 * 首次创建密码库：主密码 + 确认 + 提示语 + 自动锁定时长。
 * 主密码只进解锁命令，不落任何存储；忘了他就没人能救（页面明说）。
 */
import { useState } from 'react';
import { KeyRound, Loader2 } from 'lucide-react';
import { useI18n } from '@/shared/i18n/provider';
import { AUTOLOCK_OPTIONS, autolockKey, vaultErrIs } from './logic';
import { useVaultActions } from './hooks';

const INPUT_CLS =
  'w-full rounded-xl bg-[var(--hover)] px-3.5 py-2.5 text-sm text-[var(--text)] outline-none ring-1 ring-transparent transition-shadow placeholder:text-[var(--text-muted)] focus:ring-[var(--accent)]';

export function VaultSetup() {
  const { t } = useI18n();
  const { setup } = useVaultActions();
  const [master, setMaster] = useState('');
  const [confirm, setConfirm] = useState('');
  const [hint, setHint] = useState('');
  const [autoLockSecs, setAutoLockSecs] = useState(300);
  const [err, setErr] = useState('');

  const submit = async () => {
    setErr('');
    if (master.length < 8) return setErr(t('pages.vault.setup.tooShort'));
    if (master !== confirm) return setErr(t('pages.vault.setup.mismatch'));
    try {
      await setup.mutateAsync({
        password: master,
        hint: hint.trim() || null,
        autoLockSecs,
      });
    } catch (e) {
      setErr(
        vaultErrIs(e, 'VALIDATE')
          ? t('pages.vault.setup.tooShort')
          : String((e as { message?: string })?.message ?? e),
      );
    }
  };

  return (
    <div className="mx-auto flex max-w-md flex-col items-center px-4 py-16">
      <div className="mb-4 grid size-14 place-items-center rounded-2xl bg-[var(--accent-weak)] text-[var(--accent)]">
        <KeyRound size={26} strokeWidth={1.8} />
      </div>
      <h1 className="text-lg font-semibold">{t('pages.vault.setup.title')}</h1>
      <p className="mt-2 mb-6 text-center text-[12.5px] leading-relaxed text-[var(--text-muted)]">
        {t('pages.vault.setup.desc')}
      </p>

      <div className="card flex w-full flex-col gap-3.5 p-5">
        <label className="flex flex-col gap-1.5">
          <span className="text-xs font-medium text-[var(--text-muted)]">
            {t('pages.vault.setup.master')}
          </span>
          <input
            type="password"
            value={master}
            onChange={(e) => setMaster(e.target.value)}
            placeholder={t('pages.vault.setup.masterPh')}
            className={INPUT_CLS}
            autoFocus
          />
        </label>
        <label className="flex flex-col gap-1.5">
          <span className="text-xs font-medium text-[var(--text-muted)]">
            {t('pages.vault.setup.confirm')}
          </span>
          <input
            type="password"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
            placeholder={t('pages.vault.setup.confirmPh')}
            className={INPUT_CLS}
            onKeyDown={(e) => e.key === 'Enter' && submit()}
          />
        </label>
        <label className="flex flex-col gap-1.5">
          <span className="text-xs font-medium text-[var(--text-muted)]">
            {t('pages.vault.setup.hint')}
          </span>
          <input
            value={hint}
            onChange={(e) => setHint(e.target.value)}
            placeholder={t('pages.vault.setup.hintPh')}
            className={INPUT_CLS}
          />
        </label>
        <label className="flex items-center justify-between gap-3">
          <span className="text-xs font-medium text-[var(--text-muted)]">
            {t('pages.vault.autolock.label')}
          </span>
          <select
            value={autoLockSecs}
            onChange={(e) => setAutoLockSecs(Number(e.target.value))}
            className="cursor-pointer rounded-lg bg-[var(--hover)] px-2 py-1.5 text-xs text-[var(--text)] outline-none [&>option]:bg-[#26232b]"
          >
            {AUTOLOCK_OPTIONS.map((s) => (
              <option key={s} value={s}>
                {t(autolockKey(s))}
              </option>
            ))}
          </select>
        </label>

        {err && <p className="text-xs text-red-400">{err}</p>}

        <button
          onClick={submit}
          disabled={setup.isPending}
          className="flex items-center justify-center gap-2 rounded-xl bg-[var(--accent)] px-4 py-2.5 text-sm font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-60"
        >
          {setup.isPending && <Loader2 size={15} className="animate-spin" />}
          {t('pages.vault.setup.submit')}
        </button>
      </div>
    </div>
  );
}
