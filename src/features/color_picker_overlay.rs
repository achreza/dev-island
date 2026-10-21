//! Interactive Screen Eyedropper / Color Picker Overlay
//!
//! Provides a modern, screenshot-style interactive color picker:
//! - Dims the entire desktop with a crosshair cursor (`IDC_CROSS`)
//! - Shows a floating loupe with live color swatch, `#RRGGBB` and `rgb(...)`
//! - Left-click extracts the selected pixel color and copies to Windows clipboard
//! - Right-click or Escape cancels the picker

use std::cell::RefCell;
use std::rc::Rc;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetDC, GetPixel, ReleaseDC, HBRUSH,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos,
    GetMessageW, GetSystemMetrics, GetWindowLongPtrW, LoadCursorW,
    RegisterClassW, SetCursor, SetLayeredWindowAttributes, SetWindowLongPtrW,
    ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, HICON,
    IDC_CROSS, LWA_ALPHA, MSG, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN, SW_SHOW,
    WM_DESTROY, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_RBUTTONUP,
    WM_SETCURSOR, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::features::utilities::ScreenColor;
use crate::platform::DibBuffer;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, PixmapMut, Rect, Stroke, Transform};

const OVERLAY_CLASS_NAME: PCWSTR = w!("DynamicIslandColorPickerOverlay");
const LOUPE_CLASS_NAME: PCWSTR = w!("DynamicIslandColorPickerLoupe");

struct ColorPickerSession {
    _overlay_hwnd: HWND,
    loupe_hwnd: HWND,
    loupe_dib: DibBuffer,
    loupe_pixmap: tiny_skia::Pixmap,
    font: Option<fontdue::Font>,
    picked_color: Option<ScreenColor>,
    is_finished: bool,
}

pub struct InteractiveColorPicker;

impl InteractiveColorPicker {
    /// Launches the interactive screen eyedropper modal.
    /// Blocks until user left-clicks (picking color) or presses Esc/right-clicks (canceling).
    /// Returns `Some(ScreenColor)` if picked, or `None` if canceled.
    pub fn pick_color() -> Option<ScreenColor> {
        unsafe {
            let h_instance = match GetModuleHandleW(None) {
                Ok(h) => h,
                Err(_) => return None,
            };

            let cross_cursor = LoadCursorW(None, IDC_CROSS).unwrap_or_default();

            // 1. Register Fullscreen Dim Overlay Class
            let overlay_wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(overlay_wnd_proc),
                hInstance: h_instance.into(),
                lpszClassName: OVERLAY_CLASS_NAME,
                hbrBackground: HBRUSH(windows::Win32::Graphics::Gdi::GetStockObject(windows::Win32::Graphics::Gdi::BLACK_BRUSH).0),
                hCursor: cross_cursor,
                hIcon: HICON(std::ptr::null_mut()),
                ..Default::default()
            };
            let _ = RegisterClassW(&overlay_wc);

            // 2. Register Loupe Class
            let loupe_wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(loupe_wnd_proc),
                hInstance: h_instance.into(),
                lpszClassName: LOUPE_CLASS_NAME,
                hbrBackground: HBRUSH(std::ptr::null_mut()),
                hCursor: cross_cursor,
                hIcon: HICON(std::ptr::null_mut()),
                ..Default::default()
            };
            let _ = RegisterClassW(&loupe_wc);

            // Virtual Desktop Geometry across all monitors
            let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);

            // Create Fullscreen Overlay Window
            let overlay_hwnd = match CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                OVERLAY_CLASS_NAME,
                w!("ColorPickerOverlay"),
                WS_POPUP,
                vx,
                vy,
                vw,
                vh,
                None,
                None,
                h_instance,
                None,
            ) {
                Ok(h) => h,
                Err(_) => return None,
            };

            // Set subtle screen dimming (alpha = 35 / 255)
            let _ = SetLayeredWindowAttributes(overlay_hwnd, COLORREF(0), 35, LWA_ALPHA);

            // Loupe dimensions: 140x50
            let loupe_w = 140;
            let loupe_h = 50;

            let loupe_hwnd = match CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                LOUPE_CLASS_NAME,
                w!("ColorPickerLoupe"),
                WS_POPUP,
                0,
                0,
                loupe_w,
                loupe_h,
                None,
                None,
                h_instance,
                None,
            ) {
                Ok(h) => h,
                Err(_) => {
                    let _ = DestroyWindow(overlay_hwnd);
                    return None;
                }
            };

            let loupe_dib = match DibBuffer::new(loupe_w, loupe_h) {
                Ok(d) => d,
                Err(_) => {
                    let _ = DestroyWindow(loupe_hwnd);
                    let _ = DestroyWindow(overlay_hwnd);
                    return None;
                }
            };

            let loupe_pixmap = match tiny_skia::Pixmap::new(loupe_w as u32, loupe_h as u32) {
                Some(p) => p,
                None => {
                    let _ = DestroyWindow(loupe_hwnd);
                    let _ = DestroyWindow(overlay_hwnd);
                    return None;
                }
            };

            let font = Self::load_font();

            let session = Rc::new(RefCell::new(ColorPickerSession {
                _overlay_hwnd: overlay_hwnd,
                loupe_hwnd,
                loupe_dib,
                loupe_pixmap,
                font,
                picked_color: None,
                is_finished: false,
            }));

            let session_ptr = Rc::into_raw(session.clone());
            SetWindowLongPtrW(overlay_hwnd, GWLP_USERDATA, session_ptr as isize);

            // Show fullscreen overlay and initial cursor position
            let _ = ShowWindow(overlay_hwnd, SW_SHOW);
            let _ = ShowWindow(loupe_hwnd, SW_SHOW);

            // Initial sampling at current cursor position
            let mut pt = POINT::default();
            if GetCursorPos(&mut pt).is_ok() {
                Self::update_loupe(&mut session.borrow_mut(), pt.x, pt.y);
            }

            // Dedicated modal message loop for color picker
            let mut msg = MSG::default();
            while !session.borrow().is_finished {
                let ret = GetMessageW(&mut msg, None, 0, 0);
                if ret.0 <= 0 {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // Extract result
            let session_ref = Rc::from_raw(session_ptr);
            let result = session_ref.borrow().picked_color.clone();

            let _ = DestroyWindow(loupe_hwnd);
            let _ = DestroyWindow(overlay_hwnd);

            result
        }
    }

    fn sample_screen_pixel(x: i32, y: i32) -> Option<ScreenColor> {
        unsafe {
            let hdc = GetDC(HWND(std::ptr::null_mut()));
            if hdc.is_invalid() {
                return None;
            }

            let pixel: COLORREF = GetPixel(hdc, x, y);
            let _ = ReleaseDC(HWND(std::ptr::null_mut()), hdc);

            if pixel.0 == 0xFFFFFFFF {
                return None;
            }

            let r = (pixel.0 & 0x000000FF) as u8;
            let g = ((pixel.0 & 0x0000FF00) >> 8) as u8;
            let b = ((pixel.0 & 0x00FF0000) >> 16) as u8;

            let hex = format!("#{:02X}{:02X}{:02X}", r, g, b);
            let rgb_str = format!("rgb({}, {}, {})", r, g, b);

            Some(ScreenColor {
                hex,
                r,
                g,
                b,
                rgb_str,
                cursor_x: x,
                cursor_y: y,
            })
        }
    }

    fn update_loupe(session: &mut ColorPickerSession, cursor_x: i32, cursor_y: i32) {
        let color = match Self::sample_screen_pixel(cursor_x, cursor_y) {
            Some(c) => c,
            None => return,
        };

        // Render loupe content
        session.loupe_pixmap.fill(Color::TRANSPARENT);

        let w = 140.0;
        let h = 50.0;
        let r = 8.0;

        // Dark obsidian background pill
        let bg_color = Color::from_rgba8(18, 18, 22, 235);
        if let Some(card_path) = crate::render::drawing::rounded_rect_path(0.0, 0.0, w, h, r) {
            let mut p = Paint::default();
            p.set_color(bg_color);
            p.anti_alias = true;
            session.loupe_pixmap.fill_path(&card_path, &p, FillRule::Winding, Transform::identity(), None);

            // Subtle border
            let mut sp = Paint::default();
            sp.set_color(Color::from_rgba8(255, 255, 255, 45));
            let stroke = Stroke { width: 1.0, ..Default::default() };
            session.loupe_pixmap.stroke_path(&card_path, &sp, &stroke, Transform::identity(), None);
        }

        // Color Swatch Box (26x26)
        let swatch_rect = Rect::from_xywh(10.0, 12.0, 26.0, 26.0);
        if let Some(rect) = swatch_rect {
            let mut p = Paint::default();
            p.set_color(Color::from_rgba8(color.r, color.g, color.b, 255));
            p.anti_alias = true;
            session.loupe_pixmap.fill_rect(rect, &p, Transform::identity(), None);

            let mut sp = Paint::default();
            sp.set_color(Color::from_rgba8(255, 255, 255, 180));
            let stroke = Stroke { width: 1.0, ..Default::default() };
            let border_path = PathBuilder::from_rect(rect);
            session.loupe_pixmap.stroke_path(&border_path, &sp, &stroke, Transform::identity(), None);
        }

        // Typography: Hex code and RGB
        if let Some(ref font) = session.font {
            // Hex title (e.g. #3B82F6)
            Self::draw_text(
                &mut session.loupe_pixmap.as_mut(),
                font,
                &color.hex,
                42.0,
                24.0,
                13.0,
                255,
                255,
                255,
                255,
            );

            // Small RGB label (e.g. 59, 130, 246)
            let rgb_label = format!("{}, {}, {}", color.r, color.g, color.b);
            Self::draw_text(
                &mut session.loupe_pixmap.as_mut(),
                font,
                &rgb_label,
                42.0,
                38.0,
                9.5,
                160,
                160,
                170,
                230,
            );
        }

        // Position loupe offset from cursor (+16, +16) so it never obscures the pixel
        let mut loupe_x = cursor_x + 16;
        let mut loupe_y = cursor_y + 16;

        let vw = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let vh = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

        if loupe_x + 140 > vw {
            loupe_x = cursor_x - 156;
        }
        if loupe_y + 50 > vh {
            loupe_y = cursor_y - 66;
        }

        session.loupe_dib.update_layered_window(
            session.loupe_hwnd,
            session.loupe_pixmap.data(),
            loupe_x,
            loupe_y,
        );
    }

    fn draw_text(
        pixmap: &mut PixmapMut,
        font: &fontdue::Font,
        text: &str,
        start_x: f32,
        baseline_y: f32,
        size: f32,
        r: u8,
        g: u8,
        b: u8,
        alpha: u8,
    ) {
        let mut cur_x = start_x;
        for ch in text.chars() {
            let (metrics, bitmap) = font.rasterize(ch, size);
            let gx = (cur_x + metrics.xmin as f32).round() as i32;
            let gy = (baseline_y - metrics.height as f32 - metrics.ymin as f32).round() as i32;

            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let coverage = bitmap[row * metrics.width + col];
                    if coverage > 0 {
                        let px = gx + col as i32;
                        let py = gy + row as i32;
                        if px >= 0 && px < pixmap.width() as i32 && py >= 0 && py < pixmap.height() as i32 {
                            let factor = (coverage as f32 / 255.0) * (alpha as f32 / 255.0);
                            let a = (255.0 * factor) as u8;
                            let pr = (r as f32 * factor) as u8;
                            let pg = (g as f32 * factor) as u8;
                            let pb = (b as f32 * factor) as u8;

                            let idx = (py as usize * pixmap.width() as usize + px as usize) * 4;
                            let slice = &mut pixmap.data_mut()[idx..idx + 4];

                            let inv_a = 1.0 - factor;
                            slice[0] = (pr as f32 + slice[0] as f32 * inv_a) as u8;
                            slice[1] = (pg as f32 + slice[1] as f32 * inv_a) as u8;
                            slice[2] = (pb as f32 + slice[2] as f32 * inv_a) as u8;
                            slice[3] = (a as f32 + slice[3] as f32 * inv_a) as u8;
                        }
                    }
                }
            }
            cur_x += metrics.advance_width;
        }
    }

    fn load_font() -> Option<fontdue::Font> {
        let candidates = [
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
        ];
        for path in &candidates {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(font) = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                    return Some(font);
                }
            }
        }
        None
    }
}

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let session_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut RefCell<ColorPickerSession> };

    match msg {
        WM_SETCURSOR => {
            unsafe {
                let cross = LoadCursorW(None, IDC_CROSS).unwrap_or_default();
                SetCursor(cross);
            }
            LRESULT(1)
        }

        WM_MOUSEMOVE => {
            if !session_ptr.is_null() {
                let mut pt = POINT::default();
                if unsafe { GetCursorPos(&mut pt).is_ok() } {
                    let session = unsafe { &*session_ptr };
                    InteractiveColorPicker::update_loupe(&mut session.borrow_mut(), pt.x, pt.y);
                }
            }
            LRESULT(0)
        }

        WM_LBUTTONDOWN => {
            if !session_ptr.is_null() {
                let session = unsafe { &*session_ptr };
                let mut pt = POINT::default();
                if unsafe { GetCursorPos(&mut pt).is_ok() } {
                    if let Some(color) = InteractiveColorPicker::sample_screen_pixel(pt.x, pt.y) {
                        session.borrow_mut().picked_color = Some(color);
                    }
                }
                session.borrow_mut().is_finished = true;
                unsafe {
                    let _ = DestroyWindow(session.borrow().loupe_hwnd);
                    let _ = DestroyWindow(hwnd);
                }
            }
            LRESULT(0)
        }

        WM_RBUTTONUP => {
            if !session_ptr.is_null() {
                let session = unsafe { &*session_ptr };
                session.borrow_mut().is_finished = true;
                unsafe {
                    let _ = DestroyWindow(session.borrow().loupe_hwnd);
                    let _ = DestroyWindow(hwnd);
                }
            }
            LRESULT(0)
        }

        WM_KEYDOWN => {
            if wparam.0 == VK_ESCAPE.0 as usize {
                if !session_ptr.is_null() {
                    let session = unsafe { &*session_ptr };
                    session.borrow_mut().is_finished = true;
                    unsafe {
                        let _ = DestroyWindow(session.borrow().loupe_hwnd);
                        let _ = DestroyWindow(hwnd);
                    }
                }
            }
            LRESULT(0)
        }

        WM_DESTROY => {
            if !session_ptr.is_null() {
                let session = unsafe { &*session_ptr };
                session.borrow_mut().is_finished = true;
            }
            LRESULT(0)
        }

        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe extern "system" fn loupe_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_SETCURSOR => {
            unsafe {
                let cross = LoadCursorW(None, IDC_CROSS).unwrap_or_default();
                SetCursor(cross);
            }
            LRESULT(1)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
