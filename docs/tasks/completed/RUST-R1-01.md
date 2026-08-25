# RUST-R1-01 Rust workspace + package core/path model

**State:** COMPLETED
**Historical source task:** archived here; the obsolete standalone R1 package-core task source was removed after the Rust-first queue cleanup.

## Gate

Start only after the C++ package/path/signature semantics and hostile-path corpus remain green, and after release intent selects Rust R1 work explicitly.

## Can prepare now

- Keep the package/path corpus frozen and documented.
- Inventory C++ package-core behavior that Rust must match.
- Prepare dependency/SBOM/license review criteria for future Cargo dependencies.

## Must not do before gate opens

- Do not create a permanent C++/Rust runtime selector.
- Do not reinterpret Windows package path policy.
- Do not add Rust workspace/toolchain files as drive-by refactoring unless this real target starts.
