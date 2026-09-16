# Fcitx5 for Windows Next brand assets

The Modern Penguin mark is original geometric artwork generated from
`resources/icons/generate_icons.py`. It uses a deep-navy rounded-square plate
with a transparent outside, a dark penguin silhouette, a warm-white face and
belly, an orange beak, and a small Fcitx teal input slash.

- `fcitx5.ico`, `fcitx5-paused.ico`, and `fcitx5-error.ico` share the same
  primary Modern Penguin mark. The base icon is state-neutral; paused and
  error preserve the penguin and add lower-right state badges (amber or red)
  with a warm-white border.
- `fcitx5-tsf.ico` is an independently rendered compact penguin avatar. It is
  not a downscaled product master and remains language-neutral at small sizes.
- 16/20/24 px use a micro-penguin; 32/40/48/64 px use a compact penguin;
  128/256 px use the full logo treatment. Each band is rendered directly for
  its requested size.
- Every ICO contains exactly these nine frames, in this order:
  `16, 20, 24, 32, 40, 48, 64, 128, 256`.
- Pillow draws and encodes each requested frame independently, while the
  deterministic tiny ICO writer preserves the frame order and embedded pixel
  dimensions. The preview sheet shows state variants and product/TSF sizes on
  both light and dark backgrounds.
- The artwork contains no text, no language characters or glyphs, no Windows
  or keyboard symbols, no third-party trademarks, and no online or font assets.

Licensing: these assets are authored for this repository and may be
distributed under the repository license. They do not incorporate third-party
logo artwork or downloaded icon packs.
