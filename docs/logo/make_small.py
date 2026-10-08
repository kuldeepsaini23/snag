"""Bold, simplified Snag mark for 16-32 px: a paper arrow caught in an ink hook, on the orange tile."""
import sys
from PIL import Image, ImageDraw

OUT = sys.argv[1]
B = 1024
INK = (11, 11, 12, 255)
PAPER = (239, 233, 220, 255)
ORANGE = (217, 104, 43, 255)  # Snag copper #D9682B


def mark(tile: bool = True) -> Image.Image:
    img = Image.new("RGBA", (B, B), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    if tile:
        d.rounded_rectangle([0, 0, B - 1, B - 1], radius=int(B * 0.225), fill=ORANGE)
    hook_w = 96
    lx, rx, top, bend_y = 300, 724, 120, 520
    # The hook: shank on the left from the eye, a bend that passes behind the arrow's shaft, and
    # the point rising on the right with a barb.
    d.line([(lx, top + 60), (lx, bend_y)], fill=INK, width=hook_w)
    d.ellipse([lx - 70, top - 40, lx + 70, top + 100], outline=INK, width=52)
    d.arc([lx - hook_w // 2, bend_y - 190, rx + hook_w // 2, bend_y + 150], start=0, end=180, fill=INK, width=hook_w)
    d.line([(rx, bend_y - 20), (rx, 330)], fill=INK, width=hook_w)
    d.polygon([(rx - hook_w // 2, 340), (rx + hook_w // 2 + 40, 360), (rx - hook_w // 2, 220)], fill=INK)
    # The arrow in front: shaft through the bend, head hanging below it.
    outline = [(436, 200), (588, 200), (588, 610), (790, 610), (512, 950), (234, 610), (436, 610)]
    d.polygon(outline, fill=PAPER, outline=INK, width=44)
    return img


m = mark()
m.save(f"{OUT}/small-1024.png")
for size in (48, 32, 24, 16):
    m.resize((size, size), Image.LANCZOS).save(f"{OUT}/small-{size}.png")
mark(False).save(f"{OUT}/small-subject-1024.png")
print("ok")
