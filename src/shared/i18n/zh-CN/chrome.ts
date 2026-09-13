export const chrome = {
  // 导航标签（侧栏 / 任务栏 / 桌面快捷入口共用）
  'chrome.nav.workbench': '工作台',
  'chrome.nav.search': '搜索',
  'chrome.nav.schedule': '日程',
  'chrome.nav.apps': '应用',
  'chrome.nav.files': '文件',
  'chrome.nav.vault': '密码箱',
  'chrome.nav.agent': 'AI Agent',
  'chrome.nav.settings': '设置',
  'chrome.nav.home': '主屏',
  'chrome.nav.desktop': '桌面',
  'chrome.nav.desktopMode': '桌面模式',

  // 通用动作
  'chrome.action.enterDesktop': '进入桌面模式',
  'chrome.action.exitDesktop': '退出桌面模式',
  'chrome.action.start': '开始',
  'chrome.action.addToDock': '添加到任务栏',
  'chrome.action.addAppToDock': '添加应用到任务栏',
  'chrome.action.remove': '移除',
  'chrome.action.cancel': '取消',
  'chrome.action.close': '关闭',

  // 标题栏
  'chrome.titlebar.hideToTray': '隐藏到托盘',
  'chrome.titlebar.minimize': '最小化',
  'chrome.titlebar.pin': '窗口置顶',
  'chrome.titlebar.unpin': '取消置顶',
  'chrome.titlebar.coreVersion': '核心 v{v}',
  'chrome.app.name': '仓鼠Hub',

  // 应用外壳（返回桌面胶囊）
  'chrome.shell.backToDesktop': '返回桌面',
  'chrome.shell.backToDesktopTitle': '返回桌面主页（Esc）',

  // 托盘
  'chrome.tray.trayAndApps': '托盘与应用',

  // 任务栏
  'chrome.dock.customZone': '定制区',
  'chrome.dock.closeWindow': '关闭窗口',
  'chrome.dock.newInstance': '多开应用',

  // 应用选择器弹层
  'chrome.picker.searchApps': '搜索应用',
  'chrome.picker.dockFull': '任务栏定制区已满（{n} 个）——右键图标可移除后再添加',
  'chrome.picker.noMatch': '没有匹配「{q}」的应用',
  'chrome.picker.noApps': '没有可添加的应用',
  'chrome.picker.addTitle': '添加 {name}',

  // 下拉选择
  'chrome.select.noOptions': '暂无可选项',

  // 桌面主页
  'chrome.desktop.controlCenter': '控制中心',
  'chrome.desktop.welcomeBack': '{greeting}，欢迎回来',
  'chrome.desktop.searchPlaceholder': '搜索应用、文件…',
  'chrome.desktop.pluginTitle': '插件：{name}',
  'chrome.desktop.aiAssistant': 'AI 助手',

  // 浏览器预览调试条
  'chrome.debug.browserPreview': '浏览器预览 · IPC mock',

  // 里程碑占位页
  'chrome.stub.milestoneLive': '{milestone} 里程碑上线',
} as const;
