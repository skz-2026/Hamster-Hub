/**
 * TerminalView：PTY 会话的 xterm 终端（TUI 承载）。
 * 挂载即 benchPtyCreate（resumeKey 幂等）并注册 ipc Channel；用户键入直写 stdin，
 * GUI 尺寸变化同步 bench_pty_resize（顺带促使 TUI 重绘）。会话随组件卸载结束
 * （switch 走页面层的 keep-mounted 池，不触发卸载）。
 */
import { useEffect, useRef } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { Channel } from '@tauri-apps/api/core';
import '@xterm/xterm/css/xterm.css';
import { benchCommands, isTauri } from '@/shared/lib/ipc';
import type { LiveSessionInfo } from '@/shared/types/bench';

export default function TerminalView({
  session,
  onPtyReady,
}: {
  session: LiveSessionInfo;
  /** 创建成功后回填真实会话信息（页面更新活会话池，不重挂载） */
  onPtyReady?: (info: LiveSessionInfo) => void;
}) {
  const hostRef = useRef<HTMLDivElement>(null);
  // session 引用进 effect：挂载只发生一次（keep-mounted 池保证）
  const sessionRef = useRef(session);
  sessionRef.current = session;
  const readyRef = useRef(onPtyReady);
  readyRef.current = onPtyReady;

  useEffect(() => {
    const host = hostRef.current;
    const s = sessionRef.current;
    if (!host) return;

    const term = new Terminal({
      fontSize: 12.5,
      fontFamily: "'Cascadia Code', Consolas, 'Courier New', monospace",
      cursorBlink: true,
      scrollback: 5000,
      theme: {
        background: '#141317',
        foreground: '#e8e2da',
        cursor: '#ff8a3d',
        selectionBackground: 'rgba(255,138,61,0.30)',
      },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host);
    try {
      fit.fit();
    } catch {
      /* 容器尚未布局时忽略，ResizeObserver 会再触发 */
    }

    let disposed = false;
    let realSid = s.sessionId;
    const resize = (): void => {
      try {
        fit.fit();
        void benchCommands.benchPtyResize(realSid, term.cols, term.rows);
      } catch {
        /* 隐藏态尺寸为 0，跳过 */
      }
    };
    const observer = new ResizeObserver(() => resize());
    observer.observe(host);

    const channel = isTauri ? new Channel<number[]>() : { onmessage: (_: number[]) => {} };
    channel.onmessage = (data) => {
      if (!disposed) term.write(new Uint8Array(data));
    };

    void benchCommands
      .benchPtyCreate(
        {
          agentId: s.agentId,
          projectDir: s.projectDir,
          firstPrompt: null,
          model: null,
          effort: null,
          resumeKey: s.resumeKey ?? null,
          cols: term.cols,
          rows: term.rows,
        },
        channel,
      )
      .then((info) => {
        realSid = info.sessionId;
        // 尺寸同步一次（促使 TUI 按实际大小重绘；重复 attach 场景即由此触发重绘）
        void benchCommands.benchPtyResize(realSid, term.cols, term.rows);
        readyRef.current?.(info);
      })
      .catch((e) => {
        term.writeln(`\x1b[31m会话创建失败：${String(e).replace(/^Error:\s*/, '')}\x1b[0m`);
      });

    // 用户键入直写 stdin（TUI 自己回显）
    const inputDisp = term.onData((d) => {
      if (!disposed) void benchCommands.benchPtyWrite(realSid, d);
    });

    // 卸载不杀会话（Molto 语义：会话存活，视图重建经 attach 重新接管输出）；
    // 结束会话走头部按钮 / 页面关闭（benchPtyKill）
    return () => {
      disposed = true;
      observer.disconnect();
      inputDisp.dispose();
      term.dispose();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return <div ref={hostRef} className="h-full min-h-0 w-full px-2 py-1" />;
}
