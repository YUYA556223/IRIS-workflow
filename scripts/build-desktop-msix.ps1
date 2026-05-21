#!/usr/bin/env pwsh
# Build apps/desktop and package as MSIX (P9).
#
# Prerequisites:
#  - Flutter SDK with Windows desktop enabled
#  - msix dev_dependency installed (`flutter pub get` in apps/desktop)
#  - Visual Studio Build Tools 2022 (Desktop development with C++)
#
# Optional signing:
#  - Set $env:IRIS_SIGN_CERT (path to .pfx) and $env:IRIS_SIGN_PASSWORD,
#    then uncomment certificate_path/certificate_password in pubspec.yaml.

[CmdletBinding()]
param(
    [switch]$Install
)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$DesktopDir = Join-Path $Root 'apps\desktop'

Write-Host "▶ flutter pub get" -ForegroundColor Cyan
Push-Location $DesktopDir
try {
    flutter pub get

    Write-Host "▶ flutter build windows --release" -ForegroundColor Cyan
    flutter build windows --release

    Write-Host "▶ dart run msix:create" -ForegroundColor Cyan
    dart run msix:create

    $msix = Get-ChildItem -Recurse -Filter '*.msix' build/windows | Select-Object -First 1
    if ($msix) {
        Write-Host "`n✓ MSIX created at:" -ForegroundColor Green
        Write-Host "  $($msix.FullName)"
        if ($Install) {
            Write-Host "▶ Installing via Add-AppxPackage" -ForegroundColor Cyan
            Add-AppxPackage -Path $msix.FullName
        }
    } else {
        Write-Warning "MSIX file not found under build/windows"
    }
} finally {
    Pop-Location
}
