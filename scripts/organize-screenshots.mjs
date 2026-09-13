/**
 * AI 整理桌面功能截图：主屏悬浮球 → 指令 → mock 演示回合（真实分组建文件夹 + 布局刷新）。
 * 前置：dev server localhost:5173（pnpm dev）。出图 docs/images/manual/。
 * 用法：node scripts/organize-screenshots.mjs
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

const browser = await chromium.launch();
const ctx = await browser.newContext({
  viewport: { width: 1920, height: 1080 },
  deviceScaleFactor: 1,
  locale: 'zh-CN',
});
await ctx.addInitScript(() => {
  localStorage.removeItem('hamsterhub.mock');
});
const page = await ctx.newPage();

async function cleanBar(page) {
  await page.evaluate(() => {
    document.querySelectorAll('div.fixed').forEach((d) => {
      if (/bottom-24/.test(d.className) && /left-3/.test(d.className)) d.remove();
    });
  });
}

async function shot(name) {
  await page.screenshot({ path: resolve(OUT, `${name}.png`) });
  console.log('✓', name);
}

// 进入桌面模式 → 主屏
await page.goto(`${BASE}/#/`);
await page.waitForLoadState('networkidle').catch(() => {});
await wait(800);
await page.evaluate(() => {
  const els = [...document.querySelectorAll('button, a, [role="button"]')];
  const el = els.find((e) => /进入桌面模式/.test((e.textContent || '').trim()) && e.offsetParent !== null);
  if (el) el.click();
});
await wait(1000);
await page.evaluate(() => (location.hash = '#/home'));
await wait(1200);
await cleanBar(page);

// 1. 悬浮球（整理前主屏）
await shot('27-home-organize-ball');

// 2. 展开悬浮球面板 → 输入指令 → mock 演示回合跑完（分组建文件夹 + 完成回复）
await page.locator('[title="AI 整理助手"]').first().click();
await wait(600);
const input = page.getByPlaceholder(/下个指令/).first();
await input.click();
await input.fill('按用途分组建文件夹');
await wait(400);
await page.locator('[aria-label="发送"]').first().click();
await wait(10000); // 等 mock 流式回复 + 布局刷新完成
await cleanBar(page);
await shot('28-home-organize-panel');

// 3. 收起面板，整理后的主屏（沟通/办公/开发/娱乐/工具 文件夹）
await page.locator('[aria-label="收起"]').first().click().catch(() => {});
await wait(600);
await shot('29-home-organize-done');

await ctx.close();
await browser.close();
console.log('done →', OUT);
