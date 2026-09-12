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
import { commands, events, isTauri } from '@/shared/lib/ipc';

const appWindow = isTauri ? getCurrentWindow() : null;

/**
 * 应用骨架（对齐水豚hub 全屏接管形态）。
 * 桌面模式：完全沉浸的全屏桌面——无标题栏/无侧栏，默认页 /desktop
 * （壁纸 + 问候大时钟 + 搜索 + 小组件），底部贴边通栏任务栏完全替代系统任务栏。
 * 窗口化模式：红绿灯标题栏 + 侧栏 + 页面 + 居中胶囊 Dock。
 * /home 为「主屏」页（iOS 图标网格，自身已含交互 Dock 与顶栏）。
 */
export function AppShell() {
  const [coreVersion, setCoreVersion] = useState<string | null>(null);
  const [desktop, setDesktop] = useState(false);
  const [pickerOpen, setPickerOpen] = useState(false);
  const location = useLocation();
  const navigate = useNavigate();
  const isHome = location.pathname === '/home';
  const isDesktopHome = location.pathname === '/desktop';
  // 桌面接管下的子页（设置/代理/日程…）：真机里侧栏与任务栏都不在主窗口，
  // 悬浮返回胶囊 + Esc 是这些页面唯一的「回桌面」入口，避免进去出不来
  const showDesktopBack = desktop && !isDesktopHome && !isHome;

  // Esc 快捷返回桌面（输入框聚焦时不劫持）
  useEffect(() => {
    if (!showDesktopBack) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      const el = document.activeElement;
      if (el instanceof HTMLElement && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable)) {
        return;
      }
      navigate('/desktop');
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [showDesktopBack, navigate]);

  // Rust 核心就绪事件（IPC 事件流演示）
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    events.coreReady.listen((e) => setCoreVersion(e.payload.version)).then((fn) => (unlisten = fn));
    // 兜底：直接查一次
    commands.appHealth().then((h) => setCoreVersion(h.version)).catch(() => {});
    return () => unlisten?.();
  }, []);

  // 桌面模式事件跟随：进入 → /desktop 桌面主页（已停在 /home 主屏则不劫持），
  // 退出 → 窗口化工作台。
  // navigate 随 location 变化（标识不稳定），重挂监听无害；alive 标志避免
  // StrictMode/快速卸载时「注销函数晚于卸载 resolve」导致的重复监听。
  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;
    events.desktopModeChanged
      .listen((e) => {
        setDesktop(e.payload.active);
        navigate(
          e.payload.active ? (location.pathname === '/home' ? '/home' : '/desktop') : '/',
        );
      })
      .then((fn) => (alive ? (unlisten = fn) : fn()));
    return () => {
      alive = false;
      unlisten?.();
    };
  }, [navigate, location.pathname]);

  // 初始对齐（仅挂载时执行一次）：启动即桌面模式 → 桌面主页。
  // 注意不能依赖 [navigate]——它随路由变化，会把任意导航都 replace 弹回 /desktop。
  // 启动时 hash 可能是 ''（Tauri 初始 URL 无 hash）或 '#/'，两者都视为默认页。
  useEffect(() => {
    commands
      .desktopModeIsActive()
      .then((active) => {
        setDesktop(active);
        const h = window.location.hash;
        if (active && (h === '' || h === '#/')) navigate('/desktop', { replace: true });
      })
      .catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
        <WebDebugBar />
      </>
    );
  }

  return (
    <div
      className={`flex h-screen flex-col ${desktop ? 'overflow-hidden' : ''}`}
      style={{ background: 'var(--bg, #17151b)' }}
      data-tauri-drag-region={false}
    >
      {/* 桌面接管子页的返回胶囊（与桌面主页控制中心钮同一视觉语言） */}
      {showDesktopBack && (
        <button
          onClick={() => navigate('/desktop')}
          title="返回桌面主页（Esc）"
          className="fixed left-4 top-4 z-40 flex items-center gap-1 rounded-full bg-black/30 px-4 py-2 text-[12.5px] font-medium text-white/85 ring-1 ring-white/15 backdrop-blur-xl transition-all hover:bg-black/45 hover:text-white"
        >
          <ChevronLeft size={15} />
          返回桌面
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
          className={`min-w-0 flex-1 ${
            desktop
              ? isDesktopHome
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
      <WebDebugBar />
    </div>
  );
}
