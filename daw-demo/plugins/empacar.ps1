# Arma Beethoven.vst3 y Beethoven.clap en Windows. Uso: daw-demo\plugins\empacar.ps1 [carpeta de salida]
param([string]$Sale = "salida")
$ErrorActionPreference = "Stop"
Push-Location $PSScriptRoot
try {
  cargo build --release
  if ($LASTEXITCODE -ne 0) { exit 1 }
  $meta = (cargo metadata --format-version 1 --no-deps | ConvertFrom-Json).target_directory
  $lib = Join-Path $meta "release\beethoven_plugins.dll"
  Remove-Item -Recurse -Force "$Sale\Beethoven.vst3", "$Sale\Beethoven.clap" -ErrorAction SilentlyContinue
  New-Item -ItemType Directory -Force "$Sale\Beethoven.vst3\Contents\x86_64-win" | Out-Null
  Copy-Item $lib "$Sale\Beethoven.vst3\Contents\x86_64-win\Beethoven.vst3"
  Copy-Item $lib "$Sale\Beethoven.clap"
  Write-Host "Listos en $Sale"
} finally { Pop-Location }
