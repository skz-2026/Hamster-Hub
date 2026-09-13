import type { pages as pagesZh } from '../zh-CN/pages';

export const pages: Record<keyof typeof pagesZh, string> = {
  // Workbench
  'pages.workbench.motd': 'Have a productive day',
  'pages.workbench.weather': 'Weather',
  'pages.workbench.weatherError': 'Failed to load weather',
  'pages.workbench.weatherRetry': 'Will retry automatically',
  'pages.workbench.tempRange': 'Low {min}° · High {max}°',
  'pages.workbench.countdown': 'Countdowns',
  'pages.workbench.countdownEmpty': 'No countdowns yet',
  'pages.workbench.countdownToday': 'Today',
  'pages.workbench.countdownDays': '{days} days',
  'pages.workbench.recentFiles': 'Recent files',
  'pages.workbench.fileCount': '{count}',
  'pages.workbench.recentFilesEmpty': 'No recent files',
  'pages.workbench.topApps': 'Top apps',
  'pages.workbench.byFrequency': 'By usage frequency',
  'pages.workbench.topAppsEmptyHint1': 'Launch a few apps and',
  'pages.workbench.topAppsEmptyHint2': 'your top list will show up here',
  'pages.workbench.lunarDate': 'Lunar {month}/{day}',
  'pages.workbench.lunarFallback': 'Lunar calendar',

  // Time-of-day greetings
  'pages.greeting.lateNight': 'Working late',
  'pages.greeting.earlyMorning': 'Good morning',
  'pages.greeting.morning': 'Good morning',
  'pages.greeting.noon': 'Good afternoon',
  'pages.greeting.afternoon': 'Good afternoon',
  'pages.greeting.evening': 'Good evening',

  // Weekdays (Monday first, used with a literal key array)
  'pages.weekday.mon': 'Mon',
  'pages.weekday.tue': 'Tue',
  'pages.weekday.wed': 'Wed',
  'pages.weekday.thu': 'Thu',
  'pages.weekday.fri': 'Fri',
  'pages.weekday.sat': 'Sat',
  'pages.weekday.sun': 'Sun',

  // Schedule
  'pages.schedule.title': 'Schedule',
  'pages.schedule.monthTitle': '{month}/{year}',
  'pages.schedule.thisMonth': 'This month',
  'pages.schedule.countdown': 'Countdowns',
  'pages.schedule.eventPlaceholder': 'Event name',
  'pages.schedule.countdownHint': 'Add a custom countdown and it will be marked on the calendar',
  'pages.schedule.today': 'Today',
  'pages.schedule.daysLater': '{days} days',
  'pages.schedule.expired': 'Expired',
  'pages.schedule.notes': 'Notes',
  'pages.schedule.noteCount': '{count}',
  'pages.schedule.notePlaceholder': 'Jot down a note',
  'pages.schedule.notesEmpty': 'No notes yet',
  'pages.schedule.doubleClickEdit': 'Double-click to edit',
  'pages.schedule.pin': 'Pin',
  'pages.schedule.unpin': 'Unpin',
  'pages.schedule.delete': 'Delete',

  // Files
  'pages.files.title': 'Files',
  'pages.files.reindex': 'Rebuild index',
  'pages.files.reindexTitle': 'Rescan the whole file index (the watcher keeps it up to date)',
  'pages.files.searchPlaceholder': 'Search files by name / pinyin…',
  'pages.files.all': 'All',
  'pages.files.kindFile': 'File',
  'pages.files.noMatch': 'No files matching "{q}"',
  'pages.files.emptyIndex':
    'The index is empty — click "Rebuild index" in the top right to scan (scope in settings: file_index.roots)',
  'pages.files.reveal': 'Show in File Explorer',

  // Apps
  'pages.apps.title': 'Apps',
  'pages.apps.indexing': 'Indexing…',
  'pages.apps.summary': '{count} apps · click to launch',
  'pages.apps.emptyCategory': 'No apps in this category',

  // App categories
  'pages.cat.all': 'All',
  'pages.cat.communication': 'Communication',
  'pages.cat.office': 'Office',
  'pages.cat.dev': 'Development',
  'pages.cat.entertainment': 'Entertainment',
  'pages.cat.tools': 'Tools',
  'pages.cat.system': 'System',
  'pages.cat.other': 'Other',
};
