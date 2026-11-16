#!/usr/bin/env python3
"""Contact sheet of SVG files, straight from the files (no build needed), for
looking at art while drawing it. Uses headless Chrome.

  tools/preview.py OUT.png FILE_OR_DIR [...] [--size 160] [--bg 8fbf6a] [--cols 6]
"""
import sys, pathlib, subprocess, tempfile, html, os
args = sys.argv[1:]
def opt(name, default):
    if name in args:
        i = args.index(name); v = args[i + 1]; del args[i:i + 2]; return v
    return default
size = int(opt("--size", 160)); bg = opt("--bg", "8fbf6a"); cols = int(opt("--cols", 6))
out = pathlib.Path(args[0]).resolve()
files = []
for a in args[1:]:
    p = pathlib.Path(a)
    files += sorted(p.rglob("*.svg")) if p.is_dir() else [p]
rows = (len(files) + cols - 1) // cols
cell_w, cell_h = size + 16, size + 30
cells = "".join(
    f'<div class="c"><img src="{f.resolve().as_uri()}" style="max-width:{size}px;max-height:{size}px"><span>{html.escape(f.stem)}</span></div>'
    for f in files)
page = f"""<!doctype html><meta charset="utf-8"><style>
body{{margin:0;background:#{bg};font:11px sans-serif;width:{cols*cell_w}px}}
.c{{float:left;width:{cell_w}px;height:{cell_h}px;text-align:center;position:relative}}
.c img{{position:absolute;left:50%;top:{size/2+4}px;transform:translate(-50%,-50%)}}
.c span{{position:absolute;left:0;right:0;bottom:2px;color:#000a}}
</style>{cells}"""
tmp = pathlib.Path(os.environ.get("PREVIEW_TMP", tempfile.gettempdir()))
src = tmp / f"preview-{os.getpid()}.html"
src.write_text(page)
subprocess.run([os.environ.get("CHROME", "google-chrome"), "--headless=new", "--no-sandbox", "--hide-scrollbars",
                f"--user-data-dir={tmp}/preview-chrome-{os.getpid()}", f"--screenshot={out}",
                f"--window-size={cols*cell_w},{rows*cell_h}", src.as_uri()],
               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
src.unlink()
subprocess.run(["rm", "-rf", f"{tmp}/preview-chrome-{os.getpid()}"])
print(f"{out}: {len(files)} sprites")
