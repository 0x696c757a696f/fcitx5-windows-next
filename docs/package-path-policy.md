# Windows package path policy

Package archive and manifest paths are package-relative logical paths. They are
not Windows command-line paths and are never interpreted as URLs.

Accepted package file paths:

- use `/` as the only separator;
- are relative, non-empty, and at most 512 bytes;
- have no empty, `.`, or `..` component;
- have no absolute root, drive prefix, UNC prefix, colon/ADS marker, backslash,
  NUL byte, or C0 control character;
- have no component ending in a dot or space;
- have no DOS device component after trimming the extension stem:
  `CON`, `PRN`, `AUX`, `NUL`, `COM1` through `COM9`, and `LPT1` through
  `LPT9`, case-insensitively, including forms such as `con.txt`;
- are unique under Windows ordinal case-insensitive comparison.

Archive staging must also reject symlink/reparse entries and must verify the
staged filesystem tree after extraction/copy before activation. A package must
not be able to escape the staging root through traversal, device names,
case-collision overwrite, symlink, junction, mount point, or another reparse
point.

The machine-readable corpus for this policy is
`tests/fixtures/package_path_corpus.json`. C++ tests consume that corpus now;
Rust R1 must consume the same file as differential input before cutover.
