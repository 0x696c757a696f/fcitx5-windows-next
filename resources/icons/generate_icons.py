"""Generate deterministic Fcitx5 Windows icons from the approved brand master.

The committed PNG is the visual authority. This script only performs
deterministic crop, padding, resize, sharpening, and state-badge operations;
it does not redraw or reinterpret the penguin.
"""

from io import BytesIO
from pathlib import Path
import struct

from PIL import Image, ImageDraw, ImageFilter


OUTPUT = Path(__file__).resolve().parent
MASTER_SOURCE = OUTPUT / "source" / "fcitx5-master-approved.png"
ICO_SIZES = (16, 20, 24, 32, 40, 48, 64, 128, 256)
TSF_ICO_SIZES = ICO_SIZES
SUPERSAMPLE = 4
TRANSPARENT = (0, 0, 0, 0)
PAUSE = (245, 158, 11, 255)
ERROR = (239, 68, 68, 255)
BADGE_RING = (248, 250, 252, 255)
BADGE_GLYPH = (255, 255, 255, 255)
LIGHT_BACKGROUND = (245, 247, 250, 255)
DARK_BACKGROUND = (31, 41, 55, 255)


def load_master() -> Image.Image:
    if not MASTER_SOURCE.is_file():
        raise FileNotFoundError(f"approved brand master is missing: {MASTER_SOURCE}")
    image = Image.open(MASTER_SOURCE).convert("RGBA")
    alpha = image.getchannel("A")
    # Ignore low-alpha export fringe while preserving antialiased artwork.
    mask = alpha.point(lambda value: 255 if value >= 8 else 0)
    bounds = mask.getbbox()
    if bounds is None:
        raise ValueError("approved brand master has no visible pixels")
    return image.crop(bounds)


MASTER = load_master()


def square_frame(source: Image.Image, size: int, *, margin: float,
                 top_ratio: float | None = None) -> Image.Image:
    """Fit one deterministic crop into a transparent square frame."""
    crop = source
    if top_ratio is not None:
        crop = source.crop((0, 0, source.width, max(1, round(source.height * top_ratio))))
    extent = max(crop.width, crop.height)
    target = max(1, round(size * SUPERSAMPLE * (1.0 - 2.0 * margin)))
    scale = target / extent
    resized = crop.resize(
        (max(1, round(crop.width * scale)), max(1, round(crop.height * scale))),
        Image.Resampling.LANCZOS,
    )
    canvas_size = size * SUPERSAMPLE
    canvas = Image.new("RGBA", (canvas_size, canvas_size), TRANSPARENT)
    canvas.alpha_composite(
        resized,
        ((canvas_size - resized.width) // 2, (canvas_size - resized.height) // 2),
    )
    return canvas


def add_status_badge(image: Image.Image, state: str) -> None:
    colors = {"paused": PAUSE, "error": ERROR}
    if state not in colors:
        raise ValueError(f"unknown icon state: {state}")
    draw = ImageDraw.Draw(image)
    side = image.width
    cx = round(side * 0.78)
    cy = round(side * 0.78)
    outer = max(2, round(side * 0.16))
    ring = max(1, round(side * 0.028))
    inner = max(1, outer - ring)
    draw.ellipse((cx - outer, cy - outer, cx + outer, cy + outer), fill=BADGE_RING)
    draw.ellipse((cx - inner, cy - inner, cx + inner, cy + inner), fill=colors[state])
    if state == "paused":
        bar_width = max(1, round(side * 0.026))
        bar_height = max(2, round(side * 0.095))
        for offset in (-round(side * 0.032), round(side * 0.032)):
            draw.rounded_rectangle(
                (cx + offset - bar_width // 2, cy - bar_height // 2,
                 cx + offset + bar_width // 2, cy + bar_height // 2),
                radius=max(1, bar_width // 2), fill=BADGE_GLYPH,
            )
    else:
        stroke = max(1, round(side * 0.026))
        gap = round(side * 0.055)
        draw.line((cx - gap, cy - gap, cx + gap, cy + gap),
                  fill=BADGE_GLYPH, width=stroke)
        draw.line((cx - gap, cy + gap, cx + gap, cy - gap),
                  fill=BADGE_GLYPH, width=stroke)


def render_master_icon(size: int, state: str | None = None,
                       *, tsf: bool = False) -> Image.Image:
    if size <= 0:
        raise ValueError("icon size must be positive")
    # Tiny shell icons get a face/scarf-biased crop; larger frames preserve the
    # full approved silhouette. TSF uses the same master with a tighter crop.
    if tsf:
        top_ratio = 0.72 if size <= 32 else 0.80
        margin = 0.055
    elif size <= 24:
        top_ratio = 0.82
        margin = 0.035
    else:
        top_ratio = None
        margin = 0.04
    image = square_frame(MASTER, size, margin=margin, top_ratio=top_ratio)
    if size <= 24:
        image = image.filter(ImageFilter.UnsharpMask(radius=0.65, percent=115, threshold=2))
    if state is not None:
        add_status_badge(image, state)
    return image.resize((size, size), Image.Resampling.LANCZOS)


def png_bytes(image: Image.Image) -> bytes:
    output = BytesIO()
    image.save(output, format="PNG", optimize=False, compress_level=9)
    return output.getvalue()


def write_ico(name: str, frames: list[tuple[int, bytes]]) -> None:
    """Write PNG-backed ICO frames in ascending order."""
    directory_size = 6 + 16 * len(frames)
    directory = bytearray(struct.pack("<HHH", 0, 1, len(frames)))
    payload = bytearray()
    offset = directory_size
    previous_size = 0
    for size, encoded in frames:
        if not previous_size < size <= 256:
            raise ValueError("ICO frame sizes must be ascending and fit the directory field")
        directory.extend(struct.pack(
            "<BBBBHHII",
            0 if size == 256 else size,
            0 if size == 256 else size,
            0, 0, 1, 32, len(encoded), offset,
        ))
        payload.extend(encoded)
        offset += len(encoded)
        previous_size = size
    (OUTPUT / name).write_bytes(directory + payload)


def save_ico(name: str, renderer, sizes: tuple[int, ...] = ICO_SIZES) -> None:
    frames = []
    for size in sizes:
        frame = renderer(size)
        if frame.size != (size, size):
            raise ValueError("renderer returned pixels at the wrong ICO size")
        frames.append((size, png_bytes(frame)))
    write_ico(name, frames)


def draw_preview_row(preview: Image.Image, y: int, background: tuple[int, int, int, int]) -> None:
    draw = ImageDraw.Draw(preview)
    draw.rectangle((0, y, preview.width, y + 599), fill=background)
    label_color = (31, 41, 55, 255) if sum(background[:3]) > 400 else (245, 247, 250, 255)
    for index, state in enumerate((None, "paused", "error")):
        preview.alpha_composite(render_master_icon(176, state), (40 + index * 205, y + 24))
    for index, size in enumerate(ICO_SIZES):
        cell_x = 650 + index * 104
        display_size = 96 if size >= 128 else 76
        frame = render_master_icon(size).resize(
            (display_size, display_size),
            Image.Resampling.NEAREST if size <= 32 else Image.Resampling.LANCZOS,
        )
        preview.alpha_composite(frame, (cell_x + (96 - display_size) // 2, y + 260))
        draw.text((cell_x + 35, y + 365), f"{size}", fill=label_color)
    for index, size in enumerate((32, 24, 20, 16)):
        frame = render_master_icon(size, tsf=True).resize((76, 76), Image.Resampling.NEAREST)
        cell_x = 760 + index * 104
        preview.alpha_composite(frame, (cell_x + 10, y + 415))
        draw.text((cell_x + 34, y + 505), f"TSF {size}", fill=label_color)


def main() -> None:
    save_ico("fcitx5.ico", render_master_icon)
    save_ico("fcitx5-paused.ico", lambda size: render_master_icon(size, "paused"))
    save_ico("fcitx5-error.ico", lambda size: render_master_icon(size, "error"))
    save_ico("fcitx5-tsf.ico", lambda size: render_master_icon(size, tsf=True), TSF_ICO_SIZES)

    preview = Image.new("RGBA", (1600, 1200), LIGHT_BACKGROUND)
    draw_preview_row(preview, 0, LIGHT_BACKGROUND)
    draw_preview_row(preview, 600, DARK_BACKGROUND)
    preview.save(OUTPUT / "fcitx5-icons-preview.png", format="PNG", optimize=False)


if __name__ == "__main__":
    main()
