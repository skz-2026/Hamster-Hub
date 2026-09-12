import { expect, test } from '@playwright/test';

test.describe('仓鼠Hub 冒烟（浏览器预览 + IPC mock）', () => {
  test('工作台：仪表盘六卡渲染 + 添加待办', async ({ page }) => {
    await page.goto('/#/');
    await page.waitForTimeout(1200);

    // 六张卡标题都在
    for (const title of ['天气', '待办', '倒数日', '最近文件', '常用应用']) {
      await expect(page.locator('.card').filter({ hasText: title }).first()).toBeVisible();
    }
    // 时钟在走（格式 HH:MM:SS）
    await expect(page.getByText(/^\d{2}:\d{2}:\d{2}$/).first()).toBeVisible();

    // 添加待办
    const input = page.getByPlaceholder('添加待办，回车确认');
    await input.fill('E2E 测试待办');
    await input.press('Enter');
    await expect(page.getByText('E2E 测试待办')).toBeVisible();
  });

  test('应用页：分类筛选', async ({ page }) => {
    await page.goto('/#/apps');
    await page.waitForTimeout(1000);

    await expect(page.getByText(/共 \d+ 个/)).toBeVisible();
    await page.getByRole('button', { name: /^开发/ }).click();
    // mock 数据里开发类含 VS Code
    await expect(page.getByText('VS Code')).toBeVisible();
    // 其它分类的应用不再显示（Steam 属娱乐；限定 main——常驻 Dock 的 tooltip 含同名文本）
    await expect(page.locator('main').getByText('Steam', { exact: true })).toHaveCount(0);
  });

  test('日程：便签添加 + 倒数日联动月历', async ({ page }) => {
    await page.goto('/#/schedule');
    await page.waitForTimeout(1000);

    // 今日高亮存在
    await expect(page.locator('.bg-\\[var\\(--accent\\)\\]').first()).toBeVisible();

    // 便签添加
    const noteInput = page.getByPlaceholder('记一条便签');
    await noteInput.fill('E2E 便签内容');
    await noteInput.press('Enter');
    await expect(page.getByText('E2E 便签内容')).toBeVisible();

    // 倒数日添加（今天+7 天）→ 列表出现
    await page.getByPlaceholder('事件名').fill('E2E 事件');
    const d = new Date(Date.now() + 7 * 86400000);
    const iso = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
    await page.locator('input[type="date"]').fill(iso);
    // 提交按钮（倒数日卡里的图标按钮）
    await page.evaluate(() => {
      const card = [...document.querySelectorAll('.card')].find((c) =>
        c.textContent.includes('倒数日'),
      );
      const btn = [...card!.querySelectorAll('button')].find((b) => b.querySelector('svg'));
      btn!.click();
    });
    await expect(page.getByText('E2E 事件')).toBeVisible();
    // 月历上出现 emoji 标记（管理行与月历格都含，取第一个）
    await expect(page.getByText('🎯').first()).toBeVisible();
  });

  test('桌面模式：全屏接管桌面 + 贴边任务栏 + 主屏 + 退出还原', async ({ page }) => {
    await page.goto('/#/');
    await page.waitForTimeout(800);

    // 进入桌面模式 → 全屏桌面主页（壁纸 + 问候 + 搜索 + 小组件），贴边通栏任务栏
    await page.getByRole('button', { name: '进入桌面模式' }).click();
    await page.waitForTimeout(600);
    await expect(page).toHaveURL(/#\/desktop$/);
    await expect(page.locator('.ios-dock')).toBeVisible();
    await expect(page.getByText(/(早上好|上午好|中午好|下午好|晚上好|夜深了)，欢迎回来/)).toBeVisible();
    // 限定任务栏：WebDebugBar 的按钮文字也是「退出桌面模式」，会重名
    await expect(page.locator('.ios-dock [aria-label="退出桌面模式"]')).toBeVisible();

    // 任务栏「主屏」→ iOS 图标网格（自带交互 Dock）
    await page.getByRole('button', { name: '主屏' }).click();
    await page.waitForTimeout(500);
    await expect(page.getByPlaceholder('搜索应用')).toBeVisible();

    // 主屏「返回桌面」→ 回桌面主页
    await page.getByTitle('返回桌面').click();
    await page.waitForTimeout(500);
    await expect(page).toHaveURL(/#\/desktop$/);

    // 退出 → 回窗口化工作台；Dock 常驻（居中胶囊形态）
    await page.locator('.ios-dock [aria-label="退出桌面模式"]').click();
    await page.waitForTimeout(600);
    await expect(page).toHaveURL(/#\/$/);
    await expect(page.locator('.ios-dock')).toBeVisible();
  });
});

test.describe('代理工作台（bench，上游域层整合 MVP）', () => {
  test('GUI 流式对话：首页 composer 创建会话 → 假代理流式回一轮', async ({ page }) => {
    await page.goto('/#/bench');
    await page.waitForTimeout(1000);

    // 首页（ZCode 风欢迎态）：问候语 + 内嵌 composer，代理/项目已预选
    await expect(page.getByText(/有什么想让我帮忙的吗/)).toBeVisible();
    await page.locator('textarea[placeholder*="向代理提问"]').fill('E2E：帮我梳理仓库');
    await page.getByTitle('发送并创建会话').click();

    // 首条消息上屏 + 假代理流式输出（思考卡/工具卡/markdown）
    await expect(page.getByText('E2E：帮我梳理仓库')).toBeVisible();
    await expect(page.getByText('思考中…').or(page.getByText('已思考'))).toBeVisible();
    await expect(page.getByText('示例代码：')).toBeVisible({ timeout: 15_000 });
    // 运行徽章随轮次结束消失
    await expect(page.getByText('运行中')).toBeHidden({ timeout: 15_000 });
  });

  test('Recall 全文搜索：命中 + 上下文回看', async ({ page }) => {
    await page.goto('/#/bench');
    await page.waitForTimeout(1000);

    await page.getByTitle('Recall 全文搜索').click();
    await page.locator('input[placeholder*="搜索所有代理"]').fill('拼音');
    await page.keyboard.press('Enter');
    await expect(page.getByText('修复 Spotlight 拼音首字母搜索的回退').first()).toBeVisible();
    // 命中卡片 → 上下文消息回看
    await expect(page.locator('mark')).toBeVisible();
  });
});

test.describe('AI 助手（/agent，M4 Agent 原生桌面）', () => {
  test('桌面助手：默认模式 + 创建假会话流式回一轮 + 工具徽标；快问模式可切换', async ({ page }) => {
    await page.goto('/#/agent');
    await page.waitForTimeout(1000);

    // 默认进入桌面助手欢迎态（能力说明 + 首问输入）
    await expect(page.getByText(/桌面助手 · 听得懂话/)).toBeVisible();
    await page.locator('input[placeholder*="记成待办"]').fill('E2E：打开计算器');
    await page.getByTitle('启动桌面助手').click();

    // 首条消息上屏 + 假代理流式回一轮（与 bench 同一事件通道）
    await expect(page.getByText('E2E：打开计算器')).toBeVisible();
    await expect(page.getByText('桌面工具已接入')).toBeVisible();
    await expect(page.getByText('思考中…').or(page.getByText('已思考'))).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('运行中')).toBeHidden({ timeout: 15_000 });

    // 切到快问模式（ai_chat 兜底）不残留助手会话
    await page.getByRole('button', { name: '快问', exact: true }).click();
    await expect(page.getByText('问答、写作、翻译、点子')).toBeVisible();
  });
});

test.describe('UI 插件（M4 阶段四：用户可扩展小组件）', () => {
  test('主屏编辑模式添加插件小组件 → blob import 渲染 + 私有存储计数', async ({ page }) => {
    await page.goto('/#/');
    await page.waitForTimeout(800);
    // 主屏仅在桌面接管态可达（非接管态 HomeScreen 会弹回工作台）
    await page.getByRole('button', { name: '进入桌面模式' }).click();
    await page.waitForTimeout(600);
    await page.getByRole('button', { name: '主屏' }).click();
    await page.waitForTimeout(500);

    // 编辑模式 → 添加小组件菜单 → 插件分组「你好仓鼠」
    await page.getByTitle('长按图标整理主屏').click();
    await page.getByTitle('添加小组件').click();
    await page.getByText('🧩 你好仓鼠').click();

    // 插件渲染（mock 代码经 Blob 动态 import，与真机同一通路）
    await expect(page.getByText('你好仓鼠（浏览器预览）')).toBeVisible({ timeout: 10_000 });

    // 退出编辑模式（编辑态有拖拽捕获层，插件不响应交互——iOS 惯例）再交互
    await page.getByTitle('完成').click();
    await page.getByText('囤一口').click();
    await page.getByText('囤一口').click();
    await expect(page.locator('[data-n]')).toHaveText('2');
  });
});
