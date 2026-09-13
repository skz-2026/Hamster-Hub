import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Search, Check, SquarePlus, SlidersHorizontal, Sparkles, FolderOpen, Settings, CalendarDays, LayoutGrid, Bot } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { useClock, greetingOf, useDateTimeInfo } from '@/features/workbench/hooks';
import { useWallpaper } from '@/features/home/hooks';
import { TASKBAR_H_PX } from '@/features/home/layout';
import { WallpaperLayer, wallpaperSrcOf } from '@/features/home/WallpaperLayer';
import { useDisabledPlugins, usePluginList } from '@/features/plugins/registry';
import { useI18n } from '@/shared/i18n/provider';
import type { TKey } from '@/shared/i18n/core';
import { commands, events } from '@/shared/lib/ipc';
import { HomeSearchBox } from '@/features/search/HomeSearchBox';
import { ControlCenter } from '@/features/control/ControlCenter';
import { useDashboardLayout } from '@/features/dashboard/hooks';
import { DashboardTile } from '@/features/dashboard/DashboardTile';
import {
  addTile,
  cycleSpan,
  moveTile,
  removeTile,
  BUILTIN_TILES,
  type BuiltinTileType,
  type TileType,
} from '@/features/dashboard/layout';

const LONG_PRESS_MS = 450;

/**
 * 首页 = 工作台 × 桌面主页（合一）。**一份 tiles 布局，两态共用**（kv 持久化）：
 * - 窗口化：圆角壁纸画布 + 问候时钟 Hero + 搜索胶囊 + 可编排玻璃卡网格
 * - 桌面接管：全屏沉浸 + 大时钟 Hero + Spotlight 搜索 + 同一网格 + 快捷入口 + 控制中心
 * 编辑模式（长按任意卡 / 右上角「编辑」）：拖拽重排、✕ 移除、⤢ 调宽（1x/2x）、+ 添加（含插件）。
 */
export default function WorkbenchPage() {
  const [takeover, setTakeover] = useState(false);
  const dash = useDashboardLayout();
  const [edit, setEdit] = useState(false);

  // 模式跟随：初始对齐 + 事件跟手（同 HomeScreen 的做法，页面自持模式态）
  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;
    commands
      .desktopModeIsActive()
      .then((active) => {
        if (alive) setTakeover(active);
      })
      .catch(() => {});
    events.desktopModeChanged
      .listen((e) => {
        if (alive) setTakeover(e.payload.active);
      })
      .then((fn) => (alive ? (unlisten = fn) : fn()));
    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  return takeover ? (
    <DesktopHome dash={dash} edit={edit} setEdit={setEdit} />
  ) : (
    <WindowHome dash={dash} edit={edit} setEdit={setEdit} />
  );
}

type Dash = ReturnType<typeof useDashboardLayout>;

/* ===== 可编排网格（两态共用） ===== */

interface DragState {
  idx: number;
  x: number;
  y: number;
}

function DashboardGrid({ dash, edit, setEdit }: { dash: Dash; edit: boolean; setEdit: (v: boolean) => void }) {
  const { t } = useI18n();
  const { layout, commit } = dash;
  const tiles = layout.tiles;
  const [drag, setDrag] = useState<DragState | null>(null);
  const dragRef = useRef<DragState | null>(null);
  const cellEls = useRef(new Map<number, HTMLElement>());
  const longPressTimer = useRef<number | undefined>(undefined);
  const setEditRef = useRef(setEdit);
  setEditRef.current = setEdit;
  // move/up 常驻监听读最新状态用（commit 每渲染新建，不能进 effect 依赖）
  const commitRef = useRef(commit);
  commitRef.current = commit;
  const tilesRef = useRef(tiles);
  tilesRef.current = tiles;

  const hitTest = (x: number, y: number): number | null => {
    for (const [idx, el] of cellEls.current) {
      const r = el.getBoundingClientRect();
      if (x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) return idx;
    }
    return null;
  };

  // pointermove/up 常驻（无拖拽时 no-op）：不能等 setDrag 后的 effect 再挂——
  // 快速 down→move→up 序列里 passive effect 晚于事件到达，监听会错过 up 丢提交
  useEffect(() => {
    const onMove = (e: PointerEvent) => {
      const d = dragRef.current;
      if (!d) return;
      const next = { idx: d.idx, x: e.clientX, y: e.clientY };
      dragRef.current = next;
      setDrag(next);
    };
    const onUp = (e: PointerEvent) => {
      const d = dragRef.current;
      dragRef.current = null;
      setDrag(null);
      if (!d) return;
      const target = hitTest(e.clientX, e.clientY);
      if (target == null || target === d.idx) return;
      commitRef.current(moveTile(tilesRef.current, d.idx, target));
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    return () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /** 卡片按下：编辑态直接开拖；普通态长按 450ms 进编辑并开拖（移动超阈值取消） */
  const tilePointerDown = (idx: number) => (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    const start = { idx, x: e.clientX, y: e.clientY };
    if (edit) {
      dragRef.current = start;
      setDrag(start);
      return;
    }
    window.clearTimeout(longPressTimer.current);
    const onMove = (ev: PointerEvent) => {
      if (Math.hypot(ev.clientX - start.x, ev.clientY - start.y) > 10) cancel();
    };
    const cancel = () => {
      window.clearTimeout(longPressTimer.current);
      window.removeEventListener('pointermove', onMove);
    };
    longPressTimer.current = window.setTimeout(() => {
      window.removeEventListener('pointermove', onMove);
      setEditRef.current(true);
      dragRef.current = start;
      setDrag(start);
    }, LONG_PRESS_MS);
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', cancel, { once: true });
  };

  if (tiles.length === 0) {
    return (
      <div className="grid place-items-center rounded-[20px] bg-black/25 py-14 ring-1 ring-white/10 backdrop-blur-2xl">
        <p className="text-[13px] text-white/55">{t('pages.workbench.emptyTitle')}</p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-12 gap-4">
      {tiles.map((tile, idx) => (
        <DashboardTile
          key={tile.type}
          type={tile.type}
          span={tile.span}
          edit={edit}
          onRemove={() => commit(removeTile(tiles, idx))}
          onCycleSpan={() => commit(cycleSpan(tiles, idx))}
          onPointerDown={tilePointerDown(idx)}
          cellRef={(el) => {
            if (el) cellEls.current.set(idx, el);
            else cellEls.current.delete(idx);
          }}
        />
      ))}
      {drag && <TileDragGhost drag={drag} tiles={tiles} />}
    </div>
  );
}

function TileDragGhost({ drag, tiles }: { drag: DragState; tiles: { type: TileType }[] }) {
  const { t } = useI18n();
  const type = tiles[drag.idx]?.type;
  if (!type) return null;
  return (
    <div
      className="pointer-events-none fixed z-50 flex h-16 w-44 items-center justify-center rounded-[20px] bg-white/20 text-[13px] font-medium text-white ring-1 ring-white/25 shadow-2xl backdrop-blur-2xl"
      style={{ left: drag.x - 88, top: drag.y - 32 }}
    >
      {tileLabelOf(t, type)}
    </div>
  );
}

/** tile 类型 → i18n 标签（显式映射，禁止拼 key） */
const TILE_LABEL: Record<BuiltinTileType, TKey> = {
  weather: 'pages.workbench.weather',
  todo: 'home.todo.title',
  countdown: 'pages.workbench.countdown',
  recentFiles: 'pages.workbench.recentFiles',
  topApps: 'pages.workbench.topApps',
  focus: 'pages.workbench.focus',
  sysinfo: 'home.widget.sysinfo',
  taskmgr: 'home.widget.taskmgr',
};

function tileLabelOf(t: ReturnType<typeof useI18n>['t'], type: TileType): string {
  if (type.startsWith('plugin:')) return '🧩 ' + type.slice('plugin:'.length);
  return t(TILE_LABEL[type as BuiltinTileType]);
}

/** 编辑态控制钮（编辑/完成 + 添加菜单），两态共用 */
function EditControls({ dash, edit, setEdit }: { dash: Dash; edit: boolean; setEdit: (v: boolean) => void }) {
  const { t } = useI18n();
  const [addMenu, setAddMenu] = useState(false);
  const plugins = usePluginList();
  const disabled = useDisabledPlugins();
  const present = new Set(dash.layout.tiles.map((x) => x.type));
  const addableBuiltin = BUILTIN_TILES.filter((x) => !present.has(x));
  const addablePlugins = (plugins.data ?? []).filter(
    (p) => !disabled.data.includes(p.id) && !present.has(`plugin:${p.id}`),
  );
  const nothingToAdd = addableBuiltin.length === 0 && addablePlugins.length === 0;

  return (
    <div className="relative flex items-center gap-1.5">
      {edit && (
        <div className="relative">
          <GlassBtn title={t('pages.workbench.addTile')} onClick={() => setAddMenu((v) => !v)}>
            <SquarePlus size={15} />
          </GlassBtn>
          {addMenu && (
            <div className="absolute right-0 top-9 z-50 w-36 rounded-xl bg-neutral-900/90 p-1 ring-1 ring-white/15 backdrop-blur-xl">
              {nothingToAdd && (
                <p className="px-3 py-2 text-[11px] text-white/45">{t('pages.workbench.allAdded')}</p>
              )}
              {addableBuiltin.map((x) => (
                <button
                  key={x}
                  onClick={() => {
                    dash.commit(addTile(dash.layout.tiles, x));
                    setAddMenu(false);
                  }}
                  className="block w-full rounded-lg px-3 py-1.5 text-left text-xs text-white/90 transition-colors hover:bg-white/10"
                >
                  {t(TILE_LABEL[x])}
                </button>
              ))}
              {addablePlugins.length > 0 && <div className="my-1 border-t border-white/10" />}
              {addablePlugins.map((p) => (
                <button
                  key={p.id}
                  title={p.description}
                  onClick={() => {
                    dash.commit(addTile(dash.layout.tiles, `plugin:${p.id}` as TileType));
                    setAddMenu(false);
                  }}
                  className="block w-full rounded-lg px-3 py-1.5 text-left text-xs text-white/90 transition-colors hover:bg-white/10"
                >
                  🧩 {p.name}
                </button>
              ))}
            </div>
          )}
        </div>
      )}
      <GlassBtn
        title={edit ? t('pages.workbench.done') : t('pages.workbench.editHint')}
        onClick={() => {
          setAddMenu(false);
          setEdit(!edit);
        }}
      >
        {edit ? <Check size={15} /> : <span className="text-[11px]">{t('pages.workbench.edit')}</span>}
      </GlassBtn>
    </div>
  );
}

/** 玻璃圆钮（主屏 IconBtn 同语言） */
function GlassBtn({
  title,
  onClick,
  children,
}: {
  title: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      title={title}
      onClick={onClick}
      className="grid h-8 min-w-8 place-items-center rounded-full bg-white/18 px-2 text-white/90 ring-1 ring-white/15 backdrop-blur-md transition-colors hover:bg-white/28 ios-ease"
    >
      {children}
    </button>
  );
}

/* ===== 窗口化形态 ===== */

function WindowHome({ dash, edit, setEdit }: { dash: Dash; edit: boolean; setEdit: (v: boolean) => void }) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const wallpaper = useWallpaper();
  const now = useClock();
  const { time, date, lunar } = useDateTimeInfo(now);

  return (
    <div
      className="relative min-h-full overflow-hidden rounded-2xl p-6 text-white"
      style={{ background: wallpaper.css }}
    >
      <WallpaperLayer wallpaper={wallpaper} />
      {/* 渐变壁纸的轻纱罩层（图片壁纸已在 WallpaperLayer 内统一压暗，勿叠加） */}
      {!wallpaperSrcOf(wallpaper) && <div className="pointer-events-none absolute inset-0 bg-black/25" />}

      <div className="relative">
        {/* Hero：问候 + 时钟 + 搜索（接管态大 Hero 的窗口化收拢版） */}
        <div className="mb-5 flex flex-wrap items-end justify-between gap-4">
          <div>
            <h1 className="text-lg font-semibold text-white/95 [text-shadow:0_1px_6px_rgba(0,0,0,.35)]">
              {greetingOf(now)} <span className="opacity-80">👋</span>
            </h1>
            <div className="mt-1.5 flex flex-wrap items-baseline gap-x-3 gap-y-1">
              <span className="text-[44px] font-extralight leading-none tracking-[-0.02em] text-white tabular-nums [text-shadow:0_2px_14px_rgba(0,0,0,.4)]">
                {time.slice(0, 5)}
              </span>
              <span className="text-[13px] text-white/80 [text-shadow:0_1px_5px_rgba(0,0,0,.35)]">
                {date} · {lunar}
              </span>
            </div>
            <div className="mt-2 flex items-center gap-1.5 text-[11px] text-white/60">
              <Sparkles size={12} className="text-[var(--accent)]" />
              {t('pages.workbench.motd')}
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={() => navigate('/search')}
              className="flex h-11 items-center gap-2.5 rounded-full bg-white/15 px-5 ring-1 ring-white/20 backdrop-blur-xl transition-colors hover:bg-white/25"
            >
              <Search size={16} className="text-white/65" />
              <span className="text-[13px] text-white/60">{t('chrome.desktop.searchPlaceholder')}</span>
            </button>
            <EditControls dash={dash} edit={edit} setEdit={setEdit} />
          </div>
        </div>

        <DashboardGrid dash={dash} edit={edit} setEdit={setEdit} />
      </div>
    </div>
  );
}

/* ===== 桌面接管形态 ===== */

/** 快捷入口（桌面模式无侧栏，页面导航收进这里；首页自身不自引用） */
const QUICK_LINKS: { labelKey: TKey; to: string; Icon: LucideIcon }[] = [
  { labelKey: 'chrome.nav.agent', to: '/bench', Icon: Bot },
  { labelKey: 'chrome.nav.schedule', to: '/schedule', Icon: CalendarDays },
  { labelKey: 'chrome.nav.apps', to: '/apps', Icon: LayoutGrid },
  { labelKey: 'chrome.nav.files', to: '/files', Icon: FolderOpen },
  { labelKey: 'chrome.nav.settings', to: '/settings', Icon: Settings },
];

/**
 * 桌面接管形态（macOS 桌面质感）：全屏壁纸 + 居中大时钟 Hero + Spotlight 搜索 +
 * 快捷入口 + 与窗口化同一份 tiles 网格。内容超一屏时纵向滚动。
 */
function DesktopHome({ dash, edit, setEdit }: { dash: Dash; edit: boolean; setEdit: (v: boolean) => void }) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const now = useClock();
  const { time, date, lunar } = useDateTimeInfo(now);
  const wallpaper = useWallpaper();
  const [ccOpen, setCcOpen] = useState(false);

  return (
    <div
      className="relative h-full w-full overflow-hidden text-white"
      style={{ background: wallpaper.css }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {/* 壁纸/柔光挂在滚动容器外：absolute 元素在滚动容器里会随内容上移，
          内容超一屏时壁纸图片底边被滚上来，露出外层 css 渐变兜底形成接缝 */}
      <WallpaperLayer wallpaper={wallpaper} />
      {/* 柔光晕（模拟 macOS 壁纸景深）；只给渐变壁纸，浅色图片壁纸上叠白会更看不清 */}
      {!wallpaperSrcOf(wallpaper) && (
        <div className="pointer-events-none absolute inset-x-0 top-0 h-64 bg-gradient-to-b from-white/[0.05] to-transparent" />
      )}

      {/* 布局容器止于任务栏上沿（底部 68px 由置顶 taskbar 窗覆盖） */}
      <div
        className="relative flex w-full flex-col items-center overflow-y-auto px-8 pb-10 pt-10"
        style={{ height: `calc(100% - ${TASKBAR_H_PX}px)` }}
      >
        {/* 右上角热区：编辑入口 + 控制中心 */}
        <div className="absolute right-5 top-5 z-40 flex items-center gap-2">
          <EditControls dash={dash} edit={edit} setEdit={setEdit} />
          <button
            onClick={() => setCcOpen((v) => !v)}
            title={t('chrome.desktop.controlCenter')}
            aria-label={t('chrome.desktop.controlCenter')}
            className="grid size-10 place-items-center rounded-full bg-white/10 text-white/85 ring-1 ring-white/15 backdrop-blur-xl transition-all hover:bg-white/20 hover:text-white"
          >
            <SlidersHorizontal size={17} />
          </button>
        </div>
        <ControlCenter open={ccOpen} onClose={() => setCcOpen(false)} />

        {/* 问候 + 大时钟（SF 风：细体、收紧字距；错落入场） */}
        <div className="rise-in relative flex flex-col items-center gap-2">
          <span className="text-5xl leading-none drop-shadow-[0_4px_16px_rgba(0,0,0,.4)]">🐹</span>
          <h1 className="text-[22px] font-medium tracking-wide text-white/95 [text-shadow:0_2px_16px_rgba(0,0,0,.45)]">
            {t('chrome.desktop.welcomeBack', { greeting: greetingOf(now) })}
          </h1>
          <div className="text-[96px] font-extralight leading-none tracking-[-0.02em] tabular-nums [text-shadow:0_6px_32px_rgba(0,0,0,.45)]">
            {time.slice(0, 5)}
          </div>
          <div className="text-[13.5px] font-normal text-white/75 [text-shadow:0_1px_8px_rgba(0,0,0,.4)]">
            {date} · {lunar}
          </div>
        </div>

        {/* Spotlight 式大搜索框：内联出结果下拉，不再跳转 /search */}
        <HomeSearchBox
          className="rise-in mt-8 w-[min(600px,68vw)]"
          style={{ animationDelay: '80ms' }}
        />

        {/* 快捷入口（AI 助手优先 + 页面导航） */}
        <div className="rise-in mt-6 flex flex-wrap items-center justify-center gap-2.5" style={{ animationDelay: '160ms' }}>
          <button
            onClick={() => navigate('/agent')}
            className="flex items-center gap-2 rounded-full bg-[var(--accent)]/85 px-[18px] py-[7px] text-[12.5px] font-semibold text-white shadow-[0_4px_18px_rgba(240,112,15,.35)] transition-all hover:brightness-110"
          >
            <Sparkles size={14} />
            {t('chrome.desktop.aiAssistant')}
          </button>
          {QUICK_LINKS.map(({ labelKey, to, Icon }) => (
            <button
              key={to}
              onClick={() => navigate(to)}
              className="flex items-center gap-1.5 rounded-full bg-white/10 px-4 py-[7px] text-[12.5px] font-medium text-white/85 ring-1 ring-white/12 backdrop-blur-xl transition-all hover:bg-white/18 hover:ring-white/22"
            >
              <Icon size={14} />
              {t(labelKey)}
            </button>
          ))}
        </div>

        {/* 与窗口化同一份可编排网格 */}
        <div className="rise-in relative mt-8 w-full max-w-[1160px]" style={{ animationDelay: '240ms' }}>
          <DashboardGrid dash={dash} edit={edit} setEdit={setEdit} />
        </div>
      </div>
    </div>
  );
}
