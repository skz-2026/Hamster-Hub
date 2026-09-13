import type { zhCN } from '../zh-CN';

import { chrome } from './chrome';
import { settings } from './settings';
import { agent } from './agent';
import { bench } from './bench';
import { home } from './home';
import { pages } from './pages';

export const zhTW: Record<keyof typeof zhCN, string> = { ...chrome, ...settings, ...agent, ...bench, ...home, ...pages };
