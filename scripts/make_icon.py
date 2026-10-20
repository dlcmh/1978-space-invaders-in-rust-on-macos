#!/usr/bin/env python3
"""Generate the procedural application icon.

The icon is original artwork: a glowing pixel invader on a deep-space tile,
built from the same ASCII pixel language as the in-game sprites. No external
image assets are required. Run `make icon` after changing the artwork.
"""

from __future__ import annotations

import os
import shutil
import subprocess

from PIL import Image, ImageDraw, ImageFilter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RESOURCES = os.path.join(ROOT, "Resources")
ICONSET = os.path.join(RESOURCES, "AppIcon.iconset")

OCTOPUS = [
    "....####....",
    ".##########.",
    "############",
    "###..##..###",
    "############",
    "...##..##...",
    "..##.##.##..",
    "##........##",
]

SIZE = 1024
BG_TOP = (8, 14, 30)
BG_BOTTOM = (3, 6, 14)
GREEN = (72, 255, 128)
MINT = (150, 255, 195)
MAGENTA = (255, 76, 190)


def gradient() -> Image.Image:
    image = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    for y in range(SIZE):
        t = y / (SIZE - 1)
        color = tuple(
            round(BG_TOP[channel] * (1 - t) + BG_BOTTOM[channel] * t)
            for channel in range(3)
        )
        draw.line([(0, y), (SIZE, y)], fill=(*color, 255))
    mask = Image.new("L", (SIZE, SIZE), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, SIZE, SIZE), radius=226, fill=255)
    image.putalpha(mask)
    return image


def starfield(image: Image.Image) -> None:
    draw = ImageDraw.Draw(image)
    stars = [
        (128, 190, 8),
        (238, 128, 5),
        (820, 168, 7),
        (900, 302, 4),
        (170, 760, 6),
        (300, 870, 4),
        (760, 838, 8),
        (860, 690, 5),
        (520, 130, 3),
        (660, 226, 4),
    ]
    for x, y, size in stars:
        draw.rectangle((x, y, x + size, y + size), fill=(180, 220, 255, 150))


def invader_layer() -> Image.Image:
    cell = 58
    width = len(OCTOPUS[0]) * cell
    height = len(OCTOPUS) * cell
    layer = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)
    for row, line in enumerate(OCTOPUS):
        for column, cell_value in enumerate(line):
            if cell_value != "#":
                continue
            x = column * cell
            y = row * cell
            draw.rectangle((x, y, x + cell - 1, y + cell - 1), fill=(*MINT, 255))
    glow = layer.filter(ImageFilter.GaussianBlur(24))
    glow_layer = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    glow_layer.alpha_composite(glow)
    glow_layer = Image.composite(
        Image.new("RGBA", (width, height), (*GREEN, 180)),
        glow_layer,
        glow_layer.getchannel("A"),
    )
    result = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    result.alpha_composite(glow_layer)
    result.alpha_composite(layer)
    return result


def build() -> Image.Image:
    image = gradient()
    starfield(image)
    draw = ImageDraw.Draw(image)

    # Neon frame and horizon line.
    draw.rounded_rectangle((46, 46, SIZE - 46, SIZE - 46), radius=188, outline=(*GREEN, 210), width=12)
    draw.rectangle((188, 724, SIZE - 188, 730), fill=(*GREEN, 110))
    draw.rectangle((250, 736, SIZE - 250, 740), fill=(*MAGENTA, 80))

    alien = invader_layer()
    image.alpha_composite(alien, ((SIZE - alien.width) // 2, 252))
    return image


def main() -> None:
    os.makedirs(RESOURCES, exist_ok=True)
    if os.path.isdir(ICONSET):
        shutil.rmtree(ICONSET)
    os.makedirs(ICONSET)

    icon = build()
    icon.save(os.path.join(RESOURCES, "AppIcon.png"))

    for size in (16, 32, 128, 256, 512):
        icon.resize((size, size), Image.Resampling.LANCZOS).save(
            os.path.join(ICONSET, f"icon_{size}x{size}.png")
        )
        icon.resize((size * 2, size * 2), Image.Resampling.LANCZOS).save(
            os.path.join(ICONSET, f"icon_{size}x{size}@2x.png")
        )

    subprocess.run(
        ["iconutil", "-c", "icns", ICONSET, "-o", os.path.join(RESOURCES, "AppIcon.icns")],
        check=True,
    )
    print("Wrote Resources/AppIcon.png and Resources/AppIcon.icns")


if __name__ == "__main__":
    main()
