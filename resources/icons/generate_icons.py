"""Generate deterministic original Fcitx5 for Windows Next penguin icons.

The artwork is geometric and source-controlled so builds do not depend on
fonts, online assets, or third-party trademark/logo files.  The product icon
and the TSF picker glyph intentionally share the same silhouette family, but
the TSF icon is drawn as a separate micro glyph for 16/20/24 px instead of
shrinking the detailed product master.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter


OUTPUT = Path(__file__).resolve().parent
MASTER_SIZE = 1024
ICO_SIZES = [(16, 16), (20, 20), (24, 24), (32, 32), (40, 40), (48, 48),
             (64, 64), (128, 128), (256, 256)]
TSF_ICO_SIZES = [(16, 16), (20, 20), (24, 24), (32, 32), (48, 48), (256, 256)]


def rounded_rectangle_mask(size: int, margin: int, radius: int) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (margin, margin, size - margin - 1, size - margin - 1), radius=radius, fill=255
    )
    return mask


def background_plate(size: int = MASTER_SIZE) -> Image.Image:
    margin = round(size * 0.05)
    radius = round(size * 0.21)
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))

    shadow = Image.new("RGBA", image.size, (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle(
        (margin + 12, margin + 26, size - margin + 11, size - margin + 25),
        radius=radius,
        fill=(9, 30, 66, 50),
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(round(size * 0.015)))
    image.alpha_composite(shadow)

    gradient = Image.new("RGBA", image.size, (0, 0, 0, 0))
    pixels = gradient.load()
    top = (21, 184, 166)
    bottom = (37, 99, 235)
    for y in range(size):
        ratio = y / (size - 1)
        color = tuple(round(top[index] * (1 - ratio) + bottom[index] * ratio)
                      for index in range(3))
        for x in range(size):
            pixels[x, y] = (*color, 255)
    gradient.putalpha(rounded_rectangle_mask(size, margin, radius))
    image.alpha_composite(gradient)
    return image


def draw_product_penguin(draw: ImageDraw.ImageDraw, scale: float = 1.0,
                         offset: tuple[int, int] = (0, 0)) -> None:
    ox, oy = offset
    def box(values: tuple[int, int, int, int]) -> tuple[int, int, int, int]:
        return tuple(round(v * scale) + (ox if i % 2 == 0 else oy)
                     for i, v in enumerate(values))

    black = (18, 25, 37, 255)
    charcoal = (30, 41, 59, 255)
    white = (255, 255, 255, 248)
    orange = (249, 115, 22, 255)
    amber = (251, 191, 36, 255)

    # Flippers live behind the body.  Their oversized curves survive downscale.
    draw.ellipse(box((190, 420, 410, 760)), fill=charcoal)
    draw.ellipse(box((614, 420, 834, 760)), fill=charcoal)

    # Head and body share a simplified Fcitx-specific silhouette: rounded,
    # friendly, and neutral across languages.
    draw.ellipse(box((284, 148, 740, 604)), fill=black)
    draw.rounded_rectangle(box((252, 342, 772, 882)), radius=round(238 * scale), fill=black)
    draw.ellipse(box((344, 358, 680, 850)), fill=white)

    # Face mask, eyes, and beak are deliberately bold; no text, flags, or
    # engine-specific markings are embedded in the identity.
    draw.ellipse(box((356, 224, 506, 410)), fill=white)
    draw.ellipse(box((518, 224, 668, 410)), fill=white)
    draw.ellipse(box((418, 300, 466, 348)), fill=black)
    draw.ellipse(box((558, 300, 606, 348)), fill=black)
    draw.polygon([box((482, 382, 482, 382))[0:2],
                  box((542, 382, 542, 382))[0:2],
                  box((512, 438, 512, 438))[0:2]], fill=orange)

    # Feet form a stable orange base at small sizes.
    draw.ellipse(box((326, 794, 502, 920)), fill=amber)
    draw.ellipse(box((522, 794, 698, 920)), fill=amber)
    draw.rounded_rectangle(box((388, 808, 636, 880)), radius=round(40 * scale), fill=amber)


def draw_state_badge(draw: ImageDraw.ImageDraw, state: str) -> None:
    if state == "running":
        return
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


def product_icon(state: str = "running") -> Image.Image:
    image = background_plate()
    draw = ImageDraw.Draw(image)
    draw_product_penguin(draw)
    draw_state_badge(draw, state)
    return image


def micro_penguin(size: int = MASTER_SIZE) -> Image.Image:
    """Draw a dedicated TSF picker glyph with explicit small-size geometry."""
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    s = size / 1024.0

    def box(values: tuple[int, int, int, int]) -> tuple[int, int, int, int]:
        return tuple(round(v * s) for v in values)

    black = (15, 23, 42, 255)
    white = (255, 255, 255, 255)
    orange = (249, 115, 22, 255)
    teal = (20, 184, 166, 255)

    # A thin product-family halo gives the 16 px glyph contrast in Light, Dark,
    # and High Contrast-like shells without turning it into a colored app tile.
    draw.ellipse(box((96, 72, 928, 952)), fill=teal)
    draw.ellipse(box((150, 126, 874, 906)), fill=(255, 255, 255, 255))
    draw.ellipse(box((178, 148, 846, 888)), fill=black)
    draw.ellipse(box((308, 332, 716, 858)), fill=white)
    draw.ellipse(box((330, 210, 498, 442)), fill=white)
    draw.ellipse(box((526, 210, 694, 442)), fill=white)
    draw.ellipse(box((402, 302, 458, 358)), fill=black)
    draw.ellipse(box((566, 302, 622, 358)), fill=black)
    draw.polygon([(466 * s, 420 * s), (558 * s, 420 * s), (512 * s, 504 * s)], fill=orange)
    draw.ellipse(box((324, 782, 496, 928)), fill=orange)
    draw.ellipse(box((528, 782, 700, 928)), fill=orange)
    return image


def save_ico(name: str, image: Image.Image, sizes: list[tuple[int, int]]) -> None:
    image.save(OUTPUT / name, format="ICO", sizes=sizes, bitmap_format="png")


def main() -> None:
    save_ico("fcitx5.ico", product_icon("running"), ICO_SIZES)
    save_ico("fcitx5-paused.ico", product_icon("paused"), ICO_SIZES)
    save_ico("fcitx5-error.ico", product_icon("error"), ICO_SIZES)
    save_ico("fcitx5-tsf.ico", micro_penguin(), TSF_ICO_SIZES)

    preview = Image.new("RGBA", (1280, 340), (245, 247, 250, 255))
    for index, state in enumerate(("running", "paused", "error")):
        icon = product_icon(state).resize((256, 256), Image.Resampling.LANCZOS)
        preview.alpha_composite(icon, (40 + index * 286, 42))
    preview.alpha_composite(micro_penguin().resize((96, 96), Image.Resampling.LANCZOS),
                            (970, 60))
    preview.alpha_composite(micro_penguin().resize((24, 24), Image.Resampling.LANCZOS),
                            (1006, 188))
    preview.save(OUTPUT / "fcitx5-icons-preview.png")


if __name__ == "__main__":
    main()
