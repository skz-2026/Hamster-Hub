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
        // 近实心深色条（对齐水豚hub）：桌面上读作任务栏，app 最大化后内容
        // 不透过 dock 搅局；顶部 1px 高光标出条的边界
        background:
          'linear-gradient(to bottom, rgba(24,22,30,.88) 0%, rgba(17,15,21,.93) 55%, rgba(13,11,16,.96) 100%)',
        boxShadow: 'inset 0 1px 0 rgba(255,255,255,.10)',
      }}
      onContextMenu={(e) => e.preventDefault()}
    >
      <DockBar desktop />
    </div>
  );
}
