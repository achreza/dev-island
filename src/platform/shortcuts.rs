use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT,
};

pub const HOTKEY_TOGGLE_ID: i32 = 101;

pub struct GlobalShortcut {
    hwnd: HWND,
    is_registered: bool,
}

impl GlobalShortcut {
    pub fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            is_registered: false,
        }
    }

    /// Registers Ctrl + Shift + D to toggle Dynamic Island.
    pub fn register(&mut self) -> bool {
        unsafe {
            // 'D' key virtual code is 0x44
            let modifiers = HOT_KEY_MODIFIERS(MOD_CONTROL.0 | MOD_SHIFT.0 | MOD_NOREPEAT.0);
            let success = RegisterHotKey(self.hwnd, HOTKEY_TOGGLE_ID, modifiers, 0x44).is_ok();
            self.is_registered = success;
            if success {
                println!("Global shortcut Ctrl + Shift + D registered successfully.");
            } else {
                eprintln!("Warning: Failed to register global shortcut Ctrl + Shift + D.");
            }
            success
        }
    }

    pub fn unregister(&mut self) {
        if self.is_registered {
            unsafe {
                let _ = UnregisterHotKey(self.hwnd, HOTKEY_TOGGLE_ID);
            }
            self.is_registered = false;
        }
    }
}

impl Drop for GlobalShortcut {
    fn drop(&mut self) {
        self.unregister();
    }
}
