use tiny_skia::{
    Color, FillRule, Paint, Path, PathBuilder, PixmapMut, Stroke, Transform,
};
use fontdue::{Font, FontSettings};
use std::fs;

use crate::animation::Animation;
use crate::config::{QuickLink, Snippet};
use crate::features::{
    ActivePort, CommandRunner, FocusTimer, FocusTimerStatus, GitRepoStatus, SystemMetrics, TaskNote,
};
use crate::render::theme::Theme;
use crate::state::ActiveTab;

pub struct Renderer {
    font: Option<Font>,
    theme: Theme,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonArea {
    pub id: &'static str,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Renderer {
    pub fn new() -> Self {
        let font = Self::load_system_font();
        Self {
            font,
            theme: Theme::default(),
        }
    }

    fn load_system_font() -> Option<Font> {
        let candidates = [
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
        ];

        for path in &candidates {
            if let Ok(bytes) = fs::read(path) {
                if let Ok(font) = Font::from_bytes(bytes, FontSettings::default()) {
                    return Some(font);
                }
            }
        }
        None
    }

    pub fn draw(
        &self,
        pixmap: &mut PixmapMut,
        anim: &Animation,
        scale: f32,
        timer: &FocusTimer,
        task_note: &TaskNote,
        git_status: Option<&GitRepoStatus>,
        quick_links: &[QuickLink],
        snippets: &[Snippet],
        copied_feedback: Option<&str>,
        hovered_button: Option<&str>,
        hovered_pill: bool,
        active_tab: ActiveTab,
        cmd_runner: &CommandRunner,
        sys_metrics: Option<&SystemMetrics>,
        active_ports: &[ActivePort],
    ) -> Vec<ButtonArea> {
        pixmap.data_mut().fill(0);

        let canvas_w = pixmap.width() as f32;
        let (pill_w, pill_h) = anim.current_dimensions(scale);
        let pill_r = anim.current_corner_radius(scale);

        let margin_top = 8.0 * scale;
        let pill_x = (canvas_w - pill_w) / 2.0;
        let pill_y = margin_top;

        // 1. Draw subtle ambient drop shadow
        self.draw_shadow(pixmap, pill_x, pill_y, pill_w, pill_h, pill_r, scale, hovered_pill);

        // 2. Draw Pill background
        if let Some(path) = rounded_rect_path(pill_x, pill_y, pill_w, pill_h, pill_r) {
            let mut paint = Paint::default();
            paint.set_color(self.theme.bg);
            paint.anti_alias = true;
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);

            // Subtle border (brightens on hover when collapsed)
            let mut stroke_paint = Paint::default();
            let border_color = if hovered_pill && anim.current_progress < 0.2 {
                Color::from_rgba8(56, 189, 248, 120) // Cyan hover accent
            } else {
                self.theme.border
            };
            stroke_paint.set_color(border_color);
            stroke_paint.anti_alias = true;
            let stroke = Stroke {
                width: if hovered_pill && anim.current_progress < 0.2 { 1.5 * scale } else { 1.0 * scale },
                ..Default::default()
            };
            pixmap.stroke_path(&path, &stroke_paint, &stroke, Transform::identity(), None);
        }

        let mut buttons = Vec::new();
        let progress = anim.current_progress;

        // 3. Draw Collapsed Content
        if progress < 0.95 {
            let alpha = ((1.0 - progress / 0.7).clamp(0.0, 1.0) * 255.0) as u8;
            if alpha > 0 {
                self.draw_collapsed_content(
                    pixmap,
                    pill_x,
                    pill_y,
                    pill_w,
                    pill_h,
                    scale,
                    timer,
                    task_note,
                    git_status,
                    cmd_runner,
                    hovered_pill,
                    alpha,
                );
            }
        }

        // 4. Draw Expanded Productivity Content
        if progress > 0.05 {
            let alpha = (((progress - 0.25) / 0.75).clamp(0.0, 1.0) * 255.0) as u8;
            if alpha > 0 {
                buttons = self.draw_expanded_content(
                    pixmap,
                    pill_x,
                    pill_y,
                    pill_w,
                    pill_h,
                    scale,
                    timer,
                    task_note,
                    git_status,
                    quick_links,
                    snippets,
                    copied_feedback,
                    alpha,
                    hovered_button,
                    active_tab,
                    cmd_runner,
                    sys_metrics,
                    active_ports,
                );
            }
        }

        buttons
    }

    fn draw_shadow(
        &self,
        pixmap: &mut PixmapMut,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        scale: f32,
        hovered_pill: bool,
    ) {
        // Multi-tier diffusion shadow for realistic floating depth
        let shadow_offsets = [
            (0.5 * scale, 1.5 * scale, 45u8),
            (2.0 * scale, 4.0 * scale, 25u8),
            (4.5 * scale, 9.0 * scale, 12u8),
        ];
        for (inflate, y_off, alpha) in shadow_offsets {
            if let Some(path) = rounded_rect_path(
                x - inflate,
                y + y_off - inflate,
                w + inflate * 2.0,
                h + inflate * 2.0,
                r + inflate,
            ) {
                let mut paint = Paint::default();
                paint.set_color(Color::from_rgba8(0, 0, 0, alpha));
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
            }
        }

        // If hovered in collapsed state, draw ambient colored aura glow
        if hovered_pill {
            if let Some(aura_path) = rounded_rect_path(x - 2.0 * scale, y - 2.0 * scale, w + 4.0 * scale, h + 4.0 * scale, r + 2.0 * scale) {
                let mut aura_paint = Paint::default();
                aura_paint.set_color(Color::from_rgba8(56, 189, 248, 20)); // Soft cyan aura
                aura_paint.anti_alias = true;
                pixmap.fill_path(&aura_path, &aura_paint, FillRule::Winding, Transform::identity(), None);
            }
        }
    }

    fn draw_collapsed_content(
        &self,
        pixmap: &mut PixmapMut,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        scale: f32,
        timer: &FocusTimer,
        task_note: &TaskNote,
        git_status: Option<&GitRepoStatus>,
        cmd_runner: &CommandRunner,
        hovered_pill: bool,
        alpha: u8,
    ) {
        let center_y = y + h / 2.0;

        // Status indicator dot
        let dot_x = x + 16.0 * scale;
        let dot_r = 4.0 * scale;
        let (dot_color, dot_rgb) = if cmd_runner.is_running() {
            (Color::from_rgba8(245, 158, 11, alpha), (245, 158, 11)) // Building Amber
        } else if timer.status == FocusTimerStatus::Running {
            (Color::from_rgba8(59, 130, 246, alpha), (59, 130, 246)) // Active Blue
        } else if let Some(git) = git_status {
            if git.is_clean {
                (Color::from_rgba8(34, 197, 94, alpha), (34, 197, 94)) // Clean Green
            } else {
                (Color::from_rgba8(234, 179, 8, alpha), (234, 179, 8)) // Modified Amber
            }
        } else {
            (Color::from_rgba8(34, 197, 94, alpha), (34, 197, 94)) // Live Green
        };

        // Ambient radial glow behind dot
        let glow_r = dot_r * 2.0;
        if let Some(glow_path) = circle_path(dot_x, center_y, glow_r) {
            let mut glow_paint = Paint::default();
            let glow_a = (alpha as f32 * 0.28) as u8;
            glow_paint.set_color(Color::from_rgba8(dot_rgb.0, dot_rgb.1, dot_rgb.2, glow_a));
            glow_paint.anti_alias = true;
            pixmap.fill_path(&glow_path, &glow_paint, FillRule::Winding, Transform::identity(), None);
        }

        // Core dot
        if let Some(dot_path) = circle_path(dot_x, center_y, dot_r) {
            let mut paint = Paint::default();
            paint.set_color(dot_color);
            paint.anti_alias = true;
            pixmap.fill_path(&dot_path, &paint, FillRule::Winding, Transform::identity(), None);
        }

        if let Some(ref font) = self.font {
            let (r, g, b) = self.theme.text_primary;
            let text_y = center_y + 4.5 * scale;

            if cmd_runner.is_running() {
                let cmd_name = cmd_runner.running_cmd.as_deref().unwrap_or("task");
                let build_str = format!("⚡ Running {}...", cmd_name);
                draw_text(pixmap, font, &build_str, x + 28.0 * scale, text_y, 11.5 * scale, 245, 158, 11, alpha);
            } else if timer.status == FocusTimerStatus::Running {
                let time_str = format!("⏱ {} • Focus", timer.format_time());
                draw_text(pixmap, font, &time_str, x + 28.0 * scale, text_y, 12.0 * scale, r, g, b, alpha);
            } else if let Some(git) = git_status {
                let git_text = format!(" {}", git.branch);
                draw_text(pixmap, font, &git_text, x + 28.0 * scale, text_y, 11.5 * scale, r, g, b, alpha);

                let summary = if git.is_clean {
                    "clean".to_string()
                } else {
                    format!("~{} ?{}", git.modified_count, git.untracked_count)
                };
                let (sr, sg, sb) = self.theme.text_secondary;
                draw_text(pixmap, font, &summary, x + 115.0 * scale, text_y, 10.5 * scale, sr, sg, sb, alpha);
            } else {
                draw_text(pixmap, font, "Dev Island", x + 28.0 * scale, text_y, 12.0 * scale, r, g, b, alpha);

                let summary = task_note.display_summary(12);
                let (sr, sg, sb) = self.theme.text_secondary;
                draw_text(pixmap, font, &summary, x + 105.0 * scale, text_y, 11.0 * scale, sr, sg, sb, alpha);
            }

            // Interactive expand glyph on hover
            if hovered_pill {
                let hint = "▾";
                draw_text(pixmap, font, hint, x + w - 16.0 * scale, text_y, 11.0 * scale, 56, 189, 248, alpha);
            }
        }
    }

    fn draw_expanded_content(
        &self,
        pixmap: &mut PixmapMut,
        x: f32,
        y: f32,
        w: f32,
        _h: f32,
        scale: f32,
        timer: &FocusTimer,
        task_note: &TaskNote,
        git_status: Option<&GitRepoStatus>,
        quick_links: &[QuickLink],
        snippets: &[Snippet],
        copied_feedback: Option<&str>,
        alpha: u8,
        hovered_button: Option<&str>,
        active_tab: ActiveTab,
        cmd_runner: &CommandRunner,
        sys_metrics: Option<&SystemMetrics>,
        active_ports: &[ActivePort],
    ) -> Vec<ButtonArea> {
        let mut buttons = Vec::new();
        let pad = 14.0 * scale;
        let left = x + pad;
        let right = x + w - pad;
        let mut curr_y = y + 13.0 * scale;

        // 1. Header: App title + Interactive View Tabs + [Esc]
        if let Some(ref font) = self.font {
            let (tr, tg, tb) = self.theme.text_primary;
            draw_text(pixmap, font, "Dynamic Island", left, curr_y + 8.0 * scale, 13.0 * scale, tr, tg, tb, alpha);

            // Interactive Tab Bar Chips: [⚡ Dev] [⏱ Focus] [📊 System]
            let tabs = [
                (ActiveTab::Dev, "tab_dev", "⚡ Dev", (56, 189, 248)),
                (ActiveTab::Focus, "tab_focus", "⏱ Focus", (34, 197, 94)),
                (ActiveTab::System, "tab_system", "📊 System", (168, 85, 247)),
            ];

            let tab_w = 54.0 * scale;
            let tab_h = 19.0 * scale;
            let tab_start_x = left + 105.0 * scale;

            for (i, (tab_kind, tab_id, tab_label, tab_color)) in tabs.iter().enumerate() {
                let tx = tab_start_x + i as f32 * (tab_w + 4.0 * scale);
                let is_active = active_tab == *tab_kind;
                let is_hov = hovered_button == Some(*tab_id);

                let tint = if is_active {
                    Some(*tab_color)
                } else if is_hov {
                    Some((255, 255, 255))
                } else {
                    None
                };

                self.draw_tinted_card_bg(pixmap, tx, curr_y - 2.0 * scale, tab_w, tab_h, 4.0 * scale, alpha, is_active || is_hov, tint);

                let (lr, lg, lb) = if is_active {
                    *tab_color
                } else if is_hov {
                    (255, 255, 255)
                } else {
                    self.theme.text_secondary
                };

                draw_text(pixmap, font, tab_label, tx + 6.0 * scale, curr_y + 11.0 * scale, 9.5 * scale, lr, lg, lb, alpha);
                buttons.push(ButtonArea { id: *tab_id, x: tx, y: curr_y - 2.0 * scale, w: tab_w, h: tab_h });
            }

            // [Esc] collapse button
            let esc_id = "esc_btn";
            let is_esc_hov = hovered_button == Some(esc_id);
            let esc_w = 28.0 * scale;
            let esc_x = right - esc_w;
            self.draw_card_bg(pixmap, esc_x, curr_y - 2.0 * scale, esc_w, tab_h, 4.0 * scale, alpha, is_esc_hov);
            let (er, eg, eb) = if is_esc_hov { (255, 255, 255) } else { self.theme.text_secondary };
            draw_text(pixmap, font, "Esc", esc_x + 6.0 * scale, curr_y + 11.0 * scale, 9.5 * scale, er, eg, eb, alpha);
            buttons.push(ButtonArea { id: esc_id, x: esc_x, y: curr_y - 2.0 * scale, w: esc_w, h: tab_h });
        }

        curr_y += 20.0 * scale;

        // Separator line
        let mut sep_paint = Paint::default();
        sep_paint.set_color(Color::from_rgba8(255, 255, 255, (16.0 * (alpha as f32 / 255.0)) as u8));
        let mut pb = PathBuilder::new();
        pb.move_to(left, curr_y);
        pb.line_to(right, curr_y);
        if let Some(path) = pb.finish() {
            let stroke = Stroke { width: 1.0 * scale, ..Default::default() };
            pixmap.stroke_path(&path, &sep_paint, &stroke, Transform::identity(), None);
        }

        curr_y += 8.0 * scale;

        let col_gap = 10.0 * scale;
        let card_h = 86.0 * scale;

        match active_tab {
            ActiveTab::Dev => {
                // ─── TAB 1: DEVELOPER COMMANDS & SHORTCUTS ───
                let left_w = 145.0 * scale;
                let right_w = right - left - left_w - col_gap;

                // Left Card: Command Runner (cargo check & test)
                self.draw_card_bg(pixmap, left, curr_y, left_w, card_h, 8.0 * scale, alpha, false);
                if let Some(ref font) = self.font {
                    let (sr, sg, sb) = self.theme.text_secondary;
                    draw_text(pixmap, font, "BUILD & COMMANDS", left + 10.0 * scale, curr_y + 13.0 * scale, 9.0 * scale, sr, sg, sb, alpha);

                    // Check Command Button
                    let btn_y1 = curr_y + 20.0 * scale;
                    let btn_h = 22.0 * scale;
                    let btn_w = left_w - 20.0 * scale;
                    let check_id = "run_check";
                    let is_check_hov = hovered_button == Some(check_id);

                    let (check_label, check_tint) = if cmd_runner.running_cmd.as_deref() == Some("cargo check") {
                        ("⏳ Running check...", Some((245, 158, 11)))
                    } else if let Some(ref res) = cmd_runner.last_result {
                        if res.command == "cargo check" {
                            if res.success { ("✓ cargo check (pass)", Some((34, 197, 94))) }
                            else { ("✗ cargo check (fail)", Some((239, 68, 68))) }
                        } else {
                            ("▶ cargo check", Some((56, 189, 248)))
                        }
                    } else {
                        ("▶ cargo check", Some((56, 189, 248)))
                    };

                    self.draw_tinted_card_bg(pixmap, left + 10.0 * scale, btn_y1, btn_w, btn_h, 5.0 * scale, alpha, is_check_hov, check_tint);
                    let (cr, cg, cb) = if is_check_hov { (255, 255, 255) } else { check_tint.unwrap_or((241, 245, 249)) };
                    draw_text(pixmap, font, check_label, left + 16.0 * scale, btn_y1 + 15.0 * scale, 9.5 * scale, cr, cg, cb, alpha);
                    buttons.push(ButtonArea { id: check_id, x: left + 10.0 * scale, y: btn_y1, w: btn_w, h: btn_h });

                    // Test Command Button
                    let btn_y2 = btn_y1 + btn_h + 6.0 * scale;
                    let test_id = "run_test";
                    let is_test_hov = hovered_button == Some(test_id);

                    let (test_label, test_tint) = if cmd_runner.running_cmd.as_deref() == Some("cargo test") {
                        ("⏳ Running test...", Some((245, 158, 11)))
                    } else if let Some(ref res) = cmd_runner.last_result {
                        if res.command == "cargo test" {
                            if res.success { ("✓ cargo test (pass)", Some((34, 197, 94))) }
                            else { ("✗ cargo test (fail)", Some((239, 68, 68))) }
                        } else {
                            ("▶ cargo test", Some((168, 85, 247)))
                        }
                    } else {
                        ("▶ cargo test", Some((168, 85, 247)))
                    };

                    self.draw_tinted_card_bg(pixmap, left + 10.0 * scale, btn_y2, btn_w, btn_h, 5.0 * scale, alpha, is_test_hov, test_tint);
                    let (tr, tg, tb) = if is_test_hov { (255, 255, 255) } else { test_tint.unwrap_or((241, 245, 249)) };
                    draw_text(pixmap, font, test_label, left + 16.0 * scale, btn_y2 + 15.0 * scale, 9.5 * scale, tr, tg, tb, alpha);
                    buttons.push(ButtonArea { id: test_id, x: left + 10.0 * scale, y: btn_y2, w: btn_w, h: btn_h });

                    // Result summary footer
                    let summary_str = if let Some(ref res) = cmd_runner.last_result {
                        format!("{}: {}", res.command, res.summary)
                    } else {
                        "Background thread runner".to_string()
                    };
                    draw_text(pixmap, font, &summary_str, left + 11.0 * scale, curr_y + card_h - 6.0 * scale, 8.5 * scale, sr, sg, sb, alpha);
                }

                // Right Card: Quick Links & Snippets
                let right_x = left + left_w + col_gap;
                self.draw_card_bg(pixmap, right_x, curr_y, right_w, card_h, 8.0 * scale, alpha, false);

                // Quick Links Row
                let link_btn_w = (right_w - 24.0 * scale) / 3.0;
                let link_btn_h = 24.0 * scale;
                let link_y = curr_y + 12.0 * scale;
                let link_ids = ["quick_link_0", "quick_link_1", "quick_link_2"];
                for (i, link) in quick_links.iter().take(3).enumerate() {
                    let lx = right_x + 8.0 * scale + i as f32 * (link_btn_w + 4.0 * scale);
                    let id = link_ids[i];
                    let is_hov = hovered_button == Some(id);
                    self.draw_card_bg(pixmap, lx, link_y, link_btn_w, link_btn_h, 5.0 * scale, alpha, is_hov);

                    if let Some(ref font) = self.font {
                        let (tr, tg, tb) = self.theme.text_primary;
                        draw_text(pixmap, font, &link.title, lx + 6.0 * scale, link_y + 16.0 * scale, 10.0 * scale, tr, tg, tb, alpha);
                    }
                    buttons.push(ButtonArea { id, x: lx, y: link_y, w: link_btn_w, h: link_btn_h });
                }

                // Snippets Row
                let snip_y = link_y + link_btn_h + 8.0 * scale;
                let snip_ids = ["snippet_0", "snippet_1", "snippet_2"];
                for (i, snip) in snippets.iter().take(3).enumerate() {
                    let sx = right_x + 8.0 * scale + i as f32 * (link_btn_w + 4.0 * scale);
                    let id = snip_ids[i];
                    let is_hov = hovered_button == Some(id);
                    self.draw_card_bg(pixmap, sx, snip_y, link_btn_w, link_btn_h, 5.0 * scale, alpha, is_hov);

                    if let Some(ref font) = self.font {
                        let (sr, sg, sb) = self.theme.text_secondary;
                        let title = format!("📋 {}", snip.title);
                        draw_text(pixmap, font, &title, sx + 5.0 * scale, snip_y + 16.0 * scale, 9.5 * scale, sr, sg, sb, alpha);
                    }
                    buttons.push(ButtonArea { id, x: sx, y: snip_y, w: link_btn_w, h: link_btn_h });
                }
            }

            ActiveTab::Focus => {
                // ─── TAB 2: INTERACTIVE FOCUS TIMER & ACTIVE TASK ───
                let timer_card_w = 150.0 * scale;
                let tools_card_w = right - left - timer_card_w - col_gap;

                // Left Card: Focus Timer + Progress Bar + +5m Button
                self.draw_card_bg(pixmap, left, curr_y, timer_card_w, card_h, 8.0 * scale, alpha, false);
                if let Some(ref font) = self.font {
                    let (sr, sg, sb) = self.theme.text_secondary;
                    draw_text(pixmap, font, "FOCUS COUNTDOWN", left + 10.0 * scale, curr_y + 13.0 * scale, 9.0 * scale, sr, sg, sb, alpha);

                    let time_str = timer.format_time();
                    let (tr, tg, tb) = self.theme.text_primary;
                    draw_text(pixmap, font, &time_str, left + 10.0 * scale, curr_y + 36.0 * scale, 20.0 * scale, tr, tg, tb, alpha);

                    let status_str = timer.status_label();
                    draw_text(pixmap, font, status_str, left + 90.0 * scale, curr_y + 34.0 * scale, 10.5 * scale, 56, 189, 248, alpha);
                }

                // Interactive Progress Bar
                let bar_x = left + 10.0 * scale;
                let bar_y = curr_y + 44.0 * scale;
                let bar_w = timer_card_w - 20.0 * scale;
                let bar_h = 4.0 * scale;
                let pct = timer.progress_pct();
                draw_progress_bar(pixmap, bar_x, bar_y, bar_w, bar_h, pct, (56, 189, 248), alpha);

                // Focus Timer Action Buttons: [Start/Pause] [Reset] [+5m]
                let btn_y = curr_y + 56.0 * scale;
                let btn_h = 22.0 * scale;
                let btn_w = (timer_card_w - 28.0 * scale) / 3.0;

                // Toggle Button
                let toggle_id = "timer_toggle";
                let is_toggle_hov = hovered_button == Some(toggle_id);
                let toggle_tint = if timer.status == FocusTimerStatus::Running {
                    Some((59, 130, 246))
                } else {
                    Some((34, 197, 94))
                };
                self.draw_tinted_card_bg(pixmap, left + 8.0 * scale, btn_y, btn_w, btn_h, 4.0 * scale, alpha, is_toggle_hov, toggle_tint);
                if let Some(ref font) = self.font {
                    let label = if timer.status == FocusTimerStatus::Running { "Pause" } else { "Start" };
                    let (tr, tg, tb) = self.theme.text_primary;
                    draw_text(pixmap, font, label, left + 11.0 * scale, btn_y + 15.0 * scale, 9.5 * scale, tr, tg, tb, alpha);
                }
                buttons.push(ButtonArea { id: toggle_id, x: left + 8.0 * scale, y: btn_y, w: btn_w, h: btn_h });

                // Reset Button
                let reset_id = "timer_reset";
                let is_reset_hov = hovered_button == Some(reset_id);
                let reset_x = left + 8.0 * scale + btn_w + 4.0 * scale;
                self.draw_card_bg(pixmap, reset_x, btn_y, btn_w, btn_h, 4.0 * scale, alpha, is_reset_hov);
                if let Some(ref font) = self.font {
                    let (sr, sg, sb) = self.theme.text_secondary;
                    draw_text(pixmap, font, "Reset", reset_x + 6.0 * scale, btn_y + 15.0 * scale, 9.5 * scale, sr, sg, sb, alpha);
                }
                buttons.push(ButtonArea { id: reset_id, x: reset_x, y: btn_y, w: btn_w, h: btn_h });

                // +5m Quick Add Button
                let add_id = "timer_add_5";
                let is_add_hov = hovered_button == Some(add_id);
                let add_x = reset_x + btn_w + 4.0 * scale;
                self.draw_tinted_card_bg(pixmap, add_x, btn_y, btn_w, btn_h, 4.0 * scale, alpha, is_add_hov, Some((245, 158, 11)));
                if let Some(ref font) = self.font {
                    let (ar, ag, ab) = if is_add_hov { (255, 255, 255) } else { (245, 158, 11) };
                    draw_text(pixmap, font, "+5m", add_x + 9.0 * scale, btn_y + 15.0 * scale, 9.5 * scale, ar, ag, ab, alpha);
                }
                buttons.push(ButtonArea { id: add_id, x: add_x, y: btn_y, w: btn_w, h: btn_h });

                // Right Card: Presets & Task Note
                let right_x = left + timer_card_w + col_gap;
                self.draw_card_bg(pixmap, right_x, curr_y, tools_card_w, card_h, 8.0 * scale, alpha, false);

                // Presets Row
                let pre_w = (tools_card_w - 24.0 * scale) / 3.0;
                let pre_h = 24.0 * scale;
                let pre_y = curr_y + 12.0 * scale;
                let presets = [
                    ("preset_25", "25m Pomo", (56, 189, 248)),
                    ("preset_50", "50m Deep", (168, 85, 247)),
                    ("preset_5", "5m Break", (34, 197, 94)),
                ];
                for (i, (pid, plabel, ptint)) in presets.iter().enumerate() {
                    let px = right_x + 8.0 * scale + i as f32 * (pre_w + 4.0 * scale);
                    let is_hov = hovered_button == Some(*pid);
                    self.draw_tinted_card_bg(pixmap, px, pre_y, pre_w, pre_h, 5.0 * scale, alpha, is_hov, Some(*ptint));

                    if let Some(ref font) = self.font {
                        let (pr, pg, pb) = if is_hov { (255, 255, 255) } else { *ptint };
                        draw_text(pixmap, font, plabel, px + 7.0 * scale, pre_y + 16.0 * scale, 9.5 * scale, pr, pg, pb, alpha);
                    }
                    buttons.push(ButtonArea { id: *pid, x: px, y: pre_y, w: pre_w, h: pre_h });
                }

                // Active Task Row with interactive [Paste] & [Done]
                let task_y = pre_y + pre_h + 8.0 * scale;
                let task_display = format!("📌 {}", task_note.display_summary(18));
                if let Some(ref font) = self.font {
                    let (tr, tg, tb) = self.theme.text_primary;
                    draw_text(pixmap, font, &task_display, right_x + 10.0 * scale, task_y + 16.0 * scale, 10.0 * scale, tr, tg, tb, alpha);
                }

                let paste_btn_w = 66.0 * scale;
                let paste_x = right - paste_btn_w - 42.0 * scale;
                let paste_id = "task_paste";
                let is_paste_hov = hovered_button == Some(paste_id);
                self.draw_card_bg(pixmap, paste_x, task_y, paste_btn_w, 22.0 * scale, 4.0 * scale, alpha, is_paste_hov);
                if let Some(ref font) = self.font {
                    let (sr, sg, sb) = if is_paste_hov { (255, 255, 255) } else { self.theme.text_secondary };
                    draw_text(pixmap, font, "📋 Paste", paste_x + 6.0 * scale, task_y + 15.0 * scale, 9.0 * scale, sr, sg, sb, alpha);
                }
                buttons.push(ButtonArea { id: paste_id, x: paste_x, y: task_y, w: paste_btn_w, h: 22.0 * scale });

                let done_btn_w = 36.0 * scale;
                let done_x = right - done_btn_w;
                let done_id = "task_done";
                let is_done_hov = hovered_button == Some(done_id);
                self.draw_tinted_card_bg(pixmap, done_x, task_y, done_btn_w, 22.0 * scale, 4.0 * scale, alpha, is_done_hov, Some((34, 197, 94)));
                if let Some(ref font) = self.font {
                    let (dr, dg, db) = if is_done_hov { (255, 255, 255) } else { (34, 197, 94) };
                    draw_text(pixmap, font, "Done", done_x + 6.0 * scale, task_y + 15.0 * scale, 9.0 * scale, dr, dg, db, alpha);
                }
                buttons.push(ButtonArea { id: done_id, x: done_x, y: task_y, w: done_btn_w, h: 22.0 * scale });
            }

            ActiveTab::System => {
                // ─── TAB 3: HARDWARE METRICS & DEV PORTS ───
                let left_w = 160.0 * scale;
                let right_w = right - left - left_w - col_gap;

                // Left Card: Live Hardware Gauges
                self.draw_card_bg(pixmap, left, curr_y, left_w, card_h, 8.0 * scale, alpha, false);
                if let Some(metrics) = sys_metrics {
                    if let Some(ref font) = self.font {
                        let (sr, sg, sb) = self.theme.text_secondary;
                        draw_text(pixmap, font, "HARDWARE METRICS", left + 10.0 * scale, curr_y + 13.0 * scale, 9.0 * scale, sr, sg, sb, alpha);

                        // CPU Bar
                        let cpu_str = format!("CPU {:.0}%", metrics.cpu_usage_pct);
                        let cpu_color = if metrics.cpu_usage_pct > 80.0 { (239, 68, 68) } else if metrics.cpu_usage_pct > 50.0 { (245, 158, 11) } else { (34, 197, 94) };
                        draw_text(pixmap, font, &cpu_str, left + 10.0 * scale, curr_y + 30.0 * scale, 10.0 * scale, cpu_color.0, cpu_color.1, cpu_color.2, alpha);
                        draw_progress_bar(pixmap, left + 10.0 * scale, curr_y + 35.0 * scale, left_w - 20.0 * scale, 5.0 * scale, metrics.cpu_usage_pct / 100.0, cpu_color, alpha);

                        // RAM Bar
                        let ram_str = format!("RAM {}% ({:.1}/{:.1} GB)", metrics.ram_usage_pct, metrics.ram_used_gb, metrics.ram_total_gb);
                        let ram_color = if metrics.ram_usage_pct > 85 { (239, 68, 68) } else { (56, 189, 248) };
                        draw_text(pixmap, font, &ram_str, left + 10.0 * scale, curr_y + 57.0 * scale, 10.0 * scale, ram_color.0, ram_color.1, ram_color.2, alpha);
                        draw_progress_bar(pixmap, left + 10.0 * scale, curr_y + 63.0 * scale, left_w - 20.0 * scale, 5.0 * scale, metrics.ram_usage_pct as f32 / 100.0, ram_color, alpha);

                        draw_text(pixmap, font, "Live Win32 NT query (0% idle)", left + 10.0 * scale, curr_y + 79.0 * scale, 8.5 * scale, sr, sg, sb, alpha);
                    }
                } else if let Some(ref font) = self.font {
                    let (sr, sg, sb) = self.theme.text_secondary;
                    draw_text(pixmap, font, "Polling system...", left + 10.0 * scale, curr_y + 40.0 * scale, 10.0 * scale, sr, sg, sb, alpha);
                }

                // Right Card: Active Dev Ports & Process Killer
                let right_x = left + left_w + col_gap;
                self.draw_card_bg(pixmap, right_x, curr_y, right_w, card_h, 8.0 * scale, alpha, false);
                if let Some(ref font) = self.font {
                    let (sr, sg, sb) = self.theme.text_secondary;
                    draw_text(pixmap, font, "ACTIVE DEV PORTS", right_x + 10.0 * scale, curr_y + 13.0 * scale, 9.0 * scale, sr, sg, sb, alpha);

                    if active_ports.is_empty() {
                        draw_text(pixmap, font, "No active dev servers listening", right_x + 10.0 * scale, curr_y + 35.0 * scale, 10.0 * scale, sr, sg, sb, alpha);
                        draw_text(pixmap, font, "Monitors: 3000, 3001, 5173, 8000, 8080", right_x + 10.0 * scale, curr_y + 50.0 * scale, 9.0 * scale, sr, sg, sb, alpha);
                    } else {
                        for (idx, port) in active_ports.iter().take(2).enumerate() {
                            let py = curr_y + 22.0 * scale + idx as f32 * 26.0 * scale;
                            let p_str = format!("⚡ :{} ({})", port.port, port.process_name);
                            draw_text(pixmap, font, &p_str, right_x + 10.0 * scale, py + 15.0 * scale, 10.0 * scale, 251, 191, 36, alpha);

                            // Kill button
                            let kill_btn_w = 42.0 * scale;
                            let kill_btn_x = right - kill_btn_w - 6.0 * scale;
                            let kill_id = if idx == 0 { "kill_port_0" } else { "kill_port_1" };
                            let is_kill_hov = hovered_button == Some(kill_id);
                            self.draw_tinted_card_bg(pixmap, kill_btn_x, py, kill_btn_w, 20.0 * scale, 4.0 * scale, alpha, is_kill_hov, Some((239, 68, 68)));
                            let (kr, kg, kb) = if is_kill_hov { (255, 255, 255) } else { (239, 68, 68) };
                            draw_text(pixmap, font, "Kill", kill_btn_x + 10.0 * scale, py + 14.0 * scale, 9.0 * scale, kr, kg, kb, alpha);
                            buttons.push(ButtonArea { id: kill_id, x: kill_btn_x, y: py, w: kill_btn_w, h: 20.0 * scale });
                        }
                    }

                    // Git status row
                    if let Some(git) = git_status {
                        let git_str = format!(" {} • ~{} ?{} +{}", git.branch, git.modified_count, git.untracked_count, git.staged_count);
                        draw_text(pixmap, font, &git_str, right_x + 10.0 * scale, curr_y + card_h - 6.0 * scale, 9.0 * scale, 34, 197, 94, alpha);
                    }
                }
            }
        }

        curr_y += card_h + 8.0 * scale;

        // 3. Bottom Row: Developer Micro-Tools + Contextual Tooltips + [Hide] & [Exit]
        let btn_h = 20.0 * scale;

        // Exit button
        let exit_w = 36.0 * scale;
        let exit_x = right - exit_w;
        let exit_id = "exit_app";
        let is_exit_hov = hovered_button == Some(exit_id);
        self.draw_tinted_card_bg(pixmap, exit_x, curr_y, exit_w, btn_h, 4.0 * scale, alpha, is_exit_hov, Some((239, 68, 68)));
        if let Some(ref font) = self.font {
            let (er, eg, eb) = if is_exit_hov { (255, 255, 255) } else { (239, 68, 68) };
            draw_text(pixmap, font, "Exit", exit_x + 8.0 * scale, curr_y + 14.0 * scale, 9.5 * scale, er, eg, eb, alpha);
        }
        buttons.push(ButtonArea { id: exit_id, x: exit_x, y: curr_y, w: exit_w, h: btn_h });

        // Hide button
        let hide_w = 38.0 * scale;
        let hide_x = exit_x - hide_w - 5.0 * scale;
        let hide_id = "hide_tray";
        let is_hide_hov = hovered_button == Some(hide_id);
        self.draw_tinted_card_bg(pixmap, hide_x, curr_y, hide_w, btn_h, 4.0 * scale, alpha, is_hide_hov, Some((148, 163, 184)));
        if let Some(ref font) = self.font {
            let (sr, sg, sb) = if is_hide_hov { (255, 255, 255) } else { self.theme.text_secondary };
            draw_text(pixmap, font, "Hide", hide_x + 8.0 * scale, curr_y + 14.0 * scale, 9.5 * scale, sr, sg, sb, alpha);
        }
        buttons.push(ButtonArea { id: hide_id, x: hide_x, y: curr_y, w: hide_w, h: btn_h });

        // Format JSON/JWT button
        let fmt_w = 64.0 * scale;
        let fmt_x = hide_x - fmt_w - 6.0 * scale;
        let fmt_id = "util_format";
        let is_fmt_hov = hovered_button == Some(fmt_id);
        self.draw_tinted_card_bg(pixmap, fmt_x, curr_y, fmt_w, btn_h, 4.0 * scale, alpha, is_fmt_hov, Some((250, 204, 21)));
        if let Some(ref font) = self.font {
            let (tr, tg, tb) = if is_fmt_hov { (255, 255, 255) } else { (250, 204, 21) };
            draw_text(pixmap, font, "⚡ Format", fmt_x + 5.0 * scale, curr_y + 14.0 * scale, 9.5 * scale, tr, tg, tb, alpha);
        }
        buttons.push(ButtonArea { id: fmt_id, x: fmt_x, y: curr_y, w: fmt_w, h: btn_h });

        // UUID generator button
        let uuid_w = 46.0 * scale;
        let uuid_x = fmt_x - uuid_w - 5.0 * scale;
        let uuid_id = "util_uuid";
        let is_uuid_hov = hovered_button == Some(uuid_id);
        self.draw_tinted_card_bg(pixmap, uuid_x, curr_y, uuid_w, btn_h, 4.0 * scale, alpha, is_uuid_hov, Some((96, 165, 250)));
        if let Some(ref font) = self.font {
            let (tr, tg, tb) = if is_uuid_hov { (255, 255, 255) } else { (96, 165, 250) };
            draw_text(pixmap, font, "🆔 UUID", uuid_x + 5.0 * scale, curr_y + 14.0 * scale, 9.5 * scale, tr, tg, tb, alpha);
        }
        buttons.push(ButtonArea { id: uuid_id, x: uuid_x, y: curr_y, w: uuid_w, h: btn_h });

        // Screen Color Picker button
        let hex_w = 44.0 * scale;
        let hex_x = uuid_x - hex_w - 5.0 * scale;
        let hex_id = "util_hex";
        let is_hex_hov = hovered_button == Some(hex_id);
        self.draw_tinted_card_bg(pixmap, hex_x, curr_y, hex_w, btn_h, 4.0 * scale, alpha, is_hex_hov, Some((192, 132, 252)));
        if let Some(ref font) = self.font {
            let (tr, tg, tb) = if is_hex_hov { (255, 255, 255) } else { (192, 132, 252) };
            draw_text(pixmap, font, "🎨 Hex", hex_x + 5.0 * scale, curr_y + 14.0 * scale, 9.5 * scale, tr, tg, tb, alpha);
        }
        buttons.push(ButtonArea { id: hex_id, x: hex_x, y: curr_y, w: hex_w, h: btn_h });

        // Left Side: Feedback Toast OR Interactive Contextual Hover Tooltip Guide!
        if let Some(ref font) = self.font {
            if let Some(toast) = copied_feedback {
                let toast_w = 114.0 * scale;
                let toast_h = 20.0 * scale;
                self.draw_tinted_card_bg(pixmap, left, curr_y, toast_w, toast_h, 4.0 * scale, alpha, true, Some((34, 197, 94)));
                let (gr, gg, gb) = (74, 222, 128); // Vibrant emerald
                draw_text(pixmap, font, toast, left + 8.0 * scale, curr_y + 14.0 * scale, 9.5 * scale, gr, gg, gb, alpha);
            } else if let Some(tip) = get_tooltip_text(hovered_button) {
                // Interactive guidance tooltip for hovered action
                let (sr, sg, sb) = (56, 189, 248); // Subtle cyan guide text
                draw_text(pixmap, font, tip, left, curr_y + 14.0 * scale, 9.5 * scale, sr, sg, sb, alpha);
            } else {
                // Default shortcut hint
                let (sr, sg, sb) = self.theme.text_secondary;
                draw_text(pixmap, font, "Ctrl+Shift+D • Tab to switch", left, curr_y + 14.0 * scale, 9.0 * scale, sr, sg, sb, alpha);
            }
        }

        buttons
    }

    fn draw_card_bg(
        &self,
        pixmap: &mut PixmapMut,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        alpha: u8,
        hovered: bool,
    ) {
        self.draw_tinted_card_bg(pixmap, x, y, w, h, r, alpha, hovered, None);
    }

    fn draw_tinted_card_bg(
        &self,
        pixmap: &mut PixmapMut,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        alpha: u8,
        hovered: bool,
        tint: Option<(u8, u8, u8)>,
    ) {
        let (cr, cg, cb) = tint.unwrap_or((255, 255, 255));
        let bg_alpha = if hovered {
            if tint.is_some() { 45 } else { 32 }
        } else {
            if tint.is_some() { 20 } else { 14 }
        };
        let color = Color::from_rgba8(cr, cg, cb, (bg_alpha as f32 * (alpha as f32 / 255.0)) as u8);

        if let Some(card_path) = rounded_rect_path(x, y, w, h, r) {
            let mut p = Paint::default();
            p.set_color(color);
            p.anti_alias = true;
            pixmap.fill_path(&card_path, &p, FillRule::Winding, Transform::identity(), None);

            let stroke_alpha = if hovered {
                if tint.is_some() { 85 } else { 45 }
            } else {
                if tint.is_some() { 36 } else { 20 }
            };
            let stroke_color = Color::from_rgba8(cr, cg, cb, (stroke_alpha as f32 * (alpha as f32 / 255.0)) as u8);
            let mut sp = Paint::default();
            sp.set_color(stroke_color);
            let stroke = Stroke { width: 1.0, ..Default::default() };
            pixmap.stroke_path(&card_path, &sp, &stroke, Transform::identity(), None);
        }
    }
}

pub fn draw_progress_bar(
    pixmap: &mut PixmapMut,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    pct: f32,
    color: (u8, u8, u8),
    alpha: u8,
) {
    let r = (h / 2.0).max(1.0);
    // Background track
    if let Some(track) = rounded_rect_path(x, y, w, h, r) {
        let mut p = Paint::default();
        p.set_color(Color::from_rgba8(255, 255, 255, (20.0 * (alpha as f32 / 255.0)) as u8));
        p.anti_alias = true;
        pixmap.fill_path(&track, &p, FillRule::Winding, Transform::identity(), None);
    }
    // Filled progress
    let fill_w = (w * pct.clamp(0.0, 1.0)).max(r * 2.0);
    if let Some(bar) = rounded_rect_path(x, y, fill_w, h, r) {
        let mut p = Paint::default();
        p.set_color(Color::from_rgba8(color.0, color.1, color.2, (210.0 * (alpha as f32 / 255.0)) as u8));
        p.anti_alias = true;
        pixmap.fill_path(&bar, &p, FillRule::Winding, Transform::identity(), None);
    }
}

pub fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

pub fn circle_path(cx: f32, cy: f32, r: f32) -> Option<Path> {
    rounded_rect_path(cx - r, cy - r, r * 2.0, r * 2.0, r)
}

fn get_tooltip_text(hovered: Option<&str>) -> Option<&'static str> {
    match hovered {
        Some("tab_dev") => Some("Switch to Dev Commands & Tools"),
        Some("tab_focus") => Some("Switch to Focus Countdown & Task Note"),
        Some("tab_system") => Some("Switch to Live Hardware & Port Manager"),
        Some("run_check") => Some("Execute 'cargo check' in background"),
        Some("run_test") => Some("Execute 'cargo test' in background"),
        Some("quick_link_0") => Some("Launch Terminal (wt.exe)"),
        Some("quick_link_1") => Some("Open current folder in Explorer"),
        Some("quick_link_2") => Some("Open Rust standard library documentation"),
        Some("snippet_0") => Some("Copy 'cargo test' to clipboard"),
        Some("snippet_1") => Some("Copy 'git status -s' to clipboard"),
        Some("snippet_2") => Some("Copy 'cargo build --release' to clipboard"),
        Some("timer_toggle") => Some("Start or pause focus timer countdown"),
        Some("timer_reset") => Some("Reset focus timer back to session duration"),
        Some("timer_add_5") => Some("Add +5 minutes to active countdown"),
        Some("preset_25") => Some("Set 25-minute Pomodoro focus block"),
        Some("preset_50") => Some("Set 50-minute Deep Work session"),
        Some("preset_5") => Some("Set 5-minute restorative short break"),
        Some("task_paste") => Some("Set active task directly from clipboard text"),
        Some("task_done") => Some("Mark current task as finished"),
        Some("kill_port_0") | Some("kill_port_1") | Some("kill_port") => Some("Terminate process locking this dev port"),
        Some("util_hex") => Some("Sample pixel color under cursor & copy #RRGGBB"),
        Some("util_uuid") => Some("Generate random UUID v4 to clipboard"),
        Some("util_format") => Some("Format JSON or decode JWT from clipboard"),
        Some("hide_tray") => Some("Minimize Dynamic Island to Windows system tray"),
        Some("exit_app") => Some("Quit Dynamic Island utility"),
        Some("esc_btn") => Some("Collapse Dynamic Island to floating pill"),
        _ => None,
    }
}

fn draw_text(
    pixmap: &mut PixmapMut,
    font: &Font,
    text: &str,
    start_x: f32,
    baseline_y: f32,
    size: f32,
    r: u8,
    g: u8,
    b: u8,
    opacity: u8,
) {
    if opacity == 0 {
        return;
    }

    let mut pen_x = start_x;
    let pm_width = pixmap.width() as i32;
    let pm_height = pixmap.height() as i32;

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);

        let glyph_x = pen_x.round() as i32 + metrics.xmin;
        let glyph_y = baseline_y.round() as i32 - metrics.height as i32 - metrics.ymin;

        for (idx, &cov) in bitmap.iter().enumerate() {
            if cov == 0 {
                continue;
            }

            let px = glyph_x + (idx % metrics.width) as i32;
            let py = glyph_y + (idx / metrics.width) as i32;

            if px >= 0 && px < pm_width && py >= 0 && py < pm_height {
                let pixel_idx = ((py * pm_width + px) * 4) as usize;
                let data = pixmap.data_mut();

                let effective_alpha = ((cov as u32 * opacity as u32) / 255) as u8;
                let src_a = effective_alpha as f32 / 255.0;
                let src_r = (r as f32 * src_a) as u8;
                let src_g = (g as f32 * src_a) as u8;
                let src_b = (b as f32 * src_a) as u8;

                let dst_r = data[pixel_idx];
                let dst_g = data[pixel_idx + 1];
                let dst_b = data[pixel_idx + 2];
                let dst_a = data[pixel_idx + 3];

                let out_a = effective_alpha as u32 + (dst_a as u32 * (255 - effective_alpha as u32)) / 255;
                let out_r = src_r as u32 + (dst_r as u32 * (255 - effective_alpha as u32)) / 255;
                let out_g = src_g as u32 + (dst_g as u32 * (255 - effective_alpha as u32)) / 255;
                let out_b = src_b as u32 + (dst_b as u32 * (255 - effective_alpha as u32)) / 255;

                data[pixel_idx] = out_r.min(255) as u8;
                data[pixel_idx + 1] = out_g.min(255) as u8;
                data[pixel_idx + 2] = out_b.min(255) as u8;
                data[pixel_idx + 3] = out_a.min(255) as u8;
            }
        }

        pen_x += metrics.advance_width;
    }
}
