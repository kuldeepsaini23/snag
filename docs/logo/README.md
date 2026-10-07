# Snag's logo

- `engraving/c-gpt.png` — the chosen engraving (hook catching a download arrow), generated from the prompt in the launch notes; `a-…` and `b-…` were the alternatives.
- `snag-engraved-1024.png` — the app icon at 48 px and up (`make_logo.py` cuts the drawing out of its orange and puts it on the rounded tile).
- `snag-small-1024.png` — the bold mark for 16–32 px and small places in the app (`make_small.py` draws it).

The app draws both on a tile of the user's accent colour from two layers each (`crates/app/assets/*-64.rgba`: the whole tile on orange, and the drawing alone).
