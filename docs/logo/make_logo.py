"""Builds Snag's engraved logo set from docs/logo/engraving/c-gpt.png.

Outputs into OUT: subject.png (hook + arrow, transparent), and tiles at every size.
"""
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

SRC, OUT = sys.argv[1], sys.argv[2]
# The tile: Snag's copper brand colour (the art itself was drawn on orange and is cut out of it).
TILE = (217, 104, 43, 255)

im = np.asarray(Image.open(SRC).convert("RGB")).astype(np.float32)
corners = np.concatenate([im[:40, :40].reshape(-1, 3), im[-40:, :40].reshape(-1, 3), im[-40:, -40:].reshape(-1, 3)])
bg = np.median(corners, axis=0)
dist = np.sqrt(((im - bg) ** 2).sum(-1))
# Background noise sits well under 40; the drawing's paper and ink are far from orange.
lo, hi = 40.0, 110.0
alpha = np.clip((dist - lo) / (hi - lo), 0, 1)
# Un-mix the background from edge pixels so they don't keep an orange fringe.
a = alpha[..., None]
rgb = np.where(a > 0.01, (im - (1 - a) * bg) / np.maximum(a, 0.01), 0)
rgb = np.clip(rgb, 0, 255)
subject = np.dstack([rgb, alpha * 255]).astype(np.uint8)
sub = Image.fromarray(subject, "RGBA")
# Drop specks: keep only what's solid enough.
box = sub.getchannel("A").point(lambda v: 255 if v > 60 else 0).getbbox()
sub = sub.crop(box)
sub.save(f"{OUT}/subject.png")
print("subject bbox", box, sub.size)


def no_line(s: Image.Image) -> Image.Image:
    """The subject without the fishing line above the hook's eye (small sizes)."""
    a = np.asarray(s.getchannel("A"))
    widths = (a > 60).sum(1)
    # The line is a thin column at the top; the hook's eye starts where the shape widens.
    start = next(i for i, w in enumerate(widths) if w > s.width * 0.12)
    return s.crop((0, max(0, start - 2), s.width, s.height))


def tile(size: int, subject: Image.Image, fill: float, top_bleed: bool, radius=0.225) -> Image.Image:
    big = 1024
    canvas = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    mask = Image.new("L", (big, big), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, big - 1, big - 1], radius=int(big * radius), fill=255)
    bgl = Image.new("RGBA", (big, big), TILE)
    canvas.paste(bgl, (0, 0), mask)
    s = subject
    h = int(big * fill)
    w = int(s.width * h / s.height)
    s = s.resize((w, h), Image.LANCZOS)
    x = (big - w) // 2
    y = 0 if top_bleed else (big - h) // 2
    layer = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    layer.paste(s, (x, y), s)
    # Keep the drawing inside the rounded tile.
    la = np.asarray(layer.getchannel("A")).astype(np.float32) * (np.asarray(mask) / 255.0)
    layer.putalpha(Image.fromarray(la.astype(np.uint8)))
    canvas = Image.alpha_composite(canvas, layer)
    out = canvas.resize((size, size), Image.LANCZOS)
    if size <= 48:
        out = out.filter(ImageFilter.UnsharpMask(radius=0.6, percent=60, threshold=0))
    return out, layer.resize((size, size), Image.LANCZOS)


full = sub
small = no_line(sub)
for size in (256, 128, 64, 48, 32, 16):
    if size >= 64:
        t, layer = tile(size, full, 0.92, True)
    else:
        t, layer = tile(size, small, 0.86, False)
    t.save(f"{OUT}/snag-{size}.png")
    layer.save(f"{OUT}/subject-{size}.png")
print("done")
