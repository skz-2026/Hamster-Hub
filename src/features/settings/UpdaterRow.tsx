/**
 * 软件更新（设置 → 系统）：经 GitHub Releases 检查/下载/静默安装
 * （Rust updater 命令 + UpdateProgress 事件；浏览器 mock 层返回假更新走全流程）。
 * GitHub 直连慢或失败时提供「打开发布页」手动下载兜底。
 */
import { useEffect, useState } from 'react';
import { ExternalLink, Loader2 } from 'lucide-react';
import { commands, events } from '@/shared/lib/ipc';
import { BlockRow } from '@/features/settings/controls';
import { useI18n } from '@/shared/i18n/provider';

const RELEASES_URL = 'https://github.com/skz-2026/Hamster-Hub/releases/latest';

type Phase = 'idle' | 'checking' | 'uptodate' | 'available' | 'downloading' | 'installing';

const errText = (e: unknown) => String(e).replace(/^Error:\s*/, '');

export default function UpdaterRow() {
  const { t } = useI18n();
  const [phase, setPhase] = useState<Phase>('idle');
  const [next, setNext] = useState('');
  const [percent, setPercent] = useState(0);
  const [error, setError] = useState('');

  useEffect(() => {
    // listen 返回 Promise<unlisten>，不能把 Promise 本身当清理函数返回
    let unlisten: (() => void) | null = null;
    let disposed = false;
    events.updateProgress.listen(({ payload }) => {
      if (payload.done) {
        setPhase('installing');
        return;
      }
      if (payload.total && payload.total > 0) {
        setPercent(Math.min(99, Math.round((payload.downloaded / payload.total) * 100)));
      }
    }).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const check = async () => {
    setPhase('checking');
    setError('');
    try {
      const update = await commands.updateCheck();
      if (update) {
        setNext(update.version);
        setPhase('available');
      } else {
        setPhase('uptodate');
      }
    } catch (e) {
      setError(errText(e));
      setPhase('idle');
    }
  };

  const install = async () => {
    setPhase('downloading');
    setPercent(0);
    setError('');
    try {
      await commands.updateInstall();
      // 正常不返回：安装器接管并退出应用；走到这视为已完成交棒
      setPhase('installing');
    } catch (e) {
      setError(errText(e));
      setPhase('available');
    }
  };

  return (
    <BlockRow title={t('settings.update.label')} desc={t('settings.update.desc')}>
      <div className="flex flex-col gap-2">
        <div className="flex flex-wrap items-center gap-2">
          {phase === 'checking' || phase === 'installing' ? (
            <span className="flex items-center gap-1.5 text-xs text-[var(--text-muted)]">
              <Loader2 size={13} className="animate-spin" />
              {t(phase === 'checking' ? 'settings.update.checking' : 'settings.update.installing')}
            </span>
          ) : phase === 'downloading' ? (
            <span className="flex items-center gap-2 text-xs text-[var(--text-muted)]">
              <span className="h-1 w-40 overflow-hidden rounded-full bg-[var(--border)]">
                <span
                  className="block h-full rounded-full bg-[var(--accent)] transition-[width]"
                  style={{ width: `${percent}%` }}
                />
              </span>
              {t('settings.update.downloading', { percent })}
            </span>
          ) : phase === 'uptodate' ? (
            <span className="text-xs text-[var(--text-muted)]">{t('settings.update.upToDate')}</span>
          ) : phase === 'available' ? (
            <>
              <span className="text-xs font-medium text-[var(--accent)]">
                {t('settings.update.available', { version: next })}
              </span>
              <button
                onClick={() => void install()}
                className="flex shrink-0 items-center gap-1.5 rounded-lg bg-[var(--accent)] px-3.5 py-1.5 text-xs font-medium text-white transition-opacity hover:opacity-90"
              >
                {t('settings.update.download')}
              </button>
            </>
          ) : (
            <button
              onClick={() => void check()}
              className="flex shrink-0 items-center gap-1.5 rounded-lg bg-[var(--accent)] px-3.5 py-1.5 text-xs font-medium text-white transition-opacity hover:opacity-90"
            >
              {t('settings.update.check')}
            </button>
          )}
          {phase !== 'installing' && phase !== 'downloading' && (
            <button
              onClick={() => void commands.openUrl(RELEASES_URL).catch(() => {})}
              title={t('settings.update.releasePageHint')}
              className="flex shrink-0 items-center gap-1 text-xs text-[var(--text-muted)] underline-offset-2 hover:underline"
            >
              <ExternalLink size={12} />
              {t('settings.update.releasePage')}
            </button>
          )}
        </div>
        {error && <p className="text-[11px] text-red-300">{t('settings.update.error', { message: error })}</p>}
      </div>
    </BlockRow>
  );
}
