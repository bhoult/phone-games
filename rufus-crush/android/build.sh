#!/usr/bin/env bash
# Builds the Android app: the web build, packed into an APK that needs no
# network. Output: android/rufus-crush.apk
#
#   ./build.sh            build
#   ./build.sh install    build, then install on the device adb can see
set -euo pipefail
cd "$(dirname "$0")"

export JAVA_HOME="${JAVA_HOME:-$(ls -d "$HOME"/.local/opt/jdk-21* 2>/dev/null | head -1)}"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
[[ -d "$JAVA_HOME" && -d "$ANDROID_HOME" ]] || { echo "Set JAVA_HOME (JDK 17+) and ANDROID_HOME (Android SDK)."; exit 1; }
echo "sdk.dir=$ANDROID_HOME" > local.properties

# 1. the game itself
../build.sh

# 2. into the APK's assets (no service worker: nothing here comes from a network)
ASSETS=app/src/main/assets/game
rm -rf "$ASSETS"
mkdir -p "$ASSETS/icons" app/src/main/res/drawable-nodpi
cp ../dist/index.html ../dist/rufus.js ../dist/engine.js ../dist/rufus-crush.wasm ../dist/manifest.webmanifest "$ASSETS/"
cp ../dist/icons/*.png "$ASSETS/icons/"
cp ../dist/icons/icon-512.png app/src/main/res/drawable-nodpi/icon_art.png

# 3. the app
./gradlew --quiet assembleRelease
cp app/build/outputs/apk/release/app-release.apk rufus-crush.apk
echo "built android/rufus-crush.apk ($(du -h rufus-crush.apk | cut -f1))"

if [[ "${1:-}" == "install" ]]; then
  "$ANDROID_HOME/platform-tools/adb" install -r rufus-crush.apk
fi
