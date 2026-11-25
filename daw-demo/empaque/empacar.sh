#!/bin/zsh
# Arma "Beethoven DAW Demo.app" en esta carpeta.
set -e
cd "$(dirname "$0")/.."
nice cargo build -j 4 --release --locked
A="Beethoven DAW Demo.app"; T=".empaque-nuevo.app"
rm -rf "$T"; mkdir -p "$T/Contents/MacOS" "$T/Contents/Resources"
cp ../target/release/beethoven-daw-demo "$T/Contents/MacOS/"
sh empaque/sonidos.sh && cp -R sonidos "$T/Contents/Resources/sonidos"
cp empaque/Info.plist "$T/Contents/"
cp ../empaque/icono.icns "$T/Contents/Resources/"
cp LICENSE-MIT LICENSE-APACHE AVISOS.md ../recursos/Cuprum-OFL.txt "$T/Contents/Resources/"
codesign --force --sign - "$T"
rm -rf "$A"; mv "$T" "$A"
echo "Listo: $PWD/$A"
