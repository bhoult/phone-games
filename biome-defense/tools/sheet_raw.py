#!/usr/bin/env python3
"""Contact sheet of raw or cut pictures: tools/sheet_raw.py out.jpg [--cut] name..."""
import os, sys
from PIL import Image, ImageDraw
HERE = os.path.dirname(os.path.abspath(__file__))
args = sys.argv[1:]
out = args.pop(0)
cut = "--cut" in args
args = [a for a in args if a != "--cut"]
tile, cols = 300, 6
rows = (len(args) + cols - 1) // cols
sheet = Image.new("RGB", (tile * cols, (tile + 18) * rows), (58, 70, 48) if cut else (20, 20, 20))
d = ImageDraw.Draw(sheet)
for i, n in enumerate(args):
    p = n if os.path.exists(n) else os.path.join(HERE, "..", "art", "cut" if cut else "raw", n + ".png")
    if not os.path.exists(p):
        continue
    im = Image.open(p).convert("RGBA")
    im.thumbnail((tile, tile))
    x, y = (i % cols) * tile + (tile - im.width) // 2, (i // cols) * (tile + 18) + (tile - im.height) // 2
    sheet.paste(im, (x, y), im)
    d.text(((i % cols) * tile + 4, (i // cols) * (tile + 18) + tile + 2), os.path.basename(n), fill=(255, 255, 255))
sheet.save(out, quality=88)
