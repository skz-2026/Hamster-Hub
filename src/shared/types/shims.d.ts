// Vite 特性（import.meta.glob / `?raw` 导入）的类型支持（三斜线指令须在文件顶部）
/// <reference types="vite/client" />

// lunar-javascript 无官方类型声明，M0 仅用到 Solar → Lunar 的最小面
declare module 'lunar-javascript' {
  export interface Lunar {
    getMonthInChinese(): string;
    getDayInChinese(): string;
    getYearInGanZhi(): string;
    getJieQi(): string;
  }
  export interface Solar {
    getLunar(): Lunar;
  }
  export const Solar: {
    fromDate(d: Date): Solar;
    fromJsDate(d: Date): Solar;
  };
}

