#!/usr/bin/env python3
"""Runs the balance tuner and writes each stage's wave sizes and boss
toughness into core/src/level.rs. Re-run after changing a stage's map,
invaders or waves, anything about structures or creatures, or the curve at
the top of core/examples/balance.rs. Takes a few minutes."""
import re, subprocess, pathlib
root = pathlib.Path(__file__).resolve().parent.parent
path = root / "core/src/level.rs"
MARK = "    LevelDef {"
out = subprocess.run(["cargo", "run", "-q", "-p", "biome-core", "--release", "--example", "balance", "tune"],
                     cwd=root, capture_output=True, text=True, check=True)
print(out.stdout)
tune = {int(i): (b, crowd) for i, b, crowd in re.findall(r"^TUNE (\d+) ([\d.]+) (.+)$", out.stderr, re.M)}
head, *blocks = path.read_text().split(MARK)
assert len(blocks) == len(tune), (len(blocks), len(tune))
for i, b in enumerate(blocks):
    boss, crowd = tune[i]
    b, n = re.subn(r"crowd: &\[[^\]]*\],", f"crowd: &[{crowd}],", b, count=1)
    assert n == 1, i
    b, n = re.subn(r"boss_tough: [\d.]+,", f"boss_tough: {boss},", b, count=1)
    assert n == 1, i
    blocks[i] = b
path.write_text(MARK.join([head] + blocks))
print(f"updated {len(blocks)} stages")
