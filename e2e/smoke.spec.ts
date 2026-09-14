import { expect, test } from '@playwright/test';

test.describe('仓鼠Hub 冒烟（浏览器预览 + IPC mock）', () => {
  test('首页：玻璃卡渲染 + Hero 时钟 + 添加待办', async ({ page }) => {
    await page.goto('/#/');
    await page.waitForTimeout(1200);

    // 五张卡标题都在（时钟收进 Hero，不再单独成卡）
    for (const title of ['天气', '待办', '倒数日', '最近文件', '常用应用']) {
      await expect(page.locator('section').filter({ hasText: title }).first()).toBeVisible();
    }
    // Hero 时钟在走（格式 HH:MM）
    await expect(page.getByText(/^\d{2}:\d{2}$/).first()).toBeVisible();

    // 添加待办（输入框支持自然语言解析，纯文本无时间表述则原样入库）
    const input = page.getByPlaceholder('添加待办，试试');
    await input.fill('E2E 测试待办');
    await input.press('Enter');
    await expect(page.getByText('E2E 测试待办')).toBeVisible();
  });

  test('首页编排：编辑模式调宽/移除/添加卡片 + 拖拽重排 + 持久化', async ({ page }) => {
    await page.goto('/#/');
    await page.waitForTimeout(1000);

    // 进编辑（右上角「编辑」）：卡片抖动 + 操作钮出现
    await page.getByRole('button', { name: '编辑', exact: true }).click();
    await expect(page.getByTitle('移除卡片').first()).toBeVisible();

    // ⤢ 调宽「待办」卡：4 列 → 8 列
    const todoCard = page.locator('section').filter({ hasText: '待办' }).first();
    await todoCard.locator('[title="切换卡片宽度"]').click();
    await expect(todoCard).toHaveClass(/md:col-span-8/);

    // 拖拽重排：把「倒数日」拖到「天气」上（移到其位置）
    const srcBox = await page.locator('section').filter({ hasText: '倒数日' }).first().boundingBox();
    const dstBox = await page.locator('section').filter({ hasText: '天气' }).first().boundingBox();
    await page.mouse.move(srcBox!.x + 30, srcBox!.y + 30);
    await page.mouse.down();
    await page.mouse.move(dstBox!.x + 30, dstBox!.y + 30, { steps: 10 });
    await page.mouse.up();
    // 松手后 commit 异步渲染，用可重试断言等 DOM 更新（倒数日排到首位）
    await expect(page.locator('main section').first()).toHaveText(/倒数日/, { timeout: 5000 });

    // ✕ 移除「天气」卡
    await page.locator('section').filter({ hasText: '天气' }).first().locator('[title="移除卡片"]').click();
    await expect(page.locator('section').filter({ hasText: '天气' })).toHaveCount(0);

    // + 添加「电脑状态」卡
    await page.getByTitle('添加卡片').click();
    await page.getByRole('button', { name: '电脑状态' }).click();
    await expect(page.locator('section').filter({ hasText: '电脑状态' })).toBeVisible();

    // 完成 → 防抖落库后 reload，布局保持（一份 tiles 两态共用）
    await page.getByRole('button', { name: '完成', exact: true }).click();
    await page.waitForTimeout(900);
    await page.reload();
    await page.waitForTimeout(1000);
    await expect(page.locator('section').filter({ hasText: '电脑状态' })).toBeVisible();
    await expect(page.locator('section').filter({ hasText: '天气' })).toHaveCount(0);
    await expect(page.locator('main section').first()).toHaveText(/倒数日/);
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

  test('搜索页：应用/文件/网页/AI 四路结果聚合', async ({ page }) => {
    await page.goto('/#/search');
    await page.waitForTimeout(800);

    await page.getByPlaceholder('搜索应用、文件、网页…').fill('微信');
    // 应用路（拼音映射命中微信）+ 文件路（微信截图）+ 网页路 + 问 AI（限定 main——Dock tooltip 重名）
    await expect(page.locator('main').getByText('微信', { exact: true })).toBeVisible();
    await expect(page.locator('main').getByText('微信截图_0901.png', { exact: true })).toBeVisible();
    await expect(page.locator('main').getByText('搜索「微信」')).toBeVisible();
    await expect(page.locator('main').getByText(/问 AI：「微信」/)).toBeVisible();
    // 分组标题按序出现（应用 → 文件 → 网页 → AI）
    for (const g of ['应用', '文件', '网页', 'AI']) {
      await expect(page.locator('main').getByText(g, { exact: true }).first()).toBeVisible();
    }
  });

  test('文件页：最近文件 + 类型筛选 + 检索', async ({ page }) => {
    await page.goto('/#/files');
    await page.waitForTimeout(800);

    // 默认最近文件（mtime 降序，报销单最新）；限定 main——路径列与 Dock tooltip 重名
    await expect(page.locator('main').getByText('报销单2026.xlsx', { exact: true })).toBeVisible();
    await expect(page.locator('main').getByText('夜曲.mp3', { exact: true })).toBeVisible();

    // 类型药丸筛选（数据派生）
    await page.getByRole('button', { name: '图片', exact: true }).click();
    await expect(page.locator('main').getByText('微信截图_0901.png', { exact: true })).toBeVisible();
    await expect(page.locator('main').getByText('旅行vlog.mp4', { exact: true })).toHaveCount(0);
    await page.getByRole('button', { name: '全部', exact: true }).click();

    // 检索（子串）
    await page.getByPlaceholder('搜索文件（拼音 / 首字母 / 文件名）…').fill('报销');
    await expect(page.locator('main').getByText('报销单2026.xlsx', { exact: true })).toBeVisible();
    await expect(page.locator('main').getByText('年度总结.docx', { exact: true })).toHaveCount(0);
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

    // 进入桌面模式 → 首页接管形态（壁纸 + 问候 + 搜索 + 小组件），贴边通栏任务栏；
    // 路由不变（同一首页双形态，原地变形）
    await page.getByRole('button', { name: '进入桌面模式' }).click();
    await page.waitForTimeout(600);
    await expect(page).toHaveURL(/#\/$/);
    await expect(page.locator('.ios-dock')).toBeVisible();
    await expect(page.getByText(/(早上好|上午好|中午好|下午好|晚上好|夜深了)，欢迎回来/)).toBeVisible();
    // 限定任务栏：WebDebugBar 的按钮文字也是「退出桌面模式」，会重名
    await expect(page.locator('.ios-dock [aria-label="退出桌面模式"]')).toBeVisible();

    // 任务栏「主屏」→ iOS 图标网格（自带交互 Dock）；冷 dev server 首访 /home
    // 有按需编译延迟，用带超时的断言替代固定等待
    await page.getByRole('button', { name: '主屏' }).click();
    await expect(page.getByPlaceholder('搜索应用')).toBeVisible({ timeout: 10_000 });

    // 主屏「返回桌面」→ 回首页（接管形态）
    await page.getByTitle('返回桌面').click();
    await page.waitForTimeout(500);
    await expect(page).toHaveURL(/#\/$/);

    // 退出 → 回窗口化首页；Dock 常驻（居中胶囊形态）
    await page.locator('.ios-dock [aria-label="退出桌面模式"]').click();
    await page.waitForTimeout(600);
    await expect(page).toHaveURL(/#\/$/);
    await expect(page.locator('.ios-dock')).toBeVisible();
  });

  test('导航丝滑：侧栏直达桌面模式 + 桌面子页返回胶囊/Esc + 设置页状态感知', async ({ page }) => {
    await page.goto('/#/');
    await page.waitForTimeout(800);

    // 侧栏「主屏」一键进入桌面接管并落到主屏（原「桌面」快捷钮已并入主屏入口）
    await page.getByRole('button', { name: '主屏', exact: true }).click();
    await page.waitForTimeout(600);
    await expect(page).toHaveURL(/#\/home$/);

    // 主屏顶栏「桌面」切回桌面双形态首页（仍处接管），再经侧栏快捷链接进设置
    await page.getByRole('button', { name: '桌面', exact: true }).click();
    await page.waitForTimeout(500);
    await page.locator('main').getByRole('button', { name: '设置' }).click();
    await page.waitForTimeout(500);
    await expect(page.getByTitle('返回桌面主页（Esc）')).toBeVisible();
    await expect(page.getByRole('button', { name: '返回桌面主页' })).toBeVisible();
    await page.getByTitle('返回桌面主页（Esc）').click();
    await page.waitForTimeout(500);
    await expect(page).toHaveURL(/#\/$/);

    // 桌面 → 日程，Esc 直接回首页（接管形态）
    await page.getByRole('button', { name: '日程' }).click();
    await page.waitForTimeout(500);
    await page.keyboard.press('Escape');
    await page.waitForTimeout(500);
    await expect(page).toHaveURL(/#\/$/);
  });

  test('窗口化侧栏「主屏」：自动进入接管并直接落到主屏（不再被弹回工作台）', async ({ page }) => {
    await page.goto('/#/');
    await page.waitForTimeout(800);

    await page.getByRole('button', { name: '主屏', exact: true }).click();
    await page.waitForTimeout(800);
    await expect(page).toHaveURL(/#\/home$/);
    await expect(page.getByPlaceholder('搜索应用')).toBeVisible();
  });

  test('通知中心：未读角标 + 分组条目 + 全部已读/单条移除 + 点击跳转', async ({ page }) => {
    await page.goto('/#/');
    await page.waitForTimeout(800);

    // 空态：窗口化标题栏铃铛呼出（Esc 收起）
    await page.getByTestId('notification-bell').click();
    await expect(page.getByTestId('notification-center')).toBeVisible();
    await expect(page.getByText('暂无通知')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByTestId('notification-center')).toHaveCount(0);

    // 造两条通知（预览调试条的样例事件源）→ 未读角标累计
    await page.getByTestId('debug-notify').click();
    await page.getByTestId('debug-notify').click();
    await expect(page.getByTestId('notification-badge')).toHaveText('2');

    // 面板：今天分组 + 两条条目 + 相对时间「刚刚」
    await page.getByTestId('notification-bell').click();
    await expect(page.getByTestId('notification-group-today')).toBeVisible();
    await expect(page.getByTestId('notification-item')).toHaveCount(2);
    await expect(page.getByTestId('notification-item').first()).toContainText('AI Agent 任务完成');
    await expect(page.getByTestId('notification-item').first()).toContainText('刚刚');

    // 全部已读 → 未读点与角标都清掉
    await page.getByRole('button', { name: '全部已读', exact: true }).click();
    await expect(page.getByTestId('notification-dot')).toHaveCount(0);
    await expect(page.getByTestId('notification-badge')).toHaveCount(0);

    // ✕ 单条移除
    await page
      .getByTestId('notification-item')
      .first()
      .getByRole('button', { name: '移除通知' })
      .click();
    await expect(page.getByTestId('notification-item')).toHaveCount(1);

    // 点击条目 → 收起面板并跳到目标页（AI Agent 通知 → /bench）
    await page.getByTestId('notification-item').first().click();
    await expect(page.getByTestId('notification-center')).toHaveCount(0);
    await expect(page).toHaveURL(/#\/bench$/);
  });

  test('通知中心：桌面接管形态从右上角热区呼出 + 历史跨会话留存', async ({ page }) => {
    await page.goto('/#/');
    await page.waitForTimeout(800);

    await page.getByRole('button', { name: '进入桌面模式' }).click();
    await page.waitForTimeout(600);

    // 接管态热区（壁纸之上）的铃铛 → 面板空态
    await page.getByTestId('notification-bell').click();
    await expect(page.getByTestId('notification-center')).toBeVisible();
    await expect(
      page.getByText('待办到点、AI Agent 任务完成、更新就绪都会出现在这里'),
    ).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByTestId('notification-center')).toHaveCount(0);

    // 面板收起时照常收通知（事件订阅在壳层，与面板是否渲染无关）
    await page.getByTestId('debug-notify').click();
    await expect(page.getByTestId('notification-badge')).toHaveText('1');
    await page.getByTestId('notification-bell').click();
    await expect(page.getByTestId('notification-item')).toHaveCount(1);
    await page.keyboard.press('Escape');

    // reload（等价冷启动新会话）：历史与未读从 localStorage 恢复
    await page.reload();
    await page.waitForTimeout(900);
    await expect(page.getByTestId('notification-badge')).toHaveText('1');
    await page.getByTestId('notification-bell').click();
    await expect(page.getByTestId('notification-item')).toHaveCount(1);
  });
});

test.describe('托管分屏（dock 右键「分屏添加」→ 胶囊 → 管理 → 退出）', () => {
  test('加两扇 → 胶囊计数 → 菜单转「移出」→ 管理弹层 → 退出清空', async ({ page }) => {
    await page.goto('/#/');
    await page.waitForTimeout(1200);

    // 分屏入口只在桌面接管态给：先切到接管形态（窗口化胶囊态无分屏项）
    await page.getByRole('button', { name: '进入桌面模式' }).click();
    await page.waitForTimeout(700);
    const dock = page.locator('.ios-dock');
    await expect(dock).toBeVisible();

    // 右键运行中的应用（常用组：微信）→ 菜单出现「分屏添加」。
    // 菜单项用 dispatchEvent 派发：预览态左下角有 WebDebugBar（z-999）会压住
    // 菜单的命中区，真机没有这层调试浮层——这里验的是动作链路本身
    await dock.getByRole('button', { name: '微信' }).click({ button: 'right' });
    const addItem = page.getByRole('button', { name: '分屏添加' });
    await expect(addItem).toBeVisible();
    await addItem.dispatchEvent('click');
    // 加入第一扇 → 任务栏出现状态胶囊
    await expect(page.getByText('分屏 1/4')).toBeVisible();

    // 第二扇（Steam）→ 2/4
    await dock.getByRole('button', { name: 'Steam' }).click({ button: 'right' });
    await page.getByRole('button', { name: '分屏添加' }).dispatchEvent('click');
    await expect(page.getByText('分屏 2/4')).toBeVisible();

    // 已在分屏里的应用：菜单那一行换成「移出分屏」
    await dock.getByRole('button', { name: 'Steam' }).click({ button: 'right' });
    await expect(page.getByRole('button', { name: '移出分屏' })).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('button', { name: '移出分屏' })).toHaveCount(0);

    // 点胶囊 → 管理弹层：两行成员 + 行内移出/关闭 + 底部「退出分屏」
    await page.getByText('分屏 2/4').click();
    const panel = page.getByRole('dialog');
    await expect(panel).toBeVisible();
    await expect(panel.getByText('主窗口')).toHaveCount(2);
    await expect(panel.getByTitle('移出分屏')).toHaveCount(2);
    await expect(panel.getByRole('button', { name: '退出分屏' })).toBeVisible();

    // 移出一扇：窗口留下，会话继续 → 胶囊回到 1/4
    await panel.getByTitle('移出分屏').first().click();
    await expect(page.getByText('分屏 1/4')).toBeVisible();

    // 退出分屏：全清 → 胶囊消失（窗口各自还原）
    await panel.getByRole('button', { name: '退出分屏' }).click();
    await expect(page.getByText(/分屏 \d\/4/)).toHaveCount(0);
  });
});

test.describe('代理工作台（bench，上游域层整合 MVP）', () => {
  test('GUI 流式对话：首页 composer 创建会话 → 假代理流式回一轮', async ({ page }) => {
    await page.goto('/#/bench');
    await page.waitForTimeout(1000);

    // 首页（ZCode 风欢迎态）：问候语 + 内嵌 composer，代理/项目已预选
    await expect(page.getByText(/有什么想让我帮忙的吗/)).toBeVisible();
    await page.locator('textarea[placeholder*="提问"]').fill('E2E：帮我梳理仓库');
    await page.getByTitle('发送并创建会话').click();

    // 首条消息上屏（限定聊天正文，避免命中侧栏同名会话标题）+ 假代理流式输出（思考卡/工具卡/markdown）
    await expect(page.locator('div.whitespace-pre-wrap', { hasText: 'E2E：帮我梳理仓库' })).toBeVisible();
    await expect(page.getByText('思考中…').or(page.getByText('已思考'))).toBeVisible();
    await expect(page.getByText('示例代码：')).toBeVisible({ timeout: 15_000 });
    // 运行徽章随轮次结束消失
    await expect(page.getByText('运行中')).toBeHidden({ timeout: 15_000 });
  });

  test('Recall 全文搜索：命中 + 上下文回看', async ({ page }) => {
    await page.goto('/#/bench');
    await page.waitForTimeout(1000);

    await page.getByTitle('Recall 全文搜索').click();
    await page.locator('input[placeholder*="搜索所有"]').fill('拼音');
    await page.keyboard.press('Enter');
    await expect(page.getByText('修复 Spotlight 拼音首字母搜索的回退').first()).toBeVisible();
    // 命中卡片 → 上下文消息回看
    await expect(page.locator('mark')).toBeVisible();
  });
});

test.describe('AI 助手（/agent，M4 Agent 原生桌面）', () => {
  test('统一底栏输入框：首问启动桌面助手 → 会话 composer 落同一槽位；快问同框直连', async ({ page }) => {
    await page.goto('/#/agent');
    await page.waitForTimeout(1000);

    // 默认进入桌面助手欢迎态；模式切换贴在输入框上方
    await expect(page.getByText(/桌面助手 · 听得懂话/)).toBeVisible();
    await expect(page.getByRole('button', { name: '桌面助手', exact: true })).toBeVisible();

    // 统一底栏输入框发首问 → 启动 bench 助手会话
    await page.getByPlaceholder('给桌面助手下达任务…').fill('E2E：打开计算器');
    await page.getByTitle('启动桌面助手').click();

    // 会话建立后输入框仍常驻同一位置（ChatThread composer portal 进底栏槽位）
    await expect(page.getByPlaceholder('给桌面助手发消息…')).toBeVisible();
    await expect(page.getByText('E2E：打开计算器')).toBeVisible();
    await expect(page.getByText('桌面工具已接入')).toBeVisible();
    await expect(page.getByText('思考中…').or(page.getByText('已思考'))).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('运行中')).toBeHidden({ timeout: 15_000 });

    // 切到快问模式（ai_chat 兜底）不残留助手会话；同一只输入框直接发问
    await page.getByRole('button', { name: '快问', exact: true }).click();
    await expect(page.getByText('问答、写作、翻译、点子')).toBeVisible();
    await page.getByPlaceholder('问点什么…').fill('E2E：你好');
    await page.keyboard.press('Enter');
    await expect(page.getByText(/浏览器 mock 回复/)).toBeVisible({ timeout: 10_000 });
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

    // 受控 IPC 桥（manifest permissions: ["todo.add"]）→ 记入待办
    await page.getByText('记一条「喂仓鼠」到待办').click();
    await expect(page.getByText('✓ 已记入待办')).toBeVisible({ timeout: 10_000 });

    // 回工作台核验待办卡计数（种子 1 未完成 + 桥接新增 1 = 2）
    await page.getByTitle('返回桌面').click();
    await page.waitForTimeout(500);
    await page.locator('.ios-dock [aria-label="退出桌面模式"]').click();
    await page.waitForTimeout(600);
    await expect(page.getByText(/未完成 2/)).toBeVisible({ timeout: 10_000 });
  });
});

test.describe('密码箱（字段级加密，浏览器走明文 mock）', () => {
  test('初始化 → 生成器新建 → 搜索 → 锁定/解锁', async ({ page }) => {
    await page.goto('/#/vault');
    await page.waitForTimeout(800);

    // 未初始化：创建主密码（≥8 字符 + 确认一致）
    await page.getByPlaceholder('至少 8 个字符').fill('e2e-master-pw');
    await page.getByPlaceholder('再输入一次').fill('e2e-master-pw');
    await page.getByRole('button', { name: '创建密码库' }).click();
    await page.waitForTimeout(600);
    await expect(page.getByText(/0 条/)).toBeVisible();

    // 新建条目：生成器造密码后保存
    await page.getByRole('button', { name: '新建', exact: true }).click();
    await page.getByPlaceholder('例如：GitHub').fill('E2E 银行');
    await page.getByPlaceholder('邮箱 / 手机号 / 用户名').fill('me@bank.cn');
    await page.getByRole('button', { name: '生成密码' }).click();
    await page.getByRole('button', { name: '使用', exact: true }).click();
    await page.getByRole('button', { name: '保存', exact: true }).click();
    await page.waitForTimeout(500);
    await expect(page.locator('main').getByText('E2E 银行')).toBeVisible();
    await expect(page.getByText(/1 条/)).toBeVisible();

    // 搜索命中（标题 LIKE）
    await page.getByPlaceholder('搜索标题 / 用户名 / 网址').fill('银行');
    await page.waitForTimeout(600);
    await expect(page.locator('main').getByText('E2E 银行')).toBeVisible();
    await expect(page.locator('main').getByText('me@bank.cn')).toBeVisible();

    // 手动锁定 → 锁屏 → 主密码解锁回来
    await page.getByTitle('锁定').click();
    await page.waitForTimeout(400);
    await expect(page.getByText('密码箱已锁定')).toBeVisible();
    await page.getByPlaceholder('主密码').fill('e2e-master-pw');
    await page.getByRole('button', { name: '解锁', exact: true }).click();
    await page.waitForTimeout(800);
    await expect(page.locator('main').getByText('E2E 银行')).toBeVisible();
  });
});
