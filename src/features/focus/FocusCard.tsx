import { Pause, Play, Square, Timer } from 'lucide-react';
import { useFocus } from './hooks';
import { formatMmss } from './fmt';

/** 番茄钟卡（工作台）：25 分钟专注 / 5 分钟休息，后端计时（托盘常驻不中断），历史按日聚合 */
export default function FocusCard() {
  const { statusQ, historyQ, start, startBreak, pause, resume, stop } = useFocus();
  const status = statusQ.data ?? null;
  const history = historyQ.data ?? [];
  const today = history.length > 0 ? history[history.length - 1].minutes : 0;

  return (
    <section className="card col-span-12 flex min-h-[190px] flex-col p-5 md:col-span-4">
      <header className="mb-3 flex items-center justify-between text-sm font-medium text-white/90">
        <span className="flex items-center gap-2">
          <Timer size={16} className="text-[var(--accent)]" />
          专注
        </span>
        <span className="text-[11px] text-white/45 tabular-nums">今日 {today} 分钟</span>
      </header>

      {status ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3">
          <div className="text-[44px] font-light leading-none text-white tabular-nums">
            {formatMmss(status.remaining_secs)}
          </div>
          <div className="text-[12px] text-white/60">
            {status.paused ? '已暂停' : status.kind === 'break' ? '休息中 ☕' : '专注中 🐹'}
          </div>
          <div className="flex items-center gap-2">
            {status.paused ? (
              <button
                onClick={() => resume.mutate()}
                className="flex items-center gap-1 rounded-lg bg-[var(--accent)] px-3 py-1.5 text-[12px] text-white transition-opacity hover:opacity-90"
              >
                <Play size={12} />
                继续
              </button>
            ) : (
              <button
                onClick={() => pause.mutate()}
                className="flex items-center gap-1 rounded-lg bg-white/10 px-3 py-1.5 text-[12px] text-white/85 ring-1 ring-white/15 transition-colors hover:bg-white/16"
              >
                <Pause size={12} />
                暂停
              </button>
            )}
            <button
              onClick={() => stop.mutate()}
              className="flex items-center gap-1 rounded-lg px-3 py-1.5 text-[12px] text-white/55 transition-colors hover:text-white"
            >
              <Square size={11} />
              结束
            </button>
          </div>
        </div>
      ) : (
        <div className="flex flex-1 flex-col items-center justify-center gap-2.5">
          <button
            onClick={() => start.mutate(25)}
            className="w-full max-w-[180px] rounded-xl bg-[var(--accent)] py-2.5 text-[13px] font-medium text-white transition-opacity hover:opacity-90"
          >
            专注 25 分钟
          </button>
          <button
            onClick={() => startBreak.mutate(5)}
            className="w-full max-w-[180px] rounded-xl bg-white/8 py-2 text-[12px] text-white/70 ring-1 ring-white/12 transition-colors hover:bg-white/14"
          >
            休息 5 分钟
          </button>
          <p className="text-[10.5px] text-white/35">收进托盘也照常计时</p>
        </div>
      )}
    </section>
  );
}
