#!/usr/bin/env python3
"""Turns the raw pictures in art/raw/ into what the game is built from.

    art/.venv/bin/python tools/cut.py            everything not yet cut
    art/.venv/bin/python tools/cut.py fox ant    just these (cut again)
    art/.venv/bin/python tools/cut.py --place    only re-size and re-place cut-outs
                                                  (after changing rot/flip/px)

  creatures, enemies, towers, world, shots
      cut out of their background (art/cut/NAME.png, full size, not in git),
      then turned, mirrored, trimmed and shrunk into game/assets/img/atlas/
  faces      cropped round, into game/assets/img/atlas/
  textures   shrunk, as JPEG, into game/assets/img/tex/
  scenes     shrunk, as JPEG, into game/assets/img/scenes/
  and the app's icons, from the fox's portrait, into game/assets/img/

Needs Pillow, numpy and rembg (see the README). The game's build reads only
what this leaves under game/assets/img/, which is kept in git.
"""
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import art_list  # noqa: E402

ROOT = os.path.join(HERE, "..")
RAW = os.path.join(ROOT, "art", "raw")
CUT = os.path.join(ROOT, "art", "cut")
IMG = os.path.join(ROOT, "game", "assets", "img")
CUTOUT_GROUPS = ("creatures", "enemies", "towers", "world", "shots")

_sessions = {}


def matte(img, model):
    """RGBA cut-out of the subject of a picture taken against plain grey."""
    from rembg import new_session, remove

    if model not in _sessions:
        _sessions[model] = new_session(model)
    out = remove(img, session=_sessions[model], post_process_mask=False)
    rgba = np.asarray(out.convert("RGBA")).astype(np.float32) / 255.0
    rgb, a = rgba[..., :3], rgba[..., 3]
    src = np.asarray(img.convert("RGB")).astype(np.float32) / 255.0

    # The background's colour, everywhere: its average in each cell of a
    # coarse grid where nothing of the subject is, spread into the cells
    # the subject covers.
    n = 24
    h, w = a.shape
    ys, xs = np.linspace(0, h, n + 1).astype(int), np.linspace(0, w, n + 1).astype(int)
    grid = np.zeros((n, n, 3), np.float32)
    known = np.zeros((n, n), bool)
    for j in range(n):
        for i in range(n):
            blk_a = a[ys[j] : ys[j + 1], xs[i] : xs[i + 1]]
            m = blk_a < 0.02
            if m.sum() > blk_a.size * 0.3:
                grid[j, i] = src[ys[j] : ys[j + 1], xs[i] : xs[i + 1]][m].mean(axis=0)
                known[j, i] = True
    if not known.any():
        grid[:] = 0.5
        known[:] = True
    while not known.all():
        nk = known.copy()
        for j in range(n):
            for i in range(n):
                if known[j, i]:
                    continue
                acc, cnt = np.zeros(3, np.float32), 0
                for dj, di in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                    jj, ii = j + dj, i + di
                    if 0 <= jj < n and 0 <= ii < n and known[jj, ii]:
                        acc += grid[jj, ii]
                        cnt += 1
                if cnt:
                    grid[j, i] = acc / cnt
                    nk[j, i] = True
        known = nk
    bg = np.asarray(Image.fromarray((grid * 255).astype(np.uint8)).resize((w, h), Image.BICUBIC)).astype(np.float32) / 255.0

    # Tighten the edge a little, then take the background's share back out
    # of every part-transparent pixel so fur and antennae carry no grey rim.
    a = np.clip((a - 0.08) / 0.84, 0.0, 1.0)
    part = (a > 0.0) & (a < 0.995)
    fg = src.copy()
    aa = np.maximum(a, 0.12)[..., None]
    un = (src - (1.0 - aa) * bg) / aa
    fg[part] = np.clip(un[part], 0.0, 1.0)
    out = np.dstack([fg, a])
    return Image.fromarray((out * 255).round().astype(np.uint8), "RGBA")


def place(a):
    """The game's copy of a cut-out: turned, mirrored, trimmed, shrunk."""
    im = Image.open(os.path.join(CUT, a["name"] + ".png")).convert("RGBA")
    alpha = np.asarray(im)[..., 3]
    ys, xs = np.where(alpha > 12)
    if len(xs) == 0:
        print("  !! nothing left of", a["name"])
        return
    im = im.crop((xs.min(), ys.min(), xs.max() + 1, ys.max() + 1))
    if a.get("rot"):
        im = im.rotate(a["rot"], expand=True, resample=Image.BICUBIC)
        alpha = np.asarray(im)[..., 3]
        ys, xs = np.where(alpha > 12)
        im = im.crop((xs.min(), ys.min(), xs.max() + 1, ys.max() + 1))
    if a.get("flip"):
        im = im.transpose(Image.FLIP_LEFT_RIGHT)
    k = a.get("px", 160) / max(im.size)
    # shrink colour and coverage together (premultiplied) so edges stay clean
    px = np.asarray(im).astype(np.float32) / 255.0
    px[..., :3] *= px[..., 3:4]
    size = (max(1, round(im.width * k)), max(1, round(im.height * k)))
    small = np.dstack([np.asarray(Image.fromarray((px[..., c] * 255).astype(np.uint8)).resize(size, Image.LANCZOS)).astype(np.float32) for c in range(4)]) / 255.0
    al = np.maximum(small[..., 3:4], 1e-4)
    small[..., :3] = np.clip(small[..., :3] / al, 0, 1)
    out = Image.fromarray((small * 255).round().astype(np.uint8), "RGBA")
    dst = os.path.join(IMG, "atlas", a["group"])
    os.makedirs(dst, exist_ok=True)
    out.save(os.path.join(dst, a["name"] + ".png"), optimize=True)


def round_face(a):
    im = Image.open(os.path.join(RAW, a["name"] + ".png")).convert("RGB")
    s = min(im.size)
    im = im.crop(((im.width - s) // 2, (im.height - s) // 2, (im.width + s) // 2, (im.height + s) // 2))
    px = a.get("px", 256)
    big = px * 4
    mask = Image.new("L", (big, big), 0)
    ImageDraw.Draw(mask).ellipse([2, 2, big - 3, big - 3], fill=255)
    mask = mask.resize((px, px), Image.LANCZOS)
    out = im.resize((px, px), Image.LANCZOS).convert("RGBA")
    out.putalpha(mask)
    dst = os.path.join(IMG, "atlas", "faces")
    os.makedirs(dst, exist_ok=True)
    out.save(os.path.join(dst, a["name"] + ".png"), optimize=True)


def icons():
    """The app's icon: the fox's face, square, at the sizes a home screen wants."""
    raw = os.path.join(RAW, "face_fox.png")
    if not os.path.exists(raw):
        return
    im = Image.open(raw).convert("RGB")
    s = min(im.size)
    im = im.crop(((im.width - s) // 2, (im.height - s) // 2, (im.width + s) // 2, (im.height + s) // 2))
    # darker toward the corners, so a round or rounded mask loses nothing that matters
    yy, xx = np.mgrid[0:s, 0:s]
    d = np.hypot((xx - s / 2) / (s / 2), (yy - s / 2) / (s / 2))
    px = np.asarray(im).astype(np.float32) * (1.0 - 0.55 * np.clip((d - 0.55) / 0.75, 0, 1))[..., None]
    im = Image.fromarray(px.clip(0, 255).astype(np.uint8))
    for size in (180, 192, 512):
        im.resize((size, size), Image.LANCZOS).save(os.path.join(IMG, f"icon-{size}.png"), optimize=True)


def shrink(a, folder, quality):
    im = Image.open(os.path.join(RAW, a["name"] + ".png")).convert("RGB")
    w = a.get("px", im.width)
    im = im.resize((w, round(im.height * w / im.width)), Image.LANCZOS)
    dst = os.path.join(IMG, folder)
    os.makedirs(dst, exist_ok=True)
    im.save(os.path.join(dst, a["name"] + ".jpg"), quality=quality, optimize=True, subsampling=0 if folder == "tex" else 2)


def main():
    args = sys.argv[1:]
    only_place = "--place" in args
    names = [x for x in args if not x.startswith("--")]
    os.makedirs(CUT, exist_ok=True)
    for a in art_list.ART:
        raw = os.path.join(RAW, a["name"] + ".png")
        if names and a["name"] not in names:
            continue
        if not os.path.exists(raw):
            continue
        g = a["group"]
        if g in CUTOUT_GROUPS:
            cut = os.path.join(CUT, a["name"] + ".png")
            fresh = not os.path.exists(cut) or os.path.getmtime(cut) < os.path.getmtime(raw)
            if (fresh or (names and not only_place)) and not (only_place and os.path.exists(cut)):
                # One model is best at animals and people, down to the last
                # whisker, but takes "the subject" to mean the animal alone and
                # drops whatever it is standing on. The other keeps the whole
                # object: the bird and the snag it clings to.
                model = a.get("matte", "birefnet-dis" if a["kind"] == "thing" else "birefnet-general")
                matte(Image.open(raw).convert("RGB"), model).save(cut)
                print("cut", a["name"], flush=True)
            elif not names and not only_place:
                # already cut and not asked for by name: only place it if missing
                if os.path.exists(os.path.join(IMG, "atlas", g, a["name"] + ".png")):
                    continue
            place(a)
        elif g == "faces":
            round_face(a)
        elif g == "textures":
            shrink(a, "tex", 86)
        elif g == "scenes":
            shrink(a, "scenes", 84)
    if not names or "face_fox" in names:
        icons()


if __name__ == "__main__":
    main()
