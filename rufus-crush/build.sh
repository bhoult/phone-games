#!/usr/bin/env bash
# Builds the game into ./dist — a folder of static files that can be served
# from anywhere (or installed to a phone's home screen for offline play).
#
#   ./build.sh          build
#   ./build.sh serve    build, then serve on http://localhost:8080
#   ./build.sh pages    build, then copy into ../docs/rufus-crush, which is
#                       what GitHub Pages serves: commit and push to publish
set -euo pipefail
cd "$(dirname "$0")"

TARGET=wasm32-unknown-unknown
rustup target list --installed 2>/dev/null | grep -q "$TARGET" || rustup target add "$TARGET"

cargo build -p rufus-crush --release --target "$TARGET"

rm -rf dist
mkdir -p dist/icons
cp web/index.html web/rufus.js web/manifest.webmanifest dist/
cp web/icons/*.png dist/icons/
cp "target/$TARGET/release/rufus-crush.wasm" dist/

# The JS loader comes straight from the crates that were compiled in, so it
# always matches: miniquad's WebGL glue plus quad-snd's WebAudio glue.
crate_dir() {
  local ver
  ver=$(cargo metadata --format-version 1 --filter-platform "$TARGET" \
    | python3 -c 'import json,sys; n=sys.argv[1]; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"]==n))' "$1")
  ls -d "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/"$1-$ver" | head -1
}
{
  cat "$(crate_dir miniquad)/js/gl.js"
  echo
  echo "(function () {"
  cat "$(crate_dir quad-snd)/js/audio.js"
  echo "})();"
} > dist/engine.js

# Optional size pass if binaryen is installed.
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz dist/rufus-crush.wasm -o dist/rufus-crush.wasm
fi

# Stamp the service worker with a hash of everything it caches, so phones
# pick up a new build instead of sitting on the old cache.
BUILD_ID=$(cat dist/index.html dist/rufus.js dist/engine.js dist/rufus-crush.wasm dist/manifest.webmanifest dist/icons/*.png | sha256sum | cut -c1-12)
sed "s/__BUILD_ID__/$BUILD_ID/" web/sw.js > dist/sw.js

echo "built dist/ ($(du -sh dist | cut -f1), build $BUILD_ID)"

if [[ "${1:-}" == "pages" ]]; then
  rm -rf ../docs/rufus-crush
  mkdir -p ../docs
  cp -r dist ../docs/rufus-crush
  touch ../docs/.nojekyll
  echo "copied to ../docs/rufus-crush: commit and push to publish it"
fi

if [[ "${1:-}" == "serve" ]]; then
  PORT="${PORT:-8080}"
  echo
  echo "  this computer:  http://localhost:$PORT"
  for ip in $(hostname -I 2>/dev/null); do
    case "$ip" in 192.168.*|10.*) echo "  on the wifi:    http://$ip:$PORT   (plays, but only https/localhost can install for offline)";; esac
  done
  echo
  exec python3 -m http.server "$PORT" --directory dist --bind 0.0.0.0
fi
