# Arma "Beethoven Craft-windows.zip" en la carpeta del proyecto. Correr en Windows con Rust instalado:
#   powershell -ExecutionPolicy Bypass -File empaque\empacar-windows.ps1
# Con -Ffmpeg C:\ruta\a\bin copia ffmpeg.exe y ffprobe.exe junto a la app; si no, la app usa los del PATH.
param([string]$Ffmpeg = "")
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

cargo build --release
if ($LASTEXITCODE -ne 0) { exit 1 }
# Beethoven DAW (daw-demo) es otro proyecto de Cargo; compila en el mismo target\.
Push-Location daw-demo
cargo build --release
if ($LASTEXITCODE -ne 0) { exit 1 }
Pop-Location

$T = "dist\Beethoven Craft"
if (Test-Path dist) { Remove-Item dist -Recurse -Force }
New-Item -ItemType Directory -Force $T | Out-Null
Copy-Item target\release\toma-uno.exe "$T\beethoven-craft.exe"
Copy-Item target\release\beethoven-daw-demo.exe "$T\beethoven-daw-demo.exe"
# El banco de instrumentos grabados del DAW (MuseScore General, MIT); no va en git.
$Son = "daw-demo\sonidos"
$Url = "https://ftp.osuosl.org/pub/musescore/soundfont/MuseScore_General"
New-Item -ItemType Directory -Force $Son | Out-Null
if (-not (Test-Path "$Son\MuseScore_General.sf3")) { Invoke-WebRequest "$Url/MuseScore_General.sf3" -OutFile "$Son\MuseScore_General.sf3" }
if ((Get-FileHash "$Son\MuseScore_General.sf3" -Algorithm SHA256).Hash -ne "5B85B6C2C61D10B2B91CDDD41EFCCE7B25CD31C8271D511C73AFAFBEF20B6FA3") { throw "el banco bajado no es el esperado" }
if (-not (Test-Path "$Son\MuseScore_General_License.md")) { Invoke-WebRequest "$Url/MuseScore_General_License.md" -OutFile "$Son\MuseScore_General_License.md" }
Copy-Item $Son "$T\sonidos" -Recurse
Copy-Item modelos\UVR-MDX-NET-Voc_FT.onnx, LICENSE-MIT, LICENSE-APACHE, AVISOS.md, README.md $T
Copy-Item daw-demo\AVISOS.md "$T\AVISOS-DAW.md"
# Licencias de lo que viene adentro.
New-Item -ItemType Directory -Force "$T\recursos" | Out-Null
Copy-Item recursos\*.txt "$T\recursos"
if ($Ffmpeg -ne "") {
    Copy-Item (Join-Path $Ffmpeg "ffmpeg.exe"), (Join-Path $Ffmpeg "ffprobe.exe") $T
    $Lic = Join-Path $Ffmpeg "..\LICENSE.txt"
    if (Test-Path $Lic) { Copy-Item $Lic (Join-Path $T "FFmpeg-LICENSE.txt") }
}
$Zip = "Beethoven Craft-windows.zip"
if (Test-Path $Zip) { Remove-Item $Zip }
Compress-Archive -Path $T -DestinationPath $Zip
Write-Host "Listo: $(Resolve-Path $Zip)"
