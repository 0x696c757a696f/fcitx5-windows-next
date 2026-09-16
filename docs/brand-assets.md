# Fcitx5 for Windows Next brand assets

The shipping icon family uses the user-approved scarf penguin artwork from the
connected ChatGPT design conversation. The committed visual master is
`resources/icons/source/fcitx5-master-approved.png`; it is the only visual authority
for this family. It is treated as original project branding artwork,
with no third-party logo, mascot, or downloaded brand asset incorporated.

- `fcitx5.ico`, `fcitx5-paused.ico`, and `fcitx5-error.ico` derive from the
  same transparent master. The base is state-neutral; paused and error add a
  lower-right amber or red badge with a light contrast ring. Paused uses white pause bars; error uses a white diagonal X.
- `fcitx5-tsf.ico` derives from the same master with a tighter face/scarf crop
  for small TSF and shell surfaces. It is not a second mascot.
- The generator performs only deterministic alpha trimming, crop/padding,
  resize, small-size sharpening, badge compositing, and PNG-backed ICO writing;
  it does not redraw the penguin or depend on a network, font, or image service.
- Every ICO contains exactly these nine authored frames, in this order:
  `16, 20, 24, 32, 40, 48, 64, 128, 256`.
- `fcitx5-icons-preview.png` is a deterministic review sheet showing the
  running/paused/error family plus product and TSF small-size examples on light
  and dark backgrounds. It is not Explorer, taskbar, accessibility, or host evidence.

Licensing: the committed master and deterministic derivatives are project
branding assets and may be distributed under the repository license. Their
provenance is recorded here so future upstream synchronization does not replace
the approved visual authority with a procedural or unrelated icon.
