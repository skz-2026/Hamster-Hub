import type { chrome as chromeZh } from '../zh-CN/chrome';

export const chrome: Record<keyof typeof chromeZh, string> = {
  'chrome.nav.workbench': 'Workbench',
  'chrome.nav.search': 'Search',
  'chrome.nav.schedule': 'Schedule',
  'chrome.nav.apps': 'Apps',
  'chrome.nav.files': 'Files',
  'chrome.nav.vault': 'Vault',
  'chrome.nav.agent': 'Agent',
  'chrome.nav.settings': 'Settings',
  'chrome.nav.home': 'Home',
  'chrome.nav.desktop': 'Desktop',
  'chrome.nav.desktopMode': 'Desktop Mode',

  'chrome.action.enterDesktop': 'Enter desktop mode',
  'chrome.action.exitDesktop': 'Exit desktop mode',
  'chrome.action.start': 'Start',
  'chrome.action.addToDock': 'Add to taskbar',
  'chrome.action.addAppToDock': 'Add app to taskbar',
  'chrome.action.remove': 'Remove',
  'chrome.action.cancel': 'Cancel',
  'chrome.action.close': 'Close',

  'chrome.titlebar.hideToTray': 'Hide to tray',
  'chrome.titlebar.minimize': 'Minimize',
  'chrome.titlebar.pin': 'Pin window',
  'chrome.titlebar.unpin': 'Unpin window',
  'chrome.titlebar.coreVersion': 'Core v{v}',
  'chrome.app.name': 'HamsterHub',

  'chrome.shell.backToDesktop': 'Back to desktop',
  'chrome.shell.backToDesktopTitle': 'Back to desktop home (Esc)',

  'chrome.tray.trayAndApps': 'Tray and apps',

  'chrome.dock.customZone': 'Custom area',
  'chrome.dock.closeWindow': 'Close window',
  'chrome.dock.newInstance': 'New window',
  'chrome.dock.pickWindow': 'Pick a window to bring to front',
  'chrome.dock.minimized': 'Minimized',

  'chrome.picker.searchApps': 'Search apps',
  'chrome.picker.dockFull': 'Custom area is full ({n} apps) — right-click an icon to remove one first',
  'chrome.picker.noMatch': 'No apps matching "{q}"',
  'chrome.picker.noApps': 'No apps to add',
  'chrome.picker.addTitle': 'Add {name}',

  'chrome.select.noOptions': 'No options yet',

  'chrome.desktop.controlCenter': 'Control center',
  'chrome.desktop.welcomeBack': '{greeting}, welcome back',
  'chrome.desktop.searchPlaceholder': 'Search apps, files…',
  'chrome.desktop.pluginTitle': 'Plugin: {name}',
  'chrome.desktop.aiAssistant': 'AI assistant',

  'chrome.debug.browserPreview': 'Browser preview · IPC mock',
  'chrome.debug.notify': 'Fire a notification',

  'chrome.stub.milestoneLive': '{milestone} milestone is live',
};
