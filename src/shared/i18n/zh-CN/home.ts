/**
 * home 命名空间（基准词典）：主屏 HomeScreen / HomeWidget / AppIcon、
 * 控制中心 ControlCenter、待办卡 TodoCard 的用户可见文案。
 * en / zh-TW 的 home 词典必须包含完全相同的 key（编译期强制）。
 */
export const home = {
  // ===== 主屏：搜索 / 加载 / 空态 =====
  'home.search.placeholder': '搜索应用',
  'home.search.noMatch': '没有匹配「{query}」的应用',
  'home.loading.indexing': '正在索引本机应用…',
  'home.empty.noApps': '没有找到可显示的应用',
  'home.empty.monogram': '仓',
  'home.pageN': '第 {n} 页',
  'home.dock.dropHint': '拖入常用应用',

  // ===== 主屏：编辑 / 导航动作 =====
  'home.action.cycleWallpaper': '切换壁纸',
  'home.action.addWidget': '添加小组件',
  'home.action.removeWidget': '移除小组件',
  'home.action.done': '完成',
  'home.action.arrange': '整理',
  'home.action.arrangeHint': '长按图标整理主屏',
  'home.action.backToDesktop': '返回桌面',
  'home.action.exitDesktopMode': '退出桌面模式',
  'home.nav.desktop': '桌面',
  'home.nav.exit': '退出',
  'home.action.remove': '移除',
  'home.action.deleteFolder': '删除文件夹',

  // ===== 主屏：小组件类型标签（显式映射，禁止拼 key） =====
  'home.widget.clock': '时钟',
  'home.widget.weather': '天气',
  'home.widget.todo': '待办',
  'home.widget.countdown': '倒数日',
  'home.widget.sysinfo': '电脑状态',
  'home.widget.taskmgr': '任务管理器',

  // ===== 天气小组件 =====
  'home.weather.unavailable': '天气不可用',

  // ===== 待办（小组件 + 待办卡） =====
  'home.todo.title': '待办',
  'home.todo.summary': '待办 · {n} 未完成',
  'home.todo.allDone': '全部完成 🎉',
  'home.todo.undone': '未完成 {n}',
  'home.todo.addPlaceholder': '添加待办，试试「明天下午3点交房租」',
  'home.todo.add': '添加',
  'home.todo.remindChip': '到点提醒',
  'home.todo.empty': '暂无待办，加一条吧',
  'home.todo.delete': '删除',

  // ===== 倒数日小组件 =====
  'home.countdown.empty': '暂无倒数日',
  'home.countdown.today': '今天',
  'home.countdown.days.one': '{n}天',
  'home.countdown.days.other': '{n}天',

  // ===== 电脑状态小组件 =====
  'home.sysinfo.unavailable': '系统状态不可用',
  'home.sysinfo.memory': '内存 {used}/{total}GB',
  'home.sysinfo.cores.one': '{n} 核',
  'home.sysinfo.cores.other': '{n} 核',
  'home.sysinfo.barTitle': '{label} {value}（{percent}%）',

  // ===== 任务管理器小组件 =====
  'home.taskmgr.loading': '读取进程…',
  'home.taskmgr.title': '任务管理器 · 按内存',
  'home.taskmgr.hoverHint': '结束进程请悬停 ✕',
  'home.taskmgr.noProcesses': '无进程数据',
  'home.taskmgr.endProcess': '结束 {name} ({pid})',
  'home.taskmgr.endFailed': '结束失败（试试管理员运行）',

  // ===== 控制中心 =====
  'home.cc.bluetooth': '蓝牙',
  'home.cc.systemSettings': '系统设置',
  'home.cc.darkMode': '深色模式',
  'home.cc.lightMode': '浅色模式',
  'home.cc.pixelTheme': '像素主题',
  'home.cc.tapToSwitch': '点击切换',
  'home.cc.tapToCycle': '点击轮换',
  'home.cc.volume': '音量',
  'home.cc.aiAssistant': 'AI 助手',
  'home.cc.settings': '设置',
  'home.cc.exitTakeover': '退出接管',

  // ===== 待办：截止 / 循环 / 提醒（TodoItem / DueEditor / ReminderToast / due.ts） =====
  'home.todo.recur.daily': '每天',
  'home.todo.recur.weekly': '每周',
  'home.todo.recur.monthly': '每月',
  'home.todo.recur.weekdays': '工作日',
  'home.todo.recur.none': '不重复',
  'home.todo.recur.badge': '循环：{rule}',
  'home.todo.toggleDone': '切换完成',
  'home.todo.toggleDoneRecur': '完成并滚动到下一次',
  'home.todo.setDue': '设置截止时间',
  'home.todo.addDue': '+ 截止',
  'home.todo.remind': '提醒',
  'home.todo.remindOn': '提醒：开',
  'home.todo.remindOff': '提醒：关',
  'home.todo.clearDue': '清除截止时间',
  'home.todo.clear': '清除',
  'home.todo.save': '保存',
  'home.reminder.title': '待办提醒',
  'home.reminder.ok': '知道了',
  'home.due.today': '今天',
  'home.due.tomorrow': '明天',
  'home.due.yesterday': '昨天',
  'home.due.overdue': '已逾期 · {label}',
} as const;
