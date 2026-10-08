"""Exports Snag's logo to every place that uses it. Run from the repo root:

    python docs/logo/export.py

It runs make_logo.py and make_small.py into a temporary folder (so the tile colour set there is
used everywhere), then writes the app icons, the tray/window/in-app layers, the extension icons,
the website favicon and the store images. Afterwards run `bun run images` in site/.
"""
import importlib.util
import os
import subprocess
import sys
import tempfile

from PIL import Image, ImageDraw, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, "engraving", "c-gpt.png")


def load(name):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, f"{name}.py"))
    return spec


def main():
    tmp = tempfile.mkdtemp(prefix="snag-logo-")
    subprocess.run([sys.executable, "-I", os.path.join(HERE, "make_small.py"), tmp], check=True)
    # make_logo.py renders the tiles; import it to also get the 1024 tile and the drawing layer.
    sys.argv = ["make_logo.py", SRC, tmp]
    spec = importlib.util.spec_from_file_location("make_logo", os.path.join(HERE, "make_logo.py"))
    ml = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(ml)
    tile_rgb = ml.TILE[:3]
    eng, subj = ml.tile(1024, ml.full, 0.92, True)
    small = Image.open(os.path.join(tmp, "small-1024.png")).convert("RGBA")
    small_subj = Image.open(os.path.join(tmp, "small-subject-1024.png")).convert("RGBA")

    def at(n):
        src = small if n <= 32 else eng
        im = src.resize((n, n), Image.LANCZOS)
        if 32 < n <= 64:
            im = im.filter(ImageFilter.UnsharpMask(radius=0.6, percent=50, threshold=0))
        return im

    def mask(n):
        m = Image.new("L", (1024, 1024), 0)
        ImageDraw.Draw(m).rounded_rectangle([0, 0, 1023, 1023], radius=int(1024 * 0.225), fill=255)
        return m.resize((n, n), Image.LANCZOS)

    def layers(subject, sharpen):
        """(tile as drawn, drawing alone) at 64 px, built from the same pieces so they agree."""
        sub = subject.resize((64, 64), Image.LANCZOS)
        if sharpen:
            sub = sub.filter(ImageFilter.UnsharpMask(radius=0.6, percent=50, threshold=0))
        m = mask(64)
        icon = Image.alpha_composite(Image.new("RGBA", (64, 64), tile_rgb + (255,)), sub)
        icon.putalpha(m)
        alpha = Image.frombytes("L", (64, 64), bytes(min(a, b) for a, b in zip(sub.getchannel("A").tobytes(), m.tobytes())))
        sub.putalpha(alpha)
        return icon, sub

    root = os.path.dirname(os.path.dirname(HERE))
    j = lambda *p: os.path.join(root, *p)
    eng.save(j("docs", "logo", "snag-engraved-1024.png"))
    small.save(j("docs", "logo", "snag-small-1024.png"))
    # App: PNGs, the exe/installer .ico, tray (32), window/notifications/in-app (64 layers).
    for n in (16, 32, 48, 128, 256):
        at(n).save(j("crates", "app", "assets", "logo", f"snag-{n}.png"))
    sizes = [16, 24, 32, 48, 64, 128, 256]
    imgs = [at(n) for n in sizes]
    imgs[-1].save(j("crates", "app", "assets", "logo", "snag.ico"), format="ICO", sizes=[(n, n) for n in sizes], append_images=imgs[:-1])
    open(j("crates", "app", "assets", "icon-32.rgba"), "wb").write(at(32).tobytes())
    icon64, sub64 = layers(subj, True)
    icon64.save(j("crates", "app", "assets", "logo", "snag-64.png"))
    open(j("crates", "app", "assets", "icon-64.rgba"), "wb").write(icon64.tobytes())
    open(j("crates", "app", "assets", "logo-subject-64.rgba"), "wb").write(sub64.tobytes())
    mark64, mark_sub = layers(small_subj, False)
    open(j("crates", "app", "assets", "mark-64.rgba"), "wb").write(mark64.tobytes())
    open(j("crates", "app", "assets", "mark-subject-64.rgba"), "wb").write(mark_sub.tobytes())
    # Extension and website favicon.
    for n in (16, 32, 48, 128):
        at(n).save(j("extension", f"icon-{n}.png"))
    fav = [at(n) for n in (16, 32, 48)]
    fav[-1].save(j("site", "src", "app", "favicon.ico"), format="ICO", sizes=[(16, 16), (32, 32), (48, 48)], append_images=fav[:-1])
    # Store images.
    shots = j("docs", "store", "screenshots")
    eng.resize((300, 300), Image.LANCZOS).save(os.path.join(shots, "logo-300.png"))
    eng.resize((128, 128), Image.LANCZOS).save(os.path.join(shots, "icon-128.png"))
    print("exported, tile", "#%02x%02x%02x" % tile_rgb)


if __name__ == "__main__":
    main()
