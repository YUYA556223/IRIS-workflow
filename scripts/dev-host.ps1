#!/usr/bin/env pwsh
# Start IRIS-workflow host services in parallel for development.
# - postgres + mosquitto via docker compose
# - host-backend (cargo run)
# - web-console (pnpm dev)
#
# Stop with Ctrl+C; docker containers keep running (stop via `docker compose down`).

[CmdletBinding()]
param(
    [switch]$NoDocker
)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot

if (-not $NoDocker) {
    Write-Host "▶ Starting docker services (postgres, mosquitto)..." -ForegroundColor Cyan
    docker compose -f "$Root/infra/docker/docker-compose.yml" up -d
}

Write-Host "▶ Launching host-backend and web-console in parallel..." -ForegroundColor Cyan

$backend = Start-Process -FilePath "cargo" -ArgumentList "run", "-p", "host-backend" -WorkingDirectory $Root -PassThru -NoNewWindow
$web     = Start-Process -FilePath "pnpm"  -ArgumentList "--filter", "web-console", "dev" -WorkingDirectory $Root -PassThru -NoNewWindow

Write-Host "  - host-backend  PID=$($backend.Id)" -ForegroundColor Green
Write-Host "  - web-console   PID=$($web.Id)"     -ForegroundColor Green
Write-Host ""
Write-Host "Press Ctrl+C to stop both processes." -ForegroundColor Yellow

try {
    Wait-Process -Id $backend.Id, $web.Id
} finally {
    if (-not $backend.HasExited) { Stop-Process -Id $backend.Id -Force }
    if (-not $web.HasExited)     { Stop-Process -Id $web.Id     -Force }
}
