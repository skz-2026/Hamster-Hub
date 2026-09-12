import { useState } from 'react';
import { FlaskConical } from 'lucide-react';
import { commands, isTauri } from '@/shared/lib/ipc';

/**
 * 浏览器预览调试条（仅非 tauri 环境渲染）：模拟桌面模式进出与 Spotlight 热键，
 * 让 /home 与 /spotlight 在浏览器里可完整验证。
 */
export function WebDebugBar() {
  const [active, setActive] = useState(false);
  if (isTauri) return null;

  const toggle = async () => {
    if (active) {
      await commands.desktopModeExit();
      setActive(false);
    } else {
      await commands.desktopModeEnter();
      setActive(true);
    }
  };

  return (
    <div className="fixed bottom-24 left-3 z-[999] flex items-center gap-2 rounded-xl bg-neutral-900/85 px-3 py-2 text-[11px] text-neutral-200 ring-1 ring-white/15 backdrop-blur">
      <FlaskConical size={13} className="text-[#ff8a3d]" />
      <span className="opacity-70">浏览器预览 · IPC mock</span>
      <button
        onClick={toggle}
        className={`rounded-lg px-2.5 py-1 font-medium transition-colors ${
          active ? 'bg-[#ff8a3d] text-white' : 'bg-white/10 hover:bg-white/20'
        }`}
      >
        {active ? '退出桌面模式' : '进入桌面模式'}
      </button>
      <a
        href="#/spotlight"
        className="rounded-lg bg-white/10 px-2.5 py-1 font-medium transition-colors hover:bg-white/20"
      >
        Spotlight
      </a>
    </div>
  );
}
