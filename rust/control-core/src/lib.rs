#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_void, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use std::ptr::null_mut;

type Dword = u32;
type Lstatus = i32;
type Hkey = *mut c_void;

const ERROR_SUCCESS: Lstatus = 0;
const ERROR_FILE_NOT_FOUND: Lstatus = 2;
const KEY_QUERY_VALUE: Dword = 0x0001;
const KEY_SET_VALUE: Dword = 0x0002;
const REG_SZ: Dword = 1;
const HKEY_CURRENT_USER: Hkey = 0x8000_0001_usize as Hkey;
const RUN_KEY: &[u16] = &[
    b'S' as u16,
    b'o' as u16,
    b'f' as u16,
    b't' as u16,
    b'w' as u16,
    b'a' as u16,
    b'r' as u16,
    b'e' as u16,
    b'\\' as u16,
    b'M' as u16,
    b'i' as u16,
    b'c' as u16,
    b'r' as u16,
    b'o' as u16,
    b's' as u16,
    b'o' as u16,
    b'f' as u16,
    b't' as u16,
    b'\\' as u16,
    b'W' as u16,
    b'i' as u16,
    b'n' as u16,
    b'd' as u16,
    b'o' as u16,
    b'w' as u16,
    b's' as u16,
    b'\\' as u16,
    b'C' as u16,
    b'u' as u16,
    b'r' as u16,
    b'r' as u16,
    b'e' as u16,
    b'n' as u16,
    b't' as u16,
    b'V' as u16,
    b'e' as u16,
    b'r' as u16,
    b's' as u16,
    b'i' as u16,
    b'o' as u16,
    b'n' as u16,
    b'\\' as u16,
    b'R' as u16,
    b'u' as u16,
    b'n' as u16,
    0,
];
const CONTROL_SCHEMA_JSON: &str = concat!(
    r#"{"format_version":1,"commands":["#,
    r#""status","restart_engine","shutdown","validate_config","apply_config","#,
    r#""reset_config","reset_presentation","get_startup","set_startup","#,
    r#""get_presentation","set_presentation","get_input_methods","set_input_method","#,
    r#""themes_list","themes_detail","addons_list","packages_list","packages_detail","#,
    r#""packages_refresh","packages_install","packages_update","packages_state","#,
    r#""packages_remove","packages_repair","get_tsf_guard","reset_tsf_guard"],"#,
    r#""sensitive_input":false,"package_network_owner":"fcitx5-downloader.exe"}"#
);
const CONTROL_TSF_GUARD_RESET_JSON: &str = r#"{"format_version":1,"tsf_guard":"enabled"}"#;
const CONTROL_LAUNCHER_ACTION_RESTART_ENGINE: u32 = 1;
const CONTROL_LAUNCHER_ACTION_SHUTDOWN: u32 = 2;
const CONTROL_CONFIG_ACTION_UNKNOWN: u32 = 0;
const CONTROL_CONFIG_ACTION_VALIDATE: u32 = 1;
const CONTROL_CONFIG_ACTION_APPLY: u32 = 2;
const CONTROL_CONFIG_ACTION_RESET_CONFIG: u32 = 3;
const CONTROL_CONFIG_ACTION_RESET_PRESENTATION: u32 = 4;
const CONTROL_CONFIG_ACTION_GET_PRESENTATION: u32 = 5;
const CONTROL_CONFIG_ACTION_SET_PRESENTATION: u32 = 6;
const CONTROL_PACKAGE_ACTION_UNKNOWN: u32 = 0;
const CONTROL_PACKAGE_ACTION_PACKAGES_LIST: u32 = 1;
const CONTROL_PACKAGE_ACTION_THEMES_LIST: u32 = 2;
const CONTROL_PACKAGE_ACTION_THEMES_DETAIL: u32 = 3;
const CONTROL_PACKAGE_ACTION_ADDONS_LIST: u32 = 4;
const CONTROL_PACKAGE_ACTION_PACKAGES_DETAIL: u32 = 5;
const CONTROL_PACKAGE_ACTION_PACKAGES_REFRESH: u32 = 6;
const CONTROL_PACKAGE_ACTION_PACKAGES_INSTALL: u32 = 7;
const CONTROL_PACKAGE_ACTION_PACKAGES_UPDATE: u32 = 8;
const CONTROL_PACKAGE_ACTION_PACKAGES_STATE: u32 = 9;
const CONTROL_PACKAGE_ACTION_PACKAGES_REMOVE: u32 = 10;
const CONTROL_PACKAGE_ACTION_PACKAGES_REPAIR: u32 = 11;
const LAUNCHER_COMMAND_START_DEMAND: u32 = 1;
const LAUNCHER_COMMAND_USER_STOP: u32 = 2;
const LAUNCHER_COMMAND_RESUME: u32 = 3;
const LAUNCHER_COMMAND_SHUTDOWN: u32 = 9;
const CONTROL_RESTART_ENGINE_COMMANDS: &[u32] = &[
    LAUNCHER_COMMAND_USER_STOP,
    LAUNCHER_COMMAND_RESUME,
    LAUNCHER_COMMAND_START_DEMAND,
];
const CONTROL_SHUTDOWN_COMMANDS: &[u32] = &[LAUNCHER_COMMAND_SHUTDOWN];

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Fcitx5ControlUtf16 {
    ptr: *const u16,
    len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Fcitx5ControlUtf8 {
    ptr: *const u8,
    len: usize,
}

#[repr(C)]
pub struct Fcitx5ControlPresentation {
    appearance_mode: Fcitx5ControlUtf8,
    theme: Fcitx5ControlUtf8,
    orientation: Fcitx5ControlUtf8,
    candidate_font: Fcitx5ControlUtf8,
    candidate_page_size: Fcitx5ControlUtf8,
    candidate_max_width_dip: Fcitx5ControlUtf8,
    candidate_scroll_cell_width_dip: Fcitx5ControlUtf8,
    candidate_font_size_dip: Fcitx5ControlUtf8,
    candidate_corner_radius_dip: Fcitx5ControlUtf8,
    candidate_opacity: Fcitx5ControlUtf8,
    candidate_preedit_mode: Fcitx5ControlUtf8,
    candidate_shadow: u8,
    scroll_mode: u8,
}

#[repr(C)]
pub struct Fcitx5ControlStatus {
    launcher_reachable: u8,
    launcher_state: i32,
    engine_state: i32,
    current_input_method_id: Fcitx5ControlUtf8,
    current_input_method_name: Fcitx5ControlUtf8,
    current_input_method_native_name: Fcitx5ControlUtf8,
    current_input_method_short_label: Fcitx5ControlUtf8,
    config_valid: u8,
    tsf_guard_disabled: u8,
    tsf_guard_reason: Fcitx5ControlUtf8,
    data_root: Fcitx5ControlUtf8,
    update_owner: Fcitx5ControlUtf8,
}

#[repr(C)]
pub struct Fcitx5ControlTsfGuard {
    disabled: u8,
    reason: Fcitx5ControlUtf8,
    marker_path: Fcitx5ControlUtf8,
}

#[repr(C)]
pub struct Fcitx5ControlPackageRepair {
    repository_sequence_state: Fcitx5ControlUtf8,
}

#[repr(C)]
pub struct Fcitx5ControlAddonDescriptor {
    id: Fcitx5ControlUtf8,
    name: Fcitx5ControlUtf8,
    category: Fcitx5ControlUtf8,
    library: Fcitx5ControlUtf8,
    addon_type: Fcitx5ControlUtf8,
    version: Fcitx5ControlUtf8,
    configurable: u8,
    on_demand: u8,
    library_present: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Fcitx5ControlThemeRecord {
    id: Fcitx5ControlUtf8,
    source: Fcitx5ControlUtf8,
    name: Fcitx5ControlUtf8,
    version: Fcitx5ControlUtf8,
    license: Fcitx5ControlUtf8,
    description: Fcitx5ControlUtf8,
}

#[repr(C)]
pub struct Fcitx5ControlThemeDetail {
    theme: Fcitx5ControlThemeRecord,
    has_light_branch: u8,
    has_dark_branch: u8,
}

#[repr(C)]
pub struct Fcitx5ControlPackageSummary {
    id: Fcitx5ControlUtf8,
    title: Fcitx5ControlUtf8,
    summary: Fcitx5ControlUtf8,
    package_type: Fcitx5ControlUtf8,
    available_version: Fcitx5ControlUtf8,
    installed_version: Fcitx5ControlUtf8,
    state: Fcitx5ControlUtf8,
    update_available: u8,
}

#[repr(C)]
pub struct Fcitx5ControlPackagesList {
    repository_available: u8,
    repository_error: Fcitx5ControlUtf8,
    packages: *const Fcitx5ControlPackageSummary,
    package_count: usize,
}

#[repr(C)]
pub struct Fcitx5ControlPackageDetail {
    repository_available: u8,
    repository_error: Fcitx5ControlUtf8,
    id: Fcitx5ControlUtf8,
    title: Fcitx5ControlUtf8,
    summary: Fcitx5ControlUtf8,
    package_type: Fcitx5ControlUtf8,
    available_version: Fcitx5ControlUtf8,
    installed_version: Fcitx5ControlUtf8,
    state: Fcitx5ControlUtf8,
    bundled: u8,
    update_available: u8,
    manifest_sha256: Fcitx5ControlUtf8,
    source_commit: Fcitx5ControlUtf8,
    dependencies_json: Fcitx5ControlUtf8,
    permissions_json: Fcitx5ControlUtf8,
    config_surface_json: Fcitx5ControlUtf8,
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(
        h_key: Hkey,
        sub_key: *const u16,
        options: Dword,
        sam_desired: Dword,
        result: *mut Hkey,
    ) -> Lstatus;
    fn RegCreateKeyExW(
        h_key: Hkey,
        sub_key: *const u16,
        reserved: Dword,
        class: *mut u16,
        options: Dword,
        sam_desired: Dword,
        security_attributes: *mut c_void,
        result: *mut Hkey,
        disposition: *mut Dword,
    ) -> Lstatus;
    fn RegQueryValueExW(
        h_key: Hkey,
        value_name: *const u16,
        reserved: *mut Dword,
        value_type: *mut Dword,
        data: *mut u8,
        data_size: *mut Dword,
    ) -> Lstatus;
    fn RegSetValueExW(
        h_key: Hkey,
        value_name: *const u16,
        reserved: Dword,
        value_type: Dword,
        data: *const u8,
        data_size: Dword,
    ) -> Lstatus;
    fn RegDeleteValueW(h_key: Hkey, value_name: *const u16) -> Lstatus;
    fn RegCloseKey(h_key: Hkey) -> Lstatus;
}

struct RegistryKey(Hkey);

impl RegistryKey {
    fn get(&self) -> Hkey {
        self.0
    }
}

impl Drop for RegistryKey {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

fn string_from_utf16(value: Fcitx5ControlUtf16) -> Option<OsString> {
    if value.ptr.is_null() {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(value.ptr, value.len) };
    Some(OsString::from_wide(slice))
}

fn wide_z(value: &std::ffi::OsStr) -> Vec<u16> {
    let mut wide: Vec<u16> = value.encode_wide().collect();
    wide.push(0);
    wide
}

fn quote(value: &std::ffi::OsStr) -> OsString {
    let wide: Vec<u16> = value.encode_wide().collect();
    let mut result = Vec::with_capacity(wide.len() + 2);
    result.push(b'"' as u16);
    let mut backslashes = 0_usize;
    for character in wide {
        if character == b'\\' as u16 {
            backslashes += 1;
        } else if character == b'"' as u16 {
            result.extend(std::iter::repeat_n(b'\\' as u16, backslashes + 1));
            backslashes = 0;
            result.push(character);
        } else {
            result.extend(std::iter::repeat_n(b'\\' as u16, backslashes));
            backslashes = 0;
            result.push(character);
        }
    }
    result.extend(std::iter::repeat_n(b'\\' as u16, backslashes * 2));
    result.push(b'"' as u16);
    OsString::from_wide(&result)
}

fn json_string(value: &[u8]) -> Option<Vec<u8>> {
    let mut result = Vec::with_capacity(value.len() + 2);
    result.push(b'"');
    for byte in value {
        match *byte {
            b'\\' => result.extend_from_slice(b"\\\\"),
            b'"' => result.extend_from_slice(br#"\""#),
            b'\x08' => result.extend_from_slice(br#"\b"#),
            b'\x0c' => result.extend_from_slice(br#"\f"#),
            b'\n' => result.extend_from_slice(br#"\n"#),
            b'\r' => result.extend_from_slice(br#"\r"#),
            b'\t' => result.extend_from_slice(br#"\t"#),
            0x00..=0x1f => return None,
            other => result.push(other),
        }
    }
    result.push(b'"');
    Some(result)
}

fn utf8_slice(value: Fcitx5ControlUtf8) -> Option<&'static [u8]> {
    if value.ptr.is_null() {
        return None;
    }
    Some(unsafe { std::slice::from_raw_parts(value.ptr, value.len) })
}

fn boxed_utf8_result(value: Vec<u8>, out_ptr: *mut *mut u8, out_len: *mut usize) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return 1;
    }
    let mut bytes = value.into_boxed_slice();
    let ptr = bytes.as_mut_ptr();
    let len = bytes.len();
    std::mem::forget(bytes);
    unsafe {
        *out_ptr = ptr;
        *out_len = len;
    }
    0
}

fn push_json_string_field(output: &mut Vec<u8>, name: &[u8], value: &[u8]) -> Option<()> {
    output.extend_from_slice(name);
    output.push(b':');
    output.extend_from_slice(&json_string(value)?);
    Some(())
}

fn push_json_optional_reachable_string_field(
    output: &mut Vec<u8>,
    name: &[u8],
    reachable: bool,
    value: Fcitx5ControlUtf8,
) -> Option<()> {
    output.extend_from_slice(name);
    output.push(b':');
    if reachable {
        output.extend_from_slice(&json_string(utf8_slice(value)?)?);
    } else {
        output.extend_from_slice(b"null");
    }
    Some(())
}

fn presentation_json(presentation: &Fcitx5ControlPresentation) -> Option<Vec<u8>> {
    let fields = [
        (
            b"\"appearance_mode\"".as_slice(),
            utf8_slice(presentation.appearance_mode)?,
        ),
        (b"\"theme\"".as_slice(), utf8_slice(presentation.theme)?),
        (
            b"\"orientation\"".as_slice(),
            utf8_slice(presentation.orientation)?,
        ),
        (
            b"\"candidate_font\"".as_slice(),
            utf8_slice(presentation.candidate_font)?,
        ),
        (
            b"\"candidate_page_size\"".as_slice(),
            utf8_slice(presentation.candidate_page_size)?,
        ),
        (
            b"\"candidate_max_width_dip\"".as_slice(),
            utf8_slice(presentation.candidate_max_width_dip)?,
        ),
        (
            b"\"candidate_scroll_cell_width_dip\"".as_slice(),
            utf8_slice(presentation.candidate_scroll_cell_width_dip)?,
        ),
        (
            b"\"candidate_font_size_dip\"".as_slice(),
            utf8_slice(presentation.candidate_font_size_dip)?,
        ),
        (
            b"\"candidate_corner_radius_dip\"".as_slice(),
            utf8_slice(presentation.candidate_corner_radius_dip)?,
        ),
        (
            b"\"candidate_opacity\"".as_slice(),
            utf8_slice(presentation.candidate_opacity)?,
        ),
        (
            b"\"candidate_preedit_mode\"".as_slice(),
            utf8_slice(presentation.candidate_preedit_mode)?,
        ),
    ];
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1"#);
    for (name, value) in fields {
        output.push(b',');
        push_json_string_field(&mut output, name, value)?;
    }
    output.extend_from_slice(b",\"candidate_shadow\":");
    output.extend_from_slice(if presentation.candidate_shadow != 0 {
        b"true"
    } else {
        b"false"
    });
    output.extend_from_slice(b",\"scroll_mode\":");
    output.extend_from_slice(if presentation.scroll_mode != 0 {
        b"true"
    } else {
        b"false"
    });
    output.push(b'}');
    Some(output)
}

fn status_json(status: &Fcitx5ControlStatus) -> Option<Vec<u8>> {
    let reachable = status.launcher_reachable != 0;
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"launcher_reachable":"#);
    output.extend_from_slice(if reachable { b"true" } else { b"false" });
    output.extend_from_slice(b",\"launcher_state\":");
    if reachable {
        output.extend_from_slice(status.launcher_state.to_string().as_bytes());
    } else {
        output.extend_from_slice(b"null");
    }
    output.extend_from_slice(b",\"engine_state\":");
    if reachable {
        output.extend_from_slice(status.engine_state.to_string().as_bytes());
    } else {
        output.extend_from_slice(b"null");
    }
    output.push(b',');
    push_json_optional_reachable_string_field(
        &mut output,
        b"\"current_input_method_id\"",
        reachable,
        status.current_input_method_id,
    )?;
    output.push(b',');
    push_json_optional_reachable_string_field(
        &mut output,
        b"\"current_input_method_name\"",
        reachable,
        status.current_input_method_name,
    )?;
    output.push(b',');
    push_json_optional_reachable_string_field(
        &mut output,
        b"\"current_input_method_native_name\"",
        reachable,
        status.current_input_method_native_name,
    )?;
    output.push(b',');
    push_json_optional_reachable_string_field(
        &mut output,
        b"\"current_input_method_short_label\"",
        reachable,
        status.current_input_method_short_label,
    )?;
    output.extend_from_slice(b",\"config_valid\":");
    output.extend_from_slice(if status.config_valid != 0 {
        b"true"
    } else {
        b"false"
    });
    output.extend_from_slice(b",\"tsf_guard_disabled\":");
    output.extend_from_slice(if status.tsf_guard_disabled != 0 {
        b"true"
    } else {
        b"false"
    });
    output.push(b',');
    push_json_string_field(
        &mut output,
        b"\"tsf_guard_reason\"",
        utf8_slice(status.tsf_guard_reason)?,
    )?;
    output.push(b',');
    push_json_string_field(&mut output, b"\"data_root\"", utf8_slice(status.data_root)?)?;
    output.push(b',');
    push_json_string_field(
        &mut output,
        b"\"update_owner\"",
        utf8_slice(status.update_owner)?,
    )?;
    output.push(b'}');
    Some(output)
}

fn tsf_guard_json(status: &Fcitx5ControlTsfGuard) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"disabled":"#);
    output.extend_from_slice(if status.disabled != 0 {
        b"true"
    } else {
        b"false"
    });
    output.push(b',');
    push_json_string_field(&mut output, b"\"reason\"", utf8_slice(status.reason)?)?;
    output.push(b',');
    push_json_string_field(
        &mut output,
        b"\"marker_path\"",
        utf8_slice(status.marker_path)?,
    )?;
    output.push(b'}');
    Some(output)
}

fn startup_json(enabled: bool) -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"enabled":"#);
    output.extend_from_slice(if enabled { b"true" } else { b"false" });
    output.push(b'}');
    output
}

fn launcher_action_sequence(action: u32) -> Option<&'static [u32]> {
    match action {
        CONTROL_LAUNCHER_ACTION_RESTART_ENGINE => Some(CONTROL_RESTART_ENGINE_COMMANDS),
        CONTROL_LAUNCHER_ACTION_SHUTDOWN => Some(CONTROL_SHUTDOWN_COMMANDS),
        _ => None,
    }
}

fn package_repair_json(repair: &Fcitx5ControlPackageRepair) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"repair":"verified","#);
    push_json_string_field(
        &mut output,
        b"\"repository_sequence_state\"",
        utf8_slice(repair.repository_sequence_state)?,
    )?;
    output.push(b'}');
    Some(output)
}

fn addons_json(addons: &[Fcitx5ControlAddonDescriptor]) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.extend_from_slice(
        br#"{"format_version":1,"surface":"descriptor-inventory","typed_config":"not_available","addons":["#,
    );
    for (index, addon) in addons.iter().enumerate() {
        if index != 0 {
            output.push(b',');
        }
        output.push(b'{');
        push_json_string_field(&mut output, b"\"id\"", utf8_slice(addon.id)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"name\"", utf8_slice(addon.name)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"category\"", utf8_slice(addon.category)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"library\"", utf8_slice(addon.library)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"type\"", utf8_slice(addon.addon_type)?)?;
        output.extend_from_slice(b",\"version\":");
        let version = utf8_slice(addon.version)?;
        if version.is_empty() {
            output.extend_from_slice(b"null");
        } else {
            output.extend_from_slice(&json_string(version)?);
        }
        output.extend_from_slice(b",\"configurable\":");
        output.extend_from_slice(if addon.configurable != 0 {
            b"true"
        } else {
            b"false"
        });
        output.extend_from_slice(b",\"on_demand\":");
        output.extend_from_slice(if addon.on_demand != 0 {
            b"true"
        } else {
            b"false"
        });
        output.extend_from_slice(b",\"library_present\":");
        output.extend_from_slice(if addon.library_present != 0 {
            b"true"
        } else {
            b"false"
        });
        output.push(b'}');
    }
    output.extend_from_slice(b"]}");
    Some(output)
}

fn push_theme_record(output: &mut Vec<u8>, theme: &Fcitx5ControlThemeRecord) -> Option<()> {
    output.push(b'{');
    push_json_string_field(output, b"\"id\"", utf8_slice(theme.id)?)?;
    output.push(b',');
    push_json_string_field(output, b"\"source\"", utf8_slice(theme.source)?)?;
    output.push(b',');
    push_json_string_field(output, b"\"name\"", utf8_slice(theme.name)?)?;
    output.push(b',');
    push_json_string_field(output, b"\"version\"", utf8_slice(theme.version)?)?;
    output.push(b',');
    push_json_string_field(output, b"\"license\"", utf8_slice(theme.license)?)?;
    output.push(b',');
    push_json_string_field(output, b"\"description\"", utf8_slice(theme.description)?)?;
    output.push(b'}');
    Some(())
}

fn themes_json(themes: &[Fcitx5ControlThemeRecord]) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"themes":["#);
    for (index, theme) in themes.iter().enumerate() {
        if index != 0 {
            output.push(b',');
        }
        push_theme_record(&mut output, theme)?;
    }
    output.extend_from_slice(b"]}");
    Some(output)
}

fn theme_editable_fields_json(output: &mut Vec<u8>) {
    const FIELDS: &[&[u8]] = &[
        b"appearance.mode",
        b"candidate.orientation",
        b"candidate.page_size",
        b"candidate.scroll_mode",
        b"candidate.max_width_dip",
        b"candidate.scroll_cell_width_dip",
        b"candidate.opacity",
        b"candidate.preedit_mode",
        b"candidate.geometry.padding_x_dip",
        b"candidate.geometry.padding_y_dip",
        b"candidate.geometry.item_padding_x_dip",
        b"candidate.geometry.item_padding_y_dip",
        b"candidate.geometry.row_gap_dip",
        b"candidate.geometry.column_gap_dip",
        b"candidate.geometry.border_width_dip",
        b"candidate.geometry.corner_radius_dip",
        b"candidate.geometry.shadow",
        b"candidate.label.visible",
        b"candidate.label.style",
        b"candidate.label.font_scale",
        b"candidate.label.gap_dip",
        b"fonts.candidate.families",
        b"fonts.candidate.size_dip",
        b"fonts.candidate.weight",
        b"fonts.annotation.scale",
        b"candidate.colors.background",
        b"candidate.colors.border",
        b"candidate.colors.candidate_text",
        b"candidate.colors.label_text",
        b"candidate.colors.comment_text",
        b"candidate.colors.selected_background",
        b"candidate.colors.selected_candidate_text",
        b"candidate.colors.selected_label_text",
        b"candidate.colors.selected_comment_text",
    ];
    output.push(b'[');
    for (index, field) in FIELDS.iter().enumerate() {
        if index != 0 {
            output.push(b',');
        }
        output.extend_from_slice(&json_string(field).expect("static editable field is JSON-safe"));
    }
    output.push(b']');
}

fn theme_detail_json(detail: &Fcitx5ControlThemeDetail) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"theme":"#);
    push_theme_record(&mut output, &detail.theme)?;
    output.extend_from_slice(b",\"has_light_branch\":");
    output.extend_from_slice(if detail.has_light_branch != 0 {
        b"true"
    } else {
        b"false"
    });
    output.extend_from_slice(b",\"has_dark_branch\":");
    output.extend_from_slice(if detail.has_dark_branch != 0 {
        b"true"
    } else {
        b"false"
    });
    output.extend_from_slice(b",\"editable_fields\":");
    theme_editable_fields_json(&mut output);
    output.extend_from_slice(
        br#","security":{"script_allowed":false,"network_allowed":false,"unknown_fields":"reject","path_scope":"theme-directory"}}"#,
    );
    Some(output)
}

fn packages_list_json(list: &Fcitx5ControlPackagesList) -> Option<Vec<u8>> {
    if list.packages.is_null() && list.package_count != 0 {
        return None;
    }
    let packages = if list.package_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(list.packages, list.package_count) }
    };
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"repository_available":"#);
    output.extend_from_slice(if list.repository_available != 0 {
        b"true"
    } else {
        b"false"
    });
    output.extend_from_slice(b",\"repository_error\":");
    let repository_error = utf8_slice(list.repository_error)?;
    if repository_error.is_empty() {
        output.extend_from_slice(b"null");
    } else {
        output.extend_from_slice(&json_string(repository_error)?);
    }
    output.extend_from_slice(b",\"packages\":[");
    for (index, package) in packages.iter().enumerate() {
        if index != 0 {
            output.push(b',');
        }
        output.push(b'{');
        push_json_string_field(&mut output, b"\"id\"", utf8_slice(package.id)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"title\"", utf8_slice(package.title)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"summary\"", utf8_slice(package.summary)?)?;
        output.push(b',');
        push_json_string_field(&mut output, b"\"type\"", utf8_slice(package.package_type)?)?;
        output.extend_from_slice(b",\"available_version\":");
        let available = utf8_slice(package.available_version)?;
        if available.is_empty() {
            output.extend_from_slice(b"null");
        } else {
            output.extend_from_slice(&json_string(available)?);
        }
        output.extend_from_slice(b",\"installed_version\":");
        let installed = utf8_slice(package.installed_version)?;
        if installed.is_empty() {
            output.extend_from_slice(b"null");
        } else {
            output.extend_from_slice(&json_string(installed)?);
        }
        output.extend_from_slice(b",\"state\":");
        let state = utf8_slice(package.state)?;
        if state.is_empty() {
            output.extend_from_slice(b"null");
        } else {
            output.extend_from_slice(&json_string(state)?);
        }
        output.extend_from_slice(b",\"update_available\":");
        output.extend_from_slice(if package.update_available != 0 {
            b"true"
        } else {
            b"false"
        });
        output.push(b'}');
    }
    output.extend_from_slice(b"]}");
    Some(output)
}

fn push_json_nullable_string(
    output: &mut Vec<u8>,
    name: &[u8],
    value: Fcitx5ControlUtf8,
) -> Option<()> {
    output.extend_from_slice(name);
    output.push(b':');
    let value = utf8_slice(value)?;
    if value.is_empty() {
        output.extend_from_slice(b"null");
    } else {
        output.extend_from_slice(&json_string(value)?);
    }
    Some(())
}

fn push_json_raw_field(output: &mut Vec<u8>, name: &[u8], value: Fcitx5ControlUtf8) -> Option<()> {
    output.extend_from_slice(name);
    output.push(b':');
    let value = utf8_slice(value)?;
    if value.is_empty() {
        return None;
    }
    output.extend_from_slice(value);
    Some(())
}

fn package_detail_json(detail: &Fcitx5ControlPackageDetail) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    output.extend_from_slice(br#"{"format_version":1,"repository_available":"#);
    output.extend_from_slice(if detail.repository_available != 0 {
        b"true"
    } else {
        b"false"
    });
    output.push(b',');
    push_json_nullable_string(
        &mut output,
        b"\"repository_error\"",
        detail.repository_error,
    )?;
    output.push(b',');
    push_json_string_field(&mut output, b"\"id\"", utf8_slice(detail.id)?)?;
    output.push(b',');
    push_json_string_field(&mut output, b"\"title\"", utf8_slice(detail.title)?)?;
    output.push(b',');
    push_json_string_field(&mut output, b"\"summary\"", utf8_slice(detail.summary)?)?;
    output.push(b',');
    push_json_string_field(&mut output, b"\"type\"", utf8_slice(detail.package_type)?)?;
    output.push(b',');
    push_json_nullable_string(
        &mut output,
        b"\"available_version\"",
        detail.available_version,
    )?;
    output.push(b',');
    push_json_nullable_string(
        &mut output,
        b"\"installed_version\"",
        detail.installed_version,
    )?;
    output.push(b',');
    push_json_nullable_string(&mut output, b"\"state\"", detail.state)?;
    output.extend_from_slice(b",\"bundled\":");
    output.extend_from_slice(if detail.bundled != 0 {
        b"true"
    } else {
        b"false"
    });
    output.extend_from_slice(b",\"update_available\":");
    output.extend_from_slice(if detail.update_available != 0 {
        b"true"
    } else {
        b"false"
    });
    output.push(b',');
    push_json_nullable_string(&mut output, b"\"manifest_sha256\"", detail.manifest_sha256)?;
    output.push(b',');
    push_json_nullable_string(&mut output, b"\"source_commit\"", detail.source_commit)?;
    output.push(b',');
    push_json_raw_field(&mut output, b"\"dependencies\"", detail.dependencies_json)?;
    output.push(b',');
    push_json_raw_field(&mut output, b"\"permissions\"", detail.permissions_json)?;
    output.push(b',');
    push_json_raw_field(
        &mut output,
        b"\"config_surface\"",
        detail.config_surface_json,
    )?;
    output.push(b'}');
    Some(output)
}

fn config_action(command: &[u16], argc: usize) -> u32 {
    match argc {
        1 if ascii_utf16_eq(command, b"--reset-config") => CONTROL_CONFIG_ACTION_RESET_CONFIG,
        1 if ascii_utf16_eq(command, b"--reset-presentation") => {
            CONTROL_CONFIG_ACTION_RESET_PRESENTATION
        }
        1 if ascii_utf16_eq(command, b"--get-presentation") => {
            CONTROL_CONFIG_ACTION_GET_PRESENTATION
        }
        2 if ascii_utf16_eq(command, b"--validate-config") => CONTROL_CONFIG_ACTION_VALIDATE,
        2 if ascii_utf16_eq(command, b"--apply-config") => CONTROL_CONFIG_ACTION_APPLY,
        7 | 9 | 12 | 14 if ascii_utf16_eq(command, b"--set-presentation") => {
            CONTROL_CONFIG_ACTION_SET_PRESENTATION
        }
        _ => CONTROL_CONFIG_ACTION_UNKNOWN,
    }
}

fn ascii_utf16_eq(value: &[u16], ascii: &[u8]) -> bool {
    value.len() == ascii.len()
        && value
            .iter()
            .zip(ascii)
            .all(|(left, right)| *left == u16::from(*right))
}

fn package_action(command: &[u16], argc: usize, state: Option<&[u16]>) -> u32 {
    match argc {
        1 if ascii_utf16_eq(command, b"--packages-list") => CONTROL_PACKAGE_ACTION_PACKAGES_LIST,
        1 if ascii_utf16_eq(command, b"--themes-list") => CONTROL_PACKAGE_ACTION_THEMES_LIST,
        1 if ascii_utf16_eq(command, b"--addons-list") => CONTROL_PACKAGE_ACTION_ADDONS_LIST,
        1 if ascii_utf16_eq(command, b"--packages-repair") => {
            CONTROL_PACKAGE_ACTION_PACKAGES_REPAIR
        }
        1 | 2 if ascii_utf16_eq(command, b"--packages-refresh") => {
            CONTROL_PACKAGE_ACTION_PACKAGES_REFRESH
        }
        2 if ascii_utf16_eq(command, b"--themes-detail") => CONTROL_PACKAGE_ACTION_THEMES_DETAIL,
        2 if ascii_utf16_eq(command, b"--packages-detail") => {
            CONTROL_PACKAGE_ACTION_PACKAGES_DETAIL
        }
        2 if ascii_utf16_eq(command, b"--packages-install") => {
            CONTROL_PACKAGE_ACTION_PACKAGES_INSTALL
        }
        2 if ascii_utf16_eq(command, b"--packages-update") => {
            CONTROL_PACKAGE_ACTION_PACKAGES_UPDATE
        }
        2 if ascii_utf16_eq(command, b"--packages-remove") => {
            CONTROL_PACKAGE_ACTION_PACKAGES_REMOVE
        }
        3 if ascii_utf16_eq(command, b"--packages-state")
            && state.is_some_and(|value| {
                ascii_utf16_eq(value, b"enabled") || ascii_utf16_eq(value, b"disabled")
            }) =>
        {
            CONTROL_PACKAGE_ACTION_PACKAGES_STATE
        }
        _ => CONTROL_PACKAGE_ACTION_UNKNOWN,
    }
}

fn startup_command(executable_directory: OsString) -> Vec<u16> {
    let launcher = PathBuf::from(executable_directory).join("fcitx5-launcher.exe");
    let mut command = quote(launcher.as_os_str());
    command.push(" --background");
    wide_z(&command)
}

fn query_startup(executable_directory: OsString, registry_value: OsString) -> Result<bool, ()> {
    let expected = startup_command(executable_directory);
    let value_name = wide_z(&registry_value);
    let mut raw_key = null_mut();
    let open_result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY.as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut raw_key,
        )
    };
    if open_result != ERROR_SUCCESS {
        return Ok(false);
    }
    let key = RegistryKey(raw_key);
    let mut value_type = 0_u32;
    let mut bytes = 0_u32;
    let size_result = unsafe {
        RegQueryValueExW(
            key.get(),
            value_name.as_ptr(),
            null_mut(),
            &mut value_type,
            null_mut(),
            &mut bytes,
        )
    };
    if size_result == ERROR_FILE_NOT_FOUND {
        return Ok(false);
    }
    if size_result != ERROR_SUCCESS || value_type != REG_SZ || !(2..=64 * 1024).contains(&bytes) {
        return Err(());
    }
    let mut value = vec![0_u16; (bytes as usize).div_ceil(2)];
    let read_result = unsafe {
        RegQueryValueExW(
            key.get(),
            value_name.as_ptr(),
            null_mut(),
            &mut value_type,
            value.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    while value.last().copied() == Some(0) {
        value.pop();
    }
    let mut expected_trimmed = expected;
    while expected_trimmed.last().copied() == Some(0) {
        expected_trimmed.pop();
    }
    if read_result != ERROR_SUCCESS {
        return Err(());
    }
    Ok(value == expected_trimmed)
}

fn set_startup(
    executable_directory: OsString,
    registry_value: OsString,
    enabled: bool,
) -> Result<(), ()> {
    let value_name = wide_z(&registry_value);
    let mut raw_key = null_mut();
    let create_result = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY.as_ptr(),
            0,
            null_mut(),
            0,
            KEY_SET_VALUE,
            null_mut(),
            &mut raw_key,
            null_mut(),
        )
    };
    if create_result != ERROR_SUCCESS {
        return Err(());
    }
    let key = RegistryKey(raw_key);
    let result = if enabled {
        let command = startup_command(executable_directory);
        unsafe {
            RegSetValueExW(
                key.get(),
                value_name.as_ptr(),
                0,
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * 2) as Dword,
            )
        }
    } else {
        let delete_result = unsafe { RegDeleteValueW(key.get(), value_name.as_ptr()) };
        if delete_result == ERROR_FILE_NOT_FOUND {
            ERROR_SUCCESS
        } else {
            delete_result
        }
    };
    if result == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(())
    }
}

/// # Safety
///
/// UTF-16 slices must remain valid for the duration of the call. `out_enabled`
/// must point to writable storage. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_startup_query_utf16(
    executable_directory: Fcitx5ControlUtf16,
    registry_value: Fcitx5ControlUtf16,
    out_enabled: *mut u8,
) -> i32 {
    if out_enabled.is_null() {
        return 1;
    }
    let Some(executable_directory) = string_from_utf16(executable_directory) else {
        return 1;
    };
    let Some(registry_value) = string_from_utf16(registry_value) else {
        return 1;
    };
    match query_startup(executable_directory, registry_value) {
        Ok(enabled) => {
            unsafe {
                *out_enabled = u8::from(enabled);
            }
            0
        }
        Err(()) => 1,
    }
}

/// # Safety
///
/// UTF-16 slices must remain valid for the duration of the call. No pointer is
/// retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_startup_set_utf16(
    executable_directory: Fcitx5ControlUtf16,
    registry_value: Fcitx5ControlUtf16,
    enabled: u8,
) -> i32 {
    let Some(executable_directory) = string_from_utf16(executable_directory) else {
        return 1;
    };
    let Some(registry_value) = string_from_utf16(registry_value) else {
        return 1;
    };
    match set_startup(executable_directory, registry_value, enabled != 0) {
        Ok(()) => 0,
        Err(()) => 1,
    }
}

/// # Safety
///
/// `out_ptr` and `out_len` must point to writable storage. The returned pointer
/// is process-static UTF-8 data and must not be freed by the caller.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_schema_json_utf8(
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return 1;
    }
    unsafe {
        *out_ptr = CONTROL_SCHEMA_JSON.as_ptr();
        *out_len = CONTROL_SCHEMA_JSON.len();
    }
    0
}

/// # Safety
///
/// `id` must remain valid for the duration of the call. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_input_method_id_valid_utf16(id: Fcitx5ControlUtf16) -> u8 {
    if id.ptr.is_null() {
        return 0;
    }
    let value = unsafe { std::slice::from_raw_parts(id.ptr, id.len) };
    if value.is_empty() || value.len() > 64 {
        return 0;
    }
    u8::from(value.iter().all(|character| {
        (*character >= b'a' as u16 && *character <= b'z' as u16)
            || (*character >= b'0' as u16 && *character <= b'9' as u16)
            || *character == b'-' as u16
            || *character == b'_' as u16
    }))
}

/// # Safety
///
/// `value` must remain valid for the duration of the call. `out_ptr` and
/// `out_len` must point to writable storage. Any returned buffer must be freed
/// with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_json_string_utf8(
    value: Fcitx5ControlUtf8,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if value.ptr.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let bytes = unsafe { std::slice::from_raw_parts(value.ptr, value.len) };
    match json_string(bytes) {
        Some(escaped) => boxed_utf8_result(escaped, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// All UTF-8 slices inside `presentation` must remain valid for the duration of
/// the call. `out_ptr` and `out_len` must point to writable storage. Any
/// returned buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_presentation_json_utf8(
    presentation: *const Fcitx5ControlPresentation,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if presentation.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let presentation = unsafe { &*presentation };
    match presentation_json(presentation) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// All UTF-8 slices inside `status` must remain valid for the duration of the
/// call. `out_ptr` and `out_len` must point to writable storage. Any returned
/// buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_status_json_utf8(
    status: *const Fcitx5ControlStatus,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if status.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let status = unsafe { &*status };
    match status_json(status) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// All UTF-8 slices inside `status` must remain valid for the duration of the
/// call. `out_ptr` and `out_len` must point to writable storage. Any returned
/// buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_tsf_guard_json_utf8(
    status: *const Fcitx5ControlTsfGuard,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if status.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let status = unsafe { &*status };
    match tsf_guard_json(status) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `out_ptr` and `out_len` must point to writable storage. The returned pointer
/// is process-static UTF-8 data and must not be freed by the caller.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_tsf_guard_reset_json_utf8(
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return 1;
    }
    unsafe {
        *out_ptr = CONTROL_TSF_GUARD_RESET_JSON.as_ptr();
        *out_len = CONTROL_TSF_GUARD_RESET_JSON.len();
    }
    0
}

/// # Safety
///
/// `out_ptr` and `out_len` must point to writable storage. Any returned buffer
/// must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_startup_json_utf8(
    enabled: u8,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    boxed_utf8_result(startup_json(enabled != 0), out_ptr, out_len)
}

/// # Safety
///
/// `out_ptr` and `out_len` must point to writable storage. The returned pointer
/// is process-static command data and must not be freed by the caller.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_launcher_action_sequence(
    action: u32,
    out_ptr: *mut *const u32,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return 1;
    }
    let Some(commands) = launcher_action_sequence(action) else {
        return 1;
    };
    unsafe {
        *out_ptr = commands.as_ptr();
        *out_len = commands.len();
    }
    0
}

/// # Safety
///
/// All UTF-8 slices inside `repair` must remain valid for the duration of the
/// call. `out_ptr` and `out_len` must point to writable storage. Any returned
/// buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_repair_json_utf8(
    repair: *const Fcitx5ControlPackageRepair,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if repair.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let repair = unsafe { &*repair };
    match package_repair_json(repair) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `command` must remain valid for the duration of the call. No pointer is
/// retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_config_action_utf16(
    command: Fcitx5ControlUtf16,
    argc: usize,
) -> u32 {
    if command.ptr.is_null() {
        return CONTROL_CONFIG_ACTION_UNKNOWN;
    }
    let command = unsafe { std::slice::from_raw_parts(command.ptr, command.len) };
    config_action(command, argc)
}

/// # Safety
///
/// `command` and `state` must remain valid for the duration of the call when
/// their pointers are non-null. No pointer is retained.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_action_utf16(
    command: Fcitx5ControlUtf16,
    argc: usize,
    state: Fcitx5ControlUtf16,
) -> u32 {
    if command.ptr.is_null() {
        return CONTROL_PACKAGE_ACTION_UNKNOWN;
    }
    let command = unsafe { std::slice::from_raw_parts(command.ptr, command.len) };
    let state = if state.ptr.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(state.ptr, state.len) })
    };
    package_action(command, argc, state)
}

/// # Safety
///
/// `addons` must either be null with `addon_count == 0` or point to
/// `addon_count` valid descriptors. All UTF-8 slices inside descriptors must
/// remain valid for the duration of the call. `out_ptr` and `out_len` must
/// point to writable storage. Any returned buffer must be freed with
/// `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_addons_json_utf8(
    addons: *const Fcitx5ControlAddonDescriptor,
    addon_count: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if addons.is_null() && addon_count != 0 {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let addons = if addon_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(addons, addon_count) }
    };
    match addons_json(addons) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `themes` must either be null with `theme_count == 0` or point to
/// `theme_count` valid records. All UTF-8 slices inside records must remain
/// valid for the duration of the call. `out_ptr` and `out_len` must point to
/// writable storage. Any returned buffer must be freed with
/// `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_themes_json_utf8(
    themes: *const Fcitx5ControlThemeRecord,
    theme_count: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if themes.is_null() && theme_count != 0 {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let themes = if theme_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(themes, theme_count) }
    };
    match themes_json(themes) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// All UTF-8 slices inside `detail` must remain valid for the duration of the
/// call. `out_ptr` and `out_len` must point to writable storage. Any returned
/// buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_theme_detail_json_utf8(
    detail: *const Fcitx5ControlThemeDetail,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if detail.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let detail = unsafe { &*detail };
    match theme_detail_json(detail) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// All UTF-8 slices inside `list` and its rows must remain valid for the
/// duration of the call. `out_ptr` and `out_len` must point to writable storage.
/// Any returned buffer must be freed with `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_packages_list_json_utf8(
    list: *const Fcitx5ControlPackagesList,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if list.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let list = unsafe { &*list };
    match packages_list_json(list) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// All UTF-8 slices inside `detail` must remain valid for the duration of the
/// call. Raw JSON fields must contain valid JSON fragments produced by the
/// existing package/config-surface serializers. `out_ptr` and `out_len` must
/// point to writable storage. Any returned buffer must be freed with
/// `fcitx5_control_utf8_free`.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_package_detail_json_utf8(
    detail: *const Fcitx5ControlPackageDetail,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if detail.is_null() {
        return boxed_utf8_result(Vec::new(), out_ptr, out_len);
    }
    let detail = unsafe { &*detail };
    match package_detail_json(detail) {
        Some(json) => boxed_utf8_result(json, out_ptr, out_len),
        None => boxed_utf8_result(Vec::new(), out_ptr, out_len),
    }
}

/// # Safety
///
/// `ptr` and `len` must be the exact buffer returned by a Control core UTF-8
/// allocation function, or `ptr` must be null.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_control_utf8_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wide(value: &str) -> Vec<u16> {
        OsString::from(value).encode_wide().collect()
    }

    #[test]
    fn startup_command_quotes_launcher_path() {
        let command = startup_command(OsString::from(r"C:\Program Files\Fcitx5\bin"));
        let mut trimmed = command;
        while trimmed.last().copied() == Some(0) {
            trimmed.pop();
        }
        assert_eq!(
            trimmed,
            wide(r#""C:\Program Files\Fcitx5\bin\fcitx5-launcher.exe" --background"#)
        );
    }

    #[test]
    fn startup_json_preserves_control_contract() {
        assert_eq!(
            startup_json(true).as_slice(),
            br#"{"format_version":1,"enabled":true}"#
        );
        assert_eq!(
            startup_json(false).as_slice(),
            br#"{"format_version":1,"enabled":false}"#
        );
    }

    #[test]
    fn launcher_action_sequences_are_typed_control_commands() {
        assert_eq!(
            launcher_action_sequence(CONTROL_LAUNCHER_ACTION_RESTART_ENGINE),
            Some(
                &[
                    LAUNCHER_COMMAND_USER_STOP,
                    LAUNCHER_COMMAND_RESUME,
                    LAUNCHER_COMMAND_START_DEMAND
                ][..]
            )
        );
        assert_eq!(
            launcher_action_sequence(CONTROL_LAUNCHER_ACTION_SHUTDOWN),
            Some(&[LAUNCHER_COMMAND_SHUTDOWN][..])
        );
        assert_eq!(launcher_action_sequence(99), None);
    }

    #[test]
    fn package_repair_json_preserves_control_contract() {
        let state = b"repaired";
        let repair = Fcitx5ControlPackageRepair {
            repository_sequence_state: Fcitx5ControlUtf8 {
                ptr: state.as_ptr(),
                len: state.len(),
            },
        };
        let json = package_repair_json(&repair).expect("package repair should format");
        assert_eq!(
            json.as_slice(),
            br#"{"format_version":1,"repair":"verified","repository_sequence_state":"repaired"}"#
        );
    }

    #[test]
    fn config_actions_are_typed_control_commands() {
        let validate = wide("--validate-config");
        let apply = wide("--apply-config");
        let reset = wide("--reset-config");
        let reset_presentation = wide("--reset-presentation");
        let get_presentation = wide("--get-presentation");
        let set_presentation = wide("--set-presentation");
        assert_eq!(config_action(&validate, 2), CONTROL_CONFIG_ACTION_VALIDATE);
        assert_eq!(config_action(&apply, 2), CONTROL_CONFIG_ACTION_APPLY);
        assert_eq!(config_action(&reset, 1), CONTROL_CONFIG_ACTION_RESET_CONFIG);
        assert_eq!(
            config_action(&reset_presentation, 1),
            CONTROL_CONFIG_ACTION_RESET_PRESENTATION
        );
        assert_eq!(
            config_action(&get_presentation, 1),
            CONTROL_CONFIG_ACTION_GET_PRESENTATION
        );
        assert_eq!(
            config_action(&set_presentation, 7),
            CONTROL_CONFIG_ACTION_SET_PRESENTATION
        );
        assert_eq!(
            config_action(&set_presentation, 9),
            CONTROL_CONFIG_ACTION_SET_PRESENTATION
        );
        assert_eq!(
            config_action(&set_presentation, 12),
            CONTROL_CONFIG_ACTION_SET_PRESENTATION
        );
        assert_eq!(
            config_action(&set_presentation, 14),
            CONTROL_CONFIG_ACTION_SET_PRESENTATION
        );
        assert_eq!(
            config_action(&set_presentation, 8),
            CONTROL_CONFIG_ACTION_UNKNOWN
        );
        assert_eq!(config_action(&validate, 1), CONTROL_CONFIG_ACTION_UNKNOWN);
    }

    #[test]
    fn package_actions_are_typed_control_commands() {
        let packages_list = wide("--packages-list");
        let themes_detail = wide("--themes-detail");
        let addons_list = wide("--addons-list");
        let packages_refresh = wide("--packages-refresh");
        let packages_install = wide("--packages-install");
        let packages_update = wide("--packages-update");
        let packages_state = wide("--packages-state");
        let packages_remove = wide("--packages-remove");
        let packages_repair = wide("--packages-repair");
        let enabled = wide("enabled");
        let disabled = wide("disabled");
        let broken = wide("broken");
        assert_eq!(
            package_action(&packages_list, 1, None),
            CONTROL_PACKAGE_ACTION_PACKAGES_LIST
        );
        assert_eq!(
            package_action(&themes_detail, 2, None),
            CONTROL_PACKAGE_ACTION_THEMES_DETAIL
        );
        assert_eq!(
            package_action(&addons_list, 1, None),
            CONTROL_PACKAGE_ACTION_ADDONS_LIST
        );
        assert_eq!(
            package_action(&packages_refresh, 1, None),
            CONTROL_PACKAGE_ACTION_PACKAGES_REFRESH
        );
        assert_eq!(
            package_action(&packages_refresh, 2, None),
            CONTROL_PACKAGE_ACTION_PACKAGES_REFRESH
        );
        assert_eq!(
            package_action(&packages_install, 2, None),
            CONTROL_PACKAGE_ACTION_PACKAGES_INSTALL
        );
        assert_eq!(
            package_action(&packages_update, 2, None),
            CONTROL_PACKAGE_ACTION_PACKAGES_UPDATE
        );
        assert_eq!(
            package_action(&packages_state, 3, Some(&enabled)),
            CONTROL_PACKAGE_ACTION_PACKAGES_STATE
        );
        assert_eq!(
            package_action(&packages_state, 3, Some(&disabled)),
            CONTROL_PACKAGE_ACTION_PACKAGES_STATE
        );
        assert_eq!(
            package_action(&packages_state, 3, Some(&broken)),
            CONTROL_PACKAGE_ACTION_UNKNOWN
        );
        assert_eq!(
            package_action(&packages_remove, 2, None),
            CONTROL_PACKAGE_ACTION_PACKAGES_REMOVE
        );
        assert_eq!(
            package_action(&packages_repair, 1, None),
            CONTROL_PACKAGE_ACTION_PACKAGES_REPAIR
        );
        assert_eq!(
            package_action(&packages_list, 2, None),
            CONTROL_PACKAGE_ACTION_UNKNOWN
        );
    }

    #[test]
    fn addons_json_preserves_control_contract() {
        let id = b"pinyin";
        let name = "拼音".as_bytes();
        let category = b"InputMethod";
        let library = b"pinyin";
        let addon_type = b"SharedLibrary";
        let version = b"5.1";
        let empty_version = b"";
        let addons = [
            Fcitx5ControlAddonDescriptor {
                id: Fcitx5ControlUtf8 {
                    ptr: id.as_ptr(),
                    len: id.len(),
                },
                name: Fcitx5ControlUtf8 {
                    ptr: name.as_ptr(),
                    len: name.len(),
                },
                category: Fcitx5ControlUtf8 {
                    ptr: category.as_ptr(),
                    len: category.len(),
                },
                library: Fcitx5ControlUtf8 {
                    ptr: library.as_ptr(),
                    len: library.len(),
                },
                addon_type: Fcitx5ControlUtf8 {
                    ptr: addon_type.as_ptr(),
                    len: addon_type.len(),
                },
                version: Fcitx5ControlUtf8 {
                    ptr: version.as_ptr(),
                    len: version.len(),
                },
                configurable: 1,
                on_demand: 0,
                library_present: 1,
            },
            Fcitx5ControlAddonDescriptor {
                id: Fcitx5ControlUtf8 {
                    ptr: b"test".as_ptr(),
                    len: 4,
                },
                name: Fcitx5ControlUtf8 {
                    ptr: b"Test".as_ptr(),
                    len: 4,
                },
                category: Fcitx5ControlUtf8 {
                    ptr: b"Module".as_ptr(),
                    len: 6,
                },
                library: Fcitx5ControlUtf8 {
                    ptr: b"".as_ptr(),
                    len: 0,
                },
                addon_type: Fcitx5ControlUtf8 {
                    ptr: b"Static".as_ptr(),
                    len: 6,
                },
                version: Fcitx5ControlUtf8 {
                    ptr: empty_version.as_ptr(),
                    len: empty_version.len(),
                },
                configurable: 0,
                on_demand: 1,
                library_present: 0,
            },
        ];
        let json = addons_json(&addons).expect("addons should format");
        let text = String::from_utf8(json).expect("addons JSON should be UTF-8");
        assert!(text.starts_with(
            r#"{"format_version":1,"surface":"descriptor-inventory","typed_config":"not_available","addons":["#
        ));
        assert!(text.contains(r#""name":"拼音""#));
        assert!(text.contains(r#""version":"5.1""#));
        assert!(text.contains(r#""version":null"#));
        assert!(text.contains(r#""on_demand":true"#));
    }

    #[test]
    fn empty_addons_json_preserves_control_contract() {
        assert_eq!(
            addons_json(&[]).as_deref(),
            Some(
                &br#"{"format_version":1,"surface":"descriptor-inventory","typed_config":"not_available","addons":[]}"#[..]
            )
        );
    }

    fn test_theme_record() -> Fcitx5ControlThemeRecord {
        Fcitx5ControlThemeRecord {
            id: Fcitx5ControlUtf8 {
                ptr: b"builtin:default".as_ptr(),
                len: 15,
            },
            source: Fcitx5ControlUtf8 {
                ptr: b"builtin".as_ptr(),
                len: 7,
            },
            name: Fcitx5ControlUtf8 {
                ptr: "默认".as_bytes().as_ptr(),
                len: "默认".len(),
            },
            version: Fcitx5ControlUtf8 {
                ptr: b"1".as_ptr(),
                len: 1,
            },
            license: Fcitx5ControlUtf8 {
                ptr: b"MIT".as_ptr(),
                len: 3,
            },
            description: Fcitx5ControlUtf8 {
                ptr: b"default theme".as_ptr(),
                len: 13,
            },
        }
    }

    #[test]
    fn themes_json_preserves_control_contract() {
        let theme = test_theme_record();
        let json = themes_json(&[theme]).expect("themes should format");
        let text = String::from_utf8(json).expect("themes JSON should be UTF-8");
        assert_eq!(
            text,
            r#"{"format_version":1,"themes":[{"id":"builtin:default","source":"builtin","name":"默认","version":"1","license":"MIT","description":"default theme"}]}"#
        );
        assert_eq!(
            themes_json(&[]).as_deref(),
            Some(&br#"{"format_version":1,"themes":[]}"#[..])
        );
    }

    #[test]
    fn theme_detail_json_preserves_control_contract() {
        let detail = Fcitx5ControlThemeDetail {
            theme: test_theme_record(),
            has_light_branch: 1,
            has_dark_branch: 0,
        };
        let json = theme_detail_json(&detail).expect("theme detail should format");
        let text = String::from_utf8(json).expect("theme detail JSON should be UTF-8");
        assert!(text.starts_with(
            r#"{"format_version":1,"theme":{"id":"builtin:default","source":"builtin""#
        ));
        assert!(text.contains(r#""has_light_branch":true"#));
        assert!(text.contains(r#""has_dark_branch":false"#));
        assert!(text.contains(r#""appearance.mode""#));
        assert!(text.contains(
            r#""security":{"script_allowed":false,"network_allowed":false,"unknown_fields":"reject","path_scope":"theme-directory"}"#
        ));
    }

    #[test]
    fn packages_list_json_preserves_control_contract() {
        let id = b"fcitx5-rime";
        let title = b"Rime";
        let summary = b"Rime input method";
        let package_type = b"addon";
        let available = b"1.2.3";
        let installed = b"1.0.0";
        let state = b"enabled";
        let package = Fcitx5ControlPackageSummary {
            id: Fcitx5ControlUtf8 {
                ptr: id.as_ptr(),
                len: id.len(),
            },
            title: Fcitx5ControlUtf8 {
                ptr: title.as_ptr(),
                len: title.len(),
            },
            summary: Fcitx5ControlUtf8 {
                ptr: summary.as_ptr(),
                len: summary.len(),
            },
            package_type: Fcitx5ControlUtf8 {
                ptr: package_type.as_ptr(),
                len: package_type.len(),
            },
            available_version: Fcitx5ControlUtf8 {
                ptr: available.as_ptr(),
                len: available.len(),
            },
            installed_version: Fcitx5ControlUtf8 {
                ptr: installed.as_ptr(),
                len: installed.len(),
            },
            state: Fcitx5ControlUtf8 {
                ptr: state.as_ptr(),
                len: state.len(),
            },
            update_available: 1,
        };
        let list = Fcitx5ControlPackagesList {
            repository_available: 1,
            repository_error: Fcitx5ControlUtf8 {
                ptr: b"".as_ptr(),
                len: 0,
            },
            packages: &package,
            package_count: 1,
        };
        let json = packages_list_json(&list).expect("packages list should format");
        let text = String::from_utf8(json).expect("packages list JSON should be UTF-8");
        assert_eq!(
            text,
            r#"{"format_version":1,"repository_available":true,"repository_error":null,"packages":[{"id":"fcitx5-rime","title":"Rime","summary":"Rime input method","type":"addon","available_version":"1.2.3","installed_version":"1.0.0","state":"enabled","update_available":true}]}"#
        );
    }

    #[test]
    fn packages_list_json_preserves_null_fields() {
        let package = Fcitx5ControlPackageSummary {
            id: Fcitx5ControlUtf8 {
                ptr: b"orphan".as_ptr(),
                len: 6,
            },
            title: Fcitx5ControlUtf8 {
                ptr: b"orphan".as_ptr(),
                len: 6,
            },
            summary: Fcitx5ControlUtf8 {
                ptr: b"".as_ptr(),
                len: 0,
            },
            package_type: Fcitx5ControlUtf8 {
                ptr: b"unknown".as_ptr(),
                len: 7,
            },
            available_version: Fcitx5ControlUtf8 {
                ptr: b"".as_ptr(),
                len: 0,
            },
            installed_version: Fcitx5ControlUtf8 {
                ptr: b"1".as_ptr(),
                len: 1,
            },
            state: Fcitx5ControlUtf8 {
                ptr: b"".as_ptr(),
                len: 0,
            },
            update_available: 0,
        };
        let error = b"missing_key";
        let list = Fcitx5ControlPackagesList {
            repository_available: 0,
            repository_error: Fcitx5ControlUtf8 {
                ptr: error.as_ptr(),
                len: error.len(),
            },
            packages: &package,
            package_count: 1,
        };
        let json = packages_list_json(&list).expect("packages list should format");
        let text = String::from_utf8(json).expect("packages list JSON should be UTF-8");
        assert!(text.contains(r#""repository_available":false"#));
        assert!(text.contains(r#""repository_error":"missing_key""#));
        assert!(text.contains(r#""available_version":null"#));
        assert!(text.contains(r#""state":null"#));
    }

    #[test]
    fn package_detail_json_preserves_control_contract() {
        let deps = br#"[{"id":"dep","version":"1"}]"#;
        let permissions = br#"["input-data"]"#;
        let surfaces =
            br#"[{"kind":"fcitx-addon","owner":"fcitx5-rime","schema":"generic-fcitx-config-v1"}]"#;
        let detail = Fcitx5ControlPackageDetail {
            repository_available: 1,
            repository_error: Fcitx5ControlUtf8 {
                ptr: b"".as_ptr(),
                len: 0,
            },
            id: Fcitx5ControlUtf8 {
                ptr: b"fcitx5-rime".as_ptr(),
                len: 11,
            },
            title: Fcitx5ControlUtf8 {
                ptr: b"Rime".as_ptr(),
                len: 4,
            },
            summary: Fcitx5ControlUtf8 {
                ptr: b"Rime input method".as_ptr(),
                len: 17,
            },
            package_type: Fcitx5ControlUtf8 {
                ptr: b"addon".as_ptr(),
                len: 5,
            },
            available_version: Fcitx5ControlUtf8 {
                ptr: b"1.2.3".as_ptr(),
                len: 5,
            },
            installed_version: Fcitx5ControlUtf8 {
                ptr: b"1.0.0".as_ptr(),
                len: 5,
            },
            state: Fcitx5ControlUtf8 {
                ptr: b"enabled".as_ptr(),
                len: 7,
            },
            bundled: 0,
            update_available: 1,
            manifest_sha256: Fcitx5ControlUtf8 {
                ptr: b"abc".as_ptr(),
                len: 3,
            },
            source_commit: Fcitx5ControlUtf8 {
                ptr: b"def".as_ptr(),
                len: 3,
            },
            dependencies_json: Fcitx5ControlUtf8 {
                ptr: deps.as_ptr(),
                len: deps.len(),
            },
            permissions_json: Fcitx5ControlUtf8 {
                ptr: permissions.as_ptr(),
                len: permissions.len(),
            },
            config_surface_json: Fcitx5ControlUtf8 {
                ptr: surfaces.as_ptr(),
                len: surfaces.len(),
            },
        };
        let json = package_detail_json(&detail).expect("package detail should format");
        let text = String::from_utf8(json).expect("package detail JSON should be UTF-8");
        assert!(text.starts_with(r#"{"format_version":1,"repository_available":true"#));
        assert!(text.contains(r#""repository_error":null"#));
        assert!(text.contains(r#""update_available":true"#));
        assert!(text.contains(r#""manifest_sha256":"abc""#));
        assert!(text.contains(r#""dependencies":[{"id":"dep","version":"1"}]"#));
        assert!(text.contains(r#""permissions":["input-data"]"#));
        assert!(text.contains(r#""config_surface":[{"kind":"fcitx-addon""#));
    }

    #[test]
    fn package_detail_json_preserves_nullable_fields() {
        let empty = Fcitx5ControlUtf8 {
            ptr: b"".as_ptr(),
            len: 0,
        };
        let array = Fcitx5ControlUtf8 {
            ptr: b"[]".as_ptr(),
            len: 2,
        };
        let detail = Fcitx5ControlPackageDetail {
            repository_available: 0,
            repository_error: Fcitx5ControlUtf8 {
                ptr: b"missing_key".as_ptr(),
                len: 11,
            },
            id: Fcitx5ControlUtf8 {
                ptr: b"bundled".as_ptr(),
                len: 7,
            },
            title: Fcitx5ControlUtf8 {
                ptr: b"bundled".as_ptr(),
                len: 7,
            },
            summary: empty,
            package_type: Fcitx5ControlUtf8 {
                ptr: b"addon".as_ptr(),
                len: 5,
            },
            available_version: empty,
            installed_version: empty,
            state: empty,
            bundled: 1,
            update_available: 0,
            manifest_sha256: empty,
            source_commit: empty,
            dependencies_json: array,
            permissions_json: array,
            config_surface_json: array,
        };
        let json = package_detail_json(&detail).expect("package detail should format");
        let text = String::from_utf8(json).expect("package detail JSON should be UTF-8");
        assert!(text.contains(r#""repository_error":"missing_key""#));
        assert!(text.contains(r#""available_version":null"#));
        assert!(text.contains(r#""manifest_sha256":null"#));
        assert!(text.contains(r#""bundled":true"#));
    }

    #[test]
    fn schema_documents_typed_control_commands() {
        assert!(CONTROL_SCHEMA_JSON.contains(r#""format_version":1"#));
        assert!(CONTROL_SCHEMA_JSON.contains(r#""set_presentation""#));
        assert!(CONTROL_SCHEMA_JSON.contains(r#""packages_repair""#));
        assert!(CONTROL_SCHEMA_JSON.contains(r#""package_network_owner":"fcitx5-downloader.exe""#));
        assert!(!CONTROL_SCHEMA_JSON.contains("sensitive_input\":true"));
    }

    #[test]
    fn input_method_ids_are_bounded_lowercase_ascii_tokens() {
        let valid = wide("rime-luna_pinyin");
        let upper = wide("Rime");
        let empty: Vec<u16> = Vec::new();
        let long = wide(&"a".repeat(65));
        unsafe {
            assert_eq!(
                fcitx5_control_input_method_id_valid_utf16(Fcitx5ControlUtf16 {
                    ptr: valid.as_ptr(),
                    len: valid.len()
                }),
                1
            );
            assert_eq!(
                fcitx5_control_input_method_id_valid_utf16(Fcitx5ControlUtf16 {
                    ptr: upper.as_ptr(),
                    len: upper.len()
                }),
                0
            );
            assert_eq!(
                fcitx5_control_input_method_id_valid_utf16(Fcitx5ControlUtf16 {
                    ptr: empty.as_ptr(),
                    len: empty.len()
                }),
                0
            );
            assert_eq!(
                fcitx5_control_input_method_id_valid_utf16(Fcitx5ControlUtf16 {
                    ptr: long.as_ptr(),
                    len: long.len()
                }),
                0
            );
        }
    }

    #[test]
    fn json_string_matches_control_output_contract() {
        assert_eq!(json_string(b"plain").as_deref(), Some(&b"\"plain\""[..]));
        assert_eq!(
            json_string(b"quote\"slash\\\n\t").as_deref(),
            Some(&br#""quote\"slash\\\n\t""#[..])
        );
        assert_eq!(
            json_string("企鹅".as_bytes()).as_deref(),
            Some(&b"\"\xe4\xbc\x81\xe9\xb9\x85\""[..])
        );
        assert_eq!(json_string(&[0x01]), None);
    }

    #[test]
    fn presentation_json_preserves_existing_control_contract() {
        let mode = b"dark";
        let theme = b"builtin:default";
        let orientation = b"automatic";
        let font = "微软雅黑".as_bytes();
        let page = b"5";
        let max_width = b"860";
        let scroll_width = b"96";
        let font_size = b"18";
        let corner = b"12";
        let opacity = b"1.000000";
        let preedit = b"inline";
        let presentation = Fcitx5ControlPresentation {
            appearance_mode: Fcitx5ControlUtf8 {
                ptr: mode.as_ptr(),
                len: mode.len(),
            },
            theme: Fcitx5ControlUtf8 {
                ptr: theme.as_ptr(),
                len: theme.len(),
            },
            orientation: Fcitx5ControlUtf8 {
                ptr: orientation.as_ptr(),
                len: orientation.len(),
            },
            candidate_font: Fcitx5ControlUtf8 {
                ptr: font.as_ptr(),
                len: font.len(),
            },
            candidate_page_size: Fcitx5ControlUtf8 {
                ptr: page.as_ptr(),
                len: page.len(),
            },
            candidate_max_width_dip: Fcitx5ControlUtf8 {
                ptr: max_width.as_ptr(),
                len: max_width.len(),
            },
            candidate_scroll_cell_width_dip: Fcitx5ControlUtf8 {
                ptr: scroll_width.as_ptr(),
                len: scroll_width.len(),
            },
            candidate_font_size_dip: Fcitx5ControlUtf8 {
                ptr: font_size.as_ptr(),
                len: font_size.len(),
            },
            candidate_corner_radius_dip: Fcitx5ControlUtf8 {
                ptr: corner.as_ptr(),
                len: corner.len(),
            },
            candidate_opacity: Fcitx5ControlUtf8 {
                ptr: opacity.as_ptr(),
                len: opacity.len(),
            },
            candidate_preedit_mode: Fcitx5ControlUtf8 {
                ptr: preedit.as_ptr(),
                len: preedit.len(),
            },
            candidate_shadow: 1,
            scroll_mode: 0,
        };
        let json = presentation_json(&presentation).expect("presentation should format");
        let text = String::from_utf8(json).expect("presentation JSON should be UTF-8");
        assert!(text.starts_with(r#"{"format_version":1"#));
        assert!(text.contains(r#""candidate_font":"微软雅黑""#));
        assert!(text.contains(r#""candidate_page_size":"5""#));
        assert!(text.contains(r#""candidate_shadow":true"#));
        assert!(text.contains(r#""scroll_mode":false"#));
    }

    #[test]
    fn status_json_preserves_reachable_control_contract() {
        let id = b"rime";
        let name = b"Rime";
        let native = "中州韵".as_bytes();
        let label = "中".as_bytes();
        let reason = b"manual";
        let data_root = b"C:/Users/Test/Fcitx5";
        let owner = b"builtin";
        let status = Fcitx5ControlStatus {
            launcher_reachable: 1,
            launcher_state: 2,
            engine_state: 3,
            current_input_method_id: Fcitx5ControlUtf8 {
                ptr: id.as_ptr(),
                len: id.len(),
            },
            current_input_method_name: Fcitx5ControlUtf8 {
                ptr: name.as_ptr(),
                len: name.len(),
            },
            current_input_method_native_name: Fcitx5ControlUtf8 {
                ptr: native.as_ptr(),
                len: native.len(),
            },
            current_input_method_short_label: Fcitx5ControlUtf8 {
                ptr: label.as_ptr(),
                len: label.len(),
            },
            config_valid: 1,
            tsf_guard_disabled: 1,
            tsf_guard_reason: Fcitx5ControlUtf8 {
                ptr: reason.as_ptr(),
                len: reason.len(),
            },
            data_root: Fcitx5ControlUtf8 {
                ptr: data_root.as_ptr(),
                len: data_root.len(),
            },
            update_owner: Fcitx5ControlUtf8 {
                ptr: owner.as_ptr(),
                len: owner.len(),
            },
        };
        let json = status_json(&status).expect("status should format");
        let text = String::from_utf8(json).expect("status JSON should be UTF-8");
        assert_eq!(
            text,
            r#"{"format_version":1,"launcher_reachable":true,"launcher_state":2,"engine_state":3,"current_input_method_id":"rime","current_input_method_name":"Rime","current_input_method_native_name":"中州韵","current_input_method_short_label":"中","config_valid":true,"tsf_guard_disabled":true,"tsf_guard_reason":"manual","data_root":"C:/Users/Test/Fcitx5","update_owner":"builtin"}"#
        );
    }

    #[test]
    fn status_json_preserves_unreachable_null_fields() {
        let reason = b"";
        let data_root = br#"C:/Users/Test/Fcitx\"#;
        let owner = b"none";
        let empty = Fcitx5ControlUtf8 {
            ptr: b"".as_ptr(),
            len: 0,
        };
        let status = Fcitx5ControlStatus {
            launcher_reachable: 0,
            launcher_state: 9,
            engine_state: 9,
            current_input_method_id: empty,
            current_input_method_name: empty,
            current_input_method_native_name: empty,
            current_input_method_short_label: empty,
            config_valid: 0,
            tsf_guard_disabled: 0,
            tsf_guard_reason: Fcitx5ControlUtf8 {
                ptr: reason.as_ptr(),
                len: reason.len(),
            },
            data_root: Fcitx5ControlUtf8 {
                ptr: data_root.as_ptr(),
                len: data_root.len(),
            },
            update_owner: Fcitx5ControlUtf8 {
                ptr: owner.as_ptr(),
                len: owner.len(),
            },
        };
        let json = status_json(&status).expect("status should format");
        let text = String::from_utf8(json).expect("status JSON should be UTF-8");
        assert!(text.contains(r#""launcher_reachable":false"#));
        assert!(text.contains(r#""launcher_state":null"#));
        assert!(text.contains(r#""current_input_method_id":null"#));
        assert!(text.contains(r#""config_valid":false"#));
        assert!(text.contains(r#""data_root":"C:/Users/Test/Fcitx\\","#));
    }

    #[test]
    fn tsf_guard_json_preserves_control_contract() {
        let reason = b"manual \"disable\"";
        let marker = br#"C:/Users/Test/Fcitx5/tsf-guard.txt"#;
        let status = Fcitx5ControlTsfGuard {
            disabled: 1,
            reason: Fcitx5ControlUtf8 {
                ptr: reason.as_ptr(),
                len: reason.len(),
            },
            marker_path: Fcitx5ControlUtf8 {
                ptr: marker.as_ptr(),
                len: marker.len(),
            },
        };
        let json = tsf_guard_json(&status).expect("guard status should format");
        let text = String::from_utf8(json).expect("guard JSON should be UTF-8");
        assert_eq!(
            text,
            r#"{"format_version":1,"disabled":true,"reason":"manual \"disable\"","marker_path":"C:/Users/Test/Fcitx5/tsf-guard.txt"}"#
        );
        assert_eq!(
            CONTROL_TSF_GUARD_RESET_JSON,
            r#"{"format_version":1,"tsf_guard":"enabled"}"#
        );
    }
}
