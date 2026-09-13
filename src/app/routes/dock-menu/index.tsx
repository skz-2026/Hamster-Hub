import DockMenuWindow from '@/features/home/DockMenuWindow';

/**
 * dock 右键菜单独立置顶弹窗（桌面接管时由 Rust dock_menu_open 定位显示）：
 * 任务栏独立窗口只有一条高放不下纵向菜单，弹成独立小窗承载，
 * 主窗口被其它应用盖住时菜单照样可见可点。
 */
export default function DockMenuPage() {
  return <DockMenuWindow />;
}
