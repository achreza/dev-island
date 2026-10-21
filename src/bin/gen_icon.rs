//! Standalone icon generator for Dynamic Island Dev
//! Creates a stunning high-DPI Windows application icon in `assets/app.ico`.

use std::fs;
use std::path::Path;
use tiny_skia::{
    Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

fn draw_island_icon(size: u32) -> Vec<u8> {
    let mut pixmap = Pixmap::new(size, size).expect("Failed to create pixmap");
    let scale = size as f32 / 256.0;

    // 1. Squircle background (Dark obsidian gradient)
    let bg_rect = Rect::from_xywh(8.0 * scale, 8.0 * scale, 240.0 * scale, 240.0 * scale).unwrap();
    let r = 54.0 * scale;

    let mut bg_path = PathBuilder::new();
    bg_path.move_to(bg_rect.left() + r, bg_rect.top());
    bg_path.line_to(bg_rect.right() - r, bg_rect.top());
    bg_path.quad_to(bg_rect.right(), bg_rect.top(), bg_rect.right(), bg_rect.top() + r);
    bg_path.line_to(bg_rect.right(), bg_rect.bottom() - r);
    bg_path.quad_to(bg_rect.right(), bg_rect.bottom(), bg_rect.right() - r, bg_rect.bottom());
    bg_path.line_to(bg_rect.left() + r, bg_rect.bottom());
    bg_path.quad_to(bg_rect.left(), bg_rect.bottom(), bg_rect.left(), bg_rect.bottom() - r);
    bg_path.line_to(bg_rect.left(), bg_rect.top() + r);
    bg_path.quad_to(bg_rect.left(), bg_rect.top(), bg_rect.left() + r, bg_rect.top());
    let path = bg_path.finish().unwrap();

    let mut bg_paint = Paint::default();
    bg_paint.set_color(Color::from_rgba8(14, 15, 20, 255));
    bg_paint.anti_alias = true;
    pixmap.fill_path(&path, &bg_paint, FillRule::Winding, Transform::identity(), None);

    // Subtle outer border
    let mut stroke_paint = Paint::default();
    stroke_paint.set_color(Color::from_rgba8(255, 255, 255, 30));
    stroke_paint.anti_alias = true;
    let stroke = Stroke { width: 1.5 * scale, ..Default::default() };
    pixmap.stroke_path(&path, &stroke_paint, &stroke, Transform::identity(), None);

    // 2. The Dynamic Island Capsule
    let pill_w = 180.0 * scale;
    let pill_h = 58.0 * scale;
    let pill_x = (size as f32 - pill_w) / 2.0;
    let pill_y = 99.0 * scale;
    let pill_r = pill_h / 2.0;

    let mut pill_builder = PathBuilder::new();
    pill_builder.move_to(pill_x + pill_r, pill_y);
    pill_builder.line_to(pill_x + pill_w - pill_r, pill_y);
    pill_builder.quad_to(pill_x + pill_w, pill_y, pill_x + pill_w, pill_y + pill_r);
    pill_builder.quad_to(pill_x + pill_w, pill_y + pill_h, pill_x + pill_w - pill_r, pill_y + pill_h);
    pill_builder.line_to(pill_x + pill_r, pill_y + pill_h);
    pill_builder.quad_to(pill_x, pill_y + pill_h, pill_x, pill_y + pill_r);
    pill_builder.quad_to(pill_x, pill_y, pill_x + pill_r, pill_y);
    let pill_path = pill_builder.finish().unwrap();

    let mut pill_paint = Paint::default();
    pill_paint.set_color(Color::from_rgba8(24, 25, 33, 255));
    pill_paint.anti_alias = true;
    pixmap.fill_path(&pill_path, &pill_paint, FillRule::Winding, Transform::identity(), None);

    let mut pill_border = Paint::default();
    pill_border.set_color(Color::from_rgba8(255, 255, 255, 60));
    pill_border.anti_alias = true;
    let p_stroke = Stroke { width: 1.5 * scale, ..Default::default() };
    pixmap.stroke_path(&pill_path, &pill_border, &p_stroke, Transform::identity(), None);

    // 3. Glowing Indicator Dot (Cyan Neon)
    let dot_x = pill_x + 24.0 * scale;
    let dot_y = pill_y + pill_h / 2.0;
    let dot_r = 9.0 * scale;

    // Outer glow
    let mut glow_builder = PathBuilder::new();
    glow_builder.push_circle(dot_x, dot_y, dot_r * 1.8);
    if let Some(gp) = glow_builder.finish() {
        let mut gp_paint = Paint::default();
        gp_paint.set_color(Color::from_rgba8(6, 182, 212, 70));
        gp_paint.anti_alias = true;
        pixmap.fill_path(&gp, &gp_paint, FillRule::Winding, Transform::identity(), None);
    }

    // Core dot
    let mut dot_builder = PathBuilder::new();
    dot_builder.push_circle(dot_x, dot_y, dot_r);
    if let Some(dp) = dot_builder.finish() {
        let mut dp_paint = Paint::default();
        dp_paint.set_color(Color::from_rgba8(34, 211, 238, 255));
        dp_paint.anti_alias = true;
        pixmap.fill_path(&dp, &dp_paint, FillRule::Winding, Transform::identity(), None);
    }

    // 4. Waveform / Code Accent Bars
    let bar_x = pill_x + 52.0 * scale;
    let bar_y = pill_y + 19.0 * scale;
    let bar_h = 20.0 * scale;

    let bar_heights = [12.0, 18.0, 10.0, 16.0, 14.0, 8.0, 15.0];
    for (i, &h) in bar_heights.iter().enumerate() {
        let bx = bar_x + (i as f32) * 14.0 * scale;
        let by = bar_y + (bar_h - h * scale) / 2.0;
        let bw = 4.5 * scale;

        let r_path = PathBuilder::from_rect(Rect::from_xywh(bx, by, bw, h * scale).unwrap());
        let mut bar_paint = Paint::default();
        let alpha = (160 + (i as u8) * 12).min(240);
        bar_paint.set_color(Color::from_rgba8(168, 85, 247, alpha)); // Purple glow
        bar_paint.anti_alias = true;
        pixmap.fill_path(&r_path, &bar_paint, FillRule::Winding, Transform::identity(), None);
    }

    pixmap.encode_png().expect("Failed to encode PNG")
}

fn build_ico(png_256: &[u8], png_48: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    // ICONDIR
    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // type: 1 = icon
    out.extend_from_slice(&2u16.to_le_bytes()); // 2 images

    let header_size = 6;
    let entry_size = 16;
    let offset_0 = header_size + entry_size * 2;
    let offset_1 = offset_0 + png_256.len();

    // Image 0: 256x256
    out.push(0); // 0 = 256px
    out.push(0); // 0 = 256px
    out.push(0); // color count
    out.push(0); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // color planes
    out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
    out.extend_from_slice(&(png_256.len() as u32).to_le_bytes());
    out.extend_from_slice(&(offset_0 as u32).to_le_bytes());

    // Image 1: 48x48
    out.push(48);
    out.push(48);
    out.push(0);
    out.push(0);
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&(png_48.len() as u32).to_le_bytes());
    out.extend_from_slice(&(offset_1 as u32).to_le_bytes());

    // Image data
    out.extend_from_slice(png_256);
    out.extend_from_slice(png_48);

    out
}

fn main() {
    let assets_dir = Path::new("assets");
    if !assets_dir.exists() {
        fs::create_dir_all(assets_dir).expect("Failed to create assets dir");
    }

    let png_256 = draw_island_icon(256);
    let png_48 = draw_island_icon(48);

    let ico_bytes = build_ico(&png_256, &png_48);
    fs::write("assets/app.ico", ico_bytes).expect("Failed to write assets/app.ico");

    println!("✓ Generated assets/app.ico successfully!");
}
