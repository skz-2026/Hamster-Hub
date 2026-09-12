/**
 * Markdown 渲染（markdown-it + highlight.js，与 上游 同栈）：
 * 统一实例 + 单次构建，html 关闭防注入；代码块走 hljs 高亮。
 */
import { useMemo } from 'react';
import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js';
import 'highlight.js/styles/github-dark.css';

const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
  highlight(code, lang) {
    if (lang && hljs.getLanguage(lang)) {
      try {
        return hljs.highlight(code, { language: lang, ignoreIllegals: true }).value;
      } catch {
        /* 落入无高亮分支 */
      }
    }
    return '';
  },
});

export default function Markdown({ text }: { text: string }) {
  const html = useMemo(() => md.render(text), [text]);
  return (
    <div
      className="bench-md text-[13.5px] leading-relaxed"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
