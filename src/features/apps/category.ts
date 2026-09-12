import type { AppEntry } from '@/shared/lib/ipc';

export type AppCategory = '沟通' | '办公' | '开发' | '娱乐' | '工具' | '系统' | '其他';

export const CATEGORIES: AppCategory[] = ['沟通', '办公', '开发', '娱乐', '工具', '系统', '其他'];

/** 关键词 → 分类规则（按顺序首个命中生效；小写子串匹配） */
const RULES: [AppCategory, string[]][] = [
  [
    '沟通',
    ['微信', 'wechat', 'qq', '钉钉', 'dingtalk', '飞书', 'feishu', 'lark', 'telegram', 'discord',
      'slack', 'meeting', '会议', '向日葵', 'todesk', 'anydesk', 'teamviewer', 'uu', '千牛', '旺旺',
      '邮件', 'outlook', 'foxmail', 'thunderbird', '网易邮箱'],
  ],
  [
    '办公',
    ['word', 'excel', 'powerpoint', 'ppt', 'wps', 'office', 'pdf', 'notion', 'obsidian', 'wolai',
      '飞书文档', '思维导图', 'xmind', '幕布', '网盘', '百度网盘', 'aliyun', '夸克', 'quark',
      'markdown', 'typora', '语雀', 'summary', 'report', '报销', '发票', 'ocr', '扫描'],
  ],
  [
    '开发',
    ['code', 'cursor', 'visual studio', 'vs', 'git', 'github', 'gitlab', 'jdk', 'java', 'python',
      'pycharm', 'idea', 'webstorm', 'goland', 'rust', 'cargo', 'node', 'npm', 'pnpm', 'yarn',
      'docker', 'podman', 'postman', 'apifox', 'insomnia', 'navicat', 'datagrip', 'dbeaver',
      'redis', 'mysql', 'mongodb', 'terminal', '终端', 'powershell', 'cmd', 'wsl', 'xshell',
      'finalshell', 'ssh', 'ftp', 'xcode', 'android', 'sdk', 'nginx', 'dev-', 'ide', 'compiler',
      'hexo', 'vue', 'react', 'zcode', 'minimax', 'claude', 'copilot'],
  ],
  [
    '娱乐',
    ['steam', 'epic', '游戏', 'game', 'launcher', 'origin', 'uplay', 'battle', '网易云音乐',
      'qq音乐', 'music', 'spotify', '爱奇艺', '优酷', 'bilibili', '哔哩', '抖音', 'douyin',
      'potplayer', 'vlc', 'mpv', '影视', '电影', '视频', '直播', 'douyu', '虎牙', 'huya'],
  ],
  [
    '工具',
    ['snipaste', '截图', 'screenshot', 'snip', 'pixpin', 'everything', 'listary', 'utools',
      'quicker', 'compression', '压缩', 'winrar', '7-zip', 'bandizip', '迅雷', 'thunder',
      'download', '下载', 'idm', 'motrix', '磁盘', 'disk', 'crystaldisk', 'c盘', '清理', 'clean',
      '驱动', 'driver', '格式工厂', 'ffmpeg', 'handbrake', ' obs', 'obs ', '录屏', 'recorder',
      '计算器', 'calc', '记事本', 'notepad', '画图', 'paint', 'gimp', 'photoshop', 'ps ', 'ai ',
      'figma', 'sketch', 'blender', 'unity', 'unreal', '剪映', 'premiere', '达芬奇', ' 字体',
      '输入法', 'keyboard', 'mouse', '鼠标', '按键', 'clash', 'v2ray', 'proxy', '代理'],
  ],
  [
    '系统',
    ['setup', 'install', '更新', 'update', '驱动', 'directx', '.net', 'vc++', 'redistributable',
      'windows', 'microsoft store', '商店', '控制面板', '面板', '安全', '管家', '杀毒', 'defender',
      '火绒', '360', 'hub', 'assistant', '助手', '管家', 'watt toolkit', 'steam++'],
  ],
];

/** 应用名 → 分类（首个命中规则；无命中 → 其他） */
export function classifyApp(name: string): AppCategory {
  const lower = name.toLowerCase();
  for (const [cat, keywords] of RULES) {
    for (const kw of keywords) {
      if (lower.includes(kw)) return cat;
    }
  }
  return '其他';
}

export interface CategorizedApp extends AppEntry {
  category: AppCategory;
}

export function withCategories(apps: AppEntry[]): CategorizedApp[] {
  return apps.map((a) => ({ ...a, category: classifyApp(a.display_name) }));
}
