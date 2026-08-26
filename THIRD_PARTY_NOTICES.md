# Third-party notices

The Phase 3 native engine links and distributes the pinned components recorded in
`third_party/dependencies.json`. Their upstream license texts must accompany every
binary package; this repository does not replace those terms.

The Fcitx5 Windows fork, libime and Chinese Addons provide the input-method core,
models and dictionaries. The packaged MSYS2 CLANG64 runtime DLLs provide the C++
runtime, compression, dynamic loading, gettext/iconv, threading and event-loop
support. Exact versions and source URLs are in the dependency manifest.

toml++ provides the TOML 1.0 syntax parser used by the typed Windows configuration
model. Project code remains responsible for unknown-key, enum, range and ownership validation.

nlohmann/json 3.12.0 provides JSON syntax parsing for strict package manifests,
lockfiles and release metadata. miniz 3.1.2 provides ZIP container parsing and
decompression. Project code validates schemas, resource budgets and every archive
path before extraction. Both dependencies are MIT licensed and are downloaded from
pinned upstream release assets with SHA-256 verification.

Repositories listed only in `docs/reference-baseline.md` remain design and behavior
references unless they also appear in the dependency manifest.

wind-ui-rust is vendored under `third_party/wind-ui-rust` and consumed by the
Rust Config Settings UI prototype as a path dependency. It is licensed
MIT OR Apache-2.0; local Windows x86 compatibility fixes are kept in the
vendored source until they can be reconciled upstream.
