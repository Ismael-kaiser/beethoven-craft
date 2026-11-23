#!/bin/zsh
# Arma "Beethoven Craft.app" en la carpeta del proyecto.
set -e
cd "$(dirname "$0")/.."
nice cargo build -j 2 --release
# Beethoven DAW es otro proyecto de Cargo.
(cd daw && nice cargo build -j 2 --release -p beethoven_daw)
A="Beethoven Craft.app"; T=".empaque-nuevo.app"
rm -rf "$T"; mkdir -p "$T/Contents/MacOS" "$T/Contents/Resources"
cp target/release/toma-uno "$T/Contents/MacOS/beethoven-craft"
cp daw/target/release/beethoven_daw "$T/Contents/MacOS/beethoven-daw"
cp empaque/Info.plist "$T/Contents/"
cp empaque/icono.icns empaque/proyecto.icns "$T/Contents/Resources/"
cp modelos/UVR-MDX-NET-Voc_FT.onnx LICENSE AVISOS.md "$T/Contents/Resources/"
codesign --force --sign - "$T"
# Se cambia de golpe: la app vieja puede estar abierta.
[ -e "$A" ] && mv "$A" ".app-vieja-$$"
mv "$T" "$A"
rm -rf .app-vieja-*(N)
echo "Listo: $PWD/$A"
