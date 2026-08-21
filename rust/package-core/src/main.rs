#![forbid(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::error::Error;
use std::path::PathBuf;

use fcitx5_package_core::{
    is_safe_relative_package_path, parse_manifest, parse_trusted_keys, upsert_installed_lock_entry,
    validate_manifest_compatibility, HexDigest32, PackageId, TrustAlgorithm,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("fcitx5-package-core: {error}");
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
                println!("fcitx5-package-core {}", env!("CARGO_PKG_VERSION"));
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
    let bytes = std::fs::read_to_string(path)?;
    let keys = parse_trusted_keys(&bytes)?;
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
        "Usage: fcitx5-package-core --self-check [--audit-self-pe] [--trusted-keys security/trusted-keys.template.json]"
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
