<#
.SYNOPSIS
  Builds the WinRect React UI, embeds it, and publishes a single self-contained WinRect.exe.

.EXAMPLE
  ./build.ps1                 # full release build -> .\publish\WinRect.exe
  ./build.ps1 -SkipWeb        # reuse the existing web\dist (faster iteration)
#>
[CmdletBinding()]
param(
  [string]$Configuration = "Release",
  [string]$Runtime = "win-x64",
  [switch]$SkipWeb
)

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot

# Prefer the user-profile .NET 8 SDK if it's there; otherwise whatever's on PATH.
$dotnet = Join-Path $env:LOCALAPPDATA "Microsoft\dotnet\dotnet.exe"
if (-not (Test-Path $dotnet)) { $dotnet = "dotnet" }

if (-not $SkipWeb) {
  Write-Host "==> Installing & building the React UI..." -ForegroundColor Cyan
  & npm --prefix "$root\web" install --no-audit --no-fund
  if ($LASTEXITCODE -ne 0) { throw "npm install failed" }
  & npm --prefix "$root\web" run build
  if ($LASTEXITCODE -ne 0) { throw "vite build failed" }
}

if (-not (Test-Path "$root\web\dist\index.html")) {
  throw "web\dist\index.html not found. Run without -SkipWeb first."
}

Write-Host "==> Packing web assets into WebAssets.zip..." -ForegroundColor Cyan
$zip = Join-Path $root "WebAssets.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path "$root\web\dist\*" -DestinationPath $zip -Force

Write-Host "==> Publishing single-file self-contained exe ($Configuration / $Runtime)..." -ForegroundColor Cyan
& $dotnet publish "$root\WinRect.csproj" `
  -c $Configuration `
  -r $Runtime `
  --self-contained true `
  -p:PublishSingleFile=true `
  -p:IncludeNativeLibrariesForSelfExtract=true `
  -p:EnableCompressionInSingleFile=true `
  -p:DebugType=none `
  -o "$root\publish"
if ($LASTEXITCODE -ne 0) { throw "dotnet publish failed" }

$exe = Join-Path $root "publish\WinRect.exe"
Write-Host ""
Write-Host "==> Done: $exe" -ForegroundColor Green
Write-Host "    Double-click it (or pin it). It runs in the system tray." -ForegroundColor Green
