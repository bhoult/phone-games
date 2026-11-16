#!/usr/bin/env python3
"""Generates the pictures in tools/art_list.py with Z-Image-Turbo.

    python3 tools/paint.py                 everything not yet made
    python3 tools/paint.py fox badger      just these (made again)
    python3 tools/paint.py --group enemies

Run it with a Python that has torch and diffusers. It needs a GPU and the model
Tongyi-MAI/Z-Image-Turbo in the Hugging Face cache. Raw pictures go to
art/raw/ (not kept in git). Run tools/cut.py afterwards.
"""
import os
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import art_list  # noqa: E402

RAW = os.path.join(HERE, "..", "art", "raw")


def main():
    args = sys.argv[1:]
    group = None
    if "--group" in args:
        i = args.index("--group")
        group = args[i + 1]
        del args[i : i + 2]
    os.makedirs(RAW, exist_ok=True)
    jobs = []
    for a in art_list.ART:
        path = os.path.join(RAW, a["name"] + ".png")
        if args:
            if a["name"] in args:
                jobs.append((a, path))
        elif group:
            if a["group"] == group:
                jobs.append((a, path))
        elif not os.path.exists(path):
            jobs.append((a, path))
    if not jobs:
        print("nothing to paint")
        return

    os.environ.setdefault("HF_HUB_OFFLINE", "1")
    import torch
    from diffusers import ZImagePipeline

    pipe = ZImagePipeline.from_pretrained("Tongyi-MAI/Z-Image-Turbo", torch_dtype=torch.bfloat16)
    pipe.enable_model_cpu_offload()
    pipe.set_progress_bar_config(disable=True)
    for n, (a, path) in enumerate(jobs):
        t = time.time()
        gen = torch.Generator("cpu").manual_seed(a.get("seed", 1))
        img = pipe(
            prompt=art_list.prompt(a),
            width=a.get("w", 1024),
            height=a.get("h", 1024),
            num_inference_steps=9,
            guidance_scale=0.0,
            generator=gen,
        ).images[0]
        img.save(path)
        print(f"[{n + 1}/{len(jobs)}] {a['name']} {time.time() - t:.0f}s", flush=True)


if __name__ == "__main__":
    main()
