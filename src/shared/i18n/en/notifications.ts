/** Notification center copy (English; keys must match zh-CN exactly) */
export const notifications = {
  // —— Panel ——
  'notifications.title': 'Notification Center',
  'notifications.unread': '{n} unread',
  'notifications.markAllRead': 'Mark all read',
  'notifications.clear': 'Clear all',
  'notifications.dismiss': 'Remove notification',
  'notifications.empty.title': 'No notifications',
  'notifications.empty.desc': 'To-do reminders, AI Agent results and update notices all land here',

  // —— Kind titles ——
  'notifications.kind.todo': 'To-do reminder',
  'notifications.kind.focus': 'Focus finished',
  'notifications.kind.agent': 'AI Agent task finished',
  'notifications.kind.update': 'Update ready',

  // —— Kind bodies ——
  'notifications.todo.body': 'This to-do is due',
  'notifications.focus.body': 'A focus round has ended — take a short break',
  'notifications.break.body': 'Break over — back to focus',
  'notifications.agent.body': 'The session has ended; click to return to the workbench',
  'notifications.update.body': 'A new version has been downloaded; click to open Settings',
  'notifications.demo.body': 'Demo notification — a sample entry',

  // —— Relative time ——
  'notifications.ago.now': 'just now',
  'notifications.ago.min': '{n} min ago',
  'notifications.ago.hour': '{n} h ago',
  'notifications.ago.day': '{n} d ago',

  // —— Day groups ——
  'notifications.day.today': 'Today',
  'notifications.day.yesterday': 'Yesterday',
  'notifications.day.earlier': 'Earlier',
} as const;
