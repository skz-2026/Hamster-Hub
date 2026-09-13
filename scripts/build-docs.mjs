// 构建手册站点 → dist-docs/（GitHub Pages 部署用，CI 里跑；产物不入库）
// 用法: node scripts/build-docs.mjs
// 输入: docs/06-user-manual.{zh-CN,en}.md + docs/images + docs/*.mp4 + src-tauri/icons/64x64.png
// 输出: index.html / index.en.html（中英落地页）、manual.zh-CN.html / manual.en.html（双语手册）、静态素材
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import MarkdownIt from 'markdown-it';

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const docsDir = path.join(root, 'docs');
const outDir = path.join(root, 'dist-docs');

// GitHub 风格锚点 slug：小写、去标点、空白转连字符（保证手册内「目录」的原生链接可跳转）
const ghSlug = (text) =>
  text.trim().toLowerCase().replace(/[^\p{L}\p{N}\p{M}\s-]/gu, '').replace(/\s+/g, '-');

// 每个页面独立 md 实例（标题 id 注入 + 侧栏目录 + .md 链接改 .html）
const createRenderer = () => {
  const toc = [];
  const md = MarkdownIt({ html: true, linkify: true });

  md.core.ruler.push('heading_ids', (state) => {
    for (const token of state.tokens) {
      if (token.type !== 'heading_open') continue;
      const text = state.tokens[state.tokens.indexOf(token) + 1]?.children
        ?.filter((t) => t.type === 'text' || t.type === 'code_inline')
        .map((t) => t.content).join('') ?? '';
      const id = ghSlug(text);
      token.attrSet('id', id);
      if (token.tag === 'h2' || token.tag === 'h3') toc.push({ level: token.tag, text, id });
    }
  });

  md.core.ruler.push('md_links', (state) => {
    for (const token of state.tokens) {
      if (token.type !== 'inline' || !token.children) continue;
      for (const child of token.children) {
        if (child.type !== 'link_open') continue;
        const href = child.attrGet('href');
        if (href && !/^https?:|^#/.test(href) && href.endsWith('.md')) {
          child.attrSet('href', href.replace(/\.md$/, '.html'));
        }
      }
    }
  });

  const render = (src) => {
    // raw.githubusercontent 视频链接改为站内相对路径（Pages 域名下直连更快）
    return md.render(src).replaceAll('https://raw.githubusercontent.com/skz-2026/Hamster-Hub/main/docs/', '');
  };
  return { render, toc };
};

const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;');

const CSS = `
:root {
  --bg: #faf9f7; --fg: #2b2926; --muted: #8a837b; --line: #e8e4de;
  --card: #ffffff; --accent: #e8833a; --accent-soft: #fdf1e7; --code: #f4f2ef;
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #1c1a18; --fg: #e8e4de; --muted: #968e84; --line: #33302c;
    --card: #262321; --accent: #f0964e; --accent-soft: #3a2c1f; --code: #302c29;
  }
}
* { box-sizing: border-box; }
body {
  margin: 0; background: var(--bg); color: var(--fg);
  font: 16px/1.75 -apple-system, "Segoe UI", "Microsoft YaHei", "PingFang SC", "Noto Sans CJK SC", sans-serif;
}
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }
img, video { max-width: 100%; border-radius: 10px; border: 1px solid var(--line); display: block; margin: 12px auto; }
code { background: var(--code); border-radius: 5px; padding: 1px 6px; font-size: 0.9em; }
pre { background: var(--code); border-radius: 10px; padding: 14px 16px; overflow-x: auto; }
pre code { background: none; padding: 0; }
h1, h2, h3 { line-height: 1.35; }
h2 { margin-top: 2.2em; padding-bottom: 0.3em; border-bottom: 1px solid var(--line); }
table { border-collapse: collapse; width: 100%; margin: 14px 0; }
th, td { border: 1px solid var(--line); padding: 7px 12px; text-align: left; }
th { background: var(--card); }
blockquote { margin: 14px 0; padding: 2px 18px; border-left: 4px solid var(--accent); background: var(--accent-soft); border-radius: 0 10px 10px 0; color: var(--muted); }
.footnote { color: var(--muted); }
.badge { display: inline-block; margin: 3px 4px 3px 0; padding: 3px 12px; border-radius: 999px; background: var(--accent-soft); color: var(--accent); font-size: 0.85em; font-weight: 600; }
.btn { display: inline-block; margin: 6px 8px 6px 0; padding: 10px 22px; border-radius: 12px; background: var(--accent); color: #fff; font-weight: 600; }
.btn:hover { text-decoration: none; filter: brightness(1.08); }
.btn.ghost { background: var(--card); color: var(--fg); border: 1px solid var(--line); }
.videos { display: flex; gap: 14px; flex-wrap: wrap; }
.videos video { flex: 1 1 320px; width: 320px; }
.center { text-align: center; }
/* 顶栏（每页都有：品牌 + 首页/手册/GitHub + 语言切换） */
.topbar { display: flex; justify-content: space-between; align-items: center; max-width: 1160px; margin: 0 auto; padding: 18px 24px; font-weight: 600; }
.topbar .links a, .topbar .links strong { margin-left: 16px; }
.topbar .links strong { color: var(--fg); }
.lang-switch { color: var(--muted); }
/* 手册页：侧栏 + 正文 */
.manual { display: flex; max-width: 1200px; margin: 0 auto; padding: 0 20px; }
.toc { width: 270px; flex-shrink: 0; position: sticky; top: 0; max-height: 100vh; overflow-y: auto; padding: 32px 18px 32px 0; font-size: 0.88em; }
.toc a { display: block; padding: 3px 10px; border-radius: 8px; color: var(--fg); }
.toc a:hover { background: var(--accent-soft); text-decoration: none; }
.toc .lv2 { font-weight: 600; margin-top: 4px; }
.toc .lv3 { padding-left: 28px; color: var(--muted); }
.content { flex: 1; min-width: 0; max-width: 860px; padding: 32px 0 80px 26px; }
@media (max-width: 920px) { .toc { display: none; } .content { padding-left: 0; } }
/* 落地页 */
.hero { max-width: 900px; margin: 0 auto; padding: 64px 24px 80px; }
.hero h1 { font-size: 2.4em; margin: 0.2em 0; }
.tagline { font-size: 1.25em; color: var(--muted); margin: 0 0 18px; }
footer { text-align: center; color: var(--muted); font-size: 0.85em; padding: 30px 0 46px; }
`;

// head：favicon + 每页独立的 lang / title / description
const page = (lang, title, description, body) => `<!doctype html>
<html lang="${lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${title} · 仓鼠Hub Hamster Hub</title>
<meta name="description" content="${description}">
<link rel="icon" type="image/png" href="favicon.png">
<style>${CSS}</style>
</head>
<body>
${body}
<footer>仓鼠Hub · Hamster Hub · <a href="https://github.com/skz-2026/Hamster-Hub">GitHub</a> · MIT License © 2026 skz-2026</footer>
</body>
</html>`;

// 顶栏：nav 链接 + 语言切换（当前语言加粗不跳转）
const topbar = (nav, other, cur) => {
  const [home, manual] = nav;
  return `<div class="topbar"><span>🐹 仓鼠Hub</span><span class="links">${home}<a href="${manual.href}">${manual.label}</a><a href="https://github.com/skz-2026/Hamster-Hub">GitHub</a><span class="lang-switch">${cur === 'zh' ? `<strong>中文</strong><a href="${other}">English</a>` : `<a href="${other}">中文</a><strong>English</strong>`}</span></span></div>`;
};

fs.mkdirSync(outDir, { recursive: true });

// ---- 手册页（中 / 英）----
const manualPages = [
  { src: '06-user-manual.zh-CN.md', out: 'manual.zh-CN.html', lang: 'zh-CN', other: 'manual.en.html', index: 'index.html',
    title: '用户手册', desc: '仓鼠Hub 用户手册 —— 桌面模式、Spotlight 搜索、AI 助手、密码箱全功能指南。' },
  { src: '06-user-manual.en.md', out: 'manual.en.html', lang: 'en', other: 'manual.zh-CN.html', index: 'index.en.html',
    title: 'User Manual', desc: 'Hamster Hub user manual — desktop takeover, Spotlight search, AI assistant, vault and more.' },
];
for (const p of manualPages) {
  const { render, toc } = createRenderer();
  const html = render(fs.readFileSync(path.join(docsDir, p.src), 'utf8'));
  const tocHtml = toc
    .map((t) => `<a class="lv${t.level === 'h2' ? 2 : 3}" href="#${t.id}">${esc(t.text)}</a>`)
    .join('\n');
  const nav = p.lang === 'zh-CN'
    ? [`<a href="${p.index}">首页</a>`, { href: 'manual.zh-CN.html', label: '用户手册' }]
    : [`<a href="${p.index}">Home</a>`, { href: 'manual.en.html', label: 'Manual' }];
  const body = `${topbar(nav, p.other, p.lang === 'zh-CN' ? 'zh' : 'en')}
<div class="manual">
<nav class="toc">${tocHtml}</nav>
<main class="content">${html}</main>
</div>`;
  fs.writeFileSync(path.join(outDir, p.out), page(p.lang, p.title, p.desc, body));
}

// ---- 落地页（中 / 英）----
const zhBody = `
${topbar([`<strong>首页</strong>`, { href: 'manual.zh-CN.html', label: '用户手册' }], 'index.en.html', 'zh')}
<div class="hero center">
  <h1>仓鼠Hub <span class="footnote">Hamster Hub</span></h1>
  <p class="tagline">把桌面，囤进一个窝。· Hoard your desktop into one cozy nest.</p>
  <p>
    <span class="badge">Windows</span><span class="badge">Local-first 数据不出本机</span><span class="badge">Tauri 2</span><span class="badge">Rust</span><span class="badge">React</span><span class="badge">MIT</span>
  </p>
  <p>iOS 风格桌面接管 · AI 助手与 Agent 工作台 · 密码箱 · 待办提醒 · Spotlight 搜索</p>
  <p><a class="btn" href="manual.zh-CN.html">📖 用户手册</a><a class="btn ghost" href="https://github.com/skz-2026/Hamster-Hub">⭐ GitHub</a></p>
  <img src="images/home-desktop.png" alt="仓鼠Hub 桌面模式" width="760">
  <h2 style="border:none">🎬 演示视频</h2>
  <div class="videos"><video src="hamster-hub-manual.mp4" controls muted playsinline></video></div>
</div>`;
fs.writeFileSync(path.join(outDir, 'index.html'), page('zh-CN', 'Windows 本地优先的智能桌面助手',
  '仓鼠Hub —— Windows 本地优先的智能桌面助手。iOS 风格桌面、AI 助手、密码箱，数据不出本机。', zhBody));

const enBody = `
${topbar([`<strong>Home</strong>`, { href: 'manual.en.html', label: 'Manual' }], 'index.html', 'en')}
<div class="hero center">
  <h1>Hamster Hub <span class="footnote">仓鼠Hub</span></h1>
  <p class="tagline">Hoard your desktop into one cozy nest. · 把桌面，囤进一个窝。</p>
  <p>
    <span class="badge">Windows</span><span class="badge">Local-first — data never leaves your PC</span><span class="badge">Tauri 2</span><span class="badge">Rust</span><span class="badge">React</span><span class="badge">MIT</span>
  </p>
  <p>iOS-style desktop takeover · AI assistant &amp; Agent workbench · Password vault · Todo reminders · Spotlight search</p>
  <p><a class="btn" href="manual.en.html">📖 User Manual</a><a class="btn ghost" href="https://github.com/skz-2026/Hamster-Hub">⭐ GitHub</a></p>
  <img src="images/home-desktop.png" alt="Hamster Hub desktop mode" width="760">
  <h2 style="border:none">🎬 Demo Video</h2>
  <div class="videos"><video src="hamster-hub-manual-en.mp4" controls muted playsinline></video></div>
</div>`;
fs.writeFileSync(path.join(outDir, 'index.en.html'), page('en', 'Local-first AI desktop assistant for Windows',
  'Hamster Hub — a local-first AI desktop assistant for Windows. iOS-style desktop, AI assistant, vault; data never leaves your PC.', enBody));

// ---- 素材 ----
fs.cpSync(path.join(docsDir, 'images'), path.join(outDir, 'images'), { recursive: true });
for (const f of fs.readdirSync(docsDir)) {
  if (f.endsWith('.mp4')) fs.copyFileSync(path.join(docsDir, f), path.join(outDir, f));
}
fs.copyFileSync(path.join(root, 'src-tauri', 'icons', '64x64.png'), path.join(outDir, 'favicon.png'));

console.log('OK dist-docs/：index.html + index.en.html + manual.zh-CN.html + manual.en.html + favicon.png + images + mp4');
