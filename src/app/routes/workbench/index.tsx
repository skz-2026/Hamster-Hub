import { useEffect, useState } from 'react';
import {
  CloudSun,
  CalendarDays,
  LayoutGrid,
  FileClock,
  Sparkles,
  MapPin,
  Loader2,
  TrendingUp,
} from 'lucide-react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useClock, greetingOf } from '@/features/workbench/hooks';
import { useWeather, useCountdown, useRecentFiles, useTopApps } from '@/features/dashboard/hooks';
import { useDateTimeInfo } from '@/features/workbench/hooks';
import { AppIcon, Monogram } from '@/features/home/AppIcon';
import { useI18n } from '@/shared/i18n/provider';
import { commands } from '@/shared/lib/ipc';
import TodoCard from '@/features/todo/TodoCard';
import ReminderToast from '@/features/todo/ReminderToast';
import FocusCard from '@/features/focus/FocusCard';
import { FileKindIcon } from '@/features/spotlight/FileKindIcon';

/** 工作台：毛玻璃仪表盘（对标水豚hub 工作台，数据全部真实） */
export default function WorkbenchPage() {
  const now = useClock();
  const { lunar } = useDateTimeInfo(now);

  return (
    <div className="relative min-h-full overflow-hidden rounded-2xl bg-[radial-gradient(120%_120%_at_15%_0%,#2a2530_0%,#17151b_55%,#1d1712_100%)] p-6">
      {/* 背景光晕 */}
      <div className="pointer-events-none absolute -left-24 -top-24 size-72 rounded-full bg-[var(--accent)] opacity-[0.07] blur-3xl" />
      <div className="pointer-events-none absolute -bottom-32 right-0 size-96 rounded-full bg-sky-400 opacity-[0.05] blur-3xl" />

      <div className="relative">
        <h1 className="mb-5 text-lg font-semibold text-white/95">
          {greetingOf(now)} <span className="opacity-80">👋</span>
        </h1>

        <div className="grid grid-cols-12 gap-4">
          {/* 第一排：时钟(5) + 天气(3) + 待办(4) */}
          <ClockCard now={now} lunar={lunar} />
          <WeatherCard />
          <TodoCard />

          {/* 第二排：倒数日(4) + 最近文件(4) + 常用应用(4) */}
          <CountdownCard />
          <RecentFilesCard />
          <TopAppsCard />

          {/* 第三排：番茄钟(4) */}
          <FocusCard />
        </div>
      </div>

      {/* 待办到点提醒（后端调度线程 → 应用内浮出） */}
      <ReminderToast />
    </div>
  );
}

function ClockCard({ now, lunar }: { now: Date; lunar: string }) {
  const { t } = useI18n();
  const { time, date } = useDateTimeInfo(now);
  return (
    <section className="card col-span-12 flex min-h-[220px] flex-col justify-center p-6 md:col-span-5">
      <div className="text-[56px] font-light leading-none tracking-tight text-white tabular-nums">
        {time}
      </div>
      <div className="mt-3 text-[13px] text-white/70">{date}</div>
      <div className="mt-1 text-[12px] text-white/45">{lunar}</div>
      <div className="mt-4 flex items-center gap-1.5 text-[11px] text-white/50">
        <Sparkles size={12} className="text-[var(--accent)]" />
        {t('pages.workbench.motd')}
      </div>
    </section>
  );
}

function WeatherCard() {
  const { t } = useI18n();
  const { data: w, isLoading, isError } = useWeather();
  return (
    <section className="card col-span-12 flex min-h-[220px] flex-col p-5 md:col-span-3">
      <header className="flex items-center justify-between text-sm font-medium text-white/90">
        <span className="flex items-center gap-2">
          <CloudSun size={16} className="text-[var(--accent)]" />
          {t('pages.workbench.weather')}
        </span>
        {w && (
          <span className="flex items-center gap-1 text-[11px] text-white/50">
            <MapPin size={11} />
            {w.city}
          </span>
        )}
      </header>
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
    </section>
  );
}

function CountdownCard() {
  const { t } = useI18n();
  const { data: items = [] } = useCountdown();
  return (
    <section className="card col-span-12 flex min-h-[190px] flex-col p-5 md:col-span-4">
      <header className="mb-3 flex items-center gap-2 text-sm font-medium text-white/90">
          <CalendarDays size={16} className="text-[var(--accent)]" />
          {t('pages.workbench.countdown')}
      </header>
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
    </section>
  );
}

function RecentFilesCard() {
  const { t } = useI18n();
  const { data: files = [], isLoading } = useRecentFiles(6);
  return (
    <section className="card col-span-12 flex min-h-[190px] flex-col p-5 md:col-span-4">
      <header className="mb-3 flex items-center justify-between text-sm font-medium text-white/90">
        <span className="flex items-center gap-2">
          <FileClock size={16} className="text-[var(--accent)]" />
          {t('pages.workbench.recentFiles')}
        </span>
        <span className="text-[11px] text-white/45">{t('pages.workbench.fileCount', { count: files.length })}</span>
      </header>
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
              <span className="min-w-0 flex-1 truncate text-[12.5px] text-white/85">
                {f.name}
              </span>
              <span className="shrink-0 text-[10px] uppercase text-white/35">{f.ext}</span>
            </button>
          ))
        )}
      </div>
    </section>
  );
}

function TopAppsCard() {
  const { t } = useI18n();
  const { data: apps = [], isLoading } = useTopApps(8);
  return (
    <section className="card col-span-12 flex min-h-[190px] flex-col p-5 md:col-span-4">
      <header className="mb-3 flex items-center justify-between text-sm font-medium text-white/90">
        <span className="flex items-center gap-2">
          <LayoutGrid size={16} className="text-[var(--accent)]" />
          {t('pages.workbench.topApps')}
        </span>
        <span className="flex items-center gap-1 text-[11px] text-white/45">
          <TrendingUp size={11} />
          {t('pages.workbench.byFrequency')}
        </span>
      </header>
      {isLoading ? (
        <Loader2 className="mx-auto mt-6 animate-spin text-white/40" size={18} />
      ) : apps.length === 0 ? (
        <p className="pt-4 text-center text-xs text-white/45">
          {t('pages.workbench.topAppsEmptyHint1')}
          <br />
          {t('pages.workbench.topAppsEmptyHint2')}
        </p>
      ) : (
        <div className="grid flex-1 grid-cols-4 content-start justify-items-center gap-y-2 overflow-y-auto">
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
    </section>
  );
}
