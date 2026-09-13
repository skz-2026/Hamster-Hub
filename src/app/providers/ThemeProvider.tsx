import { createContext, useContext, useEffect } from 'react';
import { useSettings } from '@/features/settings/hooks';

export type ThemeName = 'dark' | 'light' | 'pixel';

interface ThemeCtx {
  theme: ThemeName;
  accent: string;
}

const Ctx = createContext<ThemeCtx>({ theme: 'dark', accent: '#ff8a3d' });

/** 从 Rust 设置中读取主题并应用到 <html>，全应用通过 useTheme() 消费（未知值回落 dark） */
export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const { data: settings } = useSettings();
  const raw = settings?.appearance.theme;
  const theme: ThemeName = raw === 'light' || raw === 'pixel' ? raw : 'dark';
  const accent = settings?.appearance.accent ?? '#ff8a3d';

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    document.documentElement.style.setProperty('--accent', accent);
  }, [accent]);

  return <Ctx.Provider value={{ theme, accent }}>{children}</Ctx.Provider>;
}

export const useTheme = () => useContext(Ctx);
