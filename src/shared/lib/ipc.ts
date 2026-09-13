/**
 * IPC 出口统一封装 —— 应用代码访问 commands/events 的唯一入口。
 *
 * tauri 环境：直通 tauri-specta 生成的绑定（src/shared/types/ipc.ts，勿手改）。
 * 浏览器环境：自动切换到 ipc-mock 假实现（Agent.md 第 1 层工作流），并用
 * mockEvents 模拟系统事件；home.layout 等持久化到 localStorage。
 */
import { commands as realCommands, events as realEvents } from '@/shared/types/ipc';
import { mockCommands, mockEvents } from './ipc-mock';
import type { BenchCommands } from '@/shared/types/bench';

export const isTauri =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const commands = (
  isTauri ? realCommands : mockCommands
) as typeof realCommands;

export const events = (
  isTauri ? realEvents : mockEvents
) as typeof realEvents;

/**
 * bench 域命令出口：mock 与生成绑定同名（benchXxx），此转换塑造为
 * BenchCommands 语义化门面，特性代码不直接依赖生成名。
 */
export const benchCommands = commands as unknown as BenchCommands;

export type {
  AppEntry,
  AppHealth,
  AppError,
  AppWindowInfo,
  DockMenuPayload,
  CountdownCustom,
  CountdownItem,
  FileHit,
  FocusStatus,
  GenOptions,
  Note,
  ProcInfo,
  Settings,
  SystemSnapshot,
  Todo,
  TodoReminder,
  VaultItem,
  VaultItemInput,
  VaultSecret,
  VaultStatus,
  WeatherNow,
} from '@/shared/types/ipc';
