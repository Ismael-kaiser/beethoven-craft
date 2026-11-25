#!/bin/zsh
# Arma "Beethoven Craft.app" en la carpeta del proyecto.
set -e
cd "$(dirname "$0")/.."
nice cargo build -j 2 --release
# Beethoven DAW (daw-demo) es otro proyecto de Cargo; compila en el mismo target/.
(cd daw-demo && nice cargo build -j 2 --release)
A="Beethoven Craft.app"; T=".empaque-nuevo.app"
rm -rf "$T"; mkdir -p "$T/Contents/MacOS" "$T/Contents/Resources"
cp target/release/toma-uno "$T/Contents/MacOS/beethoven-craft"
cp target/release/beethoven-daw-demo "$T/Contents/MacOS/beethoven-daw-demo"
sh daw-demo/empaque/sonidos.sh && cp -R daw-demo/sonidos "$T/Contents/Resources/sonidos"
cp empaque/Info.plist "$T/Contents/"
cp empaque/icono.icns empaque/proyecto.icns "$T/Contents/Resources/"
cp modelos/UVR-MDX-NET-Voc_FT.onnx LICENSE LICENSE-MIT LICENSE-APACHE AVISOS.md "$T/Contents/Resources/"
cp daw-demo/AVISOS.md "$T/Contents/Resources/AVISOS-DAW.md"
mkdir -p "$T/Contents/Resources/recursos" && cp recursos/*.txt "$T/Contents/Resources/recursos/"
codesign --force --sign - "$T"
# Se cambia de golpe: la app vieja puede estar abierta.
[ -e "$A" ] && mv "$A" ".app-vieja-$$"
mv "$T" "$A"
rm -rf .app-vieja-*(N)
echo "Listo: $PWD/$A"
