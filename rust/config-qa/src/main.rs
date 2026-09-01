#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const MIN_SCREENSHOT_BYTES: usize = 4_096;

struct Args {
    config_exe: PathBuf,
    candidate_ui_exe: Option<PathBuf>,
    out_dir: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PngEvidence {
    width: u32,
    height: u32,
    bytes: usize,
    checksum: u64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args(std::env::args_os().skip(1))?;
    require_file(&args.config_exe, "Config executable")?;
    if let Some(candidate_ui) = &args.candidate_ui_exe {
        require_file(candidate_ui, "Candidate UI executable")?;
    }
    fs::create_dir_all(&args.out_dir)
        .map_err(|error| format!("create {}: {error}", args.out_dir.display()))?;

    let screenshot = args.out_dir.join("windui-settings.png");
    let output = Command::new(&args.config_exe)
        .args(["--screenshot", screenshot.to_string_lossy().as_ref()])
        .output()
        .map_err(|error| format!("launch {}: {error}", args.config_exe.display()))?;
    if !output.status.success() {
        return Err(format!(
            "WindUI screenshot failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|_| "Config screenshot report is not UTF-8".to_owned())?;
    validate_report(&stdout)?;
    let png = validate_png(&screenshot)?;
    if png.width < 900 || png.height < 620 {
        return Err(format!(
            "WindUI screenshot is smaller than the Settings minimum: {}x{}",
            png.width, png.height
        ));
    }

    let candidate = args.candidate_ui_exe.as_ref().map_or_else(
        || "not supplied".to_owned(),
        |path| path.display().to_string(),
    );
    let report = format!(
        "# Rust WindUI Config QA\n\n\
         - Config: `{}`\n\
         - Candidate UI boundary: `{candidate}`\n\
         - Screenshot: `{}`\n\
         - Dimensions: `{}x{}`\n\
         - Bytes: `{}`\n\
         - Checksum: `{:016x}`\n\
         - Legacy Win32 Config host: `deleted`\n\
         - Result: `PASS`\n",
        args.config_exe.display(),
        screenshot.display(),
        png.width,
        png.height,
        png.bytes,
        png.checksum
    );
    fs::write(args.out_dir.join("config-ui-qa.md"), report)
        .map_err(|error| format!("write Config QA report: {error}"))?;
    Ok(())
}

fn parse_args<I>(mut values: I) -> Result<Args, String>
where
    I: Iterator<Item = std::ffi::OsString>,
{
    let mut config_exe = None;
    let mut candidate_ui_exe = None;
    let mut out_dir = None;
    while let Some(argument) = values.next() {
        match argument.to_string_lossy().as_ref() {
            "--config-exe" => config_exe = values.next().map(PathBuf::from),
            "--candidate-ui-exe" => candidate_ui_exe = values.next().map(PathBuf::from),
            "--out" => out_dir = values.next().map(PathBuf::from),
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }
    Ok(Args {
        config_exe: config_exe.ok_or("--config-exe is required")?,
        candidate_ui_exe,
        out_dir: out_dir.ok_or("--out is required")?,
    })
}

fn require_file(path: &Path, label: &str) -> Result<(), String> {
    if path.is_file() {
        Ok(())
    } else {
        Err(format!("{label} is missing: {}", path.display()))
    }
}

fn validate_report(report: &str) -> Result<(), String> {
    for marker in [
        "\"kind\":\"rust-config-windui-settings-shell\"",
        "\"windui_app_default_interactive\":true",
        "\"legacy_win32_preview_host_deleted\":true",
        "\"result\":\"PASS\"",
    ] {
        if !report.contains(marker) {
            return Err(format!("WindUI screenshot report is missing {marker}"));
        }
    }
    for retired in ["rust-config-win32-qa-preview-host", "qa_navigation_ids"] {
        if report.contains(retired) {
            return Err(format!("retired Config UI marker returned: {retired}"));
        }
    }
    Ok(())
}

fn validate_png(path: &Path) -> Result<PngEvidence, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    if bytes.len() < MIN_SCREENSHOT_BYTES || bytes.get(..8) != Some(PNG_SIGNATURE) {
        return Err("WindUI screenshot is missing, truncated, or not PNG".to_owned());
    }
    if bytes.get(12..16) != Some(b"IHDR") {
        return Err("WindUI screenshot has no PNG IHDR".to_owned());
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().map_err(|_| "invalid PNG width")?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().map_err(|_| "invalid PNG height")?);
    let checksum = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    Ok(PngEvidence {
        width,
        height,
        bytes: bytes.len(),
        checksum,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windui_report_rejects_the_retired_host() {
        let retired = "{\"kind\":\"rust-config-win32-qa-preview-host\"}";
        assert!(validate_report(retired).is_err());
    }

    #[test]
    fn windui_report_requires_the_deleted_host_marker() {
        let report = "{\"kind\":\"rust-config-windui-settings-shell\",\"windui_app_default_interactive\":true,\"legacy_win32_preview_host_deleted\":true,\"result\":\"PASS\"}";
        assert_eq!(validate_report(report), Ok(()));
    }
}
