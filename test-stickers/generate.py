"""Generate amorphous test stickers (transparent PNG) for Photo Filler.

Each sticker is sized so that at 300 DPI it lands ~50 mm on the long edge —
small enough to fit a couple dozen on an A4, large enough for the alpha
outline to be obviously non-rectangular.

Run with the project venv:
    /tmp/pf-venv/bin/python3 test-stickers/generate.py
"""

from __future__ import annotations

import math
import os
import random
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

OUT_DIR = Path(__file__).resolve().parent
SIZE = 600  # px, square canvas
PAD = 30    # px of empty alpha around content


def _new_canvas() -> Image.Image:
    return Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))


def _save(img: Image.Image, name: str) -> None:
    path = OUT_DIR / name
    img.save(path, "PNG", optimize=True)
    print(f"wrote {path.relative_to(OUT_DIR.parent)}")


def blob(seed: int = 7, color: tuple[int, int, int] = (90, 130, 230)) -> Image.Image:
    """Organic blob — interpolated radii around a centre, smoothed."""
    rng = random.Random(seed)
    img = _new_canvas()
    cx = cy = SIZE / 2
    n = 32
    base = SIZE / 2 - PAD - 10
    radii = [base * (0.55 + 0.45 * rng.random()) for _ in range(n)]
    # Smooth the radii so the silhouette stays continuous.
    for _ in range(4):
        radii = [(radii[(i - 1) % n] + 2 * radii[i] + radii[(i + 1) % n]) / 4 for i in range(n)]
    pts = []
    for i in range(n):
        t = 2 * math.pi * i / n
        pts.append((cx + radii[i] * math.cos(t), cy + radii[i] * math.sin(t)))
    draw = ImageDraw.Draw(img)
    draw.polygon(pts, fill=color + (255,))
    return img.filter(ImageFilter.SMOOTH_MORE)


def star(points: int = 7, color: tuple[int, int, int] = (240, 180, 60)) -> Image.Image:
    """Concave N-pointed star. Concavities matter — copies should be able to
    nestle into them when packed."""
    img = _new_canvas()
    cx = cy = SIZE / 2
    outer = SIZE / 2 - PAD
    inner = outer * 0.42
    pts = []
    for i in range(points * 2):
        r = outer if i % 2 == 0 else inner
        t = math.pi / 2 + math.pi * i / points
        pts.append((cx + r * math.cos(t), cy - r * math.sin(t)))
    draw = ImageDraw.Draw(img)
    draw.polygon(pts, fill=color + (255,))
    return img


def cloud(seed: int = 13, color: tuple[int, int, int] = (220, 220, 240)) -> Image.Image:
    """Union of overlapping circles — classic cloud silhouette."""
    rng = random.Random(seed)
    img = _new_canvas()
    cx = cy = SIZE / 2
    # 6–8 circles whose centres stay near the middle and whose radii vary.
    circles = []
    for _ in range(8):
        dx = rng.uniform(-SIZE * 0.18, SIZE * 0.18)
        dy = rng.uniform(-SIZE * 0.10, SIZE * 0.10)
        r = rng.uniform(SIZE * 0.13, SIZE * 0.20)
        circles.append((cx + dx, cy + dy, r))
    draw = ImageDraw.Draw(img)
    for (x, y, r) in circles:
        draw.ellipse((x - r, y - r, x + r, y + r), fill=color + (255,))
    # Soft outline
    edge_draw = ImageDraw.Draw(img)
    for (x, y, r) in circles:
        edge_draw.ellipse((x - r, y - r, x + r, y + r), outline=(140, 140, 170, 255), width=3)
    return img


def heart(color: tuple[int, int, int] = (230, 60, 80)) -> Image.Image:
    """Classic heart — parametric curve. Has a sharp cleft at the top."""
    img = _new_canvas()
    cx = cy = SIZE / 2
    scale = (SIZE / 2 - PAD) / 18
    pts = []
    n = 256
    for i in range(n):
        t = 2 * math.pi * i / n
        x = 16 * math.sin(t) ** 3
        y = 13 * math.cos(t) - 5 * math.cos(2 * t) - 2 * math.cos(3 * t) - math.cos(4 * t)
        pts.append((cx + x * scale, cy - y * scale))
    draw = ImageDraw.Draw(img)
    draw.polygon(pts, fill=color + (255,))
    return img


def crescent(color: tuple[int, int, int] = (240, 220, 120)) -> Image.Image:
    """Crescent moon — outer circle minus an offset inner circle. This is the
    canonical concave shape: tight bounding box vs. a huge interior void."""
    img = _new_canvas()
    cx = cy = SIZE / 2
    r_outer = SIZE / 2 - PAD
    r_inner = r_outer * 0.92
    off = r_outer * 0.42
    # Draw outer disc, then knock out the inner disc by drawing transparent
    # over it.
    outer = _new_canvas()
    od = ImageDraw.Draw(outer)
    od.ellipse((cx - r_outer, cy - r_outer, cx + r_outer, cy + r_outer), fill=color + (255,))
    od.ellipse(
        (cx + off - r_inner, cy - r_inner, cx + off + r_inner, cy + r_inner),
        fill=(0, 0, 0, 0),
    )
    img.alpha_composite(outer)
    return img


def bolt(color: tuple[int, int, int] = (255, 220, 40)) -> Image.Image:
    """Lightning bolt — angular polygon with sharp inward notches."""
    img = _new_canvas()
    s = (SIZE / 2 - PAD) / 100
    cx = cy = SIZE / 2
    # Polygon in local (centred) coordinates, scaled by s.
    local = [
        (-10, -100), (50, -100), (10, -10), (60, -10),
        (-20, 100), (5, 5), (-50, 5),
    ]
    pts = [(cx + x * s, cy + y * s) for (x, y) in local]
    draw = ImageDraw.Draw(img)
    draw.polygon(pts, fill=color + (255,))
    return img


def main() -> None:
    os.makedirs(OUT_DIR, exist_ok=True)
    _save(blob(), "blob.png")
    _save(star(), "star.png")
    _save(cloud(), "cloud.png")
    _save(heart(), "heart.png")
    _save(crescent(), "crescent.png")
    _save(bolt(), "bolt.png")


if __name__ == "__main__":
    main()
