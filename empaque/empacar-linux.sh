#!/bin/sh
# Arma "beethoven-craft-linux.tar.gz" en la carpeta del proyecto. Correr en Linux con Rust instalado y, para compilar,
# los paquetes de desarrollo de ALSA y pkg-config (Debian/Ubuntu: sudo apt install libasound2-dev pkg-config).
# Para usarla hace falta ffmpeg (sudo apt install ffmpeg).
set -e
cd "$(dirname "$0")/.."
cargo build --release
# Beethoven DAW es otro proyecto de Cargo.
(cd daw-demo && cargo build --release)

T=dist/beethoven-craft
rm -rf dist
mkdir -p "$T"
cp target/release/toma-uno "$T/beethoven-craft"
cp target/release/beethoven-daw-demo "$T/beethoven-daw-demo"
sh daw-demo/empaque/sonidos.sh && cp -R daw-demo/sonidos "$T/sonidos"
cp modelos/UVR-MDX-NET-Voc_FT.onnx LICENSE LICENSE-MIT LICENSE-APACHE AVISOS.md README.md "$T/"
cp daw-demo/AVISOS.md "$T/AVISOS-DAW.md"
mkdir -p "$T/recursos" && cp recursos/*.txt "$T/recursos/"
cp empaque/icono.png "$T/beethoven-craft.png"
cat > "$T/beethoven-craft.desktop" <<'FIN'
[Desktop Entry]
Type=Application
Name=Beethoven Craft
Comment=Estudio de música y editor de video para hacer covers
Exec=beethoven-craft %f
Icon=beethoven-craft
Categories=AudioVideo;Video;Audio;
MimeType=application/x-beethoven-craft;
FIN
tar -czf beethoven-craft-linux.tar.gz -C dist beethoven-craft
echo "Listo: $PWD/beethoven-craft-linux.tar.gz"
