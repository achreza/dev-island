use std::cell::RefCell;
use std::time::Instant;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{HBRUSH, ScreenToClient};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_ESCAPE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetMessageW, GetWindowLongPtrW, KillTimer, PostQuitMessage, RegisterClassW,
    SetForegroundWindow, SetTimer, SetWindowLongPtrW, ShowWindow, TranslateMessage,
    CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, HCURSOR, MSG, SW_HIDE, SW_SHOW,
    WNDCLASSW,
    WA_INACTIVE, WM_ACTIVATE, WM_COMMAND, WM_DESTROY, WM_DISPLAYCHANGE,
    WM_DPICHANGED, WM_HOTKEY, WM_KEYDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_NCHITTEST, WM_RBUTTONUP, WM_SETTINGCHANGE, WM_TIMER,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    HTCLIENT, HTTRANSPARENT,
    IDC_ARROW, IDC_HAND, LoadCursorW, SetCursor,
};

const WM_MOUSELEAVE: u32 = 0x02A3;

use crate::animation::Animation;
use crate::config::AppConfig;
use crate::features::{
    CommandRunner, DeveloperUtilities, DevPortsMonitor, FocusTimer, FocusTimerStatus, GitStatusMonitor,
    InteractiveColorPicker, QuickLinkLauncher, SnippetManager, SystemMonitor, TaskNote,
};
use crate::platform::{
    AutostartManager, DibBuffer, GlobalShortcut, MonitorGeometry, TrayIcon, HOTKEY_TOGGLE_ID,
    IDM_EXIT, IDM_TOGGLE_AUTOSTART, IDM_TOGGLE_EXPAND, IDM_TOGGLE_VISIBILITY, WM_TRAYICON,
};
use crate::render::{ButtonArea, Renderer};
use crate::state::{ActiveTab, AppStatus, IslandMode, Visibility};

const CLASS_NAME: PCWSTR = w!("DynamicIslandDevClass");
const WINDOW_TITLE: PCWSTR = w!("DynamicIslandOverlay");
const TIMER_ANIM_ID: usize = 1001;
const TIMER_FOCUS_ID: usize = 1002;
const TIMER_TOAST_ID: usize = 1003;

pub struct AppState {
    pub hwnd: HWND,
    pub config: AppConfig,
    pub status: AppStatus,
    pub anim: Animation,
    pub renderer: Renderer,
    pub timer: FocusTimer,
    pub task_note: TaskNote,
    pub snippets: SnippetManager,
    pub git_monitor: GitStatusMonitor,
    pub dev_ports: DevPortsMonitor,
    pub sys_monitor: SystemMonitor,
    pub cmd_runner: CommandRunner,
    pub dib: DibBuffer,
    pub pixmap: tiny_skia::Pixmap,
    pub geometry: MonitorGeometry,
    pub tray: TrayIcon,
    pub shortcut: GlobalShortcut,
    pub canvas_width: i32,
    pub canvas_height: i32,
    pub win_x: i32,
    pub win_y: i32,
    pub buttons: Vec<ButtonArea>,
    pub hovered_button: Option<&'static str>,
    pub hovered_pill: bool,
    pub active_tab: ActiveTab,
    pub copied_toast: Option<(&'static str, Instant)>,
    pub mouse_tracked: bool,
}

impl AppState {
    pub fn new(hwnd: HWND) -> Self {
        let config = AppConfig::load_or_default();
        let geometry = MonitorGeometry::for_window(hwnd);
        let scale = geometry.scale;

        let canvas_width = (440.0 * scale).round() as i32;
        let canvas_height = (210.0 * scale).round() as i32;

        let (win_x, win_y) = geometry.calculate_window_pos(canvas_width, canvas_height, 10);

        let dib = DibBuffer::new(canvas_width, canvas_height)
            .expect("Failed to create Win32 DIBSection buffer");

        let pixmap = tiny_skia::Pixmap::new(canvas_width as u32, canvas_height as u32)
            .expect("Failed to create tiny-skia Pixmap");

        let renderer = Renderer::new();
        let anim = Animation::new();
        let mut tray = TrayIcon::new(hwnd);
        tray.add();

        let mut shortcut = GlobalShortcut::new(hwnd);
        shortcut.register();

        let timer = FocusTimer::new(config.focus_duration_mins * 60);
        let task_note = TaskNote::new(&config.task_note);
        let snippets = SnippetManager::new(config.snippets.clone());
        let repo_path = config.git_repo_path.clone().unwrap_or_else(|| ".".to_string());
        let git_monitor = GitStatusMonitor::new(repo_path);
        let dev_ports = DevPortsMonitor::new();
        let sys_monitor = SystemMonitor::new();
        let cmd_runner = CommandRunner::new();

        Self {
            hwnd,
            config,
            status: AppStatus::default(),
            anim,
            renderer,
            timer,
            task_note,
            snippets,
            git_monitor,
            dev_ports,
            sys_monitor,
            cmd_runner,
            dib,
            pixmap,
            geometry,
            tray,
            shortcut,
            canvas_width,
            canvas_height,
            win_x,
            win_y,
            buttons: Vec::new(),
            hovered_button: None,
            hovered_pill: false,
            active_tab: ActiveTab::Dev,
            copied_toast: None,
            mouse_tracked: false,
        }
    }

    pub fn render_frame(&mut self) {
        if self.status.visibility == Visibility::Hidden {
            return;
        }

        let scale = self.geometry.scale;
        let toast_msg = self.copied_toast.map(|(msg, _)| msg);
        let git_status = self.git_monitor.get_status();

        let sys_metrics = if self.anim.current_progress > 0.05 {
            Some(self.sys_monitor.get_metrics())
        } else {
            None
        };

        let active_ports = if self.anim.current_progress > 0.05 {
            self.dev_ports.get_active_ports()
        } else {
            &[]
        };

        self.buttons = self.renderer.draw(
            &mut self.pixmap.as_mut(),
            &self.anim,
            scale,
            &self.timer,
            &self.task_note,
            git_status.as_ref(),
            &self.config.quick_links,
            &self.snippets.snippets,
            toast_msg,
            self.hovered_button,
            self.hovered_pill,
            self.active_tab,
            &self.cmd_runner,
            sys_metrics,
            active_ports,
        );

        self.dib.update_layered_window(
            self.hwnd,
            self.pixmap.data(),
            self.win_x,
            self.win_y,
        );
    }

    pub fn start_animation_timer(&self) {
        unsafe {
            let _ = SetTimer(self.hwnd, TIMER_ANIM_ID, 16, None);
        }
    }

    pub fn stop_animation_timer(&self) {
        unsafe {
            let _ = KillTimer(self.hwnd, TIMER_ANIM_ID);
        }
    }

    pub fn start_focus_timer(&self) {
        unsafe {
            let _ = SetTimer(self.hwnd, TIMER_FOCUS_ID, 1000, None);
        }
    }

    pub fn stop_focus_timer(&self) {
        unsafe {
            let _ = KillTimer(self.hwnd, TIMER_FOCUS_ID);
        }
    }

    pub fn show_copied_toast(&mut self, msg: &'static str) {
        self.copied_toast = Some((msg, Instant::now()));
        unsafe {
            let _ = SetTimer(self.hwnd, TIMER_TOAST_ID, 2000, None);
        }
        self.render_frame();
    }

    pub fn expand_island(&mut self) {
        self.status.mode = IslandMode::Expanded;
        self.anim.expand();
        self.start_animation_timer();
        unsafe {
            let _ = SetForegroundWindow(self.hwnd);
        }
    }

    pub fn collapse_island(&mut self) {
        self.status.mode = IslandMode::Collapsed;
        self.hovered_button = None;
        self.hovered_pill = false;
        self.anim.collapse();
        self.start_animation_timer();
    }

    pub fn toggle_expand(&mut self) {
        if self.anim.target_progress >= 0.5 {
            self.collapse_island();
        } else {
            self.expand_island();
        }
    }

    pub fn set_visible(&mut self, visible: bool) {
        if visible {
            self.status.visibility = Visibility::Visible;
            unsafe {
                let _ = ShowWindow(self.hwnd, SW_SHOW);
            }
            self.render_frame();
        } else {
            self.status.visibility = Visibility::Hidden;
            unsafe {
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
        }
    }

    pub fn toggle_visibility(&mut self) {
        let is_visible = self.status.visibility == Visibility::Visible;
        self.set_visible(!is_visible);
    }

    pub fn handle_dpi_change(&mut self, new_dpi: u32) {
        let new_scale = new_dpi as f32 / 96.0;
        self.geometry = MonitorGeometry::for_window(self.hwnd);

        let canvas_width = (440.0 * new_scale).round() as i32;
        let canvas_height = (210.0 * new_scale).round() as i32;

        if let Ok(new_dib) = DibBuffer::new(canvas_width, canvas_height) {
            self.dib = new_dib;
        }

        if let Some(new_pixmap) = tiny_skia::Pixmap::new(canvas_width as u32, canvas_height as u32) {
            self.pixmap = new_pixmap;
        }

        self.canvas_width = canvas_width;
        self.canvas_height = canvas_height;
        let (win_x, win_y) = self.geometry.calculate_window_pos(canvas_width, canvas_height, 10);
        self.win_x = win_x;
        self.win_y = win_y;

        self.render_frame();
    }

    pub fn handle_display_change(&mut self) {
        self.geometry = MonitorGeometry::for_window(self.hwnd);
        let (win_x, win_y) = self.geometry.calculate_window_pos(self.canvas_width, self.canvas_height, 10);
        self.win_x = win_x;
        self.win_y = win_y;
        self.render_frame();
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let h_instance = GetModuleHandleW(None)?;

        use windows::Win32::UI::WindowsAndMessaging::LoadIconW;
        let h_icon = LoadIconW(h_instance, PCWSTR(1 as _)).unwrap_or_default();

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: h_instance.into(),
            lpszClassName: CLASS_NAME,
            hbrBackground: HBRUSH(std::ptr::null_mut()),
            hCursor: HCURSOR(std::ptr::null_mut()),
            hIcon: h_icon,
            ..Default::default()
        };

        let atom = RegisterClassW(&wc);
        if atom == 0 {
            eprintln!("RegisterClassW returned 0");
        }

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS_NAME,
            WINDOW_TITLE,
            WS_POPUP,
            0,
            0,
            100,
            100,
            None,
            None,
            h_instance,
            None,
        )?;

        let state = Box::new(RefCell::new(AppState::new(hwnd)));
        let state_ptr = Box::into_raw(state);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

        {
            let mut state_ref = (*state_ptr).borrow_mut();
            state_ref.render_frame();
        }

        let _ = ShowWindow(hwnd, SW_SHOW);

        println!("Dynamic Island running with Highly Interactive UI & Features.");
        println!("- Interactive Multi-Tabs: Dev Commands, Focus Timer Presets, and System Port Killer.");
        println!("- Global Shortcut: Ctrl + Shift + D.");

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = Box::from_raw(state_ptr);
    }

    Ok(())
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut RefCell<AppState> };

    if state_ptr.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    let state_cell = unsafe { &*state_ptr };

    match msg {
        WM_NCHITTEST => {
            let mut pt = POINT {
                x: (lparam.0 & 0xFFFF) as i16 as i32,
                y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
            };

            unsafe {
                let _ = ScreenToClient(hwnd, &mut pt);
            }

            let state = state_cell.borrow();
            let scale = state.geometry.scale;
            let (pill_w, pill_h) = state.anim.current_dimensions(scale);
            let pill_x = (state.canvas_width as f32 - pill_w) / 2.0;
            let pill_y = 8.0 * scale;

            if pt.x as f32 >= pill_x
                && pt.x as f32 <= pill_x + pill_w
                && pt.y as f32 >= pill_y
                && pt.y as f32 <= pill_y + pill_h
            {
                LRESULT(HTCLIENT as isize)
            } else {
                LRESULT(HTTRANSPARENT as isize)
            }
        }

        WM_MOUSEMOVE => {
            let client_x = (lparam.0 & 0xFFFF) as i16 as f32;
            let client_y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;

            let mut state = state_cell.borrow_mut();

            // Track mouse leave so hover effects clear when mouse exits window
            if !state.mouse_tracked {
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                unsafe {
                    let _ = TrackMouseEvent(&mut tme);
                }
                state.mouse_tracked = true;
            }

            let scale = state.geometry.scale;
            let (pill_w, pill_h) = state.anim.current_dimensions(scale);
            let pill_x = (state.canvas_width as f32 - pill_w) / 2.0;
            let pill_y = 8.0 * scale;

            let inside_pill = client_x >= pill_x
                && client_x <= pill_x + pill_w
                && client_y >= pill_y
                && client_y <= pill_y + pill_h;

            if state.anim.current_progress < 0.2 {
                // Collapsed state: highlight pill on hover
                if state.hovered_pill != inside_pill {
                    state.hovered_pill = inside_pill;
                    state.render_frame();
                }

                if inside_pill {
                    unsafe {
                        let h_hand = LoadCursorW(None, IDC_HAND).unwrap_or_default();
                        let _ = SetCursor(h_hand);
                    }
                }
            } else if state.anim.current_progress > 0.8 {
                // Expanded state: hit test interactive buttons
                let mut hovered = None;
                for btn in &state.buttons {
                    if client_x >= btn.x
                        && client_x <= btn.x + btn.w
                        && client_y >= btn.y
                        && client_y <= btn.y + btn.h
                    {
                        hovered = Some(btn.id);
                        break;
                    }
                }

                if state.hovered_button != hovered {
                    state.hovered_button = hovered;
                    state.render_frame();
                }

                if hovered.is_some() {
                    unsafe {
                        let h_hand = LoadCursorW(None, IDC_HAND).unwrap_or_default();
                        let _ = SetCursor(h_hand);
                    }
                } else {
                    unsafe {
                        let h_arrow = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
                        let _ = SetCursor(h_arrow);
                    }
                }
            }
            LRESULT(0)
        }

        WM_MOUSELEAVE => {
            let mut state = state_cell.borrow_mut();
            state.mouse_tracked = false;
            let mut needs_render = false;
            if state.hovered_pill {
                state.hovered_pill = false;
                needs_render = true;
            }
            if state.hovered_button.is_some() {
                state.hovered_button = None;
                needs_render = true;
            }
            if needs_render {
                state.render_frame();
            }
            LRESULT(0)
        }

        WM_LBUTTONUP => {
            let client_x = (lparam.0 & 0xFFFF) as i16 as f32;
            let client_y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;

            let mut should_pick_color = false;
            {
                let mut state = state_cell.borrow_mut();
                if state.anim.current_progress >= 0.8 {
                    let mut action_handled = false;
                    for btn in state.buttons.clone() {
                        if client_x >= btn.x
                            && client_x <= btn.x + btn.w
                            && client_y >= btn.y
                            && client_y <= btn.y + btn.h
                        {
                            action_handled = true;
                            match btn.id {
                                // ─── Interactive Navigation Tabs ───
                                "tab_dev" => {
                                    state.active_tab = ActiveTab::Dev;
                                    state.render_frame();
                                }
                                "tab_focus" => {
                                    state.active_tab = ActiveTab::Focus;
                                    state.render_frame();
                                }
                                "tab_system" => {
                                    state.active_tab = ActiveTab::System;
                                    state.render_frame();
                                }

                                // ─── Command Runner ───
                                "run_check" => {
                                    if state.cmd_runner.run_task("cargo check", "cargo", &["check"], ".") {
                                        state.start_animation_timer();
                                        state.show_copied_toast("⚡ Started cargo check...");
                                    } else {
                                        state.show_copied_toast("⚠ Task already running");
                                    }
                                }
                                "run_test" => {
                                    if state.cmd_runner.run_task("cargo test", "cargo", &["test"], ".") {
                                        state.start_animation_timer();
                                        state.show_copied_toast("⚡ Started cargo test...");
                                    } else {
                                        state.show_copied_toast("⚠ Task already running");
                                    }
                                }

                                // ─── Focus Timer Controls & Presets ───
                                "timer_toggle" => {
                                    state.timer.toggle();
                                    if state.timer.status == FocusTimerStatus::Running {
                                        state.start_focus_timer();
                                    } else {
                                        state.stop_focus_timer();
                                    }
                                    state.render_frame();
                                }
                                "timer_reset" => {
                                    state.timer.reset();
                                    state.stop_focus_timer();
                                    state.render_frame();
                                }
                                "timer_add_5" => {
                                    state.timer.add_seconds(300);
                                    if state.timer.status == FocusTimerStatus::Running {
                                        state.start_focus_timer();
                                    }
                                    state.show_copied_toast("⏱ Added +5m to timer!");
                                }
                                "preset_25" => {
                                    state.timer.set_duration(25 * 60);
                                    state.timer.start();
                                    state.start_focus_timer();
                                    state.show_copied_toast("⏱ Started 25m Pomodoro!");
                                }
                                "preset_50" => {
                                    state.timer.set_duration(50 * 60);
                                    state.timer.start();
                                    state.start_focus_timer();
                                    state.show_copied_toast("⏱ Started 50m Deep Work!");
                                }
                                "preset_5" => {
                                    state.timer.set_duration(5 * 60);
                                    state.timer.start();
                                    state.start_focus_timer();
                                    state.show_copied_toast("☕ Started 5m Break!");
                                }

                                // ─── Active Task Controls ───
                                "task_paste" => {
                                    if let Some(text) = SnippetManager::get_clipboard_text() {
                                        let clean = text.trim();
                                        if !clean.is_empty() {
                                            state.task_note.set(clean);
                                            state.show_copied_toast("✓ Task updated from clipboard!");
                                        } else {
                                            state.show_copied_toast("⚠ Clipboard empty");
                                        }
                                    } else {
                                        state.show_copied_toast("⚠ Clipboard empty");
                                    }
                                }
                                "task_done" => {
                                    state.task_note.set("All tasks completed! 🎉");
                                    state.show_copied_toast("🎉 Task marked done!");
                                }

                                // ─── Quick Launch Links ───
                                "quick_link_0" => {
                                    if let Some(link) = state.config.quick_links.get(0).cloned() {
                                        QuickLinkLauncher::launch(&link);
                                    }
                                }
                                "quick_link_1" => {
                                    if let Some(link) = state.config.quick_links.get(1).cloned() {
                                        QuickLinkLauncher::launch(&link);
                                    }
                                }
                                "quick_link_2" => {
                                    if let Some(link) = state.config.quick_links.get(2).cloned() {
                                        QuickLinkLauncher::launch(&link);
                                    }
                                }

                                // ─── Snippets ───
                                "snippet_0" => {
                                    if let Some(snip) = state.snippets.snippets.get(0).cloned() {
                                        if state.snippets.copy_to_clipboard(&snip.content) {
                                            state.show_copied_toast("✓ Copied snippet 1!");
                                        }
                                    }
                                }
                                "snippet_1" => {
                                    if let Some(snip) = state.snippets.snippets.get(1).cloned() {
                                        if state.snippets.copy_to_clipboard(&snip.content) {
                                            state.show_copied_toast("✓ Copied snippet 2!");
                                        }
                                    }
                                }
                                "snippet_2" => {
                                    if let Some(snip) = state.snippets.snippets.get(2).cloned() {
                                        if state.snippets.copy_to_clipboard(&snip.content) {
                                            state.show_copied_toast("✓ Copied snippet 3!");
                                        }
                                    }
                                }

                                // ─── Micro Utilities ───
                                "util_hex" => {
                                    state.collapse_island();
                                    state.render_frame();
                                    should_pick_color = true;
                                }
                                "util_uuid" => {
                                    let uuid = DeveloperUtilities::generate_uuid_v4();
                                    if state.snippets.copy_to_clipboard(&uuid) {
                                        state.show_copied_toast("✓ Copied UUID v4!");
                                    }
                                }
                                "util_format" => {
                                    if let Some(clip_text) = SnippetManager::get_clipboard_text() {
                                        let trimmed = clip_text.trim();
                                        if trimmed.split('.').count() == 3 && (trimmed.starts_with("eyJ") || trimmed.contains('.')) {
                                            match DeveloperUtilities::inspect_jwt(trimmed) {
                                                Ok(jwt) => {
                                                    let formatted = format!("// Header:\n{}\n\n// Payload:\n{}", jwt.header_json, jwt.payload_json);
                                                    if state.snippets.copy_to_clipboard(&formatted) {
                                                        state.show_copied_toast("✓ Decoded JWT!");
                                                    }
                                                }
                                                Err(_) => {
                                                    match DeveloperUtilities::format_json(trimmed) {
                                                        Ok(formatted) => {
                                                            if state.snippets.copy_to_clipboard(&formatted) {
                                                                state.show_copied_toast("✓ Formatted JSON!");
                                                            }
                                                        }
                                                        Err(_) => {
                                                            state.show_copied_toast("⚠ Invalid JSON/JWT");
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            match DeveloperUtilities::format_json(trimmed) {
                                                Ok(formatted) => {
                                                    if state.snippets.copy_to_clipboard(&formatted) {
                                                        state.show_copied_toast("✓ Formatted JSON!");
                                                    }
                                                }
                                                Err(_) => {
                                                    state.show_copied_toast("⚠ Invalid JSON");
                                                }
                                            }
                                        }
                                    } else {
                                        state.show_copied_toast("⚠ Clipboard empty");
                                    }
                                }

                                // ─── Process & Port Killer ───
                                "kill_port" | "kill_port_0" => {
                                    if let Some(port) = state.dev_ports.cached_active.first().cloned() {
                                        if DevPortsMonitor::kill_process(port.pid) {
                                            state.dev_ports.cached_active.retain(|p| p.pid != port.pid);
                                            state.show_copied_toast("✓ Terminated port process!");
                                        } else {
                                            state.show_copied_toast("⚠ Failed to terminate process");
                                        }
                                    }
                                }
                                "kill_port_1" => {
                                    if let Some(port) = state.dev_ports.cached_active.get(1).cloned() {
                                        if DevPortsMonitor::kill_process(port.pid) {
                                            state.dev_ports.cached_active.retain(|p| p.pid != port.pid);
                                            state.show_copied_toast("✓ Terminated port process!");
                                        } else {
                                            state.show_copied_toast("⚠ Failed to terminate process");
                                        }
                                    }
                                }

                                // ─── Window & App Controls ───
                                "esc_btn" => {
                                    state.collapse_island();
                                }
                                "hide_tray" => {
                                    state.set_visible(false);
                                }
                                "exit_app" => {
                                    println!("Exit button clicked. Exiting...");
                                    unsafe {
                                        let _ = DestroyWindow(hwnd);
                                    }
                                }
                                _ => {}
                            }
                            break;
                        }
                    }

                    if !action_handled {
                        state.collapse_island();
                    }
                } else {
                    state.expand_island();
                }
            }

            if should_pick_color {
                unsafe {
                    let _ = KillTimer(hwnd, TIMER_ANIM_ID);
                }

                if let Some(color) = InteractiveColorPicker::pick_color() {
                    let mut state = state_cell.borrow_mut();
                    if state.snippets.copy_to_clipboard(&color.hex) {
                        state.show_copied_toast("✓ Copied color!");
                    }
                }
            }

            LRESULT(0)
        }

        WM_ACTIVATE => {
            let activation_type = (wparam.0 & 0xFFFF) as u32;
            if activation_type == WA_INACTIVE {
                let mut state = state_cell.borrow_mut();
                if state.anim.target_progress > 0.0 {
                    state.collapse_island();
                }
            }
            LRESULT(0)
        }

        WM_KEYDOWN => {
            let key = wparam.0;
            if key == VK_ESCAPE.0 as usize {
                let mut state = state_cell.borrow_mut();
                if state.anim.target_progress > 0.0 {
                    state.collapse_island();
                }
            }
            LRESULT(0)
        }

        WM_HOTKEY => {
            if wparam.0 as i32 == HOTKEY_TOGGLE_ID {
                let mut state = state_cell.borrow_mut();
                if state.status.visibility == Visibility::Hidden {
                    state.set_visible(true);
                    state.expand_island();
                } else {
                    state.toggle_expand();
                }
            }
            LRESULT(0)
        }

        WM_TRAYICON => {
            let event = lparam.0 as u32;
            match event {
                WM_LBUTTONUP => {
                    let mut state = state_cell.borrow_mut();
                    if state.status.visibility == Visibility::Hidden {
                        state.set_visible(true);
                    } else {
                        state.toggle_expand();
                    }
                }
                WM_RBUTTONUP => {
                    let state = state_cell.borrow();
                    let is_vis = state.status.visibility == Visibility::Visible;
                    let is_exp = state.anim.target_progress >= 0.5;
                    let is_auto = AutostartManager::is_enabled();
                    state.tray.show_context_menu(is_vis, is_exp, is_auto);
                }
                _ => {}
            }
            LRESULT(0)
        }

        WM_COMMAND => {
            let cmd_id = (wparam.0 & 0xFFFF) as usize;
            match cmd_id {
                IDM_TOGGLE_VISIBILITY => {
                    let mut state = state_cell.borrow_mut();
                    state.toggle_visibility();
                }
                IDM_TOGGLE_EXPAND => {
                    let mut state = state_cell.borrow_mut();
                    if state.status.visibility == Visibility::Hidden {
                        state.set_visible(true);
                    }
                    state.toggle_expand();
                }
                IDM_TOGGLE_AUTOSTART => {
                    let new_state = AutostartManager::toggle();
                    println!("Autostart toggled: {}", new_state);
                }
                IDM_EXIT => {
                    unsafe {
                        let _ = DestroyWindow(hwnd);
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }

        WM_DPICHANGED => {
            let new_dpi = (wparam.0 & 0xFFFF) as u32;
            let mut state = state_cell.borrow_mut();
            state.handle_dpi_change(new_dpi);
            LRESULT(0)
        }

        WM_DISPLAYCHANGE | WM_SETTINGCHANGE => {
            let mut state = state_cell.borrow_mut();
            state.handle_display_change();
            LRESULT(0)
        }

        WM_TIMER => {
            let timer_id = wparam.0;
            if timer_id == TIMER_ANIM_ID {
                let mut state = state_cell.borrow_mut();
                let still_animating = state.anim.update();

                // Poll active background command runner
                if let Some(result) = state.cmd_runner.poll_result() {
                    let toast_msg = if result.success {
                        "✓ Command finished successfully!"
                    } else {
                        "✗ Command finished with errors!"
                    };
                    state.copied_toast = Some((toast_msg, Instant::now()));
                }

                state.render_frame();

                // Keep animation timer ticking if command runner is actively running
                if !still_animating && !state.cmd_runner.is_running() {
                    state.stop_animation_timer();
                }
            } else if timer_id == TIMER_FOCUS_ID {
                let mut state = state_cell.borrow_mut();
                if state.timer.tick() {
                    state.render_frame();
                }
                if state.timer.status != FocusTimerStatus::Running {
                    state.stop_focus_timer();
                }
            } else if timer_id == TIMER_TOAST_ID {
                let mut state = state_cell.borrow_mut();
                state.copied_toast = None;
                unsafe {
                    let _ = KillTimer(hwnd, TIMER_TOAST_ID);
                }
                state.render_frame();
            }
            LRESULT(0)
        }

        WM_DESTROY => {
            {
                let mut state = state_cell.borrow_mut();
                state.stop_animation_timer();
                state.stop_focus_timer();
                state.tray.remove();
                state.shortcut.unregister();
            }
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }

        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
