/** 密碼箱文案（key 與 zh-CN 基準詞典完全一致） */
export const vault = {
  // —— 頁頭 ——
  'pages.vault.title': '密碼箱',
  'pages.vault.subtitle': '密碼欄位本地加密 · 主密碼只在記憶體',

  // —— 初始化 ——
  'pages.vault.setup.title': '建立密碼庫',
  'pages.vault.setup.desc': '主密碼用於加密所有密碼，無法找回、不會上傳——請務必牢記。',
  'pages.vault.setup.master': '主密碼',
  'pages.vault.setup.masterPh': '至少 8 個字元',
  'pages.vault.setup.confirm': '確認主密碼',
  'pages.vault.setup.confirmPh': '再輸入一次',
  'pages.vault.setup.hint': '密碼提示（可選）',
  'pages.vault.setup.hintPh': '例如：常用的那個 + 生日',
  'pages.vault.setup.submit': '建立密碼庫',
  'pages.vault.setup.mismatch': '兩次輸入的主密碼不一致',
  'pages.vault.setup.tooShort': '主密碼至少 8 個字元',

  // —— 解鎖 ——
  'pages.vault.unlock.title': '密碼箱已鎖定',
  'pages.vault.unlock.desc': '輸入主密碼解鎖',
  'pages.vault.unlock.master': '主密碼',
  'pages.vault.unlock.hint': '提示：{hint}',
  'pages.vault.unlock.submit': '解鎖',
  'pages.vault.unlock.failed': '主密碼不正確',
  'pages.vault.unlock.idle': '長時間未操作，已自動鎖定',

  // —— 列表 ——
  'pages.vault.list.searchPh': '搜尋標題 / 使用者名稱 / 網址',
  'pages.vault.list.new': '新增',
  'pages.vault.list.emptyTitle': '密碼箱是空的',
  'pages.vault.list.emptyDesc': '存入第一條密碼，之後在這裡搜尋、一鍵複製',
  'pages.vault.list.emptySearch': '沒有符合的條目',
  'pages.vault.list.count': '{count} 條',
  'pages.vault.list.lock': '鎖定',

  // —— 自動鎖定 ——
  'pages.vault.autolock.label': '自動鎖定',
  'pages.vault.autolock.60': '1 分鐘',
  'pages.vault.autolock.300': '5 分鐘',
  'pages.vault.autolock.900': '15 分鐘',
  'pages.vault.autolock.1800': '30 分鐘',
  'pages.vault.autolock.3600': '1 小時',

  // —— 條目編輯 ——
  'pages.vault.item.title': '標題',
  'pages.vault.item.titleRequired': '標題不能為空',
  'pages.vault.item.titlePh': '例如：GitHub',
  'pages.vault.item.username': '使用者名稱',
  'pages.vault.item.usernamePh': '信箱 / 手機號 / 使用者名稱',
  'pages.vault.item.url': '網址',
  'pages.vault.item.urlPh': 'https://example.com',
  'pages.vault.item.password': '密碼',
  'pages.vault.item.notes': '備註',
  'pages.vault.item.notesPh': '復原碼、綁定手機等附加資訊',
  'pages.vault.item.favorite': '收藏',
  'pages.vault.item.newTitle': '新增密碼',
  'pages.vault.item.editTitle': '編輯密碼',
  'pages.vault.item.save': '儲存',
  'pages.vault.item.cancel': '取消',
  'pages.vault.item.delete': '刪除',
  'pages.vault.item.deleteConfirm': '刪除「{title}」？刪除後進入回收站（即將上線）。',
  'pages.vault.item.reveal': '顯示密碼',
  'pages.vault.item.hide': '隱藏密碼',

  // —— 複製 ——
  'pages.vault.copy.password': '複製密碼',
  'pages.vault.copy.username': '複製使用者名稱',
  'pages.vault.copy.done': '已複製，{secs} 秒後自動清除剪貼簿',
  'pages.vault.copy.doneKeep': '已複製',
  'pages.vault.copy.hint': '提示：Windows 剪貼簿歷史（Win+V）會留存副本，可到系統設定關閉',

  // —— 產生器 ——
  'pages.vault.gen.title': '產生密碼',
  'pages.vault.gen.length': '長度',
  'pages.vault.gen.upper': '大寫字母',
  'pages.vault.gen.lower': '小寫字母',
  'pages.vault.gen.digits': '數字',
  'pages.vault.gen.symbols': '符號',
  'pages.vault.gen.ambiguous': '排除形近字元（I l 1 O 0）',
  'pages.vault.gen.refresh': '換一個',
  'pages.vault.gen.apply': '使用',

  // —— 強度 ——
  'pages.vault.strength.0': '弱',
  'pages.vault.strength.1': '較弱',
  'pages.vault.strength.2': '一般',
  'pages.vault.strength.3': '較強',
  'pages.vault.strength.4': '強',

  // —— 修改主密碼 ——
  'pages.vault.change.title': '修改主密碼',
  'pages.vault.change.old': '目前主密碼',
  'pages.vault.change.new': '新主密碼',
  'pages.vault.change.confirm': '確認新主密碼',
  'pages.vault.change.submit': '確認修改',
  'pages.vault.change.failed': '目前主密碼不正確',
  'pages.vault.change.mismatch': '兩次輸入不一致',
} as const;
