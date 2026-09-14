import type { zhCN } from '../zh-CN';

import { chrome } from './chrome';
import { settings } from './settings';
import { agent } from './agent';
import { bench } from './bench';
import { home } from './home';
import { pages } from './pages';
import { vault } from './vault';
import { notifications } from './notifications';
import { split } from './split';

export const zhTW: Record<keyof typeof zhCN, string> = { ...chrome, ...settings, ...agent, ...bench, ...home, ...pages, ...vault, ...notifications, ...split };
