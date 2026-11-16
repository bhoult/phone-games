#!/usr/bin/env python3
"""Screenshots side by side: tools/strip.py out.jpg [--h N] shot.png ..."""
import sys
from PIL import Image
args = sys.argv[1:]
out = args.pop(0)
h = 1170
if args and args[0] == "--h":
    h = int(args[1]); args = args[2:]
ims = [Image.open(a).convert("RGB") for a in args]
ims = [im.resize((round(im.width * h / im.height), h), Image.LANCZOS) for im in ims]
gap = 16
s = Image.new("RGB", (sum(im.width for im in ims) + gap * (len(ims) - 1), h), (0, 0, 0))
x = 0
for im in ims:
    s.paste(im, (x, 0)); x += im.width + gap
s.save(out, quality=90)
print(out, s.size)
