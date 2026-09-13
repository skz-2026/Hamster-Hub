# 本地全量检查（提交前跑一遍，对应 CI 门禁）
# 用法: powershell -File scripts/check.ps1
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

Write-Host '== cargo clippy ==' -ForegroundColor Cyan
cargo clippy --manifest-path "$root\src-tauri\Cargo.toml" --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host '== cargo test ==' -ForegroundColor Cyan
cargo test --manifest-path "$root\src-tauri\Cargo.toml"
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host '== ALL GREEN ==' -ForegroundColor Green
