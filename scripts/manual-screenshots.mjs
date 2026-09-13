/**
 * 用户手册截图脚本：浏览器 + IPC mock，遍历全部功能页面出图。
 * 配方要点（详见 docs/05-pitfalls.md 与 memory:readme-browser-screenshots-recipe）：
 *  - dev server 需已在 localhost:5173 运行（pnpm dev）。
 *  - addInitScript 用无参内联函数，种子 JSON 直接写进函数体（传参形式会静默失效）。
 *  - WebDebugBar（className 含 bottom-24 且 left-3）先点后删，或重开页面后再删。
 *  - 桌面模式是 mock 模块态，每次整页加载后需重新点调试条按钮；hash 跳转不重载，状态可延续。
 * 用法：node scripts/manual-screenshots.mjs
 */
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const OUT = resolve(ROOT, 'docs/images/manual');
const BASE = 'http://localhost:5173';
mkdirSync(OUT, { recursive: true });

const wait = (ms) => new Promise((r) => setTimeout(r, ms));

async function newPage(browser, seedVault) {
  const ctx = await browser.newContext({
    viewport: { width: 1920, height: 1080 },
    deviceScaleFactor: 1,
    locale: 'zh-CN',
  });
  await ctx.addInitScript(() => {
    localStorage.removeItem('hamsterhub.mock'); // 每次从干净默认态开始
  });
  if (seedVault) {
    // seedVault: { unlocked } —— 密码箱明文 mock 态，种好直接出目标界面
    const seedFn = new Function(
      'unlocked',
      `const now = 1757721600000;
       const e = (id, title, username, url, pw, fav) => ({
         id, title, username, url, favorite: fav,
         created_at: now, updated_at: now, password_updated_at: now,
         secret: { password: pw, notes: '' }, deleted: false,
       });
       const state = {
         master: 'hamster-hub-2026', hint: '仓鼠 + 年份', autoLockSecs: 300,
         unlocked, seq: 4,
         entries: [
           e(1, 'GitHub', 'hamster@example.com', 'https://github.com', 'Gh!2026#hub', true),
           e(2, '微博', '仓鼠玩家', 'https://weibo.com', 'Wb2026@hub', false),
           e(3, '公司邮箱', 'ham.zhang@company.com', 'https://mail.company.com', 'Mail#2026hub', false),
         ],
       };
       const all = JSON.parse(localStorage.getItem('hamsterhub.mock') || '{}');
       all.vault = JSON.stringify(state);
       localStorage.setItem('hamsterhub.mock', JSON.stringify(all));`
    );
    await ctx.addInitScript(seedFn, seedVault.unlocked);
  }
  const page = await ctx.newPage();
  return { ctx, page };
}

async function cleanBar(page) {
  await page.evaluate(() => {
    document.querySelectorAll('div.fixed').forEach((d) => {
      if (/bottom-24/.test(d.className) && /left-3/.test(d.className)) d.remove();
    });
  });
}

async function clickByText(page, pattern) {
  return page.evaluate((p) => {
    const re = new RegExp(p);
    const els = [...document.querySelectorAll('button, a, [role="button"]')];
    const el = els.find((e) => re.test((e.textContent || '').trim()) && e.offsetParent !== null);
    if (el) {
      el.click();
      return true;
    }
    return false;
  }, pattern);
}

async function enterDesktop(page) {
  await clickByText(page, '进入桌面模式');
  await wait(1000);
}

async function goto(page, hash) {
  await page.goto(`${BASE}/#${hash}`);
  await page.waitForLoadState('networkidle').catch(() => {});
  await wait(800);
}

async function shot(page, name) {
  await page.screenshot({ path: resolve(OUT, `${name}.png`) });
  console.log('✓', name);
}

async function typeInto(page, placeholderPattern, text) {
  const input = page.getByPlaceholder(placeholderPattern).first();
  await input.click();
  await input.fill(text);
  await wait(500);
}

const browser = await chromium.launch();

// ========== A. 窗口化页面 ==========
{
  const { ctx, page } = await newPage(browser);

  // 工作台（窗口模式）
  await goto(page, '/');
  await cleanBar(page);
  await shot(page, '01-workbench-windowed');

  // 待办 NLP：输入解析出截止时间 chip
  await typeInto(page, /添加待办/, '明天下午3点交房租');
  await shot(page, '02-workbench-todo-nlp');
  await page.getByPlaceholder(/添加待办/).first().fill('');

  // 工作台编辑态
  await clickByText(page, '^编辑$');
  await wait(600);
  await shot(page, '03-workbench-edit');
  await clickByText(page, '^完成$');

  // 应用 / 文件 / 日程
  await goto(page, '/apps');
  await cleanBar(page);
  await shot(page, '04-apps');

  await goto(page, '/files');
  await cleanBar(page);
  await shot(page, '05-files');

  await goto(page, '/schedule');
  await cleanBar(page);
  await shot(page, '06-schedule');

  // 搜索页
  await goto(page, '/search');
  await typeInto(page, /搜索应用、文件、网页/, 'wx');
  await cleanBar(page);
  await shot(page, '07-search');

  // Agent 工作台
  await goto(page, '/bench');
  await cleanBar(page);
  await shot(page, '08-bench-welcome');

  await clickByText(page, '新会话');
  await wait(600);
  await shot(page, '09-bench-new-session');
  await clickByText(page, '^取消$'); // Esc 关不掉（焦点不在弹层），用取消按钮
  await wait(400);

  // 发一条消息跑 mock 假流式（约 5 秒剧本），截真实会话视图
  const benchInput = page.getByPlaceholder(/向 AI Agent 提问/).first();
  await benchInput.click();
  await benchInput.fill('帮我梳理一下仓库结构，找出可疑的死代码');
  await page.keyboard.press('Enter');
  await wait(9500);
  await cleanBar(page);
  await shot(page, '10-bench-chat');

  // 侧栏放大镜按钮打开 Recall 全文搜索
  await page.locator('button:has(svg.lucide-search)').first().click();
  await wait(800);
  const recallInput = page.getByPlaceholder(/搜索所有 AI Agent 的历史会话/).first();
  await recallInput.click();
  await recallInput.fill('拼音');
  await wait(800);
  await cleanBar(page);
  await shot(page, '11-bench-recall');

  // AI 助手页
  await goto(page, '/agent');
  await cleanBar(page);
  await shot(page, '12-agent-assistant');

  await clickByText(page, '快问');
  await wait(600);
  await cleanBar(page);
  await shot(page, '13-agent-quick');

  // 设置页（分区滚动）
  await goto(page, '/settings');
  await cleanBar(page);
  await shot(page, '14-settings-appearance');
  await page.getByText('AI 与助手').first().scrollIntoViewIfNeeded();
  await wait(400);
  await shot(page, '15-settings-ai');
  await page.getByText('插件', { exact: true }).first().scrollIntoViewIfNeeded();
  await wait(400);
  await shot(page, '16-settings-plugins-system');

  await ctx.close();
}

// ========== B. 密码箱（种已解锁 / 已锁定两种状态） ==========
{
  const { ctx, page } = await newPage(browser, { unlocked: true });
  await goto(page, '/vault');
  await cleanBar(page);
  await shot(page, '17-vault-list');

  await clickByText(page, '新建');
  await wait(600);
  await shot(page, '18-vault-editor');
  await ctx.close();
}
{
  const { ctx, page } = await newPage(browser, { unlocked: false });
  await goto(page, '/vault');
  await cleanBar(page);
  await shot(page, '19-vault-locked');
  await ctx.close();
}

// ========== C. 桌面接管形态 ==========
{
  const { ctx, page } = await newPage(browser);

  await goto(page, '/');
  await enterDesktop(page);
  await cleanBar(page);
  await shot(page, '20-home-desktop');

  // 控制中心
  await clickByText(page, '控制中心');
  await wait(600);
  await shot(page, '21-control-center');

  // 主屏（iOS 风）
  await page.evaluate(() => (location.hash = '#/home'));
  await wait(1000);
  await cleanBar(page);
  await shot(page, '22-home-screen');

  // 主屏编辑态
  await page.evaluate(() => {
    const btn = document.querySelector('[title*="长按图标"]');
    if (btn) btn.click();
  });
  await wait(600);
  await shot(page, '23-home-screen-edit');

  // 任务栏（独立置顶条路由；从主屏编辑态 hash 跳转会白屏，直接全新加载）
  {
    const t = await newPage(browser);
    await goto(t.page, '/taskbar');
    await shot(t.page, '24-taskbar');
    await t.ctx.close();
  }

  await ctx.close();
}

// ========== D. Spotlight（独立覆盖层，全新加载更稳） ==========
{
  const { ctx, page } = await newPage(browser);
  await goto(page, '/spotlight');
  await typeInto(page, /搜索应用、文件、网页/, '微信');
  await shot(page, '25-spotlight');

  await page.getByPlaceholder(/搜索应用、文件、网页/).first().fill('明天下午3点交房租');
  await wait(500);
  await shot(page, '26-spotlight-todo');

  await ctx.close();
}

await browser.close();
console.log('done →', OUT);
