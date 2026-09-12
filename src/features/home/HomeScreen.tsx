/**
 * iOS 主屏幕：壁纸 + 图标网格分页 + 文件夹 + Dock + 搜索 + 长按编辑（抖动/拖拽/合并）。
 * 布局逻辑全部在 layout.ts 纯函数中，本组件只做交互粘合。
 */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Search, X, Check, Palette, Loader2, SquarePlus } from 'lucide-react';
import { commands, events, type AppEntry } from '@/shared/lib/ipc';
import { useApps, useHomeLayout } from './hooks';
import {
  AppIcon,
  FolderIcon,
  Monogram,
} from './AppIcon';
import { HomeWidget } from './HomeWidget';
import {
  addWidget,
  mergeIntoFolder,
  moveAcrossPages,
  moveToDock,
  rawAppKey,
  rawFolderId,
  rawWidgetType,
  reorder,
  removeFromDock,
  removeFromPage,
  WIDGET_LABEL,
  WIDGET_TYPES,
  WALLPAPERS,
  type HomeLayout,
  type WidgetType,
} from './layout';

const LONG_PRESS_MS = 450;
const MERGE_HOVER_MS = 550;

interface DragState {
  kind: 'page' | 'dock';
  page: number;
  idx: number;
  x: number;
  y: number;
}

export default function HomeScreen() {
  const navigate = useNavigate();
  const { data: apps = [], isLoading } = useApps();
  const { layout, commit, ready } = useHomeLayout(apps);
  const appByKey = useMemo(() => new Map(apps.map((a) => [a.app_key, a])), [apps]);

  const [edit, setEdit] = useState(false);
  const [widgetMenu, setWidgetMenu] = useState(false);
  const [openFolder, setOpenFolder] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [pageIdx, setPageIdx] = useState(0);
  const [drag, setDrag] = useState<DragState | null>(null);
  const dragRef = useRef<DragState | null>(null);
  const hoverRef = useRef<{ key: string; since: number } | null>(null);
  const [hoverKey, setHoverKey] = useState<string | null>(null);
  const [mergeFlash, setMergeFlash] = useState<string | null>(null);

  const [takeover, setTakeover] = useState(false);

  const scrollerRef = useRef<HTMLDivElement>(null);
  const cellEls = useRef(new Map<string, HTMLElement>());
  const longPressTimer = useRef<number | undefined>(undefined);
  const mergedRef = useRef(false);

  const now = useClockMinute();

  // 页面重载/热更新后与 Rust 侧状态对齐：桌面模式已关则退回工作台
  useEffect(() => {
    commands
      .desktopModeIsActive()
      .then((active) => {
        setTakeover(active);
        if (!active) window.location.hash = '#/';
      })
      .catch(() => {});
  }, []);

  // 接管模式开关跟手（进入/退出瞬间任务栏条出现/消失，底距需即时切换）
  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;
    events.desktopModeChanged
      .listen((e) => {
        if (!alive) return;
        setTakeover(e.payload.active);
        if (!e.payload.active) window.location.hash = '#/';
      })
      .then((fn) => (alive ? (unlisten = fn) : fn()));
    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  // ===== 拖拽 =====
  const hitTest = (x: number, y: number): string | null => {
    for (const [key, el] of cellEls.current) {
      const r = el.getBoundingClientRect();
      if (x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) return key;
    }
    return null;
  };

  const startDrag = (s: DragState) => {
    mergedRef.current = false;
    dragRef.current = s;
    setDrag(s);
  };

  const endDragCommit = (x: number, y: number) => {
    const d = dragRef.current;
    dragRef.current = null;
    setDrag(null);
    setHoverKey(null);
    hoverRef.current = null;
    if (!d) return;
    const target = hitTest(x, y);
    if (!target) return;
    commit(applyDrop(layout, d, target, hoverKeyRefToBool(hoverKey, target)));
  };

  const onPointerMoveDoc = useCallback((e: PointerEvent) => {
    const d = dragRef.current;
    if (!d) return;
    const next = { ...d, x: e.clientX, y: e.clientY };
    dragRef.current = next; // 同步给轮询（悬停判定用最新坐标）
    setDrag(next);
    updateHover(e.clientX, e.clientY);
    // 边缘翻页
    const sc = scrollerRef.current;
    if (sc) {
      const r = sc.getBoundingClientRect();
      if (e.clientX < r.left + 80) sc.scrollBy({ left: -26, behavior: 'auto' });
      else if (e.clientX > r.right - 80) sc.scrollBy({ left: 26, behavior: 'auto' });
    }
  }, []);

  /** 悬停目标与合并判定（拖拽期间由 move 与轮询共同驱动，指针静止也能合并） */
  const updateHover = (x: number, y: number) => {
    const d = dragRef.current;
    if (!d) return;
    const key = hitTest(x, y);
    if (key !== hoverRef.current?.key) {
      hoverRef.current = key ? { key, since: Date.now() } : null;
      setHoverKey(key);
    }
    if (
      key &&
      Date.now() - (hoverRef.current?.since ?? 0) > MERGE_HOVER_MS &&
      !mergedRef.current
    ) {
      const next = applyMerge(layout, d, key);
      if (next !== layout) {
        mergedRef.current = true;
        commit(next);
        setMergeFlash(key);
        window.setTimeout(() => setMergeFlash(null), 600);
        dragRef.current = null;
        setDrag(null);
        setHoverKey(null);
        hoverRef.current = null;
      }
    }
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const updateHoverRef = useRef(updateHover);
  updateHoverRef.current = updateHover;

  useEffect(() => {
    if (!drag) return;
    const up = (e: PointerEvent) => {
      if (!mergedRef.current) endDragCommit(e.clientX, e.clientY);
      else mergedRef.current = false;
    };
    // 指针静止时也要推进悬停计时（iOS 按住不动即可合并）
    const poll = window.setInterval(() => {
      const d = dragRef.current;
      if (d) updateHoverRef.current(d.x, d.y);
    }, 120);
    window.addEventListener('pointermove', onPointerMoveDoc);
    window.addEventListener('pointerup', up);
    return () => {
      window.clearInterval(poll);
      window.removeEventListener('pointermove', onPointerMoveDoc);
      window.removeEventListener('pointerup', up);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [drag, layout]);

  const iconPointerDown = (kind: 'page' | 'dock', page: number, idx: number) => (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    const pos = { kind, page, idx, x: e.clientX, y: e.clientY };
    if (edit) {
      startDrag(pos);
      return;
    }
    // 长按进入编辑：移动超过阈值才取消（手抖不应打断长按）
    window.clearTimeout(longPressTimer.current);
    const onMove = (ev: PointerEvent) => {
      if (Math.hypot(ev.clientX - pos.x, ev.clientY - pos.y) > 10) cancel();
    };
    const cancel = () => {
      window.clearTimeout(longPressTimer.current);
      window.removeEventListener('pointermove', onMove);
    };
    longPressTimer.current = window.setTimeout(() => {
      window.removeEventListener('pointermove', onMove);
      setEdit(true);
      startDrag(pos);
    }, LONG_PRESS_MS);
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', cancel, { once: true });
  };

  // ===== 应用操作 =====
  const launch = (key: string) => {
    commands.appLaunch(key).catch((e) => console.error('[home] 启动失败', e));
  };

  const removeSlot = (page: number, idx: number) => commit(removeFromPage(layout, page, idx));
  const removeDock = (idx: number) => commit(removeFromDock(layout, idx));

  const cycleWallpaper = () => {
    const ids = Object.keys(WALLPAPERS);
    const next = ids[(ids.indexOf(layout.wallpaper) + 1) % ids.length];
    commit({ ...layout, wallpaper: next });
  };

  // ===== 渲染 =====
  const wallpaper = WALLPAPERS[layout.wallpaper] ?? WALLPAPERS.hamster;
  const q = query.trim().toLowerCase();
  const results = q ? apps.filter((a) => a.display_name.toLowerCase().includes(q)) : [];
  const folder = openFolder ? layout.folders[openFolder] : null;

  return (
    <div
      className="relative h-full w-full overflow-hidden text-white"
      style={{ background: wallpaper.css }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {/* 顶部状态栏：时间 | 搜索 | 编辑/壁纸/退出 */}
      <header className="absolute inset-x-0 top-0 z-20 flex items-center gap-3 px-6 pt-4">
        <div className="w-32 text-left">
          <div className="text-[15px] font-semibold tabular-nums [text-shadow:0_1px_4px_rgba(0,0,0,.4)]">
            {now.time}
          </div>
          <div className="text-[10.5px] text-white/75 [text-shadow:0_1px_3px_rgba(0,0,0,.4)]">
            {now.date}
          </div>
        </div>

        <div className="relative mx-auto w-[min(380px,42vw)]">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-white/60" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="搜索应用"
            className="ios-ease h-9 w-full rounded-full bg-white/18 pl-9 pr-8 text-[13px] text-white placeholder:text-white/55 backdrop-blur-xl outline-none ring-1 ring-white/15 focus:bg-white/26"
          />
          {query && (
            <button
              onClick={() => setQuery('')}
              className="absolute right-2.5 top-1/2 -translate-y-1/2 text-white/60 hover:text-white"
            >
              <X size={14} />
            </button>
          )}
        </div>

        <div className="flex w-32 items-center justify-end gap-1.5">
          {edit && (
            <IconBtn title="切换壁纸" onClick={cycleWallpaper}>
              <Palette size={15} />
            </IconBtn>
          )}
          {edit && (
            <div className="relative">
              <IconBtn title="添加小组件" onClick={() => setWidgetMenu((v) => !v)}>
                <SquarePlus size={15} />
              </IconBtn>
              {widgetMenu && (
                <div className="absolute right-0 top-9 z-30 w-28 rounded-xl bg-neutral-900/90 p-1 ring-1 ring-white/15 backdrop-blur-xl">
                  {WIDGET_TYPES.map((t) => (
                    <button
                      key={t}
                      onClick={() => {
                        commit(addWidget(layout, t as WidgetType));
                        setWidgetMenu(false);
                      }}
                      className="block w-full rounded-lg px-3 py-1.5 text-left text-xs text-white/90 transition-colors hover:bg-white/10"
                    >
                      {WIDGET_LABEL[t]}
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}
          <IconBtn title={edit ? '完成' : '长按图标整理主屏'} onClick={() => setEdit((v) => !v)}>
            {edit ? <Check size={15} /> : <span className="text-[11px]">整理</span>}
          </IconBtn>
          {!edit && (
            <IconBtn title="返回桌面" onClick={() => navigate('/desktop')}>
              <span className="text-[11px]">桌面</span>
            </IconBtn>
          )}
          {!edit && (
            <IconBtn
              title="退出桌面模式"
              onClick={() => commands.desktopModeExit().catch(console.error)}
            >
              <span className="text-[11px]">退出</span>
            </IconBtn>
          )}
        </div>
      </header>

      {/* 分页网格 */}
      <div
        ref={scrollerRef}
        onScroll={(e) => {
          const el = e.currentTarget;
          setPageIdx(Math.round(el.scrollLeft / el.clientWidth));
        }}
        className="home-pages absolute inset-x-0 top-16 bottom-40 flex snap-x snap-mandatory overflow-x-auto overflow-y-hidden [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
      >
        {isLoading || !ready ? (
          <div className="grid w-full place-items-center gap-2 text-white/80">
            <Loader2 className="animate-spin" size={22} />
            <span className="text-[13px]">正在索引本机应用…</span>
          </div>
        ) : layout.pages.length === 1 && layout.pages[0].length === 0 ? (
          <div className="grid w-full place-items-center text-white/70">
            <div className="flex flex-col items-center gap-3">
              <Monogram name="仓" size={64} />
              <span className="text-[13px]">没有找到可显示的应用</span>
            </div>
          </div>
        ) : (
          layout.pages.map((page, pi) => (
            <div
              key={pi}
              className="home-page grid h-full w-full shrink-0 snap-center grid-cols-7 auto-rows-min content-start justify-items-center gap-y-6 px-10"
            >
              {page.map((item, idx) => {
                const key = `p:${pi}:${idx}`;
                const isDragSource = drag?.kind === 'page' && drag.page === pi && drag.idx === idx;
                const mergeHint = hoverKey === key && !isDragSource;
                if (item.startsWith('widget:')) {
                  const wt = rawWidgetType(item);
                  if (!wt) return null;
                  return (
                    <div
                      key={key}
                      ref={(el) => registerCell(el, key)}
                      className="col-span-2"
                    >
                      <div className="relative h-[104px]">
                        <div
                          className={`ios-ease h-full w-full transition-opacity ${
                            isDragSource ? 'opacity-30' : ''
                          } ${edit ? 'jiggling' : ''}`}
                        >
                          <HomeWidget type={wt} />
                        </div>
                        {edit && (
                          <button
                            onClick={() => removeSlot(pi, idx)}
                            title="移除小组件"
                            className="absolute -left-1.5 -top-1.5 grid size-5 place-items-center rounded-full bg-neutral-900/85 text-[11px] text-white"
                          >
                            ✕
                          </button>
                        )}
                      </div>
                    </div>
                  );
                }
                if (item.startsWith('folder:')) {
                  const id = rawFolderId(item);
                  const f = layout.folders[id];
                  if (!f) return null;
                  return (
                    <div key={key} ref={(el) => registerCell(el, key)}>
                      <FolderIcon
                        name={f.name}
                        names={f.apps.map((k) => appByKey.get(k)?.display_name ?? k)}
                        iconPaths={f.apps.map((k) => appByKey.get(k)?.icon_path)}
                        jiggle={edit}
                        mergeHint={mergeHint || mergeFlash === key}
                        dragging={isDragSource}
                        onRemove={edit ? () => removeSlot(pi, idx) : undefined}
                        onPointerDown={iconPointerDown('page', pi, idx)}
                        onClick={() => !edit && setOpenFolder(id)}
                      />
                    </div>
                  );
                }
                const a = appByKey.get(rawAppKey(item));
                if (!a) return null;
                return (
                  <div key={key} ref={(el) => registerCell(el, key)}>
                    <AppIcon
                      name={a.display_name}
                      iconPath={a.icon_path}
                      jiggle={edit}
                      mergeHint={mergeHint || mergeFlash === key}
                      dragging={isDragSource}
                      onRemove={edit ? () => removeSlot(pi, idx) : undefined}
                      onPointerDown={iconPointerDown('page', pi, idx)}
                      onClick={() => !edit && launch(a.app_key)}
                    />
                  </div>
                );
              })}
            </div>
          ))
        )}
      </div>

      {/* 页码圆点 */}
      {layout.pages.length > 1 && (
        <div
          className={`absolute inset-x-0 z-10 flex justify-center gap-2 ${
            takeover ? 'bottom-[178px]' : 'bottom-[118px]'
          }`}
        >
          {layout.pages.map((_, i) => (
            <span
              key={i}
              className={`size-[7px] rounded-full transition-colors ${
                i === pageIdx ? 'bg-white' : 'bg-white/35'
              }`}
            />
          ))}
        </div>
      )}

      {/* Dock */}
      {/* Dock：接管模式下整体抬到任务栏条上沿之上（条高 68 + 16 间距 = 84） */}
      <div
        className={`absolute inset-x-0 z-20 flex justify-center ${
          takeover ? 'bottom-[84px]' : 'bottom-6'
        }`}
      >
        <div
          className="ios-dock flex items-center gap-4 rounded-[28px] px-5 py-3"
          onDragOver={(e) => e.preventDefault()}
        >
          {layout.dock.map((key, idx) => {
            const a = appByKey.get(key);
            if (!a) return null;
            const cellKey = `d:0:${idx}`;
            return (
              <div key={key} ref={(el) => registerCell(el, cellKey)}>
                <AppIcon
                  name={a.display_name}
                  iconPath={a.icon_path}
                  label={false}
                  size={52}
                  jiggle={edit}
                  mergeHint={hoverKey === cellKey}
                  dragging={drag?.kind === 'dock' && drag.idx === idx}
                  onRemove={edit ? () => removeDock(idx) : undefined}
                  onPointerDown={iconPointerDown('dock', 0, idx)}
                  onClick={() => !edit && launch(key)}
                  className="origin-bottom transition-transform duration-150 ease-out hover:scale-125 hover:-translate-y-1.5"
                />
              </div>
            );
          })}
          {layout.dock.length === 0 && (
            <span className="px-2 text-[11.5px] text-white/60">拖入常用应用</span>
          )}
        </div>
      </div>

      {/* 拖拽幽灵（跟随指针放大半透明） */}
      {drag && <DragGhost layout={layout} drag={drag} appByKey={appByKey} />}

      {/* 文件夹打开层 */}
      {folder && (
        <div
          className="absolute inset-0 z-30 grid place-items-center bg-black/35 backdrop-blur-2xl"
          onClick={() => setOpenFolder(null)}
        >
          <div
            className="ios-folder-panel flex max-h-[68vh] w-[min(640px,80vw)] flex-col gap-5 rounded-[40px] px-8 py-7"
            onClick={(e) => e.stopPropagation()}
          >
            <h2 className="text-center text-[17px] font-semibold [text-shadow:0_1px_4px_rgba(0,0,0,.4)]">
              {folder.name}
            </h2>
            <div className="grid grid-cols-6 justify-items-center gap-y-5 overflow-y-auto">
              {folder.apps.map((key) => {
                const a = appByKey.get(key);
                if (!a) return null;
                return (
                  <AppIcon
                    key={key}
                    name={a.display_name}
                    iconPath={a.icon_path}
                    onClick={() => {
                      setOpenFolder(null);
                      launch(key);
                    }}
                  />
                );
              })}
            </div>
          </div>
        </div>
      )}

      {/* 搜索结果层 */}
      {q && (
        <div className="absolute inset-x-0 top-16 bottom-44 z-30 overflow-y-auto px-16 pt-4">
          {results.length === 0 ? (
            <p className="mt-10 text-center text-[13px] text-white/70">没有匹配「{query}」的应用</p>
          ) : (
            <div className="grid grid-cols-7 justify-items-center gap-y-6">
              {results.map((a) => (
                <AppIcon
                  key={a.app_key}
                  name={a.display_name}
                  iconPath={a.icon_path}
                  onClick={() => {
                    setQuery('');
                    launch(a.app_key);
                  }}
                />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );

  function registerCell(el: HTMLElement | null, key: string) {
    if (el) cellEls.current.set(key, el);
    else cellEls.current.delete(key);
  }
}

/** pointerup 落点提交（普通重排 / 跨容器移动） */
function applyDrop(layout: HomeLayout, d: DragState, target: string, _hint: boolean): HomeLayout {
  const [tKind, tPage, tIdx] = parseCellKey(target);
  if (tKind === 'dock') {
    if (d.kind === 'page') return moveToDock(layout, d.page, d.idx);
    if (d.kind === 'dock' && tIdx !== d.idx) {
      return { ...layout, dock: reorder(layout.dock, d.idx, tIdx) };
    }
    return layout;
  }
  if (d.kind === 'dock') {
    // Dock → 页面：先移出到最后一页再跨页调整
    const after = removeFromDock(layout, d.idx);
    const last = after.pages.length - 1;
    return moveAcrossPages(after.pages, last, after.pages[last].length - 1, tPage, tIdx).length
      ? { ...after, pages: moveAcrossPages(after.pages, last, after.pages[last].length - 1, tPage, tIdx) }
      : after;
  }
  if (d.page === tPage) {
    return { ...layout, pages: layout.pages.map((p, i) => (i === d.page ? reorder(p, d.idx, tIdx) : p)) };
  }
  return { ...layout, pages: moveAcrossPages(layout.pages, d.page, d.idx, tPage, tIdx) };
}

/** 悬停超时的文件夹合并 */
function applyMerge(layout: HomeLayout, d: DragState, target: string): HomeLayout {
  const [tKind, tPage, tIdx] = parseCellKey(target);
  if (tKind !== 'page' || d.kind !== 'page') return layout;
  return mergeIntoFolder(layout, tPage, d.idx, tIdx);
}

function parseCellKey(key: string): ['page' | 'dock', number, number] {
  const [k, p, i] = key.split(':');
  return [k === 'd' ? 'dock' : 'page', Number(p), Number(i)];
}

function hoverKeyRefToBool(_hoverKey: string | null, target: string) {
  return _hoverKey === target;
}

function DragGhost({
  layout,
  drag,
  appByKey,
}: {
  layout: HomeLayout;
  drag: DragState;
  appByKey: Map<string, AppEntry>;
}) {
  const item =
    drag.kind === 'dock'
      ? `app:${layout.dock[drag.idx]}`
      : layout.pages[drag.page]?.[drag.idx];
  if (!item) return null;
  const name = item.startsWith('folder:')
    ? layout.folders[rawFolderId(item)]?.name ?? ''
    : appByKey.get(rawAppKey(item))?.display_name ?? '';
  const icon = item.startsWith('folder:') ? null : appByKey.get(rawAppKey(item))?.icon_path;
  return (
    <div
      className="pointer-events-none fixed z-50 scale-110 opacity-85 drop-shadow-2xl"
      style={{ left: drag.x - 42, top: drag.y - 42 }}
    >
      <AppIcon name={name} iconPath={icon} label={false} size={72} />
    </div>
  );
}

/** 分钟级时钟（状态栏） */
function useClockMinute() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const t = setInterval(() => setNow(new Date()), 10_000);
    return () => clearInterval(t);
  }, []);
  const time = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
  const date = new Intl.DateTimeFormat('zh-CN', { month: 'long', day: 'numeric', weekday: 'long' }).format(now);
  return { time, date };
}

/** 顶部状态栏小圆钮 */
function IconBtn({
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
      className="grid h-8 min-w-8 place-items-center rounded-full bg-white/18 px-2 text-white/90 backdrop-blur-md ring-1 ring-white/15 transition-colors hover:bg-white/28 ios-ease"
    >
      {children}
    </button>
  );
}
