/** 通知中心文案（基准词典，zh-TW / en 的 key 必须与之完全一致） */
export const notifications = {
  // —— 面板 ——
  'notifications.title': '通知中心',
  'notifications.unread': '{n} 条未读',
  'notifications.markAllRead': '全部已读',
  'notifications.clear': '清空',
  'notifications.dismiss': '移除通知',
  'notifications.empty.title': '暂无通知',
  'notifications.empty.desc': '待办到点、AI Agent 任务完成、更新就绪都会出现在这里',

  // —— 类别标题 ——
  'notifications.kind.todo': '待办提醒',
  'notifications.kind.focus': '专注结束',
  'notifications.kind.agent': 'AI Agent 任务完成',
  'notifications.kind.update': '更新就绪',

  // —— 类别正文 ——
  'notifications.todo.body': '待办已到提醒时间',
  'notifications.focus.body': '一轮专注已结束，起来活动一下',
  'notifications.break.body': '休息结束，回到专注吧',
  'notifications.agent.body': '会话已结束，点击回到工作台',
  'notifications.update.body': '新版本已下载，点击前往设置查看',
  'notifications.demo.body': '演示通知：这是一条样例提醒',

  // —— 相对时间 ——
  'notifications.ago.now': '刚刚',
  'notifications.ago.min': '{n} 分钟前',
  'notifications.ago.hour': '{n} 小时前',
  'notifications.ago.day': '{n} 天前',

  // —— 分组 ——
  'notifications.day.today': '今天',
  'notifications.day.yesterday': '昨天',
  'notifications.day.earlier': '更早',
} as const;
