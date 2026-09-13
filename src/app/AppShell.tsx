import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { TitleBar } from '@/app/TitleBar';
import { SideNav } from '@/app/SideNav';
import { ChevronLeft } from 'lucide-react';
import { WebDebugBar } from '@/shared/components/WebDebugBar';
import { DockBar } from '@/features/home/DockBar';
import { DockPickerModal } from '@/features/home/DockPickerModal';
import { useWallpaper } from '@/features/home/hooks';
import { wallpaperSrcOf } from '@/features/home/WallpaperLayer';
import { commands, events, isTauri } from '@/shared/lib/ipc';
import { useI18n } from '@/shared/i18n/provider';
import ReminderDialog from '@/features/todo/ReminderDialog';
import { bindNotificationSources } from '@/features/notifications/store';

const appWindow = isTauri ? getCurrentWindow() : null;

/**
 * 应用骨架（全屏接管形态）。
 * 桌面模式：完全沉浸的全屏桌面——无标题栏/无侧栏，默认页 /（首页，接管形态）
 * （壁纸 + 问候大时钟 + 搜索 + 小组件），底部贴边通栏任务栏完全替代系统任务栏。
 * 窗口化模式：红绿灯标题栏 + 侧栏 + 页面 + 居中胶囊 Dock。
 * /home 为「主屏」页（iOS 图标网格，自身已含交互 Dock 与顶栏）。
 */
export function AppShell() {
  const { t } = useI18n();
  const wallpaper = useWallpaper();
  const [coreVersion, setCoreVersion] = useState<string | null>(null);
  const [desktop, setDesktop] = useState(false);
  const [pickerOpen, setPickerOpen] = useState(false);
  const location = useLocation();
  const navigate = useNavigate();
  const isHome = location.pathname === '/home';
  // 首页（工作台 × 桌面主页合一）接管形态全屏铺壁纸
  const isHomePage = location.pathname === '/';
  // 桌面接管下的子页（设置/代理/日程…）：真机里侧栏与任务栏都不在主窗口，
  // 悬浮返回胶囊 + Esc 是这些页面唯一的「回桌面」入口，避免进去出不来
  const showDesktopBack = desktop && !isHomePage && !isHome;

  // Esc 快捷返回桌面（输入框聚焦时不劫持）
  useEffect(() => {
    if (!showDesktopBack) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      const el = document.activeElement;
      if (el instanceof HTMLElement && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable)) {
        return;
      }
      navigate('/');
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [showDesktopBack, navigate]);

  // 通知源接入（幂等）：待办到点 / AI Agent 会话结束 / 番茄钟完成 / 更新就绪。
  // 挂在壳层而非通知面板里——面板只在桌面主页/窗口标题栏按需渲染，
  // 而通知要在**任何页面、两种形态**下都能收（离开页面回来仍能看到未读）。
  useEffect(() => {
    bindNotificationSources();
  }, []);

  // Rust 核心就绪事件（IPC 事件流演示）
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    events.coreReady.listen((e) => setCoreVersion(e.payload.version)).then((fn) => (unlisten = fn));
    // 兜底：直接查一次
    commands.appHealth().then((h) => setCoreVersion(h.version)).catch(() => {});
    return () => unlisten?.();
  }, []);

  // 桌面模式事件跟随：进入/退出 → 首页（/ 自身双形态，无需换页）。
  // 已停在 /home 主屏则不劫持（HomeScreen 自理退出回落）。
  // navigate 随 location 变化（标识不稳定），重挂监听无害；alive 标志避免
  // StrictMode/快速卸载时「注销函数晚于卸载 resolve」导致的重复监听。
  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;
    events.desktopModeChanged
      .listen((e) => {
        setDesktop(e.payload.active);
        if (location.pathname !== '/home') navigate('/');
      })
      .then((fn) => (alive ? (unlisten = fn) : fn()));
    return () => {
      alive = false;
      unlisten?.();
    };
  }, [navigate, location.pathname]);

  // 初始对齐（仅挂载时执行一次）：只同步模式态；首页 / 自身侦听模式切换形态，
  // 子页路由保持原样（此前「启动即桌面模式 → 跳 /desktop」已不需要）。
  useEffect(() => {
    commands
      .desktopModeIsActive()
      .then((active) => setDesktop(active))
      .catch(() => {});
  }, []);

  // 任务栏独立窗口的路由图标 → 跨窗口事件驱动主窗口导航（tauri 专属路径）。
  // 主窗口常被全屏应用（浏览器等）盖住：导航后顺带把主窗口提到前台。
  useEffect(() => {
    if (!isTauri) return;
    let unlisten: (() => void) | undefined;
    listen<{ to: string }>('hamster:navigate', async (e) => {
      navigate(e.payload.to);
      try {
        const w = getCurrentWindow();
        await w.show();
        await w.setFocus();
      } catch {
        /* 前台化失败不阻断导航 */
      }
    })
      .then((fn) => (unlisten = fn))
      .catch(console.error);
    return () => unlisten?.();
  }, [navigate]);

  // 任务栏「+」→ 主窗口弹出应用选择器（并前置）
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    const open = async () => {
      setPickerOpen(true);
      if (isTauri) {
        try {
          const w = getCurrentWindow();
          await w.show();
          await w.setFocus();
        } catch {
          /* ignore */
        }
      }
    };
    if (isTauri) {
      listen('hamster:open-app-picker', open)
        .then((fn) => (unlisten = fn))
        .catch(console.error);
    }
    // 浏览器预览：任务栏胶囊的 + 用 DOM 事件唤起本窗口弹层
    window.addEventListener('hamster:open-app-picker', open as EventListener);
    return () => {
      unlisten?.();
      window.removeEventListener('hamster:open-app-picker', open as EventListener);
    };
  }, [navigate]);


  // 主屏（iOS 图标网格）自绘全屏，无壳。任务栏「+」的选择器弹层仍要可用
  //（否则主屏页点「+」毫无反应），故在此分支同样渲染
  if (isHome) {
    return (
      <>
        <Outlet />
        {pickerOpen && <DockPickerModal onClose={() => setPickerOpen(false)} />}
        {/* 待办到点提醒弹框（全局：所有路由/双形态都要能弹） */}
        <ReminderDialog />
        <WebDebugBar />
      </>
    );
  }

  return (
    <div
      className={`relative flex h-screen flex-col ${desktop ? 'overflow-hidden' : ''}`}
      style={{ background: 'var(--bg, #17151b)' }}
      data-tauri-drag-region={false}
    >
      {/* 接管子页的壁纸 backdrop：模糊壁纸 + 主题纱罩。子页不再是一块实色 --bg
          （桌面沉浸感在进子页就断了），也不裸铺壁纸（浅色壁纸上 --text 深色字
          会没对比）；换壁纸即时同步（跟随主屏布局） */}
      {showDesktopBack && (
        <div className="desktop-backdrop absolute inset-0 overflow-hidden" aria-hidden>
          <div className="absolute inset-0" style={{ background: wallpaper.css }} />
          {wallpaperSrcOf(wallpaper) && (
            <div className="absolute -inset-12">
              <img
                src={wallpaperSrcOf(wallpaper)}
                alt=""
                draggable={false}
                className="size-full select-none object-cover"
              />
            </div>
          )}
          <div className="desktop-backdrop-veil absolute inset-0" />
        </div>
      )}
      {/* 桌面接管子页的返回胶囊（与桌面主页控制中心钮同一视觉语言） */}
      {/* 桌面接管子页的返回胶囊。子页底下是实色 --bg 而非壁纸，
          玻璃底必须走主题 token（写死 black/30 + 白字在 light 下糊成脏灰胶囊） */}
      {showDesktopBack && (
        <button
          onClick={() => navigate('/')}
          title={t('chrome.shell.backToDesktopTitle')}
          className="fixed left-4 top-4 z-40 flex items-center gap-1 rounded-full bg-[var(--panel-strong)] px-4 py-2 text-[12.5px] font-medium text-[var(--text)] ring-1 ring-[var(--border)] backdrop-blur-xl transition-colors hover:bg-[var(--hover-strong)] hover:text-[var(--text)]"
        >
          <ChevronLeft size={15} />
          {t('chrome.shell.backToDesktop')}
        </button>
      )}
      {!desktop && (
        <TitleBar
          coreVersion={coreVersion}
          onMinimize={() => appWindow?.minimize()}
          onClose={() => appWindow?.hide()}
        />
      )}
      <div className="flex min-h-0 flex-1">
        {/* 桌面模式无侧栏（沉浸桌面）；窗口化保留侧栏导航 */}
        {!desktop && <SideNav />}
        {/* /desktop 全屏铺满壁纸；其余页面留内边距。
            真机桌面接管：主窗口整屏全屏，底部 68 逻辑 px 被独立置顶任务栏窗遮挡
            （浏览器预览的 Dock 是 flex 流内嵌，不遮挡）——子页须预留底部空间，
            否则页底内容（快问输入框、设置页最后一行…）被任务栏盖住点不到；
            顶部同理：悬浮返回胶囊（left-4 top-4）压在内容上层，子页须预留
            pt-[60px]，否则左上角内容（bench 侧栏「新会话」、各页标题）被胶囊盖住 */}
        <main
          className={`relative min-w-0 flex-1 ${
            desktop
              ? isHomePage
                ? 'overflow-hidden'
                : `overflow-y-auto px-4 py-4 ${isTauri ? 'pb-[80px]' : ''} ${
                    showDesktopBack ? 'pt-[60px]' : ''
                  }`
              : 'overflow-y-auto px-6 py-5'
          }`}
        >
          <Outlet />
        </main>
      </div>
      {/* 任务栏：真机桌面模式由独立置顶 taskbar 窗口渲染（不被任何应用遮挡），
          此处不再内嵌；窗口化模式与浏览器预览保留内嵌 Dock */}
      {(!desktop || !isTauri) && <DockBar desktop={desktop} />}
      {/* 任务栏「+」唤起的应用选择器（主窗口弹层） */}
      {pickerOpen && <DockPickerModal onClose={() => setPickerOpen(false)} />}
      {/* 待办到点提醒弹框（全局：所有路由/双形态都要能弹） */}
      <ReminderDialog />
      <WebDebugBar />
    </div>
  );
}
