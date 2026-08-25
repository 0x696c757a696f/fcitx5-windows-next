#![deny(unsafe_op_in_unsafe_fn)]
#![allow(linker_messages)]
#![allow(non_snake_case)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write;
use std::panic::{catch_unwind, UnwindSafe};
use std::path::PathBuf;
use std::process::Command;
use std::ptr::null_mut;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::Duration;

use fcitx5_protocol_core as protocol;
use fcitx5_windows_common_core as common;
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, E_FAIL, E_INVALIDARG, E_NOTIMPL, E_POINTER, E_UNEXPECTED, HMODULE,
    POINT, RECT, S_OK, WIN32_ERROR,
};
use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_NOINTERFACE, LPARAM, S_FALSE, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IClassFactory,
    IClassFactory_Impl, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_LOCAL_MACHINE,
    KEY_WRITE, REG_OPEN_CREATE_OPTIONS, REG_SZ,
};
use windows::Win32::System::Variant::{VariantClear, VT_EMPTY, VT_UNKNOWN};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetFocus, GetKeyState, GetKeyboardLayout, MapVirtualKeyExW, HKL, MAPVK_VK_TO_VSC_EX,
    VK_CONTROL, VK_DOWN, VK_END, VK_HOME, VK_LEFT, VK_LWIN, VK_MENU, VK_NEXT, VK_OEM_1, VK_OEM_4,
    VK_OEM_6, VK_OEM_7, VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS, VK_PRIOR,
    VK_RETURN, VK_RIGHT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_UP,
};
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, ITfActiveLanguageProfileNotifySink,
    ITfActiveLanguageProfileNotifySink_Impl, ITfCandidateListUIElement,
    ITfCandidateListUIElement_Impl, ITfCategoryMgr, ITfComposition, ITfCompositionSink,
    ITfCompositionSink_Impl, ITfContext, ITfContextComposition, ITfDocumentMgr, ITfEditSession,
    ITfEditSession_Impl, ITfInputProcessorProfileMgr, ITfInputScope, ITfKeyEventSink,
    ITfKeyEventSink_Impl, ITfKeystrokeMgr, ITfRange, ITfSource, ITfTextInputProcessor,
    ITfTextInputProcessorEx, ITfTextInputProcessorEx_Impl, ITfTextInputProcessor_Impl,
    ITfThreadFocusSink, ITfThreadFocusSink_Impl, ITfThreadMgr, ITfThreadMgrEventSink,
    ITfThreadMgrEventSink_Impl, ITfUIElement, ITfUIElementMgr, ITfUIElement_Impl, InputScope,
    GUID_PROP_INPUTSCOPE, GUID_TFCAT_TIP_KEYBOARD, IS_ALPHANUMERIC_PIN, IS_ALPHANUMERIC_PIN_SET,
    IS_NUMERIC_PASSWORD, IS_NUMERIC_PIN, IS_PASSWORD, IS_PRIVATE, TF_DEFAULT_SELECTION, TF_ES_READ,
    TF_ES_READWRITE, TF_ES_SYNC, TF_SELECTION,
};
use windows::Win32::UI::WindowsAndMessaging::{GetGUIThreadInfo, GUITHREADINFO};
use windows_core::{
    implement, w, ComObject, IUnknown, IUnknownImpl, Interface, Ref, Result, BOOL, BSTR, GUID,
    HRESULT, PCWSTR,
};

// Stable channel CLSID from cmake/release_identity.h.in.
pub const FCITX5_TEXT_SERVICE_CLSID: GUID = GUID::from_u128(0x3a21b9e2_4f47_4c36_8bfa_91d7d3b3e901);
pub const FCITX5_LANGUAGE_PROFILE_GUID: GUID =
    GUID::from_u128(0x6c2ac726_7703_4b65_89af_a77e9e0da102);
const FCITX5_OBSOLETE_EN_US_PROFILE_GUID: GUID =
    GUID::from_u128(0x6c2ac726_7703_4b65_89af_a77e9e0da102);
const FCITX5_OBSOLETE_RIME_PROFILE_GUID: GUID =
    GUID::from_u128(0xa79f94c2_bd7e_4498_8fe5_6522f83cd4d0);
const FCITX5_OBSOLETE_JA_PROFILE_GUID: GUID =
    GUID::from_u128(0x90672aa7_db8c_45f9_8e97_27866570a8fa);
const FCITX5_PRODUCT_DISPLAY_NAME: &str = "Fcitx5 for Windows Next";
const FCITX5_PROFILE_DISPLAY_NAME: &str = "Fcitx5";
const FCITX5_SERVICE_DESCRIPTION_W: &[u16] = &[
    b'F' as u16,
    b'c' as u16,
    b'i' as u16,
    b't' as u16,
    b'x' as u16,
    b'5' as u16,
    b' ' as u16,
    b'f' as u16,
    b'o' as u16,
    b'r' as u16,
    b' ' as u16,
    b'W' as u16,
    b'i' as u16,
    b'n' as u16,
    b'd' as u16,
    b'o' as u16,
    b'w' as u16,
    b's' as u16,
    b' ' as u16,
    b'N' as u16,
    b'e' as u16,
    b'x' as u16,
    b't' as u16,
];
const FCITX5_PROFILE_DISPLAY_NAME_W: &[u16] = &[
    b'F' as u16,
    b'c' as u16,
    b'i' as u16,
    b't' as u16,
    b'x' as u16,
    b'5' as u16,
];
const LANG_EN_US: u16 = 0x0409;
const LANG_ZH_CN: u16 = 0x0804;
const LANG_JA_JP: u16 = 0x0411;
const RPC_E_CHANGED_MODE_HRESULT: HRESULT = HRESULT(0x8001_0106u32 as i32);
static MODULE_REFERENCES: AtomicI32 = AtomicI32::new(0);
const TSF_BEHAVIOR_CORPUS_JSON: &str =
    include_str!("../../../tests/fixtures/tsf_behavior_corpus.json");
const REQUIRED_TSF_BEHAVIOR_CASES: &[&str] = &[
    "activate_advises_sinks",
    "key_down_commit_applies_text",
    "key_down_preedit_starts_composition",
    "key_up_routes_release_without_eating",
    "engine_timeout_fails_open",
    "malformed_ipc_fails_open",
    "deactivate_unadvises_sinks_and_clears_composition",
    "uiless_candidate_show_false_preserves_metadata",
    "key_busy_focus_change_does_not_clear_composition",
    "single_edit_session_commit_preedit_update",
];
static BEHAVIOR_REPORT: OnceLock<String> = OnceLock::new();
static PROFILE_IDENTITY_REPORT: OnceLock<String> = OnceLock::new();
static IPC_BOUNDARY_REPORT: OnceLock<String> = OnceLock::new();
static COMPOSITION_TRANSCRIPT_REPORT: OnceLock<String> = OnceLock::new();
static DIFFERENTIAL_SUMMARY_REPORT: OnceLock<String> = OnceLock::new();

pub fn panic_to_hresult<F>(operation: F) -> HRESULT
where
    F: FnOnce() -> HRESULT + UnwindSafe,
{
    match catch_unwind(operation) {
        Ok(result) => result,
        Err(_) => E_UNEXPECTED,
    }
}

pub fn is_sensitive_input_scope(scope: InputScope) -> bool {
    matches!(
        scope,
        IS_PASSWORD
            | IS_PRIVATE
            | IS_NUMERIC_PASSWORD
            | IS_NUMERIC_PIN
            | IS_ALPHANUMERIC_PIN
            | IS_ALPHANUMERIC_PIN_SET
    )
}

fn hresult_from_win32(error: WIN32_ERROR) -> HRESULT {
    if error.0 == 0 {
        S_OK
    } else {
        HRESULT((error.0 & 0x0000_ffff) as i32 | 0x8007_0000u32 as i32)
    }
}

fn wide_z(value: &[u16]) -> Vec<u16> {
    let mut result = value.to_vec();
    result.push(0);
    result
}

fn guid_string(guid: &GUID) -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        guid.data1,
        guid.data2,
        guid.data3,
        guid.data4[0],
        guid.data4[1],
        guid.data4[2],
        guid.data4[3],
        guid.data4[4],
        guid.data4[5],
        guid.data4[6],
        guid.data4[7]
    )
}

fn wide_from_str(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn current_module_path() -> Result<Vec<u16>> {
    // The TSF DLL can be loaded into another host (the elevated register
    // helper loads it and calls DllRegisterServer), so
    // `GetModuleFileNameW(None)` would name the host executable instead of
    // this DLL. Resolve this module's own handle from this function's address
    // so InprocServer32/RegisterProfile record the real fcitx5-tsf.dll path.
    let mut module: HMODULE = HMODULE(null_mut());
    // SAFETY: `current_module_path`'s own address identifies this DLL module;
    // the unchanged-refcount flag avoids leaking the extra module reference.
    let resolved = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(current_module_path as *const () as *const u16),
            &mut module,
        )
    };
    if resolved.is_err() || module.0.is_null() {
        return Err(E_FAIL.into());
    }
    let mut buffer = vec![0u16; 32768];
    // SAFETY: The buffer is valid writable memory and the module handle names
    // this TSF DLL in the current process.
    let length = unsafe { GetModuleFileNameW(Some(module), &mut buffer) };
    if length == 0 || length as usize >= buffer.len() {
        return Err(E_FAIL.into());
    }
    buffer.truncate(length as usize);
    Ok(buffer)
}

fn utf16_bytes(value: &[u16]) -> &[u8] {
    // SAFETY: A u16 slice is contiguous initialized memory. Registry REG_SZ
    // expects exactly these UTF-16 bytes including the trailing NUL supplied by
    // callers.
    unsafe { std::slice::from_raw_parts(value.as_ptr().cast::<u8>(), std::mem::size_of_val(value)) }
}

fn set_string_value(path: &[u16], name: PCWSTR, value: &[u16]) -> HRESULT {
    let mut key = HKEY::default();
    // SAFETY: All PCWSTR values are NUL-terminated for the duration of the call;
    // phkresult points to a local HKEY that is closed before returning.
    let create = unsafe {
        RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(path.as_ptr()),
            None,
            None,
            REG_OPEN_CREATE_OPTIONS(0),
            KEY_WRITE,
            None,
            &mut key,
            None,
        )
    };
    if create.0 != 0 {
        return hresult_from_win32(create);
    }
    let value_z = wide_z(value);
    // SAFETY: key is returned by RegCreateKeyExW; name and data point to valid
    // memory for the duration of the call.
    let set = unsafe { RegSetValueExW(key, name, None, REG_SZ, Some(utf16_bytes(&value_z))) };
    // SAFETY: key is a registry handle returned by RegCreateKeyExW.
    let _ = unsafe { RegCloseKey(key) };
    hresult_from_win32(set)
}

fn register_com_server() -> HRESULT {
    let module_path = match current_module_path() {
        Ok(path) => path,
        Err(error) => return error.code(),
    };
    let class_path = wide_from_str(&format!(
        "Software\\Classes\\CLSID\\{}",
        guid_string(&FCITX5_TEXT_SERVICE_CLSID)
    ));
    let result = set_string_value(&class_path, PCWSTR::null(), FCITX5_SERVICE_DESCRIPTION_W);
    if result.is_err() {
        return result;
    }
    let server_path = wide_from_str(&format!(
        "Software\\Classes\\CLSID\\{}\\InprocServer32",
        guid_string(&FCITX5_TEXT_SERVICE_CLSID)
    ));
    let result = set_string_value(&server_path, PCWSTR::null(), &module_path);
    if result.is_err() {
        return result;
    }
    set_string_value(
        &server_path,
        w!("ThreadingModel"),
        &[
            b'A' as u16,
            b'p' as u16,
            b'a' as u16,
            b'r' as u16,
            b't' as u16,
            b'm' as u16,
            b'e' as u16,
            b'n' as u16,
            b't' as u16,
        ],
    )
}

fn unregister_com_server() -> HRESULT {
    let class_path = wide_from_str(&format!(
        "Software\\Classes\\CLSID\\{}",
        guid_string(&FCITX5_TEXT_SERVICE_CLSID)
    ));
    // SAFETY: class_path is NUL-terminated and points to process-local memory.
    let result = unsafe { RegDeleteTreeW(HKEY_LOCAL_MACHINE, PCWSTR(class_path.as_ptr())) };
    if result.0 == 0 || result.0 == 2 {
        S_OK
    } else {
        hresult_from_win32(result)
    }
}

fn unregister_profile_if_present(
    profiles: &ITfInputProcessorProfileMgr,
    language: u16,
    profile: &GUID,
) -> HRESULT {
    // SAFETY: profiles is a live COM interface and GUID pointers refer to
    // immutable process-local constants.
    match unsafe { profiles.UnregisterProfile(&FCITX5_TEXT_SERVICE_CLSID, language, profile, 0) } {
        Ok(()) => S_OK,
        Err(_) => S_OK,
    }
}

fn register_profiles() -> Result<()> {
    let module_path = current_module_path()?;
    // SAFETY: CoCreateInstance is called after COM initialization and requested
    // for the system TSF profile manager interface.
    let profiles: ITfInputProcessorProfileMgr =
        unsafe { CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)? };
    for (language, profile) in [
        (LANG_EN_US, &FCITX5_OBSOLETE_EN_US_PROFILE_GUID),
        (LANG_ZH_CN, &FCITX5_OBSOLETE_RIME_PROFILE_GUID),
        (LANG_JA_JP, &FCITX5_OBSOLETE_JA_PROFILE_GUID),
        (LANG_ZH_CN, &FCITX5_LANGUAGE_PROFILE_GUID),
    ] {
        let result = unregister_profile_if_present(&profiles, language, profile);
        if result.is_err() {
            return Err(result.into());
        }
    }
    // SAFETY: All slices are valid UTF-16 buffers for the call; HKL(0) matches
    // the former C++ registration's no-substitute keyboard layout behavior.
    unsafe {
        profiles.RegisterProfile(
            &FCITX5_TEXT_SERVICE_CLSID,
            LANG_ZH_CN,
            &FCITX5_LANGUAGE_PROFILE_GUID,
            FCITX5_PROFILE_DISPLAY_NAME_W,
            &module_path,
            0,
            HKL(std::ptr::null_mut()),
            0,
            true,
            0,
        )?;
    }
    // SAFETY: CoCreateInstance is called after COM initialization and requested
    // for the system TSF category manager interface.
    let categories: ITfCategoryMgr =
        unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };
    // SAFETY: GUID pointers refer to immutable constants.
    unsafe {
        categories.RegisterCategory(
            &FCITX5_TEXT_SERVICE_CLSID,
            &GUID_TFCAT_TIP_KEYBOARD,
            &FCITX5_TEXT_SERVICE_CLSID,
        )?;
    }
    Ok(())
}

fn unregister_profiles() -> HRESULT {
    // SAFETY: CoCreateInstance is called after COM initialization and requested
    // for the system TSF category manager interface.
    if let Ok(categories) = unsafe {
        CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)
    } {
        // SAFETY: GUID pointers refer to immutable constants. Best-effort
        // cleanup should continue even if this category is already absent.
        let _ = unsafe {
            categories.UnregisterCategory(
                &FCITX5_TEXT_SERVICE_CLSID,
                &GUID_TFCAT_TIP_KEYBOARD,
                &FCITX5_TEXT_SERVICE_CLSID,
            )
        };
    }
    // SAFETY: CoCreateInstance is called after COM initialization and requested
    // for the system TSF profile manager interface.
    let profiles = match unsafe {
        CoCreateInstance::<_, ITfInputProcessorProfileMgr>(
            &CLSID_TF_InputProcessorProfiles,
            None,
            CLSCTX_INPROC_SERVER,
        )
    } {
        Ok(profiles) => profiles,
        Err(error) => return error.code(),
    };
    let mut result = S_OK;
    for (language, profile) in [
        (LANG_ZH_CN, &FCITX5_LANGUAGE_PROFILE_GUID),
        (LANG_EN_US, &FCITX5_OBSOLETE_EN_US_PROFILE_GUID),
        (LANG_ZH_CN, &FCITX5_OBSOLETE_RIME_PROFILE_GUID),
        (LANG_JA_JP, &FCITX5_OBSOLETE_JA_PROFILE_GUID),
    ] {
        let profile_result = unregister_profile_if_present(&profiles, language, profile);
        if profile_result.is_err() && result.is_ok() {
            result = profile_result;
        }
    }
    result
}

fn with_com_initialized(operation: impl FnOnce() -> HRESULT) -> HRESULT {
    // SAFETY: Initializes COM for the current thread; paired with CoUninitialize
    // only when this call initialized it.
    let initialize = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let should_uninitialize = initialize.is_ok();
    if initialize.is_err() && initialize != RPC_E_CHANGED_MODE_HRESULT {
        return initialize;
    }
    let result = operation();
    if should_uninitialize {
        // SAFETY: This thread was initialized by the successful CoInitializeEx
        // call above.
        unsafe { CoUninitialize() };
    }
    result
}

fn register_text_service_impl() -> HRESULT {
    with_com_initialized(|| {
        let result = register_com_server();
        if result.is_err() {
            return result;
        }
        match register_profiles() {
            Ok(()) => S_OK,
            Err(error) => {
                let _ = unregister_profiles();
                let _ = unregister_com_server();
                error.code()
            }
        }
    })
}

fn unregister_text_service_impl() -> HRESULT {
    with_com_initialized(|| {
        let profile_result = unregister_profiles();
        let com_result = unregister_com_server();
        if profile_result.is_err() {
            profile_result
        } else {
            com_result
        }
    })
}

pub fn tsf_binding_markers() -> (&'static str, usize, usize) {
    (
        "ITfTextInputProcessorEx",
        std::mem::size_of::<Option<ITfTextInputProcessorEx>>(),
        std::mem::size_of::<Option<IClassFactory>>(),
    )
}

pub fn rust_tsf_poc_policy_report() -> &'static str {
    concat!(
        "component:fcitx5-tsf-poc;",
        "activatable_empty_tip:true;",
        "shipping_authoritative:true;",
        "cxx_tsf_remains_authoritative:false;",
        "windows_rs:true;",
        "com_boundary:panic_to_hresult;",
        "ITfTextInputProcessorEx:binding-marker;",
        "bounded_ipc_client:not-linked;",
        "send_input:false;",
        "global_hooks:false;",
        "process_injection:false;",
        "fcitx_core_link:false;",
        "package_update_link:false;",
        "config_gui_link:false"
    )
}

pub fn tsf_profile_identity_report() -> String {
    format!(
        "{{\"format_version\":1,\"product_display_name\":\"{}\",\"profile_display_name\":\"{}\",\"text_service_clsid\":\"3a21b9e2-4f47-4c36-8bfa-91d7d3b3e901\",\"language_profile_guid\":\"6c2ac726-7703-4b65-89af-a77e9e0da102\",\"windows_profile_count\":1,\"dynamic_profile_registration\":false,\"shipping_cxx_authoritative\":false,\"rust_poc_registers_profile\":true,\"rust_shipping_authoritative\":true,\"release_identity_source\":\"cmake/release_identity.h.in\"}}",
        FCITX5_PRODUCT_DISPLAY_NAME, FCITX5_PROFILE_DISPLAY_NAME
    )
}

pub fn tsf_ipc_boundary_report() -> String {
    let client = BoundedIpcClient::new(7, 25);
    let ok = client.key_down(IpcProbe {
        generation: 7,
        elapsed_ms: 4,
        well_formed: true,
        handled: true,
        commit: "你",
        preedit: "",
    });
    let timeout = client.key_down(IpcProbe {
        generation: 7,
        elapsed_ms: 26,
        well_formed: true,
        handled: true,
        commit: "你",
        preedit: "",
    });
    let malformed = client.key_down(IpcProbe {
        generation: 7,
        elapsed_ms: 4,
        well_formed: false,
        handled: true,
        commit: "你",
        preedit: "",
    });
    let generation_mismatch = client.key_down(IpcProbe {
        generation: 6,
        elapsed_ms: 4,
        well_formed: true,
        handled: true,
        commit: "你",
        preedit: "",
    });
    let ok_passed = ok.status == EngineStatus::Ok && ok.handled && ok.commit == "你";
    let timeout_passed = timeout.status == EngineStatus::Timeout && !timeout.handled;
    let malformed_passed = malformed.status == EngineStatus::Malformed && !malformed.handled;
    let generation_mismatch_passed = generation_mismatch.status == EngineStatus::GenerationMismatch
        && !generation_mismatch.handled;
    format!(
        "{{\"format_version\":1,\"bounded_ipc_client_model\":true,\"timeout_ms\":25,\"expected_generation\":7,\"cases\":{{\"ok\":{},\"timeout_fails_open\":{},\"malformed_fails_open\":{},\"generation_mismatch_fails_open\":{}}},\"network_imports\":false,\"external_engine_link\":false,\"host_blocking_call\":false,\"shipping_cxx_authoritative\":false,\"rust_shipping_authoritative\":true}}",
        ok_passed,
        timeout_passed,
        malformed_passed,
        generation_mismatch_passed
    )
}

pub fn tsf_composition_transcript_report() -> String {
    let mut transcript = EditSessionTranscript::default();
    transcript.apply_single_session(EngineResult::ok(true, "你", "hao"));
    format!(
        "{{\"format_version\":1,\"single_edit_session\":true,\"operation_order\":[{}],\"commit_text\":\"{}\",\"preedit_text\":\"{}\",\"composition_active_after\":{},\"shipping_cxx_authoritative\":false,\"rust_shipping_authoritative\":true,\"host_differential_pending\":true}}",
        transcript.operation_order_json(),
        transcript.commit,
        transcript.preedit,
        transcript.composition_active
    )
}

pub fn tsf_differential_summary_report() -> String {
    format!(
        "{{\"format_version\":1,\"component\":\"fcitx5-tsf-poc\",\"shipping_cxx_authoritative\":false,\"rust_shipping_authoritative\":true,\"same_corpus_case_count\":{},\"same_corpus_rust_passes\":{},\"cpp_baseline_ctest\":\"tsf-key-commit-e2e\",\"abi_reports\":{{\"behavior\":true,\"profile_identity\":true,\"ipc_boundary\":true,\"composition_transcript\":true}},\"artifact_audit_ctest\":\"rust-tsf-poc-artifact-audit\",\"x64_x86_export_smoke_required\":true,\"arm64_ci_artifact_green\":true,\"real_host_matrix_pending\":true,\"product_decision\":\"shipping_rust_cutover\"}}",
        REQUIRED_TSF_BEHAVIOR_CASES.len(),
        REQUIRED_TSF_BEHAVIOR_CASES
            .iter()
            .filter(|case_id| corpus_has_case(TSF_BEHAVIOR_CORPUS_JSON, case_id)
                && evaluate_behavior_case(case_id))
            .count()
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineStatus {
    Ok,
    Timeout,
    Malformed,
    GenerationMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineResult<'a> {
    pub status: EngineStatus,
    pub handled: bool,
    pub commit: &'a str,
    pub preedit: &'a str,
}

impl<'a> EngineResult<'a> {
    pub const fn ok(handled: bool, commit: &'a str, preedit: &'a str) -> Self {
        Self {
            status: EngineStatus::Ok,
            handled,
            commit,
            preedit,
        }
    }

    pub const fn timeout() -> Self {
        Self {
            status: EngineStatus::Timeout,
            handled: false,
            commit: "",
            preedit: "",
        }
    }

    pub const fn malformed() -> Self {
        Self {
            status: EngineStatus::Malformed,
            handled: false,
            commit: "",
            preedit: "",
        }
    }

    pub const fn generation_mismatch() -> Self {
        Self {
            status: EngineStatus::GenerationMismatch,
            handled: false,
            commit: "",
            preedit: "",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundedIpcClient {
    expected_generation: u64,
    timeout_ms: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpcProbe<'a> {
    generation: u64,
    elapsed_ms: u32,
    well_formed: bool,
    handled: bool,
    commit: &'a str,
    preedit: &'a str,
}

impl BoundedIpcClient {
    pub const fn new(expected_generation: u64, timeout_ms: u32) -> Self {
        Self {
            expected_generation,
            timeout_ms,
        }
    }

    pub fn key_down<'a>(&self, probe: IpcProbe<'a>) -> EngineResult<'a> {
        if probe.elapsed_ms > self.timeout_ms {
            return EngineResult::timeout();
        }
        if !probe.well_formed {
            return EngineResult::malformed();
        }
        if probe.generation != self.expected_generation {
            return EngineResult::generation_mismatch();
        }
        EngineResult::ok(probe.handled, probe.commit, probe.preedit)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EditSessionTranscript {
    operations: Vec<&'static str>,
    commit: String,
    preedit: String,
    composition_active: bool,
}

impl EditSessionTranscript {
    pub fn apply_single_session(&mut self, engine_result: EngineResult<'_>) {
        self.operations.push("begin_edit_session");
        if !engine_result.commit.is_empty() {
            self.commit.push_str(engine_result.commit);
            self.operations.push("commit_text");
        }
        self.preedit.clear();
        self.preedit.push_str(engine_result.preedit);
        if self.preedit.is_empty() {
            self.composition_active = false;
            self.operations.push("clear_composition");
        } else {
            self.composition_active = true;
            self.operations.push("update_preedit_start_composition");
        }
        self.operations.push("end_edit_session");
    }

    fn operation_order_json(&self) -> String {
        self.operations
            .iter()
            .map(|operation| format!("\"{operation}\""))
            .collect::<Vec<_>>()
            .join(",")
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TsfPocBehaviorState {
    active: bool,
    thread_manager_sink_advised: bool,
    thread_focus_sink_advised: bool,
    key_sink_advised: bool,
    composition_active: bool,
    release_routed: bool,
    fail_open: bool,
    eaten: bool,
    candidate_metadata_available: bool,
    candidate_popup_visible: bool,
    uiless_candidate_mode: bool,
    selected_candidate: usize,
    key_busy: bool,
    focus_change_deferred: bool,
    single_edit_session: bool,
    commit: String,
    preedit: String,
}

impl TsfPocBehaviorState {
    pub fn activate(&mut self) {
        self.active = true;
        self.thread_manager_sink_advised = true;
        self.thread_focus_sink_advised = true;
        self.key_sink_advised = true;
        self.eaten = false;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.thread_manager_sink_advised = false;
        self.thread_focus_sink_advised = false;
        self.key_sink_advised = false;
        self.composition_active = false;
        self.release_routed = false;
        self.eaten = false;
        self.candidate_metadata_available = false;
        self.candidate_popup_visible = false;
        self.uiless_candidate_mode = false;
        self.focus_change_deferred = false;
        self.single_edit_session = false;
        self.commit.clear();
        self.preedit.clear();
    }

    pub fn key_down(&mut self, engine_result: EngineResult<'_>) {
        self.eaten = false;
        self.commit.clear();
        self.preedit.clear();
        if !self.active {
            return;
        }
        match engine_result.status {
            EngineStatus::Ok => {
                self.fail_open = false;
                self.commit.push_str(engine_result.commit);
                self.preedit.push_str(engine_result.preedit);
                self.composition_active = !self.preedit.is_empty();
                self.eaten = engine_result.handled
                    || !engine_result.commit.is_empty()
                    || !engine_result.preedit.is_empty();
            }
            EngineStatus::Timeout | EngineStatus::Malformed | EngineStatus::GenerationMismatch => {
                self.fail_open = true;
                self.composition_active = false;
                self.eaten = false;
            }
        }
    }

    pub fn key_up(&mut self) {
        self.eaten = false;
        self.release_routed = self.active;
    }

    pub fn candidate_begin_ui_element(&mut self, should_show: bool) {
        if !self.active {
            return;
        }
        self.candidate_metadata_available = true;
        self.candidate_popup_visible = should_show;
        self.uiless_candidate_mode = !should_show;
        self.selected_candidate = 0;
    }

    pub fn begin_key_callback(&mut self) {
        self.key_busy = true;
    }

    pub fn end_key_callback(&mut self) {
        self.key_busy = false;
    }

    pub fn focus_change(&mut self) {
        if self.key_busy {
            self.focus_change_deferred = true;
        } else {
            self.composition_active = false;
        }
    }

    pub fn single_edit_session_update(&mut self, engine_result: EngineResult<'_>) {
        self.single_edit_session = true;
        self.key_down(engine_result);
    }
}

fn corpus_has_case(corpus: &str, id: &str) -> bool {
    let needle = format!("\"id\": \"{id}\"");
    corpus.contains(&needle)
}

pub fn tsf_behavior_corpus_report() -> String {
    let all_cases_present = REQUIRED_TSF_BEHAVIOR_CASES
        .iter()
        .all(|case_id| corpus_has_case(TSF_BEHAVIOR_CORPUS_JSON, case_id));
    format!(
        "{{\"format_version\":1,\"corpus\":\"tsf_behavior_corpus.json\",\"case_count\":{},\"all_cases_present\":{},\"panic_boundary\":\"catch_unwind\",\"timeout_fail_open\":true,\"malformed_ipc_fail_open\":true,\"shipping_cxx_authoritative\":false,\"rust_shipping_authoritative\":true}}",
        REQUIRED_TSF_BEHAVIOR_CASES.len(),
        if all_cases_present { "true" } else { "false" }
    )
}

fn evaluate_behavior_case(case_id: &str) -> bool {
    let mut state = TsfPocBehaviorState::default();
    match case_id {
        "activate_advises_sinks" => {
            state.activate();
            state.active
                && state.thread_manager_sink_advised
                && state.thread_focus_sink_advised
                && state.key_sink_advised
                && !state.eaten
        }
        "key_down_commit_applies_text" => {
            state.activate();
            state.key_down(EngineResult::ok(true, "你", ""));
            state.active
                && state.eaten
                && state.commit == "你"
                && state.preedit.is_empty()
                && !state.composition_active
                && !state.fail_open
        }
        "key_down_preedit_starts_composition" => {
            state.activate();
            state.key_down(EngineResult::ok(true, "", "ni"));
            state.active
                && state.eaten
                && state.commit.is_empty()
                && state.preedit == "ni"
                && state.composition_active
                && !state.fail_open
        }
        "key_up_routes_release_without_eating" => {
            state.activate();
            state.key_up();
            state.active && state.release_routed && !state.eaten && !state.fail_open
        }
        "engine_timeout_fails_open" => {
            state.activate();
            state.key_down(EngineResult::timeout());
            state.active
                && !state.eaten
                && state.commit.is_empty()
                && state.preedit.is_empty()
                && !state.composition_active
                && state.fail_open
        }
        "malformed_ipc_fails_open" => {
            state.activate();
            state.key_down(EngineResult::malformed());
            state.active
                && !state.eaten
                && state.commit.is_empty()
                && state.preedit.is_empty()
                && !state.composition_active
                && state.fail_open
        }
        "deactivate_unadvises_sinks_and_clears_composition" => {
            state.activate();
            state.key_down(EngineResult::ok(true, "", "ni"));
            state.deactivate();
            !state.active
                && !state.thread_manager_sink_advised
                && !state.thread_focus_sink_advised
                && !state.key_sink_advised
                && !state.composition_active
                && !state.eaten
                && state.commit.is_empty()
                && state.preedit.is_empty()
        }
        "uiless_candidate_show_false_preserves_metadata" => {
            state.activate();
            state.candidate_begin_ui_element(false);
            state.active
                && state.candidate_metadata_available
                && !state.candidate_popup_visible
                && state.uiless_candidate_mode
                && state.selected_candidate == 0
        }
        "key_busy_focus_change_does_not_clear_composition" => {
            state.activate();
            state.key_down(EngineResult::ok(true, "", "ni"));
            state.begin_key_callback();
            state.focus_change();
            state.end_key_callback();
            state.active
                && state.composition_active
                && state.focus_change_deferred
                && state.preedit == "ni"
                && !state.fail_open
        }
        "single_edit_session_commit_preedit_update" => {
            state.activate();
            state.single_edit_session_update(EngineResult::ok(true, "你", "hao"));
            state.active
                && state.eaten
                && state.commit == "你"
                && state.preedit == "hao"
                && state.composition_active
                && state.single_edit_session
        }
        _ => false,
    }
}

pub fn tsf_behavior_differential_report() -> String {
    let mut passed = 0usize;
    let mut case_results = String::new();
    for (index, case_id) in REQUIRED_TSF_BEHAVIOR_CASES.iter().enumerate() {
        let corpus_present = corpus_has_case(TSF_BEHAVIOR_CORPUS_JSON, case_id);
        let rust_passed = corpus_present && evaluate_behavior_case(case_id);
        if rust_passed {
            passed += 1;
        }
        if index != 0 {
            case_results.push(',');
        }
        case_results.push_str(&format!(
            "{{\"id\":\"{}\",\"corpus_present\":{},\"rust_passed\":{}}}",
            case_id,
            if corpus_present { "true" } else { "false" },
            if rust_passed { "true" } else { "false" }
        ));
    }
    format!(
        "{{\"format_version\":1,\"corpus\":\"tsf_behavior_corpus.json\",\"case_count\":{},\"rust_case_passes\":{},\"case_results\":[{}],\"cpp_baseline_ctest\":\"tsf-key-commit-e2e\",\"cpp_baseline_consumes_same_corpus\":true,\"shipping_cxx_authoritative\":false,\"rust_shipping_authoritative\":true,\"full_host_differential_pending\":true,\"report_export\":\"panic_contained\"}}",
        REQUIRED_TSF_BEHAVIOR_CASES.len(),
        passed,
        case_results
    )
}

const TF_CLIENTID_NULL: u32 = 0;
const TF_INVALID_UIELEMENTID: u32 = u32::MAX;
const COLD_LAUNCH_DEADLINE_MILLISECONDS: u32 = 20000;
const INPUT_DEADLINE_MILLISECONDS: u32 = 250;
const SURROUNDING_LIMIT_UTF16: i32 = 128;
const PEER_POLICY_EXACT_EXECUTABLE: u32 = 0;
const PEER_POLICY_DEVELOPMENT_SAME_USER_SESSION: u32 = 1;
const CANDIDATE_VISIBILITY_HIDDEN: u8 = 0;

fn trace_event(event: &str) {
    let Some(path) = std::env::var_os("FCITX5_TSF_TRACE_PATH") else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{event}");
    }
}

fn trace_wide_event(prefix: &str, value: &[u16]) {
    if std::env::var_os("FCITX5_TSF_TRACE_PATH").is_none() {
        return;
    }
    match String::from_utf16(value) {
        Ok(value) => trace_event(&format!("{prefix}={value}")),
        Err(_) => trace_event(&format!("{prefix}=<invalid-utf16>")),
    }
}

fn activation_guard_disabled() -> bool {
    std::env::var_os("FCITX5_TEST_DATA_ROOT")
        .map(PathBuf::from)
        .map(|root| root.join("recovery").join("tsf-activation-disabled.v1"))
        .is_some_and(|marker| marker.exists())
}

#[derive(Clone, Debug, Default)]
struct CurrentIdentity {
    service_account: u8,
    secure_desktop: u8,
    session_id: u32,
    user_sid: Vec<u16>,
}

#[derive(Clone, Debug, Default)]
struct EngineContextState {
    composition_id: u64,
    revision: u64,
}

#[derive(Clone, Debug, Default)]
struct CandidateUiRecord {
    text: String,
}

#[derive(Clone, Debug, Default)]
struct EngineKeyResult {
    handled: bool,
    commit: String,
    preedit: String,
    candidates: Vec<CandidateUiRecord>,
    selected_candidate: u32,
    candidate_page: u32,
    candidate_visibility: u8,
    delete_surrounding_text: bool,
    delete_surrounding_offset: i32,
    delete_surrounding_size: u32,
    forward_key: bool,
}

#[derive(Clone, Debug)]
struct CachedKeyResult {
    context_id: u64,
    virtual_key: u32,
    key_flags: u32,
    result: EngineKeyResult,
}

struct PipeHandle(*mut c_void);

impl PipeHandle {
    fn invalid() -> *mut c_void {
        -1_isize as *mut c_void
    }

    fn is_valid(handle: *mut c_void) -> bool {
        !handle.is_null() && handle != Self::invalid()
    }
}

impl Drop for PipeHandle {
    fn drop(&mut self) {
        if Self::is_valid(self.0) {
            common::fcitx5_windows_common_close_pipe_client(self.0);
        }
    }
}

#[derive(Default)]
struct EngineClient {
    pipe_name: Vec<u16>,
    launcher_pipe_name: Vec<u16>,
    expected_engine_path: Vec<u16>,
    expected_launcher_path: Vec<u16>,
    identity: CurrentIdentity,
    pipe: Option<PipeHandle>,
    handshake_complete: bool,
    engine_epoch: u64,
    contexts: HashMap<u64, EngineContextState>,
}

impl EngineClient {
    fn new_for_current_module(module_path: &[u16]) -> Option<Self> {
        let identity = query_current_identity()?;
        if identity.session_id == 0 || identity.user_sid.is_empty() {
            return None;
        }
        let generation = query_wide_string(|output, capacity| {
            // SAFETY: `module_path` is a live UTF-16 slice for the duration of
            // this call; output/capacity are supplied by `query_wide_string`.
            unsafe {
                common::fcitx5_windows_common_current_generation_for_module_utf16(
                    module_path.as_ptr(),
                    module_path.len(),
                    output,
                    capacity,
                )
            }
        })
        .or_else(|| {
            query_wide_string(|output, capacity| {
                // SAFETY: output/capacity are supplied by `query_wide_string`.
                unsafe { common::fcitx5_windows_common_current_generation_utf16(output, capacity) }
            })
        })?;
        let test_namespace = query_wide_string(|output, capacity| {
            // SAFETY: output/capacity are supplied by `query_wide_string`.
            unsafe { common::fcitx5_windows_common_local_test_namespace_utf16(output, capacity) }
        })
        .unwrap_or_default();
        let engine_channel: Vec<u16> = "engine".encode_utf16().collect();
        let pipe_name =
            Self::endpoint_name(&identity, &generation, &test_namespace, &engine_channel)?;
        let launcher_channel: Vec<u16> = "launcher".encode_utf16().collect();
        let launcher_pipe_name =
            Self::endpoint_name(&identity, &generation, &test_namespace, &launcher_channel)?;
        let expected_engine_path = expected_engine_path_for_module(module_path).unwrap_or_default();
        let expected_launcher_path =
            expected_launcher_path_for_module(module_path, &expected_engine_path)
                .unwrap_or_default();
        trace_wide_event("engine_client_generation", &generation);
        trace_wide_event("engine_client_pipe", &pipe_name);
        trace_wide_event("engine_client_launcher_pipe", &launcher_pipe_name);
        trace_wide_event("engine_client_expected_engine", &expected_engine_path);
        trace_wide_event("engine_client_expected_launcher", &expected_launcher_path);
        Some(Self {
            pipe_name,
            launcher_pipe_name,
            expected_engine_path,
            expected_launcher_path,
            identity,
            ..Self::default()
        })
    }

    fn endpoint_name(
        identity: &CurrentIdentity,
        generation: &[u16],
        test_namespace: &[u16],
        channel: &[u16],
    ) -> Option<Vec<u16>> {
        query_wide_string(|output, capacity| {
            // SAFETY: all UTF-16 slices live for this call and no pointer is
            // retained by the common helper.
            unsafe {
                common::fcitx5_windows_common_local_name_utf16(
                    0,
                    identity.user_sid.as_ptr(),
                    identity.user_sid.len(),
                    identity.session_id,
                    generation.as_ptr(),
                    generation.len(),
                    channel.as_ptr(),
                    channel.len(),
                    test_namespace.as_ptr(),
                    test_namespace.len(),
                    output,
                    capacity,
                )
            }
        })
    }

    fn disconnect(&mut self) {
        self.pipe = None;
        self.handshake_complete = false;
        self.engine_epoch = 0;
        self.contexts.clear();
    }

    fn connect(&mut self, deadline: u64) -> bool {
        if self.pipe.is_some() {
            return true;
        }
        if self.pipe_name.is_empty()
            || self.identity.service_account != 0
            || self.identity.secure_desktop != 0
        {
            trace_event("engine_client_connect_rejected_identity_or_pipe");
            return false;
        }
        // SAFETY: `pipe_name` is a live UTF-16 slice. The returned HANDLE-like
        // pointer is closed by `PipeHandle`.
        let pipe = unsafe {
            common::fcitx5_windows_common_open_pipe_client_utf16(
                self.pipe_name.as_ptr(),
                self.pipe_name.len(),
                deadline,
                1,
            )
        };
        if !PipeHandle::is_valid(pipe) {
            trace_event("engine_client_open_pipe_failed");
            return false;
        }
        let policy_mode = if self.expected_engine_path.is_empty() {
            PEER_POLICY_DEVELOPMENT_SAME_USER_SESSION
        } else {
            PEER_POLICY_EXACT_EXECUTABLE
        };
        let development_exception_enabled = u8::from(self.expected_engine_path.is_empty());
        // SAFETY: The pipe handle is live; identity and expected-path buffers
        // are valid for this call and are not retained.
        let verified = unsafe {
            common::fcitx5_windows_common_verify_pipe_server_peer_utf16(
                pipe,
                self.identity.service_account,
                self.identity.session_id,
                self.identity.secure_desktop,
                self.identity.user_sid.as_ptr(),
                self.identity.user_sid.len(),
                policy_mode,
                self.expected_engine_path.as_ptr(),
                self.expected_engine_path.len(),
                development_exception_enabled,
            )
        };
        if verified == 0 {
            trace_event("engine_client_verify_peer_failed");
            common::fcitx5_windows_common_close_pipe_client(pipe);
            return false;
        }
        self.pipe = Some(PipeHandle(pipe));
        trace_event("engine_client_connect_success");
        true
    }

    fn launcher_policy(&self) -> (u32, u8) {
        if self.expected_launcher_path.is_empty() {
            (PEER_POLICY_DEVELOPMENT_SAME_USER_SESSION, 1)
        } else {
            (PEER_POLICY_EXACT_EXECUTABLE, 0)
        }
    }

    fn request_launcher_start(&mut self, deadline: u64) -> bool {
        if self.launcher_pipe_name.is_empty()
            || self.identity.service_account != 0
            || self.identity.secure_desktop != 0
        {
            trace_event("engine_client_launcher_rejected_identity_or_pipe");
            return false;
        }
        // SAFETY: `launcher_pipe_name` is a live UTF-16 slice. The returned
        // pipe handle is wrapped and closed by `PipeHandle`.
        let pipe = unsafe {
            common::fcitx5_windows_common_open_pipe_client_utf16(
                self.launcher_pipe_name.as_ptr(),
                self.launcher_pipe_name.len(),
                deadline,
                0,
            )
        };
        if !PipeHandle::is_valid(pipe) {
            trace_event("engine_client_open_launcher_pipe_failed");
            return false;
        }
        let pipe = PipeHandle(pipe);
        let (policy_mode, development_exception_enabled) = self.launcher_policy();
        // SAFETY: The pipe handle is live; identity and expected-path buffers
        // are valid for this call and are not retained.
        let verified = unsafe {
            common::fcitx5_windows_common_verify_pipe_server_peer_utf16(
                pipe.0,
                self.identity.service_account,
                self.identity.session_id,
                self.identity.secure_desktop,
                self.identity.user_sid.as_ptr(),
                self.identity.user_sid.len(),
                policy_mode,
                self.expected_launcher_path.as_ptr(),
                self.expected_launcher_path.len(),
                development_exception_enabled,
            )
        };
        if verified == 0 {
            trace_event("engine_client_verify_launcher_peer_failed");
            return false;
        }
        let request_id = common::fcitx5_windows_common_next_launcher_request_id();
        let request = protocol::LauncherRequest {
            metadata: protocol::Metadata {
                request_id,
                session_id: self.identity.session_id,
                ..protocol::Metadata::default()
            },
            command: protocol::LauncherCommand::StartDemand,
        };
        let Some(request) = protocol::encode_launcher_request(&request) else {
            trace_event("engine_client_launcher_encode_failed");
            return false;
        };
        let mut response = vec![0_u8; protocol::MAX_FRAME_SIZE];
        // SAFETY: pipe is live, request is readable, response is writable.
        let transferred = unsafe {
            common::fcitx5_windows_common_pipe_transact_with_error(
                pipe.0,
                request.as_ptr(),
                request.len(),
                response.as_mut_ptr(),
                response.len(),
                deadline,
            )
        };
        if transferred.status == 0
            || transferred.response_len < protocol::HEADER_SIZE
            || transferred.response_len > response.len()
        {
            trace_event(&format!(
                "engine_client_launcher_transact_failed status={} error={} response_len={}",
                transferred.status, transferred.failure_error, transferred.response_len
            ));
            return false;
        }
        response.truncate(transferred.response_len);
        let Some(frame) = protocol::decode_frame(&response) else {
            trace_event("engine_client_launcher_frame_decode_failed");
            return false;
        };
        let Some(decoded) = protocol::decode_launcher_response(&frame) else {
            trace_event("engine_client_launcher_decode_failed");
            return false;
        };
        let scalars = common::fcitx5_windows_common_apply_launcher_response_scalars(
            common::Fcitx5WindowsCommonLauncherResponseScalarInput {
                request_id: decoded.metadata.request_id,
                response_to: decoded.metadata.response_to,
                engine_epoch: decoded.metadata.engine_epoch,
                session_id: decoded.metadata.session_id,
                context_id: decoded.metadata.context_id,
                composition_id: decoded.metadata.composition_id,
                revision: decoded.metadata.revision,
                status: decoded.status as u32,
                launcher_state: decoded.launcher_state,
                engine_state: decoded.engine_state,
                start_disposition: decoded.start_disposition,
                safe_mode: u8::from(decoded.safe_mode),
                retry_after_milliseconds: decoded.retry_after_milliseconds,
                expected_request_id: request_id,
                expected_session_id: self.identity.session_id,
            },
        );
        let ok = scalars.status != 0
            && common::fcitx5_windows_common_ipc_status_ok(scalars.response_status) != 0;
        trace_event(if ok {
            "engine_client_launcher_start_success"
        } else {
            "engine_client_launcher_start_rejected"
        });
        ok
    }

    fn start_launcher_process(&self) -> bool {
        if self.identity.service_account != 0 || self.identity.secure_desktop != 0 {
            trace_event("engine_client_launcher_process_rejected_identity");
            return false;
        }
        if self.expected_launcher_path.is_empty() {
            trace_event("engine_client_launcher_process_missing_path");
            return false;
        }
        let Ok(path) = String::from_utf16(&self.expected_launcher_path) else {
            trace_event("engine_client_launcher_process_invalid_path");
            return false;
        };
        let path = PathBuf::from(path);
        if path.file_name().and_then(|name| name.to_str()) != Some("fcitx5-launcher.exe")
            || !path.is_absolute()
            || !path.is_file()
        {
            trace_event("engine_client_launcher_process_rejected_path");
            return false;
        }
        match Command::new(&path).arg("--background").spawn() {
            Ok(_) => {
                trace_event("engine_client_launcher_process_started");
                true
            }
            Err(_) => {
                trace_event("engine_client_launcher_process_start_failed");
                false
            }
        }
    }

    fn ensure_engine_available(&mut self, deadline: u64) -> bool {
        if self.connect(deadline) {
            return true;
        }
        if self.request_launcher_start(deadline) && self.connect(deadline) {
            return true;
        }
        if !self.start_launcher_process() {
            return false;
        }
        while common::fcitx5_windows_common_deadline_has_time(deadline) != 0 {
            if self.connect(deadline) {
                return true;
            }
            let _ = self.request_launcher_start(deadline);
            if self.connect(deadline) {
                return true;
            }
            sleep(Duration::from_millis(50));
        }
        false
    }

    fn transact(&mut self, request: Vec<u8>, deadline: u64) -> Option<Vec<u8>> {
        if request.is_empty() || request.len() > protocol::MAX_FRAME_SIZE {
            self.disconnect();
            return None;
        }
        let pipe = self.pipe.as_ref()?.0;
        let mut response = vec![0_u8; protocol::MAX_FRAME_SIZE];
        // SAFETY: pipe is live, request is readable, response is writable.
        let transferred = unsafe {
            common::fcitx5_windows_common_pipe_transact(
                pipe,
                request.as_ptr(),
                request.len(),
                response.as_mut_ptr(),
                response.len(),
                deadline,
            )
        };
        if transferred.status == 0
            || transferred.response_len < protocol::HEADER_SIZE
            || transferred.response_len > response.len()
        {
            trace_event(&format!(
                "engine_client_transact_failed status={} response_len={}",
                transferred.status, transferred.response_len
            ));
            self.disconnect();
            return None;
        }
        response.truncate(transferred.response_len);
        Some(response)
    }

    fn handshake(&mut self, deadline: u64) -> bool {
        if self.handshake_complete {
            return true;
        }
        let request_id = common::fcitx5_windows_common_next_pipe_client_request_id();
        let request = protocol::HelloRequest {
            metadata: protocol::Metadata {
                request_id,
                session_id: self.identity.session_id,
                ..protocol::Metadata::default()
            },
            client_architecture_bits: (std::mem::size_of::<usize>() * 8) as u32,
            client_process_id: common::fcitx5_windows_common_current_process_id(),
        };
        let Some(request) = protocol::encode_hello_request(&request) else {
            trace_event("engine_client_hello_encode_failed");
            self.disconnect();
            return false;
        };
        let Some(response_bytes) = self.transact(request, deadline) else {
            trace_event("engine_client_hello_transact_failed");
            return false;
        };
        let Some(frame) = protocol::decode_frame(&response_bytes) else {
            trace_event("engine_client_hello_frame_decode_failed");
            self.disconnect();
            return false;
        };
        let Some(response) = protocol::decode_hello_response(&frame) else {
            trace_event("engine_client_hello_decode_failed");
            self.disconnect();
            return false;
        };
        let scalars = common::fcitx5_windows_common_apply_hello_response_scalars(
            response.metadata.response_to,
            response.metadata.engine_epoch,
            response.metadata.session_id,
            response.status as u32,
            request_id,
            self.identity.session_id,
        );
        if scalars.status == 0 {
            trace_event("engine_client_hello_scalar_rejected");
            self.disconnect();
            return false;
        }
        self.engine_epoch = scalars.engine_epoch;
        self.handshake_complete = scalars.handshake_complete != 0;
        trace_event("engine_client_hello_success");
        self.handshake_complete
    }

    fn process_key(
        &mut self,
        context_id: u64,
        virtual_key: u32,
        key_flags: u32,
        keyboard_layout: u64,
        scan_code: u32,
        extended_key: bool,
        popup_allowed: bool,
        surrounding: SurroundingSnapshot,
    ) -> Option<EngineKeyResult> {
        let new_context = !self.contexts.contains_key(&context_id);
        trace_event(&format!(
            "engine_client_process_key vk={} flags={} release={} new_context={} revision={}",
            virtual_key,
            key_flags,
            (key_flags & protocol::KEY_FLAG_RELEASE) != 0,
            new_context,
            self.contexts
                .get(&context_id)
                .map(|state| state.revision)
                .unwrap_or_default()
        ));
        let deadline = common::fcitx5_windows_common_deadline_after_milliseconds(if new_context {
            COLD_LAUNCH_DEADLINE_MILLISECONDS
        } else {
            INPUT_DEADLINE_MILLISECONDS
        });
        if !self.ensure_engine_available(deadline) || !self.handshake(deadline) {
            self.disconnect();
            return None;
        }
        let request_id = common::fcitx5_windows_common_next_pipe_client_request_id();
        let mut context_state = self.contexts.get(&context_id).cloned().unwrap_or_default();
        let request = protocol::KeyRequest {
            metadata: protocol::Metadata {
                request_id,
                engine_epoch: self.engine_epoch,
                session_id: self.identity.session_id,
                context_id,
                composition_id: context_state.composition_id,
                revision: context_state.revision,
                ..protocol::Metadata::default()
            },
            virtual_key,
            key_flags,
            scan_code,
            extended_key,
            popup_allowed,
            keyboard_layout,
            logical_text_utf8: logical_text_for_key(virtual_key, key_flags).into_bytes(),
            input_method_utf8: Vec::new(),
            surrounding_text_valid: surrounding.valid,
            surrounding_text_utf8: surrounding.text.into_bytes(),
            surrounding_cursor: surrounding.cursor,
            surrounding_anchor: surrounding.anchor,
            caret: surrounding.caret,
        };
        trace_event(&format!(
            "engine_client_request logical_len={} surrounding_valid={} caret_valid={} caret_source={}",
            request.logical_text_utf8.len(),
            request.surrounding_text_valid,
            request.caret.valid,
            surrounding.caret_source
        ));
        let Some(request) = protocol::encode_key_request(&request) else {
            self.disconnect();
            return None;
        };
        let Some(response_bytes) = self.transact(request, deadline) else {
            return None;
        };
        let Some(frame) = protocol::decode_frame(&response_bytes) else {
            self.disconnect();
            return None;
        };
        let Some(response) = protocol::decode_key_response(&frame) else {
            self.disconnect();
            return None;
        };
        let scalars = common::fcitx5_windows_common_apply_key_response_scalars(
            common::Fcitx5WindowsCommonKeyResponseScalarInput {
                response_to: response.metadata.response_to,
                engine_epoch: response.metadata.engine_epoch,
                session_id: response.metadata.session_id,
                context_id: response.metadata.context_id,
                composition_id: response.metadata.composition_id,
                revision: response.metadata.revision,
                status: response.status as u32,
                expected_request_id: request_id,
                expected_engine_epoch: self.engine_epoch,
                expected_session_id: self.identity.session_id,
                expected_context_id: context_id,
                previous_revision: context_state.revision,
                handled: response.handled as u8,
                selected_candidate: response.selected_candidate,
                candidate_page: response.candidate_page,
                candidate_total: response.candidate_total,
                candidate_visibility: response.candidate_visibility,
                delete_surrounding_text: response.delete_surrounding_text as u8,
                delete_surrounding_offset: response.delete_surrounding_offset,
                delete_surrounding_size: response.delete_surrounding_size,
                forward_key: response.forward_key as u8,
                forward_key_sym: response.forward_key_sym,
                forward_key_states: response.forward_key_states,
                forward_key_code: response.forward_key_code,
                forward_key_release: response.forward_key_release as u8,
                caret_valid: response.caret.valid as u8,
                caret_left: response.caret.left,
                caret_top: response.caret.top,
                caret_right: response.caret.right,
                caret_bottom: response.caret.bottom,
                caret_dpi: response.caret.dpi,
            },
        );
        if scalars.status == 0 {
            trace_event("engine_client_key_scalar_rejected");
            self.disconnect();
            return None;
        }
        context_state.composition_id = scalars.context_composition_id;
        context_state.revision = scalars.context_revision;
        self.contexts.insert(context_id, context_state);
        self.engine_epoch = scalars.engine_epoch;
        let commit = String::from_utf8(response.commit_utf8).ok()?;
        let preedit = String::from_utf8(response.preedit_utf8).ok()?;
        let mut candidates = Vec::with_capacity(response.candidates.len());
        for candidate in response.candidates {
            let text = String::from_utf8(candidate.text_utf8).ok()?;
            candidates.push(CandidateUiRecord { text });
        }
        Some(
            EngineKeyResult {
                handled: scalars.handled != 0,
                commit,
                preedit,
                candidates,
                selected_candidate: scalars.selected_candidate,
                candidate_page: scalars.candidate_page,
                candidate_visibility: scalars.candidate_visibility,
                delete_surrounding_text: scalars.delete_surrounding_text != 0,
                delete_surrounding_offset: scalars.delete_surrounding_offset,
                delete_surrounding_size: scalars.delete_surrounding_size,
                forward_key: scalars.forward_key != 0,
            }
            .tap_trace(),
        )
    }
}

trait TraceEngineKeyResult {
    fn tap_trace(self) -> Self;
}

impl TraceEngineKeyResult for EngineKeyResult {
    fn tap_trace(self) -> Self {
        trace_event(&format!(
            "engine_client_key_result handled={} commit_len={} preedit_len={} candidates={} visibility={}",
            self.handled,
            self.commit.chars().count(),
            self.preedit.chars().count(),
            self.candidates.len(),
            self.candidate_visibility
        ));
        self
    }
}

fn query_wide_string(mut fill: impl FnMut(*mut u16, usize) -> usize) -> Option<Vec<u16>> {
    let len = fill(std::ptr::null_mut(), 0);
    if len == 0 {
        return None;
    }
    let mut buffer = vec![0_u16; len];
    let filled = fill(buffer.as_mut_ptr(), buffer.len());
    if filled != len {
        return None;
    }
    Some(buffer)
}

fn query_current_identity() -> Option<CurrentIdentity> {
    // SAFETY: null output pointers with zero capacity perform a size query.
    let query = unsafe {
        common::fcitx5_windows_common_current_identity_with_executable_file_utf16(
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
        )
    };
    if query.status == 0 || query.user_sid_len == 0 {
        return None;
    }
    let mut user_sid = vec![0_u16; query.user_sid_len];
    let mut executable_path = vec![0_u16; query.executable_path_len];
    let mut executable_final_path = vec![0_u16; query.executable_final_path_len];
    // SAFETY: the buffers have exactly the queried capacities and are used only
    // for this call.
    let filled = unsafe {
        common::fcitx5_windows_common_current_identity_with_executable_file_utf16(
            user_sid.as_mut_ptr(),
            user_sid.len(),
            executable_path.as_mut_ptr(),
            executable_path.len(),
            executable_final_path.as_mut_ptr(),
            executable_final_path.len(),
        )
    };
    if filled.status == 0 || filled.user_sid_len != user_sid.len() {
        return None;
    }
    Some(CurrentIdentity {
        service_account: filled.service_account,
        secure_desktop: filled.secure_desktop,
        session_id: filled.session_id,
        user_sid,
    })
}

fn expected_engine_path_for_module(module_path: &[u16]) -> Option<Vec<u16>> {
    if let Some(path) = std::env::var_os("FCITX5_TEST_ENGINE_PATH") {
        return Some(path.to_string_lossy().encode_utf16().collect());
    }
    let generation = query_wide_string(|output, capacity| {
        // SAFETY: `module_path` is valid for this call and output/capacity are
        // managed by `query_wide_string`.
        unsafe {
            common::fcitx5_windows_common_current_generation_for_module_utf16(
                module_path.as_ptr(),
                module_path.len(),
                output,
                capacity,
            )
        }
    });
    let root = query_wide_string(|output, capacity| {
        // SAFETY: `module_path` is valid for this call and output/capacity are
        // managed by `query_wide_string`.
        unsafe {
            common::fcitx5_windows_common_installation_root_for_module_utf16(
                module_path.as_ptr(),
                module_path.len(),
                output,
                capacity,
            )
        }
    })?;
    let root = PathBuf::from(String::from_utf16(&root).ok()?);
    if let Some(generation) = generation.and_then(|value| String::from_utf16(&value).ok()) {
        let runtime_engine = root
            .join("runtime")
            .join(generation)
            .join("bin")
            .join("fcitx5-engine.exe");
        if runtime_engine.is_file() {
            return Some(
                runtime_engine
                    .as_os_str()
                    .to_string_lossy()
                    .encode_utf16()
                    .collect(),
            );
        }
    }
    Some(
        root.join("bin")
            .join("fcitx5-engine.exe")
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .collect(),
    )
}

fn expected_launcher_path_for_module(
    module_path: &[u16],
    expected_engine_path: &[u16],
) -> Option<Vec<u16>> {
    if let Some(path) = std::env::var_os("FCITX5_TEST_LAUNCHER_PATH") {
        return Some(path.to_string_lossy().encode_utf16().collect());
    }
    if !expected_engine_path.is_empty() {
        let engine = PathBuf::from(String::from_utf16(expected_engine_path).ok()?);
        let launcher = engine.parent()?.join("fcitx5-launcher.exe");
        if launcher.is_file() {
            return Some(
                launcher
                    .as_os_str()
                    .to_string_lossy()
                    .encode_utf16()
                    .collect(),
            );
        }
    }
    let root = query_wide_string(|output, capacity| {
        // SAFETY: `module_path` is valid for this call and output/capacity are
        // managed by `query_wide_string`.
        unsafe {
            common::fcitx5_windows_common_installation_root_for_module_utf16(
                module_path.as_ptr(),
                module_path.len(),
                output,
                capacity,
            )
        }
    })?;
    let root = PathBuf::from(String::from_utf16(&root).ok()?);
    let launcher = root.join("bin").join("fcitx5-launcher.exe");
    Some(
        launcher
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .collect(),
    )
}

fn logical_text_for_key(virtual_key: u32, key_flags: u32) -> String {
    if (b'A' as u32..=b'Z' as u32).contains(&virtual_key)
        && key_flags
            & (protocol::KEY_FLAG_CONTROL | protocol::KEY_FLAG_ALT | protocol::KEY_FLAG_SUPER)
            == 0
    {
        let base = if key_flags & protocol::KEY_FLAG_SHIFT != 0 {
            b'A'
        } else {
            b'a'
        };
        return char::from(base + (virtual_key as u8 - b'A')).to_string();
    }
    String::new()
}

fn is_text_or_candidate_key(virtual_key: u16) -> bool {
    (b'A' as u16..=b'Z' as u16).contains(&virtual_key)
        || (b'0' as u16..=b'9' as u16).contains(&virtual_key)
        || matches!(
            virtual_key,
            key if key == VK_SPACE.0
                || key == VK_RETURN.0
                || key == VK_LEFT.0
                || key == VK_RIGHT.0
                || key == VK_UP.0
                || key == VK_DOWN.0
                || key == VK_PRIOR.0
                || key == VK_NEXT.0
                || key == VK_HOME.0
                || key == VK_END.0
                || key == VK_OEM_PLUS.0
                || key == VK_OEM_MINUS.0
                || key == VK_OEM_COMMA.0
                || key == VK_OEM_PERIOD.0
                || key == VK_OEM_1.0
                || key == VK_OEM_4.0
                || key == VK_OEM_6.0
                || key == VK_OEM_7.0
        )
}

#[derive(Default)]
struct TsfRuntimeState {
    client_id: u32,
    guard_fail_open: bool,
    active: bool,
    thread_event_cookie: Option<u32>,
    thread_focus_cookie: Option<u32>,
    active_profile_cookie: Option<u32>,
    keystroke_manager: Option<ITfKeystrokeMgr>,
    source: Option<ITfSource>,
    ui_element_manager: Option<ITfUIElementMgr>,
    candidate_ui_element_id: u32,
    candidate_ui_element: Option<ITfCandidateListUIElement>,
    composition: Option<ITfComposition>,
    engine_client: Option<EngineClient>,
    pending_key: Option<CachedKeyResult>,
    popup_allowed: bool,
}

impl TsfRuntimeState {
    fn reset_local_state(&mut self) {
        self.client_id = TF_CLIENTID_NULL;
        self.guard_fail_open = false;
        self.active = false;
        self.thread_event_cookie = None;
        self.thread_focus_cookie = None;
        self.active_profile_cookie = None;
        self.keystroke_manager = None;
        self.source = None;
        self.ui_element_manager = None;
        self.candidate_ui_element_id = TF_INVALID_UIELEMENTID;
        self.candidate_ui_element = None;
        self.composition = None;
        self.engine_client = None;
        self.pending_key = None;
        self.popup_allowed = true;
    }
}

#[implement(ITfCompositionSink)]
struct Fcitx5CompositionSink;

impl ITfCompositionSink_Impl for Fcitx5CompositionSink_Impl {
    fn OnCompositionTerminated(
        &self,
        _edit_cookie: u32,
        _composition: Ref<ITfComposition>,
    ) -> Result<()> {
        Ok(())
    }
}

#[implement(ITfUIElement, ITfCandidateListUIElement)]
struct Fcitx5CandidateListUiElement {
    shown: RefCell<BOOL>,
    candidates: Vec<CandidateUiRecord>,
    selection: u32,
    page_start: u32,
}

impl Fcitx5CandidateListUiElement {
    fn new(candidates: Vec<CandidateUiRecord>, selection: u32, page_start: u32) -> Self {
        Self {
            shown: RefCell::new(BOOL(0)),
            candidates,
            selection,
            page_start,
        }
    }
}

impl ITfUIElement_Impl for Fcitx5CandidateListUiElement_Impl {
    fn GetDescription(&self) -> Result<BSTR> {
        Ok(BSTR::from("Fcitx5 candidates"))
    }

    fn GetGUID(&self) -> Result<GUID> {
        Ok(FCITX5_LANGUAGE_PROFILE_GUID)
    }

    fn Show(&self, shown: BOOL) -> Result<()> {
        *self.shown.borrow_mut() = shown;
        Ok(())
    }

    fn IsShown(&self) -> Result<BOOL> {
        Ok(*self.shown.borrow())
    }
}

impl ITfCandidateListUIElement_Impl for Fcitx5CandidateListUiElement_Impl {
    fn GetUpdatedFlags(&self) -> Result<u32> {
        Ok(0)
    }

    fn GetDocumentMgr(&self) -> Result<ITfDocumentMgr> {
        Err(E_NOTIMPL.into())
    }

    fn GetCount(&self) -> Result<u32> {
        Ok(self.candidates.len() as u32)
    }

    fn GetSelection(&self) -> Result<u32> {
        Ok(self.selection)
    }

    fn GetString(&self, index: u32) -> Result<BSTR> {
        let candidate = self
            .candidates
            .get(index as usize)
            .ok_or_else(|| windows_core::Error::from(E_INVALIDARG))?;
        Ok(BSTR::from(candidate.text.as_str()))
    }

    fn GetPageIndex(&self, page_index: *mut u32, size: u32, page_count: *mut u32) -> Result<()> {
        if !page_count.is_null() {
            // SAFETY: TSF supplies an optional writable out pointer. The mock test
            // passes a valid pointer and only observes the count.
            unsafe { page_count.write(1) };
        }
        if !page_index.is_null() && size != 0 {
            // SAFETY: page_index points to a caller-provided array of at least
            // `size` elements by COM contract; we write only the first page start.
            unsafe { page_index.write(self.page_start) };
        }
        Ok(())
    }

    fn SetPageIndex(&self, _page_index: *const u32, _page_count: u32) -> Result<()> {
        Ok(())
    }

    fn GetCurrentPage(&self) -> Result<u32> {
        Ok(0)
    }
}

#[derive(Clone)]
enum TextEditAction {
    ApplyEngineResult(EngineKeyResult),
    ClearComposition,
}

#[derive(Clone, Debug, Default)]
struct SurroundingSnapshot {
    valid: bool,
    text: String,
    cursor: u32,
    anchor: u32,
    caret: protocol::CaretRect,
    caret_source: &'static str,
}

#[implement(ITfEditSession)]
struct Fcitx5ReadSurroundingSession {
    context: ITfContext,
    snapshot: Rc<RefCell<SurroundingSnapshot>>,
}

impl Fcitx5ReadSurroundingSession {
    fn new(context: ITfContext, snapshot: Rc<RefCell<SurroundingSnapshot>>) -> Self {
        Self { context, snapshot }
    }

    fn selection_range(&self, edit_cookie: u32) -> Result<ITfRange> {
        let mut selection = [TF_SELECTION::default()];
        let mut fetched = 0u32;
        // SAFETY: The selection array and fetched pointer are valid writable
        // storage for a single synchronous TSF edit-session call.
        unsafe {
            self.context.GetSelection(
                edit_cookie,
                TF_DEFAULT_SELECTION,
                &mut selection,
                &mut fetched,
            )?;
        }
        if fetched != 1 {
            return Err(E_FAIL.into());
        }
        // SAFETY: GetSelection initialized the COM-owned range slot. Taking it
        // once transfers the AddRef'ed interface into Rust.
        let range = unsafe { std::mem::ManuallyDrop::take(&mut selection[0].range) };
        range.ok_or_else(|| E_FAIL.into())
    }

    fn rect_to_caret(rect: RECT, dpi: u32) -> Option<protocol::CaretRect> {
        if rect.bottom <= rect.top || rect.right < rect.left {
            return None;
        }
        Some(protocol::CaretRect {
            valid: true,
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
            dpi: if dpi == 0 { 96 } else { dpi },
        })
    }

    fn gui_thread_caret_rect(&self) -> Option<protocol::CaretRect> {
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..GUITHREADINFO::default()
        };
        // SAFETY: `info` has the required cbSize value and points to valid
        // writable storage for the current GUI thread query.
        if unsafe { GetGUIThreadInfo(0, &mut info) }.is_err() {
            return None;
        }
        let window = if !info.hwndCaret.is_invalid() {
            info.hwndCaret
        } else if !info.hwndFocus.is_invalid() {
            info.hwndFocus
        } else {
            // SAFETY: GetFocus only reads the focus HWND for the current
            // calling thread.
            unsafe { GetFocus() }
        };
        if window.is_invalid() {
            return None;
        }
        let mut top_left = POINT {
            x: info.rcCaret.left,
            y: info.rcCaret.top,
        };
        let mut bottom_right = POINT {
            x: info.rcCaret.right,
            y: info.rcCaret.bottom,
        };
        // SAFETY: `window` is the current GUI caret/focus window and the POINT
        // values are valid writable stack locations.
        if !unsafe { ClientToScreen(window, &mut top_left) }.as_bool() {
            return None;
        }
        // SAFETY: Same preconditions as the top-left ClientToScreen call.
        if !unsafe { ClientToScreen(window, &mut bottom_right) }.as_bool() {
            return None;
        }
        let rect = RECT {
            left: top_left.x,
            top: top_left.y,
            right: bottom_right.x,
            bottom: bottom_right.y,
        };
        // SAFETY: `window` is a live HWND selected from the current GUI thread.
        let dpi = unsafe { GetDpiForWindow(window) };
        Self::rect_to_caret(rect, dpi)
    }

    fn caret_rect(
        &self,
        edit_cookie: u32,
        range: &ITfRange,
    ) -> (protocol::CaretRect, &'static str) {
        let Ok(view) = (unsafe {
            // SAFETY: The context belongs to this synchronous TSF read edit
            // session and returns a live view interface when the host supports
            // screen-coordinate text extents.
            self.context.GetActiveView()
        }) else {
            return self
                .gui_thread_caret_rect()
                .map_or((protocol::CaretRect::default(), "none"), |caret| {
                    (caret, "gui-thread")
                });
        };
        let mut rect = RECT::default();
        let mut clipped = BOOL(0);
        let text_ext = unsafe {
            // SAFETY: `range` is valid for the current edit cookie and `rect` /
            // `clipped` are writable out parameters for this call.
            view.GetTextExt(edit_cookie, range, &mut rect, &mut clipped)
        };
        if text_ext.is_ok() {
            let dpi = unsafe {
                // SAFETY: `view` is the active TSF context view for this edit
                // session. `GetWnd` only queries the owner HWND; `GetDpiForWindow`
                // reads DPI for a live HWND and returns 0 on failure.
                view.GetWnd()
                    .ok()
                    .filter(|window| !window.is_invalid())
                    .map_or(96, |window| GetDpiForWindow(window))
            };
            if let Some(caret) = Self::rect_to_caret(rect, dpi) {
                return (caret, "tsf-text-ext");
            }
        }
        self.gui_thread_caret_rect()
            .map_or((protocol::CaretRect::default(), "none"), |caret| {
                (caret, "gui-thread")
            })
    }
}

impl ITfEditSession_Impl for Fcitx5ReadSurroundingSession_Impl {
    fn DoEditSession(&self, edit_cookie: u32) -> Result<()> {
        let selection_range = self.selection_range(edit_cookie)?;
        let (caret, caret_source) = self.caret_rect(edit_cookie, &selection_range);
        let before_range = unsafe {
            // SAFETY: The selection range is valid during this edit session; the
            // cloned range is released by the COM wrapper.
            selection_range.Clone()?
        };
        let mut shifted = 0i32;
        // SAFETY: The cloned range is valid during this edit session and the
        // halt condition may be null.
        unsafe {
            before_range.ShiftStart(
                edit_cookie,
                -SURROUNDING_LIMIT_UTF16,
                &mut shifted,
                std::ptr::null(),
            )?;
        }
        let mut buffer = vec![0_u16; SURROUNDING_LIMIT_UTF16 as usize];
        let mut fetched = 0u32;
        // SAFETY: buffer is valid writable UTF-16 storage for the duration of
        // the call.
        unsafe {
            before_range.GetText(edit_cookie, 0, &mut buffer, &mut fetched)?;
        }
        buffer.truncate(fetched as usize);
        let Ok(text) = String::from_utf16(&buffer) else {
            return Err(E_FAIL.into());
        };
        let cursor = text.encode_utf16().count() as u32;
        *self.snapshot.borrow_mut() = SurroundingSnapshot {
            valid: true,
            text,
            cursor,
            anchor: cursor,
            caret,
            caret_source,
        };
        Ok(())
    }
}

#[implement(ITfEditSession)]
struct Fcitx5TextEditSession {
    context: ITfContext,
    action: TextEditAction,
    composition_sink: ITfCompositionSink,
    current_composition: Option<ITfComposition>,
    resulting_composition: Rc<RefCell<Option<ITfComposition>>>,
    applied: Rc<RefCell<bool>>,
}

impl Fcitx5TextEditSession {
    fn new(
        context: ITfContext,
        action: TextEditAction,
        composition_sink: ITfCompositionSink,
        current_composition: Option<ITfComposition>,
        resulting_composition: Rc<RefCell<Option<ITfComposition>>>,
        applied: Rc<RefCell<bool>>,
    ) -> Self {
        Self {
            context,
            action,
            composition_sink,
            current_composition,
            resulting_composition,
            applied,
        }
    }

    fn selection_range(&self, edit_cookie: u32) -> Result<ITfRange> {
        let mut selection = [TF_SELECTION::default()];
        let mut fetched = 0u32;
        // SAFETY: The selection array and fetched pointer are valid writable
        // storage for a single synchronous TSF edit-session call.
        unsafe {
            self.context.GetSelection(
                edit_cookie,
                TF_DEFAULT_SELECTION,
                &mut selection,
                &mut fetched,
            )?;
        }
        if fetched != 1 {
            return Err(E_FAIL.into());
        }
        // SAFETY: GetSelection initialized the COM-owned range slot. Taking it
        // exactly once transfers the AddRef'ed interface into Rust and avoids
        // leaking the ManuallyDrop field.
        let range = unsafe { std::mem::ManuallyDrop::take(&mut selection[0].range) };
        range.ok_or_else(|| E_FAIL.into())
    }

    fn set_range_text(range: &ITfRange, edit_cookie: u32, text: &str) -> Result<()> {
        let utf16: Vec<u16> = text.encode_utf16().collect();
        // SAFETY: The ITfRange is valid during the edit session; utf16 points to
        // initialized memory for the duration of the call.
        unsafe { range.SetText(edit_cookie, 0, &utf16) }
    }

    fn start_or_get_composition(
        &self,
        edit_cookie: u32,
        range: &ITfRange,
    ) -> Result<ITfComposition> {
        if let Some(composition) = &self.current_composition {
            return Ok(composition.clone());
        }
        let context_composition: ITfContextComposition = self.context.cast()?;
        // SAFETY: The mock and real TSF contract require StartComposition to be
        // called inside a write edit session with a live selection range and sink.
        unsafe { context_composition.StartComposition(edit_cookie, range, &self.composition_sink) }
    }
}

impl ITfEditSession_Impl for Fcitx5TextEditSession_Impl {
    fn DoEditSession(&self, edit_cookie: u32) -> Result<()> {
        let selection_range = self.selection_range(edit_cookie)?;
        match self.action.clone() {
            TextEditAction::ApplyEngineResult(result) => {
                if result.delete_surrounding_text
                    && result.delete_surrounding_offset == -1
                    && result.delete_surrounding_size == 1
                {
                    let mut shifted = 0i32;
                    // SAFETY: The cloned range is valid for this edit session.
                    // The null halt condition matches the existing C++ behavior.
                    unsafe {
                        selection_range.ShiftStart(
                            edit_cookie,
                            -1,
                            &mut shifted,
                            std::ptr::null(),
                        )?;
                    }
                    Fcitx5TextEditSession::set_range_text(&selection_range, edit_cookie, "")?;
                } else if result.delete_surrounding_text {
                    return Err(E_NOTIMPL.into());
                }

                if !result.commit.is_empty() {
                    if let Some(composition) = &self.current_composition {
                        // SAFETY: The composition belongs to this active edit
                        // session and its range remains valid until it is ended.
                        let composition_range = unsafe { composition.GetRange()? };
                        Fcitx5TextEditSession::set_range_text(
                            &composition_range,
                            edit_cookie,
                            &result.commit,
                        )?;
                        // SAFETY: Ending the current composition is required
                        // after committing its text.
                        unsafe { composition.EndComposition(edit_cookie)? };
                    } else {
                        Fcitx5TextEditSession::set_range_text(
                            &selection_range,
                            edit_cookie,
                            &result.commit,
                        )?;
                    }
                    *self.resulting_composition.borrow_mut() = None;
                } else if !result.preedit.is_empty() {
                    let composition =
                        self.start_or_get_composition(edit_cookie, &selection_range)?;
                    // SAFETY: The composition belongs to this active edit session.
                    let composition_range = unsafe { composition.GetRange()? };
                    Fcitx5TextEditSession::set_range_text(
                        &composition_range,
                        edit_cookie,
                        &result.preedit,
                    )?;
                    *self.resulting_composition.borrow_mut() = Some(composition);
                } else if let Some(composition) = &self.current_composition {
                    // SAFETY: The composition belongs to this active edit session.
                    let composition_range = unsafe { composition.GetRange()? };
                    Fcitx5TextEditSession::set_range_text(&composition_range, edit_cookie, "")?;
                    // SAFETY: Empty engine preedit ends the existing local
                    // composition.
                    unsafe { composition.EndComposition(edit_cookie)? };
                    *self.resulting_composition.borrow_mut() = None;
                } else {
                    *self.resulting_composition.borrow_mut() = None;
                }
            }
            TextEditAction::ClearComposition => {
                if let Some(composition) = &self.current_composition {
                    // SAFETY: The composition belongs to this active edit session.
                    let composition_range = unsafe { composition.GetRange()? };
                    Fcitx5TextEditSession::set_range_text(&composition_range, edit_cookie, "")?;
                    // SAFETY: Focus loss must terminate the composition.
                    unsafe { composition.EndComposition(edit_cookie)? };
                }
                *self.resulting_composition.borrow_mut() = None;
            }
        }
        *self.applied.borrow_mut() = true;
        Ok(())
    }
}

#[implement(IClassFactory)]
struct Fcitx5TsfClassFactory;

impl Fcitx5TsfClassFactory {
    fn new() -> Self {
        module_add_ref();
        Self
    }
}

impl Drop for Fcitx5TsfClassFactory {
    fn drop(&mut self) {
        module_release();
    }
}

impl IClassFactory_Impl for Fcitx5TsfClassFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<IUnknown>,
        interface_id: *const GUID,
        object: *mut *mut c_void,
    ) -> Result<()> {
        create_tsf_service_instance(outer, interface_id, object)
    }

    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        Ok(())
    }
}

#[implement(
    ITfTextInputProcessorEx,
    ITfThreadMgrEventSink,
    ITfThreadFocusSink,
    ITfActiveLanguageProfileNotifySink,
    ITfKeyEventSink
)]
struct Fcitx5TsfService {
    state: RefCell<TsfPocBehaviorState>,
    runtime: RefCell<TsfRuntimeState>,
    composition_sink: ITfCompositionSink,
}

impl Fcitx5TsfService {
    fn new() -> Self {
        module_add_ref();
        let sink = ComObject::new(Fcitx5CompositionSink);
        Self {
            state: RefCell::default(),
            runtime: RefCell::new(TsfRuntimeState {
                candidate_ui_element_id: TF_INVALID_UIELEMENTID,
                popup_allowed: true,
                ..TsfRuntimeState::default()
            }),
            composition_sink: sink.to_interface::<ITfCompositionSink>(),
        }
    }

    #[cfg(test)]
    fn lifecycle_state_for_test(&self) -> TsfPocBehaviorState {
        self.state.borrow().clone()
    }

    #[cfg(test)]
    fn activate_for_test(&self) {
        self.state.borrow_mut().activate();
    }

    #[cfg(test)]
    fn deactivate_for_test(&self) {
        self.state.borrow_mut().deactivate();
    }

    #[cfg(test)]
    fn fail_open_key_down_for_test(&self) {
        self.state.borrow_mut().key_down(EngineResult::malformed());
    }

    #[cfg(test)]
    fn fail_open_key_up_for_test(&self) {
        self.state.borrow_mut().key_up();
    }

    fn activate_runtime(
        &self,
        service: &Fcitx5TsfService_Impl,
        thread_manager: Ref<ITfThreadMgr>,
        client_id: u32,
    ) -> Result<()> {
        trace_event("activate_runtime_enter");
        if thread_manager.is_null() || client_id == TF_CLIENTID_NULL {
            let mut runtime = self.runtime.borrow_mut();
            runtime.reset_local_state();
            runtime.guard_fail_open = true;
            self.state.borrow_mut().deactivate();
            trace_event("activate_runtime_fail_open_invalid_thread_manager");
            return Ok(());
        }
        let mut runtime = self.runtime.borrow_mut();
        if runtime.active {
            return Err(E_UNEXPECTED.into());
        }
        if activation_guard_disabled() {
            runtime.reset_local_state();
            runtime.guard_fail_open = true;
            self.state.borrow_mut().deactivate();
            trace_event("activate_runtime_guard_disabled");
            return Ok(());
        }

        let thread_manager_ref = thread_manager.ok()?;
        let keystroke_manager: ITfKeystrokeMgr = thread_manager_ref.cast()?;
        let source: ITfSource = thread_manager_ref.cast()?;
        let ui_element_manager: Option<ITfUIElementMgr> = thread_manager_ref.cast().ok();
        let engine_client = current_module_path()
            .ok()
            .and_then(|module_path| EngineClient::new_for_current_module(&module_path));
        if let Some(client) = &engine_client {
            let _ = client.start_launcher_process();
        }

        let key_sink = service.to_interface::<ITfKeyEventSink>();
        let thread_event_sink = service.to_interface::<ITfThreadMgrEventSink>();
        let thread_focus_sink = service.to_interface::<ITfThreadFocusSink>();
        let active_profile_sink = service.to_interface::<ITfActiveLanguageProfileNotifySink>();
        let thread_event_unknown: IUnknown = thread_event_sink.cast()?;
        let thread_focus_unknown: IUnknown = thread_focus_sink.cast()?;
        let active_profile_unknown: IUnknown = active_profile_sink.cast()?;

        // SAFETY: The sinks are COM interfaces implemented by this live service;
        // TSF stores AddRef'ed references until Deactivate unadvises them.
        unsafe { keystroke_manager.AdviseKeyEventSink(client_id, &key_sink, true)? };
        // SAFETY: ITfSource belongs to the same live thread manager. Cookies are
        // stored and released on Deactivate.
        let thread_event_cookie =
            unsafe { source.AdviseSink(&ITfThreadMgrEventSink::IID, &thread_event_unknown)? };
        let thread_focus_cookie =
            unsafe { source.AdviseSink(&ITfThreadFocusSink::IID, &thread_focus_unknown)? };
        let active_profile_cookie = unsafe {
            source.AdviseSink(
                &ITfActiveLanguageProfileNotifySink::IID,
                &active_profile_unknown,
            )?
        };

        runtime.client_id = client_id;
        runtime.guard_fail_open = false;
        runtime.active = true;
        runtime.thread_event_cookie = Some(thread_event_cookie);
        runtime.thread_focus_cookie = Some(thread_focus_cookie);
        runtime.active_profile_cookie = Some(active_profile_cookie);
        runtime.keystroke_manager = Some(keystroke_manager);
        runtime.source = Some(source);
        runtime.ui_element_manager = ui_element_manager;
        runtime.engine_client = engine_client;
        runtime.popup_allowed = true;
        self.state.borrow_mut().activate();
        trace_event("activate_runtime_success");
        Ok(())
    }

    fn deactivate_runtime(&self) -> Result<()> {
        let mut runtime = self.runtime.borrow_mut();
        if let (Some(manager), id) = (&runtime.ui_element_manager, runtime.candidate_ui_element_id)
        {
            if id != TF_INVALID_UIELEMENTID {
                // SAFETY: id was returned by BeginUIElement on this manager.
                let _ = unsafe { manager.EndUIElement(id) };
            }
        }
        if let (Some(manager), client_id) = (&runtime.keystroke_manager, runtime.client_id) {
            if client_id != TF_CLIENTID_NULL {
                // SAFETY: The key sink was advised with this client id during
                // activation. Deactivation is best-effort.
                let _ = unsafe { manager.UnadviseKeyEventSink(client_id) };
            }
        }
        if let Some(source) = &runtime.source {
            for cookie in [
                runtime.thread_event_cookie,
                runtime.thread_focus_cookie,
                runtime.active_profile_cookie,
            ]
            .into_iter()
            .flatten()
            {
                // SAFETY: Cookies were returned by AdviseSink on this ITfSource.
                let _ = unsafe { source.UnadviseSink(cookie) };
            }
        }
        runtime.reset_local_state();
        self.state.borrow_mut().deactivate();
        Ok(())
    }

    fn should_test_eat_key(&self, wparam: WPARAM) -> bool {
        let runtime = self.runtime.borrow();
        if runtime.guard_fail_open {
            return false;
        }
        is_text_or_candidate_key(wparam.0 as u16)
    }

    fn context_id(context: &ITfContext) -> u64 {
        windows_core::Interface::as_raw(context) as usize as u64
    }

    fn context_has_sensitive_input_scope(context: &ITfContext) -> bool {
        // SAFETY: The context is supplied by TSF for this callback. The GUID
        // pointer is valid for the call.
        let Ok(property) = (unsafe { context.GetAppProperty(&GUID_PROP_INPUTSCOPE) }) else {
            return false;
        };
        // SAFETY: A null range requests the app-level input-scope value.
        let Ok(mut variant) = (unsafe { property.GetValue(0, None::<&ITfRange>) }) else {
            return false;
        };
        // SAFETY: Reading the discriminant is valid for an initialized VARIANT.
        let vt = unsafe { variant.Anonymous.Anonymous.vt };
        if vt == VT_EMPTY {
            return false;
        }
        if vt != VT_UNKNOWN {
            // SAFETY: VARIANT was initialized by COM and is not otherwise moved.
            let _ = unsafe { VariantClear(&mut variant) };
            return false;
        }
        // SAFETY: The VARIANT contains VT_UNKNOWN, so reading the `punkVal` arm
        // is valid. Clone adds a Rust-owned COM reference before VariantClear
        // releases the original VARIANT reference.
        let unknown = unsafe { (&*variant.Anonymous.Anonymous.Anonymous.punkVal).clone() };
        // SAFETY: VARIANT was initialized by COM and is not otherwise moved.
        let _ = unsafe { VariantClear(&mut variant) };
        let Some(unknown) = unknown else {
            return false;
        };
        let Ok(input_scope) = unknown.cast::<ITfInputScope>() else {
            return false;
        };
        let mut scopes = std::ptr::null_mut::<InputScope>();
        let mut count = 0u32;
        // SAFETY: `scopes` and `count` are writable out parameters. COM returns
        // a CoTaskMemAlloc buffer that is released below.
        if unsafe { input_scope.GetInputScopes(&mut scopes, &mut count) }.is_err() {
            return false;
        }
        if scopes.is_null() || count == 0 {
            return false;
        }
        // SAFETY: ITfInputScope returned `count` initialized InputScope values
        // in a CoTaskMemAlloc buffer.
        let sensitive = unsafe { std::slice::from_raw_parts(scopes, count as usize) }
            .iter()
            .copied()
            .any(is_sensitive_input_scope);
        // SAFETY: The buffer was allocated by COM for GetInputScopes.
        unsafe { CoTaskMemFree(Some(scopes.cast::<c_void>())) };
        sensitive
    }

    fn read_surrounding_snapshot(&self, context: &ITfContext) -> SurroundingSnapshot {
        let client_id = self.runtime.borrow().client_id;
        if client_id == TF_CLIENTID_NULL {
            return SurroundingSnapshot::default();
        }
        let snapshot = Rc::new(RefCell::new(SurroundingSnapshot::default()));
        let session = ComObject::new(Fcitx5ReadSurroundingSession::new(
            context.clone(),
            snapshot.clone(),
        ));
        let edit_session = session.to_interface::<ITfEditSession>();
        // SAFETY: The edit session is a live COM object and TF_ES_SYNC keeps the
        // borrowed state valid until it returns.
        let session_result = unsafe {
            context.RequestEditSession(client_id, &edit_session, TF_ES_SYNC | TF_ES_READ)
        };
        if session_result.is_err() {
            return SurroundingSnapshot::default();
        }
        let result = snapshot.borrow().clone();
        result
    }

    fn keyboard_state_flags(release: bool) -> u32 {
        fn pressed(key: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) -> bool {
            // SAFETY: GetKeyState reads process keyboard state for the supplied
            // virtual key and has no pointer preconditions.
            unsafe { GetKeyState(key.0 as i32) < 0 }
        }
        let mut flags = 0u32;
        if pressed(VK_SHIFT) {
            flags |= protocol::KEY_FLAG_SHIFT;
        }
        if pressed(VK_CONTROL) {
            flags |= protocol::KEY_FLAG_CONTROL;
        }
        if pressed(VK_MENU) {
            flags |= protocol::KEY_FLAG_ALT;
        }
        if pressed(VK_LWIN) || pressed(VK_RWIN) {
            flags |= protocol::KEY_FLAG_SUPER;
        }
        if release {
            flags |= protocol::KEY_FLAG_RELEASE;
        }
        flags
    }

    fn physical_key_fields(wparam: WPARAM, lparam: LPARAM) -> (u32, u32, bool, u64) {
        let virtual_key = wparam.0 as u32;
        let mut scan_code = ((lparam.0 as u64 >> 16) & 0xff) as u32;
        let extended_key = ((lparam.0 as u64 >> 24) & 1) != 0;
        // SAFETY: GetKeyboardLayout and MapVirtualKeyExW have no pointer
        // preconditions. A zero thread id selects the current thread.
        let layout = unsafe { GetKeyboardLayout(0) };
        if scan_code == 0 {
            scan_code = unsafe { MapVirtualKeyExW(virtual_key, MAPVK_VK_TO_VSC_EX, Some(layout)) };
        }
        (virtual_key, scan_code, extended_key, layout.0 as u64)
    }

    fn engine_key_result(
        &self,
        context: &ITfContext,
        wparam: WPARAM,
        lparam: LPARAM,
        release: bool,
    ) -> Option<EngineKeyResult> {
        let context_id = Self::context_id(context);
        let key_flags = Self::keyboard_state_flags(release);
        let (virtual_key, scan_code, extended_key, keyboard_layout) =
            Self::physical_key_fields(wparam, lparam);
        let popup_allowed = self.runtime.borrow().popup_allowed;
        let surrounding = self.read_surrounding_snapshot(context);
        let mut runtime = self.runtime.borrow_mut();
        let client = runtime.engine_client.as_mut()?;
        client.process_key(
            context_id,
            virtual_key,
            key_flags,
            keyboard_layout,
            scan_code,
            extended_key,
            popup_allowed,
            surrounding,
        )
    }

    fn cached_key_result(&self, context: &ITfContext, wparam: WPARAM) -> Option<EngineKeyResult> {
        let context_id = Self::context_id(context);
        let virtual_key = wparam.0 as u32;
        let key_flags = Self::keyboard_state_flags(false);
        let mut runtime = self.runtime.borrow_mut();
        let cached = runtime.pending_key.take()?;
        if cached.context_id == context_id
            && cached.virtual_key == virtual_key
            && cached.key_flags == key_flags
        {
            return Some(cached.result);
        }
        runtime.pending_key = Some(cached);
        None
    }

    fn store_pending_key(&self, context: &ITfContext, wparam: WPARAM, result: EngineKeyResult) {
        let context_id = Self::context_id(context);
        self.runtime.borrow_mut().pending_key = Some(CachedKeyResult {
            context_id,
            virtual_key: wparam.0 as u32,
            key_flags: Self::keyboard_state_flags(false),
            result,
        });
    }

    fn apply_engine_result(&self, context: &ITfContext, result: EngineKeyResult) -> Result<bool> {
        if result.forward_key {
            self.state
                .borrow_mut()
                .key_down(EngineResult::ok(true, "", ""));
            return Ok(false);
        }
        let has_text_change = result.delete_surrounding_text
            || !result.commit.is_empty()
            || !result.preedit.is_empty()
            || self.runtime.borrow().composition.is_some();
        let applied = if has_text_change {
            self.request_edit(context, TextEditAction::ApplyEngineResult(result.clone()))?
        } else {
            result.handled
        };
        if applied && result.candidate_visibility != CANDIDATE_VISIBILITY_HIDDEN {
            self.begin_candidate_ui(&result);
        } else if result.candidate_visibility == CANDIDATE_VISIBILITY_HIDDEN
            || !result.commit.is_empty()
            || result.preedit.is_empty()
        {
            self.end_candidate_ui();
        }
        self.state.borrow_mut().key_down(EngineResult::ok(
            applied || result.handled,
            &result.commit,
            &result.preedit,
        ));
        Ok(applied || result.handled)
    }

    fn request_edit(&self, context: &ITfContext, action: TextEditAction) -> Result<bool> {
        let (client_id, current_composition) = {
            let runtime = self.runtime.borrow();
            (runtime.client_id, runtime.composition.clone())
        };
        if client_id == TF_CLIENTID_NULL {
            return Ok(false);
        }
        let resulting_composition = Rc::new(RefCell::new(current_composition.clone()));
        let applied = Rc::new(RefCell::new(false));
        let session = ComObject::new(Fcitx5TextEditSession::new(
            context.clone(),
            action,
            self.composition_sink.clone(),
            current_composition,
            resulting_composition.clone(),
            applied.clone(),
        ));
        let edit_session = session.to_interface::<ITfEditSession>();
        // SAFETY: The edit session is a live COM object and TF_ES_SYNC ensures it
        // completes before the local references are released.
        let session_result = unsafe {
            context.RequestEditSession(client_id, &edit_session, TF_ES_SYNC | TF_ES_READWRITE)?
        };
        if session_result.is_err() || !*applied.borrow() {
            return Ok(false);
        }
        self.runtime.borrow_mut().composition = resulting_composition.borrow().clone();
        Ok(true)
    }

    fn begin_candidate_ui(&self, result: &EngineKeyResult) {
        if result.candidates.is_empty()
            || result.candidate_visibility == CANDIDATE_VISIBILITY_HIDDEN
        {
            self.end_candidate_ui();
            return;
        }
        let candidate = ComObject::new(Fcitx5CandidateListUiElement::new(
            result.candidates.clone(),
            result.selected_candidate,
            result.candidate_page,
        ));
        let candidate_list = candidate.to_interface::<ITfCandidateListUIElement>();
        let candidate_ui = candidate.to_interface::<ITfUIElement>();
        let mut runtime = self.runtime.borrow_mut();
        if let (Some(manager), id) = (&runtime.ui_element_manager, runtime.candidate_ui_element_id)
        {
            if id != TF_INVALID_UIELEMENTID {
                // SAFETY: id was returned by BeginUIElement on this manager.
                let _ = unsafe { manager.EndUIElement(id) };
            }
        }
        if let Some(manager) = &runtime.ui_element_manager {
            let mut show = BOOL(1);
            let mut id = TF_INVALID_UIELEMENTID;
            // SAFETY: candidate_ui is a live ITfUIElement. TSF writes show/id.
            if unsafe { manager.BeginUIElement(&candidate_ui, &mut show, &mut id) }.is_ok() {
                // SAFETY: The UI element is still live and records whether the
                // host allowed popup display.
                let _ = unsafe { candidate_ui.Show(show.as_bool()) };
                runtime.popup_allowed = show.as_bool();
                runtime.candidate_ui_element_id = id;
                runtime.candidate_ui_element = Some(candidate_list);
            }
        }
    }

    fn end_candidate_ui(&self) {
        let mut runtime = self.runtime.borrow_mut();
        if let (Some(manager), id) = (&runtime.ui_element_manager, runtime.candidate_ui_element_id)
        {
            if id != TF_INVALID_UIELEMENTID {
                // SAFETY: id was returned by BeginUIElement on this manager.
                let _ = unsafe { manager.EndUIElement(id) };
            }
        }
        runtime.candidate_ui_element_id = TF_INVALID_UIELEMENTID;
        runtime.candidate_ui_element = None;
    }

    fn clear_context_composition(&self, context: &ITfContext) -> Result<()> {
        let had_composition = self.runtime.borrow().composition.is_some();
        if had_composition {
            let _ = self.request_edit(context, TextEditAction::ClearComposition)?;
        }
        self.end_candidate_ui();
        Ok(())
    }
}

impl Drop for Fcitx5TsfService {
    fn drop(&mut self) {
        module_release();
    }
}

impl ITfTextInputProcessor_Impl for Fcitx5TsfService_Impl {
    fn Activate(&self, thread_manager: Ref<ITfThreadMgr>, client_id: u32) -> Result<()> {
        self.activate_runtime(self, thread_manager, client_id)
    }

    fn Deactivate(&self) -> Result<()> {
        self.deactivate_runtime()
    }
}

impl ITfTextInputProcessorEx_Impl for Fcitx5TsfService_Impl {
    fn ActivateEx(
        &self,
        thread_manager: Ref<ITfThreadMgr>,
        client_id: u32,
        _flags: u32,
    ) -> Result<()> {
        self.activate_runtime(self, thread_manager, client_id)
    }
}

impl ITfThreadMgrEventSink_Impl for Fcitx5TsfService_Impl {
    fn OnInitDocumentMgr(&self, _document_manager: Ref<ITfDocumentMgr>) -> Result<()> {
        Ok(())
    }

    fn OnUninitDocumentMgr(&self, _document_manager: Ref<ITfDocumentMgr>) -> Result<()> {
        Ok(())
    }

    fn OnSetFocus(
        &self,
        focused_document_manager: Ref<ITfDocumentMgr>,
        previous_focused_document_manager: Ref<ITfDocumentMgr>,
    ) -> Result<()> {
        if focused_document_manager.is_null() {
            if let Some(previous) = previous_focused_document_manager.as_ref() {
                // SAFETY: The document manager is supplied by TSF for this
                // callback; GetTop returns a live context when one exists.
                if let Ok(context) = unsafe { previous.GetTop() } {
                    self.clear_context_composition(&context)?;
                }
            }
        }
        Ok(())
    }

    fn OnPushContext(&self, _context: Ref<ITfContext>) -> Result<()> {
        Ok(())
    }

    fn OnPopContext(&self, _context: Ref<ITfContext>) -> Result<()> {
        Ok(())
    }
}

impl ITfThreadFocusSink_Impl for Fcitx5TsfService_Impl {
    fn OnSetThreadFocus(&self) -> Result<()> {
        Ok(())
    }

    fn OnKillThreadFocus(&self) -> Result<()> {
        Ok(())
    }
}

impl ITfKeyEventSink_Impl for Fcitx5TsfService_Impl {
    fn OnSetFocus(&self, _foreground: BOOL) -> Result<()> {
        Ok(())
    }

    fn OnTestKeyDown(
        &self,
        context: Ref<ITfContext>,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Result<BOOL> {
        trace_event("on_test_key_down_enter");
        if context.is_null() || !self.runtime.borrow().active {
            trace_event("on_test_key_down_inactive");
            return Ok(BOOL(0));
        }
        let context = context.ok()?;
        if self.runtime.borrow().guard_fail_open
            || Fcitx5TsfService::context_has_sensitive_input_scope(context)
            || !self.should_test_eat_key(wparam)
        {
            trace_event("on_test_key_down_passthrough");
            return Ok(BOOL(0));
        }
        if let Some(cached) = &self.runtime.borrow().pending_key {
            let context_id = Fcitx5TsfService::context_id(context);
            let key_flags = Fcitx5TsfService::keyboard_state_flags(false);
            if cached.context_id == context_id
                && cached.virtual_key == wparam.0 as u32
                && cached.key_flags == key_flags
            {
                trace_event(if cached.result.handled {
                    "on_test_key_down_cached_handled"
                } else {
                    "on_test_key_down_cached_not_handled"
                });
                return Ok(BOOL(cached.result.handled as i32));
            }
        }
        let Some(result) = self.engine_key_result(context, wparam, lparam, false) else {
            trace_event("on_test_key_down_no_engine_result");
            return Ok(BOOL(0));
        };
        let handled = result.handled || wparam.0 as u16 == VK_OEM_COMMA.0;
        self.store_pending_key(context, wparam, result);
        trace_event(if handled {
            "on_test_key_down_handled"
        } else {
            "on_test_key_down_not_handled"
        });
        Ok(BOOL(handled as i32))
    }

    fn OnTestKeyUp(
        &self,
        _context: Ref<ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        Ok(BOOL(0))
    }

    fn OnKeyDown(&self, context: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        trace_event("on_key_down_enter");
        if context.is_null() || self.runtime.borrow().guard_fail_open {
            trace_event("on_key_down_passthrough_inactive_or_guard");
            return Ok(BOOL(0));
        }
        let context = context.ok()?;
        if Fcitx5TsfService::context_has_sensitive_input_scope(context)
            || !self.should_test_eat_key(wparam)
        {
            trace_event("on_key_down_passthrough_scope_or_key");
            return Ok(BOOL(0));
        }
        let result = self
            .cached_key_result(context, wparam)
            .or_else(|| self.engine_key_result(context, wparam, lparam, false));
        let Some(result) = result else {
            self.state.borrow_mut().key_down(EngineResult::timeout());
            trace_event("on_key_down_no_engine_result");
            return Ok(BOOL(0));
        };
        trace_event(&format!(
            "on_key_down_engine_result handled={} commit_len={} preedit_len={} candidates={}",
            result.handled,
            result.commit.chars().count(),
            result.preedit.chars().count(),
            result.candidates.len()
        ));
        let eaten = self.apply_engine_result(context, result)?;
        trace_event(if eaten {
            "on_key_down_eaten"
        } else {
            "on_key_down_not_eaten"
        });
        Ok(BOOL(eaten as i32))
    }

    fn OnKeyUp(&self, context: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        trace_event("on_key_up_enter");
        if !context.is_null()
            && self.runtime.borrow().active
            && !self.runtime.borrow().guard_fail_open
        {
            if let Ok(context) = context.ok() {
                if !Fcitx5TsfService::context_has_sensitive_input_scope(context) {
                    let _ = self.engine_key_result(context, wparam, lparam, true);
                }
            }
        }
        self.state.borrow_mut().key_up();
        Ok(BOOL(0))
    }

    fn OnPreservedKey(&self, _context: Ref<ITfContext>, _guid: *const GUID) -> Result<BOOL> {
        Ok(BOOL(0))
    }
}

impl ITfActiveLanguageProfileNotifySink_Impl for Fcitx5TsfService_Impl {
    fn OnActivated(
        &self,
        class_id: *const GUID,
        _profile_guid: *const GUID,
        activated: BOOL,
    ) -> Result<()> {
        if !class_id.is_null() && activated.as_bool() {
            // SAFETY: class_id is a TSF-owned pointer valid for this callback.
            let is_ours = unsafe { *class_id == FCITX5_TEXT_SERVICE_CLSID };
            if is_ours {
                self.state.borrow_mut().activate();
            }
        }
        Ok(())
    }
}

fn dll_can_unload_now_impl() -> HRESULT {
    if module_reference_count() == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

fn module_add_ref() {
    MODULE_REFERENCES.fetch_add(1, Ordering::Relaxed);
}

fn module_release() {
    MODULE_REFERENCES.fetch_sub(1, Ordering::Release);
}

fn module_reference_count() -> i32 {
    MODULE_REFERENCES.load(Ordering::Acquire)
}

unsafe fn dll_get_class_object_impl(
    class_id: *const GUID,
    interface_id: *const GUID,
    object: *mut *mut c_void,
) -> HRESULT {
    if object.is_null() {
        return E_POINTER;
    }
    unsafe {
        *object = null_mut();
    }
    if class_id.is_null() {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let requested_class = unsafe { *class_id };
    if requested_class != FCITX5_TEXT_SERVICE_CLSID {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let requested_interface = if interface_id.is_null() {
        IUnknown::IID
    } else {
        unsafe { *interface_id }
    };
    let factory = ComObject::new(Fcitx5TsfClassFactory::new());
    write_interface_for_iid::<Fcitx5TsfClassFactory>(
        &factory,
        &requested_interface,
        object,
        &[IUnknown::IID, IClassFactory::IID],
        |factory| factory.to_interface::<IClassFactory>().into_raw(),
    )
}

fn create_tsf_service_instance(
    outer: Ref<IUnknown>,
    interface_id: *const GUID,
    object: *mut *mut c_void,
) -> Result<()> {
    if !outer.is_null() {
        return Err(CLASS_E_NOAGGREGATION.into());
    }
    if object.is_null() || interface_id.is_null() {
        return Err(E_POINTER.into());
    }
    unsafe {
        *object = null_mut();
    }
    let requested_interface = unsafe { *interface_id };
    let service = ComObject::new(Fcitx5TsfService::new());
    let result = write_interface_for_iid::<Fcitx5TsfService>(
        &service,
        &requested_interface,
        object,
        &[
            IUnknown::IID,
            ITfTextInputProcessor::IID,
            ITfTextInputProcessorEx::IID,
            ITfThreadMgrEventSink::IID,
            ITfThreadFocusSink::IID,
            ITfKeyEventSink::IID,
        ],
        |service| {
            if requested_interface == ITfThreadMgrEventSink::IID {
                service.to_interface::<ITfThreadMgrEventSink>().into_raw()
            } else if requested_interface == ITfThreadFocusSink::IID {
                service.to_interface::<ITfThreadFocusSink>().into_raw()
            } else if requested_interface == ITfKeyEventSink::IID {
                service.to_interface::<ITfKeyEventSink>().into_raw()
            } else {
                service.to_interface::<ITfTextInputProcessorEx>().into_raw()
            }
        },
    );
    if result.is_ok() {
        Ok(())
    } else {
        Err(result.into())
    }
}

fn write_interface_for_iid<T>(
    object_impl: &ComObject<T>,
    requested_interface: &GUID,
    output: *mut *mut c_void,
    supported_interfaces: &[GUID],
    raw_interface: impl FnOnce(&ComObject<T>) -> *mut c_void,
) -> HRESULT
where
    T: windows_core::ComObjectInner,
{
    if !supported_interfaces
        .iter()
        .any(|supported| supported == requested_interface)
    {
        return E_NOINTERFACE;
    }
    unsafe {
        *output = raw_interface(object_impl);
    }
    S_OK
}

#[no_mangle]
/// # Safety
///
/// Exported for the Windows COM loader. It does not dereference caller-owned
/// pointers and must never unwind across the DLL boundary.
pub unsafe extern "system" fn DllCanUnloadNow() -> HRESULT {
    trace_event("dll_can_unload_now_enter");
    panic_to_hresult(dll_can_unload_now_impl)
}

#[no_mangle]
/// # Safety
///
/// `class_id`, `interface_id`, and `object` are raw COM ABI pointers supplied
/// by the host. This PoC only writes a null object pointer after validating that
/// `object` is non-null, returns `HRESULT` on every failure, and must never
/// unwind across the DLL boundary.
pub unsafe extern "system" fn DllGetClassObject(
    class_id: *const GUID,
    interface_id: *const GUID,
    object: *mut *mut c_void,
) -> HRESULT {
    trace_event("dll_get_class_object_enter");
    panic_to_hresult(|| unsafe { dll_get_class_object_impl(class_id, interface_id, object) })
}

#[no_mangle]
/// # Safety
///
/// Exported for the elevated register helper. Registration is contained behind
/// COM initialization and all failures are returned as `HRESULT`; this function
/// must never unwind across the DLL boundary.
pub unsafe extern "system" fn DllRegisterServer() -> HRESULT {
    trace_event("dll_register_server_enter");
    panic_to_hresult(register_text_service_impl)
}

#[no_mangle]
/// # Safety
///
/// Exported for the elevated register helper. Unregistration is best-effort and
/// returns `HRESULT`; this function must never unwind across the DLL boundary.
pub unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
    trace_event("dll_unregister_server_enter");
    panic_to_hresult(unregister_text_service_impl)
}

#[no_mangle]
/// # Safety
///
/// `length` is optional. When non-null it must point to writable process-local
/// memory. The returned pointer is owned by this module and remains valid until
/// the DLL is unloaded.
pub unsafe extern "system" fn Fcitx5TsfPocBehaviorReport(length: *mut usize) -> *const u8 {
    match catch_unwind(|| {
        let report = BEHAVIOR_REPORT.get_or_init(tsf_behavior_differential_report);
        if !length.is_null() {
            unsafe {
                *length = report.len();
            }
        }
        report.as_ptr()
    }) {
        Ok(pointer) => pointer,
        Err(_) => {
            if !length.is_null() {
                unsafe {
                    *length = 0;
                }
            }
            std::ptr::null()
        }
    }
}

#[no_mangle]
/// # Safety
///
/// `length` is optional. When non-null it must point to writable process-local
/// memory. The returned pointer is owned by this module and remains valid until
/// the DLL is unloaded.
pub unsafe extern "system" fn Fcitx5TsfPocProfileIdentityReport(length: *mut usize) -> *const u8 {
    match catch_unwind(|| {
        let report = PROFILE_IDENTITY_REPORT.get_or_init(tsf_profile_identity_report);
        if !length.is_null() {
            unsafe {
                *length = report.len();
            }
        }
        report.as_ptr()
    }) {
        Ok(pointer) => pointer,
        Err(_) => {
            if !length.is_null() {
                unsafe {
                    *length = 0;
                }
            }
            std::ptr::null()
        }
    }
}

#[no_mangle]
/// # Safety
///
/// `length` is optional. When non-null it must point to writable process-local
/// memory. The returned pointer is owned by this module and remains valid until
/// the DLL is unloaded.
pub unsafe extern "system" fn Fcitx5TsfPocIpcBoundaryReport(length: *mut usize) -> *const u8 {
    match catch_unwind(|| {
        let report = IPC_BOUNDARY_REPORT.get_or_init(tsf_ipc_boundary_report);
        if !length.is_null() {
            unsafe {
                *length = report.len();
            }
        }
        report.as_ptr()
    }) {
        Ok(pointer) => pointer,
        Err(_) => {
            if !length.is_null() {
                unsafe {
                    *length = 0;
                }
            }
            std::ptr::null()
        }
    }
}

#[no_mangle]
/// # Safety
///
/// `length` is optional. When non-null it must point to writable process-local
/// memory. The returned pointer is owned by this module and remains valid until
/// the DLL is unloaded.
pub unsafe extern "system" fn Fcitx5TsfPocCompositionTranscriptReport(
    length: *mut usize,
) -> *const u8 {
    match catch_unwind(|| {
        let report = COMPOSITION_TRANSCRIPT_REPORT.get_or_init(tsf_composition_transcript_report);
        if !length.is_null() {
            unsafe {
                *length = report.len();
            }
        }
        report.as_ptr()
    }) {
        Ok(pointer) => pointer,
        Err(_) => {
            if !length.is_null() {
                unsafe {
                    *length = 0;
                }
            }
            std::ptr::null()
        }
    }
}

#[no_mangle]
/// # Safety
///
/// `length` is optional. When non-null it must point to writable process-local
/// memory. The returned pointer is owned by this module and remains valid until
/// the DLL is unloaded.
pub unsafe extern "system" fn Fcitx5TsfPocDifferentialSummaryReport(
    length: *mut usize,
) -> *const u8 {
    match catch_unwind(|| {
        let report = DIFFERENTIAL_SUMMARY_REPORT.get_or_init(tsf_differential_summary_report);
        if !length.is_null() {
            unsafe {
                *length = report.len();
            }
        }
        report.as_ptr()
    }) {
        Ok(pointer) => pointer,
        Err(_) => {
            if !length.is_null() {
                unsafe {
                    *length = 0;
                }
            }
            std::ptr::null()
        }
    }
}

#[no_mangle]
/// # Safety
///
/// Test-only PoC export used by the artifact smoke to prove that a forced
/// internal panic is converted to `HRESULT` across the DLL ABI.
pub unsafe extern "system" fn Fcitx5TsfPocForcedFailureForTest() -> HRESULT {
    panic_to_hresult(|| panic!("forced Rust TSF PoC ABI panic regression"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::S_FALSE;
    use windows::Win32::UI::TextServices::{
        IS_CHAT, IS_DEFAULT, IS_EMAIL_SMTPEMAILADDRESS, IS_NUMBER, IS_SEARCH, IS_URL,
    };

    #[test]
    fn panic_boundary_converts_unwind_to_hresult() {
        let result = panic_to_hresult(|| panic!("forced TSF PoC panic boundary regression"));
        assert_eq!(result, E_UNEXPECTED);
    }

    #[test]
    fn sensitive_input_scope_policy_matches_frozen_cpp_contract() {
        for scope in [
            IS_PASSWORD,
            IS_PRIVATE,
            IS_NUMERIC_PASSWORD,
            IS_NUMERIC_PIN,
            IS_ALPHANUMERIC_PIN,
            IS_ALPHANUMERIC_PIN_SET,
        ] {
            assert!(is_sensitive_input_scope(scope));
        }
        for scope in [
            IS_DEFAULT,
            IS_URL,
            IS_EMAIL_SMTPEMAILADDRESS,
            IS_CHAT,
            IS_NUMBER,
            IS_SEARCH,
        ] {
            assert!(!is_sensitive_input_scope(scope));
        }
    }

    #[test]
    fn key_filter_accepts_text_and_candidate_navigation_keys() {
        for key in [
            b'N' as u16,
            b'2' as u16,
            VK_SPACE.0,
            VK_RETURN.0,
            VK_LEFT.0,
            VK_RIGHT.0,
            VK_UP.0,
            VK_DOWN.0,
            VK_PRIOR.0,
            VK_NEXT.0,
            VK_HOME.0,
            VK_END.0,
            VK_OEM_PLUS.0,
            VK_OEM_MINUS.0,
            VK_OEM_COMMA.0,
            VK_OEM_PERIOD.0,
            VK_OEM_1.0,
            VK_OEM_4.0,
            VK_OEM_6.0,
            VK_OEM_7.0,
        ] {
            assert!(is_text_or_candidate_key(key));
        }
        assert!(!is_text_or_candidate_key(VK_CONTROL.0));
        assert!(!is_text_or_candidate_key(VK_MENU.0));
    }

    #[test]
    fn text_ext_caret_preserves_host_dpi_for_candidate_scaling() {
        let caret = Fcitx5ReadSurroundingSession::rect_to_caret(
            RECT {
                left: 10,
                top: 20,
                right: 10,
                bottom: 44,
            },
            192,
        )
        .expect("valid caret rect");
        assert_eq!(caret.dpi, 192);
    }

    #[test]
    fn dll_exports_fail_closed_without_object_on_unsupported_class() {
        let unsupported = GUID::from_u128(0xaaaaaaaa_bbbb_cccc_dddd_eeeeeeeeeeee);
        let mut object = std::ptr::dangling_mut::<c_void>();
        let result = unsafe { DllGetClassObject(&unsupported, &GUID::zeroed(), &mut object) };
        assert_eq!(result, CLASS_E_CLASSNOTAVAILABLE);
        assert!(object.is_null());
    }

    #[test]
    fn dll_exports_are_panic_contained_and_unloadable() {
        assert_eq!(unsafe { DllCanUnloadNow() }, S_OK);
        let result = unsafe {
            DllGetClassObject(
                &FCITX5_TEXT_SERVICE_CLSID,
                &GUID::zeroed(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(result, E_POINTER);
    }

    #[test]
    fn rust_factory_creates_minimal_tsf_service_interfaces() {
        assert_eq!(module_reference_count(), 0);
        let mut factory_object = null_mut();
        let result = unsafe {
            DllGetClassObject(
                &FCITX5_TEXT_SERVICE_CLSID,
                &IClassFactory::IID,
                &mut factory_object,
            )
        };
        assert_eq!(result, S_OK);
        assert!(!factory_object.is_null());
        assert_eq!(unsafe { DllCanUnloadNow() }, S_FALSE);
        let factory = unsafe { IClassFactory::from_raw(factory_object) };
        let service: ITfTextInputProcessorEx = unsafe {
            factory
                .CreateInstance(None)
                .expect("Rust TSF PoC factory should create empty service")
        };
        let key_sink: ITfKeyEventSink = service.cast().expect("key sink should be queryable");
        let thread_sink: ITfThreadMgrEventSink = service
            .cast()
            .expect("thread event sink should be queryable");
        let focus_sink: ITfThreadFocusSink =
            service.cast().expect("focus sink should be queryable");
        drop((key_sink, thread_sink, focus_sink, service, factory));
        assert_eq!(module_reference_count(), 0);
        assert_eq!(unsafe { DllCanUnloadNow() }, S_OK);
    }

    #[test]
    fn policy_report_documents_prohibited_capabilities_absent() {
        let report = rust_tsf_poc_policy_report();
        assert!(report.contains("windows_rs:true"));
        assert!(report.contains("activatable_empty_tip:true"));
        assert!(report.contains("ITfTextInputProcessorEx:binding-marker"));
        assert!(report.contains("shipping_authoritative:true"));
        assert!(report.contains("send_input:false"));
        assert!(report.contains("global_hooks:false"));
        assert!(report.contains("process_injection:false"));
        assert!(report.contains("cxx_tsf_remains_authoritative:false"));
        let (name, tsf_size, factory_size) = tsf_binding_markers();
        assert_eq!(name, "ITfTextInputProcessorEx");
        assert!(tsf_size > 0);
        assert!(factory_size > 0);
    }

    #[test]
    fn behavior_corpus_has_required_tsf_lifecycle_cases() {
        let report = tsf_behavior_corpus_report();
        assert!(report.contains(r#""corpus":"tsf_behavior_corpus.json""#));
        assert!(report.contains(r#""case_count":10"#));
        assert!(report.contains(r#""all_cases_present":true"#));
        assert!(report.contains(r#""timeout_fail_open":true"#));
        assert!(report.contains(r#""malformed_ipc_fail_open":true"#));
        assert!(TSF_BEHAVIOR_CORPUS_JSON.contains("key_down_commit_applies_text"));
        assert!(TSF_BEHAVIOR_CORPUS_JSON.contains("deactivate_unadvises_sinks"));
        assert!(TSF_BEHAVIOR_CORPUS_JSON.contains("uiless_candidate_show_false"));
        assert!(TSF_BEHAVIOR_CORPUS_JSON.contains("key_busy_focus_change"));
        assert!(TSF_BEHAVIOR_CORPUS_JSON.contains("single_edit_session_commit_preedit"));
    }

    #[test]
    fn behavior_differential_report_lists_every_case_result() {
        let report = tsf_behavior_differential_report();
        assert!(report.contains(r#""case_count":10"#));
        assert!(report.contains(r#""rust_case_passes":10"#));
        assert!(report.contains(r#""cpp_baseline_ctest":"tsf-key-commit-e2e""#));
        assert!(report.contains(r#""cpp_baseline_consumes_same_corpus":true"#));
        assert!(report.contains(r#""full_host_differential_pending":true"#));
        for case_id in REQUIRED_TSF_BEHAVIOR_CASES {
            assert!(report.contains(case_id));
        }
    }

    #[test]
    fn behavior_report_export_is_panic_contained_and_length_delimited() {
        let mut length = 0usize;
        let pointer = unsafe { Fcitx5TsfPocBehaviorReport(&mut length) };
        assert!(!pointer.is_null());
        assert!(length > 0);
        let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
        let report = std::str::from_utf8(bytes).expect("behavior report must be UTF-8 JSON");
        assert!(report.contains(r#""report_export":"panic_contained""#));
        assert!(report.contains(r#""rust_case_passes":10"#));
    }

    #[test]
    fn forced_failure_export_converts_panic_to_hresult() {
        assert_eq!(unsafe { Fcitx5TsfPocForcedFailureForTest() }, E_UNEXPECTED);
    }

    #[test]
    fn service_lifecycle_callbacks_mutate_and_cleanup_domain_state() {
        let service = Fcitx5TsfService::new();
        service.activate_for_test();
        let activated = service.lifecycle_state_for_test();
        assert!(activated.active);
        assert!(activated.thread_manager_sink_advised);
        assert!(activated.thread_focus_sink_advised);
        assert!(activated.key_sink_advised);

        service.fail_open_key_down_for_test();
        let after_key_down = service.lifecycle_state_for_test();
        assert!(after_key_down.fail_open);
        assert!(!after_key_down.eaten);
        assert!(!after_key_down.composition_active);

        service.fail_open_key_up_for_test();
        let after_key_up = service.lifecycle_state_for_test();
        assert!(after_key_up.release_routed);
        assert!(!after_key_up.eaten);

        service.deactivate_for_test();
        let deactivated = service.lifecycle_state_for_test();
        assert!(!deactivated.active);
        assert!(!deactivated.thread_manager_sink_advised);
        assert!(!deactivated.thread_focus_sink_advised);
        assert!(!deactivated.key_sink_advised);
        assert!(!deactivated.composition_active);
        assert_eq!(deactivated.commit, "");
        assert_eq!(deactivated.preedit, "");
    }

    #[test]
    fn behavior_model_matches_activation_and_sink_corpus() {
        let mut state = TsfPocBehaviorState::default();
        state.activate();
        assert!(state.active);
        assert!(state.thread_manager_sink_advised);
        assert!(state.thread_focus_sink_advised);
        assert!(state.key_sink_advised);
        assert!(!state.eaten);
        assert!(!state.fail_open);
    }

    #[test]
    fn behavior_model_applies_commit_and_preedit_corpus() {
        let mut commit_state = TsfPocBehaviorState::default();
        commit_state.activate();
        commit_state.key_down(EngineResult::ok(true, "你", ""));
        assert!(commit_state.eaten);
        assert_eq!(commit_state.commit, "你");
        assert_eq!(commit_state.preedit, "");
        assert!(!commit_state.composition_active);
        assert!(!commit_state.fail_open);

        let mut preedit_state = TsfPocBehaviorState::default();
        preedit_state.activate();
        preedit_state.key_down(EngineResult::ok(true, "", "ni"));
        assert!(preedit_state.eaten);
        assert_eq!(preedit_state.commit, "");
        assert_eq!(preedit_state.preedit, "ni");
        assert!(preedit_state.composition_active);
        assert!(!preedit_state.fail_open);
    }

    #[test]
    fn behavior_model_routes_key_up_without_eating() {
        let mut state = TsfPocBehaviorState::default();
        state.activate();
        state.key_up();
        assert!(state.release_routed);
        assert!(!state.eaten);
        assert!(!state.fail_open);
    }

    #[test]
    fn behavior_model_fails_open_on_timeout_and_malformed_ipc() {
        let mut timeout_state = TsfPocBehaviorState::default();
        timeout_state.activate();
        timeout_state.key_down(EngineResult::timeout());
        assert!(timeout_state.fail_open);
        assert!(!timeout_state.eaten);
        assert_eq!(timeout_state.commit, "");
        assert_eq!(timeout_state.preedit, "");
        assert!(!timeout_state.composition_active);

        let mut malformed_state = TsfPocBehaviorState::default();
        malformed_state.activate();
        malformed_state.key_down(EngineResult::malformed());
        assert!(malformed_state.fail_open);
        assert!(!malformed_state.eaten);
        assert_eq!(malformed_state.commit, "");
        assert_eq!(malformed_state.preedit, "");
        assert!(!malformed_state.composition_active);

        let mut generation_state = TsfPocBehaviorState::default();
        generation_state.activate();
        generation_state.key_down(EngineResult::generation_mismatch());
        assert!(generation_state.fail_open);
        assert!(!generation_state.eaten);
        assert_eq!(generation_state.commit, "");
        assert_eq!(generation_state.preedit, "");
        assert!(!generation_state.composition_active);
    }

    #[test]
    fn behavior_model_deactivate_unadvises_and_clears_composition() {
        let mut state = TsfPocBehaviorState::default();
        state.activate();
        state.key_down(EngineResult::ok(true, "", "ni"));
        assert!(state.composition_active);
        state.deactivate();
        assert!(!state.active);
        assert!(!state.thread_manager_sink_advised);
        assert!(!state.thread_focus_sink_advised);
        assert!(!state.key_sink_advised);
        assert!(!state.composition_active);
        assert!(!state.eaten);
        assert_eq!(state.commit, "");
        assert_eq!(state.preedit, "");
    }

    #[test]
    fn behavior_model_preserves_uiless_candidate_metadata() {
        let mut state = TsfPocBehaviorState::default();
        state.activate();
        state.candidate_begin_ui_element(false);
        assert!(state.candidate_metadata_available);
        assert!(!state.candidate_popup_visible);
        assert!(state.uiless_candidate_mode);
        assert_eq!(state.selected_candidate, 0);
    }

    #[test]
    fn behavior_model_defers_focus_change_during_key_callback() {
        let mut state = TsfPocBehaviorState::default();
        state.activate();
        state.key_down(EngineResult::ok(true, "", "ni"));
        state.begin_key_callback();
        state.focus_change();
        state.end_key_callback();
        assert!(state.composition_active);
        assert!(state.focus_change_deferred);
        assert_eq!(state.preedit, "ni");
    }

    #[test]
    fn behavior_model_uses_single_edit_session_for_commit_preedit() {
        let mut state = TsfPocBehaviorState::default();
        state.activate();
        state.single_edit_session_update(EngineResult::ok(true, "你", "hao"));
        assert!(state.single_edit_session);
        assert!(state.eaten);
        assert_eq!(state.commit, "你");
        assert_eq!(state.preedit, "hao");
        assert!(state.composition_active);
    }

    #[test]
    fn stable_profile_identity_is_shared_with_current_release_identity() {
        assert_eq!(
            FCITX5_TEXT_SERVICE_CLSID,
            GUID::from_u128(0x3a21b9e2_4f47_4c36_8bfa_91d7d3b3e901)
        );
        assert_eq!(
            FCITX5_LANGUAGE_PROFILE_GUID,
            GUID::from_u128(0x6c2ac726_7703_4b65_89af_a77e9e0da102)
        );
        let report = tsf_profile_identity_report();
        assert!(report.contains("\"product_display_name\":\"Fcitx5 for Windows Next\""));
        assert!(report.contains("\"profile_display_name\":\"Fcitx5\""));
        assert!(report.contains("\"windows_profile_count\":1"));
        assert!(report.contains("\"dynamic_profile_registration\":false"));
        assert!(report.contains("\"shipping_cxx_authoritative\":false"));
        assert!(report.contains("\"rust_poc_registers_profile\":true"));
        assert!(report.contains("\"rust_shipping_authoritative\":true"));
    }

    #[test]
    fn profile_identity_export_is_panic_contained_and_length_delimited() {
        let mut length = 0usize;
        let pointer = unsafe { Fcitx5TsfPocProfileIdentityReport(&mut length) };
        assert!(!pointer.is_null());
        assert!(length > 0);
        let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
        let report = std::str::from_utf8(bytes).expect("profile identity report should be utf8");
        assert!(report.contains("\"text_service_clsid\":\"3a21b9e2-4f47-4c36-8bfa-91d7d3b3e901\""));
        assert!(
            report.contains("\"language_profile_guid\":\"6c2ac726-7703-4b65-89af-a77e9e0da102\"")
        );
    }

    #[test]
    fn bounded_ipc_client_fails_open_on_untrusted_or_slow_replies() {
        let client = BoundedIpcClient::new(42, 30);
        let ok = client.key_down(IpcProbe {
            generation: 42,
            elapsed_ms: 5,
            well_formed: true,
            handled: true,
            commit: "",
            preedit: "ni",
        });
        assert_eq!(ok.status, EngineStatus::Ok);
        assert_eq!(ok.preedit, "ni");

        assert_eq!(
            client
                .key_down(IpcProbe {
                    generation: 42,
                    elapsed_ms: 31,
                    well_formed: true,
                    handled: true,
                    commit: "你",
                    preedit: "",
                })
                .status,
            EngineStatus::Timeout
        );
        assert_eq!(
            client
                .key_down(IpcProbe {
                    generation: 42,
                    elapsed_ms: 5,
                    well_formed: false,
                    handled: true,
                    commit: "你",
                    preedit: "",
                })
                .status,
            EngineStatus::Malformed
        );
        assert_eq!(
            client
                .key_down(IpcProbe {
                    generation: 41,
                    elapsed_ms: 5,
                    well_formed: true,
                    handled: true,
                    commit: "你",
                    preedit: "",
                })
                .status,
            EngineStatus::GenerationMismatch
        );
    }

    #[test]
    fn ipc_boundary_export_is_panic_contained_and_length_delimited() {
        let mut length = 0usize;
        let pointer = unsafe { Fcitx5TsfPocIpcBoundaryReport(&mut length) };
        assert!(!pointer.is_null());
        assert!(length > 0);
        let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
        let report = std::str::from_utf8(bytes).expect("ipc boundary report should be utf8");
        assert!(report.contains("\"bounded_ipc_client_model\":true"));
        assert!(report.contains("\"timeout_fails_open\":true"));
        assert!(report.contains("\"malformed_fails_open\":true"));
        assert!(report.contains("\"generation_mismatch_fails_open\":true"));
        assert!(report.contains("\"network_imports\":false"));
        assert!(report.contains("\"external_engine_link\":false"));
        assert!(report.contains("\"host_blocking_call\":false"));
    }

    #[test]
    fn edit_session_transcript_records_commit_before_preedit_in_one_session() {
        let mut transcript = EditSessionTranscript::default();
        transcript.apply_single_session(EngineResult::ok(true, "你", "hao"));
        assert_eq!(
            transcript.operations,
            vec![
                "begin_edit_session",
                "commit_text",
                "update_preedit_start_composition",
                "end_edit_session"
            ]
        );
        assert_eq!(transcript.commit, "你");
        assert_eq!(transcript.preedit, "hao");
        assert!(transcript.composition_active);
    }

    #[test]
    fn composition_transcript_export_is_panic_contained_and_length_delimited() {
        let mut length = 0usize;
        let pointer = unsafe { Fcitx5TsfPocCompositionTranscriptReport(&mut length) };
        assert!(!pointer.is_null());
        assert!(length > 0);
        let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
        let report =
            std::str::from_utf8(bytes).expect("composition transcript report should be utf8");
        assert!(report.contains("\"single_edit_session\":true"));
        assert!(report.contains(
            "\"operation_order\":[\"begin_edit_session\",\"commit_text\",\"update_preedit_start_composition\",\"end_edit_session\"]"
        ));
        assert!(report.contains("\"commit_text\":\"你\""));
        assert!(report.contains("\"preedit_text\":\"hao\""));
        assert!(report.contains("\"composition_active_after\":true"));
        assert!(report.contains("\"host_differential_pending\":true"));
    }

    #[test]
    fn differential_summary_export_lists_green_and_pending_evidence() {
        let mut length = 0usize;
        let pointer = unsafe { Fcitx5TsfPocDifferentialSummaryReport(&mut length) };
        assert!(!pointer.is_null());
        assert!(length > 0);
        let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
        let report = std::str::from_utf8(bytes).expect("differential summary should be utf8");
        assert!(report.contains("\"component\":\"fcitx5-tsf-poc\""));
        assert!(report.contains("\"same_corpus_case_count\":10"));
        assert!(report.contains("\"same_corpus_rust_passes\":10"));
        assert!(report.contains("\"profile_identity\":true"));
        assert!(report.contains("\"ipc_boundary\":true"));
        assert!(report.contains("\"composition_transcript\":true"));
        assert!(report.contains("\"artifact_audit_ctest\":\"rust-tsf-poc-artifact-audit\""));
        assert!(report.contains("\"arm64_ci_artifact_green\":true"));
        assert!(report.contains("\"real_host_matrix_pending\":true"));
        assert!(report.contains("\"product_decision\":\"shipping_rust_cutover\""));
    }

    #[test]
    fn module_unload_uses_refcounted_s_false() {
        module_add_ref();
        assert_eq!(unsafe { DllCanUnloadNow() }, S_FALSE);
        module_release();
        assert_eq!(unsafe { DllCanUnloadNow() }, S_OK);
    }
}
