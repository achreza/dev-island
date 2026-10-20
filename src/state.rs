#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IslandMode {
    Collapsed,
    Expanded,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AppStatus {
    pub visibility: Visibility,
    pub mode: IslandMode,
    pub is_animating: bool,
    pub active_task: String,
}

impl Default for AppStatus {
    fn default() -> Self {
        Self {
            visibility: Visibility::Visible,
            mode: IslandMode::Collapsed,
            is_animating: false,
            active_task: "Idle".to_string(),
        }
    }
}
