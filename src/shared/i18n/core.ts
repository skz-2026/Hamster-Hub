import { zhCN } from './zh-CN';
import { zhTW } from './zh-TW';
import { en } from './en';

/** 支持的语言；zh-CN 是基准词典，en / zh-TW 的 key 必须与之完全一致（编译期强制） */
export type Lang = 'zh-CN' | 'zh-TW' | 'en';

export const LANGUAGES: readonly { id: Lang; label: string }[] = [
  { id: 'zh-CN', label: '简体中文' },
  { id: 'zh-TW', label: '繁體中文' },
  { id: 'en', label: 'English' },
] as const;

/** 所有翻译 key 的联合类型（以 zh-CN 词典为准） */
export type TKey = keyof typeof zhCN;
export type TParams = Record<string, string | number>;

const DICTS: Record<Lang, Record<TKey, string>> = {
  'zh-CN': zhCN,
  'zh-TW': zhTW,
  en,
};

/** 容错解析语言值（设置库里的旧值/非法值回落到 zh-CN） */
export function normalizeLang(value: string | undefined | null): Lang {
  return value === 'zh-TW' || value === 'en' ? value : 'zh-CN';
}

let currentLang: Lang = 'zh-CN';

/** 当前语言（供 React 之外的非组件代码使用；由 I18nProvider 保持同步） */
export function getLang(): Lang {
  return currentLang;
}

/** 仅供 I18nProvider 内部同步模块级当前语言 */
export function setCurrentLang(lang: Lang): void {
  currentLang = lang;
}

/** 翻译：t('settings.theme.label')；占位符写作 {name}，经 params 替换 */
export function translate(lang: Lang, key: TKey, params?: TParams): string {
  let text: string = DICTS[lang][key] ?? zhCN[key] ?? key;
  if (params) {
    for (const [name, value] of Object.entries(params)) {
      text = text.split(`{${name}}`).join(String(value));
    }
  }
  return text;
}
