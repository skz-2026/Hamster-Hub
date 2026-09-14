# 本地全量检查（提交前跑一遍，对应 CI 门禁）
# 用法: powershell -File scripts/check.ps1
#
# 注意 --workspace：src-tauri/Cargo.toml 既是 workspace 根也是主包，
# 不带 --workspace 时 cargo 只选 workspace_default_members（= hamster-hub），
# crates/* 下 7 个兄弟包的 clippy 与单测会被整体跳过（2026-09-14 踩到，见 docs/05-pitfalls.md 坑 26）。
#
# dev 实例（pnpm tauri dev）开着时 target/ 会被占用，报 "tauri-build: os error 32"。
# 此时可先设 $env:CARGO_TARGET_DIR = "$root\src-tauri\target-verify" 换个目标目录，
# 不必杀掉正在接管桌面的进程（代价是一次冷编译）。
$ErrorActionPreference = 'stop'
$root = Split-Path -Parent $PSScriptRoot

Write-Host '== pnpm typecheck ==' -ForegroundColor Cyan
pnpm -C $root typecheck
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host '== pnpm build ==' -ForegroundColor Cyan
pnpm -C $root build
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host '== cargo fmt --all --check ==' -ForegroundColor Cyan
cargo fmt --all --manifest-path "$root\src-tauri\Cargo.toml" --check
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host '== cargo clippy (--workspace) ==' -ForegroundColor Cyan
cargo clippy --manifest-path "$root\src-tauri\Cargo.toml" --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host '== cargo test (--workspace) ==' -ForegroundColor Cyan
cargo test --manifest-path "$root\src-tauri\Cargo.toml" --workspace
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host '== ALL GREEN ==' -ForegroundColor Green
