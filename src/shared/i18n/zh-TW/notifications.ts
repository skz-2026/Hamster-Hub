/** 通知中心文案（繁體中文，台灣用語；key 與 zh-CN 完全一致） */
export const notifications = {
  // —— 面板 ——
  'notifications.title': '通知中心',
  'notifications.unread': '{n} 則未讀',
  'notifications.markAllRead': '全部已讀',
  'notifications.clear': '清空',
  'notifications.dismiss': '移除通知',
  'notifications.empty.title': '暫無通知',
  'notifications.empty.desc': '待辦到期、AI Agent 任務完成、更新就緒都會出現在這裡',

  // —— 類別標題 ——
  'notifications.kind.todo': '待辦提醒',
  'notifications.kind.focus': '專注結束',
  'notifications.kind.agent': 'AI Agent 任務完成',
  'notifications.kind.update': '更新就緒',

  // —— 類別內文 ——
  'notifications.todo.body': '待辦已到提醒時間',
  'notifications.focus.body': '一輪專注已結束，起來活動一下',
  'notifications.break.body': '休息結束，回到專注吧',
  'notifications.agent.body': '工作階段已結束，點擊回到工作台',
  'notifications.update.body': '新版本已下載，點擊前往設定查看',
  'notifications.demo.body': '示範通知：這是一則範例提醒',

  // —— 相對時間 ——
  'notifications.ago.now': '剛剛',
  'notifications.ago.min': '{n} 分鐘前',
  'notifications.ago.hour': '{n} 小時前',
  'notifications.ago.day': '{n} 天前',

  // —— 分組 ——
  'notifications.day.today': '今天',
  'notifications.day.yesterday': '昨天',
  'notifications.day.earlier': '更早',
} as const;
