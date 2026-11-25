#!/bin/bash
# Arma «Beethoven.vst3» y «Beethoven.clap» en Mac y Linux. En Windows: empacar.ps1.
# Uso: plugins/empacar.sh [carpeta de salida]. En Mac, con MAC_UNIVERSAL=1, sirve en Apple e Intel.
set -e
cd "$(dirname "$0")"
SALE="${1:-salida}"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
META=$(cargo metadata --format-version 1 --no-deps | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
mkdir -p "$SALE"

if [ "$(uname)" = "Linux" ]; then
  nice cargo build --release -j 5
  rm -rf "$SALE/Beethoven.vst3" "$SALE/Beethoven.clap"
  mkdir -p "$SALE/Beethoven.vst3/Contents/$(uname -m)-linux"
  cp "$META/release/libbeethoven_plugins.so" "$SALE/Beethoven.vst3/Contents/$(uname -m)-linux/Beethoven.so"
  cp "$META/release/libbeethoven_plugins.so" "$SALE/Beethoven.clap"
  echo "Listos en $SALE"
  exit 0
fi

if [ -n "$MAC_UNIVERSAL" ]; then
  for T in aarch64-apple-darwin x86_64-apple-darwin; do
    rustup target add "$T" >/dev/null
    MACOSX_DEPLOYMENT_TARGET=11.0 nice cargo build --release -j 5 --target "$T"
  done
  LIB="$SALE/union.dylib"
  lipo -create "$META/aarch64-apple-darwin/release/libbeethoven_plugins.dylib" "$META/x86_64-apple-darwin/release/libbeethoven_plugins.dylib" -output "$LIB"
else
  MACOSX_DEPLOYMENT_TARGET=11.0 nice cargo build --release -j 5
  LIB="$META/release/libbeethoven_plugins.dylib"
fi
for TIPO in vst3 clap; do
  B="$SALE/Beethoven.$TIPO"
  rm -rf "$B"
  mkdir -p "$B/Contents/MacOS"
  cp "$LIB" "$B/Contents/MacOS/Beethoven"
  printf 'BNDL????' > "$B/Contents/PkgInfo"
  cat > "$B/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>Beethoven</string>
<key>CFBundleIdentifier</key><string>org.beethoven.plugins.$TIPO</string>
<key>CFBundleName</key><string>Beethoven</string>
<key>CFBundlePackageType</key><string>BNDL</string>
<key>CFBundleSignature</key><string>????</string>
<key>CFBundleShortVersionString</key><string>$VERSION</string>
<key>CFBundleVersion</key><string>$VERSION</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
</dict></plist>
PLIST
  codesign --force --sign - "$B" 2>/dev/null
done
rm -f "$SALE/union.dylib"
echo "Listos en $SALE"
