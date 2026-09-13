import { convertFileSrc } from '@tauri-apps/api/core';
import { useClock, useDateTimeInfo } from '@/features/workbench/hooks';
import { useCountdown, useKillProcess, useProcessList, useSystemStats, useWeather } from '@/features/dashboard/hooks';
import { useTodos } from '@/features/todo/hooks';
import { useI18n } from '@/shared/i18n/provider';
import { pluginIdOf } from './layout';
import PluginWidgetHost from '@/features/plugins/PluginWidgetHost';

const glass =
  'h-full w-full overflow-hidden rounded-[20px] bg-white/[0.13] p-3.5 ring-1 ring-white/15 backdrop-blur-2xl transition-colors hover:bg-white/[0.17]';

/** 主屏小组件（2 列宽玻璃卡）：按类型渲染，数据全部来自真实 hooks。
 *  type 为 `widget:plugin:<id>` 时渲染插件小组件（M4 插件域）。 */
export function HomeWidget({ type }: { type: string }) {
  const pluginId = pluginIdOf(type);
  return (
    <div className="col-span-2 h-[104px]">
      <div className={glass}>
        {pluginId ? (
          <PluginWidgetHost pluginId={pluginId} />
        ) : (
          <>
            {type === 'clock' && <ClockWidget />}
            {type === 'weather' && <WeatherWidget />}
            {type === 'todo' && <TodoWidget />}
            {type === 'countdown' && <CountdownWidget />}
            {type === 'sysinfo' && <SysinfoWidget />}
            {type === 'taskmgr' && <TaskmgrWidget />}
          </>
        )}
      </div>
    </div>
  );
}

function ClockWidget() {
  const now = useClock();
  const { time, date } = useDateTimeInfo(now);
  return (
    <div className="flex h-full flex-col justify-center">
      <div className="text-[26px] font-light leading-none text-white tabular-nums">{time}</div>
      <div className="mt-1.5 truncate text-[10.5px] text-white/70">{date}</div>
    </div>
  );
}

function WeatherWidget() {
  const { t } = useI18n();
  const { data: w, isError } = useWeather();
  if (isError || !w) {
    return <Empty text={t('home.weather.unavailable')} />;
  }
  return (
    <div className="flex h-full items-center gap-2.5">
      <span className="text-[30px] leading-none">{w.emoji}</span>
      <div className="min-w-0">
        <div className="text-[22px] font-light leading-none text-white tabular-nums">{w.temp}°</div>
        <div className="mt-1 truncate text-[10.5px] text-white/70">
          {w.kind} {w.temp_min}°~{w.temp_max}°
        </div>
      </div>
    </div>
  );
}

function TodoWidget() {
  const { t } = useI18n();
  const { query } = useTodos();
  const todos = query.data ?? [];
  const undone = todos.filter((td) => !td.done).slice(0, 2);
  return (
    <div className="flex h-full flex-col justify-center gap-1">
      <div className="text-[11px] font-medium text-white/90">
        {t('home.todo.summary', { n: todos.filter((td) => !td.done).length })}
      </div>
      {undone.length === 0 ? (
        <div className="text-[10.5px] text-white/55">{t('home.todo.allDone')}</div>
      ) : (
        undone.map((td) => (
          <div key={td.id} className="truncate text-[10.5px] text-white/75">
            ◦ {td.content}
          </div>
        ))
      )}
    </div>
  );
}

function CountdownWidget() {
  const { t } = useI18n();
  const { data: items = [] } = useCountdown();
  return (
    <div className="flex h-full flex-col justify-center gap-1">
      {items.length === 0 ? (
        <Empty text={t('home.countdown.empty')} />
      ) : (
        items.map((it) => (
          <div key={it.title} className="flex items-center justify-between gap-2">
            <span className="min-w-0 truncate text-[11px] text-white/85">
              {it.emoji} {it.title}
            </span>
            <span className="shrink-0 rounded-full bg-sky-500/30 px-1.5 text-[10px] text-sky-200 tabular-nums">
              {it.days === 0
                ? t('home.countdown.today')
                : it.days === 1
                  ? t('home.countdown.days.one', { n: it.days })
                  : t('home.countdown.days.other', { n: it.days })}
            </span>
          </div>
        ))
      )}
    </div>
  );
}

function Empty({ text }: { text: string }) {
  return <div className="grid h-full place-items-center text-[11px] text-white/50">{text}</div>;
}

/** 迷你进度条（电脑状态小组件用） */
function MiniBar({ label, value, percent }: { label: string; value: string; percent: number }) {
  const { t } = useI18n();
  return (
    <div title={t('home.sysinfo.barTitle', { label, value, percent })}>
      <div className="flex items-baseline justify-between gap-2 text-[10px] leading-none text-white/75">
        <span className="truncate">{label}</span>
        <span className="shrink-0 tabular-nums">{percent}%</span>
      </div>
      <div className="mt-1 h-1 rounded-full bg-white/15">
        <div
          className="h-full rounded-full bg-[var(--accent)]"
          style={{ width: `${Math.min(percent, 100)}%` }}
        />
      </div>
    </div>
  );
}

/** 电脑状态：内存 / 磁盘分区 / CPU / 温度（传感器可用时） */
function SysinfoWidget() {
  const { t } = useI18n();
  const { data, isError } = useSystemStats();
  if (isError || !data) return <Empty text={t('home.sysinfo.unavailable')} />;
  const cpuTemp = data.temps.find((t) => t.label.toLowerCase().includes('cpu')) ?? data.temps[0];
  return (
    <div
      className="flex h-full flex-col justify-center gap-1.5"
      title={data.disks.map((d) => `${d.mount} ${d.used_gb}/${d.total_gb}GB`).join('\n')}
    >
      <MiniBar
        label={t('home.sysinfo.memory', { used: data.mem.used_gb, total: data.mem.total_gb })}
        percent={data.mem.percent}
        value=""
      />
      {data.disks.slice(0, 2).map((d) => (
        <MiniBar
          key={d.mount}
          label={`${d.mount} ${d.used_gb}/${d.total_gb}GB`}
          percent={d.percent}
          value=""
        />
      ))}
      <div className="flex items-center justify-between gap-2 text-[10px] leading-none text-white/75">
        <span className="truncate">
          CPU {data.cpu.percent}%{cpuTemp ? ` · ${Math.round(cpuTemp.celsius)}°C` : ''}
        </span>
        <span className="shrink-0 text-white/45">
          {data.cpu.core_count === 1
            ? t('home.sysinfo.cores.one', { n: data.cpu.core_count })
            : t('home.sysinfo.cores.other', { n: data.cpu.core_count })}
        </span>
      </div>
    </div>
  );
}

/** 任务管理器：内存占用 Top 进程 + 悬停结束进程 */
function TaskmgrWidget() {
  const { t } = useI18n();
  const { data: procs = [], isLoading } = useProcessList(5);
  const kill = useKillProcess();
  if (isLoading) return <Empty text={t('home.taskmgr.loading')} />;
  return (
    <div className="flex h-full flex-col justify-center gap-0.5">
      <div className="mb-0.5 flex items-center justify-between text-[10px] text-white/55">
        <span>{t('home.taskmgr.title')}</span>
        <span>{t('home.taskmgr.hoverHint')}</span>
      </div>
      {procs.length === 0 && <Empty text={t('home.taskmgr.noProcesses')} />}
      {procs.slice(0, 4).map((p) => (
        <div key={p.pid} className="group flex items-center gap-1.5">
          <span className="min-w-0 flex-1 truncate text-[10.5px] text-white/80">{p.name}</span>
          <span className="shrink-0 text-[10px] text-white/55 tabular-nums">{p.mem_mb}MB</span>
          <button
            onClick={() => kill.mutate(p.pid)}
            title={t('home.taskmgr.endProcess', { name: p.name, pid: p.pid })}
            className="shrink-0 text-white/40 opacity-0 transition-opacity hover:text-red-400 group-hover:opacity-100"
          >
            ✕
          </button>
        </div>
      ))}
      {kill.isError && <div className="text-[9.5px] text-red-300">{t('home.taskmgr.endFailed')}</div>}
    </div>
  );
}

/** Dock 式大图标（常用应用卡也用）——占位避免未使用告警 */
export function iconSrcOf(path: string) {
  return path.startsWith('data:') ? path : convertFileSrc(path);
}
