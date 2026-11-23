# Arma "Beethoven Craft-windows.zip" en la carpeta del proyecto. Correr en Windows con Rust instalado:
#   powershell -ExecutionPolicy Bypass -File empaque\empacar-windows.ps1
# Con -Ffmpeg C:\ruta\a\bin copia ffmpeg.exe y ffprobe.exe junto a la app; si no, la app usa los del PATH.
param([string]$Ffmpeg = "")
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

cargo build --release
if ($LASTEXITCODE -ne 0) { exit 1 }
# Beethoven DAW es otro proyecto de Cargo.
Push-Location daw
cargo build --release -p beethoven_daw
if ($LASTEXITCODE -ne 0) { exit 1 }
Pop-Location

$T = "dist\Beethoven Craft"
if (Test-Path dist) { Remove-Item dist -Recurse -Force }
New-Item -ItemType Directory -Force $T | Out-Null
Copy-Item target\release\toma-uno.exe "$T\beethoven-craft.exe"
Copy-Item daw\target\release\beethoven_daw.exe "$T\beethoven-daw.exe"
Copy-Item modelos\UVR-MDX-NET-Voc_FT.onnx, LICENSE, AVISOS.md, README.md $T
# Licencias de lo que viene adentro.
New-Item -ItemType Directory -Force "$T\recursos" | Out-Null
Copy-Item recursos\*.txt "$T\recursos"
Copy-Item daw\THIRD-PARTY-NOTICES.txt "$T\DAW-THIRD-PARTY-NOTICES.txt"
if ($Ffmpeg -ne "") {
    Copy-Item (Join-Path $Ffmpeg "ffmpeg.exe"), (Join-Path $Ffmpeg "ffprobe.exe") $T
    $Lic = Join-Path $Ffmpeg "..\LICENSE.txt"
    if (Test-Path $Lic) { Copy-Item $Lic (Join-Path $T "FFmpeg-LICENSE.txt") }
}
$Zip = "Beethoven Craft-windows.zip"
if (Test-Path $Zip) { Remove-Item $Zip }
Compress-Archive -Path $T -DestinationPath $Zip
Write-Host "Listo: $(Resolve-Path $Zip)"
