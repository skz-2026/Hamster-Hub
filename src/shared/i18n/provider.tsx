import { createContext, useContext, useEffect, useMemo } from 'react';
import { usePatchSettings, useSettings } from '@/features/settings/hooks';
import { LANGUAGES, normalizeLang, setCurrentLang, translate, type Lang, type TParams, type TKey } from './core';

interface I18nCtx {
  lang: Lang;
  langs: typeof LANGUAGES;
  /** 切换语言并持久化到 settings.behavior.language */
  setLang: (lang: Lang) => void;
  /** 翻译：t('ns.key', { n: 3 })，占位符 {n} */
  t: (key: TKey, params?: TParams) => string;
}

const fallback: I18nCtx = {
  lang: 'zh-CN',
  langs: LANGUAGES,
  setLang: () => {},
  t: (key, params) => translate('zh-CN', key, params),
};

const Ctx = createContext<I18nCtx>(fallback);

/** 从 Rust 设置读取语言并注入 t()，全应用通过 useI18n() 消费 */
export function I18nProvider({ children }: { children: React.ReactNode }) {
  const { data: settings } = useSettings();
  const { patch } = usePatchSettings();
  const lang = normalizeLang(settings?.behavior.language);

  setCurrentLang(lang);

  const t = useMemo(() => {
    return (key: TKey, params?: TParams) => translate(lang, key, params);
  }, [lang]);

  const setLang = (next: Lang) => patch({ behavior: { language: next } });

  // 同步 <html lang>，保证字体回退与无障碍标注跟随界面语言
  useEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);

  return <Ctx.Provider value={{ lang, langs: LANGUAGES, setLang, t }}>{children}</Ctx.Provider>;
}

export const useI18n = () => useContext(Ctx);
