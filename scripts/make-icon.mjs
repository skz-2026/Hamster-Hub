// 生成占位应用图标：暖深灰圆角方块 + 胡萝卜橙仓鼠脸
// 用法: node scripts/make-icon.mjs [输出路径]  (默认 ../app-icon.png, 1024x1024)
// 正式 logo 就绪后: pnpm tauri icon <png路径> 重新生成全套图标
import { deflateSync } from 'node:zlib';
import { writeFileSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const S = 1024;
const out = resolve(process.argv[2] ?? new URL('../app-icon.png', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1'));
const R = 190; // 圆角半径

// 调色板
const BG = [40, 36, 44];
const FUR = [255, 138, 61];
const FUR_DARK = [224, 110, 38];
const CREAM = [252, 236, 218];
const DARK = [46, 38, 34];
const PINK = [255, 190, 170];

const px = new Uint8Array(S * S * 4);
const put = (i, c) => { px[i] = c[0]; px[i + 1] = c[1]; px[i + 2] = c[2]; px[i + 3] = 255; };

const inRoundRect = (x, y) => {
  const cx = Math.min(Math.max(x, R), S - R), cy = Math.min(Math.max(y, R), S - R);
  return (x - cx) ** 2 + (y - cy) ** 2 <= R * R || (x >= R && x <= S - R) || (y >= R && y <= S - R);
};
const inCircle = (x, y, cx, cy, r) => (x - cx) ** 2 + (y - cy) ** 2 <= r * r;

for (let y = 0; y < S; y++) {
  for (let x = 0; x < S; x++) {
    const i = (y * S + x) * 4;
    if (!inRoundRect(x, y)) { px[i + 3] = 0; continue; } // 透明圆角
    let c = BG;
    const cx = S / 2, cy = S / 2 + 30;
    // 耳朵
    if (inCircle(x, y, cx - 165, cy - 185, 88) || inCircle(x, y, cx + 165, cy - 185, 88)) c = FUR_DARK;
    // 脸
    if (inCircle(x, y, cx, cy, 265)) c = FUR;
    // 腮帮(奶油色椭圆近似: 两个圆)
    if (inCircle(x, y, cx - 108, cy + 62, 118) || inCircle(x, y, cx + 108, cy + 62, 118)) c = CREAM;
    // 眼睛
    if (inCircle(x, y, cx - 98, cy - 55, 26) || inCircle(x, y, cx + 98, cy - 55, 26)) c = DARK;
    // 高光
    if (inCircle(x, y, cx - 90, cy - 63, 8) || inCircle(x, y, cx + 106, cy - 63, 8)) c = [255, 255, 255];
    // 鼻子
    if (inCircle(x, y, cx, cy + 28, 20)) c = PINK;
    // 嘴(小圆点)
    if (inCircle(x, y, cx - 26, cy + 62, 9) || inCircle(x, y, cx + 26, cy + 62, 9)) c = DARK;
    put(i, c);
  }
}

// 组装 PNG
const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc32 = (buf) => {
  let c = 0xffffffff;
  for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};
const chunk = (type, data) => {
  const len = Buffer.alloc(4); len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
};

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(S, 0); ihdr.writeUInt32BE(S, 4);
ihdr[8] = 8; ihdr[9] = 6; // 8bit RGBA
const raw = Buffer.alloc(S * (S * 4 + 1));
for (let y = 0; y < S; y++) {
  raw[y * (S * 4 + 1)] = 0; // filter none
  Buffer.from(px.buffer, y * S * 4, S * 4).copy(raw, y * (S * 4 + 1) + 1);
}
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk('IHDR', ihdr), chunk('IDAT', deflateSync(raw, { level: 9 })), chunk('IEND', Buffer.alloc(0)),
]);
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, png);
console.log('icon written:', out, `${(png.length / 1024).toFixed(0)}KB`);
