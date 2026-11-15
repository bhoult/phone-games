#!/usr/bin/env bash
# Renders every synthesised sound to WAV files so they can be listened to or
# measured outside the browser:   tools/dump_audio.sh [outdir]
set -euo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-target/audio}"
mkdir -p "$OUT" target
cat > target/dump_audio.rs <<'RS'
#[path = "../game/src/synth.rs"]
mod synth;
fn main() {
    let dir = std::env::args().nth(1).unwrap();
    for s in synth::ALL {
        let name = format!("{s:?}").to_lowercase().replace(['(', ')'], "");
        std::fs::write(format!("{dir}/{name}.wav"), synth::render(s)).unwrap();
    }
    std::fs::write(format!("{dir}/music.wav"), synth::music()).unwrap();
}
RS
rustc --edition 2021 -O -A dead_code target/dump_audio.rs -o target/dump_audio
target/dump_audio "$OUT"
echo "wrote $(ls "$OUT" | wc -l) files to $OUT"
