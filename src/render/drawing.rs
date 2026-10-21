use tiny_skia::{
    Color, FillRule, Paint, Path, PathBuilder, PixmapMut, Stroke, Transform,
};
use fontdue::{Font, FontSettings};
use std::fs;

use crate::animation::Animation;
use crate::config::{QuickLink, Snippet};
use crate::features::{
    ActivePort, FocusTimer, FocusTimerStatus, GitRepoStatus, SystemMetrics, TaskNote,
};
use crate::render::theme::Theme;

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
        self.draw_shadow(pixmap, pill_x, pill_y, pill_w, pill_h, pill_r, scale);

        // 2. Draw Pill background
        if let Some(path) = rounded_rect_path(pill_x, pill_y, pill_w, pill_h, pill_r) {
            let mut paint = Paint::default();
            paint.set_color(self.theme.bg);
            paint.anti_alias = true;
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);

            // Subtle border
            let mut stroke_paint = Paint::default();
            stroke_paint.set_color(self.theme.border);
            stroke_paint.anti_alias = true;
            let stroke = Stroke {
                width: 1.0 * scale,
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
    }

    fn draw_collapsed_content(
        &self,
        pixmap: &mut PixmapMut,
        x: f32,
        y: f32,
        _w: f32,
        h: f32,
        scale: f32,
        timer: &FocusTimer,
        task_note: &TaskNote,
        git_status: Option<&GitRepoStatus>,
        alpha: u8,
    ) {
        let center_y = y + h / 2.0;

        // Status indicator dot
        let dot_x = x + 16.0 * scale;
        let dot_r = 4.0 * scale;
        let (dot_color, dot_rgb) = if timer.status == FocusTimerStatus::Running {
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

            if timer.status == FocusTimerStatus::Running {
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

                // Small task summary preview
                let summary = task_note.display_summary(12);
                let (sr, sg, sb) = self.theme.text_secondary;
                draw_text(pixmap, font, &summary, x + 105.0 * scale, text_y, 11.0 * scale, sr, sg, sb, alpha);
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
        sys_metrics: Option<&SystemMetrics>,
        active_ports: &[ActivePort],
    ) -> Vec<ButtonArea> {
        let mut buttons = Vec::new();
        let pad = 14.0 * scale;
        let left = x + pad;
        let right = x + w - pad;
        let mut curr_y = y + 14.0 * scale;

        // 1. Header: App title + Git Status / Active Task + System Resource Metrics + [Esc]
        if let Some(ref font) = self.font {
            let (tr, tg, tb) = self.theme.text_primary;
            draw_text(pixmap, font, "Dynamic Island", left, curr_y + 7.0 * scale, 13.0 * scale, tr, tg, tb, alpha);

            if let Some(git) = git_status {
                let badge = git.summary_badge();
                let (gr, gg, gb) = if git.is_clean { (34, 197, 94) } else { (234, 179, 8) };
                draw_text(pixmap, font, &badge, left + 108.0 * scale, curr_y + 7.0 * scale, 10.5 * scale, gr, gg, gb, alpha);
            } else {
                let task_display = format!("Task: {}", task_note.display_summary(16));
                let (sr, sg, sb) = self.theme.text_secondary;
                draw_text(pixmap, font, &task_display, left + 115.0 * scale, curr_y + 7.0 * scale, 11.0 * scale, sr, sg, sb, alpha);
            }

            // CPU & RAM resource monitor (Feature 5)
            if let Some(metrics) = sys_metrics {
                let sys_str = format!("CPU {:.0}%  RAM {}%", metrics.cpu_usage_pct, metrics.ram_usage_pct);
                let (mr, mg, mb) = self.theme.text_secondary;
                draw_text(pixmap, font, &sys_str, right - 142.0 * scale, curr_y + 7.0 * scale, 9.5 * scale, mr, mg, mb, alpha);
            }

            let collapse_hint = "[Esc]";
            let (hr, hg, hb) = self.theme.text_secondary;
            draw_text(pixmap, font, collapse_hint, right - 32.0 * scale, curr_y + 7.0 * scale, 11.0 * scale, hr, hg, hb, alpha);
        }

        curr_y += 18.0 * scale;

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

        curr_y += 10.0 * scale;

        // 2. Main Body: Left card (Focus Timer) + Right card (Quick Links & Snippets)
        let col_gap = 10.0 * scale;
        let timer_card_w = 120.0 * scale;
        let tools_card_w = right - left - timer_card_w - col_gap;
        let card_h = 82.0 * scale;

        // Draw Left Card: Focus Timer
        self.draw_card_bg(pixmap, left, curr_y, timer_card_w, card_h, 8.0 * scale, alpha, false);
        if let Some(ref font) = self.font {
            let (sr, sg, sb) = self.theme.text_secondary;
            draw_text(pixmap, font, "FOCUS TIMER", left + 10.0 * scale, curr_y + 14.0 * scale, 9.0 * scale, sr, sg, sb, alpha);

            let time_str = timer.format_time();
            let (tr, tg, tb) = self.theme.text_primary;
            draw_text(pixmap, font, &time_str, left + 10.0 * scale, curr_y + 36.0 * scale, 19.0 * scale, tr, tg, tb, alpha);

            let status_str = timer.status_label();
            draw_text(pixmap, font, status_str, left + 10.0 * scale, curr_y + 49.0 * scale, 10.0 * scale, sr, sg, sb, alpha);
        }

        // Focus Timer Buttons: [Start / Pause] and [Reset]
        let btn_y = curr_y + 55.0 * scale;
        let btn_h = 20.0 * scale;
        let btn_w = (timer_card_w - 24.0 * scale) / 2.0;

        let toggle_id = "timer_toggle";
        let is_toggle_hov = hovered_button == Some(toggle_id);
        let toggle_tint = if timer.status == FocusTimerStatus::Running {
            Some((59, 130, 246)) // Active Blue
        } else {
            Some((34, 197, 94))  // Ready Green
        };
        self.draw_tinted_card_bg(pixmap, left + 8.0 * scale, btn_y, btn_w, btn_h, 4.0 * scale, alpha, is_toggle_hov, toggle_tint);
        if let Some(ref font) = self.font {
            let label = if timer.status == FocusTimerStatus::Running { "Pause" } else { "Start" };
            let (tr, tg, tb) = self.theme.text_primary;
            draw_text(pixmap, font, label, left + 14.0 * scale, btn_y + 14.0 * scale, 10.0 * scale, tr, tg, tb, alpha);
        }
        buttons.push(ButtonArea { id: toggle_id, x: left + 8.0 * scale, y: btn_y, w: btn_w, h: btn_h });

        let reset_id = "timer_reset";
        let is_reset_hov = hovered_button == Some(reset_id);
        let reset_x = left + 8.0 * scale + btn_w + 6.0 * scale;
        self.draw_card_bg(pixmap, reset_x, btn_y, btn_w, btn_h, 4.0 * scale, alpha, is_reset_hov);
        if let Some(ref font) = self.font {
            let (sr, sg, sb) = self.theme.text_secondary;
            draw_text(pixmap, font, "Reset", reset_x + 9.0 * scale, btn_y + 14.0 * scale, 10.0 * scale, sr, sg, sb, alpha);
        }
        buttons.push(ButtonArea { id: reset_id, x: reset_x, y: btn_y, w: btn_w, h: btn_h });

        // Draw Right Card: Quick Links & Snippets
        let right_x = left + timer_card_w + col_gap;
        self.draw_card_bg(pixmap, right_x, curr_y, tools_card_w, card_h, 8.0 * scale, alpha, false);

        // Quick Links Chips
        let link_btn_w = (tools_card_w - 24.0 * scale) / 3.0;
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

        // Snippets Chips
        let snip_y = link_y + link_btn_h + 8.0 * scale;
        let snip_btn_w = (tools_card_w - 24.0 * scale) / 3.0;
        let snip_btn_h = 24.0 * scale;

        let snip_ids = ["snippet_0", "snippet_1", "snippet_2"];
        for (i, snip) in snippets.iter().take(3).enumerate() {
            let sx = right_x + 8.0 * scale + i as f32 * (snip_btn_w + 4.0 * scale);
            let id = snip_ids[i];
            let is_hov = hovered_button == Some(id);
            self.draw_card_bg(pixmap, sx, snip_y, snip_btn_w, snip_btn_h, 5.0 * scale, alpha, is_hov);

            if let Some(ref font) = self.font {
                let (sr, sg, sb) = self.theme.text_secondary;
                let title = format!("📋 {}", snip.title);
                draw_text(pixmap, font, &title, sx + 5.0 * scale, snip_y + 16.0 * scale, 9.5 * scale, sr, sg, sb, alpha);
            }
            buttons.push(ButtonArea { id, x: sx, y: snip_y, w: snip_btn_w, h: snip_btn_h });
        }

        curr_y += card_h + 8.0 * scale;

        // 3. Bottom Row: Developer Micro-Tools + Feedback Toast + [Hide] & [Exit]
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

        // Feedback toast or active dev port on the left
        if let Some(ref font) = self.font {
            if let Some(toast) = copied_feedback {
                let toast_w = 114.0 * scale;
                let toast_h = 20.0 * scale;
                self.draw_tinted_card_bg(pixmap, left, curr_y, toast_w, toast_h, 4.0 * scale, alpha, true, Some((34, 197, 94)));
                let (gr, gg, gb) = (74, 222, 128); // Vibrant emerald
                draw_text(pixmap, font, toast, left + 8.0 * scale, curr_y + 14.0 * scale, 9.5 * scale, gr, gg, gb, alpha);
            } else if let Some(port) = active_ports.first() {
                let text = format!("⚡:{} ({})", port.port, port.process_name);
                let port_w = 88.0 * scale;
                let port_h = 20.0 * scale;
                self.draw_tinted_card_bg(pixmap, left, curr_y, port_w, port_h, 4.0 * scale, alpha, false, Some((245, 158, 11)));
                let (pr, pg, pb) = (251, 191, 36); // Amber
                draw_text(pixmap, font, &text, left + 6.0 * scale, curr_y + 14.0 * scale, 9.5 * scale, pr, pg, pb, alpha);

                // Small [kill] chip
                let kill_id = "kill_port";
                let is_kill_hov = hovered_button == Some(kill_id);
                let kill_x = left + 92.0 * scale;
                let kill_w = 26.0 * scale;
                let kill_h = 20.0 * scale;
                if kill_x + kill_w < hex_x - 4.0 * scale {
                    self.draw_tinted_card_bg(pixmap, kill_x, curr_y, kill_w, kill_h, 3.0 * scale, alpha, is_kill_hov, Some((239, 68, 68)));
                    let (kr, kg, kb) = if is_kill_hov { (255, 255, 255) } else { (239, 68, 68) };
                    draw_text(pixmap, font, "kill", kill_x + 5.0 * scale, curr_y + 14.0 * scale, 8.5 * scale, kr, kg, kb, alpha);
                    buttons.push(ButtonArea { id: kill_id, x: kill_x, y: curr_y, w: kill_w, h: kill_h });
                }
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
