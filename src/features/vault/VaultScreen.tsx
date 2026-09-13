/**
 * 密码箱主界面（三态状态机）：未初始化 → 创建；已锁 → 解锁；已解锁 → 列表。
 * 后端闲置锁定事件把页面拉回锁屏；复制经 Rust 侧写剪贴板并到时自动清除。
 */
import { useEffect, useRef, useState } from 'react';
import {
  Check,
  Copy,
  KeyRound,
  Loader2,
  Lock,
  Plus,
  Search,
  ShieldCheck,
  Star,
  UserRound,
} from 'lucide-react';
import { useI18n } from '@/shared/i18n/provider';
import type { VaultItem } from '@/shared/lib/ipc';
import { AUTOLOCK_OPTIONS, avatarHue, autolockKey, initialOf, vaultErrIs } from './logic';
import { useVaultActions, useVaultItems, useVaultLockListener, useVaultStatus } from './hooks';
import { VaultSetup } from './VaultSetup';
import { VaultUnlock } from './VaultUnlock';
import { VaultEditor } from './VaultEditor';

/** 首次复制提示（Windows 剪贴板历史会留存副本）只弹一次 */
const CLIPBOARD_HINT_LS = 'vault.clipboardHintShown';

export function VaultScreen() {
  const { t } = useI18n();
  const statusQuery = useVaultStatus();
  const [search, setSearch] = useState('');
  const [debounced, setDebounced] = useState('');
  const [editing, setEditing] = useState<VaultItem | 'new' | null>(null);
  const [showChangePw, setShowChangePw] = useState(false);
  const [idleLocked, setIdleLocked] = useState(false);
  const [toast, setToast] = useState<{ text: string; hint?: boolean } | null>(null);
  const toastTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const itemsQuery = useVaultItems(debounced);
  const { lock, setAutoLock, toggleFavorite, copyField } = useVaultActions();

  useVaultLockListener((reason) => {
    if (reason === 'idle') setIdleLocked(true);
    setEditing(null);
    setShowChangePw(false);
  });

  // 搜索防抖（200ms）
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(search.trim()), 200);
    return () => clearTimeout(timer);
  }, [search]);

  const showToast = (text: string, hint?: boolean) => {
    if (toastTimer.current) clearTimeout(toastTimer.current);
    setToast({ text, hint });
    toastTimer.current = setTimeout(() => setToast(null), hint ? 6000 : 2600);
  };
  useEffect(
    () => () => {
      if (toastTimer.current) clearTimeout(toastTimer.current);
    },
    [],
  );

  const copy = async (id: number, field: 'password' | 'username') => {
    try {
      const secs = await copyField.mutateAsync({ id, field });
      const firstTime = !localStorage.getItem(CLIPBOARD_HINT_LS);
      if (field === 'password') localStorage.setItem(CLIPBOARD_HINT_LS, '1');
      showToast(
        secs > 0 ? t('pages.vault.copy.done', { secs }) : t('pages.vault.copy.doneKeep'),
        firstTime && field === 'password',
      );
    } catch (e) {
      showToast(String((e as { message?: string })?.message ?? e));
    }
  };

  const status = statusQuery.data;
  if (statusQuery.isLoading || !status) {
    return (
      <div className="grid place-items-center py-24">
        <Loader2 size={22} className="animate-spin text-[var(--text-muted)]" />
      </div>
    );
  }
  if (!status.initialized) return <VaultSetup />;
  if (!status.unlocked) {
    return <VaultUnlock hint={status.hint} idle={idleLocked} onUnlocked={() => setIdleLocked(false)} />;
  }

  const items = itemsQuery.data ?? [];

  return (
    <div className="mx-auto max-w-3xl">
      {/* 页头 */}
      <div className="mb-5 flex flex-wrap items-center gap-3">
        <h1 className="flex items-center gap-2 text-xl font-semibold">
          <KeyRound size={20} className="text-[var(--accent)]" />
          {t('pages.vault.title')}
          <span className="text-[11px] font-normal text-[var(--text-muted)]">
            {t('pages.vault.list.count', { count: items.length })}
          </span>
        </h1>
        <div className="ml-auto flex items-center gap-2">
          <label className="flex items-center gap-1.5 text-[11px] text-[var(--text-muted)]">
            {t('pages.vault.autolock.label')}
            <select
              value={status.auto_lock_secs}
              onChange={(e) => setAutoLock.mutate(Number(e.target.value))}
              className="cursor-pointer rounded-lg bg-[var(--panel)] px-1.5 py-1 text-[11px] text-[var(--text)] outline-none ring-1 ring-[var(--border)] [&>option]:bg-[#26232b]"
            >
              {AUTOLOCK_OPTIONS.map((s) => (
                <option key={s} value={s}>
                  {t(autolockKey(s))}
                </option>
              ))}
            </select>
          </label>
          <button
            onClick={() => setShowChangePw(true)}
            title={t('pages.vault.change.title')}
            className="rounded-xl p-2 text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--text)]"
          >
            <ShieldCheck size={17} />
          </button>
          <button
            onClick={() => lock.mutate()}
            title={t('pages.vault.list.lock')}
            className="rounded-xl p-2 text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--text)]"
          >
            <Lock size={17} />
          </button>
          <button
            onClick={() => setEditing('new')}
            className="flex items-center gap-1.5 rounded-xl bg-[var(--accent)] px-3 py-2 text-[12.5px] font-medium text-white transition-opacity hover:opacity-90"
          >
            <Plus size={15} />
            {t('pages.vault.list.new')}
          </button>
        </div>
      </div>

      {/* 搜索 */}
      <div className="relative mb-4">
        <Search size={15} className="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--text-muted)]" />
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={t('pages.vault.list.searchPh')}
          className="w-full rounded-xl bg-[var(--panel)] py-2.5 pl-9 pr-3 text-sm text-[var(--text)] outline-none ring-1 ring-[var(--border)] transition-shadow placeholder:text-[var(--text-muted)] focus:ring-[var(--accent)]"
        />
      </div>

      {/* 列表 */}
      {itemsQuery.isLoading ? (
        <div className="grid place-items-center py-20">
          <Loader2 size={22} className="animate-spin text-[var(--text-muted)]" />
        </div>
      ) : items.length === 0 ? (
        debounced ? (
          <p className="py-20 text-center text-sm text-[var(--text-muted)]">
            {t('pages.vault.list.emptySearch')}
          </p>
        ) : (
          <div className="card grid place-items-center gap-1.5 py-16">
            <KeyRound size={28} strokeWidth={1.5} className="text-[var(--text-muted)]" />
            <p className="text-sm font-medium">{t('pages.vault.list.emptyTitle')}</p>
            <p className="text-xs text-[var(--text-muted)]">{t('pages.vault.list.emptyDesc')}</p>
          </div>
        )
      ) : (
        <div className="flex flex-col gap-2">
          {items.map((item) => (
            <div
              key={item.id}
              className="card flex items-center gap-3 px-4 py-3 transition-colors hover:bg-[var(--hover)]"
            >
              <button
                onClick={() => setEditing(item)}
                className="flex min-w-0 flex-1 items-center gap-3 text-left outline-none"
              >
                <span
                  className="grid size-9 shrink-0 place-items-center rounded-xl text-sm font-semibold text-white/90"
                  style={{ background: `hsl(${avatarHue(item.title)} 42% 34%)` }}
                >
                  {initialOf(item.title)}
                </span>
                <span className="min-w-0 flex-1">
                  <span className="flex items-center gap-1.5">
                    <span className="truncate text-sm font-medium text-[var(--text)]">
                      {item.title}
                    </span>
                    {item.favorite && (
                      <Star size={12} className="shrink-0 text-amber-400" fill="currentColor" />
                    )}
                  </span>
                  <span className="block truncate text-xs text-[var(--text-muted)]">
                    {item.username || item.url || '—'}
                  </span>
                </span>
              </button>
              <div className="flex shrink-0 items-center gap-0.5">
                <button
                  onClick={() => toggleFavorite.mutate({ id: item.id, favorite: !item.favorite })}
                  title={t('pages.vault.item.favorite')}
                  className={`rounded-lg p-2 transition-colors hover:bg-[var(--hover)] ${
                    item.favorite ? 'text-amber-400' : 'text-[var(--text-muted)]'
                  }`}
                >
                  <Star size={15} fill={item.favorite ? 'currentColor' : 'none'} />
                </button>
                <button
                  onClick={() => copy(item.id, 'username')}
                  disabled={!item.username}
                  title={t('pages.vault.copy.username')}
                  className="rounded-lg p-2 text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--text)] disabled:opacity-40"
                >
                  <UserRound size={15} />
                </button>
                <button
                  onClick={() => copy(item.id, 'password')}
                  title={t('pages.vault.copy.password')}
                  className="rounded-lg p-2 text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--accent)]"
                >
                  {copyField.isPending && copyField.variables?.field === 'password' ? (
                    <Loader2 size={15} className="animate-spin" />
                  ) : (
                    <Copy size={15} />
                  )}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* 复制反馈（含 Win+V 剪贴板历史一次性提示） */}
      {toast && (
        <div className="fixed bottom-8 left-1/2 z-50 flex -translate-x-1/2 flex-col items-center gap-1">
          <div className="flex items-center gap-2 rounded-full bg-[var(--panel)] px-4 py-2 text-xs text-[var(--text)] shadow-lg ring-1 ring-[var(--border)]">
            <Check size={13} className="text-emerald-400" />
            {toast.text}
          </div>
          {toast.hint && (
            <p className="max-w-[80vw] rounded-lg bg-[var(--panel)] px-3 py-1.5 text-center text-[11px] leading-relaxed text-[var(--text-muted)] shadow-lg ring-1 ring-[var(--border)]">
              {t('pages.vault.copy.hint')}
            </p>
          )}
        </div>
      )}

      {editing != null && (
        <VaultEditor item={editing === 'new' ? null : editing} onClose={() => setEditing(null)} />
      )}
      {showChangePw && <ChangePwModal onClose={() => setShowChangePw(false)} />}
    </div>
  );
}

/** 修改主密码弹层（旧密码校验失败按 code 映射文案） */
function ChangePwModal({ onClose }: { onClose: () => void }) {
  const { t } = useI18n();
  const { changePassword } = useVaultActions();
  const [oldPw, setOldPw] = useState('');
  const [newPw, setNewPw] = useState('');
  const [confirmPw, setConfirmPw] = useState('');
  const [err, setErr] = useState('');

  const submit = async () => {
    setErr('');
    if (newPw.length < 8) return setErr(t('pages.vault.setup.tooShort'));
    if (newPw !== confirmPw) return setErr(t('pages.vault.change.mismatch'));
    try {
      await changePassword.mutateAsync({ oldPassword: oldPw, newPassword: newPw, hint: null });
      onClose();
    } catch (e) {
      setErr(
        vaultErrIs(e, 'VAULT_AUTH')
          ? t('pages.vault.change.failed')
          : String((e as { message?: string })?.message ?? e),
      );
    }
  };

  const inputCls =
    'w-full rounded-xl bg-[var(--hover)] px-3.5 py-2.5 text-sm text-[var(--text)] outline-none ring-1 ring-transparent transition-shadow placeholder:text-[var(--text-muted)] focus:ring-[var(--accent)]';

  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/45 p-4 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="card flex w-[min(420px,94vw)] flex-col gap-3.5 p-5"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="text-[15px] font-semibold">{t('pages.vault.change.title')}</h2>
        <input
          type="password"
          value={oldPw}
          onChange={(e) => setOldPw(e.target.value)}
          placeholder={t('pages.vault.change.old')}
          className={inputCls}
        />
        <input
          type="password"
          value={newPw}
          onChange={(e) => setNewPw(e.target.value)}
          placeholder={t('pages.vault.change.new')}
          className={inputCls}
        />
        <input
          type="password"
          value={confirmPw}
          onChange={(e) => setConfirmPw(e.target.value)}
          placeholder={t('pages.vault.change.confirm')}
          className={inputCls}
          onKeyDown={(e) => e.key === 'Enter' && submit()}
        />
        {err && <p className="text-xs text-red-400">{err}</p>}
        <div className="flex items-center justify-end gap-2">
          <button
            onClick={onClose}
            className="rounded-xl px-3.5 py-2 text-sm text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--text)]"
          >
            {t('pages.vault.item.cancel')}
          </button>
          <button
            onClick={submit}
            disabled={changePassword.isPending || !oldPw || !newPw}
            className="flex items-center gap-2 rounded-xl bg-[var(--accent)] px-4 py-2 text-sm font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-60"
          >
            {changePassword.isPending && <Loader2 size={14} className="animate-spin" />}
            {t('pages.vault.change.submit')}
          </button>
        </div>
      </div>
    </div>
  );
}
