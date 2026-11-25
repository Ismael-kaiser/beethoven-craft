#!/bin/sh
# Baja el banco de instrumentos grabados (MuseScore General, MIT) a daw-demo/sonidos. No va en git: pesa 40 MB.
set -e
D="$(cd "$(dirname "$0")/.." && pwd)/sonidos"
URL=https://ftp.osuosl.org/pub/musescore/soundfont/MuseScore_General
SUMA=5b85b6c2c61d10b2b91cddd41efcce7b25cd31c8271d511c73afafbef20b6fa3
mkdir -p "$D"
suma() { if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi; }
if [ ! -f "$D/MuseScore_General.sf3" ] || [ "$(suma "$D/MuseScore_General.sf3")" != "$SUMA" ]; then
  curl -fL --retry 3 -o "$D/MuseScore_General.sf3" "$URL/MuseScore_General.sf3"
  [ "$(suma "$D/MuseScore_General.sf3")" = "$SUMA" ] || { echo "el banco bajado no es el esperado"; rm -f "$D/MuseScore_General.sf3"; exit 1; }
fi
[ -f "$D/MuseScore_General_License.md" ] || curl -fL --retry 3 -o "$D/MuseScore_General_License.md" "$URL/MuseScore_General_License.md"
