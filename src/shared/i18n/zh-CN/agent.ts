/**
 * agent 命名空间：AI 助手页（/agent）、桌面助手面板（AssistantPanel）、
 * 桌面 MCP 分发卡片（McpSyncCard）、搜索页与 Spotlight 的结果文案。
 */
export const agent = {
  // 通用
  'agent.assistantName': '桌面助手',
  'agent.send': '发送',
  'agent.retry': '重试',
  'agent.fileFallback': '文件',

  // AI 助手页（/agent）
  'agent.title': 'AI 助手',
  'agent.systemPrompt': '你是仓鼠Hub 桌面助手内置的 AI 助手，回答简洁、友好、实用，默认使用中文。',
  'agent.requestFailed': '请求失败：{msg}',
  'agent.tab.assistant': '桌面助手',
  'agent.tab.quickAsk': '快问',
  'agent.subtitleAssistant': 'Agent 驱动 · 可操作桌面',
  'agent.subtitleQuick': '快问 · AI 直连补全',
  'agent.placeholderQuick': '问点什么…（Enter 发送，Shift+Enter 换行）',
  'agent.placeholderAssistant': '给桌面助手下达任务…（如「把「周五交周报」记成待办，然后打开计算器」）',
  'agent.launchAssistant': '启动桌面助手',
  'agent.clearSession': '清空会话',
  'agent.sessionActive': '助手会话进行中（结束按钮在会话头部）',
  'agent.emptyTitle': '有什么可以帮你？',
  'agent.emptySubtitle': '问答、写作、翻译、点子……配置模型后即可开始',
  'agent.suggestion.weeklyReport': '帮我写一条周报开头',
  'agent.suggestion.tools': '推荐几个效率工具',
  'agent.suggestion.mcp': '解释一下什么是 MCP',
  'agent.unconfiguredHint': '还没有配置 AI 模型——先到设置页填写 API 地址、模型名与 Key。',
  'agent.goConfigure': '去配置',

  // AssistantPanel：错误引导 / 会话横幅 / 欢迎态
  'agent.errMcpMissingTitle': '桌面 MCP server 缺失',
  'agent.errMcpMissingDetail': 'hamster-mcp.exe 不存在：重新构建（cargo build -p hamster-mcp）或重装应用。',
  'agent.errNoAgentTitle': '没有可用的 Agent CLI',
  'agent.errNoAgentDetail': '桌面助手由本机已安装的 Agent 驱动（Claude Code / ZCode 等），请先安装任意一个。',
  'agent.errStartFailedTitle': '桌面助手启动失败',
  'agent.mcpInjectedTitle': '本会话注入了 hamster-desktop MCP server：应用/文件/待办/音量 + 浏览器/桌面操作（后者受安全开关门控，全部调用留审计）',
  'agent.desktopToolsOn': '桌面工具已接入',
  'agent.computerUseOff': ' · 桌面点击/键入未开启（设置页可开启）',
  'agent.composerPlaceholder': '给桌面助手发消息…（Enter 发送，Shift+Enter 换行）',
  'agent.fallbackQuickAsk': '用快问模式继续（AI 直连）',
  'agent.welcomeTitle': '桌面助手 · 听得懂话，而且能动',
  'agent.welcomeDesc': '由本机 Agent 驱动，经桌面工具直接操作这台电脑：启动应用、找文件、记待办、调音量，需要时还能开浏览器查资料。所有工具调用留审计记录。',
  'agent.computerUseHint': '桌面点击/键入（computer use）默认关闭，可在设置页「允许操作电脑」中开启',

  // 桌面 MCP 分发卡片
  'agent.mcpCardTitle': '桌面 MCP · 分发到 Agent',
  'agent.mcpCardDesc': '把仓鼠Hub 的桌面能力（应用/文件/待办/音量/浏览器/屏幕）按标准 MCP 配置写入指定 agent——开启即生效，移除即摘除；写入前自动备份，可回滚。',
  'agent.copyUrl': '复制 URL',
  'agent.toggleToken': '显示/隐藏令牌',
  'agent.copyToken': '复制令牌',
  'agent.regenerateToken': '重新生成（旧令牌立即失效，已分发配置需更新）',
  'agent.portFallback': '首选端口 {port} 被占用，本次回退随机端口——已分发配置里的 URL 暂不可用，释放端口后重启应用即可恢复。',
  'agent.noAgents': '未检测到支持 MCP 分发的 agent（安装 Claude Code / Codex 等后自动出现）。',
  'agent.distributeTo': '分发桌面 MCP 到 {name}',

  // 搜索页 / Spotlight
  'agent.groupApps': '应用',
  'agent.groupFiles': '文件',
  'agent.groupWeb': '网页',
  'agent.groupAi': 'AI',
  'agent.appEnterHint': '应用 · Enter 启动',
  'agent.searchTitle': '搜索',
  'agent.searchHint': '拼音 / 首字母缩写 / 中文子串均可命中',
  'agent.searchPlaceholder': '搜索应用、文件、网页…',
  'agent.emptyHint': '输入即搜：应用（如「微信」「wx」「weixin」）、本地文件、网页与问 AI。',
  'agent.spotlightHint': '全局随时可用 Alt+Space 召出 Spotlight，与本页同一检索。',
  'agent.webSearch': '搜索「{q}」',
  'agent.webBaidu': '网页 · 百度',
  'agent.askAi': '问 AI：「{q}」',
  'agent.aiAssistant': 'AI 助手',
  'agent.todoQuickAdd': '记待办：「{q}」',
  'agent.todoNoDue': '无截止时间',
  'agent.noResults': '没有匹配「{q}」的结果',
} as const;
