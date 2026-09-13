import type { bench as benchZh } from '../zh-CN/bench';

/** 繁体中文译法需符合台湾习惯用词（如 设置→設定、文件→檔案、网络→網路） */
export const bench: Record<keyof typeof benchZh, string> = {
  // 通用
  'bench.common.loading': '載入中…',
  'bench.common.cancel': '取消',

  // 相對時間
  'bench.time.justNow': '剛剛',
  'bench.time.minutesAgo': '{n} 分鐘前',
  'bench.time.hoursAgo': '{n} 小時前',
  'bench.time.daysAgo': '{n} 天前',

  // 歡迎頁：時段問候語
  'bench.welcome.greeting.night': '夜深了，有什麼想讓我幫忙的嗎',
  'bench.welcome.greeting.earlyMorning': '早呀，有什麼想讓我幫忙的嗎',
  'bench.welcome.greeting.morning': '早安呀，有什麼想讓我幫忙的嗎',
  'bench.welcome.greeting.noon': '午安，有什麼想讓我幫忙的嗎',
  'bench.welcome.greeting.afternoon': '午安，有什麼想讓我幫忙的嗎',
  'bench.welcome.greeting.evening': '晚上好，有什麼想讓我幫忙的嗎',

  // 歡迎頁：快捷 prompt 卡片
  'bench.welcome.suggest.repoTitle': '梳理儲存庫結構',
  'bench.welcome.suggest.repoPrompt': '幫我梳理一下這個儲存庫的目錄結構和核心模組',
  'bench.welcome.suggest.fixTitle': '修復錯誤',
  'bench.welcome.suggest.fixPrompt': '遇到一個錯誤，幫我定位並修復：',
  'bench.welcome.suggest.testTitle': '寫單元測試',
  'bench.welcome.suggest.testPrompt': '為下面的函式補一組單元測試：',
  'bench.welcome.suggest.reviewTitle': '程式碼審查',
  'bench.welcome.suggest.reviewPrompt': '幫我審查這段程式碼，指出問題和改進點：',

  // 歡迎頁：composer 與錯誤
  'bench.welcome.promptProjectPath': '輸入專案目錄路徑（瀏覽器預覽層無原生選擇器）',
  'bench.welcome.error.noStreamableAgent': '沒有支援 GUI 對話的代理（需已安裝且支援串流通道）',
  'bench.welcome.error.selectProjectDir': '請先選擇專案目錄',
  'bench.welcome.selectProjectDir': '選擇專案目錄',
  'bench.welcome.browseFolders': '瀏覽資料夾…',
  'bench.welcome.composerPlaceholder': '向代理提問，描述你的任務…（Enter 傳送，Shift+Enter 換行）',
  'bench.welcome.addAttachment': '新增附件（即將支援）',
  'bench.welcome.selectAgent': '選擇代理',
  'bench.welcome.agentTitle': '代理（需支援 GUI 串流對話）',
  'bench.welcome.selectModel': '選擇模型',
  'bench.welcome.effort': '推理強度',
  'bench.welcome.send': '傳送並建立會話',

  // 側欄
  'bench.sidebar.removeFromList': '從清單移除',
  'bench.sidebar.error.agentNotCapable': '{agent} 未安裝或不支援 GUI 對話',
  'bench.sidebar.error.resumeFailed': '{agent} 串流續聊失敗（{msg}）',
  'bench.sidebar.newSession': '新會話',
  'bench.sidebar.recallSearchTitle': 'Recall 全文搜尋',
  'bench.sidebar.searchPlaceholder': '搜尋會話…',
  'bench.sidebar.liveSessions': '活躍會話',
  'bench.sidebar.noLiveSessions': '目前沒有進行中的會話',
  'bench.sidebar.sessionsByProject': '會話（按專案）',
  'bench.sidebar.noMatch': '沒有符合的會話',
  'bench.sidebar.empty': '還沒有會話——從上方發起第一段對話',

  // 新建會話彈層
  'bench.newSession.notGuiCapable': '暫不支援 GUI 對話',
  'bench.newSession.notInstalled': '未安裝',
  'bench.newSession.guiStreamingVersioned': 'v{ver} · GUI 串流',
  'bench.newSession.guiStreaming': 'GUI 串流',
  'bench.newSession.title': '新增代理會話',
  'bench.newSession.agentsLabel': '代理（支援 GUI 串流）',
  'bench.newSession.scanning': '掃描已安裝代理…',
  'bench.newSession.projectDir': '專案目錄',
  'bench.newSession.modelEffort': '模型 / 推理強度',
  'bench.newSession.firstMessage': '第一則訊息（選填）',
  'bench.newSession.firstMessagePlaceholder': '建立後立即傳送…',
  'bench.newSession.create': '建立會話',

  // 訊息角色徽標
  'bench.role.user': '使用者',
  'bench.role.assistant': '代理',
  'bench.role.thinking': '思考',
  'bench.role.tool': '工具',
  'bench.role.system': '系統',

  // Recall 全文搜尋
  'bench.recall.back': '返回',
  'bench.recall.title': 'Recall · 會話全文搜尋',
  'bench.recall.indexStatus': '{sessions} 會話 · {messages} 訊息 · {kb} KB',
  'bench.recall.reindexTitle': '重建索引',
  'bench.recall.reindex': '重建',
  'bench.recall.searchPlaceholder': '搜尋所有代理的歷史會話…',
  'bench.recall.searching': '搜尋中…',
  'bench.recall.noHits': '沒有符合「{query}」的結果',
  'bench.recall.backToResults': '返回結果',
  'bench.recall.hint': '輸入關鍵字，搜尋 Claude / Codex 等代理的全部歷史會話',

  // 對話視圖
  'bench.chat.thinking': '思考中…',
  'bench.chat.thought': '已思考',
  'bench.chat.running': '執行中',
  'bench.chat.endSessionTitle': '結束會話',
  'bench.chat.end': '結束',
  'bench.chat.emptyTitle': '開始與編碼代理對話',
  'bench.chat.emptyHint': '選擇代理與專案目錄後，傳送第一則訊息',
  'bench.chat.composerPlaceholder': '傳訊息給代理…（Enter 傳送，Shift+Enter 換行）',

  // 終端機（PTY）
  'bench.terminal.createFailed': '會話建立失敗：{msg}',

  // hooks（非元件上下文）
  'bench.hooks.defaultSessionName': '{agent} 會話',
};
