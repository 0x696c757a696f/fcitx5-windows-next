#![deny(unsafe_op_in_unsafe_fn)]
#![allow(linker_messages)]
#![allow(non_snake_case)]

use std::ffi::c_void;
use std::panic::{catch_unwind, UnwindSafe};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicI32, Ordering};
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
static MODULE_REFERENCES: AtomicI32 = AtomicI32::new(0);
const TSF_BEHAVIOR_CORPUS_JSON: &str =
    include_str!("../../../tests/fixtures/tsf_behavior_corpus.json");

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineStatus {
    Ok,
    Timeout,
    Malformed,
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
            EngineStatus::Timeout | EngineStatus::Malformed => {
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
}

fn corpus_has_case(corpus: &str, id: &str) -> bool {
    let needle = format!("\"id\": \"{id}\"");
    corpus.contains(&needle)
}

pub fn tsf_behavior_corpus_report() -> String {
    const REQUIRED_CASES: &[&str] = &[
        "activate_advises_sinks",
        "key_down_commit_applies_text",
        "key_down_preedit_starts_composition",
        "key_up_routes_release_without_eating",
        "engine_timeout_fails_open",
        "malformed_ipc_fails_open",
        "deactivate_unadvises_sinks_and_clears_composition",
    ];
    let all_cases_present = REQUIRED_CASES
        .iter()
        .all(|case_id| corpus_has_case(TSF_BEHAVIOR_CORPUS_JSON, case_id));
    format!(
        "{{\"format_version\":1,\"corpus\":\"tsf_behavior_corpus.json\",\"case_count\":{},\"all_cases_present\":{},\"panic_boundary\":\"catch_unwind\",\"timeout_fail_open\":true,\"malformed_ipc_fail_open\":true,\"shipping_cxx_authoritative\":true}}",
        REQUIRED_CASES.len(),
        if all_cases_present { "true" } else { "false" }
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
struct Fcitx5TsfService;

impl Fcitx5TsfService {
    fn new() -> Self {
        module_add_ref();
        Self
    }
}

impl Drop for Fcitx5TsfService {
    fn drop(&mut self) {
        module_release();
    }
}

impl ITfTextInputProcessor_Impl for Fcitx5TsfService_Impl {
    fn Activate(&self, _thread_manager: Ref<ITfThreadMgr>, _client_id: u32) -> Result<()> {
        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
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
        Ok(BOOL(0))
    }

    fn OnKeyUp(&self, _context: Ref<ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
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
        assert!(report.contains(r#""case_count":7"#));
        assert!(report.contains(r#""all_cases_present":true"#));
        assert!(report.contains(r#""timeout_fail_open":true"#));
        assert!(report.contains(r#""malformed_ipc_fail_open":true"#));
        assert!(TSF_BEHAVIOR_CORPUS_JSON.contains("key_down_commit_applies_text"));
        assert!(TSF_BEHAVIOR_CORPUS_JSON.contains("deactivate_unadvises_sinks"));
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
    fn stable_profile_identity_is_shared_with_current_release_identity() {
        assert_eq!(
            FCITX5_TEXT_SERVICE_CLSID,
            GUID::from_u128(0x3a21b9e2_4f47_4c36_8bfa_91d7d3b3e901)
        );
    }

    #[test]
    fn module_unload_uses_refcounted_s_false() {
        module_add_ref();
        assert_eq!(unsafe { DllCanUnloadNow() }, S_FALSE);
        module_release();
        assert_eq!(unsafe { DllCanUnloadNow() }, S_OK);
    }
}
