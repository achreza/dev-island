pub mod autostart;
pub mod monitors;
pub mod shortcuts;
pub mod tray;
pub mod windows;

pub use autostart::AutostartManager;
pub use monitors::MonitorGeometry;
pub use shortcuts::{GlobalShortcut, HOTKEY_TOGGLE_ID};
pub use tray::{
    TrayIcon, IDM_EXIT, IDM_TOGGLE_AUTOSTART, IDM_TOGGLE_EXPAND, IDM_TOGGLE_VISIBILITY,
    WM_TRAYICON,
};
pub use windows::DibBuffer;
