import { CloudSun, CalendarDays, LayoutGrid, FileClock, MapPin, Loader2, TrendingUp, Activity, Gauge } from 'lucide-react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useI18n } from '@/shared/i18n/provider';
import { commands } from '@/shared/lib/ipc';
import { useWeather, useCountdown, useRecentFiles, useTopApps, useSystemStats, useProcessList, useKillProcess } from './hooks';
import { pluginIdOfTile, type TileSpan, type TileType } from './layout';
import { AppIcon, Monogram } from '@/features/home/AppIcon';
import TodoCard from '@/features/todo/TodoCard';
import FocusCard from '@/features/focus/FocusCard';
import PluginWidgetHost from '@/features/plugins/PluginWidgetHost';
import { FileKindIcon } from '@/features/spotlight/FileKindIcon';

/** 首页统一卡片：壳（玻璃 + 栅格占位）+ 按 type 分发内容。
 *  此前「工作台大卡 / HomeWidget 小卡 / 桌面组件行」三套渲染在此收敛为一处。
 *  编辑态交互（抖动 / 移除 / 调宽 / 拖拽起点）由 DashboardGrid 注入。 */
export function DashboardTile({
  type,
  span,
  edit,
  onRemove,
  onCycleSpan,
  onPointerDown,
  cellRef,
}: {
  type: TileType;
  span: TileSpan;
  edit: boolean;
  onRemove?: () => void;
  onCycleSpan?: () => void;
  onPointerDown?: (e: React.PointerEvent) => void;
  cellRef?: (el: HTMLElement | null) => void;
}) {
  const { t } = useI18n();
  return (
    <section
      ref={cellRef}
      onPointerDown={onPointerDown}
      className={`relative col-span-12 flex flex-col rounded-[20px] bg-black/25 ring-1 ring-white/10 backdrop-blur-2xl transition-colors ${
        span === 2 ? 'md:col-span-8 min-h-[190px]' : 'md:col-span-4 min-h-[190px]'
      }`}
    >
      <div className={`h-full p-5 ${edit ? 'jiggling pointer-events-none' : ''}`}>
        <TileContent type={type} span={span} />
      </div>
      {edit && (
        <>
          <button
            onPointerDown={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.stopPropagation();
              onRemove?.();
            }}
            title={t('pages.workbench.removeTile')}
            className="absolute -left-1.5 -top-1.5 z-30 grid size-5 place-items-center rounded-full bg-neutral-900/85 text-[11px] text-white"
          >
            ✕
          </button>
          <button
            onPointerDown={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.stopPropagation();
              onCycleSpan?.();
            }}
            title={t('pages.workbench.spanToggle')}
            className="absolute -right-1.5 -top-1.5 z-30 grid size-5 place-items-center rounded-full bg-neutral-900/85 text-[11px] text-white"
          >
            ⤢
          </button>
        </>
      )}
    </section>
  );
}

/** 卡片标题行（多数卡共用：accent 图标 + 标题 + 右侧副信息） */
function TileHeader({ icon: Icon, title, right }: { icon: typeof CloudSun; title: string; right?: React.ReactNode }) {
  return (
    <header className="mb-3 flex items-center justify-between text-sm font-medium text-white/90">
      <span className="flex items-center gap-2">
        <Icon size={16} className="text-[var(--accent)]" />
        {title}
      </span>
      {right}
    </header>
  );
}

function TileContent({ type, span }: { type: TileType; span: TileSpan }) {
  switch (type) {
    case 'weather':
      return <WeatherContent />;
    case 'todo':
      return <TodoCard />;
    case 'countdown':
      return <CountdownContent />;
    case 'recentFiles':
      return <RecentFilesContent />;
    case 'topApps':
      return <TopAppsContent span={span} />;
    case 'focus':
      return <FocusCard />;
    case 'sysinfo':
      return <SysinfoContent />;
    case 'taskmgr':
      return <TaskmgrContent />;
    default: {
      const pid = pluginIdOfTile(type);
      return pid ? (
        <div className="flex h-full min-h-[120px] flex-col">
          <PluginWidgetHost pluginId={pid} />
        </div>
      ) : null;
    }
  }
}

function WeatherContent() {
  const { t } = useI18n();
  const { data: w, isLoading, isError } = useWeather();
  return (
    <div className="flex h-full flex-col">
      <TileHeader
        icon={CloudSun}
        title={t('pages.workbench.weather')}
        right={
          w ? (
            <span className="flex items-center gap-1 text-[11px] text-white/50">
              <MapPin size={11} />
              {w.city}
            </span>
          ) : undefined
        }
      />
      <div className="flex flex-1 flex-col justify-center">
        {isLoading ? (
          <Loader2 className="mx-auto animate-spin text-white/40" size={22} />
        ) : isError || !w ? (
          <p className="text-center text-xs text-white/45">
            {t('pages.workbench.weatherError')}
            <br />
            <span className="text-[10px]">{t('pages.workbench.weatherRetry')}</span>
          </p>
        ) : (
          <>
            <div className="flex items-end gap-2">
              <span className="text-5xl font-light text-white tabular-nums">{w.temp}°</span>
              <span className="pb-1.5 text-2xl">{w.emoji}</span>
            </div>
            <div className="mt-2 text-[13px] text-white/80">{w.kind}</div>
            <div className="mt-1 text-[11px] text-white/50 tabular-nums">
              {t('pages.workbench.tempRange', { min: w.temp_min, max: w.temp_max })}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function CountdownContent() {
  const { t } = useI18n();
  const { data: items = [] } = useCountdown();
  return (
    <div className="flex h-full flex-col">
      <TileHeader icon={CalendarDays} title={t('pages.workbench.countdown')} />
      <div className="flex-1 space-y-2.5">
        {items.length === 0 && (
          <p className="pt-4 text-center text-xs text-white/45">{t('pages.workbench.countdownEmpty')}</p>
        )}
        {items.map((it) => (
          <div key={it.title} className="flex items-center justify-between">
            <span className="text-[13px] text-white/85">
              {it.emoji} {it.title}
            </span>
            <span className="rounded-full bg-sky-500/20 px-2.5 py-0.5 text-[11px] font-medium text-sky-300 tabular-nums">
              {it.days === 0
                ? t('pages.workbench.countdownToday')
                : t('pages.workbench.countdownDays', { days: it.days })}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

function RecentFilesContent() {
  const { t } = useI18n();
  const { data: files = [], isLoading } = useRecentFiles(6);
  return (
    <div className="flex h-full flex-col">
      <TileHeader
        icon={FileClock}
        title={t('pages.workbench.recentFiles')}
        right={
          <span className="text-[11px] text-white/45">
            {t('pages.workbench.fileCount', { count: files.length })}
          </span>
        }
      />
      <div className="-mx-1 flex-1 space-y-0.5 overflow-y-auto">
        {isLoading ? (
          <Loader2 className="mx-auto mt-6 animate-spin text-white/40" size={18} />
        ) : files.length === 0 ? (
          <p className="pt-4 text-center text-xs text-white/45">{t('pages.workbench.recentFilesEmpty')}</p>
        ) : (
          files.map((f) => (
            <button
              key={f.path}
              onClick={() => commands.openPath(f.path).catch(console.error)}
              title={f.path}
              className="flex w-full items-center gap-2.5 rounded-lg px-2 py-1.5 text-left transition-colors hover:bg-white/8"
            >
              <FileKindIcon kind={f.kind} />
              <span className="min-w-0 flex-1 truncate text-[12.5px] text-white/85">{f.name}</span>
              <span className="shrink-0 text-[10px] uppercase text-white/35">{f.ext}</span>
            </button>
          ))
        )}
      </div>
    </div>
  );
}

function TopAppsContent({ span }: { span: TileSpan }) {
  const { t } = useI18n();
  const { data: apps = [], isLoading } = useTopApps(span === 2 ? 12 : 8);
  return (
    <div className="flex h-full flex-col">
      <TileHeader
        icon={LayoutGrid}
        title={t('pages.workbench.topApps')}
        right={
          <span className="flex items-center gap-1 text-[11px] text-white/45">
            <TrendingUp size={11} />
            {t('pages.workbench.byFrequency')}
          </span>
        }
      />
      {isLoading ? (
        <Loader2 className="mx-auto mt-6 animate-spin text-white/40" size={18} />
      ) : apps.length === 0 ? (
        <p className="pt-4 text-center text-xs text-white/45">
          {t('pages.workbench.topAppsEmptyHint1')}
          <br />
          {t('pages.workbench.topAppsEmptyHint2')}
        </p>
      ) : (
        <div
          className={`grid flex-1 content-start justify-items-center gap-y-2 overflow-y-auto ${
            span === 2 ? 'grid-cols-6' : 'grid-cols-4'
          }`}
        >
          {apps.map((a) => (
            <button
              key={a.app_key}
              onClick={() => commands.appLaunch(a.app_key).catch(console.error)}
              className="flex w-[72px] flex-col items-center gap-1 rounded-lg p-1.5 transition-colors hover:bg-white/8"
              title={a.display_name}
            >
              {a.icon_path ? (
                <img
                  src={a.icon_path.startsWith('data:') ? a.icon_path : convertFileSrc(a.icon_path)}
                  alt=""
                  className="squircle size-10 object-cover"
                  draggable={false}
                />
              ) : (
                <Monogram name={a.display_name} size={40} />
              )}
              <span className="w-full truncate text-center text-[10.5px] text-white/70">
                {a.display_name}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

/** 迷你进度条（电脑状态卡用，自 HomeWidget 迁入） */
function MiniBar({ label, percent }: { label: string; percent: number }) {
  const { t } = useI18n();
  return (
    <div title={t('home.sysinfo.barTitle', { label, value: '', percent })}>
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

function SysinfoContent() {
  const { t } = useI18n();
  const { data, isError } = useSystemStats();
  if (isError || !data) {
    return (
      <div className="flex h-full flex-col">
        <TileHeader icon={Gauge} title={t('home.widget.sysinfo')} />
        <div className="grid flex-1 place-items-center text-[11px] text-white/50">
          {t('home.sysinfo.unavailable')}
        </div>
      </div>
    );
  }
  const cpuTemp = data.temps.find((x) => x.label.toLowerCase().includes('cpu')) ?? data.temps[0];
  return (
    <div className="flex h-full flex-col">
      <TileHeader icon={Gauge} title={t('home.widget.sysinfo')} />
      <div
        className="flex flex-1 flex-col justify-center gap-1.5"
        title={data.disks.map((d) => `${d.mount} ${d.used_gb}/${d.total_gb}GB`).join('\n')}
      >
        <MiniBar
          label={t('home.sysinfo.memory', { used: data.mem.used_gb, total: data.mem.total_gb })}
          percent={data.mem.percent}
        />
        {data.disks.slice(0, 2).map((d) => (
          <MiniBar key={d.mount} label={`${d.mount} ${d.used_gb}/${d.total_gb}GB`} percent={d.percent} />
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
    </div>
  );
}

function TaskmgrContent() {
  const { t } = useI18n();
  const { data: procs = [], isLoading } = useProcessList(5);
  const kill = useKillProcess();
  return (
    <div className="flex h-full flex-col">
      <TileHeader
        icon={Activity}
        title={t('home.widget.taskmgr')}
        right={<span className="text-[10px] text-white/45">{t('home.taskmgr.hoverHint')}</span>}
      />
      <div className="flex flex-1 flex-col justify-center gap-0.5">
        {isLoading ? (
          <div className="grid place-items-center py-4">
            <Loader2 size={16} className="animate-spin text-white/45" />
          </div>
        ) : (
          <>
            {procs.length === 0 && (
              <div className="grid place-items-center text-[11px] text-white/50">
                {t('home.taskmgr.noProcesses')}
              </div>
            )}
            {procs.slice(0, 5).map((p) => (
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
          </>
        )}
      </div>
    </div>
  );
}
