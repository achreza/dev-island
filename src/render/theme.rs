use tiny_skia::Color;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub border: Color,
    pub accent_dot: Color,
    pub text_primary: (u8, u8, u8),
    pub text_secondary: (u8, u8, u8),
    pub card_bg: Color,
    pub card_border: Color,
    pub card_hover_bg: Color,
    pub shadow: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Dark obsidian pill background (96% opacity)
            bg: Color::from_rgba8(14, 15, 19, 245),
            // Subtle crisp border
            border: Color::from_rgba8(255, 255, 255, 30),
            // Emerald green live indicator dot
            accent_dot: Color::from_rgba8(34, 197, 94, 255),
            // Crisp white text
            text_primary: (241, 245, 249),
            // Muted secondary text
            text_secondary: (148, 163, 184),
            // Card background in expanded mode
            card_bg: Color::from_rgba8(255, 255, 255, 12),
            card_border: Color::from_rgba8(255, 255, 255, 18),
            card_hover_bg: Color::from_rgba8(255, 255, 255, 25),
            // Smooth drop shadow
            shadow: Color::from_rgba8(0, 0, 0, 75),
        }
    }
}
