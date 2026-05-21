#!/usr/bin/env pwsh
# Format all sources across Rust / Dart / TypeScript.

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot

Write-Host "▶ cargo fmt --all" -ForegroundColor Cyan
cargo fmt --all --manifest-path "$Root/Cargo.toml"

Write-Host "▶ dart format (mobile)" -ForegroundColor Cyan
if (Test-Path "$Root/apps/mobile/lib") {
    dart format "$Root/apps/mobile"
}

Write-Host "▶ dart format (desktop)" -ForegroundColor Cyan
if (Test-Path "$Root/apps/desktop/lib") {
    dart format "$Root/apps/desktop"
}

Write-Host "▶ pnpm --filter web-console lint --fix" -ForegroundColor Cyan
pnpm --filter web-console lint --fix 2>$null

Write-Host "✓ Done." -ForegroundColor Green
