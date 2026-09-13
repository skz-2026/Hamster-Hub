import type { agent as agentZh } from '../zh-CN/agent';

export const agent: Record<keyof typeof agentZh, string> = {
  // Common
  'agent.assistantName': 'Desktop Assistant',
  'agent.send': 'Send',
  'agent.retry': 'Retry',
  'agent.fileFallback': 'File',

  // AI assistant page (/agent)
  'agent.title': 'AI Assistant',
  'agent.systemPrompt':
    "You are the built-in AI assistant of the HamsterHub desktop assistant. Keep answers concise, friendly, and practical; reply in the user's language.",
  'agent.requestFailed': 'Request failed: {msg}',
  'agent.tab.assistant': 'Desktop Assistant',
  'agent.tab.quickAsk': 'Quick Ask',
  'agent.subtitleAssistant': 'Agent-powered · Can control the desktop',
  'agent.subtitleQuick': 'Quick Ask · Direct AI chat',
  'agent.placeholderQuick': 'Ask something… (Enter to send, Shift+Enter for a new line)',
  'agent.placeholderAssistant':
    'Give the desktop assistant a task… (e.g. "Add \'submit the weekly report on Friday\' as a to-do, then open the calculator")',
  'agent.launchAssistant': 'Start desktop assistant',
  'agent.clearSession': 'Clear conversation',
  'agent.sessionActive': 'Assistant session active (end button is in the session header)',
  'agent.emptyTitle': 'How can I help?',
  'agent.emptySubtitle': 'Q&A, writing, translation, ideas… configure a model to get started',
  'agent.suggestion.weeklyReport': 'Help me start a weekly report',
  'agent.suggestion.tools': 'Recommend some productivity tools',
  'agent.suggestion.mcp': 'Explain what MCP is',
  'agent.unconfiguredHint': 'No AI model configured yet — set the API URL, model name, and key on the Settings page first.',
  'agent.goConfigure': 'Configure',

  // AssistantPanel: error hints / session banner / welcome state
  'agent.errMcpMissingTitle': 'Desktop MCP server missing',
  'agent.errMcpMissingDetail': 'hamster-mcp.exe does not exist: rebuild (cargo build -p hamster-mcp) or reinstall the app.',
  'agent.errNoAgentTitle': 'No Agent CLI available',
  'agent.errNoAgentDetail': 'The desktop assistant is driven by a locally installed Agent (Claude Code / ZCode, etc.). Install any one of them first.',
  'agent.errStartFailedTitle': 'Failed to start the desktop assistant',
  'agent.mcpInjectedTitle': 'This session has the hamster-desktop MCP server injected: apps/files/to-dos/volume + browser/desktop actions (the latter are gated by a safety switch; every call is audited)',
  'agent.desktopToolsOn': 'Desktop tools connected',
  'agent.computerUseOff': ' · Desktop click/type is off (enable in Settings)',
  'agent.composerPlaceholder': 'Message the desktop assistant… (Enter to send, Shift+Enter for a new line)',
  'agent.fallbackQuickAsk': 'Continue with Quick Ask (direct AI)',
  'agent.welcomeTitle': 'Desktop Assistant · Understands you and gets things done',
  'agent.welcomeDesc': 'Powered by a local Agent, it operates this computer directly through desktop tools: launch apps, find files, add to-dos, adjust volume, and open a browser to look things up when needed. Every tool call is audited.',
  'agent.computerUseHint': 'Desktop click/type (computer use) is off by default; enable "Allow computer control" on the Settings page',

  // Desktop MCP distribution card
  'agent.mcpCardTitle': 'Desktop MCP · Distribute to Agents',
  'agent.mcpCardDesc': 'Writes HamsterHub desktop capabilities (apps/files/to-dos/volume/browser/screen) into the selected agent as standard MCP config — takes effect when enabled, removed when disabled; backed up automatically before writing so you can roll back.',
  'agent.copyUrl': 'Copy URL',
  'agent.toggleToken': 'Show/hide token',
  'agent.copyToken': 'Copy token',
  'agent.regenerateToken': 'Regenerate (the old token becomes invalid immediately; distributed configs need updating)',
  'agent.portFallback': 'Preferred port {port} is in use, so a random port was used this time — the URL in distributed configs is unavailable for now. Restart the app after freeing the port to restore it.',
  'agent.noAgents': 'No MCP-capable agents detected (they appear automatically after installing Claude Code / Codex, etc.).',
  'agent.distributeTo': 'Distribute desktop MCP to {name}',

  // Search page / Spotlight
  'agent.groupApps': 'Apps',
  'agent.groupFiles': 'Files',
  'agent.groupWeb': 'Web',
  'agent.groupAi': 'AI',
  'agent.appEnterHint': 'App · Enter to launch',
  'agent.searchTitle': 'Search',
  'agent.searchHint': 'Matches pinyin, initials, or Chinese substrings',
  'agent.searchPlaceholder': 'Search apps, files, and the web…',
  'agent.emptyHint': 'Type to search: apps (e.g. "WeChat", "wx", "weixin"), local files, the web, and Ask AI.',
  'agent.spotlightHint': 'Press Alt+Space anytime to bring up Spotlight, backed by the same search as this page.',
  'agent.webSearch': 'Search "{q}"',
  'agent.webBaidu': 'Web · Baidu',
  'agent.askAi': 'Ask AI: "{q}"',
  'agent.aiAssistant': 'AI Assistant',
  'agent.todoQuickAdd': 'Add to-do: "{q}"',
  'agent.todoNoDue': 'No due date',
  'agent.noResults': 'No results for "{q}"',
};
