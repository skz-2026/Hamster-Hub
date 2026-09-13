import type { home as homeZh } from '../zh-CN/home';

export const home: Record<keyof typeof homeZh, string> = {
  // ===== Home: search / loading / empty states =====
  'home.search.placeholder': 'Search apps',
  'home.search.noMatch': 'No apps match “{query}”',
  'home.loading.indexing': 'Indexing local apps…',
  'home.empty.noApps': 'No apps to show',
  'home.empty.monogram': 'H',
  'home.pageN': 'Page {n}',
  'home.dock.dropHint': 'Drag favorite apps here',

  // ===== Home: edit / navigation actions =====
  'home.action.cycleWallpaper': 'Change wallpaper',
  'home.action.addWidget': 'Add widget',
  'home.action.removeWidget': 'Remove widget',
  'home.action.done': 'Done',
  'home.action.arrange': 'Arrange',
  'home.action.arrangeHint': 'Long-press icons to arrange Home',
  'home.action.backToDesktop': 'Back to Desktop',
  'home.action.exitDesktopMode': 'Exit Desktop Mode',
  'home.nav.desktop': 'Desktop',
  'home.nav.exit': 'Exit',
  'home.action.remove': 'Remove',
  'home.action.deleteFolder': 'Delete folder',
  'home.action.renameFolder': 'Rename folder',

  // ===== Home: AI organizer floating ball =====
  'home.ball.title': 'AI organizer',
  'home.ball.placeholder': 'Give an instruction, e.g. "Group icons into folders by purpose"',
  'home.ball.send': 'Send',
  'home.ball.stop': 'Stop',
  'home.ball.collapse': 'Collapse',
  'home.ball.done': 'Done — home refreshed with the new layout',
  'home.ball.again': 'New instruction',
  'home.ball.goConfig': 'Set up an agent',
  'home.ball.newChat': 'Reset chat',
  'home.ball.restored': 'Reconnected to your last chat — keep asking',

  // ===== Home: widget type labels (explicit mapping, never build keys) =====
  'home.widget.clock': 'Clock',
  'home.widget.weather': 'Weather',
  'home.widget.todo': 'To-do',
  'home.widget.countdown': 'Countdown',
  'home.widget.sysinfo': 'System status',
  'home.widget.taskmgr': 'Task manager',

  // ===== Weather widget =====
  'home.weather.unavailable': 'Weather unavailable',

  // ===== To-do (widget + card) =====
  'home.todo.title': 'To-do',
  'home.todo.summary': 'To-do · {n} open',
  'home.todo.allDone': 'All done 🎉',
  'home.todo.undone': '{n} open',
  'home.todo.addPlaceholder': 'Add a to-do, try “rent due 3pm tomorrow”',
  'home.todo.add': 'Add',
  'home.todo.remindChip': 'Reminder when due',
  'home.todo.empty': 'No to-dos yet, add one',
  'home.todo.delete': 'Delete',

  // ===== Countdown widget =====
  'home.countdown.empty': 'No countdowns yet',
  'home.countdown.today': 'Today',
  'home.countdown.days.one': '{n} day',
  'home.countdown.days.other': '{n} days',

  // ===== System status widget =====
  'home.sysinfo.unavailable': 'System status unavailable',
  'home.sysinfo.memory': 'RAM {used}/{total}GB',
  'home.sysinfo.cores.one': '{n} core',
  'home.sysinfo.cores.other': '{n} cores',
  'home.sysinfo.barTitle': '{label} {value} ({percent}%)',

  // ===== Task manager widget =====
  'home.taskmgr.loading': 'Loading processes…',
  'home.taskmgr.title': 'Task manager · By memory',
  'home.taskmgr.hoverHint': 'Hover ✕ to end a process',
  'home.taskmgr.noProcesses': 'No process data',
  'home.taskmgr.endProcess': 'End {name} ({pid})',
  'home.taskmgr.endFailed': 'Failed to end (try running as admin)',

  // ===== Control center =====
  'home.cc.bluetooth': 'Bluetooth',
  'home.cc.systemSettings': 'System Settings',
  'home.cc.darkMode': 'Dark mode',
  'home.cc.lightMode': 'Light mode',
  'home.cc.pixelTheme': 'Pixel theme',
  'home.cc.tapToSwitch': 'Click to switch',
  'home.cc.tapToCycle': 'Click to cycle',
  'home.cc.volume': 'Volume',
  'home.cc.aiAssistant': 'AI assistant',
  'home.cc.settings': 'Settings',
  'home.cc.exitTakeover': 'Exit takeover',

  // ===== To-do: due / repeat / reminder (TodoItem / DueEditor / ReminderDialog / due.ts) =====
  'home.todo.recur.daily': 'Daily',
  'home.todo.recur.weekly': 'Weekly',
  'home.todo.recur.monthly': 'Monthly',
  'home.todo.recur.weekdays': 'Weekdays',
  'home.todo.recur.none': 'No repeat',
  'home.todo.recur.badge': 'Repeats: {rule}',
  'home.todo.toggleDone': 'Toggle done',
  'home.todo.toggleDoneRecur': 'Complete and roll to next',
  'home.todo.setDue': 'Set due date',
  'home.todo.addDue': '+ due',
  'home.todo.remind': 'Remind',
  'home.todo.remindOn': 'Reminder: on',
  'home.todo.remindOff': 'Reminder: off',
  'home.todo.clearDue': 'Clear due date',
  'home.todo.clear': 'Clear',
  'home.todo.save': 'Save',
  'home.reminder.title': 'To-do reminder',
  'home.reminder.ok': 'Got it',
  'home.reminder.done': 'Mark done',
  'home.reminder.more': '{n} more due',
  'home.due.today': 'Today',
  'home.due.tomorrow': 'Tomorrow',
  'home.due.yesterday': 'Yesterday',
  'home.due.overdue': 'Overdue · {label}',
};
