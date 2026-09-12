import { DockBar } from '@/features/home/DockBar';

/**
 * 任务栏独立置顶窗口（仅桌面接管时由 Rust 显示，贴屏幕底部条）：
 * 渲染 DockBar 桌面形态——定制/常用分组 + 系统入口 + 时钟，替代系统任务栏持续渲染，
 * always-on-top 保证任何应用窗口都不遮挡它。路由图标经跨窗口事件驱动主窗口导航。
 * 底条视觉：macOS 风——深色渐变 + 顶部 1px 高光 + 底部微暗角。
 */
export default function TaskbarPage() {
  return (
    <div
      className="flex h-screen w-full flex-col justify-end overflow-hidden"
      style={{
        // 轻着色半透明（macOS dock 风）：壁纸透过 dock 延续，无分界线；
        // 图标可读性由图标自身的底座承担
        background:
          'linear-gradient(to bottom, rgba(24,22,30,.42) 0%, rgba(18,16,22,.55) 55%, rgba(14,12,17,.62) 100%)',
      }}
      onContextMenu={(e) => e.preventDefault()}
    >
      <DockBar desktop />
    </div>
  );
}
