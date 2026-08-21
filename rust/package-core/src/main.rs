#![forbid(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::error::Error;
use std::path::PathBuf;

use fcitx5_package_core::{
    activate_staged_payload_tree, finalize_installed_package_removal,
    is_safe_relative_package_path, mark_installed_package_for_removal, parse_manifest,
    parse_signature_envelope, parse_trusted_keys, read_installed_lockfile,
    set_installed_package_state, sha256_digest, stage_validated_archive_zip,
    upsert_installed_lock_entry, validate_manifest_compatibility, verify_manifest_signature,
    verify_payload_root, verify_repository_index, verify_repository_index_envelope,
    verify_signature_envelope, HexDigest32, PackageId, PackageLifecycleState, RepositoryIndex,
    SignedObject, TrustAlgorithm, TrustedKey,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("fcitx5-package: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut self_check = false;
    let mut audit_self_pe = false;
    let mut trusted_keys_path = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" => {
                println!("fcitx5-package {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--validate-manifest" => {
                let path = args.next().ok_or("--validate-manifest requires MANIFEST")?;
                let manifest = parse_manifest(&read_bounded_text(path, 1024 * 1024)?)?;
                print_manifest(&manifest, false);
                return Ok(());
            }
            "--validate-keyring" => {
                let path = args.next().ok_or("--validate-keyring requires KEYRING")?;
                let keys = parse_trusted_keys(&read_bounded_text(path, 1024 * 1024)?)?;
                if keys.is_empty() {
                    return Err("trusted keyring is empty".into());
                }
                println!("keys={}", keys.len());
                return Ok(());
            }
            "--verify-manifest-v2" => {
                let manifest_path = args
                    .next()
                    .ok_or("--verify-manifest-v2 requires MANIFEST")?;
                let signature_path = args
                    .next()
                    .ok_or("--verify-manifest-v2 requires SIG_JSON")?;
                let keyring_path = args.next().ok_or("--verify-manifest-v2 requires KEYRING")?;
                let manifest_bytes = read_bounded_bytes(manifest_path, 1024 * 1024)?;
                let manifest_text = std::str::from_utf8(&manifest_bytes)?;
                let manifest = parse_manifest(manifest_text)?;
                let signature = read_bounded_text(signature_path, 1024 * 1024)?;
                let envelope = parse_signature_envelope(&signature, SignedObject::PackageManifest)?;
                let keyring = read_bounded_text(keyring_path, 1024 * 1024)?;
                let keys = parse_trusted_keys(&keyring)?;
                verify_signature_envelope(
                    &manifest_bytes,
                    &envelope,
                    &keys,
                    SignedObject::PackageManifest,
                    manifest.key_id(),
                )?;
                println!("manifest_signature=verified");
                return Ok(());
            }
            "--install" => {
                let archive_path = args.next().ok_or("--install requires ARCHIVE")?;
                let install_root = args.next().ok_or("--install requires INSTALL_ROOT")?;
                let transaction_id = args.next().ok_or("--install requires TRANSACTION_ID")?;
                let keyring_path = args.next().ok_or("--install requires KEYRING")?;
                let keys = read_trusted_keys(keyring_path)?;
                let staged = stage_validated_archive_zip(
                    archive_path,
                    &install_root,
                    &transaction_id,
                    &keys,
                )?;
                let manifest_bytes = read_bounded_bytes(staged.join("manifest.json"), 1024 * 1024)?;
                let manifest_text = std::str::from_utf8(&manifest_bytes)?;
                let manifest = parse_manifest(manifest_text)?;
                activate_staged_payload_tree(&staged, install_root, &keys)?;
                print_manifest(&manifest, true);
                return Ok(());
            }
            "--repair" => {
                let install_root = args.next().ok_or("--repair requires INSTALL_ROOT")?;
                let keyring_path = args.next().ok_or("--repair requires KEYRING")?;
                let keys = read_trusted_keys(keyring_path)?;
                verify_installed_packages(&PathBuf::from(install_root), &keys)?;
                println!("repair=verified");
                return Ok(());
            }
            "--verify-repository" => {
                let index_path = args.next().ok_or("--verify-repository requires INDEX")?;
                let signature_path = args
                    .next()
                    .ok_or("--verify-repository requires SIGNATURE")?;
                let keyring_path = args.next().ok_or("--verify-repository requires KEYRING")?;
                let channel = args.next().unwrap_or_else(|| "stable".to_owned());
                let index_bytes = read_bounded_bytes(index_path, 1024 * 1024)?;
                let signature = read_bounded_bytes(signature_path, 16 * 1024)?;
                let keys = read_trusted_keys(keyring_path)?;
                let index = verify_repository_index(&index_bytes, &signature, &keys, &channel)?;
                print_repository_index(&index);
                return Ok(());
            }
            "--verify-repository-v2" => {
                let index_path = args.next().ok_or("--verify-repository-v2 requires INDEX")?;
                let signature_path = args
                    .next()
                    .ok_or("--verify-repository-v2 requires SIG_JSON")?;
                let keyring_path = args
                    .next()
                    .ok_or("--verify-repository-v2 requires KEYRING")?;
                let channel = args.next().unwrap_or_else(|| "stable".to_owned());
                let index_bytes = read_bounded_bytes(index_path, 1024 * 1024)?;
                let signature = read_bounded_text(signature_path, 1024 * 1024)?;
                let envelope = parse_signature_envelope(&signature, SignedObject::RepositoryIndex)?;
                let keys = read_trusted_keys(keyring_path)?;
                let index =
                    verify_repository_index_envelope(&index_bytes, &envelope, &keys, &channel)?;
                print_repository_index(&index);
                return Ok(());
            }
            "--list" => {
                let install_root = args.next().ok_or("--list requires INSTALL_ROOT")?;
                for entry in read_installed_lockfile(install_root)? {
                    println!(
                        "{}\t{}\t{}\t{}",
                        entry.id().as_str(),
                        entry.version(),
                        entry.state().as_str(),
                        entry.manifest_sha256().as_str()
                    );
                }
                return Ok(());
            }
            "--state" => {
                let install_root = args.next().ok_or("--state requires INSTALL_ROOT")?;
                let package_id = args.next().ok_or("--state requires PACKAGE_ID")?;
                let state = args.next().ok_or("--state requires STATE")?;
                let state = parse_cli_lifecycle_state(&state)?;
                set_installed_package_state(install_root, &package_id, state)?;
                return Ok(());
            }
            "--mark-remove" => {
                let install_root = args.next().ok_or("--mark-remove requires INSTALL_ROOT")?;
                let package_id = args.next().ok_or("--mark-remove requires PACKAGE_ID")?;
                mark_installed_package_for_removal(install_root, &package_id)?;
                return Ok(());
            }
            "--finalize-remove" => {
                let install_root = args
                    .next()
                    .ok_or("--finalize-remove requires INSTALL_ROOT")?;
                let package_id = args.next().ok_or("--finalize-remove requires PACKAGE_ID")?;
                finalize_installed_package_removal(install_root, &package_id)?;
                return Ok(());
            }
            "--self-check" => self_check = true,
            "--audit-self-pe" => audit_self_pe = true,
            "--trusted-keys" => {
                let value = args
                    .next()
                    .ok_or("--trusted-keys requires a path argument")?;
                trusted_keys_path = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }

    if !self_check {
        print_usage();
        return Err("no command selected".into());
    }

    self_check_core()?;
    if let Some(path) = trusted_keys_path {
        self_check_trusted_keys(&path)?;
    }
    if audit_self_pe {
        audit_pe_artifact(&std::env::current_exe()?)?;
    }
    println!("fcitx5-package-core self-check ok");
    Ok(())
}

fn self_check_core() -> Result<(), Box<dyn Error>> {
    ensure(
        is_safe_relative_package_path("bin/addon.dll"),
        "valid package path rejected",
    )?;
    ensure(
        !is_safe_relative_package_path("bin/CON"),
        "DOS device package path accepted",
    )?;

    let manifest = parse_manifest(&golden_manifest())?;
    validate_manifest_compatibility(&manifest, runtime_architecture())?;

    let mut lock = Vec::new();
    upsert_installed_lock_entry(
        &mut lock,
        PackageId::parse("fcitx5-rime")?,
        "1.0.0".to_owned(),
        HexDigest32::parse(&"a".repeat(64))?,
    )?;
    ensure(lock.len() == 1, "installed lock entry was not inserted")?;
    ensure(
        lock[0].id().as_str() == "fcitx5-rime" && lock[0].version() == "1.0.0",
        "installed lock entry identity mismatch",
    )?;
    Ok(())
}

fn self_check_trusted_keys(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let keys = read_trusted_keys(path)?;
    ensure(
        keys.iter().any(|key| {
            key.id().as_str() == "official-2026-mldsa65"
                && key.algorithm() == &TrustAlgorithm::Mldsa65
                && !key.revoked()
                && key.public_key().len() == 1952
        }),
        "official ML-DSA-65 trusted public key is missing",
    )?;
    Ok(())
}

fn read_trusted_keys(path: impl AsRef<std::path::Path>) -> Result<Vec<TrustedKey>, Box<dyn Error>> {
    let keyring = read_bounded_text(path, 1024 * 1024)?;
    let keys = parse_trusted_keys(&keyring)?;
    if keys.is_empty() {
        return Err("trusted keyring is empty".into());
    }
    Ok(keys)
}

fn verify_installed_packages(
    install_root: &std::path::Path,
    trusted_keys: &[TrustedKey],
) -> Result<(), Box<dyn Error>> {
    for entry in read_installed_lockfile(install_root)? {
        let manifest_path = install_root
            .join("manifests")
            .join(entry.id().as_str())
            .join(format!("{}.json", entry.version()));
        let signature_path = install_root
            .join("manifests")
            .join(entry.id().as_str())
            .join(format!("{}.sig", entry.version()));
        let manifest_bytes = read_bounded_bytes(&manifest_path, 1024 * 1024)?;
        ensure(
            sha256_digest(&manifest_bytes) == *entry.manifest_sha256(),
            "installed manifest hash differs from packages.lock",
        )?;
        let manifest_text = std::str::from_utf8(&manifest_bytes)?;
        let manifest = parse_manifest(manifest_text)?;
        ensure(
            manifest.id() == entry.id() && manifest.version() == entry.version(),
            "installed manifest identity differs from packages.lock",
        )?;
        let trusted_key = trusted_keys
            .iter()
            .find(|candidate| candidate.id() == manifest.key_id())
            .ok_or("installed manifest key is no longer trusted")?;
        let signature = read_bounded_bytes(signature_path, 16 * 1024)?;
        verify_manifest_signature(&manifest_bytes, &signature, trusted_key)?;
        verify_payload_root(
            &manifest,
            install_root
                .join("versions")
                .join(entry.id().as_str())
                .join(entry.version()),
        )?;
    }
    Ok(())
}

fn read_bounded_text(
    path: impl AsRef<std::path::Path>,
    maximum: u64,
) -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(read_bounded_bytes(path, maximum)?)?)
}

fn read_bounded_bytes(
    path: impl AsRef<std::path::Path>,
    maximum: u64,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let path = path.as_ref();
    let metadata = std::fs::metadata(path)?;
    ensure(metadata.len() <= maximum, "input is missing or too large")?;
    Ok(std::fs::read(path)?)
}

fn print_manifest(manifest: &fcitx5_package_core::Manifest, verified: bool) {
    println!("id={}", manifest.id().as_str());
    println!("version={}", manifest.version());
    println!("source_commit={}", manifest.source_commit());
    println!("license={}", manifest.license());
    println!("key_id={}", manifest.key_id().as_str());
    println!(
        "signature_verified={}",
        if verified { "true" } else { "false" }
    );
    println!("permissions={}", manifest.permissions().join(","));
}

fn print_repository_index(index: &RepositoryIndex) {
    for entry in index.packages() {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            entry.id(),
            entry.version(),
            entry.release_sequence(),
            entry.architecture(),
            entry.sha256().as_str(),
            entry.download_url(),
            entry.title()
        );
    }
}

fn parse_cli_lifecycle_state(value: &str) -> Result<PackageLifecycleState, Box<dyn Error>> {
    match value {
        "installed" => Ok(PackageLifecycleState::Installed),
        "enabled" => Ok(PackageLifecycleState::Enabled),
        "disabled" => Ok(PackageLifecycleState::Disabled),
        "pending_update" => Ok(PackageLifecycleState::PendingUpdate),
        "pending_remove" => Ok(PackageLifecycleState::PendingRemove),
        "broken" => Ok(PackageLifecycleState::Broken),
        "quarantined" => Ok(PackageLifecycleState::Quarantined),
        _ => Err("package lifecycle state is invalid".into()),
    }
}

fn ensure(condition: bool, message: &'static str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn audit_pe_artifact(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let image = std::fs::read(path)?;
    let pe = PeImage::parse(&image)?;
    ensure(
        matches!(pe.machine, 0x014c | 0x8664 | 0xaa64),
        "Rust artifact has an unsupported PE machine type",
    )?;
    ensure(
        pe.major_os_version <= 10,
        "Rust artifact PE header requires an unsupported Windows major version",
    )?;

    let banned_imports = [
        "winhttp.dll",
        "wininet.dll",
        "urlmon.dll",
        "webview2loader.dll",
        "msedgewebview2.exe",
    ];
    for import in &pe.imports {
        ensure(
            !banned_imports
                .iter()
                .any(|banned| import.eq_ignore_ascii_case(banned)),
            "Rust artifact imports a network or web runtime library",
        )?;
    }
    Ok(())
}

struct PeImage {
    machine: u16,
    major_os_version: u16,
    imports: Vec<String>,
}

impl PeImage {
    fn parse(image: &[u8]) -> Result<Self, Box<dyn Error>> {
        ensure(read_u16(image, 0)? == 0x5a4d, "artifact is not an MZ image")?;
        let pe_offset = usize::try_from(read_u32(image, 0x3c)?)?;
        ensure(
            read_bytes(image, pe_offset, 4)? == b"PE\0\0",
            "artifact is not a PE image",
        )?;
        let machine = read_u16(image, pe_offset + 4)?;
        let section_count = usize::from(read_u16(image, pe_offset + 6)?);
        let optional_size = usize::from(read_u16(image, pe_offset + 20)?);
        let optional_offset = pe_offset + 24;
        let optional_magic = read_u16(image, optional_offset)?;
        let data_directory_offset = match optional_magic {
            0x10b => optional_offset + 96,
            0x20b => optional_offset + 112,
            _ => return Err("artifact has an unsupported PE optional header".into()),
        };
        let major_os_version = read_u16(image, optional_offset + 40)?;
        let import_rva = read_u32(image, data_directory_offset + 8)?;
        let section_offset = optional_offset + optional_size;
        let sections = parse_sections(image, section_offset, section_count)?;
        let imports = if import_rva == 0 {
            Vec::new()
        } else {
            parse_imports(image, &sections, import_rva)?
        };
        Ok(Self {
            machine,
            major_os_version,
            imports,
        })
    }
}

struct PeSection {
    virtual_address: u32,
    virtual_size: u32,
    raw_offset: u32,
    raw_size: u32,
}

fn parse_sections(
    image: &[u8],
    section_offset: usize,
    section_count: usize,
) -> Result<Vec<PeSection>, Box<dyn Error>> {
    let mut sections = Vec::new();
    for index in 0..section_count {
        let offset = section_offset + index * 40;
        sections.push(PeSection {
            virtual_size: read_u32(image, offset + 8)?,
            virtual_address: read_u32(image, offset + 12)?,
            raw_size: read_u32(image, offset + 16)?,
            raw_offset: read_u32(image, offset + 20)?,
        });
    }
    Ok(sections)
}

fn parse_imports(
    image: &[u8],
    sections: &[PeSection],
    import_rva: u32,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut imports = Vec::new();
    let mut descriptor_offset = rva_to_offset(sections, import_rva)?;
    loop {
        let original_first_thunk = read_u32(image, descriptor_offset)?;
        let time_date_stamp = read_u32(image, descriptor_offset + 4)?;
        let forwarder_chain = read_u32(image, descriptor_offset + 8)?;
        let name_rva = read_u32(image, descriptor_offset + 12)?;
        let first_thunk = read_u32(image, descriptor_offset + 16)?;
        if original_first_thunk == 0
            && time_date_stamp == 0
            && forwarder_chain == 0
            && name_rva == 0
            && first_thunk == 0
        {
            break;
        }
        let name_offset = rva_to_offset(sections, name_rva)?;
        imports.push(read_c_string(image, name_offset)?.to_ascii_lowercase());
        descriptor_offset += 20;
    }
    Ok(imports)
}

fn rva_to_offset(sections: &[PeSection], rva: u32) -> Result<usize, Box<dyn Error>> {
    for section in sections {
        let span = section.virtual_size.max(section.raw_size);
        if rva >= section.virtual_address && rva < section.virtual_address.saturating_add(span) {
            return Ok(usize::try_from(
                section
                    .raw_offset
                    .saturating_add(rva.saturating_sub(section.virtual_address)),
            )?);
        }
    }
    Err("artifact contains an RVA outside mapped sections".into())
}

fn read_c_string(image: &[u8], offset: usize) -> Result<String, Box<dyn Error>> {
    let mut end = offset;
    while end < image.len() && image[end] != 0 {
        end += 1;
    }
    ensure(end < image.len(), "artifact string is unterminated")?;
    Ok(std::str::from_utf8(&image[offset..end])?.to_owned())
}

fn read_bytes(image: &[u8], offset: usize, length: usize) -> Result<&[u8], Box<dyn Error>> {
    image
        .get(offset..offset + length)
        .ok_or_else(|| "artifact PE structure is truncated".into())
}

fn read_u16(image: &[u8], offset: usize) -> Result<u16, Box<dyn Error>> {
    let bytes: [u8; 2] = read_bytes(image, offset, 2)?.try_into()?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(image: &[u8], offset: usize) -> Result<u32, Box<dyn Error>> {
    let bytes: [u8; 4] = read_bytes(image, offset, 4)?.try_into()?;
    Ok(u32::from_le_bytes(bytes))
}

fn print_usage() {
    println!(
        "Usage:\n  fcitx5-package --self-check [--audit-self-pe] [--trusted-keys security/trusted-keys.template.json]\n  fcitx5-package --validate-manifest MANIFEST\n  fcitx5-package --validate-keyring KEYRING\n  fcitx5-package --install ARCHIVE INSTALL_ROOT TRANSACTION_ID KEYRING\n  fcitx5-package --repair INSTALL_ROOT KEYRING\n  fcitx5-package --verify-repository INDEX SIGNATURE KEYRING [CHANNEL]\n  fcitx5-package --verify-repository-v2 INDEX SIG_JSON KEYRING [CHANNEL]\n  fcitx5-package --verify-manifest-v2 MANIFEST SIG_JSON KEYRING\n  fcitx5-package --list INSTALL_ROOT\n  fcitx5-package --state INSTALL_ROOT PACKAGE_ID STATE\n  fcitx5-package --mark-remove INSTALL_ROOT PACKAGE_ID\n  fcitx5-package --finalize-remove INSTALL_ROOT PACKAGE_ID"
    );
}

fn golden_manifest() -> String {
    format!(
        "{{\"format_version\":1,\"id\":\"fcitx5-rime\",\"version\":\"1.0.0\",\"type\":\"addon\",\
         \"architecture\":\"{}\",\"min_os\":\"6.1-sp1\",\"core_api\":\"1\",\
         \"addon_abi\":\"1\",\"dependencies\":[],\"license\":\"MIT\",\
         \"source_commit\":\"0123456789abcdef\",\"permissions\":[\"native-code\"],\
         \"files\":[{{\"path\":\"bin/addon.dll\",\"size\":12,\"sha256\":\"{}\"}}],\
         \"key_id\":\"official-2026-mldsa65\"}}",
        runtime_architecture(),
        "a".repeat(64)
    )
}

#[cfg(target_pointer_width = "64")]
fn runtime_architecture() -> &'static str {
    "x64"
}

#[cfg(not(target_pointer_width = "64"))]
fn runtime_architecture() -> &'static str {
    "x86"
}
