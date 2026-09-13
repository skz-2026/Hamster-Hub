import type { chrome as chromeZh } from '../zh-CN/chrome';

/** 繁体中文译法需符合台湾习惯用词（如 设置→設定、文件→檔案、网络→網路） */
export const chrome: Record<keyof typeof chromeZh, string> = {
  'chrome.nav.workbench': '工作台',
  'chrome.nav.search': '搜尋',
  'chrome.nav.schedule': '行程',
  'chrome.nav.apps': '應用程式',
  'chrome.nav.files': '檔案',
  'chrome.nav.vault': '密碼箱',
  'chrome.nav.agent': 'AI Agent',
  'chrome.nav.settings': '設定',
  'chrome.nav.home': '主畫面',
  'chrome.nav.desktop': '桌面',
  'chrome.nav.desktopMode': '桌面模式',

  'chrome.action.enterDesktop': '進入桌面模式',
  'chrome.action.exitDesktop': '離開桌面模式',
  'chrome.action.start': '開始',
  'chrome.action.addToDock': '新增到工作列',
  'chrome.action.addAppToDock': '新增應用程式到工作列',
  'chrome.action.remove': '移除',
  'chrome.action.cancel': '取消',
  'chrome.action.close': '關閉',

  'chrome.titlebar.hideToTray': '隱藏到通知區',
  'chrome.titlebar.minimize': '最小化',
  'chrome.titlebar.pin': '視窗置頂',
  'chrome.titlebar.unpin': '取消置頂',
  'chrome.titlebar.coreVersion': '核心 v{v}',
  'chrome.app.name': '倉鼠Hub',

  'chrome.shell.backToDesktop': '返回桌面',
  'chrome.shell.backToDesktopTitle': '返回桌面首頁（Esc）',

  'chrome.tray.trayAndApps': '通知區與應用程式',

  'chrome.dock.customZone': '自訂區',
  'chrome.dock.closeWindow': '關閉視窗',
  'chrome.dock.newInstance': '開新視窗',

  'chrome.picker.searchApps': '搜尋應用程式',
  'chrome.picker.dockFull': '工作列自訂區已滿（{n} 個）——右鍵圖示可移除後再新增',
  'chrome.picker.noMatch': '沒有符合「{q}」的應用程式',
  'chrome.picker.noApps': '沒有可新增的應用程式',
  'chrome.picker.addTitle': '新增 {name}',

  'chrome.select.noOptions': '暫無可選項',

  'chrome.desktop.controlCenter': '控制中心',
  'chrome.desktop.welcomeBack': '{greeting}，歡迎回來',
  'chrome.desktop.searchPlaceholder': '搜尋應用程式、檔案…',
  'chrome.desktop.pluginTitle': '外掛程式：{name}',
  'chrome.desktop.aiAssistant': 'AI 助理',

  'chrome.debug.browserPreview': '瀏覽器預覽 · IPC mock',

  'chrome.stub.milestoneLive': '{milestone} 里程碑上線',
};
