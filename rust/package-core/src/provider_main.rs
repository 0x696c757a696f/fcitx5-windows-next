#![forbid(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Duration;

use fcitx5_package_core::{make_plum_plan, run_plum_provider, ProviderTrust};

const DEFAULT_PROVIDER_TIMEOUT: Duration = Duration::from_secs(10 * 60);

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let mut allow_unverified = false;
    let mut self_test = false;
    let mut audit_self_pe = false;

    while let Some(arg) = args.next() {
        if arg == "--version" {
            println!("fcitx5-provider {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        if arg == "--allow-unverified" {
            allow_unverified = true;
            continue;
        }
        if arg == "--self-test" {
            self_test = true;
            continue;
        }
        if arg == "--audit-self-pe" {
            audit_self_pe = true;
            continue;
        }
        if arg == "--plum" {
            let provider_root = args.next().ok_or("--plum requires PROVIDER_ROOT")?;
            let rime_user_directory = args.next().ok_or("--plum requires RIME_USER_DIR")?;
            let download_cache_directory = args.next().ok_or("--plum requires CACHE_DIR")?;
            let package_spec = args.next().ok_or("--plum requires PACKAGE_SPEC")?;
            if args.next().is_some() {
                return Err("too many provider arguments".into());
            }
            let package_spec = package_spec
                .into_string()
                .map_err(|_| "PACKAGE_SPEC must be valid UTF-16/UTF-8 text")?;
            let plan = make_plum_plan(
                PathBuf::from(provider_root),
                PathBuf::from(rime_user_directory),
                PathBuf::from(download_cache_directory),
                &package_spec,
            )
            .map_err(|error| format!("{}: {error}", error.code()))?;
            println!("provider=plum");
            println!("source={}", plan.package_spec());
            println!("trust={}", plan.trust().as_str());
            println!("rime_user_dir_explicit=true");
            let exit_code = run_plum_provider(&plan, allow_unverified, DEFAULT_PROVIDER_TIMEOUT)
                .map_err(|error| format!("{}: {error}", error.code()))?;
            std::process::exit(exit_code);
        }
        if arg == "--help" || arg == "-h" {
            print_usage();
            return Ok(());
        }
        return Err(format!("unknown argument: {}", arg.to_string_lossy()).into());
    }

    if self_test {
        provider_self_test()?;
        if audit_self_pe {
            audit_pe_artifact(&std::env::current_exe()?)?;
        }
        println!("fcitx5-provider self-test ok");
        return Ok(());
    }

    print_usage();
    Err("no command selected".into())
}

fn provider_self_test() -> Result<(), Box<dyn Error>> {
    let root =
        std::env::temp_dir().join(format!("fcitx5-provider-rust-self-{}", std::process::id()));
    let provider = root.join("plum");
    let user = root.join("rime-user");
    let cache = root.join("cache");
    let cleanup = Cleanup(root.clone());
    std::fs::create_dir_all(&provider)?;
    std::fs::create_dir_all(&user)?;
    std::fs::create_dir_all(&cache)?;
    std::fs::write(provider.join("rime-install.bat"), b"@exit /b 0\r\n")?;
    let official = make_plum_plan(&provider, &user, &cache, ":preset")?;
    ensure(
        official.trust() == ProviderTrust::Official && official.rime_user_directory() == user,
        "official Plum plan mismatch",
    )?;
    let community = make_plum_plan(&provider, &user, &cache, "someone/schema")?;
    ensure(
        community.trust() == ProviderTrust::Unverified,
        "community provider trust mismatch",
    )?;
    ensure(
        make_plum_plan(&provider, Path::new(""), &cache, ":preset").is_err(),
        "empty Rime user path accepted",
    )?;
    ensure(
        make_plum_plan(&provider, &user, &cache, "repo & calc.exe").is_err(),
        "command metacharacter package spec accepted",
    )?;
    ensure(
        make_plum_plan(&provider, &user, &cache, "../evil").is_err(),
        "traversal package spec accepted",
    )?;
    drop(cleanup);
    Ok(())
}

struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn ensure(condition: bool, message: &'static str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn audit_pe_artifact(path: &Path) -> Result<(), Box<dyn Error>> {
    let image = std::fs::read(path)?;
    let pe = PeImage::parse(&image)?;
    ensure(
        matches!(pe.machine, 0x014c | 0x8664 | 0xaa64),
        "Rust provider artifact has an unsupported PE machine type",
    )?;
    ensure(
        pe.major_os_version <= 10,
        "Rust provider PE header requires an unsupported Windows major version",
    )?;
    for import in &pe.imports {
        ensure(
            !matches!(
                import.as_str(),
                "winhttp.dll" | "wininet.dll" | "urlmon.dll" | "webview2loader.dll"
            ),
            "Rust provider imports a network or web runtime library",
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
        "Usage:\n  fcitx5-provider --version\n  fcitx5-provider --self-test [--audit-self-pe]\n  fcitx5-provider [--allow-unverified] --plum PROVIDER_ROOT RIME_USER_DIR CACHE_DIR PACKAGE_SPEC"
    );
}
