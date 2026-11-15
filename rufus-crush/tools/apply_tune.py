#!/usr/bin/env python3
"""Runs the balance tuner and writes its move budgets (Easy, Normal, Hard) and
paw thresholds into core/src/level.rs. Re-run after changing a level's goals
or layout, or the difficulty curves in core/examples/balance.rs."""
import re, subprocess, sys, pathlib
root = pathlib.Path(__file__).resolve().parent.parent
games = sys.argv[1] if len(sys.argv) > 1 else "400"
out = subprocess.run(["cargo", "run", "-q", "-p", "rufus-core", "--release", "--example", "balance", "tune", games],
                     cwd=root, capture_output=True, text=True, check=True)
print(out.stdout)
tune = {int(i): tuple(map(int, rest.split())) for i, rest in re.findall(r"^TUNE (\d+) ((?:\d+ ){4}\d+)$", out.stderr, re.M)}
path = root / "core/src/level.rs"
head, *blocks = path.read_text().split("    LevelDef {")
assert len(blocks) == len(tune), (len(blocks), len(tune))
for i, b in enumerate(blocks):
    easy, normal, hard, two, three = tune[i]
    b, n = re.subn(r"moves: \[\d+, \d+, \d+\],", f"moves: [{easy}, {normal}, {hard}],", b, count=1)
    assert n == 1, i
    b, n = re.subn(r"paws: \[\d+, \d+\],", f"paws: [{two}, {three}],", b, count=1)
    assert n == 1, i
    blocks[i] = b
path.write_text("    LevelDef {".join([head] + blocks))
print(f"updated {len(blocks)} levels")
