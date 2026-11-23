#!/bin/sh
# Arma "beethoven-craft-linux.tar.gz" en la carpeta del proyecto. Correr en Linux con Rust instalado y, para compilar,
# los paquetes de desarrollo de ALSA y pkg-config (Debian/Ubuntu: sudo apt install libasound2-dev pkg-config).
# Para usarla hace falta ffmpeg (sudo apt install ffmpeg).
set -e
cd "$(dirname "$0")/.."
cargo build --release
# Beethoven DAW es otro proyecto de Cargo.
(cd daw && cargo build --release -p beethoven_daw)

T=dist/beethoven-craft
rm -rf dist
mkdir -p "$T"
cp target/release/toma-uno "$T/beethoven-craft"
cp daw/target/release/beethoven_daw "$T/beethoven-daw"
cp modelos/UVR-MDX-NET-Voc_FT.onnx LICENSE AVISOS.md README.md "$T/"
mkdir -p "$T/recursos" && cp recursos/*.txt "$T/recursos/"
cp daw/THIRD-PARTY-NOTICES.txt "$T/DAW-THIRD-PARTY-NOTICES.txt"
cp daw/crates/os-app/assets/beethoven_daw.png "$T/beethoven-craft.png"
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
