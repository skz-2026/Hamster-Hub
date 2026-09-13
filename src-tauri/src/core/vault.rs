//! 密码箱加密原语与会话状态（纯计算/内存态，无 IO；DB 读写见 store/vault.rs）。
//!
//! 字段级加密：title/username/url 明文入库供 SQL 搜索；密码与备注序列化为
//! JSON 后整体装进信封（0x01 | nonce 12B | AES-256-GCM 密文+tag）。
//! 主密钥 = Argon2id(主密码, 随机盐)，只存在 [`VaultSession`] 内存中，
//! 锁定/退出即丢弃（zeroize），永不过 IPC、永不落盘。

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rand_core::{OsRng, RngCore};
use zeroize::Zeroizing;

use crate::error::AppError;

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 12;
/// GCM tag 长度（信封最短长度校验用）
const TAG_LEN: usize = 16;
/// 信封版本字节；算法升级时递增并在 open 处分支迁移
pub const ENVELOPE_V1: u8 = 0x01;
/// 校验密文的已知明文（解锁时验证主密码是否正确）
const VERIFIER_PLAINTEXT: &[u8] = b"hamsterhub-vault-v1";
/// 默认自动锁定闲置时长（秒）
pub const DEFAULT_AUTO_LOCK_SECS: u32 = 300;
/// 复制密码后剪贴板自动清除延时（秒）
pub const CLIPBOARD_CLEAR_SECS: u32 = 15;

fn crypto_err(msg: impl Into<String>) -> AppError {
    AppError::new("CRYPTO", msg)
}

/// KDF 参数（JSON 存 vault_meta.kdf_params；m 单位 KiB，19456 = 19MiB，OWASP 推荐）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct KdfParams {
    pub m: u32,
    pub t: u32,
    pub p: u32,
    pub salt: String,
}

pub fn fresh_params() -> KdfParams {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    KdfParams {
        m: 19_456,
        t: 2,
        p: 1,
        salt: B64.encode(salt),
    }
}

/// Argon2id 派生主密钥
pub fn derive_key(
    password: &str,
    params: &KdfParams,
) -> Result<Zeroizing<[u8; KEY_LEN]>, AppError> {
    let salt = B64
        .decode(&params.salt)
        .map_err(|e| crypto_err(format!("盐值解码失败: {e}")))?;
    let argon_params = Params::new(params.m, params.t, params.p, None)
        .map_err(|e| crypto_err(format!("KDF 参数非法: {e}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    argon
        .hash_password_into(password.as_bytes(), &salt, key.as_mut())
        .map_err(|e| crypto_err(format!("密钥派生失败: {e}")))?;
    Ok(key)
}

/// 明文装信封（每次随机 nonce，同一明文两次加密结果不同）
pub fn seal(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<Vec<u8>, AppError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|_| crypto_err("加密失败"))?;
    let mut envelope = Vec::with_capacity(1 + NONCE_LEN + ct.len());
    envelope.push(ENVELOPE_V1);
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(&ct);
    Ok(envelope)
}

/// 拆信封；密钥不匹配或密文被篡改时 AEAD 校验失败 → CRYPTO 错误
pub fn open(key: &[u8; KEY_LEN], envelope: &[u8]) -> Result<Vec<u8>, AppError> {
    if envelope.len() < 1 + NONCE_LEN + TAG_LEN || envelope[0] != ENVELOPE_V1 {
        return Err(crypto_err("密文信封格式非法"));
    }
    let (nonce, ct) = envelope[1..].split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(nonce), ct)
        .map_err(|_| crypto_err("解密失败（密钥不匹配或数据被篡改）"))
}

/// 生成解锁校验密文
pub fn make_verifier(key: &[u8; KEY_LEN]) -> Result<Vec<u8>, AppError> {
    seal(key, VERIFIER_PLAINTEXT)
}

/// 校验主密码：能解开 verifier 即通过
pub fn verify(key: &[u8; KEY_LEN], verifier: &[u8]) -> bool {
    open(key, verifier)
        .map(|plain| plain == VERIFIER_PLAINTEXT)
        .unwrap_or(false)
}

// ===== 密码生成 =====

/// 生成选项（length 8..=64；至少启用一个字符类）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct GenOptions {
    pub length: u8,
    pub upper: bool,
    pub lower: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
}

const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{}<>?,.";
/// 形近字符（I l 1 O 0 o）在勾选去歧义时剔除
const AMBIGUOUS: &str = "Il1O0o";

/// 拒绝采样取 [0, n) 均匀随机数（避开模偏差）
fn rand_index(n: usize) -> usize {
    debug_assert!(n > 0);
    let n64 = n as u64;
    let zone = (u64::from(u32::MAX) + 1) / n64 * n64;
    loop {
        let r = u64::from(OsRng.next_u32());
        if r < zone {
            return (r % n64) as usize;
        }
    }
}

pub fn generate_password(opts: &GenOptions) -> Result<String, AppError> {
    let length = opts.length.clamp(8, 64) as usize;
    let mut classes: Vec<&str> = Vec::new();
    if opts.upper {
        classes.push(UPPER);
    }
    if opts.lower {
        classes.push(LOWER);
    }
    if opts.digits {
        classes.push(DIGITS);
    }
    if opts.symbols {
        classes.push(SYMBOLS);
    }
    if classes.is_empty() {
        return Err(AppError::validate("至少启用一个字符类"));
    }
    // 按字符过滤歧义字符（字符类均为 ASCII，按 char 收集安全）
    let filtered: Vec<Vec<char>> = classes
        .iter()
        .map(|c| -> Vec<char> {
            c.chars()
                .filter(|ch| !opts.exclude_ambiguous || !AMBIGUOUS.contains(*ch))
                .collect()
        })
        .filter(|c| !c.is_empty())
        .collect();
    if filtered.is_empty() {
        return Err(AppError::validate("去歧义后无可用字符"));
    }

    // 每个启用的类先保底 1 个字符，再补齐随机字符，最后洗牌
    let mut chars: Vec<char> = filtered.iter().map(|c| c[rand_index(c.len())]).collect();
    let pool: Vec<char> = filtered.concat();
    while chars.len() < length {
        chars.push(pool[rand_index(pool.len())]);
    }
    for i in (1..chars.len()).rev() {
        let j = rand_index(i + 1);
        chars.swap(i, j);
    }
    chars.truncate(length);
    Ok(chars.into_iter().collect())
}

/// 密码强度估算 0..=4（熵值分段 + 重复率惩罚；纯启发式，供 UI 展示）
pub fn password_strength(password: &str) -> u8 {
    let chars: Vec<char> = password.chars().collect();
    if chars.is_empty() {
        return 0;
    }
    let has_upper = chars.iter().any(|c| c.is_ascii_uppercase());
    let has_lower = chars.iter().any(|c| c.is_ascii_lowercase());
    let has_digit = chars.iter().any(|c| c.is_ascii_digit());
    let has_symbol = chars.iter().any(|c| !c.is_ascii_alphanumeric());
    let mut pool = 0f64;
    if has_upper {
        pool += 26.0;
    }
    if has_lower {
        pool += 26.0;
    }
    if has_digit {
        pool += 10.0;
    }
    if has_symbol {
        pool += 32.0;
    }
    let entropy = chars.len() as f64 * pool.log2();
    // 重复率过高（如 aaaaaaaa）封顶 2
    let unique = chars.iter().collect::<std::collections::HashSet<_>>().len();
    let repeat_ratio = 1.0 - unique as f64 / chars.len() as f64;
    let score = if entropy < 28.0 {
        0
    } else if entropy < 40.0 {
        1
    } else if entropy < 60.0 {
        2
    } else if entropy < 90.0 {
        3
    } else {
        4
    };
    if repeat_ratio > 0.6 {
        score.min(2)
    } else {
        score
    }
}

// ===== 会话状态（lib.rs AppState 持有；密钥不出该模块除外的 commands 层） =====

/// 解锁态：主密钥 + 活跃时间（自动锁看门狗判定用）
#[derive(Debug, Clone)]
pub struct VaultSession {
    key: Option<Zeroizing<[u8; KEY_LEN]>>,
    last_active: std::time::Instant,
    auto_lock_secs: u32,
    failed_attempts: u32,
}

impl Default for VaultSession {
    fn default() -> Self {
        Self::locked()
    }
}

impl VaultSession {
    pub fn locked() -> Self {
        Self {
            key: None,
            last_active: std::time::Instant::now(),
            auto_lock_secs: DEFAULT_AUTO_LOCK_SECS,
            failed_attempts: 0,
        }
    }

    pub fn is_unlocked(&self) -> bool {
        self.key.is_some()
    }

    pub fn auto_lock_secs(&self) -> u32 {
        self.auto_lock_secs
    }

    /// 更新闲置锁定时长（设置命令用；不影响密钥与活跃时间）
    pub fn set_auto_lock(&mut self, secs: u32) {
        self.auto_lock_secs = secs;
    }

    /// 连续失败次数（解锁命令做递增延迟用）
    pub fn failed_attempts(&self) -> u32 {
        self.failed_attempts
    }

    pub fn note_failed_unlock(&mut self) {
        self.failed_attempts = self.failed_attempts.saturating_add(1);
    }

    pub fn note_successful_unlock(&mut self) {
        self.failed_attempts = 0;
    }

    pub fn unlock(&mut self, key: Zeroizing<[u8; KEY_LEN]>, auto_lock_secs: u32) {
        self.key = Some(key);
        self.last_active = std::time::Instant::now();
        self.auto_lock_secs = auto_lock_secs;
    }

    /// 丢弃密钥（手动锁定 / 看门狗闲置锁定 / 退出）
    pub fn lock(&mut self) {
        self.key = None;
    }

    /// 命令入口统一取密钥：未解锁报错，已解锁刷新活跃时间
    pub fn key_touch(&mut self) -> Result<Zeroizing<[u8; KEY_LEN]>, AppError> {
        let key = self
            .key
            .clone()
            .ok_or_else(|| AppError::new("VAULT_LOCKED", "密码箱未解锁"))?;
        self.last_active = std::time::Instant::now();
        Ok(key)
    }

    /// 看门狗轮询：已解锁且闲置超时 → 锁定并返回 true
    pub fn lock_if_idle(&mut self) -> bool {
        if self.key.is_some() && self.last_active.elapsed().as_secs() >= self.auto_lock_secs as u64
        {
            self.lock();
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_open_roundtrip() {
        let key = Zeroizing::new([7u8; KEY_LEN]);
        let env = seal(&key, b"secret-json").unwrap();
        assert_eq!(env[0], ENVELOPE_V1);
        assert_eq!(open(&key, &env).unwrap(), b"secret-json");
        // 同明文两次加密 nonce 不同
        assert_ne!(seal(&key, b"secret-json").unwrap(), env);
    }

    #[test]
    fn open_rejects_wrong_key_and_tamper() {
        let key = Zeroizing::new([1u8; KEY_LEN]);
        let env = seal(&key, b"secret").unwrap();
        assert!(open(&Zeroizing::new([2u8; KEY_LEN]), &env).is_err());
        let mut tampered = env.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0xff;
        assert!(open(&key, &tampered).is_err());
        assert!(open(&key, &env[..env.len() - 1]).is_err());
        assert!(open(&key, &[]).is_err());
    }

    #[test]
    fn verifier_roundtrip_and_derive() {
        let params = fresh_params();
        let key = derive_key("correct horse", &params).unwrap();
        // 派生确定性：同参数同密码同密钥
        assert_eq!(
            derive_key("correct horse", &params).unwrap().as_ref(),
            key.as_ref()
        );
        // 换盐换密钥
        let params2 = fresh_params();
        assert_ne!(
            derive_key("correct horse", &params2).unwrap().as_ref(),
            key.as_ref()
        );
        let verifier = make_verifier(&key).unwrap();
        assert!(verify(&key, &verifier));
        let wrong = derive_key("wrong", &params).unwrap();
        assert!(!verify(&wrong, &verifier));
    }

    #[test]
    fn generate_respects_options() {
        let pw = generate_password(&GenOptions {
            length: 20,
            upper: true,
            lower: true,
            digits: false,
            symbols: false,
            exclude_ambiguous: true,
        })
        .unwrap();
        assert_eq!(pw.chars().count(), 20);
        assert!(pw.chars().all(|c| c.is_ascii_alphabetic()));
        assert!(!pw.chars().any(|c| AMBIGUOUS.contains(c)));

        let full = generate_password(&GenOptions {
            length: 64,
            upper: true,
            lower: true,
            digits: true,
            symbols: true,
            exclude_ambiguous: false,
        })
        .unwrap();
        assert!(full.chars().any(|c| c.is_ascii_uppercase()));
        assert!(full.chars().any(|c| c.is_ascii_lowercase()));
        assert!(full.chars().any(|c| c.is_ascii_digit()));
        assert!(full.chars().any(|c| !c.is_ascii_alphanumeric()));

        // 长度夹紧 + 空字符类拒绝
        assert_eq!(
            generate_password(&GenOptions {
                length: 4,
                upper: true,
                lower: false,
                digits: false,
                symbols: false,
                exclude_ambiguous: false
            })
            .unwrap()
            .chars()
            .count(),
            8
        );
        assert!(generate_password(&GenOptions {
            length: 16,
            upper: false,
            lower: false,
            digits: false,
            symbols: false,
            exclude_ambiguous: false
        })
        .is_err());
    }

    #[test]
    fn strength_bands() {
        assert_eq!(password_strength(""), 0);
        assert_eq!(password_strength("abc"), 0);
        assert_eq!(password_strength("aaaaaaaa"), 1); // 8 字符低熵 + 高重复
        assert_eq!(password_strength("abcdefgh"), 1);
        assert!(password_strength("Tr0ub4dor&3hamst3r!") >= 3);
        assert_eq!(password_strength("vX9#kQ2m$Lp7!zR5w&N4"), 4);
    }

    #[test]
    fn session_lock_flow() {
        let mut s = VaultSession::locked();
        assert!(!s.is_unlocked());
        assert!(s.key_touch().is_err());
        s.unlock(Zeroizing::new([3u8; KEY_LEN]), 60);
        assert!(s.is_unlocked());
        assert!(s.key_touch().is_ok());
        assert!(!s.lock_if_idle()); // 刚解锁不闲置
        s.lock();
        assert!(!s.is_unlocked());
    }
}
