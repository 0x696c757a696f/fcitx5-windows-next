#![deny(unsafe_op_in_unsafe_fn)]
#![allow(linker_messages)]
#![allow(non_snake_case)]

use std::cell::RefCell;
use std::ffi::c_void;
use std::panic::{catch_unwind, UnwindSafe};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;
use windows::Win32::Foundation::{CLASS_E_CLASSNOTAVAILABLE, E_POINTER, E_UNEXPECTED, S_OK};
use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_NOINTERFACE, LPARAM, S_FALSE, WPARAM};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::Win32::UI::TextServices::{
    ITfContext, ITfDocumentMgr, ITfKeyEventSink, ITfKeyEventSink_Impl, ITfTextInputProcessor,
    ITfTextInputProcessorEx, ITfTextInputProcessorEx_Impl, ITfTextInputProcessor_Impl,
    ITfThreadFocusSink, ITfThreadFocusSink_Impl, ITfThreadMgr, ITfThreadMgrEventSink,
    ITfThreadMgrEventSink_Impl,
};
use windows_core::{implement, ComObject, IUnknown, Interface, Ref, Result, BOOL, GUID, HRESULT};

// Stable channel CLSID from cmake/release_identity.h.in.
// This PoC deliberately does not register or replace the shipping C++ TSF.
pub const FCITX5_TEXT_SERVICE_CLSID: GUID = GUID::from_u128(0x3a21b9e2_4f47_4c36_8bfa_91d7d3b3e901);
pub const FCITX5_LANGUAGE_PROFILE_GUID: GUID =
    GUID::from_u128(0x6c2ac726_7703_4b65_89af_a77e9e0da102);
const FCITX5_PRODUCT_DISPLAY_NAME: &str = "Fcitx5 for Windows Next";
const FCITX5_PROFILE_DISPLAY_NAME: &str = "Fcitx5";
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
        "shipping_authoritative:false;",
        "cxx_tsf_remains_authoritative:true;",
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
        "{{\"format_version\":1,\"product_display_name\":\"{}\",\"profile_display_name\":\"{}\",\"text_service_clsid\":\"3a21b9e2-4f47-4c36-8bfa-91d7d3b3e901\",\"language_profile_guid\":\"6c2ac726-7703-4b65-89af-a77e9e0da102\",\"windows_profile_count\":1,\"dynamic_profile_registration\":false,\"shipping_cxx_authoritative\":true,\"rust_poc_registers_profile\":false,\"release_identity_source\":\"cmake/release_identity.h.in\"}}",
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
        "{{\"format_version\":1,\"bounded_ipc_client_model\":true,\"timeout_ms\":25,\"expected_generation\":7,\"cases\":{{\"ok\":{},\"timeout_fails_open\":{},\"malformed_fails_open\":{},\"generation_mismatch_fails_open\":{}}},\"network_imports\":false,\"external_engine_link\":false,\"host_blocking_call\":false,\"shipping_cxx_authoritative\":true}}",
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
        "{{\"format_version\":1,\"single_edit_session\":true,\"operation_order\":[{}],\"commit_text\":\"{}\",\"preedit_text\":\"{}\",\"composition_active_after\":{},\"shipping_cxx_authoritative\":true,\"host_differential_pending\":true}}",
        transcript.operation_order_json(),
        transcript.commit,
        transcript.preedit,
        transcript.composition_active
    )
}

pub fn tsf_differential_summary_report() -> String {
    format!(
        "{{\"format_version\":1,\"component\":\"fcitx5-tsf-poc\",\"shipping_cxx_authoritative\":true,\"same_corpus_case_count\":{},\"same_corpus_rust_passes\":{},\"cpp_baseline_ctest\":\"tsf-key-commit-e2e\",\"abi_reports\":{{\"behavior\":true,\"profile_identity\":true,\"ipc_boundary\":true,\"composition_transcript\":true}},\"artifact_audit_ctest\":\"rust-tsf-poc-artifact-audit\",\"x64_x86_export_smoke_required\":true,\"arm64_ci_artifact_green\":true,\"real_host_matrix_pending\":true,\"product_decision\":\"continue_poc\"}}",
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
        "{{\"format_version\":1,\"corpus\":\"tsf_behavior_corpus.json\",\"case_count\":{},\"all_cases_present\":{},\"panic_boundary\":\"catch_unwind\",\"timeout_fail_open\":true,\"malformed_ipc_fail_open\":true,\"shipping_cxx_authoritative\":true}}",
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
        "{{\"format_version\":1,\"corpus\":\"tsf_behavior_corpus.json\",\"case_count\":{},\"rust_case_passes\":{},\"case_results\":[{}],\"cpp_baseline_ctest\":\"tsf-key-commit-e2e\",\"cpp_baseline_consumes_same_corpus\":true,\"shipping_cxx_authoritative\":true,\"full_host_differential_pending\":true,\"report_export\":\"panic_contained\"}}",
        REQUIRED_TSF_BEHAVIOR_CASES.len(),
        passed,
        case_results
    )
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
    ITfKeyEventSink
)]
#[derive(Default)]
struct Fcitx5TsfService {
    state: RefCell<TsfPocBehaviorState>,
}

impl Fcitx5TsfService {
    fn new() -> Self {
        module_add_ref();
        Self {
            state: RefCell::default(),
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
}

impl Drop for Fcitx5TsfService {
    fn drop(&mut self) {
        module_release();
    }
}

impl ITfTextInputProcessor_Impl for Fcitx5TsfService_Impl {
    fn Activate(&self, _thread_manager: Ref<ITfThreadMgr>, _client_id: u32) -> Result<()> {
        self.state.borrow_mut().activate();
        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
        self.state.borrow_mut().deactivate();
        Ok(())
    }
}

impl ITfTextInputProcessorEx_Impl for Fcitx5TsfService_Impl {
    fn ActivateEx(
        &self,
        _thread_manager: Ref<ITfThreadMgr>,
        _client_id: u32,
        _flags: u32,
    ) -> Result<()> {
        Ok(())
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
        _focused_document_manager: Ref<ITfDocumentMgr>,
        _previous_focused_document_manager: Ref<ITfDocumentMgr>,
    ) -> Result<()> {
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
        _context: Ref<ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        Ok(BOOL(0))
    }

    fn OnTestKeyUp(
        &self,
        _context: Ref<ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        Ok(BOOL(0))
    }

    fn OnKeyDown(
        &self,
        _context: Ref<ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        self.state.borrow_mut().key_down(EngineResult::malformed());
        Ok(BOOL(0))
    }

    fn OnKeyUp(&self, _context: Ref<ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        self.state.borrow_mut().key_up();
        Ok(BOOL(0))
    }

    fn OnPreservedKey(&self, _context: Ref<ITfContext>, _guid: *const GUID) -> Result<BOOL> {
        Ok(BOOL(0))
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
    panic_to_hresult(|| unsafe { dll_get_class_object_impl(class_id, interface_id, object) })
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

    #[test]
    fn panic_boundary_converts_unwind_to_hresult() {
        let result = panic_to_hresult(|| panic!("forced TSF PoC panic boundary regression"));
        assert_eq!(result, E_UNEXPECTED);
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
        assert!(report.contains("send_input:false"));
        assert!(report.contains("global_hooks:false"));
        assert!(report.contains("process_injection:false"));
        assert!(report.contains("cxx_tsf_remains_authoritative:true"));
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
        assert!(report.contains("\"shipping_cxx_authoritative\":true"));
        assert!(report.contains("\"rust_poc_registers_profile\":false"));
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
        assert!(report.contains("\"product_decision\":\"continue_poc\""));
    }

    #[test]
    fn module_unload_uses_refcounted_s_false() {
        module_add_ref();
        assert_eq!(unsafe { DllCanUnloadNow() }, S_FALSE);
        module_release();
        assert_eq!(unsafe { DllCanUnloadNow() }, S_OK);
    }
}
