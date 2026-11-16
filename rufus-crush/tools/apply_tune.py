#!/usr/bin/env python3
"""Runs the balance tuner and writes its move budgets (Easy, Normal, Hard) and
paw thresholds into the level data: core/src/level.rs for the hand-made
levels, core/src/later.rs for the rest. Re-run after changing a level's goals
or layout, or the difficulty curves in core/src/tune.rs."""
import re, subprocess, sys, pathlib
root = pathlib.Path(__file__).resolve().parent.parent
games = sys.argv[1] if len(sys.argv) > 1 else "400"
out = subprocess.run(["cargo", "run", "-q", "-p", "rufus-core", "--release", "--example", "balance", "tune", games],
                     cwd=root, capture_output=True, text=True, check=True)
print(out.stdout)
tune = {int(i): tuple(map(int, rest.split())) for i, rest in re.findall(r"^TUNE (\d+) ((?:\d+ ){4}\d+)$", out.stderr, re.M)}
# The levels are numbered straight through both files, in this order.
files = [root / "core/src/level.rs", root / "core/src/later.rs"]
parts = [path.read_text().split("    LevelDef {") for path in files]
assert sum(len(p) - 1 for p in parts) == len(tune), (sum(len(p) - 1 for p in parts), len(tune))
i = changed = 0
for path, (head, *blocks) in zip(files, parts):
    for k, b in enumerate(blocks):
        easy, normal, hard, two, three = tune[i]
        was = b
        b, n = re.subn(r"moves: \[\d+, \d+, \d+\],", f"moves: [{easy}, {normal}, {hard}],", b, count=1)
        assert n == 1, i
        b, n = re.subn(r"paws: \[\d+, \d+\]", f"paws: [{two}, {three}]", b, count=1)
        assert n == 1, i
        blocks[k] = b
        changed += b != was
        i += 1
    path.write_text("    LevelDef {".join([head] + blocks))
print(f"checked {i} levels, changed {changed}")
