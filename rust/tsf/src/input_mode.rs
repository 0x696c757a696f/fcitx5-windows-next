//! Taskbar input indicator: a language bar button that shows the current
//! input method as one glyph, toggles it on a left click and offers Settings
//! on a right click.

#![deny(unsafe_op_in_unsafe_fn)]

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::ptr::null_mut;

use windows::Win32::Foundation::{COLORREF, ERROR_SUCCESS, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject,
    DrawTextW, GdiFlush, SelectObject, SetBkMode, SetTextColor, ANTIALIASED_QUALITY, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DIB_RGB_COLORS, DT_CENTER,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FW_NORMAL, OUT_DEFAULT_PRECIS, TRANSPARENT,
};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows::Win32::UI::TextServices::{
    ITfLangBarItem, ITfLangBarItemButton, ITfLangBarItemButton_Impl, ITfLangBarItemSink,
    ITfLangBarItem_Impl, ITfMenu, ITfSource, ITfSource_Impl, TfLBIClick, GUID_LBI_INPUTMODE,
    TF_LANGBARITEMINFO, TF_LBI_CLK_LEFT, TF_LBI_CLK_RIGHT, TF_LBI_ICON, TF_LBI_STYLE_BTN_BUTTON,
    TF_LBI_STYLE_SHOWNINTRAY, TF_LBI_TEXT, TF_LBI_TOOLTIP,
};
use windows::Win32::UI::WindowsAndMessaging as wam;
use windows_core::{implement, w, IUnknown, Interface, Ref, Result, BOOL, BSTR, GUID, HRESULT};

use super::{
    com_guard, module_add_ref, module_release, Fcitx5TsfService_Impl, FCITX5_TEXT_SERVICE_CLSID,
};

const CONNECT_E_ADVISELIMIT: HRESULT = HRESULT(0x8004_0201_u32 as i32);
const CONNECT_E_CANNOTCONNECT: HRESULT = HRESULT(0x8004_0202_u32 as i32);
const SINK_COOKIE: u32 = 1;
const MENU_TOGGLE: usize = 1;
const MENU_SETTINGS: usize = 2;
const KEYBOARD_GLYPH: &str = "英";
const UNKNOWN_GLYPH: &str = "中";

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetUserDefaultUILanguage() -> u16;
}

/// The input method the engine reports for one context.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct InputModeStatus {
    pub id: String,
    pub name: String,
    pub native_name: String,
    pub short_label: String,
}

impl InputModeStatus {
    /// One glyph for a 16-pixel icon: keyboard layouts read as English,
    /// everything else uses the input method's own short label.
    pub fn glyph(&self) -> String {
        if self.id.starts_with("keyboard-") {
            return KEYBOARD_GLYPH.to_owned();
        }
        let label = self.short_label.trim();
        if label.is_empty() {
            return UNKNOWN_GLYPH.to_owned();
        }
        if label.is_ascii() {
            return label.chars().take(2).collect();
        }
        label.chars().take(1).collect()
    }

    pub fn tooltip(&self) -> String {
        let name = if self.native_name.is_empty() {
            &self.name
        } else {
            &self.native_name
        };
        if name.is_empty() {
            "Fcitx5".to_owned()
        } else {
            format!("Fcitx5 · {name}")
        }
    }
}

/// Menu labels for the user's display language.
pub(crate) fn menu_labels(language: u16) -> (&'static str, &'static str) {
    const LANG_CHINESE: u16 = 0x04;
    const LANG_JAPANESE: u16 = 0x11;
    const LANG_KOREAN: u16 = 0x12;
    match (language & 0x3ff, language) {
        (LANG_CHINESE, 0x0404 | 0x0c04 | 0x1404) => ("切換輸入法", "設定"),
        (LANG_CHINESE, _) => ("切换输入法", "设置"),
        (LANG_JAPANESE, _) => ("入力方式を切り替え", "設定"),
        (LANG_KOREAN, _) => ("입력기 전환", "설정"),
        _ => ("Toggle input method", "Settings"),
    }
}

/// Turns white-on-black GDI coverage into the ink color with matching
/// alpha. GDI leaves the alpha byte at zero, so coverage comes from the
/// brightest channel of each pixel.
pub(crate) fn tint_coverage(pixels: &mut [u32], ink: u32) {
    for pixel in pixels {
        let coverage = (*pixel & 0xff)
            .max((*pixel >> 8) & 0xff)
            .max((*pixel >> 16) & 0xff);
        *pixel = if coverage == 0 {
            0
        } else {
            (coverage << 24) | (ink & 0x00ff_ffff)
        };
    }
}

fn taskbar_uses_light_theme() -> bool {
    let mut value = 0_u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: value and size are writable locals of the declared DWORD size.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut size),
        )
    };
    status == ERROR_SUCCESS && value != 0
}

fn render_icon(glyph: &str) -> Option<wam::HICON> {
    // SAFETY: GetSystemMetrics has no preconditions.
    let size = unsafe { wam::GetSystemMetrics(wam::SM_CXSMICON) }.clamp(16, 64);
    let ink = if taskbar_uses_light_theme() {
        0x1f1f1f
    } else {
        0xffffff
    };
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size,
            biHeight: -size,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    // SAFETY: every GDI object created here is selected out and deleted
    // before returning; `bits` points at size*size pixels owned by `color`.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return None;
        }
        let mut bits: *mut c_void = null_mut();
        let Ok(color) = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
        else {
            let _ = DeleteDC(dc);
            return None;
        };
        let previous_bitmap = SelectObject(dc, color.into());
        let font = CreateFontW(
            -(size * 7 / 8),
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            ANTIALIASED_QUALITY,
            0,
            w!("Microsoft YaHei UI"),
        );
        let previous_font = SelectObject(dc, font.into());
        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, COLORREF(0x00ff_ffff));
        let mut text: Vec<u16> = glyph.encode_utf16().collect();
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: size,
            bottom: size,
        };
        DrawTextW(
            dc,
            &mut text,
            &mut rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        let _ = GdiFlush();
        if !bits.is_null() {
            let pixels = std::slice::from_raw_parts_mut(bits.cast::<u32>(), (size * size) as usize);
            tint_coverage(pixels, ink);
        }
        SelectObject(dc, previous_font);
        SelectObject(dc, previous_bitmap);
        let _ = DeleteObject(font.into());
        let mask_row_bytes = ((size + 15) / 16 * 2) as usize;
        let mask_bits = vec![0_u8; mask_row_bytes * size as usize];
        let mask = CreateBitmap(size, size, 1, 1, Some(mask_bits.as_ptr().cast()));
        let icon = wam::CreateIconIndirect(&wam::ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: color,
        })
        .ok();
        let _ = DeleteObject(mask.into());
        let _ = DeleteObject(color.into());
        let _ = DeleteDC(dc);
        icon
    }
}

/// Shows the right-click menu and returns the chosen command, or 0.
fn track_menu(point: POINT) -> usize {
    // SAFETY: GetUserDefaultUILanguage has no preconditions.
    let (toggle, settings) = menu_labels(unsafe { GetUserDefaultUILanguage() });
    let toggle: Vec<u16> = toggle.encode_utf16().chain([0]).collect();
    let settings: Vec<u16> = settings.encode_utf16().chain([0]).collect();
    // SAFETY: the owner window and menu are created, used and destroyed on
    // this thread; the label buffers outlive the menu.
    unsafe {
        let Ok(owner) = wam::CreateWindowExW(
            wam::WS_EX_TOOLWINDOW,
            w!("STATIC"),
            w!(""),
            wam::WS_POPUP,
            point.x,
            point.y,
            0,
            0,
            None,
            None,
            None,
            None,
        ) else {
            return 0;
        };
        let mut command = 0;
        if let Ok(menu) = wam::CreatePopupMenu() {
            let _ = wam::AppendMenuW(
                menu,
                wam::MF_STRING,
                MENU_TOGGLE,
                windows_core::PCWSTR(toggle.as_ptr()),
            );
            let _ = wam::AppendMenuW(menu, wam::MF_SEPARATOR, 0, windows_core::PCWSTR::null());
            let _ = wam::AppendMenuW(
                menu,
                wam::MF_STRING,
                MENU_SETTINGS,
                windows_core::PCWSTR(settings.as_ptr()),
            );
            // A popup owned by a background window would not close when the
            // user clicks elsewhere.
            let _ = wam::SetForegroundWindow(owner);
            command = wam::TrackPopupMenuEx(
                menu,
                (wam::TPM_RETURNCMD
                    | wam::TPM_NONOTIFY
                    | wam::TPM_RIGHTBUTTON
                    | wam::TPM_BOTTOMALIGN
                    | wam::TPM_LEFTALIGN)
                    .0,
                point.x,
                point.y,
                owner,
                None,
            )
            .0 as usize;
            let _ = wam::DestroyMenu(menu);
        }
        let _ = wam::DestroyWindow(owner);
        command
    }
}

#[implement(ITfLangBarItemButton, ITfLangBarItem, ITfSource)]
pub(crate) struct Fcitx5InputModeItem {
    service: Cell<*const Fcitx5TsfService_Impl>,
    sink: RefCell<Option<ITfLangBarItemSink>>,
    status: RefCell<Option<InputModeStatus>>,
}

impl Fcitx5InputModeItem {
    pub fn new(service: *const Fcitx5TsfService_Impl) -> Self {
        // TSF may hold the item after the service is gone, so the DLL must
        // stay loaded while it can still be called.
        module_add_ref();
        Self {
            service: Cell::new(service),
            sink: RefCell::new(None),
            status: RefCell::new(None),
        }
    }

    /// Called on Deactivate. TSF may keep the item alive a little longer,
    /// so later clicks must not reach the released service.
    pub fn detach(&self) {
        self.service.set(std::ptr::null());
        self.sink.borrow_mut().take();
    }

    pub fn set_status(&self, status: InputModeStatus) {
        if self.status.borrow().as_ref() == Some(&status) {
            return;
        }
        *self.status.borrow_mut() = Some(status);
        let sink = self.sink.borrow().clone();
        if let Some(sink) = sink {
            // SAFETY: the sink was advised by TSF on this thread.
            let _ = unsafe { sink.OnUpdate(TF_LBI_ICON | TF_LBI_TEXT | TF_LBI_TOOLTIP) };
        }
    }

    fn glyph(&self) -> String {
        self.status
            .borrow()
            .as_ref()
            .map(InputModeStatus::glyph)
            .unwrap_or_else(|| UNKNOWN_GLYPH.to_owned())
    }

    fn with_service(&self, operation: impl FnOnce(&Fcitx5TsfService_Impl)) {
        let service = self.service.get();
        if !service.is_null() {
            // SAFETY: the pointer is cleared by `detach` before the service
            // is released, and TSF calls the item on the service's thread.
            operation(unsafe { &*service });
        }
    }
}

impl Drop for Fcitx5InputModeItem {
    fn drop(&mut self) {
        module_release();
    }
}

impl ITfLangBarItem_Impl for Fcitx5InputModeItem_Impl {
    fn GetInfo(&self, info: *mut TF_LANGBARITEMINFO) -> Result<()> {
        if info.is_null() {
            return Err(windows::Win32::Foundation::E_INVALIDARG.into());
        }
        let mut description = [0_u16; 32];
        for (slot, unit) in description.iter_mut().zip("Fcitx5".encode_utf16()) {
            *slot = unit;
        }
        // SAFETY: TSF passes a writable TF_LANGBARITEMINFO.
        unsafe {
            *info = TF_LANGBARITEMINFO {
                clsidService: FCITX5_TEXT_SERVICE_CLSID,
                guidItem: GUID_LBI_INPUTMODE,
                dwStyle: TF_LBI_STYLE_BTN_BUTTON | TF_LBI_STYLE_SHOWNINTRAY,
                ulSort: 0,
                szDescription: description,
            };
        }
        Ok(())
    }

    fn GetStatus(&self) -> Result<u32> {
        Ok(0)
    }

    fn Show(&self, _show: BOOL) -> Result<()> {
        Ok(())
    }

    fn GetTooltipString(&self) -> Result<BSTR> {
        let tooltip = self
            .status
            .borrow()
            .as_ref()
            .map(InputModeStatus::tooltip)
            .unwrap_or_else(|| "Fcitx5".to_owned());
        Ok(BSTR::from(tooltip))
    }
}

impl ITfLangBarItemButton_Impl for Fcitx5InputModeItem_Impl {
    fn OnClick(&self, click: TfLBIClick, point: &POINT, _area: *const RECT) -> Result<()> {
        com_guard(Ok(()), || {
            if click == TF_LBI_CLK_LEFT {
                self.with_service(|service| service.toggle_input_mode());
            } else if click == TF_LBI_CLK_RIGHT {
                match track_menu(*point) {
                    MENU_TOGGLE => self.with_service(|service| service.toggle_input_mode()),
                    MENU_SETTINGS => self.with_service(|service| service.open_settings()),
                    _ => {}
                }
            }
            Ok(())
        })
    }

    fn InitMenu(&self, _menu: Ref<ITfMenu>) -> Result<()> {
        Ok(())
    }

    fn OnMenuSelect(&self, _id: u32) -> Result<()> {
        Ok(())
    }

    fn GetIcon(&self) -> Result<wam::HICON> {
        // The caller owns and destroys the returned icon.
        com_guard(Ok(wam::HICON::default()), || {
            Ok(render_icon(&self.glyph()).unwrap_or_default())
        })
    }

    fn GetText(&self) -> Result<BSTR> {
        Ok(BSTR::from(self.glyph()))
    }
}

impl ITfSource_Impl for Fcitx5InputModeItem_Impl {
    fn AdviseSink(&self, iid: *const GUID, sink: Ref<IUnknown>) -> Result<u32> {
        // SAFETY: TSF passes a valid IID pointer when it is non-null.
        if iid.is_null() || unsafe { *iid } != ITfLangBarItemSink::IID {
            return Err(CONNECT_E_CANNOTCONNECT.into());
        }
        if self.sink.borrow().is_some() {
            return Err(CONNECT_E_ADVISELIMIT.into());
        }
        let sink: ITfLangBarItemSink = sink.ok()?.cast()?;
        *self.sink.borrow_mut() = Some(sink);
        Ok(SINK_COOKIE)
    }

    fn UnadviseSink(&self, cookie: u32) -> Result<()> {
        if cookie != SINK_COOKIE || self.sink.borrow_mut().take().is_none() {
            return Err(CONNECT_E_CANNOTCONNECT.into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(id: &str, short_label: &str) -> InputModeStatus {
        InputModeStatus {
            id: id.to_owned(),
            short_label: short_label.to_owned(),
            ..InputModeStatus::default()
        }
    }

    #[test]
    fn glyph_reads_keyboards_as_english_and_keeps_engine_labels() {
        assert_eq!(status("keyboard-us", "en").glyph(), "英");
        assert_eq!(status("pinyin", "拼").glyph(), "拼");
        assert_eq!(status("rime", "中州韻").glyph(), "中");
        assert_eq!(status("mozc", "あ").glyph(), "あ");
        assert_eq!(status("hangul", "ko").glyph(), "ko");
        assert_eq!(status("", "").glyph(), "中");
    }

    #[test]
    fn tooltip_prefers_the_native_name() {
        let mut mode = status("pinyin", "拼");
        mode.name = "Pinyin".to_owned();
        assert_eq!(mode.tooltip(), "Fcitx5 · Pinyin");
        mode.native_name = "拼音".to_owned();
        assert_eq!(mode.tooltip(), "Fcitx5 · 拼音");
        assert_eq!(InputModeStatus::default().tooltip(), "Fcitx5");
    }

    #[test]
    fn menu_follows_the_display_language() {
        assert_eq!(menu_labels(0x0804).1, "设置");
        assert_eq!(menu_labels(0x0404).1, "設定");
        assert_eq!(menu_labels(0x0411).1, "設定");
        assert_eq!(menu_labels(0x0412).1, "설정");
        assert_eq!(menu_labels(0x0409).1, "Settings");
    }

    #[test]
    fn coverage_becomes_alpha_over_the_ink_color() {
        let mut pixels = [0x0000_0000, 0x00ff_ffff, 0x0040_8020];
        tint_coverage(&mut pixels, 0x1f1f1f);
        assert_eq!(pixels, [0, 0xff1f_1f1f, 0x801f_1f1f]);
    }
}
