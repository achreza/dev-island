use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;

#[derive(Debug, Clone, Copy)]
pub struct MonitorGeometry {
    pub work_left: i32,
    pub work_top: i32,
    pub work_width: i32,
    #[allow(dead_code)]
    pub work_height: i32,
    #[allow(dead_code)]
    pub dpi: u32,
    pub scale: f32,
}

impl MonitorGeometry {
    pub fn for_window(hwnd: HWND) -> Self {
        unsafe {
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };

            let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
            let _ = GetMonitorInfoW(hmon, &mut mi);

            let dpi = GetDpiForWindow(hwnd);
            let dpi = if dpi == 0 { 96 } else { dpi };
            let scale = dpi as f32 / 96.0;

            let work_left = mi.rcWork.left;
            let work_top = mi.rcWork.top;
            let work_width = mi.rcWork.right - mi.rcWork.left;
            let work_height = mi.rcWork.bottom - mi.rcWork.top;

            Self {
                work_left,
                work_top,
                work_width,
                work_height,
                dpi,
                scale,
            }
        }
    }

    /// Calculate window position centered at top of monitor work area.
    pub fn calculate_window_pos(&self, window_width: i32, _window_height: i32, top_offset: i32) -> (i32, i32) {
        let x = self.work_left + (self.work_width - window_width) / 2;
        let y = self.work_top + (top_offset as f32 * self.scale) as i32;
        (x, y)
    }
}
