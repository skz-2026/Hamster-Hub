/** 密码箱文案（基准词典，zh-TW / en 的 key 必须与之完全一致） */
export const vault = {
  // —— 页头 ——
  'pages.vault.title': '密码箱',
  'pages.vault.subtitle': '密码字段本地加密 · 主密码只在内存',

  // —— 初始化 ——
  'pages.vault.setup.title': '创建密码库',
  'pages.vault.setup.desc': '主密码用于加密所有密码，无法找回、不会上传——请务必牢记。',
  'pages.vault.setup.master': '主密码',
  'pages.vault.setup.masterPh': '至少 8 个字符',
  'pages.vault.setup.confirm': '确认主密码',
  'pages.vault.setup.confirmPh': '再输入一次',
  'pages.vault.setup.hint': '密码提示（可选）',
  'pages.vault.setup.hintPh': '例如：常用的那个 + 生日',
  'pages.vault.setup.submit': '创建密码库',
  'pages.vault.setup.mismatch': '两次输入的主密码不一致',
  'pages.vault.setup.tooShort': '主密码至少 8 个字符',

  // —— 解锁 ——
  'pages.vault.unlock.title': '密码箱已锁定',
  'pages.vault.unlock.desc': '输入主密码解锁',
  'pages.vault.unlock.master': '主密码',
  'pages.vault.unlock.hint': '提示：{hint}',
  'pages.vault.unlock.submit': '解锁',
  'pages.vault.unlock.failed': '主密码不正确',
  'pages.vault.unlock.idle': '长时间未操作，已自动锁定',

  // —— 列表 ——
  'pages.vault.list.searchPh': '搜索标题 / 用户名 / 网址',
  'pages.vault.list.new': '新建',
  'pages.vault.list.emptyTitle': '密码箱是空的',
  'pages.vault.list.emptyDesc': '存入第一条密码，之后在这里搜索、一键复制',
  'pages.vault.list.emptySearch': '没有匹配的条目',
  'pages.vault.list.count': '{count} 条',
  'pages.vault.list.lock': '锁定',

  // —— 自动锁定 ——
  'pages.vault.autolock.label': '自动锁定',
  'pages.vault.autolock.60': '1 分钟',
  'pages.vault.autolock.300': '5 分钟',
  'pages.vault.autolock.900': '15 分钟',
  'pages.vault.autolock.1800': '30 分钟',
  'pages.vault.autolock.3600': '1 小时',

  // —— 条目编辑 ——
  'pages.vault.item.title': '标题',
  'pages.vault.item.titleRequired': '标题不能为空',
  'pages.vault.item.titlePh': '例如：GitHub',
  'pages.vault.item.username': '用户名',
  'pages.vault.item.usernamePh': '邮箱 / 手机号 / 用户名',
  'pages.vault.item.url': '网址',
  'pages.vault.item.urlPh': 'https://example.com',
  'pages.vault.item.password': '密码',
  'pages.vault.item.notes': '备注',
  'pages.vault.item.notesPh': '恢复码、绑定手机等附加信息',
  'pages.vault.item.favorite': '收藏',
  'pages.vault.item.newTitle': '新建密码',
  'pages.vault.item.editTitle': '编辑密码',
  'pages.vault.item.save': '保存',
  'pages.vault.item.cancel': '取消',
  'pages.vault.item.delete': '删除',
  'pages.vault.item.deleteConfirm': '删除「{title}」？删除后进入回收站（即将上线）。',
  'pages.vault.item.reveal': '显示密码',
  'pages.vault.item.hide': '隐藏密码',

  // —— 复制 ——
  'pages.vault.copy.password': '复制密码',
  'pages.vault.copy.username': '复制用户名',
  'pages.vault.copy.done': '已复制，{secs} 秒后自动清除剪贴板',
  'pages.vault.copy.doneKeep': '已复制',
  'pages.vault.copy.hint': '提示：Windows 剪贴板历史（Win+V）会留存副本，可到系统设置关闭',

  // —— 生成器 ——
  'pages.vault.gen.title': '生成密码',
  'pages.vault.gen.length': '长度',
  'pages.vault.gen.upper': '大写字母',
  'pages.vault.gen.lower': '小写字母',
  'pages.vault.gen.digits': '数字',
  'pages.vault.gen.symbols': '符号',
  'pages.vault.gen.ambiguous': '排除形近字符（I l 1 O 0）',
  'pages.vault.gen.refresh': '换一个',
  'pages.vault.gen.apply': '使用',

  // —— 强度（0..=4，配合字面量 key 数组使用） ——
  'pages.vault.strength.0': '弱',
  'pages.vault.strength.1': '较弱',
  'pages.vault.strength.2': '一般',
  'pages.vault.strength.3': '较强',
  'pages.vault.strength.4': '强',

  // —— 修改主密码 ——
  'pages.vault.change.title': '修改主密码',
  'pages.vault.change.old': '当前主密码',
  'pages.vault.change.new': '新主密码',
  'pages.vault.change.confirm': '确认新主密码',
  'pages.vault.change.submit': '确认修改',
  'pages.vault.change.failed': '当前主密码不正确',
  'pages.vault.change.mismatch': '两次输入不一致',
} as const;
