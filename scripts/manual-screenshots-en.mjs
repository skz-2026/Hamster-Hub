/**
 * 英文素材全量截图：语言种子 en + mock 英文演示数据（ipc-mock L()）。
 * 出图 docs/images/manual-en/，编号与中文版 manual/ 一一对应。
 * 前置：dev server localhost:5173。用法：node scripts/manual-screenshots-en.mjs
 */
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const OUT = resolve(ROOT, 'docs/images/manual-en');
const BASE = 'http://localhost:5173';
mkdirSync(OUT, { recursive: true });

const wait = (ms) => new Promise((r) => setTimeout(r, ms));

async function newPage(browser, seedVault) {
  const ctx = await browser.newContext({
    viewport: { width: 1920, height: 1080 },
    deviceScaleFactor: 1,
    locale: 'en-US',
  });
  // 语言种子：内联无参形式（传参形式会静默失效）
  await ctx.addInitScript(() => {
    const all = JSON.parse(localStorage.getItem('hamsterhub.mock') || '{}');
    all.settings = JSON.stringify({ behavior: { language: 'en' } });
    localStorage.setItem('hamsterhub.mock', JSON.stringify(all));
  });
  if (seedVault) {
    const seedFn = new Function(
      'unlocked',
      `const now = 1757721600000;
       const e = (id, title, username, url, pw, fav) => ({
         id, title, username, url, favorite: fav,
         created_at: now, updated_at: now, password_updated_at: now,
         secret: { password: pw, notes: '' }, deleted: false,
       });
       const state = {
         master: 'hamster-hub-2026', hint: 'ham + year', autoLockSecs: 300,
         unlocked, seq: 4,
         entries: [
           e(1, 'GitHub', 'hamster@example.com', 'https://github.com', 'Gh!2026#hub', true),
           e(2, 'Twitter', '@hamsterhub', 'https://twitter.com', 'Tw2026@hub', false),
           e(3, 'Company Mail', 'ham.zhang@company.com', 'https://mail.company.com', 'Mail#2026hub', false),
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
  await clickByText(page, '^Enter desktop mode');
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

  await goto(page, '/');
  await cleanBar(page);
  await shot(page, '01-workbench-windowed');

  await typeInto(page, /Add a to-do/, 'Pay rent tomorrow 3 PM');
  await shot(page, '02-workbench-todo-nlp');
  await page.getByPlaceholder(/Add a to-do/).first().fill('');

  await clickByText(page, '^Edit$');
  await wait(600);
  await shot(page, '03-workbench-edit');
  await clickByText(page, '^Done$');

  await goto(page, '/apps');
  await cleanBar(page);
  await shot(page, '04-apps');

  await goto(page, '/files');
  await cleanBar(page);
  await shot(page, '05-files');

  await goto(page, '/schedule');
  await cleanBar(page);
  await shot(page, '06-schedule');

  await goto(page, '/search');
  await typeInto(page, /Search apps, files/, 'git');
  await cleanBar(page);
  await shot(page, '07-search');

  await goto(page, '/bench');
  await cleanBar(page);
  await shot(page, '08-bench-welcome');

  await clickByText(page, 'New session');
  await wait(600);
  await shot(page, '09-bench-new-session');
  await clickByText(page, '^Cancel$');
  await wait(400);

  const benchInput = page.getByPlaceholder(/Ask the agent about your task/).first();
  await benchInput.click();
  await benchInput.fill('Review the repo structure and spot dead code');
  await page.keyboard.press('Enter');
  await wait(9500);
  await cleanBar(page);
  await shot(page, '10-bench-chat');

  await page.locator('button:has(svg.lucide-search)').first().click();
  await wait(800);
  const recallInput = page.getByPlaceholder(/Search history across all agents/).first();
  await recallInput.click();
  await recallInput.fill('repo');
  await wait(800);
  await cleanBar(page);
  await shot(page, '11-bench-recall');

  await goto(page, '/agent');
  await cleanBar(page);
  await shot(page, '12-agent-assistant');

  await clickByText(page, 'Quick Ask');
  await wait(600);
  await cleanBar(page);
  await shot(page, '13-agent-quick');

  await goto(page, '/settings');
  await cleanBar(page);
  await shot(page, '14-settings-appearance');
  await page.getByText('AI & assistant').first().scrollIntoViewIfNeeded();
  await wait(400);
  await shot(page, '15-settings-ai');
  await page.getByText('Plugins', { exact: true }).first().scrollIntoViewIfNeeded();
  await wait(400);
  await shot(page, '16-settings-plugins-system');

  await ctx.close();
}

// ========== B. 密码箱 ==========
{
  const { ctx, page } = await newPage(browser, { unlocked: true });
  await goto(page, '/vault');
  await cleanBar(page);
  await shot(page, '17-vault-list');

  await clickByText(page, 'New');
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

  await clickByText(page, '^Control center');
  await wait(600);
  await shot(page, '21-control-center');
  await page.keyboard.press('Escape').catch(() => {});

  await page.evaluate(() => (location.hash = '#/home'));
  await wait(1000);
  await cleanBar(page);
  await shot(page, '22-home-screen');

  await page.evaluate(() => {
    const btn = [...document.querySelectorAll('button')].find((b) =>
      /arrange/i.test(b.getAttribute('title') || ''),
    );
    if (btn) btn.click();
  });
  await wait(600);
  await shot(page, '23-home-screen-edit');

  {
    const t = await newPage(browser);
    await goto(t.page, '/taskbar');
    await shot(t.page, '24-taskbar');
    await t.ctx.close();
  }

  // ========== D. AI 整理（悬浮球演示回合） ==========
  await page.evaluate(() => (location.hash = '#/home'));
  await wait(1200);
  await cleanBar(page);
  await shot(page, '27-home-organize-ball');

  await page.locator('[title="AI organizer"]').first().click();
  await wait(600);
  const input = page.getByPlaceholder(/Give an instruction/).first();
  await input.click();
  await input.fill('Group apps into folders by purpose');
  await wait(400);
  await page.locator('[aria-label="Send"]').first().click();
  await wait(10000);
  await cleanBar(page);
  await shot(page, '28-home-organize-panel');

  await page.locator('[aria-label="Collapse"]').first().click().catch(() => {});
  await wait(600);
  await shot(page, '29-home-organize-done');

  await ctx.close();
}

// ========== E. Spotlight ==========
{
  const { ctx, page } = await newPage(browser);
  await goto(page, '/spotlight');
  await typeInto(page, /Search apps, files/, 'Git');
  await shot(page, '25-spotlight');

  await page.getByPlaceholder(/Search apps, files/).first().fill('travel');
  await wait(500);
  await shot(page, '26-spotlight-todo');

  await ctx.close();
}

await browser.close();
console.log('done →', OUT);
