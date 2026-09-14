#!/usr/bin/env node
/**
 * 更新链路端到端校验（发布前 / 发布后各跑一次）。
 *
 * 为什么需要它：自更新是「发布后才发现坏了就晚了」的功能——release 还停在草稿、
 * 安装包签名与内嵌公钥不配对、平台键写错，任一项都会让**所有用户静默收不到更新**。
 * 本脚本按应用运行时的同一套逻辑做端到端断言：
 *   1. 拉取 tauri.conf.json 里配置的每个 endpoint 的 latest.json（未认证，等价用户视角）
 *   2. 校验 latest.json 结构 + 平台键（默认 windows-x86_64-nsis）齐全
 *   3. 下载安装包，用**内嵌公钥**验签（minisign：BLAKE2b-512 预哈希 Ed25519 + 全局签名）
 *   4. 与当前版本做语义化比较，判断用户是否真能收到这次更新
 *
 * 用法：
 *   node scripts/verify-updater.mjs                 # 校验配置里的全部端点
 *   node scripts/verify-updater.mjs --endpoint <url> # 校验指定端点（如发布前用 draft 的直链）
 *   node scripts/verify-updater.mjs --skip-download  # 只验 latest.json 结构，不下载安装包
 *
 * 退出码：0 全绿；1 有失败项（CI / 发布清单里可直接当门禁用）。
 */
import crypto from 'node:crypto';
import fs from 'node:fs';

const confPath = new URL('../src-tauri/tauri.conf.json', import.meta.url);
const conf = JSON.parse(fs.readFileSync(confPath, 'utf8'));
const currentVersion = conf.version;
const pubkeyField = conf.plugins?.updater?.pubkey;
const configuredEndpoints = conf.plugins?.updater?.endpoints ?? [];
const PLATFORM_KEY = 'windows-x86_64-nsis';
const TIMEOUT_MS = 30_000;

const args = process.argv.slice(2);
const flag = (name) => args.includes(name);
const valueOf = (name) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};

const endpoints = valueOf('--endpoint') ? [valueOf('--endpoint')] : configuredEndpoints;
const skipDownload = flag('--skip-download');

const b64 = (s) => Buffer.from(s, 'base64');
const b64url = (buf) => buf.toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
const fail = [];
const ok = (msg) => console.log('  ✅ ' + msg);
const bad = (msg) => {
  fail.push(msg);
  console.log('  ❌ ' + msg);
};

/** Tauri 的密钥文件/配置字段是 base64(minisign 文本文件)；解两层拿到二进制结构 */
function decodeKeyMaterial(fieldValue) {
  const text = b64(fieldValue).toString('utf8');
  const lines = text.trim().split('\n').filter((l) => l && !l.startsWith('untrusted comment'));
  return b64(lines[lines.length - 1]);
}

function parsePubkey(field) {
  const raw = decodeKeyMaterial(field);
  if (raw.length !== 42) throw new Error(`公钥长度异常：${raw.length}（期望 42）`);
  return { alg: raw.subarray(0, 2).toString(), keyId: raw.subarray(2, 10), pub: raw.subarray(10, 42) };
}

/** minisign 签名（Tauri 产出）：base64(注释行 + sig 行 + trusted comment 行 + 全局签名行) */
function parseSignature(field) {
  const lines = b64(field)
    .toString('utf8')
    .split('\n')
    .filter((l) => l.length > 0);
  const sigLine = lines.find((l) => !l.startsWith('untrusted comment') && !l.startsWith('trusted comment'));
  const trustedIdx = lines.findIndex((l) => l.startsWith('trusted comment'));
  const raw = b64(sigLine);
  return {
    alg: raw.subarray(0, 2).toString(),
    keyId: raw.subarray(2, 10),
    sig: raw.subarray(10, 74),
    trustedComment: trustedIdx >= 0 ? lines[trustedIdx] : '',
    globalSig: trustedIdx >= 0 && lines[trustedIdx + 1] ? b64(lines[trustedIdx + 1]) : null,
  };
}

function verifySignature(pub, data, sigField) {
  const sig = parseSignature(sigField);
  // minisign 约定：公钥 alg 恒为 "Ed"；签名 alg 大写 D（"ED"）表示预哈希。
  // 二者是同一密钥类型的两种模式，不能要求字符串相等。
  if (sig.alg[0] !== pub.alg[0]) {
    return { ok: false, why: `密钥类型不一致（包 ${sig.alg} / 公钥 ${pub.alg}）` };
  }
  if (!sig.keyId.equals(pub.keyId)) {
    return { ok: false, why: `key_id 不匹配（包 ${sig.keyId.toString('hex')} / 公钥 ${pub.keyId.toString('hex')}）——签名私钥与内嵌公钥不是一对` };
  }
  const key = crypto.createPublicKey({ key: { kty: 'OKP', crv: 'Ed25519', x: b64url(pub.pub) }, format: 'jwk' });
  // 签名 alg = "ED" 时先对内容做 BLAKE2b-512 预哈希（Tauri/rsign 的默认模式）
  const payload = sig.alg === 'ED' ? crypto.createHash('blake2b512').update(data).digest() : data;
  if (!crypto.verify(null, payload, key, sig.sig)) {
    return { ok: false, why: `包体签名校验不通过（模式 ${sig.alg}${sig.alg === 'ED' ? '：BLAKE2b-512 预哈希' : ''}）` };
  }
  if (sig.globalSig) {
    const stripped = sig.trustedComment.replace(/^trusted comment:\s*/, '');
    const globalOk = crypto.verify(null, Buffer.concat([sig.sig, Buffer.from(stripped, 'utf8')]), key, sig.globalSig);
    if (!globalOk) return { ok: false, why: '全局签名校验不通过（trusted comment 被改动）' };
  }
  return { ok: true, alg: sig.alg };
}

/** 语义化版本比较：a > b 返回 1，相等 0，小于 -1（忽略预发布后缀差异，够用即可） */
function compareVersions(a, b) {
  const norm = (v) => v.replace(/^v/, '').split('-')[0].split('.').map((n) => Number.parseInt(n, 10) || 0);
  const [x, y] = [norm(a), norm(b)];
  for (let i = 0; i < Math.max(x.length, y.length); i += 1) {
    const d = (x[i] ?? 0) - (y[i] ?? 0);
    if (d !== 0) return d > 0 ? 1 : -1;
  }
  return 0;
}

async function fetchWithTimeout(url, asBuffer = false) {
  const res = await fetch(url, {
    redirect: 'follow',
    signal: AbortSignal.timeout(TIMEOUT_MS),
    headers: { 'User-Agent': 'hamsterhub-verify-updater' },
  });
  if (!res.ok) return { res, body: null };
  return { res, body: asBuffer ? Buffer.from(await res.arrayBuffer()) : await res.json() };
}

console.log('仓鼠Hub 更新链路校验');
console.log('  当前版本（tauri.conf.json）：%s', currentVersion);
console.log('  期望平台键：%s', PLATFORM_KEY);
console.log('  端点：%s', endpoints.join(', ') || '(未配置)');
console.log('');

const pub = parsePubkey(pubkeyField);
console.log('内嵌公钥：alg=%s key_id=%s', pub.alg, pub.keyId.toString('hex'));

if (endpoints.length === 0) bad('tauri.conf.json 未配置 updater.endpoints');

/** 任一成功端点上的发布版本（用于最后的版本结论） */
let latestVersionSeen = null;

for (const endpoint of endpoints) {
  console.log('\n== 端点 %s ==', endpoint);
  let latest;
  try {
    const { res, body } = await fetchWithTimeout(endpoint);
    if (res.status === 404) {
      bad(`404 —— 点用户视角拿不到更新清单。常见原因：①release 仍是草稿（releaseDraft: true，需手动 Publish）②仓库是私有 ③端点 URL 拼错`);
      continue;
    }
    if (!res.ok) {
      bad(`HTTP ${res.status} ${res.statusText}`);
      continue;
    }
    latest = body;
  } catch (e) {
    bad(`请求失败：${e.message}（国内直连 github.com 可能被墙，测试时可用代理）`);
    continue;
  }

  if (!latest || typeof latest !== 'object') {
    bad('latest.json 不是合法 JSON 对象');
    continue;
  }
  ok(`latest.json 可获取（version=${latest.version} pub_date=${latest.pub_date ?? '—'}）`);
  if (!latest.version) bad('latest.json 缺 version 字段');
  else latestVersionSeen = latest.version;

  const entry = latest.platforms?.[PLATFORM_KEY];
  if (!entry) {
    bad(`latest.json 缺平台键 ${PLATFORM_KEY}（现有：${Object.keys(latest.platforms ?? {}).join(', ') || '无'}）——用户端会报「找不到匹配的更新」`);
    continue;
  }
  ok(`平台键 ${PLATFORM_KEY} 存在`);
  if (!entry.url) bad('平台条目缺 url');
  if (!entry.signature) bad('平台条目缺 signature');
  if (!entry.url || !entry.signature || skipDownload) continue;

  console.log('  下载安装包校验签名：%s', entry.url.split('/').pop());
  let artifact;
  try {
    const { res, body } = await fetchWithTimeout(entry.url, true);
    if (!res.ok) {
      bad(`安装包下载失败 HTTP ${res.status}（资产名/发布状态异常）`);
      continue;
    }
    artifact = body;
    ok(`安装包下载成功（${(artifact.length / 1048576).toFixed(1)} MB）`);
  } catch (e) {
    bad(`安装包下载失败：${e.message}`);
    continue;
  }

  const verdict = verifySignature(pub, artifact, entry.signature);
  if (verdict.ok) ok(`签名校验通过（模式 ${verdict.alg}，与内嵌公钥配对，用户端可安装）`);
  else bad(`签名校验失败：${verdict.why}`);
  // 顺带确认签名文件本身也在链上（.sig 资产）
  const sigUrl = artUrlToSigUrl(entry.url);
  if (sigUrl) {
    try {
      const { res } = await fetchWithTimeout(sigUrl);
      if (res.ok) ok('.sig 资产存在（部分手动安装流程需要）');
      else console.log('  ⚠️ .sig 资产不存在（%s → HTTP %s），自动更新不需要它，手动校验会缺一份', sigUrl.split('/').pop(), res.status);
    } catch {
      console.log('  ⚠️ .sig 资产探测失败（网络原因，忽略）');
    }
  }
}

function artUrlToSigUrl(url) {
  const name = url.split('/').pop();
  if (!name || name.endsWith('.sig')) return null;
  return url.replace(/\/[^/]+$/, '') + '/' + name + '.sig';
}

console.log('\n== 版本结论 ==');
if (!latestVersionSeen) {
  console.log('  ⚠️ 未取到任何发布版本，无法判断用户能否升级（见上方失败项）');
} else {
  const cmp = compareVersions(latestVersionSeen, currentVersion);
  console.log('  线上发布版本 %s vs 本地构建版本 %s', latestVersionSeen, currentVersion);
  if (cmp > 0) ok(`线上更高：装 ${currentVersion} 的用户能收到 ${latestVersionSeen} 的更新`);
  else if (cmp === 0) {
    console.log('  ℹ️ 线上与本地同版本：用户端会显示「已是最新」——若这次要发新功能，请先升 package.json / tauri.conf.json / Cargo.toml 的版本号');
  } else {
    bad(`线上版本低于本地（${latestVersionSeen} < ${currentVersion}）：用户不会被提示升级，检查是否发错 tag`);
  }
}

if (fail.length > 0) {
  console.log('\n结果：%d 项失败 ❌', fail.length);
  fail.forEach((f) => console.log('  - ' + f));
  console.log('\n排查顺序：release 是否已 Publish（不能停在草稿，预发布也不进 /releases/latest）→ 版本号是否已升 → 平台键是否为 windows-x86_64-nsis → 签名私钥是否与内嵌公钥配对（用 git 历史确认公钥从未更换）。');
  process.exit(1);
}
console.log('\n结果：全部通过 ✅ 用户可以收到更新');
