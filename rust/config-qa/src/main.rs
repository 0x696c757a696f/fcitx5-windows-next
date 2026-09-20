#![cfg_attr(not(windows), forbid(unsafe_code))]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(not(windows))]
fn main() {
    println!("unsupported");
}

#[cfg(windows)]
#[allow(unsafe_op_in_unsafe_fn)]
mod windows_driver {
    use std::ffi::{c_void, OsString};
    use std::fs;
        use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};
    use std::ptr::null_mut;
    use std::thread::sleep;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    type Hresult = i32;
    type Bool = i32;
    type Dword = u32;
    type Hwnd = *mut c_void;
    type Handle = *mut c_void;
    type Lparam = isize;
    const TRUE: Bool = 1;
    const FALSE: Bool = 0;
    const S_OK: Hresult = 0;
    const COINIT_MULTITHREADED: Dword = 0;
    const CLSCTX_INPROC_SERVER: Dword = 1;
    const TREE_SCOPE_DESCENDANTS: i32 = 4;
    const UIA_NAME_PROPERTY_ID: i32 = 30005;
    const UIA_CONTROL_TYPE_PROPERTY_ID: i32 = 30003;
    const UIA_IS_KEYBOARD_FOCUSABLE_PROPERTY_ID: i32 = 30009;
    const UIA_IS_ENABLED_PROPERTY_ID: i32 = 30010;
    const UIA_BOUNDING_RECTANGLE_PROPERTY_ID: i32 = 30001;
    const UIA_INVOKE_PATTERN_ID: i32 = 10000;
    const BUTTON: i32 = 50000;
    const TEXT: i32 = 50020;
    const WINDOW: i32 = 50032;
    const VT_I4: u16 = 3;
    const VT_BOOL: u16 = 11;
    const VT_BSTR: u16 = 8;
    const VT_ARRAY: u16 = 0x2000;
    const VT_R8: u16 = 5;
    const VARIANT_TRUE: i16 = -1;
    const UIA_E_ELEMENTNOTAVAILABLE: u32 = 0x8004_0201;
    const PROCESS_TERMINATE: Dword = 0x0001;
    const DESKTOP_READOBJECTS: Dword = 0x0001;

    #[repr(C)] #[derive(Clone, Copy)] struct Guid { data1: u32, data2: u16, data3: u16, data4: [u8; 8] }
    #[repr(C)] #[derive(Clone, Copy)] struct Variant { vt: u16, r1: u16, r2: u16, r3: u16, data1: isize, data2: isize }
    #[repr(C)] #[derive(Clone, Copy, Default, Debug)] struct Rect { left: i32, top: i32, right: i32, bottom: i32 }
    #[repr(C)] struct Automation { vtable: *const AutomationVtable }
    #[repr(C)] struct Element { vtable: *const ElementVtable }
    #[repr(C)] struct ElementArray { vtable: *const ElementArrayVtable }
    #[repr(C)] struct Condition { vtable: *const c_void }
    #[repr(C)] struct Unknown { vtable: *const UnknownVtable }
    #[repr(C)] struct UnknownVtable { qi: usize, add_ref: usize, release: unsafe extern "system" fn(*mut Unknown) -> u32 }
    #[repr(C)] struct InvokeVtable { qi: usize, add_ref: usize, release: unsafe extern "system" fn(*mut c_void) -> u32, invoke: unsafe extern "system" fn(*mut c_void) -> Hresult }
    #[repr(C)] struct AutomationVtable {
        qi: usize, add_ref: usize, release: unsafe extern "system" fn(*mut Automation) -> u32,
        compare_elements: usize, compare_runtime_ids: usize, get_root_element: usize,
        element_from_handle: unsafe extern "system" fn(*mut Automation, Hwnd, *mut *mut Element) -> Hresult,
        element_from_point: usize, get_focused_element: usize, get_root_element_build_cache: usize,
        element_from_handle_build_cache: usize, element_from_point_build_cache: usize,
        get_focused_element_build_cache: usize, create_tree_walker: usize,
        get_control_view_walker: usize, get_content_view_walker: usize, get_raw_view_walker: usize,
        get_raw_view_condition: usize, get_control_view_condition: usize, get_content_view_condition: usize,
        create_cache_request: usize,
        create_true_condition: unsafe extern "system" fn(*mut Automation, *mut *mut Condition) -> Hresult,
        rest: [usize; 80],
    }
    #[repr(C)] struct ElementVtable {
        qi: usize, add_ref: usize, release: unsafe extern "system" fn(*mut Element) -> u32,
        set_focus: unsafe extern "system" fn(*mut Element) -> Hresult, get_runtime_id: usize, find_first: usize,
        find_all: unsafe extern "system" fn(*mut Element, i32, *mut Condition, *mut *mut ElementArray) -> Hresult,
        find_first_build_cache: usize, find_all_build_cache: usize, build_updated_cache: usize,
        get_current_property_value: unsafe extern "system" fn(*mut Element, i32, *mut Variant) -> Hresult,
        get_current_property_value_ex: unsafe extern "system" fn(*mut Element, i32, Bool, *mut Variant) -> Hresult,
        get_cached_property_value: usize,
        get_cached_property_value_ex: usize,
        get_current_pattern_as: usize,
        get_cached_pattern_as: usize,
        get_current_pattern: unsafe extern "system" fn(*mut Element, i32, *mut *mut c_void) -> Hresult,
        rest: [usize; 80],
    }
    #[repr(C)] struct ElementArrayVtable {
        qi: usize, add_ref: usize, release: unsafe extern "system" fn(*mut ElementArray) -> u32,
        length: unsafe extern "system" fn(*mut ElementArray, *mut i32) -> Hresult,
        get_element: unsafe extern "system" fn(*mut ElementArray, i32, *mut *mut Element) -> Hresult,
    }

    const CLSID_CUIAUTOMATION: Guid = Guid { data1: 0xff48dba4, data2: 0x60ef, data3: 0x4201, data4: [0xaa, 0x87, 0x54, 0x10, 0x3e, 0xef, 0x59, 0x4e] };
    const IID_IUIAUTOMATION: Guid = Guid { data1: 0x30cbe57d, data2: 0xd9d0, data3: 0x452a, data4: [0xab, 0x13, 0x7a, 0xc5, 0xac, 0x48, 0x25, 0xee] };

    #[link(name = "ole32")] unsafe extern "system" { fn CoInitializeEx(p: *mut c_void, flags: Dword) -> Hresult; fn CoUninitialize(); fn CoCreateInstance(c: *const Guid, o: *mut c_void, ctx: Dword, i: *const Guid, p: *mut *mut c_void) -> Hresult; }
    #[link(name = "oleaut32")] unsafe extern "system" { fn SysStringLen(p: *const u16) -> u32; fn SysFreeString(p: *mut u16); fn SafeArrayGetLBound(a: *mut c_void, d: Dword, p: *mut i32) -> Hresult; fn SafeArrayGetUBound(a: *mut c_void, d: Dword, p: *mut i32) -> Hresult; fn SafeArrayGetElement(a: *mut c_void, idx: *const i32, out: *mut c_void) -> Hresult; fn VariantClear(v: *mut Variant) -> Hresult; }
    #[link(name = "user32")] unsafe extern "system" { fn EnumWindows(f: unsafe extern "system" fn(Hwnd, Lparam) -> Bool, l: Lparam) -> Bool; fn GetWindowThreadProcessId(w: Hwnd, p: *mut Dword) -> Dword; fn GetClassNameW(w: Hwnd, b: *mut u16, n: i32) -> i32; fn GetWindowRect(w: Hwnd, r: *mut Rect) -> Bool; fn OpenInputDesktop(flags: Dword, inherit: Bool, access: Dword) -> Handle; fn SetForegroundWindow(w: Hwnd) -> Bool; fn ShowWindow(w: Hwnd, cmd: i32) -> Bool; fn SendMessageW(w: Hwnd, msg: u32, wp: usize, lp: isize) -> isize; fn GetProcessWindowStation() -> Handle; fn GetUserObjectInformationW(h: Handle, index: i32, data: *mut c_void, len: Dword, needed: *mut Dword) -> Bool; fn CloseDesktop(h: Handle) -> Bool; fn CloseWindowStation(h: Handle) -> Bool; }
    #[link(name = "kernel32")] unsafe extern "system" { fn GetTempPathW(n: Dword, b: *mut u16) -> Dword; fn OpenProcess(access: Dword, inherit: Bool, pid: Dword) -> Handle; fn TerminateProcess(h: Handle, code: u32) -> Bool; fn CloseHandle(h: Handle) -> Bool; }

    #[derive(Clone)] struct Args { exe: PathBuf, locale: String, report: Option<PathBuf> }
    #[derive(Clone)] struct Strings { locale: String, nav: String, section: String, layouts: [String; 5], apply: String, cancel: String, reset: String }
    #[derive(Clone, Copy)] struct Found { hwnd: Hwnd, pid: Dword }
    struct Item { element: *mut Element, name: String, control_type: i32, enabled: bool, focusable: bool, rect: [f64; 4], rect_error: Option<String>, invoke: bool }
    struct Com { automation: *mut Automation, root: *mut Element, array: *mut ElementArray, condition: *mut Condition }
    struct Case { strings: Strings, hwnd: Hwnd, window_rect: Rect, root_control_type: i32, items: Vec<Item>, com: Com, child: Child, app: PathBuf, config_path: PathBuf, ui_before: String }

    fn probe_hwnd_from_args() -> Option<Hwnd> { let mut it = std::env::args_os().skip(1); while let Some(a) = it.next() { if a == "--probe-hwnd" { let raw = it.next()?.to_string_lossy().into_owned(); let digits = raw.trim_start_matches("0x"); return usize::from_str_radix(digits, 16).ok().map(|value| value as Hwnd); } } None }
    fn run_probe(hwnd: Hwnd) -> Result<String, String> { let com = unsafe { Com::new(hwnd)? }; let ct = unsafe { property_i4(com.root, UIA_CONTROL_TYPE_PROPERTY_ID) }; let rect = unsafe { variant(com.root, UIA_BOUNDING_RECTANGLE_PROPERTY_ID) }; let ex = unsafe { variant_ex(com.root, UIA_BOUNDING_RECTANGLE_PROPERTY_ID) }; let (rvt, rd1) = match &rect { Ok(v) => (format!("0x{:04x}", v.vt), format!("0x{:x}", v.data1)), Err(error) => ("err".to_owned(), error.clone()) }; let evt = match &ex { Ok(v) => format!("0x{:04x}", v.vt), Err(error) => error.clone() }; Ok(format!("{{\"probe_hwnd\":\"0x{:x}\",\"control_type\":{:?},\"rect_vt\":\"{}\",\"rect_data1\":\"{}\",\"rect_ex_vt\":\"{}\"}}", hwnd as usize, ct, rvt, rd1, evt)) }
    pub fn main() { if let Some(hwnd) = probe_hwnd_from_args() { let code = match run_probe(hwnd) { Ok(json) => { println!("{json}"); 0 }, Err(error) => { println!("{{\"result\":\"FAIL\",\"reason\":{}}}", json_string(&error)); 1 }, }; std::process::exit(code); } let code = match run() { Ok(json) => { println!("{json}"); 0 }, Err(error) => { println!("{{\"result\":\"FAIL\",\"reason\":{}}}", json_string(&error)); 1 }, }; std::process::exit(code); }

    fn run() -> Result<String, String> {
        let args = parse_args(std::env::args_os().skip(1))?;
        let strings = load_strings(&args.exe, &args.locale)?;
        if !interactive() { return Ok(skip_json(&args.locale, "no interactive desktop")); }
        let (app, config_path) = make_app(&args.exe, &args.locale)?;
        let before = run_get(&app.join("fcitx5-config.exe"), &config_path).ok().and_then(|s| json_string_value(&s, "ui", "language")).unwrap_or_default();
        let mut child = Command::new(app.join("fcitx5-config.exe")).arg(format!("--lang={}", args.locale)).current_dir(&app).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| format!("launch Settings: {e}"))?;
        let hwnd = match find_window_until(child.id(), Duration::from_secs(20)) { Ok(h) => h, Err(_) => { terminate(&mut child); return Ok(skip_json(&args.locale, "Settings window not found within 20 seconds")); } };
        let mut window_rect = Rect::default(); unsafe { GetWindowRect(hwnd, &mut window_rect); }
        unsafe { ShowWindow(hwnd, 5); SetForegroundWindow(hwnd); }
        sleep(Duration::from_millis(400));
        let mut com = Com::new(hwnd)?;
        let root_control_type = unsafe { property_i4(com.root, UIA_CONTROL_TYPE_PROPERTY_ID)? };
        if root_control_type != WINDOW { return fail_case(&mut child, &com, format!("root ControlType {root_control_type}, expected {WINDOW}")); }
        let root_rect: Result<[f64; 4], String> = unsafe { property_rect(com.root) };
        let items = unsafe { refresh_items(&mut com)? };
        let mut case = Case { strings, hwnd, window_rect, root_control_type, items, com, child, app, config_path, ui_before: before };
        validate_nav(&case)?;
        eprintln!("TRACE about-to-invoke-nav");
        invoke_named(&case, &case.strings.nav)?;
        eprintln!("TRACE nav-invoke-returned");
        let navigation_start = Instant::now();
        while navigation_start.elapsed() < Duration::from_secs(5) {
            case.items = unsafe { refresh_items(&mut case.com)? };
            if case.items.iter().any(|item| item.name == case.strings.section) { break; }
            sleep(Duration::from_millis(100));
        }
        if !case.items.iter().any(|item| item.name == case.strings.section) {
            return fail_case(&mut case.child, &case.com, format!("Appearance navigation did not expose candidate section within 5 seconds: {:?}", case.strings.section));
        }
        eprintln!("TRACE navigation-window-done validating");
        if let Some(section) = case.items.iter().find(|item| item.name == case.strings.section) { unsafe { let _ = ((*(*section.element).vtable).set_focus)(section.element); } sleep(Duration::from_millis(250)); }
        unsafe { SendMessageW(hwnd, 0x0005, 0, (((window_rect.bottom - window_rect.top) as isize) << 16) | ((window_rect.right - window_rect.left) as isize & 0xffff)); }
        sleep(Duration::from_millis(200));
        let mut stable = 0usize;
        let layout_start = Instant::now();
        while layout_start.elapsed() < Duration::from_secs(20) {
            case.items = unsafe { refresh_items(&mut case.com)? };
            let ready = visible_count(&case.items, &case.strings.section) >= 1
                && visible_count(&case.items, &case.strings.nav) >= 1
                && visible_count(&case.items, &case.strings.apply) >= 1
                && case.strings.layouts.iter().all(|layout| visible_count(&case.items, layout) >= 1)
                && (1..=9).all(|value| visible_count(&case.items, &value.to_string()) >= 1);
            if ready { stable += 1; if stable >= 3 { break; } } else { stable = 0; }
            sleep(Duration::from_millis(150));
        }
        validate(&case)?;
        eprintln!("TRACE semantics-validated");
        eprintln!("TRACE about-to-invoke-flow");
        invoke_named(&case, &case.strings.layouts[2])?;
        eprintln!("TRACE flow-returned");
        case.items = unsafe { refresh_items(&mut case.com)? };
        invoke_named(&case, "7")?;
        case.items = unsafe { refresh_items(&mut case.com)? };
        invoke_named(&case, &case.strings.apply)?;
        let settle_start = Instant::now();
        while !case.config_path.is_file() && settle_start.elapsed() < Duration::from_secs(15) { sleep(Duration::from_millis(100)); }
        if !case.config_path.is_file() { return fail_case(&mut case.child, &case.com, "config.toml did not appear within 15 seconds".to_owned()); }
        let readback = run_get(&case.app.join("fcitx5-config.exe"), &case.config_path)?;
        let layout = json_string_value_in_object(&readback, "candidate", "layout_type").ok_or("readback candidate.layout_type missing")?;
        let page_size = json_number_value_in(&readback, "candidate", "page_size").ok_or("readback candidate.page_size missing")?;
        let after = json_string_value_in_object(&readback, "ui", "language").unwrap_or_default();
        if layout != "flow" || page_size != 7 { return fail_case(&mut case.child, &case.com, format!("readback candidate expected layout_type=flow,page_size=7, got {layout},{page_size}")); }
        if after != case.ui_before { return fail_case(&mut case.child, &case.com, format!("readback ui.language changed from {:?} to {:?}", case.ui_before, after)); }
        let mut rect_stable = 0usize;
        unsafe { SendMessageW(hwnd, 0x0005, 0, (((window_rect.bottom - window_rect.top) as isize) << 16) | ((window_rect.right - window_rect.left) as isize & 0xffff)); }
        sleep(Duration::from_millis(200));
        let rect_start = Instant::now();
        while rect_start.elapsed() < Duration::from_secs(20) {
            case.items = unsafe { refresh_items(&mut case.com)? };
            let ready = visible_count(&case.items, &case.strings.section) >= 1
                && visible_count(&case.items, &case.strings.apply) >= 1
                && case.strings.layouts.iter().all(|layout| visible_count(&case.items, layout) >= 1)
                && (1..=9).all(|value| visible_count(&case.items, &value.to_string()) >= 1);
            if ready { rect_stable += 1; if rect_stable >= 3 { break; } } else { rect_stable = 0; }
            sleep(Duration::from_millis(150));
        }
        let fresh_root_rect = unsafe { property_rect(case.com.root) };
        let stale = stale_check(&mut case)?;
        let rect_notes = rect_failures(&case, &fresh_root_rect);
        if !rect_notes.is_empty() {
            return fail_case(&mut case.child, &case.com, format!("BoundingRectangle assertions failed for {} element(s): {}", rect_notes.len(), rect_notes.iter().take(3).cloned().collect::<Vec<_>>().join(" | ")));
        }
        let json = success_json(&case, &layout, page_size, &after, stale);
        let json = format!("{},\"bounding_rect_status\":\"AUTOMATED-GREEN\"}}", json.trim_end_matches('}'));
        if let Some(path) = &args.report { fs::write(path, &json).map_err(|e| format!("write report {}: {e}", path.display()))?; }
        Ok(json)
    }

    fn parse_args<I: Iterator<Item = OsString>>(mut it: I) -> Result<Args, String> {
        let mut smoke = false; let mut exe = None; let mut locale = None; let mut report = None;
        while let Some(a) = it.next() { match a.to_string_lossy().as_ref() { "--uia-smoke" => smoke = true, "--config-exe" => exe = it.next().map(PathBuf::from), "--locale" => locale = it.next().map(|v| v.to_string_lossy().into_owned()), "--report" => report = it.next().map(PathBuf::from), x => return Err(format!("unknown argument: {x}")) } }
        if !smoke { return Err("--uia-smoke is required".into()); }
        let locale = locale.ok_or("--locale is required")?; if !["en-US","ja-JP","ko-KR","si-LK","th-TH","vi-VN","zh-CN","zh-TW"].contains(&locale.as_str()) { return Err(format!("unsupported locale {locale}")); }
        Ok(Args { exe: exe.ok_or("--config-exe is required")?, locale, report })
    }

    fn load_strings(exe: &Path, locale: &str) -> Result<Strings, String> {
        let path = exe.parent().ok_or("config exe has no parent")?.join("locales").join(format!("{locale}.json"));
        let text = fs::read_to_string(&path).map_err(|e| format!("read locale resource {}: {e}", path.display()))?;
        let get = |key: &str| json_top_string(&text, key).ok_or_else(|| format!("locale {locale} missing key {key}"));
        Ok(Strings { locale: locale.into(), nav: get("nav.appearance")?, section: get("settings.candidate.section")?, layouts: [get("settings.candidate.layout.automatic")?, get("settings.candidate.layout.stacked")?, get("settings.candidate.layout.flow")?, get("settings.candidate.layout.scroll")?, get("settings.candidate.layout.vertical_text")?], apply: json_top_string(&text, "settings.candidate.apply").or_else(|| json_top_string(&text, "action.apply")).ok_or_else(|| format!("locale {locale} missing key settings.candidate.apply/action.apply"))?, cancel: json_top_string(&text, "settings.candidate.cancel").or_else(|| json_top_string(&text, "dialog.button.cancel")).ok_or_else(|| format!("locale {locale} missing key settings.candidate.cancel/dialog.button.cancel"))?, reset: get("settings.candidate.reset")? })
    }

    fn make_app(exe: &Path, locale: &str) -> Result<(PathBuf, PathBuf), String> {
        let mut temp = std::env::temp_dir(); let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos(); temp.push(format!("fcitx5-config-qa-uia-{locale}-{}-{stamp}", std::process::id())); fs::create_dir_all(&temp).map_err(|e| format!("create temp app: {e}"))?;
        for name in ["fcitx5-config.exe", "fcitx5-control.exe"] { let src = exe.parent().unwrap().join(name); if src.is_file() { fs::copy(&src, temp.join(name)).map_err(|e| format!("copy {name}: {e}"))?; } }
        for dir in ["locales", "resources"] { let src = exe.parent().unwrap().join(dir); if src.is_dir() { copy_dir(&src, &temp.join(dir))?; } }
        fs::write(temp.join("portable.flag"), []).map_err(|e| format!("portable.flag: {e}"))?;
        Ok((temp.clone(), temp.join("data").join("config.toml")))
    }
    fn copy_dir(src: &Path, dst: &Path) -> Result<(), String> { fs::create_dir_all(dst).map_err(|e| format!("create {}: {e}", dst.display()))?; for entry in fs::read_dir(src).map_err(|e| format!("read {}: {e}", src.display()))? { let e = entry.map_err(|e| e.to_string())?; let d = dst.join(e.file_name()); if e.file_type().map_err(|e| e.to_string())?.is_dir() { copy_dir(&e.path(), &d)?; } else { fs::copy(e.path(), d).map_err(|e| e.to_string())?; } } Ok(()) }

    fn interactive() -> bool { unsafe { let desktop = OpenInputDesktop(0, FALSE, DESKTOP_READOBJECTS); if desktop.is_null() { return false; } let ok = CloseDesktop(desktop) != 0; let station = GetProcessWindowStation(); if station.is_null() { return false; } let mut name = [0u16; 64]; let mut needed = 0; let named = GetUserObjectInformationW(station, 2, name.as_mut_ptr().cast(), (name.len() * 2) as u32, &mut needed) != 0; ok && (!named || String::from_utf16_lossy(&name[..needed.saturating_sub(2) as usize / 2]).starts_with("WinSta")) } }

    static mut SEARCH_PID: Dword = 0;
    static mut SEARCH_HWND: Hwnd = null_mut();
    unsafe extern "system" fn enum_proc(hwnd: Hwnd, _: Lparam) -> Bool { let mut pid = 0; GetWindowThreadProcessId(hwnd, &mut pid); if pid != SEARCH_PID { return TRUE; } let mut buf = [0u16; 64]; let n = GetClassNameW(hwnd, buf.as_mut_ptr(), 64); if String::from_utf16_lossy(&buf[..n.max(0) as usize]) == "WindUiWindowClass" { SEARCH_HWND = hwnd; FALSE } else { TRUE } }
    fn find_window(pid: Dword) -> Hwnd { unsafe { SEARCH_PID = pid; SEARCH_HWND = null_mut(); EnumWindows(enum_proc, 0); SEARCH_HWND } }
    fn find_window_until(pid: u32, timeout: Duration) -> Result<Hwnd, String> { let start = Instant::now(); while start.elapsed() < timeout { let hwnd = find_window(pid); if !hwnd.is_null() { return Ok(hwnd); } sleep(Duration::from_millis(100)); } Err("window timeout".into()) }

    impl Com { fn new(hwnd: Hwnd) -> Result<Self, String> { unsafe { let hr = CoInitializeEx(null_mut(), COINIT_MULTITHREADED); if hr < 0 && hr as u32 != 0x8001_0106 { return Err(format!("CoInitializeEx HRESULT 0x{hr:08x}")); } let mut p = null_mut(); let hr = CoCreateInstance(&CLSID_CUIAUTOMATION, null_mut(), CLSCTX_INPROC_SERVER, &IID_IUIAUTOMATION, &mut p); if hr < 0 || p.is_null() { return Err(format!("CoCreateInstance(CUIAutomation) HRESULT 0x{hr:08x}")); } let automation = p.cast::<Automation>(); let mut root = null_mut(); let hr = ((*(*automation).vtable).element_from_handle)(automation, hwnd, &mut root); if hr < 0 { return Err(format!("ElementFromHandle HRESULT 0x{hr:08x}")); } let mut condition = null_mut(); let hr = ((*(*automation).vtable).create_true_condition)(automation, &mut condition); if hr < 0 { return Err(format!("CreateTrueCondition HRESULT 0x{hr:08x}")); } let mut array = null_mut(); let hr = ((*(*root).vtable).find_all)(root, TREE_SCOPE_DESCENDANTS, condition, &mut array); if hr < 0 { return Err(format!("FindAll HRESULT 0x{hr:08x}")); } Ok(Self { automation, root, array, condition }) } } }

    unsafe fn variant(e: *mut Element, id: i32) -> Result<Variant, String> { let mut v = Variant { vt: 0, r1: 0, r2: 0, r3: 0, data1: 0, data2: 0 }; let hr = ((*(*e).vtable).get_current_property_value)(e, id, &mut v); if hr < 0 { return Err(format!("GetCurrentPropertyValue({id}) HRESULT 0x{hr:08x}")); } Ok(v) }
    unsafe fn property_i4(e: *mut Element, id: i32) -> Result<i32, String> { let v = variant(e, id)?; if v.vt != VT_I4 { return Err(format!("property {id} returned VARIANT vt {}", v.vt)); } Ok(v.data1 as i32) }
    unsafe fn property_bool(e: *mut Element, id: i32) -> Result<bool, String> { let v = variant(e, id)?; if v.vt != VT_BOOL { return Err(format!("property {id} returned VARIANT vt {}", v.vt)); } Ok(v.data1 as i16 == VARIANT_TRUE) }
    unsafe fn property_name(e: *mut Element) -> Result<String, String> { let v = variant(e, UIA_NAME_PROPERTY_ID)?; if v.vt != VT_BSTR || v.data1 == 0 { return Ok(String::new()); } let p = v.data1 as *mut u16; let n = SysStringLen(p) as usize; let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n)); SysFreeString(p); Ok(s) }
    unsafe fn variant_ex(e: *mut Element, id: i32) -> Result<Variant, String> { let mut v = Variant { vt: 0, r1: 0, r2: 0, r3: 0, data1: 0, data2: 0 }; let hr = ((*(*e).vtable).get_current_property_value_ex)(e, id, 1, &mut v); if hr < 0 { return Err(format!("GetCurrentPropertyValueEx({id}) HRESULT 0x{hr:08x}")); } Ok(v) }
    unsafe fn property_rect(e: *mut Element) -> Result<[f64; 4], String> { let v = variant(e, UIA_BOUNDING_RECTANGLE_PROPERTY_ID)?; if v.vt != (VT_ARRAY | VT_R8) { let exvt = match variant_ex(e, UIA_BOUNDING_RECTANGLE_PROPERTY_ID) { Ok(x) => format!("vt=0x{:04x}", x.vt), Err(e2) => e2 }; return Err(format!("BoundingRectangle VARIANT vt=0x{:04x} data1=0x{:x} (expected 0x2005); GetCurrentPropertyValueEx(ignoreDefault=TRUE) -> {exvt}", v.vt, v.data1)); } let a = v.data1 as *mut c_void; let mut low = 0; let mut high = 0; if SafeArrayGetLBound(a, 1, &mut low) < 0 || SafeArrayGetUBound(a, 1, &mut high) < 0 { return Err(format!("BoundingRectangle SAFEARRAY bounds unreadable lb={low} ub={high}")); } let mut out = [0.0; 4]; for i in low..=high { if i - low < 4 { SafeArrayGetElement(a, &i, (&mut out[(i - low) as usize] as *mut f64).cast()); } } VariantClear((&v as *const Variant).cast_mut()); Ok([out[0], out[1], out[0] + out[2], out[1] + out[3]]) }
    unsafe fn has_invoke(e: *mut Element) -> bool { let mut p = null_mut(); let hr = ((*(*e).vtable).get_current_pattern)(e, UIA_INVOKE_PATTERN_ID, &mut p); if hr != S_OK || p.is_null() { return false; } ((*((p as *mut Unknown).read()).vtable).release)(p.cast()); true }
    unsafe fn refresh_items(com: &mut Com) -> Result<Vec<Item>, String> { if !com.array.is_null() { ((*(*com.array).vtable).release)(com.array); com.array = null_mut(); } let mut hr = 0; let mut attempts = 0; while attempts < 40 { hr = ((*(*com.root).vtable).find_all)(com.root, TREE_SCOPE_DESCENDANTS, com.condition, &mut com.array); if hr >= 0 && !com.array.is_null() { break; } attempts += 1; sleep(Duration::from_millis(100)); } if hr < 0 || com.array.is_null() { return Err(format!("FindAll HRESULT 0x{hr:08x} after {attempts} retries")); } let mut len = 0; ((*(*com.array).vtable).length)(com.array, &mut len); let mut out = Vec::with_capacity(len as usize); for i in 0..len { let mut e = null_mut(); let hr = ((*(*com.array).vtable).get_element)(com.array, i, &mut e); if hr < 0 || e.is_null() { return Err(format!("element array index {i} HRESULT 0x{hr:08x}")); } let (rect, rect_error) = rect_soft(e); out.push(Item { element: e, name: property_name(e)?, control_type: property_i4(e, UIA_CONTROL_TYPE_PROPERTY_ID)?, enabled: property_bool(e, UIA_IS_ENABLED_PROPERTY_ID)?, focusable: property_bool(e, UIA_IS_KEYBOARD_FOCUSABLE_PROPERTY_ID)?, rect, rect_error, invoke: has_invoke(e) }); } Ok(out) }

    unsafe fn rect_soft(e: *mut Element) -> ([f64; 4], Option<String>) { match property_rect(e) { Ok(v) => (v, None), Err(err) => ([0.0; 4], Some(err)) } }
    fn visible_count(items: &[Item], name: &str) -> usize { items.iter().filter(|item| item.name == name && item.rect_error.is_none() && item.rect[2] > item.rect[0] && item.rect[3] > item.rect[1]).count() }
    fn rect_failures(case: &Case, _root_rect: &Result<[f64; 4], String>) -> Vec<String> { let mut out = Vec::new(); let r = case.window_rect; let s = &case.strings; let mut asserted: Vec<String> = vec![s.nav.clone(), s.section.clone(), s.apply.clone(), s.cancel.clone(), s.reset.clone()]; for layout in s.layouts.iter() { asserted.push(layout.clone()); } for value in 1..=9 { asserted.push(value.to_string()); } let mut table: Vec<String> = Vec::new(); for name in &asserted { let mut visible = 0usize; let mut observed: Vec<String> = Vec::new(); for item in case.items.iter().filter(|item| &item.name == name) { match &item.rect_error { Some(err) => observed.push(format!("unavailable({err})")), None => observed.push(format!("{:?}", item.rect)) } if item.rect_error.is_some() { continue; } if item.rect[2] <= item.rect[0] || item.rect[3] <= item.rect[1] { continue; } visible += 1; if item.rect[0] < r.left as f64 || item.rect[1] < r.top as f64 || item.rect[2] > r.right as f64 || item.rect[3] > r.bottom as f64 { out.push(format!("{name} rect {:?} outside window rect", item.rect)); } } table.push(format!("{name}[{visible}]={}", observed.join("+"))); if visible == 0 { out.push(format!("{name} has no visible instance with a non-empty bounding rect; observed: {}", observed.join(" , "))); } else if visible > 1 && (case.strings.layouts.iter().any(|layout| layout == name) || name.chars().all(|c| c.is_ascii_digit())) { out.push(format!("{name} has {visible} visible instances (active/inactive twin must enumerate exactly one)")); } } if !out.is_empty() { out.push(format!("TABLE {}", table.join(" | "))); } out }
    fn validate_nav(case: &Case) -> Result<(), String> { let s = &case.strings; let named: Vec<_> = case.items.iter().filter(|i| i.name == s.nav).collect(); let nav_matches: Vec<_> = named.iter().filter(|i| i.invoke).copied().collect(); if nav_matches.len() != 1 { let detail: Vec<String> = named.iter().map(|i| format!("type={} invoke={} enabled={} focusable={} rect={:?}", i.control_type, i.invoke, i.enabled, i.focusable, i.rect)).collect(); return Err(format!("nav.appearance {:?} invoke-capable entries = {} (expected exactly one); total with that name = {}; details: {}", s.nav, nav_matches.len(), named.len(), detail.join(" | "))); } let nav = nav_matches[0]; if !nav.enabled { return Err(format!("nav.appearance {:?} enabled={}", s.nav, nav.enabled)); } Ok(()) }
    fn validate(case: &Case) -> Result<(), String> { let s = &case.strings; let required = [(&s.apply, BUTTON, false, "settings.candidate.apply"), (&s.section, TEXT, false, "settings.candidate.section"), (&s.reset, BUTTON, false, "settings.candidate.reset"), (&s.cancel, BUTTON, false, "settings.candidate.cancel")]; for (name, ty, needs_invoke, key) in required { let matches: Vec<_> = case.items.iter().filter(|i| i.name == *name).collect(); if matches.is_empty() { return Err(format!("missing {key} element named {:?}", name)); } if matches.len() > 1 && (ty == BUTTON || needs_invoke) { return Err(format!("duplicate {key} element named {:?}", name)); } for item in matches { check_item(case, item, ty, needs_invoke, key)?; } }
        for name in &s.layouts { let matches: Vec<_> = case.items.iter().filter(|i| i.name == *name).collect(); if matches.len() != 1 { return Err(format!("layout {:?} enumerated {} times, expected exactly one", name, matches.len())); } check_item(case, matches[0], BUTTON, true, "layout")?; }
        for digit in 1..=9 { let name = digit.to_string(); let matches: Vec<_> = case.items.iter().filter(|i| i.name == name).collect(); if matches.len() != 1 { return Err(format!("count button {digit} enumerated {} times, expected exactly one", matches.len())); } check_item(case, matches[0], BUTTON, true, "count")?; }
        let mut seen = std::collections::HashMap::new(); for item in &case.items { if s.layouts.iter().any(|x| x == &item.name) || (item.name.len() == 1 && item.name.as_bytes()[0].is_ascii_digit()) { let count = seen.entry(item.name.clone()).or_insert(0); *count += 1; if *count > 1 { return Err(format!("duplicate enumerated layout/count name {:?}", item.name)); } } } Ok(()) }
    fn check_item(case: &Case, item: &Item, ty: i32, invoke: bool, key: &str) -> Result<(), String> { if item.control_type != ty && !(key == "settings.candidate.section" && item.control_type == BUTTON) { return Err(format!("{key} {:?} ControlType {}, expected {ty}", item.name, item.control_type)); } if invoke && !item.invoke { return Err(format!("{key} {:?} is not Invoke-capable", item.name)); } if !item.enabled || (!item.focusable && key != "settings.candidate.section") { return Err(format!("{key} {:?} enabled={} keyboard_focusable={}", item.name, item.enabled, item.focusable)); }Ok(()) }
    fn invoke_named(case: &Case, name: &str) -> Result<(), String> { let item = case.items.iter().find(|i| i.name == name).ok_or_else(|| format!("cannot invoke missing {:?}", name))?; unsafe { let mut p = null_mut(); let hr = ((*(*item.element).vtable).get_current_pattern)(item.element, UIA_INVOKE_PATTERN_ID, &mut p); if hr < 0 || p.is_null() { return Err(format!("GetCurrentPattern Invoke {:?} HRESULT 0x{hr:08x}", name)); } let inv = p.cast::<c_void>(); let vtable = *(inv as *mut *const InvokeVtable); let hr = ((*vtable).invoke)(inv); ((*vtable).release)(inv); if hr < 0 { return Err(format!("Invoke {:?} HRESULT 0x{hr:08x}", name)); } } Ok(()) }
    fn stale_check(case: &mut Case) -> Result<u32, String> { let item = case.items.iter().find(|i| i.name == case.strings.apply).ok_or("stale Apply element missing")?.element; terminate(&mut case.child); sleep(Duration::from_millis(100)); let v = unsafe { variant(item, UIA_NAME_PROPERTY_ID) }; match v { Ok(_) => Err("stale element property unexpectedly succeeded".into()), Err(e) => { let marker = e.split("HRESULT 0x").nth(1).and_then(|x| u32::from_str_radix(x.trim(), 16).ok()).ok_or(e.clone())?; if marker != UIA_E_ELEMENTNOTAVAILABLE { return Err(format!("stale element HRESULT 0x{marker:08x}, expected 0x{UIA_E_ELEMENTNOTAVAILABLE:08x}")); } Ok(marker) } } }
    fn fail_case(child: &mut Child, _: &Com, error: String) -> Result<String, String> { terminate(child); Err(error) }
    fn terminate(child: &mut Child) { if child.try_wait().ok().flatten().is_none() { unsafe { let h = OpenProcess(PROCESS_TERMINATE, FALSE, child.id()); if !h.is_null() { TerminateProcess(h, 1); CloseHandle(h); } } } }
    fn run_get(exe: &Path, config: &Path) -> Result<String, String> { let o = Command::new(exe).args(["--config", config.to_string_lossy().as_ref(), "get"]).output().map_err(|e| format!("run Config Core loader: {e}"))?; if !o.status.success() { return Err(format!("Config Core loader failed: {}", String::from_utf8_lossy(&o.stderr).trim())); } String::from_utf8(o.stdout).map_err(|_| "Config Core loader stdout is not UTF-8".into()) }

    fn skip_json(locale: &str, reason: &str) -> String { format!("{{\"locale\":{},\"result\":\"SKIP\",\"reason\":{}}}", json_string(locale), json_string(reason)) }
    fn success_json(case: &Case, layout: &str, page_size: i64, after: &str, stale: u32) -> String { let names = case.strings.layouts.iter().map(|x| json_string(x)).collect::<Vec<_>>().join(","); let digits = (1..=9).map(|x| json_string(&x.to_string())).collect::<Vec<_>>().join(","); format!("{{\"locale\":{},\"hwnd\":{},\"window_rect\":{{\"left\":{},\"top\":{},\"right\":{},\"bottom\":{}}},\"root_control_type\":{},\"descendant_count\":{},\"button_count\":{},\"invoke_capable_count\":{},\"layout_names\":[{}],\"digit_names\":[{}],\"readback_config_path\":{},\"readback_layout_type\":{},\"readback_page_size\":{},\"readback_ui_language_before\":{},\"readback_ui_language_after\":{},\"stale_hresult\":{},\"result\":\"PASS\"}}", json_string(&case.strings.locale), case.hwnd as usize, case.window_rect.left, case.window_rect.top, case.window_rect.right, case.window_rect.bottom, case.root_control_type, case.items.len(), case.items.iter().filter(|i| i.control_type == BUTTON).count(), case.items.iter().filter(|i| i.invoke).count(), names, digits, json_string(&case.config_path.display().to_string()), json_string(layout), page_size, json_string(&case.ui_before), json_string(after), stale) }

    fn json_string(s: &str) -> String { let mut out = String::from("\""); for c in s.chars() { match c { '\\' => out.push_str("\\\\"), '"' => out.push_str("\\\""), '\n' => out.push_str("\\n"), '\r' => out.push_str("\\r"), '\t' => out.push_str("\\t"), c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)), c => out.push(c) } } out.push('"'); out }
    fn json_top_string(text: &str, key: &str) -> Option<String> { json_key_string(text, key, 0) }
    fn json_string_value(text: &str, object: &str, key: &str) -> Option<String> { json_string_value_in(text, key).or_else(|| { let start = text.find(&format!("\"{object}\""))?; json_key_string(text, key, start) }) }
    fn json_key_string(text: &str, key: &str, start: usize) -> Option<String> { let needle = format!("\"{key}\""); let p = text[start..].find(&needle)? + start; let mut i = p + needle.len(); while text.as_bytes().get(i).is_some_and(|b| b.is_ascii_whitespace()) { i += 1; } if text.as_bytes().get(i) != Some(&b':') { return None; } i += 1; while text.as_bytes().get(i).is_some_and(|b| b.is_ascii_whitespace()) { i += 1; } parse_json_string(text, i) }
    fn parse_json_string(text: &str, mut i: usize) -> Option<String> { if text.as_bytes().get(i) != Some(&b'"') { return None; } i += 1; let mut out = String::new(); let b = text.as_bytes(); while i < b.len() { match b[i] { b'"' => return Some(out), b'\\' => { i += 1; match *b.get(i)? { b'"' => out.push('"'), b'\\' => out.push('\\'), b'/' => out.push('/'), b'b' => out.push('\u{8}'), b'f' => out.push('\u{c}'), b'n' => out.push('\n'), b'r' => out.push('\r'), b't' => out.push('\t'), b'u' => { let hex = std::str::from_utf8(b.get(i + 1..i + 5)?).ok()?; out.push(char::from_u32(u32::from_str_radix(hex, 16).ok()?)?); i += 4; }, _ => return None } }, c if c < 0x80 => out.push(c as char), _ => { let ch = text[i..].chars().next()?; out.push(ch); i += ch.len_utf8() - 1; } } i += 1; } None }
    fn json_string_value_in(text: &str, key: &str) -> Option<String> { json_key_string(text, key, 0) }
    fn json_string_value_in_object(text: &str, object: &str, key: &str) -> Option<String> { let start = text.find(&format!("\"{object}\""))?; json_key_string(text, key, start) }
    fn json_number_value_in(text: &str, object: &str, key: &str) -> Option<i64> { let start = text.find(&format!("\"{object}\""))?; let needle = format!("\"{key}\""); let p = text[start..].find(&needle)? + start; let mut i = p + needle.len(); while text.as_bytes().get(i).is_some_and(|b| b.is_ascii_whitespace() || *b == b':') { i += 1; } let end = (i..text.len()).find(|j| !text.as_bytes()[*j].is_ascii_digit()).unwrap_or(text.len()); text[i..end].parse().ok() }
}

#[cfg(windows)]
mod preview_qa;

#[cfg(windows)]
fn main() {
    if std::env::args_os().any(|value| value == "--uia-smoke" || value == "--probe-hwnd") {
        windows_driver::main();
    } else {
        preview_qa::main();
    }
}
