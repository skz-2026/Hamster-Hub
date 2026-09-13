import type { home as homeZh } from '../zh-CN/home';

/** 繁体中文译法需符合台湾习惯用词（如 设置→設定、文件→檔案、网络→網路） */
export const home: Record<keyof typeof homeZh, string> = {
  // ===== 主屏：搜尋 / 載入 / 空白狀態 =====
  'home.search.placeholder': '搜尋應用程式',
  'home.search.noMatch': '沒有符合「{query}」的應用程式',
  'home.loading.indexing': '正在索引本機應用程式…',
  'home.empty.noApps': '沒有找到可顯示的應用程式',
  'home.empty.monogram': '倉',
  'home.pageN': '第 {n} 頁',
  'home.dock.dropHint': '拖入常用應用程式',

  // ===== 主屏：編輯 / 導覽動作 =====
  'home.action.cycleWallpaper': '切換桌布',
  'home.action.addWidget': '新增小工具',
  'home.action.removeWidget': '移除小工具',
  'home.action.done': '完成',
  'home.action.arrange': '整理',
  'home.action.arrangeHint': '長按圖示整理主畫面',
  'home.action.backToDesktop': '返回桌面',
  'home.action.exitDesktopMode': '結束桌面模式',
  'home.nav.desktop': '桌面',
  'home.nav.exit': '結束',
  'home.action.remove': '移除',
  'home.action.deleteFolder': '刪除資料夾',

  // ===== 主屏：小工具類型標籤（顯式映射，禁止拼 key） =====
  'home.widget.clock': '時鐘',
  'home.widget.weather': '天氣',
  'home.widget.todo': '待辦',
  'home.widget.countdown': '倒數日',
  'home.widget.sysinfo': '電腦狀態',
  'home.widget.taskmgr': '工作管理員',

  // ===== 天氣小工具 =====
  'home.weather.unavailable': '天氣不可用',

  // ===== 待辦（小工具 + 待辦卡） =====
  'home.todo.title': '待辦',
  'home.todo.summary': '待辦 · {n} 未完成',
  'home.todo.allDone': '全部完成 🎉',
  'home.todo.undone': '未完成 {n}',
  'home.todo.addPlaceholder': '新增待辦，試試「明天下午3點交房租」',
  'home.todo.add': '新增',
  'home.todo.remindChip': '到點提醒',
  'home.todo.empty': '尚無待辦，加一條吧',
  'home.todo.delete': '刪除',

  // ===== 倒數日小工具 =====
  'home.countdown.empty': '尚無倒數日',
  'home.countdown.today': '今天',
  'home.countdown.days.one': '{n}天',
  'home.countdown.days.other': '{n}天',

  // ===== 電腦狀態小工具 =====
  'home.sysinfo.unavailable': '系統狀態不可用',
  'home.sysinfo.memory': '記憶體 {used}/{total}GB',
  'home.sysinfo.cores.one': '{n} 核',
  'home.sysinfo.cores.other': '{n} 核',
  'home.sysinfo.barTitle': '{label} {value}（{percent}%）',

  // ===== 工作管理員小工具 =====
  'home.taskmgr.loading': '讀取處理程序…',
  'home.taskmgr.title': '工作管理員 · 按記憶體',
  'home.taskmgr.hoverHint': '結束處理程序請懸停 ✕',
  'home.taskmgr.noProcesses': '無處理程序資料',
  'home.taskmgr.endProcess': '結束 {name} ({pid})',
  'home.taskmgr.endFailed': '結束失敗（試試以系統管理員執行）',

  // ===== 控制中心 =====
  'home.cc.bluetooth': '藍牙',
  'home.cc.systemSettings': '系統設定',
  'home.cc.darkMode': '深色模式',
  'home.cc.lightMode': '淺色模式',
  'home.cc.pixelTheme': '像素主題',
  'home.cc.tapToSwitch': '點擊切換',
  'home.cc.tapToCycle': '點擊輪換',
  'home.cc.volume': '音量',
  'home.cc.aiAssistant': 'AI 助理',
  'home.cc.settings': '設定',
  'home.cc.exitTakeover': '結束接管',

  // ===== 待辦：截止 / 循環 / 提醒（TodoItem / DueEditor / ReminderToast / due.ts） =====
  'home.todo.recur.daily': '每天',
  'home.todo.recur.weekly': '每週',
  'home.todo.recur.monthly': '每月',
  'home.todo.recur.weekdays': '工作日',
  'home.todo.recur.none': '不重複',
  'home.todo.recur.badge': '循環：{rule}',
  'home.todo.toggleDone': '切換完成',
  'home.todo.toggleDoneRecur': '完成並前進到下一次',
  'home.todo.setDue': '設定截止時間',
  'home.todo.addDue': '+ 截止',
  'home.todo.remind': '提醒',
  'home.todo.remindOn': '提醒：開',
  'home.todo.remindOff': '提醒：關',
  'home.todo.clearDue': '清除截止時間',
  'home.todo.clear': '清除',
  'home.todo.save': '儲存',
  'home.reminder.title': '待辦提醒',
  'home.reminder.ok': '知道了',
  'home.due.today': '今天',
  'home.due.tomorrow': '明天',
  'home.due.yesterday': '昨天',
  'home.due.overdue': '已逾期 · {label}',
};
