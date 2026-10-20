use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, LoadIconW,
    SetForegroundWindow, TrackPopupMenuEx,
    HICON, IDI_APPLICATION, MF_SEPARATOR, MF_STRING, TPM_BOTTOMALIGN, TPM_RIGHTBUTTON,
    WM_USER,
};

pub const WM_TRAYICON: u32 = WM_USER + 200;

pub const IDM_TOGGLE_VISIBILITY: usize = 2001;
pub const IDM_TOGGLE_EXPAND: usize = 2002;
pub const IDM_TOGGLE_AUTOSTART: usize = 2004;
pub const IDM_EXIT: usize = 2003;

pub struct TrayIcon {
    hwnd: HWND,
    is_added: bool,
}

impl TrayIcon {
    pub fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            is_added: false,
        }
    }

    pub fn add(&mut self) -> bool {
        unsafe {
            let h_icon = LoadIconW(None, IDI_APPLICATION).unwrap_or(HICON::default());

            let mut tip = [0u16; 128];
            let tip_text: Vec<u16> = "Dev Dynamic Island\0".encode_utf16().collect();
            let copy_len = tip_text.len().min(127);
            tip[..copy_len].copy_from_slice(&tip_text[..copy_len]);

            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: 1,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: WM_TRAYICON,
                hIcon: h_icon,
                szTip: tip,
                ..Default::default()
            };

            let success = Shell_NotifyIconW(NIM_ADD, &nid).as_bool();
            self.is_added = success;
            if success {
                println!("System tray icon registered.");
            } else {
                eprintln!("Warning: Failed to create system tray icon.");
            }
            success
        }
    }

    pub fn show_context_menu(&self, is_visible: bool, is_expanded: bool, is_autostart: bool) {
        unsafe {
            let hmenu = match CreatePopupMenu() {
                Ok(m) => m,
                Err(_) => return,
            };

            let vis_label = if is_visible {
                w!("Hide Island")
            } else {
                w!("Show Island")
            };

            let exp_label = if is_expanded {
                w!("Collapse Island")
            } else {
                w!("Expand Island")
            };

            let auto_label = if is_autostart {
                w!("Start with Windows: [Enabled]")
            } else {
                w!("Start with Windows: [Disabled]")
            };

            let _ = AppendMenuW(hmenu, MF_STRING, IDM_TOGGLE_VISIBILITY, vis_label);
            let _ = AppendMenuW(hmenu, MF_STRING, IDM_TOGGLE_EXPAND, exp_label);
            let _ = AppendMenuW(hmenu, MF_STRING, IDM_TOGGLE_AUTOSTART, auto_label);
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
            let _ = AppendMenuW(hmenu, MF_STRING, IDM_EXIT, w!("Exit Dev Island"));

            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);

            // Ensure menu dismisses when clicking away
            let _ = SetForegroundWindow(self.hwnd);
            let _ = TrackPopupMenuEx(
                hmenu,
                (TPM_RIGHTBUTTON | TPM_BOTTOMALIGN).0,
                pt.x,
                pt.y,
                self.hwnd,
                None,
            );
            let _ = DestroyMenu(hmenu);
        }
    }

    pub fn remove(&mut self) {
        if self.is_added {
            unsafe {
                let nid = NOTIFYICONDATAW {
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: self.hwnd,
                    uID: 1,
                    ..Default::default()
                };
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            }
            self.is_added = false;
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.remove();
    }
}
