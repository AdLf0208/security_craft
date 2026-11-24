#!/usr/bin/env python3
"""Report the average RGB of the keypad face in an --offscreen screenshot.

The e2e aims the crosshair at a keypad's south face; the chat overlay starts lower on
the screen, so the face is sampled a little above the centre. Prints `PIXEL r g b`
for the e2e to assert a parsed colour, never on HUD noise.
"""
import sys

from PIL import Image


def main() -> None:
    path = sys.argv[1]
    with Image.open(path) as img:
        im = img.convert("RGB")
    w, h = im.size
    cx, cy = w // 2, h // 2 - 100  # the keypad face, above the chat overlay
    box = 10
    total = [0, 0, 0]
    n = 0
    for y in range(cy - box, cy + box + 1):
        for x in range(cx - box, cx + box + 1):
            r, g, b = im.getpixel((x, y))
            total[0] += r
            total[1] += g
            total[2] += b
            n += 1
    r, g, b = (v // n for v in total)
    print(f"PIXEL {r} {g} {b} sampled={n} at={cx},{cy}")


if __name__ == "__main__":
    main()
