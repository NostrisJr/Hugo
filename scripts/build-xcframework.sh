#!/usr/bin/env bash
#
# Construit Hugo.xcframework (iOS device + simulateur + macOS) à partir du
# staticlib hugo-ffi, pour intégration dans un projet Swift/Xcode.
#
# Prérequis :
#   - Xcode (pour `xcodebuild` et `lipo`) ;
#   - les cibles Rust Apple :
#       rustup target add aarch64-apple-ios aarch64-apple-ios-sim \
#         x86_64-apple-ios aarch64-apple-darwin x86_64-apple-darwin
#
# Usage : ./scripts/build-xcframework.sh
# Sortie : target/xcframework/Hugo.xcframework

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

LIB="libhugo_ffi.a"
HEADERS="crates/hugo-ffi/include"
OUT="target/xcframework"

rm -rf "$OUT"
mkdir -p "$OUT/ios-sim" "$OUT/macos"

build() {
  echo "▶ build $1"
  cargo build -p hugo-ffi --release --target "$1"
}

# iOS (device, arm64).
build aarch64-apple-ios

# iOS simulateur : binaire universel arm64 + x86_64.
build aarch64-apple-ios-sim
build x86_64-apple-ios
lipo -create \
  "target/aarch64-apple-ios-sim/release/$LIB" \
  "target/x86_64-apple-ios/release/$LIB" \
  -output "$OUT/ios-sim/$LIB"

# macOS : binaire universel arm64 + x86_64.
build aarch64-apple-darwin
build x86_64-apple-darwin
lipo -create \
  "target/aarch64-apple-darwin/release/$LIB" \
  "target/x86_64-apple-darwin/release/$LIB" \
  -output "$OUT/macos/$LIB"

echo "▶ assemblage du XCFramework"
xcodebuild -create-xcframework \
  -library "target/aarch64-apple-ios/release/$LIB" -headers "$HEADERS" \
  -library "$OUT/ios-sim/$LIB" -headers "$HEADERS" \
  -library "$OUT/macos/$LIB" -headers "$HEADERS" \
  -output "$OUT/Hugo.xcframework"

echo "✔ $OUT/Hugo.xcframework"
