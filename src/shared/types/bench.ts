/**
 * 代理工作台（bench）类型：契约类型已由 tauri-specta 生成（src/shared/types/ipc.ts），
 * 此处统一再导出供 bench 域引用；仅前端专用视图类型在本文件定义。
 */
import type {
  AgentInfo,
  IndexStatus,
  SearchHit,
  SearchQuery,
  SessionMessagesPage,
  SessionSummary,
  SnapshotMessage,
  StreamEvent,
  StreamEventDiff,
  LiveSessionInfo,
  WorkspaceRecord,
} from '@/shared/types/ipc';

export type {
  AgentInfo,
  IndexStatus,
  LaunchOption,
  LaunchOptions,
  LiveSessionInfo,
  PromptInject,
  SearchHit,
  SearchQuery,
  SessionMessagesPage,
  SessionSummary,
  SnapshotMessage,
  SnapshotRole,
  StreamEvent,
  StreamEventDiff,
  WorkspaceRecord,
} from '@/shared/types/ipc';

/** 渲染行（stream-registry 归并产物；对齐 Molto StreamRow） */
export interface StreamRow {
  itemId: string;
  role: 'user' | 'assistant' | 'thinking' | 'tool' | 'system';
  text: string;
  toolName?: string;
  status?: string;
  at: number;
  streaming?: boolean;
  pending?: boolean;
  diff?: StreamEventDiff | null;
}

/** 流式事件回调（事件总线订阅，见 features/bench/events） */
export type StreamOnEvent = (ev: StreamEvent) => void;

/** PTY 数据出口（真机 = tauri Channel 实例；结构化子集，mock 直接回调 onmessage） */
export interface PtyChannelOut {
  onmessage: (data: number[]) => void;
}

/** bench_pty_create 参数（specta SpectaFn 上限 10 参，命令侧打包为结构体） */
export interface PtyCreateArgs {
  agentId: string;
  projectDir: string;
  firstPrompt: string | null;
  model: string | null;
  effort: string | null;
  resumeKey: string | null;
  cols: number;
  rows: number;
}

/**
 * bench 域命令面（生成绑定 benchXxx 的语义化门面）。
 * ipc.ts 的 benchCommands 导出按本接口塑造，特性代码不直接依赖生成名。
 */
export interface BenchCommands {
  // agents / discovery
  benchScanAgents(): Promise<AgentInfo[]>;
  benchListProjects(): Promise<string[]>;
  /** 各已安装 Agent 会话记录里发现的历史工作目录 */
  benchAgentWorkspaces(): Promise<WorkspaceRecord[]>;
  // 流式通道（GUI 对话；事件经 events.benchStreamEvent 推送）
  benchStreamCreate(
    agentId: string,
    projectDir: string,
    firstPrompt: string | null,
    model: string | null,
    effort: string | null,
    resumeKey: string | null,
    fork: boolean,
  ): Promise<LiveSessionInfo>;
  benchStreamSend(sessionId: string, text: string, model: string | null, effort: string | null): Promise<null>;
  benchStreamInterrupt(sessionId: string): Promise<null>;
  benchStreamKill(sessionId: string): Promise<null>;
  benchListStreamSessions(): Promise<LiveSessionInfo[]>;
  // PTY 终端（TUI resume / 混用模式；输出经 onData 回调，退出经 events.benchPtyExit）
  benchPtyCreate(args: PtyCreateArgs, onData: PtyChannelOut): Promise<LiveSessionInfo>;
  benchPtyWrite(sessionId: string, data: string): Promise<null>;
  benchPtySendPrompt(sessionId: string, text: string): Promise<null>;
  benchPtyResize(sessionId: string, cols: number, rows: number): Promise<null>;
  benchPtyKill(sessionId: string): Promise<null>;
  benchListLiveSessions(): Promise<LiveSessionInfo[]>;
  // 会话索引（Recall）
  benchListHistorySessions(): Promise<SessionSummary[]>;
  /** 某代理在项目下最新入索引的会话键（登记表恢复回退） */
  benchLatestIndexedSession(agent: string, projectDir: string): Promise<SessionSummary | null>;
  benchSearchSessions(query: SearchQuery): Promise<SearchHit[]>;
  benchSessionMessages(agent: string, sessionKey: string, aroundSeq: number, window: number): Promise<SnapshotMessage[]>;
  benchListSessionMessages(agent: string, sessionKey: string, fromSeq: number, limit: number): Promise<SessionMessagesPage>;
  benchIndexStatus(): Promise<IndexStatus>;
  benchIndexRefresh(): Promise<IndexStatus>;
  benchReindex(): Promise<IndexStatus>;
  benchSessionDelete(agent: string, sessionKey: string): Promise<null>;
}