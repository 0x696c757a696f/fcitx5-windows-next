"""Generate deterministic Windows product and tray icons.

The artwork is geometric so builds do not depend on installed fonts. Run the
script with the repository's pinned Python environment.
"""

from pathlib import Path

from PIL import Image, ImageDraw


OUTPUT = Path(__file__).resolve().parent
MASTER_SIZE = 1024
ICO_SIZES = [(16, 16), (20, 20), (24, 24), (32, 32), (40, 40), (48, 48),
             (64, 64), (128, 128), (256, 256)]


def rounded_rectangle_mask(size: int, margin: int, radius: int) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (margin, margin, size - margin - 1, size - margin - 1), radius=radius, fill=255
    )
    return mask


def product_icon(state: str = "running") -> Image.Image:
    size = MASTER_SIZE
    margin = 52
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))

    shadow = Image.new("RGBA", image.size, (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle(
        (margin + 10, margin + 24, size - margin + 9, size - margin + 23),
        radius=210,
        fill=(9, 30, 66, 54),
    )
    image.alpha_composite(shadow)

    gradient = Image.new("RGBA", image.size, (0, 0, 0, 0))
    pixels = gradient.load()
    top = (20, 184, 198)
    bottom = (37, 99, 235)
    for y in range(size):
        ratio = y / (size - 1)
        color = tuple(round(top[index] * (1 - ratio) + bottom[index] * ratio)
                      for index in range(3))
        for x in range(size):
            pixels[x, y] = (*color, 255)
    gradient.putalpha(rounded_rectangle_mask(size, margin, 210))
    image.alpha_composite(gradient)

    draw = ImageDraw.Draw(image)
    rows = ((325, 650), (512, 585), (699, 505))
    for y, end_x in rows:
        draw.ellipse((236, y - 40, 316, y + 40), fill=(255, 255, 255, 245))
        draw.rounded_rectangle((362, y - 37, end_x, y + 37), radius=37,
                               fill=(255, 255, 255, 245))

    if state != "running":
        center = (790, 790)
        radius = 156
        color = (245, 158, 11, 255) if state == "paused" else (239, 68, 68, 255)
        draw.ellipse((center[0] - radius, center[1] - radius,
                      center[0] + radius, center[1] + radius),
                     fill=(255, 255, 255, 255))
        inner = radius - 20
        draw.ellipse((center[0] - inner, center[1] - inner,
                      center[0] + inner, center[1] + inner), fill=color)
        if state == "paused":
            for x in (746, 814):
                draw.rounded_rectangle((x, 710, x + 38, 870), radius=18,
                                       fill=(255, 255, 255, 255))
        else:
            draw.rounded_rectangle((772, 695, 808, 820), radius=18,
                                   fill=(255, 255, 255, 255))
            draw.ellipse((771, 844, 809, 882), fill=(255, 255, 255, 255))

    return image


def save_ico(name: str, state: str) -> None:
    icon = product_icon(state)
    icon.save(OUTPUT / name, format="ICO", sizes=ICO_SIZES, bitmap_format="png")


def main() -> None:
    save_ico("fcitx5.ico", "running")
    save_ico("fcitx5-paused.ico", "paused")
    save_ico("fcitx5-error.ico", "error")

    preview = Image.new("RGBA", (960, 320), (245, 247, 250, 255))
    for index, state in enumerate(("running", "paused", "error")):
        icon = product_icon(state).resize((256, 256), Image.Resampling.LANCZOS)
        preview.alpha_composite(icon, (48 + index * 304, 32))
    preview.save(OUTPUT / "fcitx5-icons-preview.png")


if __name__ == "__main__":
    main()
