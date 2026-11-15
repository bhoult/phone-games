#!/usr/bin/env python3
"""Contact sheet of the rendered sprite atlas, for eyeballing art changes.

  tools/sheet.py OUT.png [name-prefix ...] [--size N] [--bg RRGGBB] [--cols N]

Build first (./build.sh or cargo build) so the atlas exists.
"""
import re, sys, pathlib
from PIL import Image, ImageDraw

root = pathlib.Path(__file__).resolve().parent.parent
outs = sorted(root.glob("target/wasm32-unknown-unknown/release/build/rufus-crush-*/out/atlas.rs"),
              key=lambda p: p.stat().st_mtime)
if not outs:
    sys.exit("no atlas yet: build first")
out_dir = outs[-1].parent
args = sys.argv[1:]
def opt(name, default):
    if name in args:
        i = args.index(name); v = args[i + 1]; del args[i:i + 2]; return v
    return default
size = int(opt("--size", 150)); bg = opt("--bg", "f4ead2"); cols = int(opt("--cols", 8))
dest, prefixes = args[0], [a.upper() for a in args[1:]]
atlas = Image.open(out_dir / "atlas.png").convert("RGBA")
sprites = re.findall(r"pub const (\w+): Spr = Spr \{ x: ([\d.]+), y: ([\d.]+), w: ([\d.]+), h: ([\d.]+)", outs[-1].read_text())
sprites = [(n, *map(lambda v: int(float(v)), r)) for n, *r in sprites if not prefixes or any(n.startswith(p) for p in prefixes)]
rows = (len(sprites) + cols - 1) // cols
cell = size + 26
sheet = Image.new("RGBA", (cols * cell, rows * cell), "#" + bg)
draw = ImageDraw.Draw(sheet)
for i, (name, x, y, w, h) in enumerate(sprites):
    im = atlas.crop((x, y, x + w, y + h))
    s = min(size / w, size / h)
    im = im.resize((max(1, round(w * s)), max(1, round(h * s))), Image.LANCZOS)
    cx, cy = (i % cols) * cell, (i // cols) * cell
    # checker so transparency and white art both read
    for ty in range(0, size, 15):
        for tx in range(0, size, 15):
            if (tx // 15 + ty // 15) % 2:
                draw.rectangle([cx + 13 + tx, cy + 4 + ty, cx + 13 + tx + 14, cy + 4 + ty + 14], fill="#" + "".join(f"{max(0, int(bg[k:k+2], 16) - 16):02x}" for k in (0, 2, 4)))
    sheet.alpha_composite(im, (cx + 13 + (size - im.width) // 2, cy + 4 + (size - im.height) // 2))
    draw.text((cx + 13, cy + size + 8), name.lower()[:22], fill="#333333")
sheet.convert("RGB").save(dest)
print(f"{dest}: {len(sprites)} sprites, atlas {atlas.size[0]}x{atlas.size[1]}")
