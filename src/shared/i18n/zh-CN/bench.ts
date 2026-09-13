/**
 * bench 命名空间基准词典（多代理会话：欢迎页/侧栏/新建会话/Recall/对话视图/终端）。
 * en / zh-TW 必须包含完全相同的 key（编译期强制）。
 */
export const bench = {
  // 通用
  'bench.common.loading': '加载中…',
  'bench.common.cancel': '取消',

  // 相对时间（en 用不受单复数影响的简洁写法）
  'bench.time.justNow': '刚刚',
  'bench.time.minutesAgo': '{n} 分钟前',
  'bench.time.hoursAgo': '{n} 小时前',
  'bench.time.daysAgo': '{n} 天前',

  // 欢迎页：时段问候语
  'bench.welcome.greeting.night': '夜深了，有什么想让我帮忙的吗',
  'bench.welcome.greeting.earlyMorning': '早上好呀，有什么想让我帮忙的吗',
  'bench.welcome.greeting.morning': '上午好呀，有什么想让我帮忙的吗',
  'bench.welcome.greeting.noon': '中午好，有什么想让我帮忙的吗',
  'bench.welcome.greeting.afternoon': '下午好，有什么想让我帮忙的吗',
  'bench.welcome.greeting.evening': '晚上好，有什么想让我帮忙的吗',

  // 欢迎页：快捷 prompt 卡片（label + 发给 AI 的 prompt 正文）
  'bench.welcome.suggest.repoTitle': '梳理仓库结构',
  'bench.welcome.suggest.repoPrompt': '帮我梳理一下这个仓库的目录结构和核心模块',
  'bench.welcome.suggest.fixTitle': '修复报错',
  'bench.welcome.suggest.fixPrompt': '遇到一个报错，帮我定位并修复：',
  'bench.welcome.suggest.testTitle': '写单元测试',
  'bench.welcome.suggest.testPrompt': '为下面的函数补一组单元测试：',
  'bench.welcome.suggest.reviewTitle': '代码审查',
  'bench.welcome.suggest.reviewPrompt': '帮我审查这段代码，指出问题和改进点：',

  // 欢迎页：composer 与错误
  'bench.welcome.promptProjectPath': '输入项目目录路径（浏览器预览层无原生选择器）',
  'bench.welcome.error.noStreamableAgent': '没有支持 GUI 对话的代理（需已安装且支持流式通道）',
  'bench.welcome.error.selectProjectDir': '请先选择项目目录',
  'bench.welcome.selectProjectDir': '选择项目目录',
  'bench.welcome.browseFolders': '浏览文件夹…',
  'bench.welcome.composerPlaceholder': '向代理提问，描述你的任务…（Enter 发送，Shift+Enter 换行）',
  'bench.welcome.addAttachment': '添加附件（即将支持）',
  'bench.welcome.selectAgent': '选择代理',
  'bench.welcome.agentTitle': '代理（需支持 GUI 流式对话）',
  'bench.welcome.selectModel': '选择模型',
  'bench.welcome.effort': '推理强度',
  'bench.welcome.send': '发送并创建会话',

  // 侧栏
  'bench.sidebar.removeFromList': '从列表移除',
  'bench.sidebar.error.agentNotCapable': '{agent} 未安装或不支持 GUI 对话',
  'bench.sidebar.error.resumeFailed': '{agent} 流式续聊失败（{msg}）',
  'bench.sidebar.newSession': '新会话',
  'bench.sidebar.recallSearchTitle': 'Recall 全文搜索',
  'bench.sidebar.searchPlaceholder': '搜索会话…',
  'bench.sidebar.liveSessions': '活会话',
  'bench.sidebar.noLiveSessions': '暂无进行中的会话',
  'bench.sidebar.sessionsByProject': '会话（按项目）',
  'bench.sidebar.noMatch': '没有匹配的会话',
  'bench.sidebar.empty': '还没有会话——从上方发起第一段对话',

  // 新建会话弹层
  'bench.newSession.notGuiCapable': '暂不支持 GUI 对话',
  'bench.newSession.notInstalled': '未安装',
  'bench.newSession.guiStreamingVersioned': 'v{ver} · GUI 流式',
  'bench.newSession.guiStreaming': 'GUI 流式',
  'bench.newSession.title': '新建代理会话',
  'bench.newSession.agentsLabel': '代理（支持 GUI 流式）',
  'bench.newSession.scanning': '扫描已安装代理…',
  'bench.newSession.projectDir': '项目目录',
  'bench.newSession.modelEffort': '模型 / 推理强度',
  'bench.newSession.firstMessage': '首条消息（可选）',
  'bench.newSession.firstMessagePlaceholder': '创建后立即发送…',
  'bench.newSession.create': '创建会话',

  // 消息角色徽标
  'bench.role.user': '用户',
  'bench.role.assistant': '代理',
  'bench.role.thinking': '思考',
  'bench.role.tool': '工具',
  'bench.role.system': '系统',

  // Recall 全文搜索
  'bench.recall.back': '返回',
  'bench.recall.title': 'Recall · 会话全文搜索',
  'bench.recall.indexStatus': '{sessions} 会话 · {messages} 消息 · {kb} KB',
  'bench.recall.reindexTitle': '重建索引',
  'bench.recall.reindex': '重建',
  'bench.recall.searchPlaceholder': '搜索所有代理的历史会话…',
  'bench.recall.searching': '搜索中…',
  'bench.recall.noHits': '没有关于「{query}」的命中',
  'bench.recall.backToResults': '返回结果',
  'bench.recall.hint': '输入关键词，搜索 Claude / Codex 等代理的全部历史会话',

  // 对话视图
  'bench.chat.thinking': '思考中…',
  'bench.chat.thought': '已思考',
  'bench.chat.running': '运行中',
  'bench.chat.endSessionTitle': '结束会话',
  'bench.chat.end': '结束',
  'bench.chat.emptyTitle': '开始与编码代理对话',
  'bench.chat.emptyHint': '选择代理与项目目录后，发送第一条消息',
  'bench.chat.composerPlaceholder': '给代理发消息…（Enter 发送，Shift+Enter 换行）',

  // 终端（PTY）
  'bench.terminal.createFailed': '会话创建失败：{msg}',

  // hooks（非组件上下文）
  'bench.hooks.defaultSessionName': '{agent} 会话',
} as const;
