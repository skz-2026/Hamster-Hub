/**
 * 条目编辑弹层：新建 / 编辑共用。保存是全量替换，因此编辑态打开时会
 * 自动取一次密（默认仍打码显示）。内嵌生成器与强度条。
 */
import { useEffect, useState } from 'react';
import { Eye, EyeOff, Loader2, RefreshCw, Star, Trash2, Wand2, X } from 'lucide-react';
import { commands, type VaultItem } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';
import { STRENGTH_KEYS, strengthTone } from './logic';
import { useVaultActions } from './hooks';

const INPUT_CLS =
  'w-full rounded-xl bg-[var(--hover)] px-3.5 py-2.5 text-sm text-[var(--text)] outline-none ring-1 ring-transparent transition-shadow placeholder:text-[var(--text-muted)] focus:ring-[var(--accent)]';

interface GenState {
  length: number;
  upper: boolean;
  lower: boolean;
  digits: boolean;
  symbols: boolean;
  exclude_ambiguous: boolean;
}

export function VaultEditor({ item, onClose }: { item: VaultItem | null; onClose: () => void }) {
  const { t } = useI18n();
  const { create, update, remove, reveal, generate } = useVaultActions();
  const editing = item != null;

  const [title, setTitle] = useState(item?.title ?? '');
  const [username, setUsername] = useState(item?.username ?? '');
  const [url, setUrl] = useState(item?.url ?? '');
  const [password, setPassword] = useState('');
  const [notes, setNotes] = useState('');
  const [favorite, setFavorite] = useState(item?.favorite ?? false);
  const [showPw, setShowPw] = useState(false);
  const [strength, setStrength] = useState(0);
  const [err, setErr] = useState('');
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [genOpen, setGenOpen] = useState(false);
  const [gen, setGen] = useState<GenState>({
    length: 16,
    upper: true,
    lower: true,
    digits: true,
    symbols: true,
    exclude_ambiguous: false,
  });
  const [genPw, setGenPw] = useState('');

  // 编辑态取现密（保存是全量替换；默认仍打码，仅强度条需要值本身在后端算）
  useEffect(() => {
    if (item == null) return;
    let alive = true;
    reveal.mutate(item.id, {
      onSuccess: (secret) => {
        if (!alive) return;
        setPassword(secret.password);
        setNotes(secret.notes);
      },
      onError: (e) => alive && setErr(String((e as { message?: string })?.message ?? e)),
    });
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 只在目标条目变化时取一次
  }, [item?.id]);

  useEffect(() => {
    let alive = true;
    if (!password) {
      setStrength(0);
      return;
    }
    commands
      .vaultPasswordStrength(password)
      .then((s) => alive && setStrength(s))
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [password]);

  const runGenerate = async () => {
    try {
      const pw = await generate.mutateAsync(gen);
      setGenPw(pw);
    } catch (e) {
      setErr(String((e as { message?: string })?.message ?? e));
    }
  };

  const save = async () => {
    setErr('');
    if (!title.trim()) return setErr(t('pages.vault.item.titleRequired'));
    const input = {
      title: title.trim(),
      username: username.trim(),
      url: url.trim(),
      password,
      notes,
      favorite,
    };
    try {
      if (editing) await update.mutateAsync({ id: item.id, input });
      else await create.mutateAsync(input);
      onClose();
    } catch (e) {
      setErr(String((e as { message?: string })?.message ?? e));
    }
  };

  const doDelete = async () => {
    if (item == null) return;
    try {
      await remove.mutateAsync(item.id);
      onClose();
    } catch (e) {
      setErr(String((e as { message?: string })?.message ?? e));
    }
  };

  const saving = create.isPending || update.isPending;

  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/45 p-4 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="card flex max-h-[88vh] w-[min(540px,94vw)] flex-col gap-3.5 overflow-y-auto p-5"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between">
          <h2 className="text-[15px] font-semibold">
            {editing ? t('pages.vault.item.editTitle') : t('pages.vault.item.newTitle')}
          </h2>
          <div className="flex items-center gap-1.5">
            <button
              onClick={() => setFavorite((v) => !v)}
              title={t('pages.vault.item.favorite')}
              className={`rounded-lg p-1.5 transition-colors hover:bg-[var(--hover)] ${
                favorite ? 'text-amber-400' : 'text-[var(--text-muted)]'
              }`}
            >
              <Star size={16} fill={favorite ? 'currentColor' : 'none'} />
            </button>
            <button
              onClick={onClose}
              className="rounded-lg p-1.5 text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--text)]"
            >
              <X size={16} />
            </button>
          </div>
        </div>

        <label className="flex flex-col gap-1.5">
          <span className="text-xs font-medium text-[var(--text-muted)]">
            {t('pages.vault.item.title')}
          </span>
          <input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder={t('pages.vault.item.titlePh')}
            className={INPUT_CLS}
            autoFocus={!editing}
          />
        </label>
        <div className="grid grid-cols-2 gap-3">
          <label className="flex flex-col gap-1.5">
            <span className="text-xs font-medium text-[var(--text-muted)]">
              {t('pages.vault.item.username')}
            </span>
            <input
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder={t('pages.vault.item.usernamePh')}
              className={INPUT_CLS}
            />
          </label>
          <label className="flex flex-col gap-1.5">
            <span className="text-xs font-medium text-[var(--text-muted)]">
              {t('pages.vault.item.url')}
            </span>
            <input
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder={t('pages.vault.item.urlPh')}
              className={INPUT_CLS}
            />
          </label>
        </div>

        {/* 密码：打码切换 + 强度条 + 生成器开关 */}
        <div className="flex flex-col gap-1.5">
          <span className="text-xs font-medium text-[var(--text-muted)]">
            {t('pages.vault.item.password')}
          </span>
          <div className="flex gap-2">
            <div className="relative flex-1">
              <input
                type={showPw ? 'text' : 'password'}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className={`${INPUT_CLS} pr-9 font-mono`}
              />
              <button
                onClick={() => setShowPw((v) => !v)}
                title={showPw ? t('pages.vault.item.hide') : t('pages.vault.item.reveal')}
                className="absolute right-1.5 top-1/2 -translate-y-1/2 rounded-lg p-1.5 text-[var(--text-muted)] transition-colors hover:text-[var(--text)]"
              >
                {showPw ? <EyeOff size={14} /> : <Eye size={14} />}
              </button>
            </div>
            <button
              onClick={() => {
                setGenOpen((v) => !v);
                if (!genOpen && !genPw) void runGenerate();
              }}
              title={t('pages.vault.gen.title')}
              className={`flex items-center gap-1.5 rounded-xl px-3 text-xs transition-colors ${
                genOpen
                  ? 'bg-[var(--accent-weak)] text-[var(--accent)]'
                  : 'text-[var(--text-muted)] hover:bg-[var(--hover)] hover:text-[var(--text)]'
              }`}
            >
              <Wand2 size={14} />
              {t('pages.vault.gen.title')}
            </button>
          </div>
          {password && (
            <div className="flex items-center gap-2">
              <div className="flex h-1 flex-1 gap-1 overflow-hidden rounded-full">
                {[0, 1, 2, 3].map((i) => (
                  <div
                    key={i}
                    className={`h-full flex-1 rounded-full ${
                      i < Math.max(1, strength) ? strengthTone(strength) : 'bg-[var(--hover)]'
                    }`}
                  />
                ))}
              </div>
              <span className="w-10 text-right text-[11px] text-[var(--text-muted)]">
                {t(STRENGTH_KEYS[strength])}
              </span>
            </div>
          )}
          {genOpen && (
            <div className="mt-1 rounded-xl bg-[var(--hover)] p-3">
              <div className="flex items-center gap-2">
                <input
                  value={genPw}
                  readOnly
                  className="min-w-0 flex-1 rounded-lg bg-[var(--panel)] px-2.5 py-1.5 font-mono text-xs text-[var(--text)] outline-none ring-1 ring-transparent"
                />
                <button
                  onClick={runGenerate}
                  disabled={generate.isPending}
                  title={t('pages.vault.gen.refresh')}
                  className="rounded-lg p-1.5 text-[var(--text-muted)] transition-colors hover:text-[var(--text)]"
                >
                  <RefreshCw size={14} className={generate.isPending ? 'animate-spin' : ''} />
                </button>
                <button
                  onClick={() => {
                    if (!genPw) return;
                    setPassword(genPw);
                    setGenOpen(false);
                  }}
                  disabled={!genPw}
                  className="rounded-lg bg-[var(--accent)] px-2.5 py-1.5 text-xs font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-50"
                >
                  {t('pages.vault.gen.apply')}
                </button>
              </div>
              <div className="mt-2.5 flex flex-wrap items-center gap-x-4 gap-y-2 text-[11px] text-[var(--text-muted)]">
                <label className="flex items-center gap-1.5">
                  {t('pages.vault.gen.length')}
                  <input
                    type="range"
                    min={8}
                    max={64}
                    value={gen.length}
                    onChange={(e) => setGen({ ...gen, length: Number(e.target.value) })}
                    onMouseUp={runGenerate}
                    onTouchEnd={runGenerate}
                    className="h-1 w-24 accent-[var(--accent)]"
                  />
                  <span className="w-5 tabular-nums">{gen.length}</span>
                </label>
                {(
                  [
                    ['upper', 'pages.vault.gen.upper'],
                    ['lower', 'pages.vault.gen.lower'],
                    ['digits', 'pages.vault.gen.digits'],
                    ['symbols', 'pages.vault.gen.symbols'],
                  ] as const
                ).map(([key, labelKey]) => (
                  <label key={key} className="flex cursor-pointer items-center gap-1">
                    <input
                      type="checkbox"
                      checked={gen[key]}
                      onChange={(e) => setGen({ ...gen, [key]: e.target.checked })}
                      className="accent-[var(--accent)]"
                    />
                    {t(labelKey)}
                  </label>
                ))}
                <label className="flex cursor-pointer items-center gap-1">
                  <input
                    type="checkbox"
                    checked={gen.exclude_ambiguous}
                    onChange={(e) => setGen({ ...gen, exclude_ambiguous: e.target.checked })}
                    className="accent-[var(--accent)]"
                  />
                  {t('pages.vault.gen.ambiguous')}
                </label>
              </div>
            </div>
          )}
        </div>

        <label className="flex flex-col gap-1.5">
          <span className="text-xs font-medium text-[var(--text-muted)]">
            {t('pages.vault.item.notes')}
          </span>
          <textarea
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            placeholder={t('pages.vault.item.notesPh')}
            rows={2}
            className={`${INPUT_CLS} resize-none`}
          />
        </label>

        {err && <p className="text-xs text-red-400">{err}</p>}

        <div className="mt-1 flex items-center gap-2">
          {editing && !confirmDelete && (
            <button
              onClick={() => setConfirmDelete(true)}
              className="flex items-center gap-1.5 rounded-xl px-3 py-2 text-xs text-red-400 transition-colors hover:bg-red-500/10"
            >
              <Trash2 size={14} />
              {t('pages.vault.item.delete')}
            </button>
          )}
          {editing && confirmDelete && (
            <div className="flex flex-1 flex-wrap items-center gap-2 text-xs">
              <span className="text-[var(--text-muted)]">
                {t('pages.vault.item.deleteConfirm', { title: item.title })}
              </span>
              <button
                onClick={doDelete}
                disabled={remove.isPending}
                className="flex items-center gap-1 rounded-lg bg-red-500/90 px-2.5 py-1 text-white transition-opacity hover:opacity-90"
              >
                {remove.isPending && <Loader2 size={12} className="animate-spin" />}
                {t('pages.vault.item.delete')}
              </button>
              <button
                onClick={() => setConfirmDelete(false)}
                className="rounded-lg px-2.5 py-1 text-[var(--text-muted)] hover:text-[var(--text)]"
              >
                {t('pages.vault.item.cancel')}
              </button>
            </div>
          )}
          <div className="ml-auto flex items-center gap-2">
            <button
              onClick={onClose}
              className="rounded-xl px-3.5 py-2 text-sm text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--text)]"
            >
              {t('pages.vault.item.cancel')}
            </button>
            <button
              onClick={save}
              disabled={saving}
              className="flex items-center gap-2 rounded-xl bg-[var(--accent)] px-4 py-2 text-sm font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-60"
            >
              {saving && <Loader2 size={14} className="animate-spin" />}
              {t('pages.vault.item.save')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
