import type { bench as benchZh } from '../zh-CN/bench';

export const bench: Record<keyof typeof benchZh, string> = {
  // Common
  'bench.common.loading': 'Loading…',
  'bench.common.cancel': 'Cancel',

  // Relative time (compact, plural-safe)
  'bench.time.justNow': 'Just now',
  'bench.time.minutesAgo': '{n} min ago',
  'bench.time.hoursAgo': '{n} h ago',
  'bench.time.daysAgo': '{n} d ago',

  // Welcome: time-of-day greetings
  'bench.welcome.greeting.night': 'Working late? What can I help you with?',
  'bench.welcome.greeting.earlyMorning': 'Good morning — what can I help you with?',
  'bench.welcome.greeting.morning': 'Good morning — what can I help you with?',
  'bench.welcome.greeting.noon': 'Good afternoon — what can I help you with?',
  'bench.welcome.greeting.afternoon': 'Good afternoon — what can I help you with?',
  'bench.welcome.greeting.evening': 'Good evening — what can I help you with?',

  // Welcome: quick prompt cards (label + prompt body sent to the AI)
  'bench.welcome.suggest.repoTitle': 'Map repo structure',
  'bench.welcome.suggest.repoPrompt': 'Walk me through the directory structure and core modules of this repo',
  'bench.welcome.suggest.fixTitle': 'Fix an error',
  'bench.welcome.suggest.fixPrompt': 'I hit an error — help me locate and fix it:',
  'bench.welcome.suggest.testTitle': 'Write unit tests',
  'bench.welcome.suggest.testPrompt': 'Write a set of unit tests for the following function:',
  'bench.welcome.suggest.reviewTitle': 'Code review',
  'bench.welcome.suggest.reviewPrompt': 'Review this code and point out issues and improvements:',

  // Welcome: composer and errors
  'bench.welcome.promptProjectPath': 'Enter the project directory path (no native picker in browser preview)',
  'bench.welcome.error.noStreamableAgent': 'No agent supports GUI chat (must be installed and support streaming)',
  'bench.welcome.error.selectProjectDir': 'Select a project directory first',
  'bench.welcome.selectProjectDir': 'Select project directory',
  'bench.welcome.browseFolders': 'Browse folders…',
  'bench.welcome.composerPlaceholder': 'Ask the agent about your task… (Enter to send, Shift+Enter for a new line)',
  'bench.welcome.addAttachment': 'Add attachment (coming soon)',
  'bench.welcome.selectAgent': 'Select agent',
  'bench.welcome.agentTitle': 'Agent (must support GUI streaming)',
  'bench.welcome.selectModel': 'Select model',
  'bench.welcome.effort': 'Reasoning effort',
  'bench.welcome.send': 'Send and create session',

  // Sidebar
  'bench.sidebar.removeFromList': 'Remove from list',
  'bench.sidebar.error.agentNotCapable': '{agent} is not installed or does not support GUI chat',
  'bench.sidebar.error.resumeFailed': 'Failed to resume streaming session for {agent} ({msg})',
  'bench.sidebar.newSession': 'New session',
  'bench.sidebar.recallSearchTitle': 'Recall full-text search',
  'bench.sidebar.searchPlaceholder': 'Search sessions…',
  'bench.sidebar.liveSessions': 'Live sessions',
  'bench.sidebar.noLiveSessions': 'No active sessions',
  'bench.sidebar.sessionsByProject': 'Sessions (by project)',
  'bench.sidebar.noMatch': 'No matching sessions',
  'bench.sidebar.empty': 'No sessions yet — start your first conversation above',

  // New session sheet
  'bench.newSession.notGuiCapable': 'GUI chat not supported yet',
  'bench.newSession.notInstalled': 'Not installed',
  'bench.newSession.guiStreamingVersioned': 'v{ver} · GUI streaming',
  'bench.newSession.guiStreaming': 'GUI streaming',
  'bench.newSession.title': 'New agent session',
  'bench.newSession.agentsLabel': 'Agents (GUI streaming)',
  'bench.newSession.scanning': 'Scanning installed agents…',
  'bench.newSession.projectDir': 'Project directory',
  'bench.newSession.modelEffort': 'Model / reasoning effort',
  'bench.newSession.firstMessage': 'First message (optional)',
  'bench.newSession.firstMessagePlaceholder': 'Send immediately after creation…',
  'bench.newSession.create': 'Create session',

  // Message role badges
  'bench.role.user': 'User',
  'bench.role.assistant': 'Agent',
  'bench.role.thinking': 'Thinking',
  'bench.role.tool': 'Tool',
  'bench.role.system': 'System',

  // Recall full-text search
  'bench.recall.back': 'Back',
  'bench.recall.title': 'Recall · Session full-text search',
  'bench.recall.indexStatus': '{sessions} sessions · {messages} messages · {kb} KB',
  'bench.recall.reindexTitle': 'Rebuild index',
  'bench.recall.reindex': 'Rebuild',
  'bench.recall.searchPlaceholder': 'Search history across all agents…',
  'bench.recall.searching': 'Searching…',
  'bench.recall.noHits': 'No matches for "{query}"',
  'bench.recall.backToResults': 'Back to results',
  'bench.recall.hint': 'Type keywords to search the history of all agents, including Claude and Codex',

  // Chat view
  'bench.chat.thinking': 'Thinking…',
  'bench.chat.thought': 'Thought',
  'bench.chat.running': 'Running',
  'bench.chat.endSessionTitle': 'End session',
  'bench.chat.end': 'End',
  'bench.chat.emptyTitle': 'Start chatting with a coding agent',
  'bench.chat.emptyHint': 'Pick an agent and a project directory, then send your first message',
  'bench.chat.composerPlaceholder': 'Message the agent… (Enter to send, Shift+Enter for a new line)',

  // Terminal (PTY)
  'bench.terminal.createFailed': 'Failed to create session: {msg}',

  // Hooks (non-component context)
  'bench.hooks.defaultSessionName': '{agent} session',
};
