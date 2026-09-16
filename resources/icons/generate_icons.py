"""Generate deterministic Modern Penguin icon assets.

The artwork is an original, language-neutral penguin mark for Fcitx5 for
Windows Next. A deep navy rounded-square plate contains a compact penguin
with a warm-white face/belly, orange beak, and a small Fcitx teal slash. The
area outside the plate stays transparent. There is no text, language glyph,
Windows or keyboard symbol, third-party trademark, online asset, or font dependency
in the artwork.

Every ICO frame is rendered independently at its requested size. Small,
compact, and full penguin variants are drawn directly for their size bands;
the generator never resizes one large master into all ICO frames.
"""

from io import BytesIO
from pathlib import Path
import struct

from PIL import Image, ImageDraw


OUTPUT = Path(__file__).resolve().parent
ICO_SIZES = (16, 20, 24, 32, 40, 48, 64, 128, 256)
TSF_ICO_SIZES = ICO_SIZES
SUPERSAMPLE = 4

PLATE = (23, 32, 51, 255)
PENGUIN = (9, 13, 22, 255)
PENGUIN_ALT = (34, 43, 58, 255)
BELLY = (247, 245, 239, 255)
BEAK = (245, 158, 11, 255)
BRAND_TEAL = (34, 184, 167, 255)
PAUSE = (245, 158, 11, 255)
ERROR = (220, 60, 69, 255)
BADGE_RING = (255, 253, 247, 255)
LIGHT_BACKGROUND = (244, 244, 244, 255)
DARK_BACKGROUND = (32, 32, 32, 255)
MICRO_MIN_FACE_WIDTH_PX = 7
MICRO_MIN_BEAK_WIDTH_PX = 3
MICRO_MIN_EYE_SEPARATION_PX = 2
MICRO_EYE_RADIUS = 0.025


def point(canvas_size: int, x: float, y: float) -> tuple[int, int]:
    return round(x * canvas_size), round(y * canvas_size)


def box(canvas_size: int, left: float, top: float, right: float, bottom: float) -> tuple[int, int, int, int]:
    return (*point(canvas_size, left, top), *point(canvas_size, right, bottom))


def rounded_polyline(draw: ImageDraw.ImageDraw, canvas_size: int,
                     coordinates: tuple[tuple[float, float], ...],
                     color: tuple[int, int, int, int], width_ratio: float) -> None:
    points = [point(canvas_size, x, y) for x, y in coordinates]
    width = max(1, round(width_ratio * canvas_size))
    draw.line(points, fill=color, width=width, joint="curve")
    cap = max(1, width // 2)
    for x, y in points:
        draw.ellipse((x - cap, y - cap, x + cap, y + cap), fill=color)


def plate(draw: ImageDraw.ImageDraw, canvas_size: int, margin: float, radius: float) -> None:
    draw.rounded_rectangle(
        box(canvas_size, margin, margin, 1.0 - margin, 1.0 - margin),
        radius=round(radius * canvas_size),
        fill=PLATE,
    )


def ellipse(draw: ImageDraw.ImageDraw, canvas_size: int,
            bounds: tuple[float, float, float, float],
            fill: tuple[int, int, int, int]) -> None:
    draw.ellipse(box(canvas_size, *bounds), fill=fill)


def rounded_rect(draw: ImageDraw.ImageDraw, canvas_size: int,
                 bounds: tuple[float, float, float, float],
                 radius: float, fill: tuple[int, int, int, int]) -> None:
    draw.rounded_rectangle(box(canvas_size, *bounds), radius=round(radius * canvas_size), fill=fill)


def penguin_face_belly(draw: ImageDraw.ImageDraw, canvas_size: int,
                       eye_radius: float, beak_top: float,
                       slash_width: float,
                       face_bounds: tuple[float, float, float, float] =
                       (0.335, 0.285, 0.665, 0.785),
                       beak_left: float = 0.445,
                       beak_right: float = 0.555) -> None:
    """Draw the shared high-contrast face/belly identity block."""
    # One continuous warm-white face/belly block, rather than separate eyes
    # and belly blobs. This is the key small-size silhouette cue.
    rounded_rect(draw, canvas_size, face_bounds, 0.15, BELLY)
    ellipse(draw, canvas_size, (0.405 - eye_radius, 0.345 - eye_radius,
                                0.405 + eye_radius, 0.345 + eye_radius), PENGUIN)
    ellipse(draw, canvas_size, (0.595 - eye_radius, 0.345 - eye_radius,
                                0.595 + eye_radius, 0.345 + eye_radius), PENGUIN)
    beak = [point(canvas_size, beak_left, beak_top),
            point(canvas_size, beak_right, beak_top),
            point(canvas_size, 0.50, beak_top + 0.07)]
    draw.polygon(beak, fill=BEAK)
    rounded_polyline(draw, canvas_size,
                     ((0.595, 0.555), (0.625, 0.625)), BRAND_TEAL, slash_width)


def micro_penguin(draw: ImageDraw.ImageDraw, canvas_size: int) -> None:
    """Draw the five-element 16/20/24px micro-penguin."""
    # Keep the 16px identity floor explicit so coordinate tweaks cannot make
    # the face, beak, or eye spacing silently disappear at shell size.
    face_width = (0.72 - 0.28) * 16
    beak_width = (0.595 - 0.405) * 16
    eye_separation = (0.595 - 0.405 - (2 * MICRO_EYE_RADIUS)) * 16
    assert face_width >= MICRO_MIN_FACE_WIDTH_PX
    assert beak_width >= MICRO_MIN_BEAK_WIDTH_PX
    assert eye_separation >= MICRO_MIN_EYE_SEPARATION_PX

    # A continuous dark teardrop silhouette keeps the mark recognizable at
    # 16px without relying on a detailed illustration.
    rounded_rect(draw, canvas_size, (0.265, 0.14, 0.735, 0.86), 0.20, PENGUIN)
    ellipse(draw, canvas_size, (0.265, 0.10, 0.735, 0.58), PENGUIN)
    penguin_face_belly(draw, canvas_size, MICRO_EYE_RADIUS, 0.405, 0.050,
                       face_bounds=(0.28, 0.285, 0.72, 0.785),
                       beak_left=0.405, beak_right=0.595)


def compact_penguin(draw: ImageDraw.ImageDraw, canvas_size: int) -> None:
    """Draw the winged 32/40/48/64px compact penguin variant."""
    rounded_rect(draw, canvas_size, (0.265, 0.14, 0.735, 0.86), 0.20, PENGUIN)
    ellipse(draw, canvas_size, (0.265, 0.10, 0.735, 0.58), PENGUIN)
    rounded_polyline(draw, canvas_size,
                     ((0.31, 0.45), (0.22, 0.54), (0.30, 0.62)), PENGUIN_ALT, 0.065)
    rounded_polyline(draw, canvas_size,
                     ((0.69, 0.45), (0.78, 0.54), (0.70, 0.62)), PENGUIN_ALT, 0.065)
    penguin_face_belly(draw, canvas_size, 0.022, 0.405, 0.042)
    ellipse(draw, canvas_size, (0.355, 0.785, 0.475, 0.855), BEAK)
    ellipse(draw, canvas_size, (0.525, 0.785, 0.645, 0.855), BEAK)


def full_penguin(draw: ImageDraw.ImageDraw, canvas_size: int) -> None:
    """Draw the 128/256px full penguin logo treatment."""
    rounded_rect(draw, canvas_size, (0.265, 0.14, 0.735, 0.86), 0.20, PENGUIN)
    ellipse(draw, canvas_size, (0.265, 0.10, 0.735, 0.58), PENGUIN)
    rounded_polyline(draw, canvas_size,
                     ((0.31, 0.45), (0.22, 0.54), (0.30, 0.62)), PENGUIN_ALT, 0.075)
    rounded_polyline(draw, canvas_size,
                     ((0.69, 0.45), (0.78, 0.54), (0.70, 0.62)), PENGUIN_ALT, 0.075)
    penguin_face_belly(draw, canvas_size, 0.022, 0.405, 0.042)
    ellipse(draw, canvas_size, (0.355, 0.785, 0.475, 0.855), BEAK)
    ellipse(draw, canvas_size, (0.525, 0.785, 0.645, 0.855), BEAK)


def penguin_body(draw: ImageDraw.ImageDraw, canvas_size: int, size: int) -> None:
    """Select the direct-rendered size band for one product frame."""
    if size <= 24:
        micro_penguin(draw, canvas_size)
    elif size <= 64:
        compact_penguin(draw, canvas_size)
    else:
        full_penguin(draw, canvas_size)


def tsf_penguin_mark(draw: ImageDraw.ImageDraw, canvas_size: int, size: int) -> None:
    """Draw the compact TSF avatar directly, not from the product master."""
    rounded_rect(draw, canvas_size, (0.235, 0.20, 0.765, 0.78), 0.22, PENGUIN)
    ellipse(draw, canvas_size, (0.235, 0.16, 0.765, 0.58), PENGUIN)
    rounded_rect(draw, canvas_size, (0.315, 0.31, 0.685, 0.72), 0.16, BELLY)
    eye_radius = 0.026 if size <= 24 else 0.022
    ellipse(draw, canvas_size, (0.405 - eye_radius, 0.37 - eye_radius,
                                0.405 + eye_radius, 0.37 + eye_radius), PENGUIN)
    ellipse(draw, canvas_size, (0.595 - eye_radius, 0.37 - eye_radius,
                                0.595 + eye_radius, 0.37 + eye_radius), PENGUIN)
    draw.polygon([point(canvas_size, 0.44, 0.43),
                  point(canvas_size, 0.56, 0.43),
                  point(canvas_size, 0.50, 0.50)], fill=BEAK)
    rounded_polyline(draw, canvas_size,
                     ((0.60, 0.555), (0.625, 0.61)), BRAND_TEAL,
                     0.050 if size <= 24 else 0.042)


def status_badge(draw: ImageDraw.ImageDraw, canvas_size: int, state: str) -> None:
    """Draw a lower-right, warm-white-bordered state badge."""
    colors = {"paused": PAUSE, "error": ERROR}
    if state not in colors:
        raise ValueError(f"unknown icon state: {state}")

    center = point(canvas_size, 0.77, 0.77)
    outer_radius = round(0.14 * canvas_size)
    inner_radius = outer_radius - max(1, round(0.024 * canvas_size))
    draw.ellipse((center[0] - outer_radius, center[1] - outer_radius,
                  center[0] + outer_radius, center[1] + outer_radius), fill=BADGE_RING)
    draw.ellipse((center[0] - inner_radius, center[1] - inner_radius,
                  center[0] + inner_radius, center[1] + inner_radius),
                 fill=colors[state])

    if state == "paused":
        for left in (0.735, 0.795):
            rounded_rect(draw, canvas_size, (left, 0.715, left + 0.026, 0.825),
                         0.012, PENGUIN)
    else:
        rounded_polyline(draw, canvas_size,
                         ((0.77, 0.715), (0.77, 0.795)), BADGE_RING, 0.026)
        dot = max(1, round(0.026 * canvas_size))
        x, y = point(canvas_size, 0.77, 0.835)
        draw.ellipse((x - dot, y - dot, x + dot, y + dot), fill=BADGE_RING)


def render_product_icon(size: int, state: str | None = None) -> Image.Image:
    """Render one product frame; no other-size image is used as a source."""
    if size <= 0:
        raise ValueError("icon size must be positive")
    canvas_size = size * SUPERSAMPLE
    image = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    plate(draw, canvas_size, margin=0.075, radius=0.20)
    penguin_body(draw, canvas_size, size)
    if state is not None:
        status_badge(draw, canvas_size, state)
    return image.resize((size, size), Image.Resampling.LANCZOS)


def render_tsf_icon(size: int) -> Image.Image:
    """Render the TSF compact penguin directly for this size."""
    if size <= 0:
        raise ValueError("icon size must be positive")
    canvas_size = size * SUPERSAMPLE
    image = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    plate(draw, canvas_size, margin=0.11, radius=0.36)
    tsf_penguin_mark(draw, canvas_size, size)
    return image.resize((size, size), Image.Resampling.LANCZOS)


def png_bytes(image: Image.Image) -> bytes:
    output = BytesIO()
    image.save(output, format="PNG", optimize=False, compress_level=9)
    return output.getvalue()


def write_ico(name: str, frames: list[tuple[int, bytes]]) -> None:
    """Write PNG-backed ICO frames in the supplied ascending order."""
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
            0,
            0,
            1,
            32,
            len(encoded),
            offset,
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


def main() -> None:
    save_ico("fcitx5.ico", render_product_icon)
    save_ico("fcitx5-paused.ico", lambda size: render_product_icon(size, "paused"))
    save_ico("fcitx5-error.ico", lambda size: render_product_icon(size, "error"))
    save_ico("fcitx5-tsf.ico", render_tsf_icon, TSF_ICO_SIZES)

    # Preview includes state variants and the size bands on both light and
    # dark backgrounds. It is a review sheet, not a source image.
    preview = Image.new("RGBA", (1280, 700), LIGHT_BACKGROUND)
    preview.alpha_composite(Image.new("RGBA", (1280, 350), DARK_BACKGROUND), (0, 350))
    for row_y in (0, 350):
        for index, state in enumerate((None, "paused", "error")):
            preview.alpha_composite(render_product_icon(256, state), (24 + index * 286, row_y + 22))
        for index, size in enumerate((64, 32, 24, 16)):
            preview.alpha_composite(render_product_icon(size), (900 + index * 84, row_y + 130))
        for index, size in enumerate((32, 24, 20, 16)):
            preview.alpha_composite(render_tsf_icon(size), (900 + index * 84, row_y + 240))
    preview.save(OUTPUT / "fcitx5-icons-preview.png", format="PNG", optimize=False)


if __name__ == "__main__":
    main()
