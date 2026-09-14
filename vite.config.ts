/// <reference types="vitest/config" />
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { fileURLToPath, URL } from 'node:url';
import { readFileSync } from 'node:fs';

// 版本单一来源：package.json。注入成 __APP_VERSION__，前端（含 ipc-mock 的浏览器预览桩）
// 就永远不用手写版本号，避免「改了 package.json 忘了改别处」的漂移。
const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8'));

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  // vitest 只跑单元测试；Playwright E2E 在 e2e/ 由 pnpm e2e 单独跑
  test: {
    exclude: ['e2e/**', 'node_modules/**', 'dist/**'],
  },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  // Tauri 固定端口，避免 devUrl 失联；忽略 Rust 构建产物（否则 vite watch 会被
  // 写入中的 target/*.dll 触发 EBUSY 崩溃）
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**', '**/e2e/**', '**/scripts/**', '**/test-results/**'],
    },
  },
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    target: 'chrome105',
    minify: 'esbuild',
    sourcemap: false,
  },
});
