import { chrome } from "./chrome";
import { settings } from "./settings";
import { agent } from "./agent";
import { bench } from "./bench";
import { home } from "./home";
import { pages } from "./pages";
import { vault } from "./vault";
import { notifications } from "./notifications";

/** 简体中文 = 基准词典；en / zh-TW 必须包含完全相同的 key（编译期强制） */
export const zhCN = { ...chrome, ...settings, ...agent, ...bench, ...home, ...pages, ...vault, ...notifications } as const;
